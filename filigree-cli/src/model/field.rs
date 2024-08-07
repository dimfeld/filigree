use std::{borrow::Cow, collections::HashMap, fmt::Display};

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};

use super::SqlDialect;
use crate::Error;

#[derive(Serialize, Clone, Debug)]
pub struct ModelFieldTemplateContext {
    pub name: String,
    pub label: String,
    pub description: String,
    pub base_type: SqlType,
    pub sql_name: String,
    pub sql_full_name: String,
    pub sql_type: &'static str,
    pub snake_case_name: String,
    pub pascal_case_name: String,
    pub rust_name: String,
    pub base_rust_type: String,
    pub rust_type: String,
    pub is_custom_rust_type: bool,
    pub is_object_id: bool,
    pub client_type: String,
    pub default_sql: String,
    pub default_rust: String,
    pub nullable: bool,
    pub filterable: FilterableType,
    pub sortable: SortableType,
    pub indexed: bool,
    pub unique: bool,
    pub globally_unique: bool,
    pub omit_in_list: bool,
    pub foreign_key_sql: Option<String>,
    pub extra_sql_modifiers: String,
    pub readable: bool,
    pub writable: bool,
    pub never_read: bool,

    // The fields below are filled in later, from a place with more context.
    /// If this field is writable and is not a "parent ID" field
    pub writable_non_parent: bool,

    /// Override the binding name to be used with this field in a query.
    pub override_binding_name: Option<String>,
}

impl ModelFieldTemplateContext {
    /// Return the binding name to be used with this field in a query.
    /// Normally this is the same as the field name but it can be overridden.
    pub fn param_binding_name(&self) -> &str {
        self.override_binding_name.as_deref().unwrap_or(&self.name)
    }

    /// Rust syntax that can be submitted as a query binding for this field. The returned text
    /// contains the string `$payload` which can be replaced with the appropriate variable name.
    pub fn param_binding(&self) -> String {
        let is_custom_type = self.is_custom_rust_type && matches!(self.base_type, SqlType::Json);
        if self.nullable {
            if is_custom_type {
                format!(
                    "sqlx::types::Json($payload.{}.as_ref()) as _",
                    self.rust_name
                )
            } else {
                format!("$payload.{}.as_ref() as _", self.rust_name)
            }
        } else {
            if is_custom_type {
                format!("sqlx::types::Json(&$payload.{}) as _", self.rust_name)
            } else {
                format!("&$payload.{} as _", self.rust_name)
            }
        }
    }
}

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

    /// If true, omit this field from the `list` query.
    #[serde(default)]
    pub omit_in_list: bool,

    /// The SQL type for this field.
    #[serde(rename = "type")]
    pub typ: SqlType,
    /// The Rust type for this field. If omitted, the type will be inferred from the SQL
    /// type. This type should be fully qualified, e.g. `crate::MyType`.
    pub rust_type: Option<String>,
    /// The Zod type for this field. If omitted, the type will be inferred from the SQL
    /// type.
    pub zod_type: Option<String>,

    /// This field is optional. In Rust this translates to `Option<T>`. In SQL this controls
    /// the presence or absence of the NOT NULL qualification.
    #[serde(default)]
    pub nullable: bool,

    /// This field's value should be unique within the organization.
    #[serde(default)]
    pub unique: bool,

    /// This field's value should be unique within the entire table. Use with care.
    #[serde(default)]
    pub globally_unique: bool,

    /// Additional SQL to apply to this column.
    #[serde(default)]
    pub extra_sql_modifiers: String,

    /// Define how callers to the API can access this field. This is still gated on the user having
    /// the relevant read or write permission.
    #[serde(default)]
    pub access: Access,

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

    pub fn readable(&self) -> bool {
        self.access.can_read() && !self.never_read
    }

    pub fn writable(&self) -> bool {
        !self.fixed && self.access.can_write() && !self.never_read
    }

    pub fn template_context(&self) -> ModelFieldTemplateContext {
        let is_object_id = self.name == "id"
            || self
                .references
                .as_ref()
                .map(|r| r.field == "id")
                .unwrap_or(false);
        ModelFieldTemplateContext {
            name: self.name.clone(),
            label: self
                .label
                .clone()
                .unwrap_or_else(|| self.name.to_case(Case::Title)),
            is_object_id,
            description: self.description.clone().unwrap_or_default(),
            base_type: self.typ,
            sql_name: self.sql_field_name(),
            sql_full_name: self.qualified_sql_field_name(),
            sql_type: self.typ.to_sql_type(SqlDialect::Postgresql),
            snake_case_name: self.name.to_case(Case::Snake),
            pascal_case_name: self.name.to_case(Case::Pascal),
            rust_name: self.rust_field_name(),
            base_rust_type: self.base_rust_type().to_string(),
            rust_type: self.rust_type().to_string(),
            is_custom_rust_type: self.rust_type.is_some(),
            client_type: self.typ.to_client_type().to_string(),
            default_sql: self.default_sql.clone(),
            default_rust: self.default_rust.clone(),
            nullable: self.nullable,
            filterable: self.filterable,
            sortable: self.sortable,
            indexed: self.indexed || self.unique,
            unique: self.unique,
            globally_unique: self.globally_unique,
            omit_in_list: self.omit_in_list,
            foreign_key_sql: self.references.as_ref().map(|r| r.to_string()),
            extra_sql_modifiers: self.extra_sql_modifiers.clone(),
            readable: self.readable(),
            writable: self.writable(),
            never_read: self.never_read,
            // These get set later, where appropriate
            writable_non_parent: false,
            override_binding_name: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelFieldReference {
    /// The name of the model to reference
    pub model: Option<String>,
    /// A table to reference, if you want to reference a table not in a model.
    pub table: Option<String>,
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
        model: impl Into<String>,
        field: impl Into<String>,
        on_delete: Option<ReferentialAction>,
    ) -> Self {
        Self {
            model: Some(model.into()),
            table: None,
            field: field.into(),
            on_delete,
            on_update: None,
            deferrable: None,
            populate: None,
        }
    }

    pub fn validate(&self, model_name: &str, field_name: &str) -> Result<(), Error> {
        if self.model.is_none() && self.table.is_none() {
            Err(Error::FieldReferenceConfig(
                model_name.to_string(),
                field_name.to_string(),
                "must specify either model or table",
            ))
        } else if self.model.is_some() && self.table.is_some() {
            Err(Error::FieldReferenceConfig(
                model_name.to_string(),
                field_name.to_string(),
                "can not specify both model and table",
            ))
        } else {
            Ok(())
        }
    }

    pub fn fill_table(
        &mut self,
        this_model: &str,
        this_field: &str,
        model_tables: &HashMap<String, String>,
    ) -> Result<(), Error> {
        if self.table.is_some() {
            return Ok(());
        }

        let table = model_tables
            .get(self.model.as_deref().unwrap())
            .ok_or_else(|| {
                Error::MissingModel(
                    self.model.clone().unwrap(),
                    this_model.to_string(),
                    this_field.to_string(),
                )
            })?;
        self.table = Some(table.clone());
        self.model = None;
        Ok(())
    }

    pub fn with_deferrable(mut self, deferrable: Deferrable) -> Self {
        self.deferrable = Some(deferrable);
        self
    }
}

impl Display for ModelFieldReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "REFERENCES {} ({})",
            self.table
                .as_deref()
                .expect("Reference displayed before fill_table was called"),
            self.field
        )?;

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
