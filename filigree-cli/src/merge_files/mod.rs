use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use error_stack::{Report, ResultExt};

use crate::{state::GeneratedFiles, RenderedFile};

pub struct MergeTracker {
    pub generated: Arc<GeneratedFiles>,
    pub prefix: PathBuf,
    output_path: PathBuf,
}

impl std::fmt::Debug for MergeTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MergeTracker")
            .field("output_path", &self.output_path)
            .finish()
    }
}

impl MergeTracker {
    pub fn new(generated: Arc<GeneratedFiles>, prefix: String, output_path: PathBuf) -> Self {
        Self {
            generated,
            prefix: PathBuf::from(prefix),
            output_path,
        }
    }

    pub fn from_rendered_file(&self, file: RenderedFile) -> MergeFile {
        self.file(file.path, String::from_utf8(file.contents).unwrap())
    }

    pub fn file(&self, path: PathBuf, new_output: String) -> MergeFile {
        let output_path = self.output_path.join(&path);

        let state_path = self.prefix.join(&path);
        let previous_generation = self.generated.get(&state_path);
        let gen_exists = previous_generation.is_some();
        let users_file = std::fs::read_to_string(&output_path).ok();

        let merged = generate_merged_output(
            previous_generation.as_deref(),
            &new_output,
            users_file.as_deref(),
        );

        let empty = new_output.trim().is_empty();
        // Can remove the user file if it hadn't changed since the previous generation, and this
        // one is empty.
        let remove_user_file = users_file
            .as_ref()
            .zip(previous_generation.as_ref())
            .map(|u| empty && u.0.trim() == u.1.trim())
            .unwrap_or(false);
        let generation_changed = previous_generation
            .map(|p| *p != new_output)
            .unwrap_or(true);
        let output_changed = users_file
            .as_ref()
            .map(|u| u.trim() != merged.output.trim())
            .unwrap_or(!empty);

        MergeFile {
            generation_changed,
            output_changed,
            generated: self.generated.clone(),
            state_path,
            output_path,
            output_relative_path: path,
            this_generation: new_output,
            gen_exists,
            empty,
            remove_user_file,
            merged,
        }
    }
}

pub struct MergeOutput {
    pub output: String,
    pub conflicts: bool,
}

impl From<Result<String, String>> for MergeOutput {
    fn from(result: Result<String, String>) -> Self {
        match result {
            Ok(output) => MergeOutput {
                output,
                conflicts: false,
            },
            Err(output) => MergeOutput {
                output,
                conflicts: true,
            },
        }
    }
}

fn generate_merged_output(
    previous_generation: Option<&String>,
    this_generation: &str,
    users_file: Option<&str>,
) -> MergeOutput {
    match (previous_generation, users_file) {
        (None, None) => MergeOutput {
            output: this_generation.to_string(),
            conflicts: false,
        },
        (Some(_), None) => MergeOutput {
            output: this_generation.to_string(),
            conflicts: false,
        },
        (None, Some(users_file)) => diffy::merge("", users_file, this_generation).into(),
        (Some(previous), Some(users_file)) => {
            diffy::merge(previous, users_file, this_generation).into()
        }
    }
}

pub struct MergeFile {
    generated: Arc<GeneratedFiles>,
    state_path: PathBuf,
    pub output_path: PathBuf,
    pub output_relative_path: PathBuf,

    pub generation_changed: bool,
    pub output_changed: bool,

    pub this_generation: String,
    pub merged: MergeOutput,
    /// If true, the previous generation file exists.
    pub gen_exists: bool,
    /// If true, the generated file was empty after trimming whitespace.
    pub empty: bool,
    /// If true, the generated file is empty, and the user's file has not been customized at all,
    /// so it's safe to remove it.
    pub remove_user_file: bool,
}

impl MergeFile {
    pub fn write(&self) -> Result<(), Report<std::io::Error>> {
        if self.empty {
            self.generated.remove(&self.state_path);
        } else if self.generation_changed {
            self.generated
                .insert(self.state_path.clone(), self.this_generation.clone());
        }

        if self.remove_user_file {
            println!("Removing file {}", self.output_relative_path.display());
            std::fs::remove_file(&self.output_path)
                .attach_printable_lazy(|| self.output_path.display().to_string())?;
        } else if self.output_changed {
            println!("Writing file {}", self.output_relative_path.display());
            std::fs::write(&self.output_path, self.merged.output.as_bytes())
                .attach_printable_lazy(|| self.output_path.display().to_string())?;
        }

        Ok(())
    }
}
