//! Single-worker analyzer calls report the process status, exit code, signal, and output.
//! Measured single-worker Debug cost: 0.24 seconds for analyzer_status.
use std::{
    ffi::OsStr,
    process::{Command, Output},
};

/// Construct an analyzer command with one worker.
pub fn command(executable: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(executable);
    command.arg("-j1");
    command
}

/// Format the process status, exit code, signal, and captured output.
pub fn failure(output: &Output) -> String {
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        output.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    format!(
        "status={} code={:?} signal={signal:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
