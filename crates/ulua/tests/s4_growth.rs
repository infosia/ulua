//! Growth gate: full ABI timing at n and 4n; output checks always use the source ceiling.
//! Dense fixtures cover every P1, P2.1, and P2.2 emitted construct.
//! Ratios above 6.5 use the minimum of two runs per size.
//! Measured Debug cost: 8.56 seconds, with serial ABI timing and two joined size-check workers.
// This test uses the source and graph generators only.
#[allow(dead_code)]
#[path = "../src/p22_inputs.rs"]
mod arrays;
#[path = "../src/dense_inputs.rs"]
mod dense;
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
    let n = 2 * 1024;
    let mut max_growth: f64 = 0.0;
    let mut max_output: f64 = 0.0;
    let mut outputs = vec![];
    for shape in inputs::SOURCE_SHAPES
        .iter()
        .chain(review::SHAPES)
        .chain(joins::SHAPES)
        .chain(functions::SHAPES)
        .chain(arrays::SHAPES)
    {
        let make = |bytes| {
            if arrays::SHAPES.contains(shape) {
                arrays::source(shape, bytes)
            } else if functions::SHAPES.contains(shape) {
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
        let n = if shape.starts_with("type-product-") || *shape == "array-wide-function-product" {
            16 * 1024
        } else if shape.starts_with("spilled-mod-") {
            8 * 1024
        } else if *shape == "spilled-and" {
            4 * 1024
        } else {
            n
        };
        let small = make(n);
        let large = make(4 * n);
        assert_eq!(small.len(), n);
        assert_eq!(large.len(), 4 * n);
        // Dense fixtures replace sparse output samples. Keep independent type, join, and spill stress cases.
        if shape.starts_with("type-product-")
            || shape.starts_with("join-")
            || shape.starts_with("alias-chain")
            || shape.starts_with("spilled-")
            || matches!(
                *shape,
                "function-bindings"
                    | "module-bindings"
                    | "short-functions"
                    | "nested-bindings"
                    | "nested-writes"
                    | "array-wide-function-product"
                    | "array-function-product"
            )
        {
            outputs.push((*shape, make(65536)));
        }
        let measure = |source: &str| {
            let now = Instant::now();
            let script = Script::compile(
                source,
                ulua::CompileLimits::default(),
                functions::diagnostic(shape)
                    .or_else(|| arrays::diagnostic(shape))
                    .unwrap_or(""),
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
    outputs.extend(dense::SHAPES.iter().map(|s| (*s, dense::source(s, 65536))));
    let directory = std::env::var("ULUA_GROWTH_OUTPUT_DIR").ok();
    if let Some(directory) = &directory {
        std::fs::create_dir_all(directory).unwrap();
    }
    // Size checks use two joined workers. ABI timing above stays serial.
    let mut workers = vec![];
    let mut partitions = [vec![], vec![]];
    for (i, output) in outputs.into_iter().enumerate() {
        partitions[i % 2].push((i, output));
    }
    for partition in partitions {
        let directory = directory.clone();
        workers.push(
            std::thread::Builder::new()
                .stack_size(COMPILER_STACK_BYTES)
                .spawn(move || {
                    let mut results = vec![];
                    for (index, (shape, source)) in partition {
                        assert_eq!(source.len(), 65536);
                        let generated =
                            ulua_compiler::compile(source.as_bytes(), Limits::default())
                                .unwrap_or_else(|e| panic!("output {shape}: {e:?}"))
                                .luau;
                        let ratio =
                            generated.len().saturating_sub(OUTPUT_PRELUDE_BYTES) as f64 / 65536.0;
                        results.push((shape, generated.len(), ratio));
                        if let Some(directory) = &directory {
                            std::fs::write(
                                std::path::Path::new(directory)
                                    .join(format!("{index:03}-{shape}.luau")),
                                generated,
                            )
                            .unwrap();
                        }
                    }
                    results
                })
                .unwrap(),
        );
    }
    for worker in workers {
        for (shape, bytes, ratio) in worker.join().unwrap() {
            eprintln!("S6 output {shape}: source=65536 generated={bytes} ratio={ratio:.3}");
            max_output = max_output.max(ratio);
        }
    }
    eprintln!("S4 growth gate cost: {:?}", start.elapsed());
    assert!(max_growth <= 8.0, "growth ratio {max_growth}");
    assert!(
        1.25 * max_output <= OUTPUT_FACTOR as f64,
        "output margin: ratio {max_output}"
    );
}
