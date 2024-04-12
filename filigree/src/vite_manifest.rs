//! Parse a Vite manifest and generate HTML to include each entrypoint.

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::RwLock,
};

use error_stack::{Report, ResultExt};
use maud::PreEscaped;
use serde::Deserialize;
use thiserror::Error;

struct ManifestData {
    /// The data for the index page
    index: PreEscaped<String>,
    /// All the entry points
    entries: HashMap<String, PreEscaped<String>>,
}

/// A parsed Vite manifest
pub struct Manifest(RwLock<Option<ManifestData>>);

#[derive(Error, Debug, PartialEq, Eq)]
/// An error that occurs while loading the Vite manifest
pub enum ManifestError {
    /// The file was not founbd
    #[error("Vite manifest not found")]
    NotFound,
    /// The manifest could not be parsed
    #[error("Failed to read Vite manifest")]
    FailedToRead,
    /// Missing information in the Vite manifest
    #[error("Missing information in Vite manifest")]
    Inconsistent,
}

type ViteManifest = BTreeMap<String, ViteManifestEntry>;
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ViteManifestEntry {
    file: String,
    // name: String,
    // src: String,
    is_entry: bool,
    is_dynamic_entry: bool,
    #[serde(default)]
    css: Vec<String>,
    #[serde(default)]
    imports: Vec<String>,
}

fn push_import(
    output: &mut String,
    base_url: &str,
    manifest: &BTreeMap<String, ViteManifestEntry>,
    import_name: &str,
) -> Result<(), Report<ManifestError>> {
    let entry = manifest
        .get(import_name)
        .ok_or(ManifestError::Inconsistent)
        .attach_printable_lazy(|| format!("failed to find import {}", import_name))?;

    output.push_str(&format!(
        r##"<link rel="modulepreload" href="{base_url}/{}" type="module" />"##,
        entry.file
    ));

    for file in &entry.css {
        format!(r##"<link rel="stylesheet" href="{base_url}/{}" />"##, file);
    }

    for import in &entry.imports {
        push_import(output, base_url, manifest, import)?;
    }

    Ok(())
}

impl Manifest {
    /// Create an empty manifest
    pub const fn new() -> Self {
        Self(RwLock::new(None))
    }

    /// Read a Vite manifest
    pub fn read_manifest(
        &self,
        base_url: &str,
        manifest_path: &Path,
    ) -> Result<(), Report<ManifestError>> {
        let manifest_file = std::fs::read_to_string(manifest_path);

        let manifest_file = match manifest_file {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(Report::new(ManifestError::NotFound));
            }
            file => file
                .change_context(ManifestError::FailedToRead)
                .attach_printable("failed to read manifest")?,
        };

        let manifest: ViteManifest = serde_json::from_str(&manifest_file)
            .change_context(ManifestError::FailedToRead)
            .attach_printable("failed to parse manifest")?;

        let entries = manifest
            .iter()
            .filter(|(_, value)| value.is_entry || value.is_dynamic_entry)
            .map(|(key, value)| {
                let mut output = String::new();

                output.push_str(
                    format!(
                        r##"<script type="module" src="{base_url}/{}" defer></script>"##,
                        value.file
                    )
                    .as_str(),
                );

                for file in &value.imports {
                    push_import(&mut output, base_url, &manifest, file)?;
                }

                for file in &value.css {
                    output.push_str(
                        format!(r##"<link rel="stylesheet" href="{base_url}/{}">"##, file).as_str(),
                    )
                }

                Ok::<_, Report<ManifestError>>((key.to_string(), PreEscaped(output)))
            })
            .collect::<Result<HashMap<String, PreEscaped<String>>, _>>()?;

        let index_data = entries.get("index").unwrap().clone();

        let mut wrapper = self.0.write().unwrap();
        if let Some(data) = wrapper.as_mut() {
            data.index = index_data;
            data.entries = entries;
        } else {
            *wrapper = Some(ManifestData {
                index: index_data,
                entries,
            })
        }

        Ok(())
    }

    /// Get the HTML to include the JS and CSS for the index page
    pub fn index(&self) -> PreEscaped<String> {
        self.0
            .read()
            .unwrap()
            .as_ref()
            .expect("manifest not initialized")
            .index
            .clone()
    }

    /// Get the HTML to include the JS and CSS for an entrypoint
    pub fn get(&self, name: &str) -> Option<PreEscaped<String>> {
        self.0
            .read()
            .unwrap()
            .as_ref()
            .expect("manifest not initialized")
            .entries
            .get(name)
            .cloned()
    }
}

#[cfg(feature = "watch-manifest")]
/// Watch a Vite manifest for changes
pub mod watch {
    use std::{path::PathBuf, time::Duration};

    use notify_debouncer_mini::{
        new_debouncer,
        notify::{FsEventWatcher, RecursiveMode},
        DebounceEventResult, Debouncer,
    };

    use super::{Manifest, ManifestError};

    /// A file watcher for the Manifest
    pub type ManifestWatcher = Debouncer<FsEventWatcher>;

    /// Watch the manifest and reload when it changes
    #[must_use = "Dropping the watcher will cause watching to stop"]
    pub fn watch_manifest(
        base_url: String,
        manifest_path: PathBuf,
        manifest: &'static Manifest,
    ) -> ManifestWatcher {
        let path = manifest_path.clone();
        let mut watcher = new_debouncer(
            Duration::from_millis(500),
            move |res: DebounceEventResult| match res {
                Ok(_) => {
                    let result = manifest.read_manifest(&base_url, &manifest_path);
                    if let Err(e) = result {
                        if e.current_context() != &ManifestError::NotFound {
                            tracing::error!(?e, "failed to read manifest");
                        }
                    }
                }
                Err(e) => tracing::error!(?e, "manifest watcher error"),
            },
        )
        .unwrap();

        watcher
            .watcher()
            .watch(&path, RecursiveMode::NonRecursive)
            .unwrap();

        watcher
    }
}
