//! C ABI entry points. Invalid host arguments never run script code.
use crate::{native, objects::*, *};
use std::{
    ffi::{CString, c_char},
    ptr, slice,
};

#[cfg(test)]
thread_local! {
    pub(crate) static TEST_OUTPUT_FACTOR: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    pub(crate) static REFUSE_COMPILER_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) unsafe fn diagnostic_valid(d: *mut Diagnostic) -> bool {
    // SAFETY: Non-null descriptors must refer to aligned storage of their declared size.
    unsafe {
        d.is_null() || ((*d).struct_size == size_of::<Diagnostic>() as u32 && (*d).reserved == 0)
    }
}
pub(crate) unsafe fn bytes<'a>(p: *const c_char, n: usize) -> Option<&'a [u8]> {
    if n > isize::MAX as usize || (p.is_null() && n != 0) {
        return None;
    }
    if n == 0 {
        return Some(&[]);
    }
    // SAFETY: The host provides readable storage of n bytes for non-null source pointers.
    Some(unsafe { slice::from_raw_parts(p.cast(), n) })
}

/// Return the ABI version.
#[unsafe(no_mangle)]
pub extern "C" fn ulua_abi_version() -> u32 {
    1
}

/// Write default compile limits.
/// # Safety
/// `out` must be null or a writable descriptor with its struct_size set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_default_compile_limits(out: *mut CompileLimits) -> Status {
    // SAFETY: The host supplies aligned writable descriptor storage.
    unsafe {
        if out.is_null() || (*out).struct_size != size_of::<CompileLimits>() as u32 {
            return ULUA_INVALID_ARGUMENT;
        }
        out.write(CompileLimits::default());
    }
    ULUA_OK
}
/// Write default run limits.
/// # Safety
/// `out` must be null or a writable descriptor with its struct_size set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_default_run_limits(out: *mut RunLimits) -> Status {
    // SAFETY: The host supplies aligned writable descriptor storage.
    unsafe {
        if out.is_null() || (*out).struct_size != size_of::<RunLimits>() as u32 {
            return ULUA_INVALID_ARGUMENT;
        }
        out.write(RunLimits::default());
    }
    ULUA_OK
}
/// Allocate one independent context.
/// # Safety
/// `out` must be null or writable pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_context_new(out: *mut *mut Context) -> Status {
    if out.is_null() {
        return ULUA_INVALID_ARGUMENT;
    }
    match std::panic::catch_unwind(|| Box::into_raw(Box::new(Context::new()))) {
        Ok(context) => {
            // SAFETY: The host provides writable pointer storage.
            unsafe {
                out.write(context);
            }
            ULUA_OK
        }
        Err(_) => ULUA_INTERNAL,
    }
}
/// Free the context and all its modules and instances. Null succeeds.
/// # Safety
/// A non-null context must be live. All child pointers become invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_context_free(context: *mut Context) -> Status {
    if context.is_null() {
        return ULUA_OK;
    }
    // SAFETY: The host supplies a live context. Busy prevents callback invalidation.
    unsafe {
        if (*context).busy.get() {
            return ULUA_BUSY;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = Box::from_raw(context);
            for instance in c.instances.get_mut().drain(..) {
                drop(Box::from_raw(instance));
            }
            for module in c.modules.get_mut().drain(..) {
                drop(Box::from_raw(module));
            }
            for capability in c.capabilities.get_mut().drain(..) {
                let capability = Box::from_raw(capability);
                capability.state.revoked.set(true);
            }
        }));
        if result.is_ok() {
            ULUA_OK
        } else {
            ULUA_INTERNAL
        }
    }
}
/// Compile TypeScript on a bounded library-owned thread.
/// # Safety
/// Pointers must be live, aligned, and readable or writable for their stated sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_compile(
    context: *mut Context,
    source: *const c_char,
    length: usize,
    grants: *const *mut Capability,
    grant_count: usize,
    limits: *const CompileLimits,
    out: *mut *mut Module,
    diagnostic: *mut Diagnostic,
) -> Status {
    // SAFETY: The host supplies valid pointer storage and serializes the context.
    unsafe {
        if context.is_null() || out.is_null() || !diagnostic_valid(diagnostic) {
            return ULUA_INVALID_ARGUMENT;
        }
        let Some(source) = bytes(source, length) else {
            return ULUA_INVALID_ARGUMENT;
        };
        let limits = if limits.is_null() {
            CompileLimits::default()
        } else {
            if (*limits).struct_size != size_of::<CompileLimits>() as u32
                || (*limits).reserved != 0
                || (*limits).reserved2 != 0
            {
                return ULUA_INVALID_ARGUMENT;
            }
            *limits
        };
        guard(context, diagnostic, || {
            out.write(ptr::null_mut());
            let Some(grants) = crate::capabilities::grants(context, grants, grant_count) else {
                return ULUA_INVALID_ARGUMENT;
            };
            let signatures: Vec<_> = grants.iter().map(|g| g.signature.clone()).collect();
            #[cfg(feature = "timing")]
            let observer = crate::compile_measurement::observer();
            #[cfg(test)]
            let output_factor = TEST_OUTPUT_FACTOR.with(|factor| factor.take());
            let joined = std::thread::scope(|scope| {
                #[cfg(test)]
                if REFUSE_COMPILER_THREAD.with(|flag| flag.replace(false)) {
                    return Err((ULUA_FAILED, "Compiler thread refused by test".to_owned()));
                }
                let worker = std::thread::Builder::new()
                    .name("ulua-compiler".into())
                    .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
                    .spawn_scoped(scope, move || -> Result<_, ulua_compiler::Diagnostic> {
                        #[cfg(feature = "timing")]
                        crate::compile_measurement::notify(
                            observer,
                            crate::compile_measurement::CompileEvent::Start,
                        );
                        #[cfg(test)]
                        let compiled = match output_factor {
                            Some(factor) => ulua_compiler::test_compile_with_output_factor(
                                source,
                                compiler_limits(limits),
                                factor,
                            ),
                            None => ulua_compiler::compile_with_grants(
                                source,
                                compiler_limits(limits),
                                &signatures,
                            ),
                        };
                        #[cfg(not(test))]
                        let compiled = ulua_compiler::compile_with_grants(
                            source,
                            compiler_limits(limits),
                            &signatures,
                        );
                        let compiled = match compiled {
                            Ok(compiled) => compiled,
                            Err(diagnostic) => {
                                #[cfg(feature = "timing")]
                                crate::compile_measurement::notify(
                                    observer,
                                    crate::compile_measurement::CompileEvent::Rejected,
                                );
                                return Err(diagnostic);
                            }
                        };
                        #[cfg(feature = "timing")]
                        crate::compile_measurement::notify(
                            observer,
                            crate::compile_measurement::CompileEvent::Rust(&compiled.luau),
                        );
                        let mut bytecode = ptr::null_mut();
                        let mut size = 0;
                        let mut error = [0; 256];
                        let status = native::ulua_internal_shim_compile(
                            compiled.luau.as_ptr().cast(),
                            compiled.luau.len(),
                            &mut bytecode,
                            &mut size,
                            error.as_mut_ptr(),
                        );
                        #[cfg(feature = "timing")]
                        crate::compile_measurement::notify(
                            observer,
                            crate::compile_measurement::CompileEvent::Native,
                        );
                        if status != 0 {
                            return Ok(Err(error));
                        }
                        let owner = Bytecode(bytecode);
                        let code = slice::from_raw_parts(owner.0.cast::<u8>(), size).to_vec();
                        Ok(Ok((
                            compiled.exports,
                            code,
                            compiled.hosts,
                            compiled.native_math,
                        )))
                    });
                match worker {
                    Ok(worker) => worker
                        .join()
                        .map_err(|_| (ULUA_INTERNAL, "Compiler thread panicked".to_owned())),
                    Err(e) => Err((ULUA_FAILED, e.to_string())),
                }
            });
            match joined {
                Ok(Ok(Ok((exports, bytecode, hosts, native_math)))) => {
                    let by_name: std::collections::HashMap<_, _> = grants
                        .iter()
                        .map(|g| (g.signature.name.as_str(), g))
                        .collect();
                    let module = Box::new(Module {
                        native_math,
                        context,
                        grants: hosts
                            .iter()
                            .filter_map(|n| by_name.get(n.as_str()).map(|g| (*g).clone()))
                            .collect(),
                        bytecode,
                        exports,
                    });
                    let Ok(mut modules) = (*context).modules.try_borrow_mut() else {
                        write_run_diagnostic(
                            diagnostic,
                            "internal",
                            "Context registry is borrowed",
                        );
                        return ULUA_INTERNAL;
                    };
                    let module = Box::into_raw(module);
                    modules.push(module);
                    out.write(module);
                    ULUA_OK
                }
                Ok(Err(d)) => {
                    out.write(ptr::null_mut());
                    let status = if d.code == "internal" {
                        ULUA_INTERNAL
                    } else {
                        ULUA_REJECTED
                    };
                    write_compile_diagnostic(diagnostic, &Some(d));
                    status
                }
                Ok(Ok(Err(error))) => {
                    out.write(ptr::null_mut());
                    finish(5, &error, diagnostic)
                }
                Err((status, message)) => {
                    out.write(ptr::null_mut());
                    let code = if status == ULUA_FAILED {
                        "host:resources"
                    } else {
                        "internal"
                    };
                    write_run_diagnostic(diagnostic, code, &message);
                    status
                }
            }
        })
    }
}
fn compiler_limits(limits: CompileLimits) -> ulua_compiler::Limits {
    let mut result = ulua_compiler::Limits::default();
    result.max_source_bytes = limits.max_source_bytes;
    result.max_nesting_depth = limits.max_nesting_depth;
    result
}
unsafe fn write_compile_diagnostic(out: *mut Diagnostic, d: &Option<ulua_compiler::Diagnostic>) {
    if out.is_null() {
        return;
    }
    if let Some(d) = d {
        let mut value = Diagnostic::default();
        value.set(d.code, &d.message);
        value.offset = d.offset;
        value.line = d.line;
        value.column = d.column;
        // SAFETY: Public entry points validate optional diagnostic storage.
        unsafe {
            out.write(value);
        }
    }
}
/// Free a compiled module. Its live instances remain valid. Null succeeds.
/// # Safety
/// A non-null module must be live and belong to a live context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_module_free(module: *mut Module) -> Status {
    if module.is_null() {
        return ULUA_OK;
    }
    // SAFETY: The host supplies a live module; registry removal precedes its destruction.
    unsafe {
        let context = (*module).context;
        if (*context).busy.get() {
            return ULUA_BUSY;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let Ok(mut modules) = (*context).modules.try_borrow_mut() else {
                return ULUA_INTERNAL;
            };
            modules.retain(|p| *p != module);
            drop(Box::from_raw(module));
            ULUA_OK
        }));
        let status = result.unwrap_or(ULUA_INTERNAL);
        if status == ULUA_INTERNAL {
            (*context).poisoned.set(true);
        }
        status
    }
}
/// Write the number of exports.
/// # Safety
/// The module must be live. `out` must be writable if non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_module_export_count(module: *const Module, out: *mut u32) -> Status {
    // SAFETY: The host supplies live module and writable output storage.
    unsafe {
        if module.is_null() || out.is_null() {
            return ULUA_INVALID_ARGUMENT;
        }
        guard((*module).context, ptr::null_mut(), || {
            out.write((*module).exports.len() as u32);
            ULUA_OK
        })
    }
}
/// Write the borrowed export name and byte length.
/// # Safety
/// The module must be live. Outputs must be writable if non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_module_export_name(
    module: *const Module,
    index: u32,
    name: *mut *const c_char,
    length: *mut usize,
) -> Status {
    // SAFETY: The host supplies live module and writable output storage.
    unsafe {
        if module.is_null() || name.is_null() || length.is_null() {
            return ULUA_INVALID_ARGUMENT;
        }
        guard((*module).context, ptr::null_mut(), || {
            let exports = &(*module).exports;
            let Some(export) = exports.get(index as usize) else {
                return ULUA_INVALID_ARGUMENT;
            };
            name.write(export.name.as_ptr().cast());
            length.write(export.name.len());
            ULUA_OK
        })
    }
}
/// Write the export signature. Insufficient capacity writes count and result only.
/// # Safety
/// The module must be live. All output pointers must cover their stated capacities.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_module_export_signature(
    module: *const Module,
    index: u32,
    parameter_types: *mut u32,
    capacity: usize,
    parameter_count: *mut usize,
    result_type: *mut u32,
) -> Status {
    // SAFETY: The host supplies live module and writable output storage.
    unsafe {
        if module.is_null()
            || parameter_count.is_null()
            || result_type.is_null()
            || (parameter_types.is_null() && capacity != 0)
        {
            return ULUA_INVALID_ARGUMENT;
        }
        guard((*module).context, ptr::null_mut(), || {
            let exports = &(*module).exports;
            let Some(export) = exports.get(index as usize) else {
                return ULUA_INVALID_ARGUMENT;
            };
            parameter_count.write(export.parameters.len());
            result_type.write(export.result);
            if capacity < export.parameters.len() {
                return ULUA_INVALID_ARGUMENT;
            }
            if !export.parameters.is_empty() {
                ptr::copy_nonoverlapping(
                    export.parameters.as_ptr(),
                    parameter_types,
                    export.parameters.len(),
                );
            }
            ULUA_OK
        })
    }
}
/// Find an export by UTF-8 name. An unknown name writes u32::MAX.
/// # Safety
/// The module must be live. Name bytes must be readable and index must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_module_find_export(
    module: *const Module,
    name: *const c_char,
    length: usize,
    index: *mut u32,
) -> Status {
    // SAFETY: The host supplies live module, readable name, and writable output storage.
    unsafe {
        if module.is_null() || index.is_null() {
            return ULUA_INVALID_ARGUMENT;
        }
        let Some(name) = bytes(name, length).and_then(|b| std::str::from_utf8(b).ok()) else {
            return ULUA_INVALID_ARGUMENT;
        };
        guard((*module).context, ptr::null_mut(), || {
            match (*module).exports.iter().position(|e| e.name == name) {
                Some(i) => {
                    index.write(i as u32);
                    ULUA_OK
                }
                None => {
                    index.write(u32::MAX);
                    ULUA_INVALID_ARGUMENT
                }
            }
        })
    }
}
/// Create a sandboxed instance and run the module chunk once.
/// # Safety
/// Objects must be live. Descriptors and outputs must cover their declared sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_instance_new(
    context: *mut Context,
    module: *const Module,
    host: *const Host,
    limits: *const RunLimits,
    out: *mut *mut Instance,
    diagnostic: *mut Diagnostic,
) -> Status {
    // SAFETY: The host supplies live objects and valid descriptor and output storage.
    unsafe {
        if context.is_null()
            || module.is_null()
            || host.is_null()
            || out.is_null()
            || !diagnostic_valid(diagnostic)
        {
            return ULUA_INVALID_ARGUMENT;
        }
        if (*module).context != context
            || (*host).struct_size != size_of::<Host>() as u32
            || (*host).reserved != 0
            || (*host).random_seed_set > 1
            || (*host).reserved2 != 0
        {
            return ULUA_INVALID_ARGUMENT;
        }
        let limits = if limits.is_null() {
            default_run()
        } else {
            if (*limits).struct_size != size_of::<RunLimits>() as u32
                || (*limits).reserved != 0
                || (*limits).reserved2 != 0
            {
                return ULUA_INVALID_ARGUMENT;
            }
            *limits
        };
        guard(context, diagnostic, || {
            out.write(ptr::null_mut());
            let mut callback = Box::new(Callback {
                context,
                host: *host,
                panicked: std::cell::Cell::new(false),
            });
            let mut grants: Vec<_> = (*module)
                .grants
                .iter()
                .map(|state| crate::capabilities::Binding {
                    context,
                    state: state.clone(),
                })
                .collect();
            let native_grants: Vec<_> = grants.iter_mut().map(|b| b.native()).collect();
            let host = shim_host(&mut callback);
            let mut vm = ptr::null_mut();
            let mut error = [0; 256];
            let code = native::ulua_internal_shim_new_math(
                (*module).bytecode.as_ptr().cast(),
                (*module).bytecode.len(),
                &host,
                native_grants.as_ptr(),
                native_grants.len(),
                &limits,
                &mut vm,
                error.as_mut_ptr(),
                if (*module).native_math {
                    Some(crate::math::dispatch)
                } else {
                    None
                },
            );
            if callback.panicked.get() {
                native::ulua_internal_shim_free(vm);
                write_run_diagnostic(diagnostic, "internal", "Output callback panicked");
                return ULUA_INTERNAL;
            }
            if code != 0 {
                out.write(ptr::null_mut());
                return finish(code, &error, diagnostic);
            }
            let instance = Box::new(Instance {
                context,
                vm,
                exports: (*module).exports.clone(),
                callback,
                _grants: grants,
            });
            let Ok(mut instances) = (*context).instances.try_borrow_mut() else {
                write_run_diagnostic(diagnostic, "internal", "Context registry is borrowed");
                return ULUA_INTERNAL;
            };
            let instance = Box::into_raw(instance);
            instances.push(instance);
            out.write(instance);
            ULUA_OK
        })
    }
}
/// Free a VM instance. Null succeeds.
/// # Safety
/// A non-null instance must be live and belong to a live context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_instance_free(instance: *mut Instance) -> Status {
    if instance.is_null() {
        return ULUA_OK;
    }
    // SAFETY: The host supplies a live instance; registry removal precedes its destruction.
    unsafe {
        let context = (*instance).context;
        if (*context).busy.get() {
            return ULUA_BUSY;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let Ok(mut instances) = (*context).instances.try_borrow_mut() else {
                return ULUA_INTERNAL;
            };
            instances.retain(|p| *p != instance);
            drop(Box::from_raw(instance));
            ULUA_OK
        }));
        let status = result.unwrap_or(ULUA_INTERNAL);
        if status == ULUA_INTERNAL {
            (*context).poisoned.set(true);
        }
        status
    }
}
/// Call one export. Validate arguments before any script runs.
/// # Safety
/// The instance must be live. Arguments, result, and diagnostic must cover their stated sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ulua_call(
    instance: *mut Instance,
    export_index: u32,
    arguments: *const Value,
    argument_count: usize,
    result: *mut Value,
    diagnostic: *mut Diagnostic,
) -> Status {
    // SAFETY: The caller provides live instance, arguments, and output storage.
    unsafe {
        call_entry(
            instance,
            export_index,
            arguments,
            argument_count,
            result,
            ptr::null_mut(),
            diagnostic,
        )
    }
}
pub(crate) unsafe fn call_entry(
    instance: *mut Instance,
    export_index: u32,
    arguments: *const Value,
    argument_count: usize,
    result: *mut Value,
    task: *mut u64,
    diagnostic: *mut Diagnostic,
) -> Status {
    // SAFETY: The host supplies a live instance and valid argument and output storage.
    unsafe {
        if instance.is_null()
            || (result.is_null() && task.is_null())
            || !diagnostic_valid(diagnostic)
            || (arguments.is_null() && argument_count != 0)
            || argument_count > isize::MAX as usize / size_of::<Value>()
        {
            return ULUA_INVALID_ARGUMENT;
        }
        guard((*instance).context, diagnostic, || {
            let exports = &(*instance).exports;
            let Some(export) = exports.get(export_index as usize) else {
                return ULUA_INVALID_ARGUMENT;
            };
            if argument_count != export.parameters.len() {
                return ULUA_INVALID_ARGUMENT;
            }
            let values = if argument_count == 0 {
                &[]
            } else {
                slice::from_raw_parts(arguments, argument_count)
            };
            if !values
                .iter()
                .zip(&export.parameters)
                .all(|(v, t)| v.r#type == *t && value_valid(v))
            {
                return ULUA_INVALID_ARGUMENT;
            }
            let name = match CString::new(export.name.as_str()) {
                Ok(name) => name,
                Err(_) => {
                    write_run_diagnostic(diagnostic, "internal", "Export name contains NUL");
                    return ULUA_INTERNAL;
                }
            };
            let mut error = [0; 256];
            let mut value = Value::default();
            let code = if task.is_null() {
                native::ulua_internal_shim_call(
                    (*instance).vm,
                    name.as_ptr(),
                    arguments,
                    argument_count,
                    export.result,
                    &mut value,
                    error.as_mut_ptr(),
                )
            } else {
                native::ulua_internal_shim_call_start(
                    (*instance).vm,
                    name.as_ptr(),
                    arguments,
                    argument_count,
                    export.result,
                    task,
                    error.as_mut_ptr(),
                )
            };
            if (*instance).callback.panicked.get() {
                write_run_diagnostic(diagnostic, "internal", "Output callback panicked");
                return ULUA_INTERNAL;
            }
            let status = finish(code, &error, diagnostic);
            if status == ULUA_OK && task.is_null() {
                result.write(value);
            }
            status
        })
    }
}
pub(crate) fn value_valid(v: &Value) -> bool {
    match v.r#type {
        ULUA_TYPE_VOID => {
            v.boolean == 0 && v.number.to_bits() == 0 && v.data.is_null() && v.length == 0
        }
        ULUA_TYPE_NUMBER => v.boolean == 0 && v.data.is_null() && v.length == 0,
        ULUA_TYPE_BOOLEAN => {
            v.boolean <= 1 && v.number.to_bits() == 0 && v.data.is_null() && v.length == 0
        }
        ULUA_TYPE_STRING => {
            v.boolean == 0 && v.number.to_bits() == 0 && {
                // SAFETY: The host guarantees readable value storage.
                unsafe { bytes(v.data, v.length).is_some_and(|b| std::str::from_utf8(b).is_ok()) }
            }
        }
        _ => false,
    }
}

struct Bytecode(*mut c_char);
impl Drop for Bytecode {
    fn drop(&mut self) {
        // SAFETY: The shim allocates this bytecode and this owner frees it exactly once.
        unsafe {
            native::ulua_internal_shim_bytecode_free(self.0);
        }
    }
}
