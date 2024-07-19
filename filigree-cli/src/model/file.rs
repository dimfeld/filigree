use std::path::Path;

use cargo_toml::Manifest;
use convert_case::{Case, Casing};
use error_stack::Report;
use serde::Deserialize;
use serde_json::json;

use super::{
    field::{Access, FilterableType, ModelField, SqlType},
    Endpoints, HasModel, Model, Pagination, ReferenceFetchType,
};
use crate::{config::Config, Error};

/// Options for a model that represents a file upload
#[derive(Deserialize, Clone, Debug)]
pub struct FileModelOptions {
    /// The name of this file model. This affects both the model name, which is an concatenation
    /// of the parent model name and this name, and also the rust module and URL segments for the model.
    pub name: String,

    /// The storage bucket where the files should be stored. This must be one of the keys
    /// of [storage.bucket] in the primary configuration file.
    pub bucket: String,

    /// The prefix to use for this file's object IDs
    pub id_prefix: Option<String>,

    /// If true, the hosting model can reference many files.
    #[serde(default)]
    pub many: bool,

    /// How to fetch the referenced instances of the model in the "list" endpoint
    #[serde(default)]
    pub populate_on_list: ReferenceFetchType,
    /// How to fetch the referenced instances of the model in the "get" endpoint
    #[serde(default)]
    pub populate_on_get: ReferenceFetchType,

    /// How to determine the keey at which an uploaded file will be stored.
    ///
    /// This template can include some special values which will be replaced dynamically:
    /// - `{org}` will be replaced with the ID of the organization that owns the object.
    /// - `{user}` will be replaced with the ID of the user that uploaded the object.
    /// - `{id}` will be replaced with the ID of the object.
    /// - `{filename}` will be replaced with the original filename of the uploaded file, if known.
    /// - '{year}' will be replaced with the current year
    /// - '{month}' will be replaced with the current month, 01-12
    /// - '{day}' will be replaced with the current day, 01-31
    /// - '{hour}' will be replaced with the current hour, 00-23
    /// - '{minute}' will be replaced with the current minute, 00-59
    /// - '{second}' will be replaced with the current second, 00-59
    ///
    /// All date and time paramters use UTC.
    ///
    /// The default template is "{id}-{filename}". This helps to guarantee
    /// unique file names, while still aiding manual inspection.
    ///
    /// Note that the only guaranteed way to ensure uniqueness is to use `{id}` in the template,
    /// and you risk overwriting existing files if you do not use it. This might be ok based on
    /// your use case, but it should be a conscious decision. The original filename (when known)
    /// is still stored in this model, so even if the filename_template does not reflect the
    /// original filename, it can still be retained and used when sending the file back to the
    /// user.
    #[serde(default = "default_filename_template")]
    pub filename_template: String,

    /// True if the user should be able to see the key at which the file is stored
    #[serde(default)]
    pub storage_key_readable: bool,

    // /// True if the user should be able to see the public URL of this file. Only applies when the
    // /// backing storage `bucket` has a public URL set.
    // #[serde(default)]
    // pub public_url_readable: bool,
    #[serde(default)]
    pub meta: FileUploadRecordMetadata,

    /// Limit the maximum size of uploaded files
    pub upload_size_limit: Option<usize>,

    /// If omitted or false, the file will be deleted from storage when the model is deleted.
    /// If true, the file will be retained in object storage even after the model is deleted.
    #[serde(default)]
    pub retain_file_on_delete: bool,
}

fn default_filename_template() -> String {
    String::from("{id}-{filename}")
}

impl FileModelOptions {
    pub fn validate(&self, model_name: &str, config: &Config) -> Result<(), Error> {
        config.storage.bucket.get(&self.bucket).ok_or_else(|| {
            Error::InvalidStorageBucket(model_name.to_string(), self.bucket.clone())
        })?;
        Ok(())
    }

    pub fn add_deps(&self, api_dir: &Path, manifest: &mut Manifest) -> Result<(), Report<Error>> {
        if let Some(hash) = &self.meta.hash {
            hash.add_deps(api_dir, manifest)?;
        }

        Ok(())
    }

    pub fn template_context(&self) -> serde_json::Value {
        let template_func = self.filename_template_function();

        serde_json::json!({
            "bucket": self.bucket,
            "filename_template_function_body": template_func,
            "many": self.many,
            "hash": self.meta.hash.as_ref().map(|h| h.template_context()),
            "record_size": self.meta.size,
            "upload_size_limit": self.upload_size_limit,
            "record_filename": self.meta.filename,
            "retain_file_on_delete": self.retain_file_on_delete,
        })
    }

    fn filename_template_function(&self) -> String {
        let mut uses_date = false;
        let mut format_parameters = String::new();

        let parameters = [
            ("year", 0, "now.year()", true),
            ("month", 2, "now.month()", true),
            ("day", 2, "now.day()", true),
            ("hour", 2, "now.hour()", true),
            ("minute", 2, "now.minute()", true),
            ("second", 2, "now.second()", true),
            ("org", 0, "auth.organization_id", false),
            ("user", 0, "auth.user_id", false),
            ("id", 0, "id", false),
            ("filename", 0, "filename", false),
        ];

        let mut template = self.filename_template.clone();
        for (name, padding, value, is_date) in parameters {
            let search_param = format!("{{{}}}", name);
            if !template.contains(&search_param) {
                continue;
            }

            if is_date {
                uses_date = true;
            }

            if padding > 0 {
                template = template.replace(&search_param, &format!("{{{name}:0{padding}}}"));
            }

            format_parameters.push_str(&name);
            format_parameters.push('=');
            format_parameters.push_str(value);
            format_parameters.push_str(",\n");
        }

        let now = if uses_date {
            "let now = chrono::Utc::now();"
        } else {
            ""
        };

        format!(
            r###"
        {now}
        format!(
            r##"{template}"##,
            {format_parameters}
        )
        "###
        )
    }

    pub fn model_name(&self, parent: &Model) -> String {
        format!(
            "{}{}",
            parent.name.to_case(Case::Pascal),
            self.name.to_case(Case::Pascal)
        )
    }

    fn child_field_name(&self) -> String {
        let field_name = self.name.to_case(Case::Snake);
        if self.many {
            format!("{field_name}s")
        } else {
            field_name
        }
    }

    pub fn has_for_parent(&self, parent: &Model) -> HasModel {
        HasModel {
            model: self.model_name(parent),
            many: self.many,
            through: None,
            populate_on_get: self.populate_on_get,
            populate_on_list: self.populate_on_list,
            update_with_parent: false,
            field_name: Some(self.child_field_name()),
        }
    }

    pub fn generate_model(&self, parent: &Model) -> Model {
        Model {
            name: self.model_name(parent),
            file_for: Some((parent.name.clone(), self.clone())),
            // file upload submodel does not have embedded file upload submodels
            files: Vec::new(),
            shared_types: Vec::new(),
            id_prefix: self.id_prefix.clone().or_else(|| {
                let self_prefix: String = self.name.to_lowercase().chars().take(3).collect();
                Some(format!("{}{}", parent.id_prefix(), self_prefix))
            }),
            fields: self.file_model_fields(),
            belongs_to: vec![super::BelongsTo::Simple(parent.name.clone())],
            // The object is only accessible via the parent model, so don't generate endpoints
            // here.
            standard_endpoints: Endpoints::All(false),
            plural: None,
            default_sort_field: None,
            pagination: Pagination::default(),
            extra_create_table_sql: String::new(),
            extra_sql: String::new(),
            global: false,
            allow_id_in_create: false,
            auth_scope: None,
            endpoints: Vec::new(),
            indexes: vec![],
            index_created_at: true,
            index_updated_at: true,
            joins: None,
            has: vec![],
            is_auth_model: false,
            schema: parent.schema.clone(),
        }
    }

    fn file_model_fields(&self) -> Vec<ModelField> {
        let key_access = if self.storage_key_readable {
            Access::ReadWrite
        } else {
            Access::Write
        };

        let mut fields = vec![
            ModelField {
                name: "file_storage_key".to_string(),
                typ: SqlType::Text,
                nullable: false,
                access: key_access,
                ..Default::default()
            },
            // The id of the bucket where the file is stored.
            // Generally this will be all the same, but can be useful when migrating from one
            // bucket to another.
            ModelField {
                name: "file_storage_bucket".to_string(),
                typ: SqlType::Text,
                nullable: false,
                access: Access::Write,
                ..Default::default()
            },
        ];

        if self.meta.filename {
            fields.push(ModelField {
                name: "file_original_name".to_string(),
                typ: SqlType::Text,
                nullable: true,
                access: Access::Write,
                filterable: FilterableType::Exact,
                ..Default::default()
            });
        }

        if self.meta.size {
            fields.push(ModelField {
                name: "file_size".to_string(),
                typ: SqlType::BigInt,
                nullable: true,
                access: Access::ReadWrite,
                ..Default::default()
            });
        }

        if self.meta.hash.is_some() {
            fields.push(ModelField {
                name: "file_hash".to_string(),
                typ: SqlType::Bytes,
                nullable: true,
                access: Access::ReadWrite,
                filterable: FilterableType::Exact,
                ..Default::default()
            });
        }

        fields
    }
}

/// Metadata that we might want to record about the uploaded file. Setting these fields will
/// add code to calculate the metadata and add fields to the model in which to record it.
#[derive(Deserialize, Default, Clone, Debug)]
pub struct FileUploadRecordMetadata {
    /// Generate a `filename` field in the model, and record the original filename of the uploaded file, if it is known.
    #[serde(default)]
    pub filename: bool,

    /// Generate a `size` field in this model and set it automatically when a file
    /// is uploaded.
    #[serde(default)]
    pub size: bool,

    /// Add a `hash` field to the model, and hash the file with the specified algorithm as it is uploaded.
    pub hash: Option<HashType>,
}

/// The hashing algorithm to use when uploading files
#[derive(Deserialize, Clone, Debug)]
pub enum HashType {
    Sha3_224,
    Sha3_256,
    Sha3_384,
    Sha3_512,
    Blake3,
}

impl HashType {
    fn template_context(&self) -> serde_json::Value {
        let crate_name = self.crate_name().0;
        let hasher = format!("{}::{}", crate_name, self.crate_member());
        json!({
            "crate": self.crate_name().0,
            "hasher": hasher,
            "use_statement": self.use_statement(),
        })
    }

    fn add_deps(&self, api_dir: &Path, manifest: &mut Manifest) -> Result<(), Report<Error>> {
        let crate_dep = self.crate_name();
        crate::add_deps::add_dep(api_dir, manifest, crate_dep.0, crate_dep.1, crate_dep.2)?;
        crate::add_deps::add_dep(api_dir, manifest, "digest", "0.10.7", &[])?;
        Ok(())
    }

    fn use_statement(&self) -> &'static str {
        "use digest::Digest;"
    }

    fn crate_name(&self) -> (&'static str, &'static str, &[&'static str]) {
        match self {
            HashType::Sha3_224 | HashType::Sha3_256 | HashType::Sha3_384 | HashType::Sha3_512 => {
                ("sha3", "0.10.8", &[])
            }
            HashType::Blake3 => ("blake3", "1.5.1", &["traits-preview"]),
        }
    }

    fn crate_member(&self) -> &'static str {
        match self {
            HashType::Sha3_224 => "Sha3_224",
            HashType::Sha3_256 => "Sha3_256",
            HashType::Sha3_384 => "Sha3_384",
            HashType::Sha3_512 => "Sha3_512",
            HashType::Blake3 => "Hasher",
        }
    }
}
