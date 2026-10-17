//! Private, protected C++ shim bindings.
use crate::{Host, RunLimits, Value};
use std::ffi::{c_char, c_void};
// SAFETY: These declarations match the private noexcept shim and the version 1 C layouts.
unsafe extern "C" {
    pub(crate) fn ulua_internal_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        out_length: *mut usize,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_bytecode_free(bytecode: *mut c_char);
    pub(crate) fn ulua_internal_shim_new_math(
        bytecode: *const c_char,
        length: usize,
        host: *const Host,
        grants: *const crate::capabilities::NativeCapability,
        grant_count: usize,
        limits: *const RunLimits,
        out: *mut *mut c_void,
        error: *mut c_char,
        math: Option<extern "C" fn(u32, f64, f64) -> f64>,
        number: Option<
            unsafe extern "C" fn(u32, f64, f64, i32, *const u8, usize, *mut u8, *mut f64) -> i32,
        >,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_call(
        vm: *mut c_void,
        name: *const c_char,
        arguments: *const Value,
        count: usize,
        result_type: u32,
        result: *mut Value,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_call_start(
        vm: *mut c_void,
        name: *const c_char,
        arguments: *const Value,
        count: usize,
        result_type: u32,
        task: *mut u64,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_task_result(
        vm: *mut c_void,
        task: u64,
        result: *mut Value,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_task_discard(vm: *mut c_void, task: u64) -> i32;
    pub(crate) fn ulua_internal_shim_complete(
        vm: *mut c_void,
        request: u64,
        value: *const Value,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_pump(
        vm: *mut c_void,
        maximum: u32,
        settled: *mut u32,
        error: *mut c_char,
    ) -> i32;
    pub(crate) fn ulua_internal_shim_counts(vm: *mut c_void, open: *mut u32, recorded: *mut u32);
    pub(crate) fn ulua_internal_shim_seed_random(vm: *mut c_void, seed: u64);
    pub(crate) fn ulua_internal_shim_free(vm: *mut c_void);
}
