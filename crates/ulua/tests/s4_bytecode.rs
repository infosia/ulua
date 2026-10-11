//! Assignment assertions erase. Initializer assertions can change native optimizations.
// The stress source generators serve both the CLI and this bytecode comparison.
#[allow(dead_code)]
#[path = "../../ulua_corpus/src/stress.rs"]
mod stress;
use std::{
    ffi::{CStr, c_char},
    ptr,
};
use ulua_compiler::{COMPILER_STACK_BYTES, Limits};

// SAFETY: These declarations match the linked private compiler shim.
unsafe extern "C" {
    fn ulua_internal_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        size: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(out: *mut c_char);
}
fn bytecode(source: &str) -> Vec<u8> {
    let mut out = ptr::null_mut();
    let mut size = 0;
    let mut error = [0; 256];
    // SAFETY: Source and output storage cover their lengths. This scope frees the native output once.
    unsafe {
        let status = ulua_internal_shim_compile(
            source.as_ptr().cast(),
            source.len(),
            &mut out,
            &mut size,
            error.as_mut_ptr(),
        );
        assert_eq!(
            status,
            0,
            "{}",
            CStr::from_ptr(error.as_ptr()).to_string_lossy()
        );
        let result = std::slice::from_raw_parts(out.cast::<u8>(), size).to_vec();
        ulua_internal_shim_bytecode_free(out);
        result
    }
}
fn without_write_assertions(source: &str, initializers: bool) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let line = if !initializers && line.trim_start().starts_with("local ") {
            line.to_owned()
        } else if let Some(end) = line.rfind(") :: ") {
            if let Some(start) = line.find(" = (") {
                format!("{} = {}", &line[..start], &line[start + 4..end])
            } else if line.trim_start().starts_with("end) :: ") {
                format!("{}end", &line[..line.len() - line.trim_start().len()])
            } else {
                line.to_owned()
            }
        } else {
            line.replace(" = (function(", " = function(")
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}
#[test]
fn assignment_assertions_erase_and_initializers_report_native_differences() {
    assert_eq!(ulua::ulua_abi_version(), 1);
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| {
            for form in stress::FORMS {
                let source = stress::source(form).unwrap();
                let compiled =
                    ulua_compiler::compile(source.as_bytes(), Limits::default()).unwrap();
                let stripped = without_write_assertions(&compiled.luau, false);
                assert_ne!(compiled.luau, stripped);
                let typed = bytecode(&compiled.luau);
                let plain = bytecode(&stripped);
                assert!(typed == plain, "{form}: assignment assertions change bytecode ({} / {} bytes)", typed.len(), plain.len());
                let all_plain = bytecode(&without_write_assertions(&compiled.luau, true));
                let first = typed.iter().zip(&all_plain).position(|(a, b)| a != b);
                let suffix = typed.iter().rev().zip(all_plain.iter().rev()).take_while(|(a, b)| a == b).count();
                // Unspilled fixtures keep the full stress body in the common suffix.
                // Spill fixtures also change table access optimization and constant layout.
                if !compiled.luau.contains("local __ulua_spill") {
                    assert!(suffix >= typed.len().min(all_plain.len()).saturating_sub(1024));
                }
                eprintln!("S4 bytecode {form}: assignment assertions identical; all assertions typed={} plain={} first_difference={first:?} common_suffix={suffix}", typed.len(), all_plain.len());
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
