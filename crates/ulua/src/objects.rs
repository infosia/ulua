//! Context-owned objects and ABI error boundaries.
use crate::{Diagnostic, Host, RunLimits, Status, ULUA_BUSY, ULUA_INTERNAL, ULUA_POISONED, native};
use std::{
    cell::{Cell, RefCell},
    ffi::{c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
};

/// Opaque context. The host serializes this context and its children.
#[non_exhaustive]
pub struct Context {
    pub(crate) poisoned: Cell<bool>,
    pub(crate) busy: Cell<bool>,
    pub(crate) modules: RefCell<Vec<*mut Module>>,
    pub(crate) instances: RefCell<Vec<*mut Instance>>,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self {
            poisoned: Cell::new(false),
            busy: Cell::new(false),
            modules: RefCell::new(vec![]),
            instances: RefCell::new(vec![]),
        }
    }
}
/// Opaque compiled module. Export names live until this object is freed.
#[non_exhaustive]
pub struct Module {
    pub(crate) context: *mut Context,
    pub(crate) bytecode: Vec<u8>,
    pub(crate) exports: Vec<ulua_compiler::Export>,
}
/// Opaque sandboxed VM instance. It owns an export signature snapshot.
#[non_exhaustive]
pub struct Instance {
    pub(crate) context: *mut Context,
    pub(crate) vm: *mut c_void,
    pub(crate) exports: Vec<ulua_compiler::Export>,
    pub(crate) callback: Box<Callback>,
}
impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: This instance exclusively owns its VM. The shim accepts null.
        unsafe {
            native::ulua_internal_shim_free(self.vm);
        }
    }
}
pub(crate) struct Callback {
    pub(crate) context: *mut Context,
    pub(crate) host: Host,
    pub(crate) panicked: Cell<bool>,
}
pub(crate) unsafe extern "C" fn output(
    userdata: *mut c_void,
    text: *const c_char,
    length: usize,
) -> i32 {
    // SAFETY: The shim receives a stable Callback allocation owned by this operation or its instance.
    let callback = unsafe { &*userdata.cast::<Callback>() };
    let result = catch_unwind(AssertUnwindSafe(|| {
        match callback.host.output {
            // SAFETY: The host guarantees the callback ABI and userdata lifetime. Text lives through this call.
            Some(function) => unsafe { function(callback.host.userdata, text, length) },
            None => 0,
        }
    }));
    match result {
        Ok(status) => status,
        Err(_) => {
            callback.panicked.set(true);
            // SAFETY: The context outlives the callback and all its instances.
            unsafe {
                (*callback.context).poisoned.set(true);
            }
            1
        }
    }
}
pub(crate) fn shim_host(callback: &mut Callback) -> Host {
    Host {
        userdata: std::ptr::from_mut(callback).cast(),
        output: Some(output),
        ..Host::default()
    }
}
pub(crate) unsafe fn guard(
    context: *mut Context,
    diagnostic: *mut Diagnostic,
    function: impl FnOnce() -> Status,
) -> Status {
    // SAFETY: Public entry points validate null and require live context pointers from this library.
    let context = unsafe { &*context };
    if context.busy.get() {
        return ULUA_BUSY;
    }
    if context.poisoned.get() {
        return ULUA_POISONED;
    }
    context.busy.set(true);
    let result = catch_unwind(AssertUnwindSafe(function));
    context.busy.set(false);
    let status = match result {
        Ok(status) => status,
        Err(_) => {
            // SAFETY: The caller validated the optional descriptor before this guard.
            unsafe {
                write_run_diagnostic(
                    diagnostic,
                    "internal",
                    "Rust panic caught at the ABI boundary",
                );
            }
            ULUA_INTERNAL
        }
    };
    if status == ULUA_INTERNAL {
        context.poisoned.set(true);
    }
    status
}
pub(crate) unsafe fn write_run_diagnostic(out: *mut Diagnostic, code: &str, message: &str) {
    if !out.is_null() {
        let mut value = Diagnostic::default();
        value.set(code, message);
        // SAFETY: The ABI caller provides writable, validated diagnostic storage.
        unsafe {
            out.write(value);
        }
    }
}
pub(crate) unsafe fn finish(
    code: i32,
    error: &[c_char; 256],
    diagnostic: *mut Diagnostic,
) -> Status {
    let (status, name) = match code {
        0 => return crate::ULUA_OK,
        1 => (crate::ULUA_FAILED, "limit:memory"),
        2 => (crate::ULUA_FAILED, "limit:steps"),
        3 => (crate::ULUA_FAILED, "limit:stack"),
        4 => (crate::ULUA_HOST_ERROR, "host:output"),
        _ => (crate::ULUA_INTERNAL, "internal"),
    };
    let bytes: Vec<u8> = error
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    // SAFETY: The caller validates the optional diagnostic descriptor.
    unsafe {
        write_run_diagnostic(diagnostic, name, &String::from_utf8_lossy(&bytes));
    }
    status
}
pub(crate) fn default_run() -> RunLimits {
    RunLimits::default()
}
