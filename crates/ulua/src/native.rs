//! Private, protected C++ shim bindings.
use crate::{Host, RunLimits, Value};
use std::ffi::{c_char, c_void};
// SAFETY: These declarations match the private noexcept shim and the version 1 C layouts.
unsafe extern "C" {
    pub(crate) fn ulua_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        out_length: *mut usize,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_shim_bytecode_free(bytecode: *mut c_char);
    pub(crate) fn ulua_shim_new(
        bytecode: *const c_char,
        length: usize,
        host: *const Host,
        limits: *const RunLimits,
        out: *mut *mut c_void,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_shim_call(
        vm: *mut c_void,
        name: *const c_char,
        arguments: *const Value,
        count: usize,
        result_type: u32,
        result: *mut Value,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_shim_free(vm: *mut c_void);
}
