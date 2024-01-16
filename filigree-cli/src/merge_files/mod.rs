use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};

use crate::RenderedFile;

#[derive(Debug)]
pub struct MergeTracker {
    pub base_generated_path: PathBuf,
    output_path: PathBuf,
}

impl MergeTracker {
    pub fn new(base_generated_path: PathBuf, output_path: PathBuf) -> Self {
        Self {
            base_generated_path,
            output_path,
        }
    }

    fn internal_file_path(&self, path: &Path) -> PathBuf {
        let path = self.base_generated_path.join(path);
        let new_file = format!("{}.gen", path.display());
        PathBuf::from(new_file)
    }

    pub fn from_rendered_file(&self, file: RenderedFile) -> MergeFile {
        self.file(&file.path, String::from_utf8(file.contents).unwrap())
    }

    pub fn file(&self, path: &Path, new_output: String) -> MergeFile {
        let base_generated_path = self.internal_file_path(path);

        let output_path = self.output_path.join(&path);

        let previous_generation = std::fs::read_to_string(&base_generated_path).ok();
        let users_file = std::fs::read_to_string(&output_path).ok();

        let merged = generate_merged_output(
            previous_generation.as_deref(),
            &new_output,
            users_file.as_deref(),
        );

        let generation_changed = previous_generation.map(|p| p != new_output).unwrap_or(true);
        let output_changed = users_file.map(|u| u != new_output).unwrap_or(true);

        MergeFile {
            generation_changed,
            output_changed,
            base_generated_path,
            output_path,
            this_generation: new_output,
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
        (None, Some(users_file)) => diffy::merge("", this_generation, users_file).into(),
        (Some(previous), Some(users_file)) => {
            diffy::merge(previous, this_generation, users_file).into()
        }
    }
}

pub struct MergeFile {
    base_generated_path: PathBuf,
    pub output_path: PathBuf,

    pub generation_changed: bool,
    pub output_changed: bool,

    pub this_generation: String,
    pub merged: MergeOutput,
}

impl MergeFile {
    pub fn write(&self) -> Result<(), Report<std::io::Error>> {
        if self.generation_changed {
            std::fs::write(&self.base_generated_path, self.this_generation.as_bytes())
                .attach_printable_lazy(|| self.base_generated_path.display().to_string())?;
        }

        if self.output_changed {
            std::fs::write(&self.output_path, self.merged.output.as_bytes())
                .attach_printable_lazy(|| self.output_path.display().to_string())?;
        }

        Ok(())
    }
}
