use std::borrow::Cow;

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};

use super::SqlDialect;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelField {
    /// The name of the field
    pub name: String,

    /// The SQL type for this field.
    #[serde(rename = "type")]
    pub typ: SqlType,
    /// The Rust type for this field. If omitted, the type will be inferred from the SQL
    /// type. This type should be fully qualified, e.g. `crate::MyType`.
    pub rust_type: Option<String>,

    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub unique: bool,

    #[serde(default)]
    pub extra_sql_modifiers: String,

    /// Define how callers to the API can access this field. This is still gated on the user having
    /// the relevant read or write permission.
    #[serde(default)]
    pub user_access: Access,

    /// Define how owners on this object can access the field
    /// This is always at least as permissive as [user_access].
    #[serde(default)]
    pub owner_access: Access,

    /// The default value of this field, as a SQL expression. This requires a migration to change.
    #[serde(default)]
    pub default_sql: String,

    /// The default value of the field, as a Rust expression. This can be useful for expressing
    /// more complex types, or values that may need to change regularly and so are less suited for
    /// migrations.
    #[serde(default)]
    pub default_rust: String,

    /// If true, create an index on this field.
    /// More exotic index types can be specified using [Model#indexes].
    #[serde(default)]
    pub indexed: bool,

    /// If true, allow filtering on this field in the list endpoint's query string
    #[serde(default)]
    pub filterable: FilterableType,

    #[serde(default)]
    pub sortable: SortableType,

    /// A field in another model that this field references. This sets up a foreign
    /// key in the SQL definition.
    pub references: Option<ModelFieldReference>,
}

impl ModelField {
    pub fn rust_field_name(&self) -> String {
        let base_name = match self.name.as_str() {
            "type" => "typ",
            _ => &self.name,
        };

        base_name.to_case(Case::Snake)
    }

    /// The type of this field.
    pub fn base_rust_type(&self) -> &str {
        self.rust_type
            .as_deref()
            .unwrap_or_else(|| self.typ.to_rust_type())
    }

    /// The type of this field, wrapped in an Option if [nullable] is true.
    pub fn rust_type(&self) -> Cow<str> {
        if self.nullable {
            format!("Option<{}>", self.base_rust_type()).into()
        } else {
            self.base_rust_type().into()
        }
    }

    pub fn sql_field_name(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn qualified_sql_field_name(&self) -> String {
        let field_name = self.sql_field_name();
        if let Some(rust_type) = &self.rust_type {
            // If the type is different from the default SQL type, specify it explicitly.
            // Don't add Option like self.rust_type() does because sqlx will do that itself.
            format!(r##"{field_name} as "{field_name}: {rust_type}""##)
        } else {
            field_name
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelFieldReference {
    table: String,
    field: String,
    delete_behavior: DeleteBehavior,
}

impl ModelFieldReference {
    pub fn new(
        table: impl Into<String>,
        field: impl Into<String>,
        delete_behavior: DeleteBehavior,
    ) -> Self {
        Self {
            table: table.into(),
            field: field.into(),
            delete_behavior,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DeleteBehavior {
    Ignore,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SqlType {
    Text,
    Int,
    BigInt,
    Uuid,
    Float,
    Boolean,
    Json,
    Timestamp,
}

impl SqlType {
    pub fn to_rust_type(&self) -> &'static str {
        match self {
            SqlType::Text => "String",
            SqlType::Int => "i64",
            SqlType::BigInt => "i64",
            SqlType::Float => "f64",
            SqlType::Boolean => "bool",
            SqlType::Json => "serde_json::Value",
            SqlType::Timestamp => "chrono::DateTime<chrono::Utc>",
            SqlType::Uuid => "uuid::Uuid",
        }
    }

    pub fn to_sql_type(&self, dialect: SqlDialect) -> &'static str {
        match (self, dialect) {
            (SqlType::Text, _) => "TEXT",
            (SqlType::Int, _) => "INTEGER",
            (SqlType::BigInt, SqlDialect::Postgresql) => "BIGINT",
            (SqlType::BigInt, SqlDialect::SQLite) => "INTEGER",
            (SqlType::Float, _) => "DOUBLE PRECISION",
            (SqlType::Boolean, SqlDialect::Postgresql) => "BOOLEAN",
            (SqlType::Boolean, SqlDialect::SQLite) => "INTEGER",
            (SqlType::Json, SqlDialect::Postgresql) => "JSONB",
            (SqlType::Json, SqlDialect::SQLite) => "JSON",
            (SqlType::Timestamp, SqlDialect::Postgresql) => "TIMESTAMPTZ",
            (SqlType::Timestamp, SqlDialect::SQLite) => "INTEGER",
            (SqlType::Uuid, SqlDialect::Postgresql) => "UUID",
            (SqlType::Uuid, SqlDialect::SQLite) => "BLOB",
        }
    }
}

/// Define how callers to the API can access this field
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// No access
    None,
    /// Only read
    Read,
    /// Only write
    Write,
    /// Read and write
    ReadWrite,
}

impl Default for Access {
    fn default() -> Self {
        Self::ReadWrite
    }
}

impl Access {
    pub fn can_read(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    pub fn can_write(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum FilterableType {
    #[default]
    None,
    Exact,
    Range,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortableType {
    #[default]
    None,
    DefaultAscending,
    DefaultDescending,
    AscendingOnly,
    DescendingOnly,
}
