//! This library is meant to parse multiple related SQL migration files, and calculate the final
//! schema that resutls from running them in order.
//!
//! ## Example
//!
//! ```
//! use sql_migration_sim::{Schema, Error, ast::DataType};
//!
//! let mut schema = Schema::new();
//!
//! let create_statement = r##"CREATE TABLE ships (
//!    id BIGINT PRIMARY KEY,
//!    name TEXT NOT NULL,
//!    mast_count INT not null
//! );"##;
//!
//! let alter = r##"
//!     ALTER TABLE ships ALTER COLUMN mast_count DROP NOT NULL;
//!     ALTER TABLE ships ADD COLUMN has_motor BOOLEAN NOT NULL;
//!     "##;
//!
//! schema.apply_sql(create_statement)?;
//! schema.apply_sql(alter)?;
//!
//!
//! let result = schema.tables.get("ships").unwrap();
//! assert_eq!(result.columns.len(), 4);
//! assert_eq!(result.columns[0].name(), "id");
//! assert!(matches!(result.columns[0].data_type, DataType::BigInt(_)));
//! assert_eq!(result.columns[0].not_null(), true);
//! assert_eq!(result.columns[1].name(), "name");
//! assert_eq!(result.columns[1].not_null(), true);
//! assert_eq!(result.columns[2].name(), "mast_count");
//! assert_eq!(result.columns[2].not_null(), false);
//! assert_eq!(result.columns[3].name(), "has_motor");
//! assert_eq!(result.columns[3].not_null(), true);
//!
//! # Ok::<(), Error>(())
//!
//! ```
//!

#![warn(missing_docs)]
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use sqlparser::ast::{
    AlterColumnOperation, AlterTableOperation, ColumnDef, ColumnOption, ColumnOptionDef,
    ObjectType, Statement,
};
pub use sqlparser::{ast, dialect::Dialect};

/// A column in a database table
#[derive(Debug)]
pub struct Column(pub ColumnDef);

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
    /// The name of the column
    pub fn name(&self) -> &str {
        self.name.value.as_str()
    }

    /// Whether the column is nullable or not
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

/// A table in the database
pub struct Table {
    /// The name of the table
    pub name: String,
    /// The columns in the table
    pub columns: Vec<Column>,
}

/// A view in the database
pub struct View {
    /// The name of the view
    pub name: String,
    /// The columns in the view
    pub columns: Vec<String>,
}

/// Errors that can occur while parsing SQL and updating the schema
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Encountered an ALTER TABLE statement on a nonexistent table.
    #[error("Attempted to alter a table {0} that does not exist")]
    AlteredMissingTable(String),
    /// Encountered an ALTER COLUMN statement on a nonexistent column.
    #[error("Attempted to alter a column {0} that does not exist in table {1}")]
    AlteredMissingColumn(String, String),
    /// Attempted to create a table that already exists
    #[error("Attempted to create table {0} that already exists")]
    TableAlreadyExists(String),
    /// Attempted to create a column that already exists
    #[error("Attempted to create column {0} that already exists in table {1}")]
    ColumnAlreadyExists(String, String),
    /// The SQL parser encountered an error
    #[error("SQL Parse Error {0}")]
    Parse(#[from] sqlparser::parser::ParserError),
    /// Error reading a file
    #[error("Failed to read file {filename}")]
    File {
        /// The underlying error
        #[source]
        source: std::io::Error,
        /// The name of the file on which the error occurred
        filename: String,
    },
}

/// The database schema, built from parsing one or more SQL statements.
pub struct Schema {
    dialect: Box<dyn Dialect>,
    /// The tables in the schema
    pub tables: HashMap<String, Table>,
    /// The views in the schema
    pub views: HashMap<String, View>,
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

    fn create_view(
        &mut self,
        name: String,
        or_replace: bool,
        columns: Vec<String>,
    ) -> Result<(), Error> {
        if !or_replace && self.views.contains_key(&name) {
            return Err(Error::TableAlreadyExists(name));
        }

        self.views.insert(name.clone(), View { name, columns });

        Ok(())
    }

    fn apply_statement(&mut self, statement: Statement) -> Result<(), Error> {
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
                                return Err(Error::ColumnAlreadyExists(
                                    column_def.name.value,
                                    name.clone(),
                                ));
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
            Statement::CreateView {
                name,
                columns,
                or_replace,
                ..
            } => {
                self.create_view(
                    name.to_string(),
                    or_replace,
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

    /// Apply one or more SQL statements to the schema
    pub fn apply_sql(&mut self, sql: &str) -> Result<(), Error> {
        sqlparser::parser::Parser::new(self.dialect.as_ref())
            .try_with_sql(sql)?
            .parse_statements()?
            .into_iter()
            .try_for_each(|statement| self.apply_statement(statement))
    }

    /// Read a SQL file and apply its contents to the schema
    pub fn apply_file(&mut self, filename: &str) -> Result<(), Error> {
        let contents = std::fs::read_to_string(filename).map_err(|e| Error::File {
            source: e,
            filename: filename.to_string(),
        })?;

        self.apply_sql(&contents)
    }
}
