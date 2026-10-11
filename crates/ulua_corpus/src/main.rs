//! Corpus runner: compile and execute through the product C ABI.
#![warn(missing_docs)]
use std::{
    ffi::{c_char, c_void},
    path::{Path, PathBuf},
    ptr,
};
use ulua::*;

struct ContextOwner(*mut Context);
impl Drop for ContextOwner {
    fn drop(&mut self) {
        // SAFETY: This owner keeps the context live and frees it exactly once, with all remaining children.
        unsafe {
            ulua_context_free(self.0);
        }
    }
}
struct Record {
    entry: String,
    status: &'static str,
    output: Vec<String>,
    code: Option<String>,
    message: Option<String>,
}
unsafe extern "C" fn collect(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The run passes a live output Vec and the shim guarantees readable UTF-8 text during this call.
    unsafe {
        let bytes = std::slice::from_raw_parts(text.cast::<u8>(), length);
        let Ok(text) = std::str::from_utf8(bytes) else {
            return 1;
        };
        (*userdata.cast::<Vec<String>>()).push(text.to_owned());
    }
    0
}
fn string(bytes: &[c_char]) -> String {
    let bytes: Vec<_> = bytes
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}
fn quote(text: &str) -> String {
    use std::fmt::Write;
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\u{20}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
impl Record {
    fn json(&self) -> String {
        let output = self
            .output
            .iter()
            .map(|s| quote(s))
            .collect::<Vec<_>>()
            .join(",");
        let code = self
            .code
            .as_ref()
            .map_or_else(|| "null".to_owned(), |s| quote(s));
        let message = self
            .message
            .as_ref()
            .map_or_else(String::new, |s| format!(",\"message\":{}", quote(s)));
        format!(
            "{{\"entry\":{},\"status\":{},\"output\":[{output}],\"code\":{code}{message}}}",
            quote(&self.entry),
            quote(self.status)
        )
    }
    fn fail(&mut self, status: Status, diagnostic: &Diagnostic) {
        self.status = match status {
            ULUA_REJECTED => "rejected",
            ULUA_FAILED | ULUA_HOST_ERROR => "failed",
            _ => "internal",
        };
        self.code = Some(string(&diagnostic.code));
        self.message = Some(string(&diagnostic.message));
    }
}
fn run_entry(path: &Path, emit: Option<&Path>) -> Result<Record, String> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Entry has no UTF-8 name")?;
    let directory = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .ok_or("Entry has no directory")?;
    let source = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut record = Record {
        entry: format!("{directory}/{name}"),
        status: "ok",
        output: vec![],
        code: None,
        message: None,
    };
    let mut context = ptr::null_mut();
    let mut diagnostic = Diagnostic::default();
    // SAFETY: Local descriptors, outputs, and source cover all stated lengths. The context owner serializes and frees all objects.
    unsafe {
        let status = ulua_context_new(&mut context);
        if status != ULUA_OK {
            record.fail(status, &diagnostic);
            return Ok(record);
        }
        let owner = ContextOwner(context);
        let mut compile_limits = CompileLimits::default();
        let mut run_limits = RunLimits::default();
        if ulua_default_compile_limits(&mut compile_limits) != ULUA_OK
            || ulua_default_run_limits(&mut run_limits) != ULUA_OK
        {
            return Err("Default limits failed".into());
        }
        let mut module = ptr::null_mut();
        let status = ulua_compile(
            owner.0,
            source.as_ptr().cast(),
            source.len(),
            &compile_limits,
            &mut module,
            &mut diagnostic,
        );
        if status != ULUA_OK {
            record.fail(status, &diagnostic);
            return Ok(record);
        }
        if let Some(directory) = emit {
            // Witness emission uses the same safe compiler. Product execution uses only the C ABI.
            let compiled = ulua_compiler::compile(&source, ulua_compiler::Limits::default())
                .map_err(|d| d.message)?;
            std::fs::write(directory.join(format!("{name}.luau")), compiled.luau)
                .map_err(|e| e.to_string())?;
        }
        let mut host = Host::default();
        host.userdata = ptr::from_mut(&mut record.output).cast();
        host.output = Some(collect);
        let mut instance = ptr::null_mut();
        let status = ulua_instance_new(
            owner.0,
            module,
            &host,
            &run_limits,
            &mut instance,
            &mut diagnostic,
        );
        if status != ULUA_OK {
            record.fail(status, &diagnostic);
            return Ok(record);
        }
        let mut index = u32::MAX;
        let status = ulua_module_find_export(module, c"main".as_ptr(), 4, &mut index);
        if status != ULUA_OK {
            record.fail(status, &diagnostic);
            return Ok(record);
        }
        let mut value = Value::default();
        let status = ulua_call(instance, index, ptr::null(), 0, &mut value, &mut diagnostic);
        if status != ULUA_OK {
            record.fail(status, &diagnostic);
        }
        if ulua_instance_free(instance) != ULUA_OK || ulua_module_free(module) != ULUA_OK {
            return Err("Object free failed".into());
        }
    }
    Ok(record)
}
fn run() -> Result<(), String> {
    let mut emit = None;
    let mut entries = vec![];
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--emit-luau" {
            emit = Some(PathBuf::from(
                args.next().ok_or("--emit-luau requires a directory")?,
            ));
        } else {
            entries.push(PathBuf::from(arg));
        }
    }
    if entries.is_empty() {
        return Err("usage: ulua-corpus [--emit-luau <dir>] <entry.ts>...".into());
    }
    if let Some(path) = &emit {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    }
    for path in entries {
        println!("{}", run_entry(&path, emit.as_deref())?.json());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("ulua-corpus: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_escapes_output() {
        assert_eq!(quote("a\n\"\0é"), "\"a\\n\\\"\\u0000é\"");
    }
}
