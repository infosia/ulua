//! P6.4 keeps a small live heap below the byte limit during closure churn.
use std::{
    ffi::{c_char, c_void},
    time::Instant,
};
use ulua::RunLimits;

// SAFETY: The probes accept live byte buffers, a limits descriptor, and fixed output arrays.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        size: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(bytecode: *mut c_char);
    fn ulua_internal_probe_capture_memory(
        bytecode: *const c_char,
        length: usize,
        limits: *const c_void,
        goal: i32,
        values: *mut u64,
        error: *mut c_char,
    ) -> i32;
}

fn source() -> String {
    let mut source = String::new();
    for frame in 0..18 {
        source.push_str(&format!("function f{frame}():number{{"));
        for binding in frame * 10..frame * 10 + 10 {
            source.push_str(&format!("const v{binding}={binding};"));
        }
    }
    // The closure destination spills. Each enclosing frame keeps ten captured locals.
    for binding in 0..220 {
        source.push_str(&format!("const unused{binding}={binding};"));
    }
    source.push_str("while(true){const g=():number=>{let s=0;");
    for binding in 0..180 {
        source.push_str(&format!("s+=v{binding};"));
    }
    source.push_str("return s;};}return 0;}");
    for frame in (0..17).rev() {
        source.push_str(&format!("return f{}();}}", frame + 1));
    }
    source.push_str("export function main():number{return f0();}");
    source
}

// Cost: one small compile and two default-step runs; Release 0.085 seconds. Byte counts are the assertions.
#[test]
fn spilled_closure_loop_finishes_collection_before_memory_limit() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let luau = ulua_compiler::compile(source().as_bytes(), Default::default())
        .unwrap()
        .luau;
    let mut bytecode = std::ptr::null_mut();
    let mut length = 0;
    let mut error = [0; 256];
    // SAFETY: The native compiler reads the source and returns one owned bytecode buffer.
    assert_eq!(
        unsafe {
            ulua_internal_shim_compile(
                luau.as_ptr().cast(),
                luau.len(),
                &mut bytecode,
                &mut length,
                error.as_mut_ptr(),
            )
        },
        0
    );
    let mut failures = Vec::new();
    for memory in [4 * 1024 * 1024, 16 * 1024 * 1024] {
        let mut limits = RunLimits::default();
        limits.max_memory_bytes = memory;
        let mut values = [0; 17];
        // SAFETY: The synchronous probe owns its VM. Every borrowed buffer covers its declared size.
        let status = unsafe {
            ulua_internal_probe_capture_memory(
                bytecode,
                length,
                (&limits as *const RunLimits).cast(),
                0,
                values.as_mut_ptr(),
                error.as_mut_ptr(),
            )
        };
        eprintln!(
            "P6.4 memory={memory} status={status} peak={} steps={} cycles={}",
            values[2], values[6], values[7]
        );
        if status != 2 || values[2] >= 4 * 1024 * 1024 || values[14] != 0 {
            failures.push((memory, status, values));
        }
    }
    // SAFETY: Both probes finished. Release the compiler-owned buffer exactly once.
    unsafe { ulua_internal_shim_bytecode_free(bytecode) };
    eprintln!("P6.4 test cost={:.3}s", start.elapsed().as_secs_f64());
    assert!(failures.is_empty(), "GC pacing failures: {failures:?}");
}
