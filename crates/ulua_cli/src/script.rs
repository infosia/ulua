//! Product ABI compilation and task execution.
use crate::{Result, diagnostic, io_error};
use std::{
    ffi::{OsStr, c_char, c_void},
    io::{Read, Write},
    ptr,
};
use ulua::*;

struct Objects {
    context: *mut Context,
    module: *mut Module,
    instance: *mut Instance,
}
impl Drop for Objects {
    fn drop(&mut self) {
        // SAFETY: These objects belong to this context and are freed once, in dependency order.
        unsafe {
            ulua_instance_free(self.instance);
            ulua_module_free(self.module);
            ulua_context_free(self.context);
        }
    }
}
struct Output<'a> {
    writer: &'a mut dyn Write,
}
unsafe extern "C" fn output(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The userdata is live. Text is readable for nonzero length; zero length permits null.
    unsafe {
        let out = &mut *userdata.cast::<Output<'_>>();
        let bytes = if length == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(text.cast::<u8>(), length)
        };
        // A writer can panic. Keep that panic inside the callback boundary.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            out.writer.write_all(bytes)?;
            out.writer.write_all(b"\n")?;
            out.writer.flush()
        })) {
            Ok(Ok(())) => 0,
            _ => 1,
        }
    }
}
/// Compile a script and optionally run its main task through the C ABI.
pub(crate) fn execute(
    command: &str,
    file: &OsStr,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<i32> {
    let mut limits = CompileLimits::default();
    // SAFETY: The initialized limits descriptor is live and writable.
    let status = unsafe { ulua_default_compile_limits(&mut limits) };
    if status != ULUA_OK {
        return Err(status_failure(status, &Diagnostic::default(), file, b""));
    }
    let source = read_source(
        std::fs::File::open(file).map_err(|e| crate::path_error(std::path::Path::new(file), e))?,
        limits.max_source_bytes,
    )
    .map_err(|e| crate::path_error(std::path::Path::new(file), e))?;
    // Declare userdata before its owner so every exit frees the instance first.
    let mut out = Output { writer: stdout };
    let mut objects = Objects {
        context: ptr::null_mut(),
        module: ptr::null_mut(),
        instance: ptr::null_mut(),
    };
    let mut diag = Diagnostic::default();
    let fail = |status, diag: &Diagnostic| -> Result<i32> {
        Err(status_failure(status, diag, file, &source))
    };
    // SAFETY: All descriptors have ABI defaults. Objects remain live and calls are serialized.
    unsafe {
        let status = ulua_context_new(&mut objects.context);
        if status != ULUA_OK {
            return fail(status, &diag);
        }
        let status = ulua_compile(
            objects.context,
            source.as_ptr().cast(),
            source.len(),
            ptr::null(),
            0,
            ptr::null(),
            &mut objects.module,
            &mut diag,
        );
        if status != ULUA_OK {
            return fail(status, &diag);
        }
        if command == "check" {
            writeln!(stderr, "check: {}: no errors", diagnostic::filename(file))
                .map_err(|e| io_error(format!("stderr: {e}")))?;
            return Ok(0);
        }
        let mut index = u32::MAX;
        let status = ulua_module_find_export(objects.module, c"main".as_ptr(), 4, &mut index);
        let missing = status == ULUA_INVALID_ARGUMENT && index == u32::MAX;
        if !missing {
            if status != ULUA_OK {
                return fail(status, &diag);
            }
            let mut count = 0;
            let mut result_type = 0;
            let status = ulua_module_export_signature(
                objects.module,
                index,
                ptr::null_mut(),
                0,
                &mut count,
                &mut result_type,
            );
            if count > 0 {
                return Err(crate::usage_error(format!(
                    "run: {}: export main must have no parameters",
                    diagnostic::filename(file)
                )));
            }
            if status != ULUA_OK {
                return fail(status, &diag);
            }
        }
        let mut host = Host::default();
        host.userdata = ptr::from_mut(&mut out).cast();
        host.output = Some(output);
        let status = ulua_instance_new(
            objects.context,
            objects.module,
            &host,
            ptr::null(),
            &mut objects.instance,
            &mut diag,
        );
        if status != ULUA_OK {
            return fail(status, &diag);
        }
        if missing {
            writeln!(
                stderr,
                "run: {}: no export main",
                diagnostic::filename(file)
            )
            .map_err(|e| io_error(format!("stderr: {e}")))?;
            return Ok(0);
        }
        let mut task = 0;
        let status = ulua_call_start(
            objects.instance,
            index,
            ptr::null(),
            0,
            &mut task,
            &mut diag,
        );
        if status != ULUA_OK {
            return fail(status, &diag);
        }
        let mut settled = 0;
        let status = ulua_pump(objects.instance, 0, &mut settled, &mut diag);
        if status != ULUA_OK {
            return fail(status, &diag);
        }
        let status = ulua_task_result(objects.instance, task, &mut Value::default(), &mut diag);
        if status == ULUA_PENDING {
            return Err((
                1,
                diagnostic::render(
                    "async:pending",
                    "The main task is still pending",
                    "",
                    "",
                    0,
                    0,
                )
                .trim_end_matches('\n')
                .into(),
            ));
        }
        if status != ULUA_OK {
            return fail(status, &diag);
        }
    }
    Ok(0)
}

/// Read no more than the source ceiling and one sentinel byte.
pub(crate) fn read_source(reader: impl Read, maximum: u64) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}
/// Render a product failure, including statuses that carry no diagnostic.
pub(crate) fn status_failure(
    status: Status,
    diag: &Diagnostic,
    file: &OsStr,
    source: &[u8],
) -> crate::Failure {
    let code = diagnostic::cstring(&diag.code);
    if code.is_empty() {
        let name = match status {
            ULUA_OK => "ULUA_OK",
            ULUA_INVALID_ARGUMENT => "ULUA_INVALID_ARGUMENT",
            ULUA_REJECTED => "ULUA_REJECTED",
            ULUA_FAILED => "ULUA_FAILED",
            ULUA_HOST_ERROR => "ULUA_HOST_ERROR",
            ULUA_BUSY => "ULUA_BUSY",
            ULUA_INTERNAL => "ULUA_INTERNAL",
            ULUA_POISONED => "ULUA_POISONED",
            ULUA_PENDING => "ULUA_PENDING",
            _ => "ULUA_UNKNOWN",
        };
        return (1, format!("error[abi:status]: {name} ({status})"));
    }
    (
        1,
        diagnostic::render(
            &code,
            &diagnostic::cstring(&diag.message),
            &file.to_string_lossy(),
            source,
            diag.line,
            diag.column,
        )
        .trim_end_matches('\n')
        .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn null_zero_length_output_is_an_empty_line() {
        let mut bytes = Vec::new();
        {
            let mut out = Output { writer: &mut bytes };
            // SAFETY: The userdata is live, and the ABI permits a null pointer with length zero.
            let status = unsafe { output(ptr::from_mut(&mut out).cast(), ptr::null(), 0) };
            assert_eq!(status, 0);
        }
        assert_eq!(bytes, b"\n");
    }
}
