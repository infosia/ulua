//! Await shapes compile and run at 4 KiB, 16 KiB, and the source ceiling.
//! The witness-time target asserts analyzer ratios. This target reports ABI costs and output growth.
#[path = "support/p41_shapes.rs"]
mod shapes;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::time::Instant;
use ulua::*;
#[test]
fn await_pipeline_growth_and_drain_limits() {
    let start = Instant::now();
    for shape in shapes::SHAPES {
        let mut numbers = vec![];
        for bytes in [4096, 16384, 65536] {
            let source = shapes::source(shape, bytes);
            assert_eq!(source.len(), bytes);
            let now = Instant::now();
            let compiled =
                ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
                    .unwrap();
            let mut script = support::Script::new(&source);
            assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
            assert_eq!(script.call(script.index("main"), &[]).0, ULUA_OK);
            numbers.push((compiled.luau.len(), now.elapsed().as_secs_f64()));
        }
        eprintln!(
            "P4.1 growth {shape} source=4096/16384/65536 output={}/{}/{} ABI_ms={:.3}/{:.3}/{:.3}",
            numbers[0].0,
            numbers[1].0,
            numbers[2].0,
            numbers[0].1 * 1000.0,
            numbers[1].1 * 1000.0,
            numbers[2].1 * 1000.0
        );
    }
    let mut script = support::Script::new(
        "async function tick():Promise<number>{return 1;}export async function main():Promise<void>{const p=tick();for(let i=0;i<10000;i++){await p;}}export function good():number{return 42;}",
    );
    let mut limits = RunLimits::default();
    limits.max_steps = 10000;
    assert_eq!(script.instance(limits).0, ULUA_OK);
    assert_eq!(script.call(script.index("main"), &[]).1, "limit:steps");
    assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
    eprintln!(
        "P4.1 growth shapes={} drain_step_limit=10000 cost={:.3}s",
        shapes::SHAPES.len(),
        start.elapsed().as_secs_f64()
    );
}
