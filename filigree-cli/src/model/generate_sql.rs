use error_stack::Report;

use super::ModelGenerator;
use crate::Error;

impl<'a> ModelGenerator<'a> {
    pub fn generate_up_migration(&self) -> Result<String, tera::Error> {
        self.tera.render("migrate_up.sql.tera", &self.context)
    }

    pub fn generate_down_migration(&self) -> Result<String, tera::Error> {
        self.tera.render("migrate_down.sql.tera", &self.context)
    }

    pub fn write_select_one_query(&self) -> Result<(), Report<Error>> {
        self.render_to_file("select_one.sql.tera", "select_one.sql")
    }

    pub fn write_insert_query(&self) -> Result<(), Report<Error>> {
        self.render_to_file("insert.sql.tera", "insert.sql")
    }

    pub fn write_update_query(&self) -> Result<(), Report<Error>> {
        self.render_to_file("update.sql.tera", "update.sql")
    }

    pub fn write_delete_query(&self) -> Result<(), Report<Error>> {
        self.render_to_file("delete.sql.tera", "delete.sql")
    }
}
