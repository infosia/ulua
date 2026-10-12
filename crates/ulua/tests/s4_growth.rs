//! Growth gate: source ceiling shapes, 8 KiB and 32 KiB, through the full ABI.
//! Ratios above 6.5 use the minimum of two runs per size. Lower ratios retain ample margin.
//! Measured Debug cost: 8.18 seconds with all four type products (16/64 KiB, widths 15/31).
//! Product time ratios: 3.882 / 3.768 / 3.857 / 3.768 (assign / declare / call / self).
//! Measured Release cost: 2.41 seconds. Product time ratios: 3.702 / 3.672 / 3.656 / 3.734.
//! Product output ratios: 10.141 / 10.381 / 11.935 / 10.141 at 65,536 bytes.
//! The largest output ratio is 62.671; 1.25 times this ratio stays below 80.
// This test uses the source and graph generators only.
#[allow(dead_code)]
#[path = "../src/s3_inputs.rs"]
mod inputs;
#[path = "support/growth_joins.rs"]
mod joins;
#[path = "../src/s6_inputs.rs"]
mod review;
// This test uses the compile operation only.
#[path = "../src/p21_inputs.rs"]
#[allow(dead_code)]
mod functions;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::time::Instant;
use support::Script;
use ulua_compiler::{COMPILER_STACK_BYTES, Limits, OUTPUT_FACTOR, OUTPUT_PRELUDE_BYTES};

#[test]
fn full_pipeline_growth_stays_below_eight() {
    let start = Instant::now();
    let n = 8 * 1024;
    let mut max_growth: f64 = 0.0;
    let mut max_output: f64 = 0.0;
    for shape in inputs::SOURCE_SHAPES
        .iter()
        .chain(review::SHAPES)
        .chain(joins::SHAPES)
        .chain(functions::SHAPES)
    {
        let make = |bytes| {
            if functions::SHAPES.contains(shape) {
                functions::source(shape, bytes)
            } else if joins::SHAPES.contains(shape) {
                joins::source(shape, bytes)
            } else if review::SHAPES.contains(shape) {
                review::source(shape, bytes)
            } else if matches!(*shape, "forward" | "backward" | "star") {
                inputs::call_graph(shape, bytes)
            } else {
                inputs::source_ceiling(shape, bytes)
            }
        };
        // The review type occupies about 12.7 KiB. Product shapes use 16/64 KiB.
        let n = if shape.starts_with("type-product-") {
            16 * 1024
        } else {
            n
        };
        let small = make(n);
        let large = make(4 * n);
        let ceiling = make(64 * 1024);
        assert_eq!(small.len(), n);
        assert_eq!(large.len(), 4 * n);
        let generated = std::thread::Builder::new()
            .stack_size(COMPILER_STACK_BYTES)
            .spawn({
                let source = ceiling.clone();
                move || {
                    let result = ulua_compiler::compile(source.as_bytes(), Limits::default());
                    if let Some(code) = functions::diagnostic(shape) {
                        assert_eq!(result.unwrap_err().code, code);
                        String::new()
                    } else {
                        result.unwrap().luau
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
        let output_ratio =
            generated.len().saturating_sub(OUTPUT_PRELUDE_BYTES) as f64 / ceiling.len() as f64;
        eprintln!(
            "S6 output {shape}: source={} generated={} ratio={output_ratio:.3} bound={OUTPUT_FACTOR}",
            ceiling.len(),
            generated.len()
        );
        max_output = max_output.max(output_ratio);
        if let Ok(directory) = std::env::var("ULUA_GROWTH_OUTPUT_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            if !generated.is_empty() {
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("{shape}.luau")),
                    &generated,
                )
                .unwrap();
            }
        }
        let measure = |source: &str| {
            let now = Instant::now();
            let script = Script::compile(
                source,
                ulua::CompileLimits::default(),
                functions::diagnostic(shape).unwrap_or(""),
            );
            let elapsed = now.elapsed();
            drop(script);
            elapsed
        };
        let mut times = [&small, &large].map(|source| measure(source));
        let repeat = times[1].as_secs_f64() / times[0].as_secs_f64() >= 6.5;
        if repeat {
            for (time, source) in times.iter_mut().zip([&small, &large]) {
                *time = (*time).min(measure(source));
            }
        }
        let ratio = times[1].as_secs_f64() / times[0].as_secs_f64();
        eprintln!(
            "S4 growth {shape}: n={n} {:?}; 4n={} {:?}; ratio={ratio:.3} runs={}",
            times[0],
            4 * n,
            times[1],
            if repeat { 2 } else { 1 }
        );
        max_growth = max_growth.max(ratio);
    }
    eprintln!("S4 growth gate cost: {:?}", start.elapsed());
    assert!(max_growth <= 8.0, "growth ratio {max_growth}");
    assert!(
        1.25 * max_output <= OUTPUT_FACTOR as f64,
        "output margin: ratio {max_output}"
    );
}
