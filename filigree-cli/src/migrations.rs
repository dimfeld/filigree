use std::path::Path;

use error_stack::{Report, ResultExt};
use glob::glob;
use sql_migration_sim::Schema;

use crate::Error;

pub struct MigrationGenerator {}

pub fn read_existing_migrations(migrations_dir: &Path) -> Result<Schema, Report<Error>> {
    let mut schema = Schema::new();

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

pub fn resolve_migration(migrations_dir: &Path) -> Result<(), Report<Error>> {
    let existing_schema = read_existing_migrations(migrations_dir)?;
    // Read the migrations from the migrations directory
    // Build the schema
    // Compare the schema to the schema generated from the templates
    // If there is a difference, create a migration
    // Completely new models should just use the template
    // Changes to existing models should have their DDL generated from the diff

    // TODO what to return here? The new migration?
    Ok(())
}
