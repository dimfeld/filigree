use std::{io::Write, process::Stdio};

use error_stack::{Report, ResultExt};

use crate::Error;

pub fn run_formatter(formatter: &str, input: Vec<u8>) -> Result<Vec<u8>, Report<Error>> {
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

    let code = result.status.code().unwrap_or(0);
    if !result.status.success() {
        return Err(Error::Formatter)
            .attach_printable(format!("Formatter {formatter} exited with code {code}"))
            .attach_printable(String::from_utf8(result.stderr).unwrap_or_default());
    }

    Ok(result.stdout)
}
