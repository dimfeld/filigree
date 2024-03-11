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
    bucket: String,

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
    #[serde(default = "default_filename_template")]
    filename_template: String,

    /// Generate a `file_size` field in this model and set it automatically when a file
    /// is uploaded.
    #[serde(default)]
    record_size: bool,

    /// Add a field to hash the file with the specified algorithm as it is uploaded.
    hash: Option<HashType>,
}

impl FileModelOptions {
    pub fn add_deps(&self, manifest: &Manifest) -> Result<(), Report<Error>> {
        if let Some(hash) = &self.hash {
            let (crate_name, crate_version) = hash.crate_name();
            crate::add_deps::add_dep(manifest, &(crate_name, crate_version, &[]))?;
        }

        Ok(())
    }

    pub fn template_context(&self) -> serde_json::Value {
        serde_json::json!({
            "bucket": self.bucket,
            "filename_template": self.filename_template,
            "hash": self.hash.as_ref().map(|h| h.template_context()),
        })
    }
}

fn default_filename_template() -> String {
    String::from("{id}-{filename}")
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
        json!({
            "crate": self.crate_name().0,
            "crate_member": self.crate_member(),
            "use_statement": self.use_statement(),
        })
    }

    fn use_statement(&self) -> &'static str {
        match self {
            HashType::Sha3_224 | HashType::Sha3_256 | HashType::Sha3_384 | HashType::Sha3_512 => {
                "use sha3::Digest as _;"
            }
            HashType::Blake3 => "",
        }
    }

    fn crate_name(&self) -> (&'static str, &'static str) {
        match self {
            HashType::Sha3_224 | HashType::Sha3_256 | HashType::Sha3_384 | HashType::Sha3_512 => {
                ("sha3", "0.10.8")
            }
            HashType::Blake3 => ("blake3", "1.5.0"),
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
