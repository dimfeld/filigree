use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    path::Path,
};

use error_stack::{Report, ResultExt};
use glob::glob;
use itertools::Itertools;
use sql_migration_sim::{
    ast::{
        AlterColumnOperation, AlterTableOperation, ColumnOption, DataType, Ident, ObjectName,
        Statement,
    },
    Column, Schema, Table,
};

use crate::{model::Model, Error};

pub struct SingleMigration<'a> {
    pub name: String,
    pub model: Option<&'a Model>,
    pub up: Cow<'static, str>,
    pub down: Cow<'static, str>,
}

pub fn read_existing_migrations(migrations_dir: &Path) -> Result<Schema, Report<Error>> {
    let mut schema = Schema::new_with_dialect(sql_migration_sim::dialect::PostgreSqlDialect {});

    let mut files = glob(migrations_dir.join("*.up.sql").to_string_lossy().as_ref())
        .change_context(Error::ReadMigrationFiles)?
        .collect::<Result<Vec<_>, _>>()
        .change_context(Error::ReadMigrationFiles)?;
    files.sort();

    for file in files {
        schema
            .apply_file(&file)
            .change_context(Error::ReadMigrationFiles)?;
    }

    Ok(schema)
}

struct ParsedMigration<'a> {
    source: SingleMigration<'a>,
    statements: Vec<sql_migration_sim::ast::Statement>,
}

fn parse_new_migrations(
    migrations: Vec<SingleMigration>,
) -> Result<(Schema, Vec<ParsedMigration>), Report<Error>> {
    let mut new_schema = Schema::new_with_dialect(sql_migration_sim::dialect::PostgreSqlDialect {});

    let migrations = migrations
        .into_iter()
        .map(|m| {
            let statements = new_schema.parse_sql(&m.up)?;
            for statement in &statements {
                new_schema.apply_statement(statement.clone())?;
            }

            Ok::<_, sql_migration_sim::Error>(ParsedMigration {
                source: m,
                statements,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .change_context(Error::ReadMigrationFiles)?;

    Ok((new_schema, migrations))
}

pub fn resolve_migration(
    migrations_dir: &Path,
    new_migrations: Vec<SingleMigration<'_>>,
) -> Result<SingleMigration<'static>, Report<Error>> {
    let existing_schema = read_existing_migrations(migrations_dir)?;
    let (new_schema, migrations) = parse_new_migrations(new_migrations)?;

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
        .collect::<Vec<_>>();

    let indexes_to_create = new_schema
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
                && !tables_to_drop.contains(table)
        })
        .map(|i| i.0.clone())
        .collect::<Vec<_>>();

    let create_migration = migrations.iter().map(|m| {
        m.statements
            .iter()
            .filter(|s| match s {
                Statement::CreateTable { name, .. } => tables_to_create.contains(&name.to_string()),
                Statement::CreateIndex { name, .. } => {
                    let Some(name) = name.as_ref() else {
                        return true;
                    };

                    indexes_to_create.contains(&name.to_string())
                }
                _ => false,
            })
            .map(|s| format!("{s};"))
            .join("\n")
    });

    // TODO Alter tables that exist on both sides.

    let drop_tables_migration = tables_to_drop
        .iter()
        .map(|name| format!("DROP TABLE {name};"));
    let drop_indices_migration = indices_to_drop
        .iter()
        .map(|name| format!("DROP INDEX {name};"));

    let up_migration = create_migration
        .chain(drop_tables_migration)
        .chain(drop_indices_migration)
        .join("\n\n");

    let down_migration = tables_to_create
        .iter()
        .map(|name| format!("DROP TABLE {name};"))
        .chain(
            indexes_to_create
                .iter()
                .map(|name| format!("DROP INDEX {name};")),
        )
        .join("\n\n");

    Ok(SingleMigration {
        name: "migrations".into(),
        model: None,
        up: up_migration.into(),
        down: down_migration.into(),
    })
}

enum TableChange {
    AddColumn {
        table: ObjectName,
        column: Column,
    },
    RemoveColumn {
        table: ObjectName,
        column: Ident,
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
}

impl Display for TableChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableChange::AddColumn { table, column } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    operations: vec![AlterTableOperation::AddColumn {
                        column_keyword: true,
                        if_not_exists: false,
                        column_def: column.0.clone(),
                    }],
                };

                write!(f, "{statement};\n")
            }
            TableChange::RemoveColumn { table, column } => {
                let statement = Statement::AlterTable {
                    name: table.clone(),
                    if_exists: false,
                    only: false,
                    operations: vec![AlterTableOperation::DropColumn {
                        column_name: column.clone(),
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
                        operations: vec![AlterTableOperation::AlterColumn {
                            column_name: column_name.clone(),
                            op: op.clone(),
                        }],
                    };

                    write!(f, "{statement};\n")?;
                }

                Ok(())
            }
        }
    }
}

struct AlterColumnOperations {
    name: String,
    changes: Vec<AlterColumnOperation>,
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

    // Look for new or altered columns
    for column in &new_table.columns {
        let column_name = column.name();
        let model_field =
            model.and_then(|m| m.fields.iter().find(|f| f.sql_field_name() == column_name));

        let matching_column = old_columns.get(column_name).or_else(|| {
            fields_previous_names
                .get(column_name)
                .and_then(|f| old_columns.get(f.as_str()))
        });

        if let Some(matching_column) = matching_column {
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

        if matching_column.is_none() {
            changes.push(TableChange::RemoveColumn {
                table: new_table.name.clone(),
                column: column.name.clone(),
            })
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

    // TODO add constraints once submodels are supported

    changes
}