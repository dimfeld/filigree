use std::{io::Write, path::PathBuf, process::Stdio};

use error_stack::{Report, ResultExt};
use serde::Deserialize;

use crate::Error;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct FormatterConfig {
    /// The formatter to use for Rust code. Defaults to rustfmt.
    pub rust: Option<Vec<String>>,
    /// The formatter to use for Javascript and Typescript code. Defaults to pretter
    pub js: Option<Vec<String>>,
    /// The formatter to use for SQL files.
    pub sql: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct Formatters {
    pub config: FormatterConfig,
    pub api_base_dir: PathBuf,
    pub web_base_dir: PathBuf,
}

impl Formatters {
    pub fn run_formatter(&self, filename: &str, input: Vec<u8>) -> Result<Vec<u8>, Report<Error>> {
        let (base_dir, formatter) = if filename.ends_with(".sql") {
            (&self.api_base_dir, self.config.sql.clone())
        } else if filename.ends_with(".rs") {
            (
                &self.api_base_dir,
                self.config.rust.clone().or(Some(vec![
                    "rustfmt".to_string(),
                    "--edition".to_string(),
                    "2021".to_string(),
                ])),
            )
        } else if filename.ends_with(".ts") || filename.ends_with(".js") {
            (
                &self.web_base_dir,
                self.config.js.clone().or(Some(vec![
                    "prettier".to_string(),
                    "--stdin-path=stdin.ts".to_string(),
                ])),
            )
        } else {
            (&self.api_base_dir, None)
        };

        let formatter = formatter.filter(|s| !s.is_empty());

        let Some(formatter) = formatter else {
            return Ok(input);
        };

        let args = if formatter.len() > 1 {
            &formatter[1..]
        } else {
            &[]
        };

        let mut format_process = std::process::Command::new(&formatter[0])
            .current_dir(base_dir)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .change_context(Error::Formatter)
            .attach_printable_lazy(|| format!("Tried to run {}", formatter.join(" ")))
            .attach_printable_lazy(|| format!("on {filename}"))?;

        let mut stdin = format_process.stdin.take().ok_or(Error::Formatter)?;
        let writer_thread =
            std::thread::spawn(move || stdin.write_all(&input).change_context(Error::Formatter));

        let result = format_process
            .wait_with_output()
            .change_context(Error::Formatter)
            .attach_printable_lazy(|| filename.to_string())?;

        writer_thread
            .join()
            .expect("format writer thread")
            .change_context(Error::Formatter)
            .attach_printable_lazy(|| filename.to_string())?;

        if !result.status.success() {
            let code = result.status.code().unwrap_or(0);
            return Err(Error::Formatter)
                .attach_printable(format!("Formatting file {}", filename))
                .attach_printable(format!(
                    "Formatter {cmd} exited with code {code}",
                    cmd = formatter.join(" ")
                ))
                .attach_printable(String::from_utf8(result.stderr).unwrap_or_default())
                .attach_printable(String::from_utf8(result.stdout).unwrap_or_default());
        }

        Ok(result.stdout)
    }
}
