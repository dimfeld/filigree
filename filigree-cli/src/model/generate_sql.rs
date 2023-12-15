use error_stack::Report;
use rayon::prelude::*;

use super::generator::ModelGenerator;
use crate::Error;

type GenResult = Result<(&'static str, Vec<u8>), Report<Error>>;

impl<'a> ModelGenerator<'a> {
    pub fn render_up_migration(&self) -> GenResult {
        self.render("migrate_up.sql.tera")
    }

    pub fn render_down_migration(&self) -> GenResult {
        self.render("migrate_down.sql.tera")
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
