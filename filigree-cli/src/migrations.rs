use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    path::{Path, PathBuf},
};

use error_stack::{Report, ResultExt};
use glob::glob;
use itertools::Itertools;
use sql_migration_sim::{
    ast::{
        AlterColumnOperation, AlterTableOperation, ColumnOption, ColumnOptionDef, Ident,
        ObjectName, Statement, TableConstraint,
    },
    table_constraint_name, Column, Schema, SchemaObjectType, Table,
};

use crate::{model::Model, Error};

#[derive(Debug, Clone)]
pub struct SingleMigration<'a> {
    pub name: String,
    pub model: Option<&'a Model>,
    pub up: Cow<'static, str>,
    pub down: Cow<'static, str>,
}

fn last_migration_path(state_dir: &Path) -> PathBuf {
    state_dir.join("schema.sql.gen")
}

pub fn read_previous_migration(state_dir: &Path) -> Result<Schema, Report<Error>> {
    let mut schema = Schema::new_with_dialect(sql_migration_sim::dialect::PostgreSqlDialect {});

    let file = last_migration_path(state_dir);

    // It's ok if the schema SQL doesn't exist.
    if let Ok(data) = std::fs::read_to_string(&file) {
        schema
            .apply_sql(&data)
            .change_context(Error::ReadMigrationFiles)
            .attach_printable_lazy(|| file.display().to_string())?;
    }

    Ok(schema)
}

pub fn save_migration_state(
    state_dir: &Path,
    migrations: &[SingleMigration<'_>],
) -> Result<(), Report<Error>> {
    let file = last_migration_path(state_dir);

    let full_migration = migrations.iter().map(|m| m.up.as_ref()).join("\n\n");
    std::fs::write(&file, full_migration)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| file.display().to_string())
}

struct ParsedMigration<'a> {
    source: SingleMigration<'a>,
    statements: Vec<sql_migration_sim::ast::Statement>,
}

fn parse_new_migrations<'a>(
    migrations: &'a [SingleMigration<'a>],
) -> Result<(Schema, Vec<ParsedMigration<'a>>), Report<Error>> {
    let mut new_schema = Schema::new_with_dialect(sql_migration_sim::dialect::PostgreSqlDialect {});

    let migrations = migrations
        .into_iter()
        .map(|m| {
            let statements = new_schema.parse_sql(&m.up)?;
            for statement in &statements {
                new_schema.apply_statement(statement.clone())?;
            }

            Ok::<_, sql_migration_sim::Error>(ParsedMigration {
                source: m.clone(),
                statements,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .change_context(Error::ReadMigrationFiles)?;

    Ok((new_schema, migrations))
}

pub fn resolve_migration(
    migrations_dir: &Path,
    state_dir: &Path,
    new_migrations: &[SingleMigration<'_>],
) -> Result<SingleMigration<'static>, Report<Error>> {
    let any_migrations_exist = glob(migrations_dir.join("*.sql").to_string_lossy().as_ref())
        .ok()
        .map(|mut g| g.next().is_some())
        .unwrap_or(false);
    let existing_schema = if any_migrations_exist {
        read_previous_migration(state_dir)?
    } else {
        // If the migrations were cleared out, then we need to regenerate the whole thing
        // regardless of what's in the state directory.
        Schema::new()
    };

    let (new_schema, migrations) = parse_new_migrations(new_migrations)?;

    let existing_pg_schemas = existing_schema
        .tables
        .iter()
        .filter_map(|t| t.1.schema())
        .collect::<HashSet<_>>();
    let schemas_to_create = new_schema
        .tables
        .iter()
        .filter_map(|t| t.1.schema())
        .filter(|s| !existing_pg_schemas.contains(s))
        .collect::<HashSet<_>>();

    let tables_to_create = new_schema
        .tables
        .iter()
        .filter(|t| !existing_schema.tables.contains_key(t.0))
        .map(|t| t.0.clone())
        .collect::<HashSet<_>>();

    let tables_to_drop = existing_schema
        .tables
        .iter()
        .filter(|t| !new_schema.tables.contains_key(t.0))
        .map(|t| t.0.clone())
        .collect::<HashSet<_>>();

    let indices_to_create = new_schema
        .indices
        .iter()
        .filter(|i| !existing_schema.indices.contains_key(i.0))
        .map(|i| i.0.clone())
        .collect::<HashSet<_>>();

    let indices_to_drop = existing_schema
        .indices
        .iter()
        .filter(|(index, table)| {
            !new_schema.indices.contains_key(index.as_str())
                // If we're dropping the table then it implicitly drops the indexes too
                && !tables_to_drop.contains(table.as_str())
        })
        .map(|i| i.0.clone())
        .collect::<HashSet<_>>();

    let functions_to_create = new_schema
        .functions
        .iter()
        .filter(|(name, f)| {
            existing_schema
                .functions
                .get(name.as_str())
                .map(|existing| &existing != f)
                .unwrap_or(true)
        })
        .map(|f| f.0.clone())
        .collect::<HashSet<_>>();

    let create_migration = migrations.iter().map(|m| {
        m.statements
            .iter()
            .filter(|s| match s {
                Statement::CreateSchema { schema_name, .. } => {
                    schemas_to_create.contains(schema_name.to_string().as_str())
                }
                Statement::CreateTable { name, .. } => tables_to_create.contains(&name.to_string()),
                Statement::CreateIndex { name, .. } => {
                    let Some(name) = name.as_ref() else {
                        return true;
                    };

                    indices_to_create.contains(&name.to_string())
                }
                Statement::CreateFunction { name, .. } => {
                    functions_to_create.contains(&name.to_string())
                }
                _ => false,
            })
            .map(|s| format!("{s};"))
            .join("\n")
    });

    let models_by_table = migrations
        .iter()
        .filter_map(|m| {
            let model = m.source.model?;
            Some((model.table(), model))
        })
        .collect::<HashMap<_, _>>();

    let mut up_changes = vec![];
    let mut down_changes = vec![];

    for (table_name, new_table) in &new_schema.tables {
        let Some(old_table) = existing_schema.tables.get(table_name) else {
            continue;
        };

        let model = models_by_table.get(table_name).map(|m| *m);
        let changes = diff_table(model, old_table, new_table);

        up_changes.extend(
            changes
                .iter()
                .map(|c| TableChangeUpMigration(c).to_string()),
        );
        down_changes.extend(
            changes
                .iter()
                .rev()
                .map(|c| TableChangeDownMigration(c, old_table).to_string()),
        );
    }

    let up_drop_migration = existing_schema
        .creation_order
        .iter()
        // TODO Would be better to topologically sort the tables according to foreign keys but
        // plain reverse order will usually be correct.
        .rev()
        .filter(|o| match o.object_type {
            SchemaObjectType::Table => tables_to_drop.contains(&o.name),
            SchemaObjectType::Index => indices_to_drop.contains(&o.name),
            _ => false,
        })
        .map(|o| {
            format!(
                "DROP {obj_type} {name};",
                obj_type = o.object_type,
                name = o.name
            )
        });

    let up_migration = up_drop_migration
        .chain(create_migration)
        .chain(up_changes)
        .join("\n\n");

    let down_migration = new_schema
        .creation_order
        .iter()
        .rev()
        .filter(|o| match o.object_type {
            SchemaObjectType::Table => tables_to_create.contains(&o.name),
            SchemaObjectType::Index => indices_to_create.contains(&o.name),
            _ => false,
        })
        .map(|o| {
            format!(
                "DROP {obj_type} {name};",
                obj_type = o.object_type,
                name = o.name
            )
        })
        .chain(down_changes)
        .join("\n\n");

    Ok(SingleMigration {
        name: "migrations".into(),
        model: None,
        up: up_migration.trim().to_string().into(),
        down: down_migration.trim().to_string().into(),
    })
}

struct TableChangeUpMigration<'a>(&'a TableChange);

impl<'a> Display for TableChangeUpMigration<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.up(f)
    }
}

struct TableChangeDownMigration<'a>(&'a TableChange, &'a Table);

impl<'a> Display for TableChangeDownMigration<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.down(f, self.1)
    }
}

enum TableChange {
    AddColumn {
        table: ObjectName,
        column: Column,
    },
    RemoveColumn {
        table: ObjectName,
        column: Column,
    },
    RenameColumn {
        table: ObjectName,
        old_name: Ident,
        new_name: Ident,
    },
    AlterColumn {
        table: ObjectName,
        column_name: Ident,
        changes: Vec<AlterColumnOperation>,
    },
    AddConstraint {
        table: ObjectName,
        constraint: TableConstraint,
    },
    RemoveConstraint {
        table: ObjectName,
        constraint_name: Ident,
        constraint: Option<TableConstraint>,
    },
}

impl TableChange {
    fn up(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableChange::AddColumn { table, column } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    location: None,
                    operations: vec![AlterTableOperation::AddColumn {
                        column_keyword: true,
                        if_not_exists: false,
                        column_def: column.0.clone(),
                        column_position: None,
                    }],
                };

                write!(f, "{statement};\n")
            }
            TableChange::RemoveColumn { table, column } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    location: None,
                    operations: vec![AlterTableOperation::DropColumn {
                        column_name: column.name.clone(),
                        if_exists: false,
                        cascade: true,
                    }],
                };

                write!(f, "{statement};\n")
            }
            TableChange::RenameColumn {
                table,
                old_name,
                new_name,
            } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    location: None,
                    operations: vec![AlterTableOperation::RenameColumn {
                        old_column_name: old_name.clone(),
                        new_column_name: new_name.clone(),
                    }],
                };

                write!(f, "{statement};\n")
            }
            TableChange::AlterColumn {
                table,
                column_name,
                changes,
            } => {
                for op in changes {
                    let statement = Statement::AlterTable {
                        name: table.clone(),
                        if_exists: false,
                        only: false,
                        location: None,
                        operations: vec![AlterTableOperation::AlterColumn {
                            column_name: column_name.clone(),
                            op: op.clone(),
                        }],
                    };

                    write!(f, "{statement};\n")?;
                }

                Ok(())
            }
            TableChange::AddConstraint {
                table, constraint, ..
            } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    location: None,
                    operations: vec![AlterTableOperation::AddConstraint(constraint.clone())],
                };

                write!(f, "{statement};\n")
            }
            TableChange::RemoveConstraint {
                table,
                constraint_name,
                ..
            } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    location: None,
                    operations: vec![AlterTableOperation::DropConstraint {
                        if_exists: true,
                        cascade: false,
                        name: constraint_name.clone(),
                    }],
                };

                write!(f, "{statement};\n")
            }
        }
    }

    fn down(&self, f: &mut std::fmt::Formatter<'_>, old_table: &Table) -> std::fmt::Result {
        match self {
            TableChange::AddColumn { table, column } => TableChange::RemoveColumn {
                table: table.clone(),
                column: column.clone(),
            }
            .up(f),

            TableChange::RemoveColumn { table, column } => TableChange::AddColumn {
                table: table.clone(),
                column: column.clone(),
            }
            .up(f),

            TableChange::RenameColumn {
                table,
                old_name,
                new_name,
            } => TableChange::RenameColumn {
                table: table.clone(),
                old_name: new_name.clone(),
                new_name: old_name.clone(),
            }
            .up(f),

            TableChange::AlterColumn {
                table,
                column_name,
                changes,
            } => {
                for op in changes {
                    if let Some(op) = opposite_alter_op(column_name, op, old_table) {
                        let statement = Statement::AlterTable {
                            name: table.clone(),
                            if_exists: true,
                            only: false,
                            location: None,
                            operations: vec![AlterTableOperation::AlterColumn {
                                column_name: column_name.clone(),
                                op,
                            }],
                        };

                        write!(f, "{statement};\n")?;
                    }
                }

                Ok(())
            }

            TableChange::AddConstraint { table, constraint } => {
                if let Some(constraint_name) = table_constraint_name(constraint) {
                    TableChange::RemoveConstraint {
                        table: table.clone(),
                        constraint_name: constraint_name.clone(),
                        constraint: Some(constraint.clone()),
                    }
                    .up(f)
                } else {
                    Ok(())
                }
            }

            TableChange::RemoveConstraint {
                table, constraint, ..
            } => {
                if let Some(constraint) = constraint {
                    TableChange::AddConstraint {
                        table: table.clone(),
                        constraint: constraint.clone(),
                    }
                    .up(f)
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn opposite_alter_op(
    column_name: &Ident,
    op: &AlterColumnOperation,
    old_table: &Table,
) -> Option<AlterColumnOperation> {
    match op {
        AlterColumnOperation::SetNotNull => Some(AlterColumnOperation::DropNotNull),
        AlterColumnOperation::DropNotNull => Some(AlterColumnOperation::SetNotNull),
        AlterColumnOperation::SetDefault { .. } => {
            let default_value = old_table
                .columns
                .iter()
                .find(|c| &c.name == column_name)
                .and_then(|c| c.default_value());
            if let Some(default_value) = default_value {
                Some(AlterColumnOperation::SetDefault {
                    value: default_value.clone(),
                })
            } else {
                Some(AlterColumnOperation::DropDefault)
            }
        }
        AlterColumnOperation::DropDefault => {
            let default_value = old_table
                .columns
                .iter()
                .find(|c| &c.name == column_name)
                .and_then(|c| c.default_value());
            if let Some(default_value) = default_value {
                Some(AlterColumnOperation::SetDefault {
                    value: default_value.clone(),
                })
            } else {
                None
            }
        }
        AlterColumnOperation::SetDataType { .. } => {
            let data_type = old_table
                .columns
                .iter()
                .find(|c| &c.name == column_name)
                .map(|c| &c.data_type);

            if let Some(data_type) = data_type {
                Some(AlterColumnOperation::SetDataType {
                    data_type: data_type.clone(),
                    using: None,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Given a column constraint, generate an equivalent table constraint, if there is one.
fn process_column_option_to_table_constraint(
    column_name: &Ident,
    option: &ColumnOptionDef,
) -> Option<TableConstraint> {
    match &option.option {
        ColumnOption::ForeignKey {
            foreign_table,
            referred_columns,
            on_delete,
            on_update,
            characteristics,
        } => Some(TableConstraint::ForeignKey {
            name: option.name.clone(),
            columns: vec![column_name.clone()],
            foreign_table: foreign_table.clone(),
            referred_columns: referred_columns.clone(),
            on_delete: on_delete.clone(),
            on_update: on_update.clone(),
            characteristics: characteristics.clone(),
        }),

        ColumnOption::Unique {
            is_primary,
            characteristics,
        } => {
            if *is_primary {
                Some(TableConstraint::PrimaryKey {
                    name: option.name.clone(),
                    index_name: None,
                    index_type: None,
                    index_options: vec![],
                    columns: vec![column_name.clone()],
                    characteristics: characteristics.clone(),
                })
            } else {
                Some(TableConstraint::Unique {
                    name: option.name.clone(),
                    columns: vec![column_name.clone()],
                    characteristics: characteristics.clone(),
                    index_name: None,
                    index_type: None,
                    index_options: vec![],
                    index_type_display: sql_migration_sim::ast::KeyOrIndexDisplay::None,
                })
            }
        }

        ColumnOption::Check(expr) => Some(TableConstraint::Check {
            name: option.name.clone(),
            expr: Box::new(expr.clone()),
        }),

        _ => None,
    }
}

fn diff_table(model: Option<&Model>, old_table: &Table, new_table: &Table) -> Vec<TableChange> {
    let mut changes = vec![];

    let old_columns = old_table
        .columns
        .iter()
        .map(|c| (c.name(), c))
        .collect::<HashMap<_, _>>();
    let new_columns = new_table
        .columns
        .iter()
        .map(|c| (c.name(), c))
        .collect::<HashMap<_, _>>();

    let fields_previous_names = model
        .map(|m| {
            m.fields
                .iter()
                .filter_map(|f| {
                    let old_name = f.previous_sql_field_name()?;
                    let new_name = f.sql_field_name();
                    Some((new_name, old_name))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    let fields_by_previous_name = fields_previous_names
        .iter()
        .map(|(new_name, old_name)| (old_name.as_str(), new_name.as_str()))
        .collect::<HashMap<_, _>>();

    let mut new_table_constraints = new_table.constraints.clone();
    let mut old_table_constraints = old_table.constraints.clone();

    // Look for new or altered columns
    for column in &new_table.columns {
        let column_name = column.name();
        let matching_column = old_columns.get(column_name).or_else(|| {
            fields_previous_names
                .get(column_name)
                .and_then(|f| old_columns.get(f.as_str()))
        });

        new_table_constraints.extend(
            column
                .options
                .iter()
                .filter_map(|o| process_column_option_to_table_constraint(&column.name, o)),
        );

        if let Some(matching_column) = matching_column {
            if matching_column.name() != column_name {
                changes.push(TableChange::RenameColumn {
                    table: new_table.name.clone(),
                    old_name: matching_column.name.clone(),
                    new_name: column.name.clone(),
                })
            }

            let operations = diff_column(matching_column, column);
            if !operations.is_empty() {
                changes.push(TableChange::AlterColumn {
                    table: new_table.name.clone(),
                    column_name: column.name.clone(),
                    changes: operations,
                })
            }
        } else {
            changes.push(TableChange::AddColumn {
                table: new_table.name.clone(),
                column: column.clone(),
            })
        }
    }

    // Look for removed columns
    for column in &old_table.columns {
        let column_name = column.name();
        let matching_column = new_columns.get(column_name).or_else(|| {
            fields_by_previous_name
                .get(column_name)
                .and_then(|f| new_columns.get(f))
        });

        old_table_constraints.extend(
            column
                .options
                .iter()
                .filter_map(|o| process_column_option_to_table_constraint(&column.name, o)),
        );

        if matching_column.is_none() {
            changes.push(TableChange::RemoveColumn {
                table: new_table.name.clone(),
                column: column.clone(),
            })
        }
    }

    // Look for constraints to drop
    for constraint in &old_table_constraints {
        if let Some(name) = table_constraint_name(constraint) {
            if new_table_constraints
                .iter()
                .find(|c| c == &constraint)
                .is_none()
            {
                changes.push(TableChange::RemoveConstraint {
                    table: old_table.name.clone(),
                    constraint_name: name.clone(),
                    constraint: Some(constraint.clone()),
                });
            }
        }
    }

    // Look for new constraints to add
    for constraint in &new_table_constraints {
        if old_table_constraints
            .iter()
            .find(|c| c == &constraint)
            .is_none()
        {
            changes.push(TableChange::AddConstraint {
                table: new_table.name.clone(),
                constraint: constraint.clone(),
            });
        }
    }

    changes
}

fn diff_column(old_column: &Column, new_column: &Column) -> Vec<AlterColumnOperation> {
    let mut changes = vec![];
    if new_column.data_type != old_column.data_type {
        changes.push(AlterColumnOperation::SetDataType {
            data_type: new_column.data_type.clone(),
            using: None,
        })
    };

    let new_not_null = new_column.not_null();
    let old_not_null = old_column.not_null();
    if new_not_null != old_not_null {
        changes.push(if new_not_null {
            AlterColumnOperation::SetNotNull
        } else {
            AlterColumnOperation::DropNotNull
        });
    }

    let new_default = new_column.default_value();
    let old_default = old_column.default_value();
    if new_default != old_default {
        if let Some(new_default) = new_default {
            changes.push(AlterColumnOperation::SetDefault {
                value: new_default.clone(),
            })
        } else {
            changes.push(AlterColumnOperation::DropDefault)
        }
    }

    changes
}
