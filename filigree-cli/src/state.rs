use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{config::Config, Error};

/// State that is not represented in the config file, but is relevant to the application generator.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// Background jobs
    pub background_jobs: ActiveAndRemoved,
}

impl State {
    /// Read the state from the given directory, or return an empty state.
    pub fn from_dir(dir: &Path) -> Self {
        Self::try_read(dir).unwrap_or_default()
    }

    fn try_read(dir: &Path) -> Option<Self> {
        let data = std::fs::read(dir.join("state.json")).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn update_from_config(&mut self, _config: &Config) {
        // Update background jobs once there is config for them
    }

    pub fn save(&self, dir: &Path) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(dir)?;
        let data = serde_json::to_vec_pretty(self)?;
        std::fs::write(dir.join("state.json"), data)?;
        Ok(())
    }
}

/// List items which are present and those which have been removed.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ActiveAndRemoved {
    active: Vec<String>,
    removed: Vec<String>,
}

impl ActiveAndRemoved {
    /// Update the active and removed lists given a new list of active items.
    #[allow(dead_code)] // might use this at some point
    pub fn with_updated_active_list(&self, new_list: Vec<String>) -> Self {
        let mut removed = self.removed.iter().collect::<HashSet<_>>();

        for item in &self.active {
            if !new_list.contains(item) {
                removed.insert(item);
            }
        }

        for item in &new_list {
            removed.remove(&item);
        }

        ActiveAndRemoved {
            removed: removed.into_iter().map(|s| s.to_string()).collect(),
            active: new_list,
        }
    }
}

struct GeneratedInner {
    files: BTreeMap<PathBuf, Arc<String>>,
    changed: bool,
}

impl GeneratedInner {
    fn new(files: BTreeMap<PathBuf, Arc<String>>) -> Self {
        Self {
            files,
            changed: false,
        }
    }
}

pub struct GeneratedFiles {
    inner: Mutex<GeneratedInner>,
}

impl GeneratedFiles {
    pub fn read(base_dir: &Path) -> Result<Self, Report<Error>> {
        let tar_path = base_dir.join("generated.tar");
        let file = match std::fs::File::open(&tar_path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let old_gen_files = try_read_old_gen_files(base_dir).unwrap_or_default();
                let mut inner = GeneratedInner::new(old_gen_files);
                // Since the archive doesn't exist yet, we always want to write it.
                inner.changed = true;

                return Ok(Self {
                    inner: Mutex::new(inner),
                });
            }
            Err(e) => return Err(e).change_context(Error::ReadGeneratedState),
        };

        let mut archive = tar::Archive::new(std::io::BufReader::new(file));
        let files = archive
            .entries()
            .change_context(Error::ReadGeneratedState)
            .attach_printable("Error while reading generated state archive")?
            .map(|entry| {
                let entry = entry?;

                let path = entry.path()?.into_owned();
                let contents = std::io::read_to_string(entry)?;

                Ok((path, Arc::new(contents)))
            })
            .collect::<Result<BTreeMap<PathBuf, _>, std::io::Error>>()
            .change_context(Error::ReadGeneratedState)
            .attach_printable("Error while reading generated state archive")?;

        Ok(Self {
            inner: Mutex::new(GeneratedInner::new(files)),
        })
    }

    pub fn get(&self, path: &Path) -> Option<Arc<String>> {
        let inner = self.inner.lock().unwrap();
        inner.files.get(path).cloned()
    }

    pub fn remove(&self, path: &Path) {
        let mut inner = self.inner.lock().unwrap();
        let result = inner.files.remove(path);
        if result.is_some() {
            inner.changed = true;
        }
    }

    pub fn insert(&self, path: PathBuf, contents: String) {
        let contents = Arc::new(contents);
        let mut inner = self.inner.lock().unwrap();
        let previous = inner.files.insert(path, contents.clone());
        let changed = previous
            .map(|previous| previous != contents)
            .unwrap_or(true);
        if changed {
            inner.changed = true;
        }
    }

    pub fn write(&self, base_dir: &Path) -> Result<(), Report<Error>> {
        let tar_path = base_dir.join("generated.tar");
        let inner = self.inner.lock().unwrap();

        if !inner.changed {
            return Ok(());
        }

        let mut archive = tar::Builder::new(std::io::BufWriter::new(
            std::fs::File::create(&tar_path)
                .change_context(Error::WriteFile)
                .attach_printable_lazy(|| tar_path.display().to_string())?,
        ));

        for (path, contents) in &inner.files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_cksum();
            archive
                .append_data(&mut header, &path, contents.as_bytes())
                .change_context(Error::WriteFile)
                .attach_printable_lazy(|| tar_path.display().to_string())?;
        }

        let mut writer = archive
            .into_inner()
            .change_context(Error::WriteFile)
            .attach_printable_lazy(|| tar_path.display().to_string())?;
        writer
            .flush()
            .change_context(Error::WriteFile)
            .attach_printable_lazy(|| tar_path.display().to_string())?;

        Ok(())
    }
}

fn try_read_old_gen_files(
    base_dir: &Path,
) -> Result<BTreeMap<PathBuf, Arc<String>>, std::io::Error> {
    let files = glob::glob(&format!("{}/**/*.gen", base_dir.display())).unwrap();

    let mut result = BTreeMap::new();
    for path in files {
        let Ok(path) = path else {
            continue;
        };

        let rel = path
            .strip_prefix(base_dir)
            .unwrap()
            .to_owned()
            .with_extension("");
        println!("Reading old generated state for {:?}", rel);
        let contents = std::fs::read_to_string(&path)?;
        result.insert(rel, Arc::new(contents));
    }

    Ok(result)
}
