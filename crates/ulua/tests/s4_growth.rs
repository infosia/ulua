//! Growth gate: source ceiling shapes, 16 KiB and 64 KiB, minimum of three full ABI compiles.
//! Measured Debug cost: about 11 seconds. The test prints each ratio and its wall cost.
// This test uses the source and graph generators only.
#[allow(dead_code)]
#[path = "../src/s3_inputs.rs"]
mod inputs;
// This test uses the compile operation only.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::time::{Duration, Instant};
use support::Script;
use ulua_compiler::{COMPILER_STACK_BYTES, Limits, OUTPUT_FACTOR, OUTPUT_PRELUDE_BYTES};

#[test]
fn full_pipeline_growth_stays_below_eight() {
    let start = Instant::now();
    let n = 16 * 1024;
    for shape in inputs::SOURCE_SHAPES {
        let make = |bytes| {
            if matches!(*shape, "forward" | "backward" | "star") {
                inputs::call_graph(shape, bytes)
            } else {
                inputs::source_ceiling(shape, bytes)
            }
        };
        let small = make(n);
        let large = make(4 * n);
        assert_eq!(small.len(), n);
        assert_eq!(large.len(), 4 * n);
        let generated = std::thread::Builder::new()
            .stack_size(COMPILER_STACK_BYTES)
            .spawn({
                let source = large.clone();
                move || {
                    ulua_compiler::compile(source.as_bytes(), Limits::default())
                        .unwrap()
                        .luau
                }
            })
            .unwrap()
            .join()
            .unwrap();
        let output_ratio =
            generated.len().saturating_sub(OUTPUT_PRELUDE_BYTES) as f64 / large.len() as f64;
        eprintln!(
            "S5 output {shape}: source={} generated={} ratio={output_ratio:.3} bound={OUTPUT_FACTOR}",
            large.len(),
            generated.len()
        );
        assert!(
            generated.len() <= OUTPUT_PRELUDE_BYTES + OUTPUT_FACTOR * large.len(),
            "{shape}: output ratio {output_ratio}"
        );
        let mut times = [Duration::MAX; 2];
        for _ in 0..3 {
            for (i, source) in [&small, &large].into_iter().enumerate() {
                let now = Instant::now();
                let script = Script::new(source);
                times[i] = times[i].min(now.elapsed());
                drop(script);
            }
        }
        let ratio = times[1].as_secs_f64() / times[0].as_secs_f64();
        eprintln!(
            "S4 growth {shape}: n={n} {:?}; 4n={} {:?}; ratio={ratio:.3}",
            times[0],
            4 * n,
            times[1]
        );
        assert!(ratio <= 8.0, "{shape}: ratio {ratio}");
    }
    eprintln!("S4 growth gate cost: {:?}", start.elapsed());
}
