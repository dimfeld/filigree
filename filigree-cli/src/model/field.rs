use std::{borrow::Cow, fmt::Display};

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::SqlDialect;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ModelField {
    /// The name of the field
    pub name: String,

    /// A label to be used when displaying this field. If omitted, `name` will be used, converted
    /// to Title Case.
    pub label: Option<String>,

    /// A description of the field
    pub description: Option<String>,

    /// If this field was renamed, this is the old name of the field. This helps with
    /// creating migrations, so that the column can be renamed instead of deleted and recreated.
    pub previous_name: Option<String>,

    /// The SQL type for this field.
    #[serde(rename = "type")]
    pub typ: SqlType,
    /// The Rust type for this field. If omitted, the type will be inferred from the SQL
    /// type. This type should be fully qualified, e.g. `crate::MyType`.
    pub rust_type: Option<String>,
    /// The Typescript type for this field. If omitted, the type will be inferred from the SQL
    /// type.
    pub zod_type: Option<String>,

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
    ///
    /// For true parent-child relationships, you may prefer to use `has` and `belongs_to` in the
    /// model definitions.
    pub references: Option<ModelFieldReference>,

    /// Fields such as `updated_at` which are fixed for each model and can not be updated.
    /// Fields defined in the config should not set this.
    #[doc(hidden)]
    #[serde(skip, default)]
    pub fixed: bool,

    /// Never read this field from the database in autogenerated queries. This is for sensitive
    /// data which should not even be read in normal queries.
    ///
    /// This setting only controls the generated Rust code, and so is not a substitute for real
    /// security measures around sensitive data.
    #[serde(default)]
    pub never_read: bool,
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

    /// The Typescript type of this field.
    pub fn base_zod_type(&self) -> &str {
        self.zod_type
            .as_deref()
            .unwrap_or_else(|| self.typ.to_zod_type())
    }

    /// The Typescript type of this field accounting for nullability
    pub fn zod_type(&self) -> Cow<str> {
        if self.nullable {
            format!("{}.optional()", self.base_zod_type()).into()
        } else {
            self.base_zod_type().into()
        }
    }

    pub fn sql_field_name(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn previous_sql_field_name(&self) -> Option<String> {
        self.previous_name.as_ref().map(|s| s.to_case(Case::Snake))
    }

    pub fn qualified_sql_field_name(&self) -> String {
        let field_name = self.sql_field_name();
        let rust_name = self.rust_field_name();
        if let Some(rust_type) = &self.rust_type {
            // If the type is different from the default SQL type, specify it explicitly.
            // Don't add Option like self.rust_type() does because sqlx will do that itself.
            format!(r##"{field_name} as "{rust_name}: {rust_type}""##)
        } else if rust_name != field_name {
            format!(r##"{field_name} as "{rust_name}""##)
        } else {
            field_name
        }
    }

    pub fn user_read(&self) -> bool {
        self.user_access.can_read() && !self.never_read
    }

    pub fn user_write(&self) -> bool {
        !self.fixed && self.user_access.can_write() && !self.never_read
    }

    pub fn owner_read(&self) -> bool {
        (self.owner_access.can_read() || self.user_access.can_read()) && !self.never_read
    }

    pub fn owner_write(&self) -> bool {
        !self.fixed
            && !self.never_read
            && (self.owner_access.can_write() || self.user_access.can_write())
    }

    pub fn template_context(&self) -> serde_json::Value {
        json!({
            "name": self.name,
            "label": self.label.clone().unwrap_or_else(|| self.name.to_case(Case::Title)),
            "description": self.description.as_deref().unwrap_or_default(),
            "base_type": self.typ,
            "sql_name": self.sql_field_name(),
            "sql_full_name": self.qualified_sql_field_name(),
            "sql_type": self.typ.to_sql_type(SqlDialect::Postgresql),
            "snake_case_name": self.name.to_case(Case::Snake),
            "pascal_case_name": self.name.to_case(Case::Pascal),
            "rust_name": self.rust_field_name(),
            "base_rust_type": self.base_rust_type(),
            "rust_type": self.rust_type(),
            "is_custom_rust_type": self.rust_type.is_some(),
            "client_type": self.typ.to_client_type(),
            "default_sql": self.default_sql,
            "default_rust": self.default_rust,
            "nullable": self.nullable,
            "filterable": self.filterable,
            "sortable": self.sortable,
            "unique": self.unique,
            "indexed": self.indexed,
            "foreign_key_sql": self.references.as_ref().map(|r| r.to_string()),
            "extra_sql_modifiers": self.extra_sql_modifiers,
            "user_read": self.user_read(),
            "user_write": self.user_write(),
            "owner_read": self.owner_read(),
            "owner_write": self.owner_write(),
            "never_read": self.never_read,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelFieldReference {
    pub table: String,
    pub field: String,
    pub on_delete: Option<ReferentialAction>,
    pub on_update: Option<ReferentialAction>,
    pub deferrable: Option<Deferrable>,

    pub populate: Option<ReferencePopulation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReferencePopulation {
    pub on_get: bool,
    pub on_list: bool,
    /// The name of the structure member in which the populated data will be stored.
    /// If not specified, the name of the referencing field plus "_data" will be used.
    pub field_name: Option<String>,
    pub model: String,
}

impl ModelFieldReference {
    pub fn new(
        table: impl Into<String>,
        field: impl Into<String>,
        on_delete: Option<ReferentialAction>,
    ) -> Self {
        Self {
            table: table.into(),
            field: field.into(),
            on_delete,
            on_update: None,
            deferrable: None,
            populate: None,
        }
    }

    pub fn with_deferrable(mut self, deferrable: Deferrable) -> Self {
        self.deferrable = Some(deferrable);
        self
    }
}

impl Display for ModelFieldReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "REFERENCES {} ({})", self.table, self.field)?;

        if let Some(delete_behavior) = self.on_delete {
            write!(f, "ON DELETE {}", delete_behavior)?;
        }

        if let Some(update_behavior) = self.on_update {
            write!(f, "ON UPDATE {}", update_behavior)?;
        }

        if let Some(deferrable) = self.deferrable {
            write!(f, " {}", deferrable)?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ReferentialAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

impl Display for ReferentialAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferentialAction::NoAction => write!(f, "NO ACTION"),
            ReferentialAction::Restrict => write!(f, "RESTRICT"),
            ReferentialAction::Cascade => write!(f, "CASCADE"),
            ReferentialAction::SetNull => write!(f, "SET NULL"),
            ReferentialAction::SetDefault => write!(f, "SET DEFAULT"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Deferrable {
    NotDeferrable,
    InitiallyDeferred,
    InitiallyImmediate,
}

impl Display for Deferrable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Deferrable::NotDeferrable => write!(f, "NOT DEFERRABLE"),
            Deferrable::InitiallyDeferred => write!(f, "DEFERRABLE INITIALLY DEFERRED"),
            Deferrable::InitiallyImmediate => write!(f, "DEFERRABLE INITIALLY IMMEDIATE"),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SqlType {
    #[default]
    Text,
    Int,
    BigInt,
    Uuid,
    Float,
    Boolean,
    Json,
    Timestamp,
    Date,
    Bytes,
}

impl SqlType {
    /// Convert the type to its Rust equivalent
    pub fn to_rust_type(&self) -> &'static str {
        match self {
            SqlType::Text => "String",
            SqlType::Int => "i32",
            SqlType::BigInt => "i64",
            SqlType::Float => "f64",
            SqlType::Boolean => "bool",
            SqlType::Json => "serde_json::Value",
            SqlType::Timestamp => "chrono::DateTime<chrono::Utc>",
            SqlType::Date => "chrono::NaiveDate",
            SqlType::Uuid => "uuid::Uuid",
            SqlType::Bytes => "Vec<u8>",
        }
    }

    /// Convert the type to its ModelField Typescript value
    pub fn to_client_type(&self) -> &'static str {
        match self {
            SqlType::Text => "text",
            SqlType::Int => "integer",
            SqlType::BigInt => "integer",
            SqlType::Float => "float",
            SqlType::Boolean => "boolean",
            SqlType::Json => "object",
            SqlType::Timestamp => "date-time",
            SqlType::Date => "date",
            SqlType::Uuid => "uuid",
            SqlType::Bytes => "blob",
        }
    }

    /// Convert the type to its Zod equivalent
    pub fn to_zod_type(&self) -> &'static str {
        match self {
            SqlType::Text => "z.string()",
            SqlType::Int => "z.number().int()",
            SqlType::BigInt => "z.number().int()",
            SqlType::Float => "z.number()",
            SqlType::Boolean => "z.boolean()",
            SqlType::Json => "z.any()",
            SqlType::Timestamp => "z.string().datetime()",
            SqlType::Date => "z.string()",
            SqlType::Uuid => "z.string().uuid()",
            SqlType::Bytes => "z.string()",
        }
    }

    /// Convert the type to its SQL equivalent
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
            (SqlType::Date, SqlDialect::Postgresql) => "DATE",
            (SqlType::Date, SqlDialect::SQLite) => "INTEGER",
            (SqlType::Uuid, SqlDialect::Postgresql) => "UUID",
            (SqlType::Uuid, SqlDialect::SQLite) => "BLOB",
            (SqlType::Bytes, SqlDialect::Postgresql) => "BYTEA",
            (SqlType::Bytes, SqlDialect::SQLite) => "BLOB",
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
