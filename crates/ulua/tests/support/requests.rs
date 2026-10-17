//! ABI hostLater runs share the corpus fixture and its one-settle loop.
// External non-exhaustive descriptors require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "p3_declaration_text.rs"]
mod declaration;
#[path = "../../../ulua_corpus/src/fixtures.rs"]
mod fixtures;
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
};
use ulua::*;
pub fn signatures(source: &str) -> Vec<ulua_compiler::HostSignature> {
    fixtures::Fixtures::signatures(source.as_bytes()).unwrap()
}
unsafe extern "C" fn collect(data: *mut c_void, text: *const c_char, len: usize) -> i32 {
    // SAFETY: The caller keeps the output vector and UTF-8 bytes live through this callback.
    unsafe {
        (*data.cast::<Vec<String>>()).push(
            std::str::from_utf8(std::slice::from_raw_parts(text.cast(), len))
                .unwrap()
                .to_owned(),
        );
    }
    0
}
struct Owner(*mut Context);
impl Drop for Owner {
    fn drop(&mut self) {
        // SAFETY: This owner serializes and releases its context and remaining children once.
        unsafe {
            assert_eq!(ulua_context_free(self.0), ULUA_OK);
        }
    }
}
pub fn run(source: &str, limits: RunLimits) -> (Status, String, String, String) {
    let mut f = fixtures::Fixtures::new();
    let mut c = ptr::null_mut();
    let mut m = ptr::null_mut();
    let mut i = ptr::null_mut();
    let mut d = Diagnostic::default();
    let mut index = 0;
    let mut task = 0;
    let mut value = Value::default();
    // SAFETY: This helper owns and serializes all object, callback, and output storage.
    unsafe {
        assert_eq!(ulua_context_new(&mut c), ULUA_OK);
        let _owner = Owner(c);
        let grants = f.grants(c, &signatures(source)).unwrap();
        let declarations = declaration::text(&grants).unwrap();
        assert_eq!(
            ulua_compile(
                c,
                source.as_ptr().cast(),
                source.len(),
                grants.as_ptr(),
                grants.len(),
                ptr::null(),
                &mut m,
                &mut d
            ),
            ULUA_OK,
            "{}",
            CStr::from_ptr(d.message.as_ptr()).to_string_lossy()
        );
        let mut host = Host::default();
        host.output = Some(collect);
        host.userdata = ptr::from_mut(&mut f.output).cast();
        let status = ulua_instance_new(c, m, &host, &limits, &mut i, &mut d);
        if status != ULUA_OK {
            return (
                status,
                CStr::from_ptr(d.code.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                f.output.iter().map(|s| format!("{s}\n")).collect(),
                declarations,
            );
        }
        assert_eq!(
            ulua_module_find_export(m, c"main".as_ptr(), 4, &mut index),
            ULUA_OK
        );
        let mut status = ulua_call_start(i, index, ptr::null(), 0, &mut task, &mut d);
        let mut pending = None;
        if status == ULUA_OK {
            status = ulua_task_result(i, task, &mut value, &mut d);
            pending = (status == ULUA_PENDING).then_some(task);
            if pending.is_some() {
                status = ULUA_OK;
            }
        }
        if status == ULUA_OK {
            status = f.settle(i, &mut pending, &mut d);
        }
        if status == ULUA_OK && pending.is_some() {
            status = ulua_task_result(i, task, &mut value, &mut d);
        }
        if status == ULUA_PENDING {
            return (
                ULUA_FAILED,
                "async:pending".to_owned(),
                f.output.iter().map(|s| format!("{s}\n")).collect(),
                declarations,
            );
        }
        (
            status,
            CStr::from_ptr(d.code.as_ptr())
                .to_string_lossy()
                .into_owned(),
            f.output.iter().map(|s| format!("{s}\n")).collect(),
            declarations,
        )
    }
}
