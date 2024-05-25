mod base_models;
pub mod field;
pub mod file;
mod generate_types;
pub mod generator;
pub mod validate;

use std::{borrow::Cow, path::Path};

use cargo_toml::Manifest;
use convert_case::{Case, Casing};
use error_stack::Report;
use serde::{Deserialize, Serialize};

use self::{
    field::{Access, ModelField, ModelFieldReference, SqlType},
    file::FileModelOptions,
};
use crate::{
    config::{custom_endpoint::CustomEndpoint, Config},
    Error,
};

#[derive(Deserialize, Clone, Debug)]
pub struct Model {
    /// The name of the model
    pub name: String,

    /// File fields that this model has
    #[serde(default)]
    pub files: Vec<FileModelOptions>,

    /// Paths to types that exist in the Rust application and should be replicated in
    /// Typescript. These types must derive or otherwise implement the [schemars::JsonSchema] trait.
    ///
    /// The paths here will be prefixed with `crate::models::MODEL_MODULE::`. For types not specific to a
    /// model you can use `shared_types` in the primary configuration file.
    #[serde(default)]
    pub shared_types: Vec<String>,

    /// The plural of [name], if not generated by adding the letter 's' to the end.
    #[serde(default)]
    pub plural: Option<String>,
    /// A prefix of a few characters for the ID of this type.
    /// Defaults to the first three characters of the name
    pub id_prefix: Option<String>,
    #[serde(default)]
    pub fields: Vec<ModelField>,
    /// If true, generate API endpoints for this model.
    pub standard_endpoints: Endpoints,

    #[serde(default)]
    /// Custom endpoints to create. This will generate Rust code and equivalent Typescript
    /// functions and types.
    endpoints: Vec<CustomEndpoint>,

    /// The default field to order by on list operations. Prefix with '-' to order descending.
    /// If omitted, "-updated_at" is used.
    pub default_sort_field: Option<String>,

    /// Pagination options for this model, if not the default.
    #[serde(default)]
    pub pagination: Pagination,

    /// Extra SQL to place after the column definitions inside the `CREATE TABLE` statement,
    /// such as multi-column foreign keys or other constraints.
    #[serde(default)]
    pub extra_create_table_sql: String,

    /// Extra SQL to place in the migration after creating the table and indexes.
    #[serde(default)]
    pub extra_sql: String,

    /// If true, this model does not have a organization_id.
    /// This mostly applies to the organization object itself but may be useful for other things.
    #[serde(default)]
    pub global: bool,

    /// Allow specifying the ID of the object when creating it
    #[serde(default)]
    pub allow_id_in_create: bool,

    /// Set how permissions are tracked on this model. If omitted, it will use [Config#default_auth_scope]
    #[serde(default)]
    pub auth_scope: Option<ModelAuthScope>,

    /// SQL to create indexes on the field
    #[serde(default)]
    pub indexes: Vec<String>,
    // TODO ability to define extra permissions
    // TODO ability to define extra operations that update specific things and require specific
    // permissions. Maybe this should just be defined in the normal code instead though...
    /// Add a descending index on the `created_at` field. Default true
    #[serde(default = "true_t")]
    pub index_created_at: bool,

    /// Add a descending index on the `updated_at` field. Default true
    #[serde(default = "true_t")]
    pub index_updated_at: bool,

    // References to other models
    /// This model joins two other models, rather than being a normal model.
    /// A joining model does not have a normal id field; instead its id is the combination of the ids of the
    /// models specified here, and the primary key is composed of those two fields.
    #[serde(default)]
    pub joins: Option<(String, String)>,

    /// A parent model for this model, when the other model has a `has` relationship
    /// This adds a field to this model that references the ID of the parent model.
    #[serde(default)]
    pub belongs_to: Option<BelongsTo>,

    /// This model links to other instances of the listed models and can optionally manage them
    /// as sub-entities, updating in the same operation as the update to the parent.
    #[serde(default)]
    pub has: Vec<HasModel>,

    /// If set, this model is a file submodel and `file_for` is its parent.
    #[serde(skip, default)]
    pub(crate) file_for: Option<(String, FileModelOptions)>,

    #[serde(skip, default)]
    /// Set to true if this model is an "auth" model. This only affects which schema
    /// it takes.
    is_auth_model: bool,

    /// The schema to use for this model. If not set, it will use the schema settings
    /// from the main configuration.
    pub schema: Option<String>,
}

impl Model {
    pub fn module_name(&self) -> String {
        self.name.to_case(Case::Snake)
    }

    pub fn foreign_key_id_field_name(&self) -> String {
        format!("{}_id", self.name.to_case(Case::Snake))
    }

    pub fn table(&self) -> String {
        self.plural().to_case(Case::Snake)
    }

    pub fn schema(&self) -> &str {
        self.schema.as_deref().unwrap_or("public")
    }

    /// The table including schema and table name
    pub fn full_table(&self) -> String {
        format!("{}.{}", self.schema(), self.table())
    }

    pub fn id_prefix(&self) -> Cow<str> {
        self.id_prefix
            .as_deref()
            .map(Cow::from)
            .unwrap_or_else(|| Cow::Owned(self.name.to_lowercase().chars().take(3).collect()))
    }

    pub fn qualified_object_id_type(&self) -> String {
        format!(
            "crate::models::{}::{}",
            self.module_name(),
            self.object_id_type()
        )
    }

    pub fn object_id_type(&self) -> String {
        format!("{}Id", self.name.to_case(Case::Pascal))
    }

    pub fn qualified_struct_name(&self) -> String {
        format!(
            "crate::models::{}::{}",
            self.module_name(),
            self.struct_name()
        )
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

    pub fn add_deps(&self, api_dir: &Path, manifest: &mut Manifest) -> Result<(), Report<Error>> {
        for file in &self.files {
            file.add_deps(api_dir, manifest)?;
        }

        Ok(())
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

        self.standard_endpoints
            .merge_from(other_model.standard_endpoints);
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

    /// Return true if this table depends on the `other` table in some way.
    fn depends_on(&self, other: &Model) -> bool {
        if other.name == "Organization" {
            // Everything except User depends on organization. This is hardcoded here because
            // this function doesn't look at the "standard" fields.
            return self.name != "User";
        }

        if self
            .joins
            .as_ref()
            .map(|(j1, j2)| j1 == &other.name || j2 == &other.name)
            .unwrap_or(false)
        {
            return true;
        }

        if let Some(b) = &self.belongs_to {
            if b.model() == other.name {
                return true;
            }
        }

        if self.fields.iter().any(|f| {
            let Some(r) = &f.references else {
                return false;
            };

            r.table
                .as_deref()
                .expect("reference table was not filled in")
                == other.table()
        }) {
            return true;
        }

        false
    }

    /// Comparison function to sort models so that child models come first,
    /// in order to write templates in the right order when parent models reference
    /// types defined by the children.
    pub fn order_by_dependency(&self, other: &Model) -> std::cmp::Ordering {
        if self.depends_on(other) {
            std::cmp::Ordering::Less
        } else if other.depends_on(self) {
            std::cmp::Ordering::Greater
        } else {
            self.name.cmp(&other.name)
        }
    }

    pub fn apply_config(&mut self, config: &Config) {
        if self.schema.is_none() {
            self.schema = if self.is_auth_model {
                config.database.auth_schema().map(|s| s.to_string())
            } else {
                config.database.model_schema().map(|s| s.to_string())
            };
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Pagination {
    /// Disable pagination completely unless it's explicitly requested.
    pub disable: bool,
    #[serde(default = "default_per_page")]
    pub default_per_page: u32,
    #[serde(default = "default_max_per_page")]
    pub max_per_page: u32,
    /// Maximum number of pages possible to return.
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

#[derive(Deserialize, Clone, Debug)]
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
    // TODO implement this soon, and make it configurable whether objects are globally readable by
    // default or not
    // /// Permissions on existing objects are set per-object.
    // /// Creators of an object automatically get owner permission on the object.
    // Object,
}

impl std::fmt::Display for ModelAuthScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelAuthScope::Model => write!(f, "model"),
            // ModelAuthScope::Project => write!(f, "project"),
            // ModelAuthScope::Object => write!(f, "object"),
        }
    }
}

/// How to fetch child models the parent model
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceFetchType {
    /// Do not fetch the child models at all.
    #[default]
    None,
    /// Return just the IDs of the child models
    Id,
    /// Fetch and return the entire data for the child models
    Data,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HasModel {
    /// The name of the child model
    pub model: String,

    /// If true, this model can be a parent to more than one of the child model.
    #[serde(default)]
    pub many: bool,

    /// The model that acts as the linking table between this model and the other model
    /// if there is one. If not set, it is assumed that the child model contains the
    /// relevant ID field to query against.
    pub through: Option<String>,

    /// How to fetch the referenced instances of the model in the "list" endpoint
    #[serde(default)]
    pub populate_on_list: ReferenceFetchType,
    /// How to fetch the referenced instances of the model in the "get" endpoint
    #[serde(default)]
    pub populate_on_get: ReferenceFetchType,

    /// If set, allow adding, updating, and deleting instances of the child model
    /// during the "create", "update", and "delete" operations on this parent model.
    ///
    /// When `through` is `None`:
    /// For single relationships, this adds an Option<ChildModelUpdatePayload> field to the model's update payload, and
    /// for `many` relationships, this adds a Vec<ChildModelUpdatePayload> field to the model.
    /// Instances of the child model will be created and deleted when they are created and deleted
    /// here.
    ///
    /// When `through` is set:
    /// For single relationships, this adds an Option<ChildModelId> field to the model's update payload, and
    /// for `many` relationships, this adds a Vec<ChildModelId> field to the model.
    /// Instances of the child model will be created and deleted when they are created and deleted
    /// here.
    ///
    /// Using `id` here only really makes sense when `many` and `through` are both set, for
    /// example, when managing tags on an object.
    ///
    #[serde(default)]
    pub update_with_parent: bool,

    /// Override the field name at which the populated children will be placed in the model.
    pub field_name: Option<String>,
}

impl HasModel {
    pub fn rust_child_field_name(&self, model: &Model) -> String {
        self.field_name.clone().unwrap_or_else(|| {
            if self.many {
                model.plural().to_case(Case::Snake)
            } else {
                model.name.to_case(Case::Snake)
            }
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum BelongsTo {
    Simple(String),
    Full(FullBelongsTo),
}

impl BelongsTo {
    pub fn model(&self) -> &str {
        match self {
            BelongsTo::Simple(m) => m,
            BelongsTo::Full(b) => &b.model,
        }
    }

    pub fn optional(&self) -> bool {
        match self {
            BelongsTo::Simple(_) => FullBelongsTo::default().optional,
            BelongsTo::Full(b) => b.optional,
        }
    }

    pub fn indexed(&self) -> bool {
        match self {
            BelongsTo::Simple(_) => FullBelongsTo::default().indexed,
            BelongsTo::Full(b) => b.indexed,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FullBelongsTo {
    /// The name of the model to link to
    model: String,

    /// If true, it's ok for this object to not be linked to a parent. Defaults to false.
    #[serde(default)]
    optional: bool,

    /// If true, a database index will be generated for this field. Defaults to true.
    #[serde(default = "true_t")]
    indexed: bool,
}

impl Default for FullBelongsTo {
    fn default() -> Self {
        Self {
            model: String::new(),
            optional: false,
            indexed: true,
        }
    }
}

fn true_t() -> bool {
    true
}
