use error_stack::Report;
use rayon::prelude::*;

use super::generator::ModelGenerator;
use crate::Error;

impl<'a> ModelGenerator<'a> {
    pub fn render_up_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_up.sql.tera").map(|(_, v)| v)
    }

    pub fn render_down_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_down.sql.tera").map(|(_, v)| v)
    }

    pub fn write_sql_queries(&self) -> Result<(), Report<Error>> {
        let files = [
            "select_one.sql.tera",
            "insert.sql.tera",
            "update.sql.tera",
            "delete.sql.tera",
        ];

        files.into_par_iter().try_for_each(|file| {
            let (filename, output) = self.render(file)?;
            self.write_to_file(filename, &output)?;
            Ok::<_, Report<Error>>(())
        })?;

        Ok(())
    }
}
