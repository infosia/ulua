//! Two 1,000-call regions through the public ABI or private native probes.
use crate::measurement::Workload;
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
};
use ulua::*;

pub(crate) const MAX_STEPS: u64 = 10_000_000_000;
pub(crate) const MAX_MEMORY_BYTES: u64 = 1_073_741_824;
pub(super) const EXPECTED: u64 = 500_500;
const HOST_SOURCE: &[u8] = b"export function f(a: number, b: number): number { return a + b; }";
const SCRIPT_SOURCE: &[u8] = b"export function f(): number { let s = 0; let i = 0; while (i < 1000) { s = s + hostAdd(i, 1); i = i + 1; } return s; }";

// SAFETY: These declarations match boundary_host.cpp. Handles have one owner.
unsafe extern "C" {
    fn ulua_boundary_new(
        direction: i32,
        level: i32,
        callback: HostFn,
        error: *mut c_char,
    ) -> *mut c_void;
    fn ulua_boundary_region(state: *mut c_void) -> i32;
    fn ulua_boundary_result(state: *mut c_void) -> f64;
    fn ulua_boundary_free(state: *mut c_void);
    fn ulua_boundary_error(state: *mut c_void) -> *const c_char;
}
fn number(n: f64) -> Value {
    let mut value = Value::default();
    value.r#type = ULUA_TYPE_NUMBER;
    value.number = n;
    value
}
#[unsafe(no_mangle)]
unsafe extern "C" fn ulua_boundary_add(
    _: *mut c_void,
    arguments: *const Value,
    _: usize,
    result: *mut Value,
) -> i32 {
    // SAFETY: Both native probes and the product pass two numbers and writable result storage.
    unsafe {
        result.write(number((*arguments).number + (*arguments.add(1)).number));
    }
    0
}
fn error(status: i32, d: &Diagnostic) -> String {
    // SAFETY: The ABI terminates these fixed diagnostic buffers.
    unsafe {
        format!(
            "ABI {status}: {}: {}",
            CStr::from_ptr(d.code.as_ptr()).to_string_lossy(),
            CStr::from_ptr(d.message.as_ptr()).to_string_lossy()
        )
    }
}
pub(super) struct Product {
    context: *mut Context,
    instance: *mut Instance,
    index: u32,
    script: bool,
    result: Value,
    diagnostic: Diagnostic,
    sum: f64,
}
impl Product {
    pub(super) fn new(script: bool) -> Result<Self, String> {
        let mut s = Self {
            context: ptr::null_mut(),
            instance: ptr::null_mut(),
            index: 0,
            script,
            result: Value::default(),
            diagnostic: Diagnostic::default(),
            sum: 0.0,
        };
        // SAFETY: This owner keeps every ABI object and descriptor live and serializes all calls.
        unsafe {
            let status = ulua_context_new(&mut s.context);
            if status != ULUA_OK {
                return Err(error(status, &s.diagnostic));
            }
            let mut capability = ptr::null_mut();
            if script {
                let parameters = [ULUA_TYPE_NUMBER; 2];
                let mut desc = CapabilityDesc::default();
                desc.name = c"hostAdd".as_ptr();
                desc.name_length = 7;
                desc.parameter_types = parameters.as_ptr();
                desc.parameter_count = 2;
                desc.result_type = ULUA_TYPE_NUMBER;
                desc.function = Some(ulua_boundary_add);
                let status = ulua_capability_new(s.context, &desc, &mut capability);
                if status != ULUA_OK {
                    return Err(error(status, &s.diagnostic));
                }
            }
            let source = if script { SCRIPT_SOURCE } else { HOST_SOURCE };
            let mut module = ptr::null_mut();
            let status = ulua_compile(
                s.context,
                source.as_ptr().cast(),
                source.len(),
                if script { &capability } else { ptr::null() },
                usize::from(script),
                &CompileLimits::default(),
                &mut module,
                &mut s.diagnostic,
            );
            if status != ULUA_OK {
                return Err(error(status, &s.diagnostic));
            }
            let status = ulua_module_find_export(module, c"f".as_ptr(), 1, &mut s.index);
            if status != ULUA_OK {
                return Err(error(status, &s.diagnostic));
            }
            let mut limits = RunLimits::default();
            limits.max_steps = MAX_STEPS;
            limits.max_memory_bytes = MAX_MEMORY_BYTES;
            let status = ulua_instance_new(
                s.context,
                module,
                &Host::default(),
                &limits,
                &mut s.instance,
                &mut s.diagnostic,
            );
            if status != ULUA_OK {
                return Err(error(status, &s.diagnostic));
            }
        }
        Ok(s)
    }
}
impl Workload for Product {
    fn prepare(&mut self) {
        self.diagnostic = Diagnostic::default();
        self.sum = 0.0;
    }
    fn call(&mut self) -> i32 {
        for i in 0..if self.script { 1 } else { 1000 } {
            let args = [number(f64::from(i)), number(1.0)];
            // SAFETY: The prepared export takes zero or two numbers. Outputs remain live.
            let status = unsafe {
                ulua_call(
                    self.instance,
                    self.index,
                    if self.script {
                        ptr::null()
                    } else {
                        args.as_ptr()
                    },
                    if self.script { 0 } else { 2 },
                    &mut self.result,
                    &mut self.diagnostic,
                )
            };
            if status != ULUA_OK {
                return status;
            }
            self.sum += self.result.number;
        }
        ULUA_OK
    }
    fn result(&mut self, status: i32) -> Result<u64, String> {
        if status != ULUA_OK {
            return Err(error(status, &self.diagnostic));
        }
        if self.result.r#type != ULUA_TYPE_NUMBER {
            return Err("Expected numeric result".into());
        }
        checked(self.sum)
    }
}
impl Drop for Product {
    fn drop(&mut self) {
        // SAFETY: The context owns the module, capability, and instance. This owner frees it once.
        unsafe {
            ulua_context_free(self.context);
        }
    }
}
pub(super) struct Native(*mut c_void);
impl Native {
    pub(super) fn new(script: bool, level: i32) -> Result<Self, String> {
        let mut error = [0; 256];
        // SAFETY: The callback has the required ABI. The error buffer holds 256 bytes.
        let state = unsafe {
            ulua_boundary_new(
                i32::from(script),
                level,
                ulua_boundary_add,
                error.as_mut_ptr(),
            )
        };
        if state.is_null() {
            // SAFETY: The native host terminates the zero-initialized buffer.
            return Err(unsafe {
                CStr::from_ptr(error.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            });
        }
        Ok(Self(state))
    }
}
impl Workload for Native {
    fn prepare(&mut self) {}
    fn call(&mut self) -> i32 {
        // SAFETY: This owner holds a live native probe.
        unsafe { ulua_boundary_region(self.0) }
    }
    fn result(&mut self, status: i32) -> Result<u64, String> {
        if status != 0 {
            // SAFETY: The probe owns a NUL-terminated error buffer for its lifetime.
            let error = unsafe { CStr::from_ptr(ulua_boundary_error(self.0)).to_string_lossy() };
            return Err(format!("Native probe status {status}: {error}"));
        }
        // SAFETY: The region completed and retained its checksum.
        checked(unsafe { ulua_boundary_result(self.0) })
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        // SAFETY: This owner frees the probe once after all regions.
        unsafe {
            ulua_boundary_free(self.0);
        }
    }
}
fn checked(n: f64) -> Result<u64, String> {
    if n == EXPECTED as f64 {
        Ok(EXPECTED)
    } else {
        Err(format!("Expected {EXPECTED}, got {n}"))
    }
}
