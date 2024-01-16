//! This library is meant to parse multiple related SQL migration files, and calculate the final
//! schema that resutls from running them in order.
//!
//! ## Example
//!
//! ```rust
//! use sql_migration_sim::{Schema, Error};
//!
//! fn main() -> Result<(), error_stack::Report<Error>> {
//!     let mut schema = Schema::new();
//!
//!     let create_statement = r##"CREATE TABLE ships (
//!        id BIGINT PRIMARY KEY,
//!        name TEXT NOT NULL,
//!        mast_count INT not null
//!     );"##;
//!
//!     let alter = r##"
//!         ALTER TABLE ships ALTER COLUMN mast_count DROP NOT NULL;
//!         ALTER TABLE ships ADD COLUMN has_motor BOOLEAN NOT NULL;
//!         "##;
//!
//!     schema.apply_sql(create_statement)?;
//!     schema.apply_sql(alter)?;
//!
//!
//!     let result = schema.tables.get("ships").unwrap();
//!     println!("{:#?}", result.columns);
//!     assert_eq!(result.columns.len(), 4);
//!     assert_eq!(result.columns[0].name(), "id");
//!     assert!(matches!(result.columns[0].data_type, sqlparser::ast::DataType::BigInt(_)));
//!     assert_eq!(result.columns[0].not_null(), true);
//!     assert_eq!(result.columns[1].name(), "name");
//!     assert_eq!(result.columns[1].not_null(), true);
//!     assert_eq!(result.columns[2].name(), "mast_count");
//!     assert_eq!(result.columns[2].not_null(), false);
//!     assert_eq!(result.columns[3].name(), "has_motor");
//!     assert_eq!(result.columns[3].not_null(), true);
//!
//!     Ok(())
//! }
//! ```
//!

#[warn(missing_docs)]
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use error_stack::{Report, ResultExt};
use sqlparser::{
    ast::{
        AlterColumnOperation, AlterTableOperation, ColumnDef, ColumnOption, ColumnOptionDef,
        ObjectType, Statement,
    },
    dialect::Dialect,
};

#[derive(Debug)]
pub struct Column(ColumnDef);

impl Deref for Column {
    type Target = ColumnDef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Column {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Column {
    pub fn name(&self) -> &str {
        self.name.value.as_str()
    }

    pub fn not_null(&self) -> bool {
        self.options
            .iter()
            .find_map(|o| match o.option {
                ColumnOption::Null => Some(false),
                ColumnOption::NotNull => Some(true),
                ColumnOption::Unique { is_primary } => is_primary.then_some(true),
                _ => None,
            })
            .unwrap_or(false)
    }
}

pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

pub struct View {
    pub name: String,
    pub columns: Vec<String>,
}

pub struct Schema {
    dialect: Box<dyn Dialect>,
    pub tables: HashMap<String, Table>,
    pub views: HashMap<String, View>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Attempted to alter a table {0} that does not exist")]
    AlteredMissingTable(String),
    #[error("Attempted to alter a column {0} that does not exist in table {1}")]
    AlteredMissingColumn(String, String),
    #[error("Attempted to create table {0} that already exists")]
    TableAlreadyExists(String),
    #[error("Attempted to create column {0} that already exists in table {1}")]
    ColumnAlreadyExists(String, String),
    #[error("SQL Parse Error")]
    Parse,
    #[error("Failed to read file")]
    File,
}

impl Schema {
    /// Create a new [Schema] that parses with a generic SQL dialect
    pub fn new() -> Self {
        Self::new_with_dialect(sqlparser::dialect::GenericDialect {})
    }

    /// Create a new [Schema] that parses with the given SQL dialect
    pub fn new_with_dialect<D: Dialect>(dialect: D) -> Self {
        let dialect = Box::new(dialect);
        Self {
            tables: HashMap::new(),
            views: HashMap::new(),
            dialect,
        }
    }

    fn create_table(&mut self, name: String, columns: Vec<ColumnDef>) -> Result<(), Error> {
        if self.tables.contains_key(&name) {
            return Err(Error::TableAlreadyExists(name));
        }

        self.tables.insert(
            name.clone(),
            Table {
                name,
                columns: columns.into_iter().map(Column).collect(),
            },
        );

        Ok(())
    }

    fn create_view(&mut self, name: String, columns: Vec<String>) -> Result<(), Error> {
        if self.views.contains_key(&name) {
            return Err(Error::TableAlreadyExists(name));
        }

        self.views.insert(name.clone(), View { name, columns });

        Ok(())
    }

    fn apply_statement(&mut self, statement: Statement) -> Result<(), Report<Error>> {
        match statement {
            Statement::CreateTable { name, columns, .. } => {
                self.create_table(name.to_string(), columns)?;
            }
            Statement::AlterTable {
                name, operations, ..
            } => {
                let name = name.to_string();
                for operation in operations {
                    match operation {
                        AlterTableOperation::AddColumn {
                            if_not_exists,
                            column_def,
                            ..
                        } => {
                            let table = self
                                .tables
                                .get_mut(&name)
                                .ok_or_else(|| Error::AlteredMissingTable(name.clone()))?;

                            let existing_column =
                                table.columns.iter().find(|c| c.name == column_def.name);

                            if existing_column.is_none() {
                                table.columns.push(Column(column_def));
                            } else if !if_not_exists {
                                return Err(Report::new(Error::ColumnAlreadyExists(
                                    column_def.name.value,
                                    name.clone(),
                                )));
                            }
                        }

                        AlterTableOperation::DropColumn { column_name, .. } => {
                            let table = self
                                .tables
                                .get_mut(&name)
                                .ok_or_else(|| Error::AlteredMissingTable(name.clone()))?;
                            table.columns.retain(|c| c.name != column_name);
                        }

                        AlterTableOperation::RenameColumn {
                            old_column_name,
                            new_column_name,
                        } => {
                            let table = self
                                .tables
                                .get_mut(&name)
                                .ok_or_else(|| Error::AlteredMissingTable(name.clone()))?;

                            let column = table
                                .columns
                                .iter_mut()
                                .find(|c| c.name == old_column_name)
                                .ok_or_else(|| {
                                    Error::AlteredMissingColumn(
                                        old_column_name.value.clone(),
                                        name.clone(),
                                    )
                                })?;
                            column.name = new_column_name;
                        }

                        AlterTableOperation::RenameTable {
                            table_name: new_table_name,
                        } => {
                            let new_table_name = new_table_name.to_string();
                            let mut table = self.tables.remove(&name).ok_or_else(|| {
                                Error::AlteredMissingTable(new_table_name.clone())
                            })?;
                            table.name = new_table_name.clone();
                            // TODO this probably doesn't properly handle tables that are in a
                            // non-default schema
                            self.tables.insert(new_table_name, table);
                        }

                        AlterTableOperation::AlterColumn { column_name, op } => {
                            let table = self
                                .tables
                                .get_mut(&name)
                                .ok_or_else(|| Error::AlteredMissingTable(name.clone()))?;

                            let column = table
                                .columns
                                .iter_mut()
                                .find(|c| c.name == column_name)
                                .ok_or_else(|| {
                                Error::AlteredMissingColumn(
                                    table.name.clone(),
                                    column_name.value.clone(),
                                )
                            })?;

                            match op {
                                AlterColumnOperation::SetNotNull => {
                                    if column
                                        .options
                                        .iter()
                                        .find(|o| o.option == ColumnOption::NotNull)
                                        .is_none()
                                    {
                                        column.options.push(ColumnOptionDef {
                                            name: None,
                                            option: ColumnOption::NotNull,
                                        });
                                    }

                                    column.options.retain(|o| o.option != ColumnOption::Null);
                                }
                                AlterColumnOperation::DropNotNull => {
                                    column.options.retain(|o| o.option != ColumnOption::NotNull);
                                }
                                AlterColumnOperation::SetDefault { value } => {
                                    if let Some(default_option) = column
                                        .options
                                        .iter_mut()
                                        .find(|o| matches!(o.option, ColumnOption::Default(_)))
                                    {
                                        default_option.option = ColumnOption::Default(value);
                                    } else {
                                        column.options.push(ColumnOptionDef {
                                            name: None,
                                            option: ColumnOption::Default(value),
                                        })
                                    }
                                }
                                AlterColumnOperation::DropDefault => {
                                    column
                                        .options
                                        .retain(|o| !matches!(o.option, ColumnOption::Default(_)));
                                }

                                AlterColumnOperation::SetDataType { data_type, .. } => {
                                    column.data_type = data_type
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
            Statement::CreateView { name, columns, .. } => {
                self.create_view(
                    name.to_string(),
                    columns.into_iter().map(|c| c.value).collect(),
                )?;
            }

            Statement::Drop {
                object_type, names, ..
            } => {
                for name in names {
                    let name = name.to_string();
                    match object_type {
                        ObjectType::Table => {
                            self.tables.remove(&name);
                        }
                        ObjectType::View => {
                            self.views.remove(&name);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn apply_sql(&mut self, sql: &str) -> Result<(), Report<Error>> {
        sqlparser::parser::Parser::new(self.dialect.as_ref())
            .try_with_sql(sql)
            .change_context(Error::Parse)?
            .parse_statements()
            .change_context(Error::Parse)?
            .into_iter()
            .try_for_each(|statement| self.apply_statement(statement))
    }

    pub fn apply_file(&mut self, filename: &str) -> Result<(), Report<Error>> {
        let contents = std::fs::read_to_string(filename)
            .change_context(Error::File)
            .attach_printable_lazy(|| filename.to_string())?;

        self.apply_sql(&contents)
            .attach_printable_lazy(|| filename.to_string())
    }
}
