//! Calibration measures capture counts near the limit. It reports its cost without a time assertion.
//! The 16-shape ratio test costs 1.668 seconds in Debug and 1.531 seconds in Release.
//! Each sample uses two warm runs and the default one-million-step budget.
#[path = "../support/capture_shapes.rs"]
#[allow(dead_code)]
mod shapes;
#[path = "../support/script.rs"]
#[allow(dead_code)]
mod support;
use std::time::Instant;
use ulua::{RunLimits, ULUA_FAILED};

fn run(source: &str) -> (String, f64) {
    let mut script = support::Script::new(source);
    // One warm run left a 7% initial excess. Two warm runs exclude the observed startup cost.
    for _ in 0..2 {
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_FAILED);
    }
    let start = Instant::now();
    let (status, code) = script.instance(RunLimits::default());
    assert_eq!(status, ULUA_FAILED);
    (code, start.elapsed().as_secs_f64())
}

#[test]
#[ignore = "capture limit calibration; run before a storage change"]
fn captured_local_calibration() {
    // After this fix, use CAPTURE_LOCALS >= 16 in a scratch build for an unspilled calibration.
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    for count in [4, 8, 9, 10, 11, 12, 16] {
        for (kind, writes, nested) in [
            ("read", false, false),
            ("write", true, false),
            ("nested-read", false, true),
            ("nested-write", true, true),
        ] {
            let (code, time) = run(&shapes::source(count, writes, nested));
            assert_eq!(code, "limit:steps");
            // Pair each sample with a warm baseline to exclude the initial clock ramp.
            let (code, baseline) = run(shapes::BASELINE);
            assert_eq!(code, "limit:steps");
            eprintln!(
                "capture calibration N={count} {kind}: baseline={baseline:.6}s time={time:.6}s ratio={:.3}",
                time / baseline
            );
        }
    }
    eprintln!(
        "capture calibration cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn closure_capture_step_time_ratios() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let mut failures = Vec::new();
    for (name, source) in shapes::ratios(ulua_compiler::CAPTURE_LOCALS) {
        let (code, time) = run(&source);
        let (baseline_code, baseline) = run(shapes::BASELINE);
        assert_eq!(baseline_code, "limit:steps");
        let ratio = time / baseline;
        eprintln!(
            "capture ratio {name}: code={code} baseline={baseline:.6}s time={time:.6}s ratio={ratio:.3}"
        );
        if code != "limit:steps" || ratio > 6.0 {
            failures.push((name, code, ratio));
        }
    }
    eprintln!("capture ratios cost={:.3}s", start.elapsed().as_secs_f64());
    assert!(failures.is_empty(), "capture ratios exceed 6: {failures:?}");
}

// SAFETY: The private probes accept live byte buffers and fixed output arrays.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_capture_memory(
        bytecode: *const std::ffi::c_char,
        length: usize,
        limits: *const std::ffi::c_void,
        goal: i32,
        values: *mut u64,
        error: *mut std::ffi::c_char,
    ) -> i32;
    fn ulua_internal_shim_compile(
        source: *const std::ffi::c_char,
        length: usize,
        bytecode: *mut *mut std::ffi::c_char,
        size: *mut usize,
        error: *mut std::ffi::c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(bytecode: *mut std::ffi::c_char);
}

#[test]
#[ignore = "GC pacing measurement; no production policy change"]
fn spilled_capture_memory() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    // The prefix forces all 180 captured bindings to spill, including at HEAD.
    let prefix = (0..220)
        .map(|i| format!("const unused{i}={i};"))
        .collect::<String>();
    for (name, prefix) in [("mixed", ""), ("all-spilled", prefix.as_str())] {
        let source = shapes::source(180, false, false)
            .replacen(
                "function start():number{",
                &format!("function start():number{{{prefix}"),
                1,
            )
            .replace(
                "const stop=start();export function main():void{}",
                "export function main():number{return start();}",
            );
        let luau = ulua_compiler::compile(source.as_bytes(), Default::default())
            .unwrap()
            .luau;
        let mut bytecode = std::ptr::null_mut();
        let mut size = 0;
        let mut error = [0; 256];
        // SAFETY: All buffers cover their lengths. Free the returned bytecode once below.
        assert_eq!(
            unsafe {
                ulua_internal_shim_compile(
                    luau.as_ptr().cast(),
                    luau.len(),
                    &mut bytecode,
                    &mut size,
                    error.as_mut_ptr(),
                )
            },
            0
        );
        for (memory, goal) in [
            (16 * 1024 * 1024, 0),
            (32 * 1024 * 1024, 0),
            (16 * 1024 * 1024, 110),
            (16 * 1024 * 1024, -1000),
        ] {
            let mut limits = RunLimits::default();
            limits.max_memory_bytes = memory;
            let mut values = [0u64; 17];
            // SAFETY: The probe runs synchronously and owns its VM. Each output has its declared size.
            let status = unsafe {
                ulua_internal_probe_capture_memory(
                    bytecode,
                    size,
                    (&limits as *const RunLimits).cast(),
                    goal,
                    values.as_mut_ptr(),
                    error.as_mut_ptr(),
                )
            };
            eprintln!(
                "capture memory {name} budget={memory} goal={goal} status={status} values={values:?}"
            );
            assert_ne!(status, 5, "the measurement must not return SHIM_INTERNAL");
            assert_eq!(
                values[14], 0,
                "all VM allocator bytes must return after close"
            );
        }
        // SAFETY: The compiler returned this buffer. All synchronous readers finished.
        unsafe { ulua_internal_shim_bytecode_free(bytecode) };
    }
    eprintln!("capture memory cost={:.3}s", start.elapsed().as_secs_f64());
}
