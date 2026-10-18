//! Partial export creation must release all native records and registry storage.
use std::{
    ffi::{c_char, c_void},
    ptr,
};
use ulua::*;

// The non-exhaustive descriptors have fixed C layouts.
#[allow(improper_ctypes)]
// SAFETY: These declarations match the private compiler and VM shim.
unsafe extern "C" {
    fn ulua_internal_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        size: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(out: *mut c_char);
    fn ulua_internal_shim_new(
        code: *const c_char,
        length: usize,
        host: *const Host,
        grants: *const c_void,
        grant_count: usize,
        export_names: *const *const c_char,
        export_count: usize,
        limits: *const RunLimits,
        out: *mut *mut c_void,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_free(vm: *mut c_void);
}

#[test]
fn malformed_export_table_fails_at_creation() {
    let mut outcomes = Vec::new();
    for source in [
        "return {first = function() end}",
        "return {first = function() end, main = 42}",
    ] {
        let mut bytecode = ptr::null_mut();
        let mut size = 0;
        let mut error = [0; 256];
        let mut vm = ptr::null_mut();
        // SAFETY: All buffers cover their lengths. Each native allocation is freed once.
        unsafe {
            assert_eq!(
                ulua_internal_shim_compile(
                    source.as_ptr().cast(),
                    source.len(),
                    &mut bytecode,
                    &mut size,
                    error.as_mut_ptr()
                ),
                0
            );
            fault_injection::arm(0);
            let status = ulua_internal_shim_new(
                bytecode,
                size,
                &Host::default(),
                ptr::null(),
                0,
                [c"first".as_ptr(), c"main".as_ptr()].as_ptr(),
                2,
                &RunLimits::default(),
                &mut vm,
                error.as_mut_ptr(),
            );
            ulua_internal_shim_bytecode_free(bytecode);
            let absent = vm.is_null();
            ulua_internal_shim_free(vm);
            super::assert_clean(fault_injection::last()).unwrap();
            outcomes.push((status, absent));
        }
    }
    assert_eq!(outcomes, vec![(5, true), (5, true)]);
}
