//! The full ABI measures fact and integral mark shapes at 4, 16, and 64 KiB.
#[path = "../support/remainder_growth.rs"]
mod shape;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::time::Instant;
#[test]
fn eight_fact_rounds_keep_linear_pipeline_growth() {
    measure("fact-rounds");
}
#[test]
fn eight_element_fact_rounds_keep_linear_pipeline_growth() {
    measure("element-fact-rounds");
}
#[test]
fn direct_call_chain_keeps_linear_pipeline_growth() {
    measure("parameter-call-chain");
}
#[test]
fn many_direct_calls_keep_linear_pipeline_growth() {
    measure("parameter-many-calls");
}
#[test]
fn nested_direct_calls_keep_bounded_pipeline_growth() {
    measure("parameter-nested-calls");
}
#[test]
fn integral_marks_keep_linear_pipeline_growth() {
    measure("integral-marks");
}
fn measure(shape: &str) {
    assert!(shape::KEYS.contains(&shape));
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    let mut times = vec![];
    for bytes in [4096, 16384, 65536] {
        let source = shape::source(shape, bytes);
        let mut minimum = f64::INFINITY;
        for _ in 0..2 {
            let now = Instant::now();
            let script = support::Script::new(&source);
            minimum = minimum.min(now.elapsed().as_secs_f64());
            drop(script);
        }
        let compiled = ulua_compiler::compile(source.as_bytes(), Default::default()).unwrap();
        assert!(compiled.luau.contains(" % "));
        assert!(
            compiled.luau.len()
                <= ulua_compiler::OUTPUT_PRELUDE_BYTES + bytes * ulua_compiler::OUTPUT_FACTOR
        );
        eprintln!("P6 {shape} bytes={bytes} seconds={minimum:.6}");
        times.push(minimum);
    }
    for pair in times.windows(2) {
        assert!(
            pair[1] / pair[0] <= 8.0,
            "{shape} ratio={:.3}",
            pair[1] / pair[0]
        );
    }
    eprintln!(
        "P6 {shape} ratios={:.3}/{:.3} cost={:.3}s",
        times[1] / times[0],
        times[2] / times[1],
        start.elapsed().as_secs_f64()
    );
}
