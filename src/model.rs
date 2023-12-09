mod base_models;
mod generate_endpoints;
mod generate_sql;

use convert_case::{Case, Casing};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Model {
    pub name: String,
    // A prefix of a few characters for the ID of this type.
    pub id_prefix: String,
    pub fields: Vec<ModelField>,
    /// If true, generate API endpoints for this model.
    pub endpoints: bool,
}

#[derive(Deserialize, Debug)]
pub struct ModelField {
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

    /// SQL to create indexes on the field
    #[serde(default)]
    pub indexes: Vec<String>,

    /// If true, this field should be returned from the API.
    #[serde(default)]
    pub public: bool,

    /// The default value of this field
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
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SqlDialect {
    Postgresql,
    SQLite,
}

#[derive(Deserialize, Debug)]
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
