use std::{collections::HashSet, path::Path};

use serde::{Deserialize, Serialize};

use crate::config::Config;

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
