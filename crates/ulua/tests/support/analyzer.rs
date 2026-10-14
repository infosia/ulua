//! Single-worker analyzer calls report the process status, exit code, signal, and output.
//! Measured single-worker Debug cost: 0.24 seconds for analyzer_status.
use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Output},
};

/// Construct an analyzer command with one worker.
pub fn command(executable: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(executable);
    command.arg("-j1");
    command
}

/// Pass sibling fixtures relative to their directory to bound command length.
#[allow(dead_code)]
fn files(executable: impl AsRef<OsStr>, paths: &[impl AsRef<Path>]) -> Command {
    let mut cmd = command(executable);
    let directory = paths[0].as_ref().parent().unwrap();
    cmd.current_dir(directory);
    for path in paths {
        let path = path.as_ref();
        assert_eq!(path.parent(), Some(directory));
        cmd.arg(path.file_name().unwrap());
    }
    cmd
}

/// Analyze every fixture, splitting Windows batches below its command line limit.
#[allow(dead_code)]
pub fn output(executable: impl AsRef<OsStr>, paths: &[impl AsRef<Path>]) -> Output {
    // Windows limits CreateProcess to 32767 UTF-16 units. Each file name is at most 255.
    let size = if cfg!(windows) { 64 } else { paths.len() };
    let mut outputs = paths
        .chunks(size)
        .map(|batch| files(&executable, batch).output().unwrap());
    let mut result = outputs.next().unwrap();
    for out in outputs {
        if result.status.success() {
            result.status = out.status;
        }
        result.stdout.extend(out.stdout);
        result.stderr.extend(out.stderr);
    }
    result
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
