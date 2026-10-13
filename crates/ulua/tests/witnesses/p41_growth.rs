//! Await shapes compile and run at 4 KiB, 16 KiB, and the source ceiling.
//! Compiler ratios stay at most eight for four times the source. Each size uses the minimum of three runs.
//! Costs are reported for twenty-two shapes, including nine P4.2 shapes.
//! The witness-time target asserts analyzer ratios. This target also reports ABI costs and output growth.
#[path = "../support/p41_shapes.rs"]
mod shapes;
use std::time::Instant;

#[test]
fn await_compiler_growth_is_bounded() {
    let _measurement = crate::measurement_lock();
    compiler_growth([4096, 16384]);
}

#[cfg(feature = "measurements")]
#[test]
fn await_compiler_ceiling_growth_is_bounded() {
    let _measurement = crate::measurement_lock();
    compiler_growth([16384, 65536]);
}

fn compiler_growth(sizes: [usize; 2]) {
    let start = Instant::now();
    let mut failures = vec![];
    for shape in shapes::SHAPES {
        let mut times = vec![];
        for bytes in sizes {
            let source = shapes::source(shape, bytes);
            assert_eq!(source.len(), bytes);
            let mut minimum = f64::INFINITY;
            for _ in 0..3 {
                let now = Instant::now();
                let compiled =
                    ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
                        .unwrap();
                minimum = minimum.min(now.elapsed().as_secs_f64());
                assert!(
                    compiled.luau.len()
                        <= ulua_compiler::OUTPUT_PRELUDE_BYTES
                            + ulua_compiler::OUTPUT_ASYNC_PRELUDE_BYTES
                            + bytes * ulua_compiler::OUTPUT_FACTOR
                );
            }
            times.push(minimum);
        }
        let ratio = times[1] / times[0];
        eprintln!(
            "P4.1 compiler {shape} source={}/{} seconds={:.6}/{:.6} ratio={ratio:.3}",
            sizes[0], sizes[1], times[0], times[1],
        );
        if ratio > 8.0 {
            failures.push(format!("{shape}: {ratio:.3}"));
        }
    }
    eprintln!(
        "P4.1 compiler growth cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(
        failures.is_empty(),
        "Compiler ratios: {}",
        failures.join(", ")
    );
}
