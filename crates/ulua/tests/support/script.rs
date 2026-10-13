//! Serialized ABI fixture for compiler and runtime boundary tests.
use std::{ffi::CStr, ptr};
use ulua::*;
pub struct Script {
    context: *mut Context,
    module: *mut Module,
    instance: *mut Instance,
}
fn code(d: &Diagnostic) -> String {
    // SAFETY: The ABI always terminates the fixed diagnostic code array.
    unsafe {
        CStr::from_ptr(d.code.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
fn message(d: &Diagnostic) -> String {
    // SAFETY: The ABI always terminates the fixed diagnostic message array.
    unsafe {
        CStr::from_ptr(d.message.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
impl Script {
    pub fn compile(source: &str, limits: CompileLimits, expected: &str) -> Self {
        let mut s = Self {
            context: ptr::null_mut(),
            module: ptr::null_mut(),
            instance: ptr::null_mut(),
        };
        let mut d = Diagnostic::default();
        // SAFETY: Source and descriptors cover their lengths; the test serializes all object access.
        unsafe {
            assert_eq!(ulua_context_new(&mut s.context), ULUA_OK);
            let status = ulua_compile(
                s.context,
                source.as_ptr().cast(),
                source.len(),
                std::ptr::null(),
                0,
                &limits,
                &mut s.module,
                &mut d,
            );
            if expected.is_empty() {
                assert_eq!(status, ULUA_OK, "{}: {}", code(&d), message(&d));
                assert!(!s.module.is_null());
            } else {
                assert_eq!(status, ULUA_REJECTED, "{}: {}", code(&d), message(&d));
                assert_eq!(code(&d), expected, "{source}");
                assert!(s.module.is_null());
            }
        }
        s
    }
    pub fn new(source: &str) -> Self {
        Self::compile(source, CompileLimits::default(), "")
    }
    pub fn index(&self, name: &str) -> u32 {
        let mut index = u32::MAX;
        // SAFETY: The fixture owns a live module, name bytes, and output storage.
        unsafe {
            assert_eq!(
                ulua_module_find_export(self.module, name.as_ptr().cast(), name.len(), &mut index),
                ULUA_OK
            );
        }
        index
    }
    pub fn instance(&mut self, limits: RunLimits) -> (Status, String) {
        self.instance_with_host(limits, &Host::default())
    }
    pub fn instance_with_host(&mut self, limits: RunLimits, host: &Host) -> (Status, String) {
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns both pointers and serializes their lifetime transitions.
        unsafe {
            ulua_instance_free(self.instance);
            self.instance = ptr::null_mut();
            let status = ulua_instance_new(
                self.context,
                self.module,
                host,
                &limits,
                &mut self.instance,
                &mut d,
            );
            assert!(
                status == ULUA_OK || status == ULUA_FAILED,
                "{}: {}",
                code(&d),
                message(&d)
            );
            if code(&d) == "limit:steps" {
                assert_eq!(message(&d), "Step limit exceeded");
            }
            (status, code(&d))
        }
    }
    pub fn call(&self, index: u32, arguments: &[Value]) -> (Status, String, Value) {
        let (status, d, value) = self.call_diagnostic(index, arguments);
        assert!(
            status == ULUA_OK || status == ULUA_FAILED,
            "{}: {}",
            code(&d),
            message(&d)
        );
        if code(&d) == "limit:steps" {
            assert_eq!(message(&d), "Step limit exceeded");
        }
        (status, code(&d), value)
    }
    pub fn call_diagnostic(&self, index: u32, arguments: &[Value]) -> (Status, Diagnostic, Value) {
        let mut d = Diagnostic::default();
        let mut value = Value::default();
        // SAFETY: The fixture owns a live instance and every argument and output covers its length.
        unsafe {
            let status = ulua_call(
                self.instance,
                index,
                arguments.as_ptr(),
                arguments.len(),
                &mut value,
                &mut d,
            );
            (status, d, value)
        }
    }
}
impl Drop for Script {
    fn drop(&mut self) {
        // SAFETY: The fixture owns its context; context_free releases all remaining children once.
        unsafe {
            assert_eq!(ulua_context_free(self.context), ULUA_OK);
        }
    }
}
pub fn number(n: usize) -> Value {
    let mut value = Value::default();
    value.r#type = ULUA_TYPE_NUMBER;
    value.number = n as f64;
    value
}
