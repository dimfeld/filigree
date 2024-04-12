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
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::Path,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    AlterColumnOperation, AlterIndexOperation, AlterTableOperation, ColumnDef, ColumnOption,
    ColumnOptionDef, CreateFunctionBody, DataType, Ident, ObjectName, ObjectType,
    OperateFunctionArg, Statement, TableConstraint,
};
pub use sqlparser::{ast, dialect};

/// A column in a database table
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
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
                ColumnOption::Unique { is_primary, .. } => is_primary.then_some(true),
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

/// A function in the database
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The name of the function
    pub name: ObjectName,
    /// The arguments of the function
    pub args: Option<Vec<OperateFunctionArg>>,
    /// The return type of the function
    pub return_type: Option<DataType>,
    /// The options and body of the function
    pub params: CreateFunctionBody,
}

/// A table in the database
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Table {
    /// The name of the table
    pub name: ObjectName,
    /// The columns in the table
    pub columns: Vec<Column>,
    /// Constraints on this table
    pub constraints: Vec<TableConstraint>,
}

impl Table {
    /// The name of the table
    pub fn name(&self) -> String {
        self.name.to_string()
    }
}

/// A view in the database
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct View {
    /// The name of the view
    pub name: ObjectName,
    /// The columns in the view
    pub columns: Vec<String>,
}

impl View {
    /// The name of the view
    pub fn name(&self) -> String {
        self.name.to_string()
    }
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
    /// Attempted to create a function that already exists
    #[error("Attempted to create function {0} that already exists")]
    FunctionAlreadyExists(String),
    /// Attempted to create a column that already exists
    #[error("Attempted to create column {0} that already exists in table {1}")]
    ColumnAlreadyExists(String, String),
    /// Attempted to rename an index that doesn't exist
    #[error("Attempted to rename index {0} that does not exist")]
    RenameMissingIndex(String),
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Schema {
    #[cfg_attr(feature = "serde", serde(skip, default = "default_dialect"))]
    /// The SQL dialect used for pasing
    pub dialect: Box<dyn dialect::Dialect>,
    /// The tables in the schema
    pub tables: HashMap<String, Table>,
    /// The views in the schema
    pub views: HashMap<String, View>,
    /// The created indices. The key is the index name and the value is the table the index is on.
    pub indices: HashMap<String, String>,
    /// Functions in the schema
    pub functions: HashMap<String, Function>,
    /// References to the schema objects, in the order they were created.
    pub creation_order: Vec<ObjectNameAndType>,
}

#[cfg(feature = "serde")]
fn default_dialect() -> Box<dyn dialect::Dialect> {
    Box::new(dialect::GenericDialect {})
}

/// An object and its type
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectNameAndType {
    /// The name of the object
    pub name: String,
    /// The type of object this is
    pub object_type: SchemaObjectType,
}

/// The type of an object in the [Schema]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaObjectType {
    /// A SQL table
    Table,
    /// A view
    View,
    /// An index
    Index,
    /// A Function
    Function,
}

impl std::fmt::Display for SchemaObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaObjectType::Table => write!(f, "table"),
            SchemaObjectType::View => write!(f, "view"),
            SchemaObjectType::Index => write!(f, "index"),
            SchemaObjectType::Function => write!(f, "function"),
        }
    }
}

impl Schema {
    /// Create a new [Schema] that parses with a generic SQL dialect
    pub fn new() -> Self {
        Self::new_with_dialect(sqlparser::dialect::GenericDialect {})
    }

    /// Create a new [Schema] that parses with the given SQL dialect
    pub fn new_with_dialect<D: dialect::Dialect>(dialect: D) -> Self {
        let dialect = Box::new(dialect);
        Self {
            tables: HashMap::new(),
            views: HashMap::new(),
            indices: HashMap::new(),
            functions: HashMap::new(),
            creation_order: Vec::new(),
            dialect,
        }
    }

    fn create_table(&mut self, name: ObjectName, columns: Vec<ColumnDef>) -> Result<(), Error> {
        let name_str = name.to_string();
        if self.tables.contains_key(&name_str) {
            return Err(Error::TableAlreadyExists(name_str));
        }

        self.tables.insert(
            name_str.clone(),
            Table {
                name,
                columns: columns.into_iter().map(Column).collect(),
                constraints: Vec::new(),
            },
        );

        self.creation_order.push(ObjectNameAndType {
            name: name_str,
            object_type: SchemaObjectType::Table,
        });

        Ok(())
    }

    fn create_view(
        &mut self,
        name: ObjectName,
        or_replace: bool,
        columns: Vec<String>,
    ) -> Result<(), Error> {
        let name_str = name.to_string();
        if !or_replace && self.views.contains_key(&name_str) {
            return Err(Error::TableAlreadyExists(name_str));
        }

        self.views.insert(name_str.clone(), View { name, columns });

        self.creation_order.push(ObjectNameAndType {
            name: name_str,
            object_type: SchemaObjectType::View,
        });

        Ok(())
    }

    fn create_function(
        &mut self,
        name: ObjectName,
        or_replace: bool,
        args: Option<Vec<OperateFunctionArg>>,
        return_type: Option<DataType>,
        params: CreateFunctionBody,
    ) -> Result<(), Error> {
        let name_str = name.to_string();
        if !or_replace && self.functions.contains_key(&name_str) {
            return Err(Error::TableAlreadyExists(name_str));
        }

        self.functions.insert(
            name_str.clone(),
            Function {
                name,
                args,
                return_type,
                params,
            },
        );

        self.creation_order.push(ObjectNameAndType {
            name: name_str,
            object_type: SchemaObjectType::Function,
        });

        Ok(())
    }

    fn handle_alter_table(
        &mut self,
        name: &str,
        name_ident: &ObjectName,
        operation: AlterTableOperation,
    ) -> Result<(), Error> {
        match operation {
            AlterTableOperation::AddColumn {
                if_not_exists,
                column_def,
                ..
            } => {
                let table = self
                    .tables
                    .get_mut(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;

                let existing_column = table.columns.iter().find(|c| c.name == column_def.name);

                if existing_column.is_none() {
                    table.columns.push(Column(column_def));
                } else if !if_not_exists {
                    return Err(Error::ColumnAlreadyExists(
                        column_def.name.value,
                        name.to_string(),
                    ));
                }
            }

            AlterTableOperation::DropColumn { column_name, .. } => {
                let table = self
                    .tables
                    .get_mut(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;
                table.columns.retain(|c| c.name != column_name);
            }

            AlterTableOperation::RenameColumn {
                old_column_name,
                new_column_name,
            } => {
                let table = self
                    .tables
                    .get_mut(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;

                let column = table
                    .columns
                    .iter_mut()
                    .find(|c| c.name == old_column_name)
                    .ok_or_else(|| {
                        Error::AlteredMissingColumn(old_column_name.value.clone(), name.to_string())
                    })?;
                column.name = new_column_name;
            }

            AlterTableOperation::RenameTable {
                table_name: new_table_name,
            } => {
                let mut table = self
                    .tables
                    .remove(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;

                let (schema, _) = object_schema_and_name(&name_ident);
                let (_, new_table_name) = object_schema_and_name(&new_table_name);
                let new_table_name = name_with_schema(schema.cloned(), new_table_name.clone());

                let new_name_str = new_table_name.to_string();
                table.name = new_table_name;

                self.tables.insert(new_name_str.clone(), table);
                // Update the name in creation_order to match
                if let Some(i) = self
                    .creation_order
                    .iter_mut()
                    .find(|o| o.name == name && o.object_type == SchemaObjectType::Table)
                {
                    i.name = new_name_str;
                }
            }

            AlterTableOperation::AlterColumn { column_name, op } => {
                let table = self
                    .tables
                    .get_mut(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;

                let column = table
                    .columns
                    .iter_mut()
                    .find(|c| c.name == column_name)
                    .ok_or_else(|| {
                        Error::AlteredMissingColumn(
                            table.name.to_string(),
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
                    _ => {}
                }
            }

            AlterTableOperation::AddConstraint(c) => {
                let table = self
                    .tables
                    .get_mut(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;

                table.constraints.push(c);
            }

            AlterTableOperation::DropConstraint {
                name: constraint_name,
                ..
            } => {
                let table = self
                    .tables
                    .get_mut(name)
                    .ok_or_else(|| Error::AlteredMissingTable(name.to_string()))?;

                table.constraints.retain(|c| {
                    let name = table_constraint_name(c);
                    name.as_ref().map(|n| n != &constraint_name).unwrap_or(true)
                });
            }

            _ => {}
        }

        Ok(())
    }

    /// Apply a parsed statement to the schema
    pub fn apply_statement(&mut self, statement: Statement) -> Result<(), Error> {
        match statement {
            Statement::CreateTable { name, columns, .. } => {
                self.create_table(name, columns)?;
            }
            Statement::AlterTable {
                name: name_ident,
                operations,
                ..
            } => {
                let name = name_ident.to_string();
                for operation in operations {
                    self.handle_alter_table(&name, &name_ident, operation)?;
                }
            }
            Statement::CreateView {
                name,
                columns,
                or_replace,
                ..
            } => {
                self.create_view(
                    name,
                    or_replace,
                    columns.into_iter().map(|c| c.name.value).collect(),
                )?;
            }

            Statement::CreateFunction {
                or_replace,
                temporary,
                name,
                args,
                return_type,
                params,
            } => {
                if !temporary {
                    self.create_function(name, or_replace, args, return_type, params)?;
                }
            }

            Statement::Drop {
                object_type, names, ..
            } => {
                for name in names {
                    let name = name.to_string();
                    match object_type {
                        ObjectType::Table => {
                            self.tables.remove(&name);
                            self.creation_order.retain(|c| {
                                c.object_type != SchemaObjectType::Table || c.name != name
                            });
                        }
                        ObjectType::View => {
                            self.views.remove(&name);
                            self.creation_order.retain(|c| {
                                c.object_type != SchemaObjectType::View || c.name != name
                            });
                        }
                        ObjectType::Index => {
                            self.indices.remove(&name);
                            self.creation_order.retain(|c| {
                                c.object_type != SchemaObjectType::Index || c.name != name
                            });
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
                    let (_, name) = object_schema_and_name(&name);
                    let full_name = name_with_schema(schema.cloned(), name.clone());
                    self.indices
                        .insert(full_name.to_string(), table_name.to_string());
                    self.creation_order.push(ObjectNameAndType {
                        name: full_name.to_string(),
                        object_type: SchemaObjectType::Index,
                    });
                }
            }

            Statement::AlterIndex { name, operation } => {
                match operation {
                    AlterIndexOperation::RenameIndex { index_name } => {
                        let Some(table_name) = self.indices.remove(&name.to_string()) else {
                            return Err(Error::RenameMissingIndex(name.to_string()));
                        };

                        let (schema, _) = object_schema_and_name(&name);
                        let (_, index_name) = object_schema_and_name(&index_name);
                        let new_name = name_with_schema(schema.cloned(), index_name.clone());
                        let new_name = new_name.to_string();
                        let old_name = name.to_string();
                        self.indices.insert(new_name.clone(), table_name);

                        // Update the name in creation_order to match
                        if let Some(i) = self.creation_order.iter_mut().find(|o| {
                            o.name == old_name && o.object_type == SchemaObjectType::Index
                        }) {
                            i.name = new_name;
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Parse some SQL into a list of statements
    pub fn parse_sql(&self, sql: &str) -> Result<Vec<Statement>, Error> {
        sqlparser::parser::Parser::new(self.dialect.as_ref())
            .try_with_sql(sql)?
            .parse_statements()
            .map_err(Error::from)
    }

    /// Apply one or more SQL statements to the schema
    pub fn apply_sql(&mut self, sql: &str) -> Result<(), Error> {
        self.parse_sql(sql)?
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

fn name_with_schema(schema: Option<Ident>, name: Ident) -> ObjectName {
    if let Some(schema) = schema {
        ObjectName(vec![schema, name])
    } else {
        ObjectName(vec![name])
    }
}

fn object_schema_and_name(name: &ObjectName) -> (Option<&Ident>, &Ident) {
    if name.0.len() == 2 {
        (Some(&name.0[0]), &name.0[1])
    } else {
        (None, &name.0[0])
    }
}

/// Get the name of a table constraint
pub fn table_constraint_name(constraint: &TableConstraint) -> &Option<Ident> {
    match constraint {
        TableConstraint::Unique { name, .. } => name,
        TableConstraint::PrimaryKey { name, .. } => name,
        TableConstraint::ForeignKey { name, .. } => name,
        TableConstraint::Check { name, .. } => name,
        TableConstraint::Index { name, .. } => name,
        TableConstraint::FulltextOrSpatial { .. } => &None,
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
        assert_eq!(
            schema.creation_order,
            vec![ObjectNameAndType {
                name: "ships_2".to_string(),
                object_type: SchemaObjectType::Table
            }]
        );
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
        assert_eq!(
            schema.creation_order,
            vec![ObjectNameAndType {
                name: "sch.ships_2".to_string(),
                object_type: SchemaObjectType::Table
            }]
        );
    }

    #[test]
    fn drop_table() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE).unwrap();
        schema.apply_sql(CREATE_WITH_SCHEMA).unwrap();
        schema.apply_sql("DROP TABLE ships").unwrap();

        assert!(!schema.tables.contains_key("ships"));
        assert!(schema.tables.contains_key("sch.ships"));
        assert_eq!(
            schema.creation_order,
            vec![ObjectNameAndType {
                name: "sch.ships".to_string(),
                object_type: SchemaObjectType::Table
            }]
        );
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

        assert_eq!(schema.indices.get("idx_name").unwrap(), "ships");
        assert_eq!(schema.indices.get("sch.idx_name_2").unwrap(), "sch.ships");
        assert_eq!(
            schema.creation_order,
            vec![
                ObjectNameAndType {
                    name: "idx_name".to_string(),
                    object_type: SchemaObjectType::Index
                },
                ObjectNameAndType {
                    name: "sch.idx_name_2".to_string(),
                    object_type: SchemaObjectType::Index
                },
            ]
        );
    }

    #[test]
    fn drop_index() {
        let mut schema = Schema::new();
        schema.apply_sql(CREATE).unwrap();
        schema
            .apply_sql("CREATE INDEX idx_name ON sch.ships(name);")
            .unwrap();

        schema.apply_sql("DROP INDEX sch.idx_name;").unwrap();

        assert!(schema.indices.is_empty());
        assert_eq!(
            schema.creation_order,
            vec![ObjectNameAndType {
                name: "ships".to_string(),
                object_type: SchemaObjectType::Table
            }]
        );
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
