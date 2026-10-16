//! Queue keys and FIFO output use independent native observations. No assertion reads wall-clock time.
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
    time::Instant,
};
use ulua::{Host, RunLimits};

// SAFETY: The probe protects VM operations and writes only to the supplied live output storage.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_async_queue(
        source: *const u8,
        length: usize,
        host: *const c_void,
        limits: *const c_void,
        largest: *mut f64,
        rebases: *mut usize,
        error: *mut c_char,
    ) -> i32;
}

unsafe extern "C" fn output(data: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The fixture keeps the vector live. The callback receives readable bytes for this call.
    unsafe {
        (*data.cast::<Vec<String>>()).push(
            String::from_utf8_lossy(std::slice::from_raw_parts(text.cast(), length)).into_owned(),
        );
    }
    0
}

// Cost: three small compiler calls and three drains of 3079 jobs. No external witness runs here.
#[test]
fn a_single_drain_bounds_indices_and_keeps_fifo_across_empty_resets_and_compaction() {
    const THRESHOLD: usize = 1024;
    const JOBS: usize = 3 * THRESHOLD + 7;
    let start = Instant::now();
    for width in [1, 3, THRESHOLD + 5] {
        let source = format!(
            r#"
async function ready():Promise<number>{{return 0;}}
const p=ready();
let next=0;
function enqueue():void{{
    const id=next;next++;
    const reaction=p.then((n:number):number=>{{
        print(`${{id}}`);
        if(next<{JOBS}){{enqueue();}}
        return n;
    }});
}}
export function main():void{{for(let i=0;i<{width};i++){{enqueue();}}}}
"#
        );
        let luau = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
            .unwrap()
            .luau;
        let mut lines = Vec::<String>::new();
        let mut host = Host::default();
        host.userdata = ptr::from_mut(&mut lines).cast();
        host.output = Some(output);
        let limits = RunLimits::default();
        let mut largest = 0.0;
        let mut rebases = 0;
        let mut error = [0 as c_char; 256];
        // SAFETY: The buffers, descriptors, and callback state stay live until the probe returns.
        let status = unsafe {
            ulua_internal_probe_async_queue(
                luau.as_ptr(),
                luau.len(),
                ptr::from_ref(&host).cast(),
                ptr::from_ref(&limits).cast(),
                &mut largest,
                &mut rebases,
                error.as_mut_ptr(),
            )
        };
        // SAFETY: The probe and shim terminate each diagnostic with a NUL byte.
        let diagnostic = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
        assert_eq!(status, 0, "width={width}: {diagnostic}");
        let expected: Vec<_> = (0..JOBS).map(|id| id.to_string()).collect();
        assert_eq!(lines, expected, "width={width}");
        let bound = THRESHOLD + width - 1;
        assert!(
            (1.0..=bound as f64).contains(&largest),
            "width={width} index={largest}"
        );
        if width == 1 {
            assert_eq!(
                largest, 1.0,
                "the queue resets before the next job can enqueue"
            );
        } else {
            assert!(
                rebases > 0,
                "the live queue must cross the compaction threshold"
            );
        }
        eprintln!(
            "async queue jobs={JOBS} live_width={width} largest_index={largest} bound={bound} rebases={rebases}"
        );
    }
    eprintln!(
        "async queue index test cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
