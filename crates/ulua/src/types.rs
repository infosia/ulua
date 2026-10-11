//! Version 1 descriptor layouts and constants.
use std::ffi::{c_char, c_void};

/// ABI status integer.
pub type Status = i32;
/// Success.
pub const ULUA_OK: Status = 0;
/// Invalid host argument.
pub const ULUA_INVALID_ARGUMENT: Status = 1;
/// Source rejection.
pub const ULUA_REJECTED: Status = 2;
/// Runtime trap or limit.
pub const ULUA_FAILED: Status = 3;
/// Output callback failure.
pub const ULUA_HOST_ERROR: Status = 4;
/// Callback reentry.
pub const ULUA_BUSY: Status = 5;
/// Library defect.
pub const ULUA_INTERNAL: Status = 6;
/// Poisoned context.
pub const ULUA_POISONED: Status = 7;
/// Void type tag.
pub const ULUA_TYPE_VOID: u32 = 0;
/// Number type tag.
pub const ULUA_TYPE_NUMBER: u32 = 1;
/// Boolean type tag.
pub const ULUA_TYPE_BOOLEAN: u32 = 2;
/// String type tag.
pub const ULUA_TYPE_STRING: u32 = 3;
/// Minimum host stack for VM operations.
pub const ULUA_MIN_RUN_STACK: usize = 4 * 1024 * 1024; // provisional: S3 measures
/// Default per-instance memory budget.
pub const DEFAULT_MEMORY_BYTES: u64 = 64 * 1024 * 1024; // provisional: S3 measures
/// Default interrupt budget for one operation.
pub const DEFAULT_STEPS: u64 = 1_000_000; // provisional: S3 measures

/// Compiler limits descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct CompileLimits {
    /// Must equal sizeof this descriptor.
    pub struct_size: u32,
    /// Must be zero.
    pub reserved: u32,
    /// Source byte budget.
    pub max_source_bytes: u64,
    /// Parser and AST depth budget.
    pub max_nesting_depth: u32,
    /// Must be zero.
    pub reserved2: u32,
}
impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            reserved: 0,
            max_source_bytes: ulua_compiler::SOURCE_CEILING,
            max_nesting_depth: ulua_compiler::DEPTH_CEILING,
            reserved2: 0,
        }
    }
}
/// Runtime limits descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct RunLimits {
    /// Must equal sizeof this descriptor.
    pub struct_size: u32,
    /// Must be zero.
    pub reserved: u32,
    /// Per-instance allocator budget.
    pub max_memory_bytes: u64,
    /// Interrupt budget for each operation.
    pub max_steps: u64,
}
impl Default for RunLimits {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            reserved: 0,
            max_memory_bytes: DEFAULT_MEMORY_BYTES,
            max_steps: DEFAULT_STEPS,
        }
    }
}
/// Output callback. The text remains valid until the callback returns.
pub type OutputFn = unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> i32;
/// Host output capability descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Host {
    /// Must equal sizeof this descriptor.
    pub struct_size: u32,
    /// Must be zero.
    pub reserved: u32,
    /// Host callback data.
    pub userdata: *mut c_void,
    /// Optional output capability.
    pub output: Option<OutputFn>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            reserved: 0,
            userdata: std::ptr::null_mut(),
            output: None,
        }
    }
}
/// Version 1 value. Unused fields must be zero or null.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Value {
    /// ABI type tag.
    pub r#type: u32,
    /// Boolean value, zero or one.
    pub boolean: u32,
    /// Number value.
    pub number: f64,
    /// UTF-8 string bytes.
    pub data: *const c_char,
    /// String byte count.
    pub length: usize,
}
impl Default for Value {
    fn default() -> Self {
        Self {
            r#type: ULUA_TYPE_VOID,
            boolean: 0,
            number: 0.0,
            data: std::ptr::null(),
            length: 0,
        }
    }
}
/// Compiler or runtime diagnostic descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Diagnostic {
    /// Must equal sizeof this descriptor.
    pub struct_size: u32,
    /// Must be zero.
    pub reserved: u32,
    /// NUL-terminated ASCII diagnostic code.
    pub code: [c_char; 32],
    /// NUL-terminated UTF-8 diagnostic message.
    pub message: [c_char; 256],
    /// Source byte offset, or u64::MAX for runtime failures.
    pub offset: u64,
    /// One-based line, or zero for runtime failures.
    pub line: u32,
    /// One-based byte column, or zero for runtime failures.
    pub column: u32,
}
impl Default for Diagnostic {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            reserved: 0,
            code: [0; 32],
            message: [0; 256],
            offset: u64::MAX,
            line: 0,
            column: 0,
        }
    }
}
impl Diagnostic {
    pub(crate) fn set(&mut self, code: &str, message: &str) {
        copy_text(&mut self.code, code);
        copy_text(&mut self.message, message);
    }
}
fn copy_text<const N: usize>(out: &mut [c_char; N], text: &str) {
    out.fill(0);
    let mut len = text.len().min(N - 1);
    while !text.is_char_boundary(len) {
        len -= 1;
    }
    for (target, byte) in out.iter_mut().zip(&text.as_bytes()[..len]) {
        *target = *byte as c_char;
    }
}
