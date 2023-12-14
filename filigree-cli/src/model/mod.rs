mod base_models;
mod generate_endpoints;
mod generate_sql;

use std::borrow::Cow;

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

    /// If true, this model does not have a team_id.
    /// This mostly applies to the team object itself but may be useful for other things.
    #[serde(default)]
    pub global: bool,

    /// SQL to create indexes on the field
    #[serde(default)]
    pub indexes: Vec<String>,
    // TODO ability to define extra permissions
    // TODO ability to define extra operations that update specific things and require specific
    // permissions.
}
impl Model {
    pub fn create_template_context(&self, dialect: SqlDialect) -> tera::Context {
        let mut context = tera::Context::new();
        context.insert("table", &self.table());
        context.insert("indexes", &self.indexes);

        context.insert("global", &self.global);
        context.insert("owner_permission", &format!("{}::owner", self.id_prefix));
        context.insert("read_permission", &format!("{}::read", self.name));
        context.insert("write_permission", &format!("{}::write", self.id_prefix));

        let fields = self
            .all_fields()
            .map(|(fixed, field)| {
                json!({
                    "sql_name": field.sql_field_name(),
                    "sql_full_name": field.qualified_sql_field_name(),
                    "sql_type": field.typ.to_sql_type(dialect),
                    "rust_name": field.rust_field_name(),
                    "rust_type": field.rust_type.clone().unwrap_or_else(|| field.typ.to_rust_type().to_string()),
                    "default": field.default,
                    "nullable": field.nullable,
                    "unique": field.unique,
                    "extra_sql_modifiers": field.extra_sql_modifiers,
                    "user_read": field.user_access.can_read(),
                    "user_write": !fixed && field.user_access.can_write(),
                    "owner_read": field.owner_access.can_read() || field.user_access.can_read(),
                    "owner_write": !fixed && (field.owner_access.can_write() || field.user_access.can_write()),
                    "updatable": !fixed,
                })
            })
            .collect::<Vec<_>>();

        context.insert("fields", &fields);
        context
    }

    pub fn table(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn object_id_type(&self) -> String {
        format!("{}Id", self.id_prefix.to_case(Case::Camel))
    }

    pub fn all_fields(&self) -> impl Iterator<Item = (bool, Cow<ModelField>)> {
        self.standard_fields()
            .map(|field| (true, Cow::Owned(field)))
            .chain(
                self.fields
                    .iter()
                    .map(|field| (false, Cow::Borrowed(field))),
            )
    }

    /// The fields that apply to every object
    fn standard_fields(&self) -> impl Iterator<Item = ModelField> {
        let team_field = if self.global {
            None
        } else {
            Some(ModelField {
                name: "team_id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some("TeamId".to_string()),
                nullable: false,
                unique: false,
                indexed: false,
                extra_sql_modifiers: "primary key".to_string(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default: String::new(),
            })
        };

        [
            Some(ModelField {
                name: "id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some(self.object_id_type()),
                nullable: false,
                unique: false,
                indexed: false,
                extra_sql_modifiers: "primary key".to_string(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default: String::new(),
            }),
            team_field,
            Some(ModelField {
                name: "updated_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                indexed: false,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default: "now()".to_string(),
            }),
            Some(ModelField {
                name: "created_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                indexed: false,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default: "now()".to_string(),
            }),
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub user_access: Access,

    /// Define how owners on this object can access the field
    /// Allthough this defaults to [Access::None], it is effectively always
    /// at least as permissive as [user_access].
    #[serde(default)]
    pub owner_access: Access,

    /// The default value of this field, as a SQL expression
    #[serde(default)]
    pub default: String,

    /// If true, create an index on this field.
    /// More exotic index types can be specified using [Model#indexes].
    #[serde(default)]
    pub indexed: bool,
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

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SqlDialect {
    Postgresql,
    SQLite,
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
