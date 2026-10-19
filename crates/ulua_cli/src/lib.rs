//! The offline Ulua developer command.
#![warn(missing_docs)]
mod diagnostic;
mod native;
mod script;
use std::{ffi::OsString, io::Write};

const USAGE: &str = "usage: ulua check <file.ts>\n       ulua run <file.ts>\n       ulua link-flags [--cc unix|msvc] [--lib <file>] [--include <dir>]\n       ulua build --host <file>... [-o <dir>] [--lib <file>] [--include <dir>] [--run [-- <arg>...]]\n       ulua help\n";

type Failure = (i32, String);
type Result<T> = std::result::Result<T, Failure>;
fn io_error(e: impl std::fmt::Display) -> Failure {
    (2, diagnostic::escape(&e.to_string(), false))
}
fn path_error(path: &std::path::Path, error: std::io::Error) -> Failure {
    io_error(format!("{}: {error}", path.display()))
}
fn usage_error(reason: impl std::fmt::Display) -> Failure {
    (
        2,
        format!(
            "ulua: {}\n{}",
            diagnostic::escape(&reason.to_string(), false),
            USAGE.trim_end()
        ),
    )
}

/// Execute arguments after the binary name, and write the command streams.
/// The host of `build --run` inherits process streams instead of these writers.
/// Return the process exit code. This function never exits the process.
pub fn execute<I, S>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let result = match args.first().and_then(|s| s.to_str()) {
        None if args.is_empty() => stdout
            .write_all(USAGE.as_bytes())
            .map(|_| 0)
            .map_err(|e| io_error(format!("stdout: {e}"))),
        Some("help" | "--help") if args.len() == 1 => stdout
            .write_all(USAGE.as_bytes())
            .map(|_| 0)
            .map_err(|e| io_error(format!("stdout: {e}"))),
        Some(command @ ("check" | "run"))
            if args.len() == 2 && !args[1].to_string_lossy().starts_with('-') =>
        {
            script::execute(command, &args[1], stdout, stderr)
        }
        Some("link-flags" | "build") => native::execute(&args, stdout, stderr),
        Some(command @ ("check" | "run")) => Err(usage_error(if args.len() == 1 {
            format!("{command}: missing <file.ts>")
        } else {
            format!(
                "{command}: unexpected argument {}",
                args[if args.len() > 2 { 2 } else { 1 }].display()
            )
        })),
        Some("help" | "--help") => Err(usage_error(format!(
            "unexpected argument {}",
            args[1].display()
        ))),
        _ => Err(usage_error(format!(
            "unknown command {}",
            args[0].display()
        ))),
    };
    match result {
        Ok(code) => code,
        Err((code, message)) => {
            if writeln!(stderr, "{message}").is_err() {
                2
            } else {
                code
            }
        }
    }
}
#[cfg(test)]
mod tests;
