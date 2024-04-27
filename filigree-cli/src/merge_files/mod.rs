use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use error_stack::{Report, ResultExt};

use crate::write::RenderedFile;

#[derive(Debug)]
pub struct MergeTracker {
    pub base_generated_path: PathBuf,
    output_path: PathBuf,
    overwrite: bool,
}

impl MergeTracker {
    pub fn new(base_generated_path: PathBuf, output_path: PathBuf, overwrite: bool) -> Self {
        Self {
            base_generated_path,
            output_path,
            overwrite,
        }
    }

    fn internal_file_path(&self, path: &Path) -> PathBuf {
        let path = self.base_generated_path.join(path);
        let new_file = format!("{}.gen", path.display());
        PathBuf::from(new_file)
    }

    pub fn from_rendered_file(&self, file: RenderedFile) -> MergeFile {
        self.file(file.path, String::from_utf8(file.contents).unwrap())
    }

    /// Go backwards from an internal path to the output path it represents
    fn empty_from_internal_file(&self, path: &Path) -> MergeFile {
        let mut relative = pathdiff::diff_paths(path, &self.base_generated_path).unwrap();
        let r = relative
            .file_name()
            .unwrap()
            .to_string_lossy()
            .strip_suffix(".gen")
            .unwrap()
            .to_string();
        relative.set_file_name(r);

        self.file(relative, String::new())
    }

    /// Generate a list of files that are in the state but were not generated on this run.
    pub fn generate_empty_files(
        &self,
        existing_files: &[MergeFile],
        always_keep: &[&str],
    ) -> Vec<MergeFile> {
        let mut with_content = existing_files
            .iter()
            .map(|f| f.base_generated_path.clone())
            .collect::<HashSet<_>>();

        for name in always_keep {
            with_content.insert(self.internal_file_path(Path::new(name)));
        }

        let walker = ignore::Walk::new(&self.base_generated_path);

        walker
            .flatten()
            .filter(|entry| {
                let path = entry.path();
                entry.file_type().map(|f| f.is_file()).unwrap_or(false)
                    && path.extension().unwrap_or_default() == "gen"
                    && !with_content.contains(path)
            })
            .map(|entry| self.empty_from_internal_file(entry.path()))
            .collect()
    }

    pub fn file(&self, path: PathBuf, new_output: String) -> MergeFile {
        let base_generated_path = self.internal_file_path(&path);

        let output_path = self.output_path.join(&path);

        let previous_generation_result = std::fs::read_to_string(&base_generated_path);
        let gen_exists = previous_generation_result.is_ok();
        let previous_generation = previous_generation_result.ok();
        let users_file = if self.overwrite {
            None
        } else {
            std::fs::read_to_string(&output_path).ok()
        };

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
        let generation_changed = previous_generation.map(|p| p != new_output).unwrap_or(true);
        let output_changed = users_file
            .as_ref()
            .map(|u| u.trim() != merged.output.trim())
            .unwrap_or(!empty);

        MergeFile {
            generation_changed,
            output_changed,
            base_generated_path,
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
    previous_generation: Option<&str>,
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
    pub base_generated_path: PathBuf,
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
            if self.gen_exists {
                std::fs::remove_file(&self.base_generated_path).ok();
            }
        } else if self.generation_changed {
            std::fs::write(&self.base_generated_path, self.this_generation.as_bytes())
                .attach_printable_lazy(|| self.base_generated_path.display().to_string())?;
        }

        if self.remove_user_file {
            println!("Removing file {}", self.output_relative_path.display());
            std::fs::remove_file(&self.output_path)
                .attach_printable_lazy(|| self.output_path.display().to_string())?;
        } else if self.output_changed {
            println!("Writing file {}", self.output_relative_path.display());
            std::fs::write(&self.output_path, self.merged.output.as_bytes())
                .attach_printable_lazy(|| self.output_path.display().to_string())?;
        } else if self.empty && self.gen_exists {
            println!(
                "Not removing empty file {} because it has been modified",
                self.output_relative_path.display()
            );
        }

        Ok(())
    }
}
