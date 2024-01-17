use std::{borrow::Cow, collections::HashSet, path::Path};

use error_stack::{Report, ResultExt};
use glob::glob;
use itertools::Itertools;
use sql_migration_sim::{ast::Statement, Schema};

use crate::Error;

pub struct SingleMigration {
    pub name: String,
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

struct ParsedMigration {
    source: SingleMigration,
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
    new_migrations: Vec<SingleMigration>,
) -> Result<SingleMigration, Report<Error>> {
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
        up: up_migration.into(),
        down: down_migration.into(),
    })
}
