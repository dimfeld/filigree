use std::{io::Write, process::Stdio};

use error_stack::{Report, ResultExt};
use serde::Deserialize;

use crate::Error;

#[derive(Deserialize, Debug, Default)]
pub struct Formatters {
    /// The formatter to use for Rust code. Defaults to rustfmt.
    pub rust: Option<String>,
    /// The formatter to use for SQL files.
    pub sql: Option<String>,
}

impl Formatters {
    pub fn run_formatter(&self, filename: &str, input: Vec<u8>) -> Result<Vec<u8>, Report<Error>> {
        let formatter = if filename.ends_with(".sql") {
            self.sql.as_deref()
        } else if filename.ends_with(".rs") {
            self.rust.as_deref().or(Some("rustfmt"))
        } else {
            None
        };

        let formatter = formatter.filter(|s| !s.is_empty());

        let Some(formatter) = formatter else {
            return Ok(input);
        };

        let mut format_process = std::process::Command::new(formatter)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .change_context(Error::Formatter)?;

        let mut stdin = format_process.stdin.take().ok_or(Error::Formatter)?;
        let writer_thread =
            std::thread::spawn(move || stdin.write_all(&input).change_context(Error::Formatter));

        let result = format_process
            .wait_with_output()
            .change_context(Error::Formatter)?;

        writer_thread
            .join()
            .expect("format writer thread")
            .change_context(Error::Formatter)?;

        if !result.status.success() {
            let code = result.status.code().unwrap_or(0);
            return Err(Error::Formatter)
                .attach_printable(format!("Formatter {formatter} exited with code {code}"))
                .attach_printable(String::from_utf8(result.stderr).unwrap_or_default());
        }

        Ok(result.stdout)
    }
}
