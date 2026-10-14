//! Instruction gaps use lua_singlestep and the production compile options.
//! Measured Debug cost: 8.529 seconds for 26 source-ceiling shapes and both branch choices.
// Shared generators also serve the ratio and boundary fixtures.
#[allow(dead_code)]
#[path = "../support/interrupt_shapes.rs"]
mod shapes;
use std::{ffi::CStr, time::Instant};
use ulua_compiler::{COMPILER_STACK_BYTES, Limits};

// SAFETY: The probe reads source bytes and writes only the supplied counters and diagnostic.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_interrupts(
        source: *const u8,
        length: usize,
        branch: i32,
        largest: *mut u64,
        interrupts: *mut u64,
        error: *mut std::ffi::c_char,
    ) -> i32;
}

#[test]
fn source_ceiling_instruction_gaps() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    assert_eq!(ulua::ulua_abi_version(), 1);
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| {
            check_gaps(shapes::all());
        })
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt instruction test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

/// Measured Debug cost: 0.898 seconds for seven source-ceiling conditions and both branch outcomes.
#[test]
fn wrapped_condition_instruction_gaps() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| check_gaps(shapes::conditions(65536)))
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt wrapped condition cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

fn check_gaps<S: AsRef<str>>(sources: Vec<(S, String)>) {
    let mut maximum = 0;
    for (name, source) in sources {
        let name = name.as_ref();
        assert_eq!(source.len(), 65536, "{name}");
        let luau = ulua_compiler::compile(source.as_bytes(), Limits::default())
            .unwrap_or_else(|error| panic!("{name}: {error:?}"))
            .luau;
        let mut worst = 0;
        for branch in [0, 1] {
            let mut gap = 0;
            let mut interrupts = 0;
            let mut error = [0; 256];
            // SAFETY: All pointers cover their declared lengths and remain live for this synchronous probe.
            let status = unsafe {
                ulua_internal_probe_interrupts(
                    luau.as_ptr(),
                    luau.len(),
                    branch,
                    &mut gap,
                    &mut interrupts,
                    error.as_mut_ptr(),
                )
            };
            // SAFETY: The probe terminates this fixed diagnostic buffer.
            assert_eq!(
                status,
                0,
                "{name}: {}",
                unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
            );
            assert!(interrupts > 0);
            worst = worst.max(gap);
        }
        maximum = maximum.max(worst);
        eprintln!("interrupt gap {name}: {worst}");
    }
    eprintln!("interrupt maximum={maximum}");
    assert!(
        maximum <= ulua_compiler::INTERRUPT_INSTRUCTIONS,
        "instruction gap {maximum} exceeds {}",
        ulua_compiler::INTERRUPT_INSTRUCTIONS
    );
}

/// Measured Debug cost: 3.976 seconds for twelve source-ceiling growth forms under the VM instruction probe.
#[test]
fn analyzer_growth_instruction_gaps() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| check_gaps(shapes::analyzer_growth(65536)))
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt analyzer growth cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
