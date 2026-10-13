//! Host-driven completion records, settle runs, and call task observations.
use crate::{
    abi::{call_entry, diagnostic_valid, value_valid},
    objects::{finish, guard},
    *,
};
use std::ptr;

/// Start an export and retain one task result after its initial drain.
/// # Safety
/// The instance and argument storage must be live. Task and diagnostic storage must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_call_start(
    instance: *mut Instance,
    export_index: u32,
    arguments: *const Value,
    argument_count: usize,
    task: *mut u64,
    diagnostic: *mut Diagnostic,
) -> Status {
    if task.is_null() {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller provides the same live storage as ulua_call, with a task output.
    unsafe {
        call_entry(
            instance,
            export_index,
            arguments,
            argument_count,
            ptr::null_mut(),
            task,
            diagnostic,
        )
    }
}
/// Observe a task. A terminal query consumes its id; a pending query preserves it.
/// # Safety
/// The instance must be live. Result and diagnostic storage must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_task_result(
    instance: *mut Instance,
    task: u64,
    result: *mut Value,
    diagnostic: *mut Diagnostic,
) -> Status {
    // SAFETY: The caller supplies readable optional diagnostic storage.
    if instance.is_null() || result.is_null() || !unsafe { diagnostic_valid(diagnostic) } {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller serializes the instance and provides validated output storage.
    unsafe {
        guard((*instance).context, diagnostic, || {
            let mut error = [0; 256];
            let mut value = Value::default();
            let code = native::ulua_internal_shim_task_result(
                (*instance).vm,
                task,
                &mut value,
                error.as_mut_ptr(),
            );
            let status = finish(code, &error, diagnostic);
            if status == ULUA_OK {
                result.write(value);
            }
            status
        })
    }
}
/// Copy a completion value without executing script code.
/// # Safety
/// The instance and value must be live. String bytes must cover the stated length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_request_complete(
    instance: *mut Instance,
    request: u64,
    value: *const Value,
) -> Status {
    if instance.is_null() || value.is_null() {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller supplies live instance and readable value storage.
    unsafe {
        guard((*instance).context, ptr::null_mut(), || {
            if !value_valid(&*value) {
                return ULUA_INVALID_ARGUMENT;
            }
            let mut error = [0; 256];
            let code = native::ulua_internal_shim_complete(
                (*instance).vm,
                request,
                value,
                error.as_mut_ptr(),
            );
            finish(code, &error, ptr::null_mut())
        })
    }
}
/// Record a host request failure without executing script code.
/// # Safety
/// The instance must be live and its context must be serialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_request_fail(instance: *mut Instance, request: u64) -> Status {
    if instance.is_null() {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller serializes this live instance and its context.
    unsafe {
        guard((*instance).context, ptr::null_mut(), || {
            let mut error = [0; 256];
            let code = native::ulua_internal_shim_complete(
                (*instance).vm,
                request,
                ptr::null(),
                error.as_mut_ptr(),
            );
            finish(code, &error, ptr::null_mut())
        })
    }
}
/// Settle recorded requests in record order. Zero maximum means no limit.
/// # Safety
/// The instance must be live. Count and diagnostic storage must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_pump(
    instance: *mut Instance,
    max_requests: u32,
    settled: *mut u32,
    diagnostic: *mut Diagnostic,
) -> Status {
    // SAFETY: The caller supplies readable optional diagnostic storage.
    if instance.is_null() || settled.is_null() || !unsafe { diagnostic_valid(diagnostic) } {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller serializes the live instance and supplies validated output storage.
    unsafe {
        guard((*instance).context, diagnostic, || {
            let mut error = [0; 256];
            let code = native::ulua_internal_shim_pump(
                (*instance).vm,
                max_requests,
                settled,
                error.as_mut_ptr(),
            );
            if (*instance).callback.panicked.get() {
                objects::write_run_diagnostic(diagnostic, "internal", "Output callback panicked");
                return ULUA_INTERNAL;
            }
            finish(code, &error, diagnostic)
        })
    }
}
/// Write open request and recorded completion counts.
/// # Safety
/// The instance must be live. Both count outputs must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_instance_requests(
    instance: *const Instance,
    open: *mut u32,
    recorded: *mut u32,
) -> Status {
    if instance.is_null() || open.is_null() || recorded.is_null() {
        return ULUA_INVALID_ARGUMENT;
    }
    // SAFETY: The caller serializes this live instance and supplies count storage.
    unsafe {
        guard((*instance).context, ptr::null_mut(), || {
            native::ulua_internal_shim_counts((*instance).vm, open, recorded);
            ULUA_OK
        })
    }
}
