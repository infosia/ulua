//! The benchmark calls the product C ABI and the pinned VM directly.
use crate::stats::{Measurement, SAMPLES, Sample, WARMUP_ITERATIONS, WARMUP_SECONDS};
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
    time::Instant,
};
use ulua::*;

pub(crate) const MAX_STEPS: u64 = 10_000_000_000;
pub(crate) const MAX_MEMORY_BYTES: u64 = 1_073_741_824;

// SAFETY: These declarations match the pinned VM and the private noexcept host.
unsafe extern "C" {
    fn ulua_bench_luau_new(source: *const c_char, length: usize, error: *mut c_char)
    -> *mut c_void;
    fn ulua_bench_luau_result(
        state: *mut c_void,
        status: i32,
        result: *mut f64,
        error: *mut c_char,
    ) -> i32;
    fn ulua_bench_luau_prepare(state: *mut c_void);
    fn ulua_bench_luau_call(state: *mut c_void) -> i32;
    fn ulua_bench_luau_free(state: *mut c_void);
}
fn text(bytes: &[c_char]) -> String {
    // SAFETY: Both error producers terminate their fixed 256-byte arrays.
    unsafe {
        CStr::from_ptr(bytes.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
fn diagnostic(status: Status, d: &Diagnostic) -> String {
    format!(
        "ABI status {status}: {}: {}",
        text(&d.code),
        text(&d.message)
    )
}
fn checksum(number: f64) -> Result<u64, String> {
    if !number.is_finite() || number.fract() != 0.0 || !(0.0..4_294_967_296.0).contains(&number) {
        return Err(format!("Checksum is not a reduced integer: {number}"));
    }
    Ok(number as u64)
}
pub(crate) trait Workload {
    fn prepare(&mut self);
    fn call(&mut self) -> i32;
    fn result(&mut self, status: i32) -> Result<u64, String>;
}

pub(crate) struct Ulua {
    context: *mut Context,
    module: *mut Module,
    instance: *mut Instance,
    index: u32,
    value: Value,
    diagnostic: Diagnostic,
}
impl Ulua {
    pub fn compile(source: &[u8]) -> Result<Self, String> {
        let mut s = Self {
            context: ptr::null_mut(),
            module: ptr::null_mut(),
            instance: ptr::null_mut(),
            index: 0,
            value: Value::default(),
            diagnostic: Diagnostic::default(),
        };
        // SAFETY: Local storage covers all lengths. This owner serializes and frees the ABI objects.
        unsafe {
            let status = ulua_context_new(&mut s.context);
            if status != ULUA_OK {
                return Err(diagnostic(status, &s.diagnostic));
            }
            let status = ulua_compile(
                s.context,
                source.as_ptr().cast(),
                source.len(),
                ptr::null(),
                0,
                &CompileLimits::default(),
                &mut s.module,
                &mut s.diagnostic,
            );
            if status != ULUA_OK {
                return Err(diagnostic(status, &s.diagnostic));
            }
            let mut count = 0;
            let status = ulua_module_export_count(s.module, &mut count);
            if status != ULUA_OK || count != 1 {
                return Err("Expected exactly one exported function".into());
            }
            let status = ulua_module_find_export(s.module, c"main".as_ptr(), 4, &mut s.index);
            if status != ULUA_OK {
                return Err(diagnostic(status, &s.diagnostic));
            }
        }
        Ok(s)
    }
    pub fn new(source: &[u8]) -> Result<Self, String> {
        let mut s = Self::compile(source)?;
        let mut limits = RunLimits::default();
        limits.max_steps = MAX_STEPS;
        limits.max_memory_bytes = MAX_MEMORY_BYTES;
        // SAFETY: This owner keeps the context, module, descriptors, and output storage live.
        let status = unsafe {
            ulua_instance_new(
                s.context,
                s.module,
                &Host::default(),
                &limits,
                &mut s.instance,
                &mut s.diagnostic,
            )
        };
        if status != ULUA_OK {
            return Err(diagnostic(status, &s.diagnostic));
        }
        Ok(s)
    }
}
impl Workload for Ulua {
    fn prepare(&mut self) {
        self.value = Value::default();
        self.diagnostic = Diagnostic::default();
    }
    fn call(&mut self) -> i32 {
        // SAFETY: This owner holds the live instance and outputs. main takes no arguments.
        unsafe {
            ulua_call(
                self.instance,
                self.index,
                ptr::null(),
                0,
                &mut self.value,
                &mut self.diagnostic,
            )
        }
    }
    fn result(&mut self, status: i32) -> Result<u64, String> {
        if status != ULUA_OK {
            return Err(diagnostic(status, &self.diagnostic));
        }
        if self.value.r#type != ULUA_TYPE_NUMBER {
            return Err("Checksum must be a number".into());
        }
        checksum(self.value.number)
    }
}
impl Drop for Ulua {
    fn drop(&mut self) {
        // SAFETY: The context owns all remaining modules and instances and is freed once.
        unsafe {
            ulua_context_free(self.context);
        }
    }
}

pub(crate) struct Luau(*mut c_void);
impl Luau {
    pub fn new(source: &[u8]) -> Result<Self, String> {
        let mut error = [0; 256];
        // SAFETY: The source and error buffer cover their lengths. The host returns an owned VM.
        let state = unsafe {
            ulua_bench_luau_new(source.as_ptr().cast(), source.len(), error.as_mut_ptr())
        };
        if state.is_null() {
            return Err(text(&error));
        }
        Ok(Self(state))
    }
}
impl Workload for Luau {
    fn prepare(&mut self) {
        // SAFETY: Slot 1 retains the workload function in this owned VM.
        unsafe {
            ulua_bench_luau_prepare(self.0);
        }
    }
    fn call(&mut self) -> i32 {
        // SAFETY: prepare pushed a function with no arguments; the VM returns one result.
        unsafe { ulua_bench_luau_call(self.0) }
    }
    fn result(&mut self, status: i32) -> Result<u64, String> {
        let mut result = 0.0;
        let mut error = [0; 256];
        // SAFETY: The VM has the call result or error. The host copies and pops a numeric result.
        let status =
            unsafe { ulua_bench_luau_result(self.0, status, &mut result, error.as_mut_ptr()) };
        if status != 0 {
            return Err(text(&error));
        }
        checksum(result)
    }
}
impl Drop for Luau {
    fn drop(&mut self) {
        // SAFETY: This owner closes the VM exactly once, after the last result.
        unsafe {
            ulua_bench_luau_free(self.0);
        }
    }
}

fn iteration(subject: &mut impl Workload) -> Result<Sample, String> {
    subject.prepare();
    let start = Instant::now();
    let status = subject.call();
    let seconds = start.elapsed().as_secs_f64();
    let checksum = subject.result(status)?;
    Ok(Sample { checksum, seconds })
}
pub(crate) fn measure(subject: &mut impl Workload) -> Result<Measurement, String> {
    let first = iteration(subject)?;
    let mut measurement = Measurement {
        warmup_iterations: 1,
        warmup_seconds: first.seconds,
        warmup_checksum: first.checksum,
        warmup_consistent: true,
        samples: Vec::with_capacity(SAMPLES),
    };
    while measurement.warmup_iterations < WARMUP_ITERATIONS
        || measurement.warmup_seconds < WARMUP_SECONDS
    {
        let sample = iteration(subject)?;
        measurement.warmup_consistent &= sample.checksum == first.checksum;
        measurement.warmup_iterations += 1;
        measurement.warmup_seconds += sample.seconds;
    }
    for _ in 0..SAMPLES {
        measurement.samples.push(iteration(subject)?);
    }
    measurement.validate()?;
    Ok(measurement)
}
