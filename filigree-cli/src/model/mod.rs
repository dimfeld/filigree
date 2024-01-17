mod base_models;
pub mod field;
mod generate_types;
pub mod generator;

use std::borrow::Cow;

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use serde_json::json;

use self::field::{
    Access, DeleteBehavior, FilterableType, ModelField, ModelFieldReference, SqlType,
};
use crate::{config::Config, model::field::SortableType};

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
    pub endpoints: Endpoints,
    /// The default field to order by on list operations. Prefix with '-' to order descending.
    /// If omitted, "-updated_at" is used.
    pub default_sort_field: Option<String>,

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

    /// Set how permissions are tracked on this model. If omitted, it will use [Config#default_auth_scope]
    #[serde(default)]
    pub auth_scope: Option<ModelAuthScope>,

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

    pub fn all_fields(&self) -> impl Iterator<Item = Cow<ModelField>> {
        self.standard_fields()
            .map(|field| Cow::Owned(field))
            .chain(self.fields.iter().map(|field| Cow::Borrowed(field)))
    }

    pub fn write_payload_struct_fields(&self) -> impl Iterator<Item = Cow<ModelField>> {
        self.all_fields()
            .filter(|f| f.owner_access.can_write() && !f.never_read)
    }

    pub fn template_context(&self, config: &Config) -> serde_json::Value {
        let full_default_sort_field = self.default_sort_field.as_deref().unwrap_or("-updated_at");
        let default_sort_field = if full_default_sort_field.starts_with('-') {
            &full_default_sort_field[1..]
        } else {
            full_default_sort_field
        };

        {
            let default_field = self.all_fields().find(|f| f.name == default_sort_field);
            if let Some(default_field) = default_field {
                if default_field.sortable == SortableType::None {
                    panic!(
                        "Model {}: Default sort field {default_sort_field} is not sortable",
                        self.name
                    );
                }
            } else {
                panic!(
                    "Model {}, Default sort field {} does not exist in model {}",
                    self.name, default_sort_field, self.name
                );
            }
        }

        let predefined_object_id =
            &["role", "user", "organization"].contains(&self.module_name().as_str());

        let fields = self
            .all_fields()
            .map(|field| field.template_context())
            .collect::<Vec<_>>();

        json!({
            "name": self.name,
            "plural": self.plural(),
            "table": self.table(),
            "indexes": self.indexes,
            "global": self.global,
            "fields": fields,
            "owner_permission": format!("{}::owner", self.name),
            "read_permission": format!("{}::read", self.name),
            "write_permission": format!("{}::write", self.name),
            "extra_create_table_sql": self.extra_create_table_sql,
            "pagination": self.pagination,
            "full_default_sort_field": full_default_sort_field,
            "default_sort_field": default_sort_field,
            "id_type": self.object_id_type(),
            "id_prefix": self.id_prefix(),
            "predefined_object_id": predefined_object_id,
            "url_path": self.plural().to_lowercase(),
            "has_any_endpoints": self.endpoints.any_enabled(),
            "endpoints": self.endpoints.per_endpoint(),
            "auth_scope": self.auth_scope.unwrap_or(config.default_auth_scope),
        })
    }

    /// The fields that apply to every object
    fn standard_fields(&self) -> impl Iterator<Item = ModelField> {
        let org_field = if self.global {
            None
        } else {
            Some(ModelField {
                name: "organization_id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some("crate::models::organization::OrganizationId".to_string()),
                nullable: false,
                unique: false,
                indexed: true,
                sortable: field::SortableType::None,
                filterable: FilterableType::None,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default_sql: String::new(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
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
                filterable: FilterableType::Exact,
                sortable: field::SortableType::None,
                extra_sql_modifiers: "primary key".to_string(),
                user_access: Access::Read,
                owner_access: Access::Read,
                references: None,
                default_sql: String::new(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
            }),
            org_field,
            Some(ModelField {
                name: "updated_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                indexed: false,
                filterable: FilterableType::Range,
                sortable: field::SortableType::DefaultDescending,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                references: None,
                default_sql: "now()".to_string(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
            }),
            Some(ModelField {
                name: "created_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                indexed: false,
                filterable: FilterableType::Range,
                sortable: field::SortableType::DefaultDescending,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                references: None,
                default_sql: "now()".to_string(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
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

        self.endpoints.merge_from(other_model.endpoints);
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

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Endpoints {
    All(bool),
    Only(PerEndpoint),
}

impl Endpoints {
    pub fn per_endpoint(&self) -> PerEndpoint {
        match self {
            Endpoints::All(choice) => PerEndpoint::from(*choice),
            Endpoints::Only(p) => p.clone(),
        }
    }

    pub fn any_enabled(&self) -> bool {
        match self {
            Endpoints::All(choice) => *choice,
            Endpoints::Only(p) => p.any_enabled(),
        }
    }

    pub fn merge_from(&mut self, other: Endpoints) {
        match (&self, other) {
            (Endpoints::All(true), _) => {}
            (_, Endpoints::All(false)) => {}
            (Endpoints::All(false), b) => *self = b,
            (Endpoints::Only(_), Endpoints::All(true)) => *self = Endpoints::All(true),
            (Endpoints::Only(a), Endpoints::Only(b)) => {
                *self = Endpoints::Only(PerEndpoint {
                    get: a.get || b.get,
                    list: a.list || b.list,
                    create: a.create || b.create,
                    update: a.update || b.update,
                    delete: a.delete || b.delete,
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerEndpoint {
    pub get: bool,
    pub list: bool,
    pub create: bool,
    pub update: bool,
    pub delete: bool,
}

impl PerEndpoint {
    pub fn any_enabled(&self) -> bool {
        self.get || self.list || self.create || self.update || self.delete
    }
}

impl From<bool> for PerEndpoint {
    fn from(b: bool) -> Self {
        Self {
            get: b,
            list: b,
            create: b,
            update: b,
            delete: b,
        }
    }
}

/// The scope at which at an object's permissions are tracked
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelAuthScope {
    /// There is a single set of owner/editor/viewer permissions that applies to all objects of this model.
    Model,
    // TODO projects not implemented yet
    // /// Object permissions are inherited from the user's permissions on the project that
    // /// contains the object.
    // Project,
    // TODO implement this soon
    // /// Permissions on existing objects are set per-object.
    // /// Creators of an object get owner permission by default.
    // Object,
}
