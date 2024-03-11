use cargo_toml::Manifest;
use error_stack::Report;
use serde::Deserialize;
use serde_json::json;

use crate::Error;

/// Options for a model that represents a file upload
#[derive(Deserialize, Clone, Debug)]
pub struct FileModelOptions {
    /// The storage bucket where the files should be stored. This must be one of the keys
    /// of [storage.bucket] in the primary configuration file.
    pub bucket: String,

    /// How to determine the keey at which an uploaded file will be stored.
    ///
    /// This template can include some special values which will be replaced dynamically:
    /// - `{org}` will be replaced with the ID of the organization that owns the object.
    /// - `{user}` will be replaced with the ID of the user that uploaded the object.
    /// - `{id}` will be replaced with the ID of the object.
    /// - `{filename}` will be replaced with the original filename of the uploaded file, if known.
    /// You can also use `strftime` percent parameters to insert time-based values.
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

    #[serde(default)]
    pub meta: FileUploadRecordMetadata,
}

impl FileModelOptions {
    pub fn add_deps(&self, manifest: &Manifest) -> Result<(), Report<Error>> {
        if let Some(hash) = &self.meta.hash {
            hash.add_deps(manifest)?;
        }

        Ok(())
    }

    pub fn template_context(&self) -> serde_json::Value {
        serde_json::json!({
            "bucket": self.bucket,
            "filename_template": self.filename_template,
            "hash": self.meta.hash.as_ref().map(|h| h.template_context()),
        })
    }
}

fn default_filename_template() -> String {
    String::from("{id}-{filename}")
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

    fn add_deps(&self, manifest: &Manifest) -> Result<(), Report<Error>> {
        let crate_dep = self.crate_name();
        crate::add_deps::add_dep(manifest, &crate_dep)?;
        crate::add_deps::add_dep(manifest, &("digest", "0.10.7", &[]))?;
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
            HashType::Blake3 => ("blake3", "1.5.0", &["traits-preview"]),
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
