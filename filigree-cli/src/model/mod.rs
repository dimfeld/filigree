mod base_models;
pub mod field;
mod generate_endpoints;
mod generate_types;
pub mod generator;

use std::borrow::Cow;

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};

use self::field::{Access, DeleteBehavior, ModelField, ModelFieldReference, SqlType};

#[derive(Deserialize, Debug)]
pub struct Model {
    pub name: String,
    /// The plural of [name], if not generated by adding the letter 's' to the end.
    #[serde(default)]
    pub plural: Option<String>,
    /// A prefix of a few characters for the ID of this type.
    /// Defaults to the first three characters of the name
    pub id_prefix: Option<String>,
    pub fields: Vec<ModelField>,
    /// If true, generate API endpoints for this model.
    pub endpoints: bool,

    /// Pagination options for this model, if not the default.
    #[serde(default)]
    pub pagination: Pagination,

    /// Extra SQL to place after the column definitions inside the `CREATE TABLE` statement.
    #[serde(default)]
    pub extra_create_table_sql: String,

    /// If true, this model does not have a organization_id.
    /// This mostly applies to the organization object itself but may be useful for other things.
    #[serde(default)]
    pub global: bool,

    /// SQL to create indexes on the field
    #[serde(default)]
    pub indexes: Vec<String>,
    // TODO ability to define extra permissions
    // TODO ability to define extra operations that update specific things and require specific
    // permissions. Maybe this should just be defined in the normal code instead though...
}

impl Model {
    pub fn module_name(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn table(&self) -> String {
        self.plural().to_case(Case::Snake)
    }

    pub fn id_prefix(&self) -> Cow<str> {
        self.id_prefix
            .as_deref()
            .map(Cow::from)
            .unwrap_or_else(|| Cow::Owned(self.name.to_lowercase().chars().take(3).collect()))
    }

    pub fn object_id_type(&self) -> String {
        format!("{}Id", self.name.to_case(Case::Pascal))
    }

    pub fn struct_name(&self) -> String {
        self.name.to_case(Case::Pascal)
    }

    pub fn plural(&self) -> Cow<str> {
        self.plural
            .as_deref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(format!("{}s", self.name)))
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
        let org_field = if self.global {
            None
        } else {
            Some(ModelField {
                name: "organization_id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some("OrganizationId".to_string()),
                nullable: false,
                unique: false,
                indexed: true,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default: String::new(),
                references: Some(ModelFieldReference::new(
                    "organizations",
                    "id",
                    DeleteBehavior::Cascade,
                )),
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
                references: None,
                default: String::new(),
            }),
            org_field,
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
                references: None,
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
                references: None,
                default: "now()".to_string(),
            }),
        ]
        .into_iter()
        .flatten()
    }

    /// The base models use this function to merge in fields added from the config.
    /// Fields with the same name as an existing field in the base model will be replaced,
    /// and other fields will be appended to the end.
    /// Use caution when
    pub(crate) fn merge_from(&mut self, mut other_model: Model) {
        for field in self.fields.iter_mut() {
            let existing = other_model.fields.iter().position(|f| f.name == field.name);
            let Some(existing) = existing else {
                continue;
            };

            let existing = other_model.fields.remove(existing);
            *field = existing;
        }

        self.fields.extend(other_model.fields.into_iter());

        if other_model.id_prefix.is_some() {
            self.id_prefix = other_model.id_prefix;
        }

        self.endpoints = self.endpoints || other_model.endpoints;
        match (
            self.extra_create_table_sql.is_empty(),
            other_model.extra_create_table_sql.is_empty(),
        ) {
            (true, false) => self.extra_create_table_sql = other_model.extra_create_table_sql,
            (false, false) => {
                self.extra_create_table_sql = format!(
                    "{},\n{}",
                    self.extra_create_table_sql, other_model.extra_create_table_sql
                )
            }
            _ => {}
        };

        self.indexes.extend(other_model.indexes.into_iter());

        // Don't merge `global`, `plural`, or `name` since these must not change
        // for things to work properly.
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Pagination {
    /// Disable pagination completely unless it's explicitly requested.
    pub disable: bool,
    #[serde(default = "default_per_page")]
    pub default_per_page: u32,
    #[serde(default = "default_max_per_page")]
    pub max_per_page: u32,
    /// Maximum number of pages possible to return.
    /// Usually you won't want to set this
    pub max_page: Option<u32>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            disable: false,
            default_per_page: default_per_page(),
            max_per_page: default_max_per_page(),
            max_page: None,
        }
    }
}

const fn default_per_page() -> u32 {
    50
}

const fn default_max_per_page() -> u32 {
    200
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SqlDialect {
    Postgresql,
    SQLite,
}
