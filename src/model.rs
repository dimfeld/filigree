mod base_models;
mod generate_endpoints;
mod generate_sql;

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct Model {
    pub name: String,
    /// A prefix of a few characters for the ID of this type.
    pub id_prefix: String,
    pub fields: Vec<ModelField>,
    /// If true, generate API endpoints for this model.
    pub endpoints: bool,

    /// SQL to create indexes on the field
    #[serde(default)]
    pub indexes: Vec<String>,
}

impl Model {
    pub fn create_context(&self, model_index: usize, dialect: SqlDialect) -> tera::Context {
        let mut context = tera::Context::new();
        context.insert("table", &self.table());
        context.insert("indexes", &self.indexes);

        let fields = self
            .fields
            .iter()
            .map(|field| (false, field))
            .chain(
                self.fixed_fields(model_index)
                    .iter()
                    .map(|field| (true, field)),
            )
            .map(|(fixed, field)| {
                json!({
                    "name": field.name,
                    "sql_full_name": field.qualified_sql_field_name(),
                    "default": field.default,
                    "nullable": field.nullable,
                    "unique": field.unique,
                    "extra_sql_modifiers": field.extra_sql_modifiers,
                    "public_read": field.public.can_read(),
                    "public_write": field.public.can_write(),
                    "updatable": !fixed,
                })
            })
            .collect::<Vec<_>>();

        context
    }

    pub fn table(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    fn fixed_fields(&self, model_index: usize) -> Vec<ModelField> {
        vec![
            ModelField {
                name: "id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some(format!("ObjectId<{model_index}>")),
                nullable: false,
                unique: false,
                extra_sql_modifiers: "primary key".to_string(),
                public: Access::Read,
                default: "uuid_gen_v7()".to_string(),
            },
            ModelField {
                name: "updated_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                extra_sql_modifiers: String::new(),
                public: Access::Read,
                default: "now()".to_string(),
            },
            ModelField {
                name: "created_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                extra_sql_modifiers: String::new(),
                public: Access::Read,
                default: "now()".to_string(),
            },
        ]
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelField {
    /// The name of the field
    pub name: String,

    /// The SQL type for this field.
    #[serde(rename = "type")]
    pub typ: SqlType,
    /// The Rust type for this field. If omitted, the type will be inferred from the SQL
    /// type.
    pub rust_type: Option<String>,

    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub unique: bool,

    #[serde(default)]
    pub extra_sql_modifiers: String,

    /// Define how callers to the API can access this field
    #[serde(default)]
    pub public: Access,

    /// The default value of this field, as a SQL expression
    #[serde(default)]
    pub default: String,
}

impl ModelField {
    pub fn rust_field_name(&self) -> String {
        let base_name = match self.name.as_str() {
            "type" => "typ",
            _ => &self.name,
        };

        base_name.to_case(Case::Snake)
    }

    pub fn sql_field_name(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn qualified_sql_field_name(&self) -> String {
        let field_name = self.sql_field_name();
        if let Some(rust_type) = &self.rust_type {
            format!("{field_name}: {rust_type}")
        } else {
            field_name
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SqlDialect {
    Postgresql,
    SQLite,
}

#[derive(Serialize, Deserialize, Debug)]
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
#[derive(Serialize, Deserialize, Debug)]
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
        Self::None
    }
}

impl Access {
    fn can_read(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    fn can_write(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}
