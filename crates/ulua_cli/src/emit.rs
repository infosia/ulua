//! Developer-only Luau output from the same bounded Rust compiler as the ABI.
use crate::{Failure, Result, diagnostic, io_error, path_error, script, usage_error};
use std::{
    ffi::{OsStr, OsString, c_char},
    io::Write,
    path::Path,
};

/// Compile one file and write its unescaped Luau artifact.
pub(crate) fn execute(
    args: &[OsString],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<i32> {
    let (file, output) = arguments(args)?;
    let (source, abi_limits) = script::read_file(file)?;
    script::validate(file, &source)?;
    if let Some(output) = output {
        let input = std::fs::canonicalize(file).map_err(|e| path_error(Path::new(file), e))?;
        match std::fs::canonicalize(output) {
            Ok(path) if path == input => {
                return Err(usage_error(format!(
                    "emit: output {} names input {}",
                    output.display(),
                    file.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(path_error(Path::new(output), e)),
        }
    }
    // Keep this mapping identical to abi.rs compiler_limits; the remaining defaults belong to the compiler.
    let mut limits = ulua_compiler::Limits::default();
    limits.max_source_bytes = abi_limits.max_source_bytes;
    limits.max_nesting_depth = abi_limits.max_nesting_depth;
    let joined = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("ulua-compiler".into())
            .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
            .spawn_scoped(scope, || {
                ulua_compiler::compile_with_grants(&source, limits, &[])
            })
            .map(|worker| worker.join())
    });
    let compiled = finish_worker(joined, file, &source)?
        .map_err(|d| failure(d.code, &d.message, d.line, d.column, file, &source))?;
    if let Some(output) = output {
        replace_output(Path::new(output), |new| {
            new.write_all(compiled.luau.as_bytes())
        })?;
        writeln!(
            stderr,
            "emit: {}: {}",
            diagnostic::filename(file),
            diagnostic::filename(output)
        )
        .map_err(|e| io_error(format!("stderr: {e}")))?;
    } else {
        stdout
            .write_all(compiled.luau.as_bytes())
            .map_err(|e| io_error(format!("stdout: {e}")))?;
    }
    Ok(0)
}
fn arguments(args: &[OsString]) -> Result<(&OsStr, Option<&OsStr>)> {
    let mut file = None;
    let mut output = None;
    let mut arguments = args.iter().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "-o" && output.is_none() {
            let value = arguments
                .next()
                .filter(|value| !value.is_empty() && !value.to_string_lossy().starts_with('-'))
                .ok_or_else(|| usage_error("emit: missing output file for -o"))?;
            output = Some(value.as_os_str());
        } else if file.is_none()
            && !argument.is_empty()
            && !argument.to_string_lossy().starts_with('-')
        {
            file = Some(argument.as_os_str());
        } else {
            return Err(usage_error(format!(
                "emit: unexpected argument {}",
                argument.display()
            )));
        }
    }
    Ok((
        file.ok_or_else(|| usage_error("emit: missing <file.ts>"))?,
        output,
    ))
}

struct Temporary(std::path::PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn replace_output(
    output: &Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temporary, mut file) = loop {
        let path = directory.join(format!(
            ".ulua-emit-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => break (Temporary(path), file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(path_error(output, e)),
        }
    };
    // Close the file before cleanup or rename, including a failed write.
    let result = write(&mut file).and_then(|()| file.sync_all());
    drop(file);
    result.map_err(|e| path_error(output, e))?;
    std::fs::rename(&temporary.0, output).map_err(|e| path_error(output, e))?;
    Ok(())
}

fn finish_worker<T>(
    result: std::io::Result<std::thread::Result<T>>,
    file: &OsStr,
    source: &[u8],
) -> Result<T> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(failure(
            "internal",
            "Compiler thread panicked",
            0,
            0,
            file,
            source,
        )),
        Err(e) => Err(failure(
            "host:resources",
            &e.to_string(),
            0,
            0,
            file,
            source,
        )),
    }
}
fn failure(
    code: &str,
    message: &str,
    line: u32,
    column: u32,
    file: &OsStr,
    source: &[u8],
) -> Failure {
    // Apply the ABI's NUL-terminated field widths and UTF-8 truncation before the shared renderer.
    let mut diag = ulua::Diagnostic::default();
    copy_text(&mut diag.code, code);
    copy_text(&mut diag.message, message);
    diag.line = line;
    diag.column = column;
    script::status_failure(ulua::ULUA_REJECTED, &diag, file, source)
}
fn copy_text(out: &mut [c_char], text: &str) {
    let mut length = text.len().min(out.len().saturating_sub(1));
    while !text.is_char_boundary(length) {
        length -= 1;
    }
    for (target, byte) in out.iter_mut().zip(&text.as_bytes()[..length]) {
        *target = *byte as c_char;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_write_preserves_output_and_removes_temporary() {
        let directory =
            std::env::temp_dir().join(format!("ulua-emit-write-failure-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let output = directory.join("old.luau");
        std::fs::write(&output, b"old output").unwrap();
        let error = replace_output(&output, |file| {
            file.write_all(b"partial output")?;
            Err(std::io::Error::other("injected write failure"))
        })
        .unwrap_err();
        assert_eq!(error.0, 2);
        assert!(error.1.contains("injected write failure"));
        assert_eq!(std::fs::read(&output).unwrap(), b"old output");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn worker_failures_and_abi_message_width() {
        let result = finish_worker::<()>(
            Err(std::io::Error::other("thread refused")),
            OsStr::new("x"),
            b"bad",
        )
        .unwrap_err();
        assert_eq!(result, (1, "error[host:resources]: thread refused\n  = rule: the OS refused a compiler thread or its resources".into()));
        let result =
            finish_worker::<()>(Ok(Err(Box::new("panic"))), OsStr::new("x"), b"bad").unwrap_err();
        assert!(
            result
                .1
                .starts_with("error[internal]: Compiler thread panicked\n")
        );
        let text = failure("syntax", &"é".repeat(200), 0, 0, OsStr::new("x"), b"").1;
        assert_eq!(
            text.lines().next().unwrap(),
            format!("error[syntax]: {}", "é".repeat(127))
        );
    }
}
