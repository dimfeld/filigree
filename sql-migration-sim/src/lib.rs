//! This library is meant to parse multiple related SQL migration files, and calculate the final
//! schema that results from running them in order.
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
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::{Deref, DerefMut},
    path::Path,
};

use sqlparser::ast::{
    AlterColumnOperation, AlterIndexOperation, AlterTableOperation, ColumnDef, ColumnOption,
    ColumnOptionDef, Ident, ObjectName, ObjectType, Statement,
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

    /// Returns the default value of the column
    pub fn default_value(&self) -> Option<&ast::Expr> {
        self.options.iter().find_map(|o| match &o.option {
            ColumnOption::Default(expr) => Some(expr),
            _ => None,
        })
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
    /// The names of created indexes
    pub indices: HashSet<String>,
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
            indices: HashSet::new(),
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
                name: name_ident,
                operations,
                ..
            } => {
                let name = name_ident.to_string();
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
                            let mut table = self
                                .tables
                                .remove(&name)
                                .ok_or_else(|| Error::AlteredMissingTable(name.clone()))?;

                            let (schema, _) = object_schema_and_name(&name_ident);
                            let new_table_name = name_with_schema(schema, &new_table_name);

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
                        ObjectType::Index => {
                            self.indices.remove(&name);
                        }
                        _ => {}
                    }
                }
            }

            Statement::CreateIndex {
                name, table_name, ..
            } => {
                // For now we ignore indexes without names.
                if let Some(name) = name {
                    let (schema, _) = object_schema_and_name(&table_name);
                    let full_name = name_with_schema(schema, &name);
                    self.indices.insert(full_name);
                }
            }

            Statement::AlterIndex { name, operation } => match operation {
                AlterIndexOperation::RenameIndex { index_name } => {
                    self.indices.remove(&name.to_string());
                    let (schema, _) = object_schema_and_name(&name);
                    let new_name = name_with_schema(schema, index_name);
                    self.indices.insert(new_name);
                }
            },
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
    pub fn apply_file(&mut self, filename: &Path) -> Result<(), Error> {
        let contents = std::fs::read_to_string(filename).map_err(|e| Error::File {
            source: e,
            filename: filename.display().to_string(),
        })?;

        self.apply_sql(&contents)
    }
}

fn name_with_schema(schema: Option<impl Display>, name: impl Display) -> String {
    if let Some(schema) = schema {
        format!("{schema}.{name}")
    } else {
        name.to_string()
    }
}

fn object_schema_and_name(name: &ObjectName) -> (Option<&Ident>, &Ident) {
    if name.0.len() == 2 {
        (Some(&name.0[0]), &name.0[1])
    } else {
        (None, &name.0[0])
    }
}

#[cfg(test)]
mod test {
    use sqlparser::{ast::DataType, dialect};

    use super::*;

    const CREATE: &str = r##"
    CREATE TABLE ships (
        id BIGINT PRIMARY KEY,
        name TEXT NOT NULL,
        mast_count INT not null
    );"##;

    const CREATE_WITH_SCHEMA: &str = r##"
    CREATE TABLE sch.ships (
        id BIGINT PRIMARY KEY,
        name TEXT NOT NULL,
        mast_count INT not null
    );"##;

    #[test]
    fn rename_table() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("ALTER TABLE ships RENAME TO ships_2;")
            .unwrap();

        assert!(schema.tables.contains_key("ships_2"));
        assert!(!schema.tables.contains_key("ships"));
    }

    #[test]
    fn rename_table_with_schema() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE_WITH_SCHEMA).unwrap();
        schema
            .apply_sql("ALTER TABLE sch.ships RENAME TO ships_2;")
            .unwrap();

        assert!(schema.tables.contains_key("sch.ships_2"));
        assert!(!schema.tables.contains_key("sch.ships"));
    }

    #[test]
    fn create_index() {
        let mut schema = Schema::new();
        schema
            .apply_sql(
                "
            CREATE INDEX idx_name ON ships(name);
            CREATE INDEX idx_name_2 ON sch.ships(name);
        ",
            )
            .unwrap();

        assert!(schema.indices.contains("idx_name"));
        assert!(schema.indices.contains("sch.idx_name_2"));
    }

    #[test]
    fn drop_index() {
        let mut schema = Schema::new();
        schema
            .apply_sql("CREATE INDEX idx_name ON sch.ships(name);")
            .unwrap();

        schema.apply_sql("DROP INDEX sch.idx_name;").unwrap();

        assert!(schema.indices.is_empty());
    }

    #[test]
    fn add_column() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("ALTER TABLE ships ADD COLUMN has_motor BOOLEAN NOT NULL;")
            .unwrap();
        assert!(schema.tables["ships"].columns[3].name() == "has_motor");
    }

    #[test]
    fn drop_column() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("ALTER TABLE ships DROP COLUMN name;")
            .unwrap();
        assert!(schema.tables["ships"].columns.len() == 2);
        assert!(schema.tables["ships"]
            .columns
            .iter()
            .find(|c| c.name() == "name")
            .is_none());
    }

    #[test]
    fn rename_column() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("ALTER TABLE ships RENAME COLUMN mast_count TO mast_count_2;")
            .unwrap();
        assert!(schema.tables["ships"].columns[2].name() == "mast_count_2");
    }

    #[test]
    fn alter_column_change_nullable() {
        let mut schema = Schema::new_with_dialect(dialect::PostgreSqlDialect {});
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("ALTER TABLE ships ALTER COLUMN mast_count DROP NOT NULL")
            .unwrap();
        assert!(!schema.tables["ships"].columns[2].not_null());

        schema
            .apply_sql("ALTER TABLE ships ALTER COLUMN mast_count SET NOT NULL")
            .unwrap();
        assert!(schema.tables["ships"].columns[2].not_null());
    }

    #[test]
    fn alter_column_default() {
        let mut schema = Schema::new_with_dialect(dialect::PostgreSqlDialect {});
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("ALTER TABLE ships ALTER COLUMN mast_count SET DEFAULT 2")
            .unwrap();
        assert_eq!(
            schema.tables["ships"].columns[2]
                .default_value()
                .unwrap()
                .to_string(),
            "2"
        );

        schema
            .apply_sql("ALTER TABLE ships ALTER COLUMN mast_count DROP DEFAULT")
            .unwrap();
        assert!(schema.tables["ships"].columns[2].default_value().is_none());
    }

    #[test]
    fn alter_column_data_type() {
        let mut schema = Schema::new_with_dialect(dialect::PostgreSqlDialect {});
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql(
                "ALTER TABLE ships ALTER COLUMN mast_count TYPE JSON USING(mast_count::json);",
            )
            .unwrap();
        println!("{:?}", schema.tables["ships"].columns[2]);
        assert!(schema.tables["ships"].columns[2].data_type == DataType::JSON);
    }
}
