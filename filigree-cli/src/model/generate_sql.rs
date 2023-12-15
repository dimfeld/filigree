use error_stack::Report;
use rayon::prelude::*;

use super::generator::ModelGenerator;
use crate::{Error, RenderedFile};

impl<'a> ModelGenerator<'a> {
    pub fn render_up_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_up.sql.tera").map(|f| f.contents)
    }

    pub fn render_down_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_down.sql.tera").map(|f| f.contents)
    }

    pub fn render_sql_queries(&self) -> Result<Vec<RenderedFile>, Report<Error>> {
        let files = [
            "select_one.sql.tera",
            "insert.sql.tera",
            "update.sql.tera",
            "delete.sql.tera",
        ];

        let output = files
            .into_par_iter()
            .map(|file| self.render(file))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(output)
    }
}
