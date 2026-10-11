//! S4 Release measurements use the production compiler, shim, and budget allocator.
//! Run with `cargo run --offline --release -p ulua --features measurements --example s4_measure`.
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
    time::Instant,
};
use ulua::*;
use ulua_compiler::{COMPILER_STACK_BYTES, Limits};

// The repr(C) ABI descriptors have fixed layouts; non_exhaustive prevents Rust struct literals only.
#[allow(improper_ctypes)]
// SAFETY: These declarations match the private shim and its opt-in measurement probes.
#[link(name = "ulua_test_native", kind = "static")]
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
        limits: *const RunLimits,
        out: *mut *mut c_void,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_call(
        vm: *mut c_void,
        name: *const c_char,
        args: *const Value,
        count: usize,
        result_type: u32,
        result: *mut Value,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_free(vm: *mut c_void);
    fn ulua_internal_probe_run_stats(
        vm: *mut c_void,
        bytes: *mut u64,
        charges: *mut u64,
        interrupts: *mut u64,
    );
}
fn measure(label: &str, setup: &str, body: &str, iterations: usize) {
    measure_budget(label, setup, body, iterations, u64::MAX);
}
fn measure_budget(label: &str, setup: &str, body: &str, iterations: usize, steps: u64) {
    let source = format!(
        "export function main():void {{{setup}let t=\"\";let i=0;while(i<{iterations}){{{body}i++;}}}}"
    );
    let compiled = ulua_compiler::compile(source.as_bytes(), Limits::default()).unwrap();
    let mut bytecode = ptr::null_mut();
    let mut size = 0;
    let mut error = [0; 256];
    let mut limits = RunLimits::default();
    limits.max_steps = steps;
    let mut vm = ptr::null_mut();
    // SAFETY: This scope owns the shim outputs and frees each object after its last use.
    unsafe {
        assert_eq!(
            ulua_internal_shim_compile(
                compiled.luau.as_ptr().cast(),
                compiled.luau.len(),
                &mut bytecode,
                &mut size,
                error.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ulua_internal_shim_new(
                bytecode,
                size,
                &Host::default(),
                &limits,
                &mut vm,
                error.as_mut_ptr()
            ),
            0
        );
        ulua_internal_shim_bytecode_free(bytecode);
        let mut result = Value::default();
        let start = Instant::now();
        let status = ulua_internal_shim_call(
            vm,
            c"main".as_ptr(),
            ptr::null(),
            0,
            ULUA_TYPE_VOID,
            &mut result,
            error.as_mut_ptr(),
        );
        let seconds = start.elapsed().as_secs_f64();
        let (mut bytes, mut charges, mut interrupts) = (0, 0, 0);
        ulua_internal_probe_run_stats(vm, &mut bytes, &mut charges, &mut interrupts);
        println!(
            "S4 runtime {label}: iterations={iterations} seconds={seconds:.6} allocated_bytes={bytes} allocation_steps={charges} interrupts={interrupts} ns_per_byte={:.6} ns_per_step={:.3} million_steps_seconds={:.6} status={status} {}",
            seconds * 1e9 / bytes.max(1) as f64,
            seconds * 1e9 / (charges + interrupts).max(1) as f64,
            seconds * 1e6 / (charges + interrupts).max(1) as f64,
            CStr::from_ptr(error.as_ptr()).to_string_lossy()
        );
        assert!(status == 0 || (status == 2 && steps == DEFAULT_STEPS));
        ulua_internal_shim_free(vm);
    }
}
fn main() {
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| {
            println!("ALLOCATION_STEP_BYTES={ALLOCATION_STEP_BYTES}");
            measure("tight-loop", "", "", 1_000_000);
            let setup = "let s=\"x\";let j=0;while(j<18){s=`${s}${s}`;j++;}";
            measure("concat-4-large", setup, "t=`${s}${s}${s}${s}`;", 1000);
            measure("concat-4-distinct", setup, "t=`${s}${s}${s}${i}`;", 1000);
            measure_budget(
                "concat-4-large-budget",
                setup,
                "t=`${s}${s}${s}${s}`;",
                1000,
                DEFAULT_STEPS,
            );
            let template = format!("t=`{}${{i}}`;", "${s}".repeat(256));
            let setup = "let s=\"x\";let j=0;while(j<14){s=`${s}${s}`;j++;}";
            measure("template-515-parts", setup, &template, 3);
            measure(
                "template-17-parts",
                setup,
                "t=`${s}${s}${s}${s}${s}${s}${s}${i}`;",
                1000,
            );
            measure("String-integer", "", "t=String(i);", 100_000);
            measure("String-fraction", "", "t=String(i+0.123456789);", 100_000);
            measure("String-exponent", "", "t=String(i*1e280);", 100_000);
            measure_budget(
                "String-exponent-budget",
                "",
                "t=String(i*1e280);",
                1_000_000,
                DEFAULT_STEPS,
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
