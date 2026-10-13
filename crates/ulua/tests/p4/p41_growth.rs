//! Await shapes compile and run at 4 KiB, 16 KiB, and the source ceiling.
//! Costs are reported for twenty-eight shapes, including nine P4.2 and six P4.3 shapes.
#[path = "../support/requests.rs"]
mod requests;
#[path = "../support/p41_shapes.rs"]
mod shapes;
#[allow(dead_code)]
#[path = "../support/script.rs"]
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
            let compiled = ulua_compiler::compile_with_grants(
                source.as_bytes(),
                ulua_compiler::Limits::default(),
                &shapes::grants(&source),
            )
            .unwrap();
            if shape.starts_with("host-") {
                let mut limits = RunLimits::default();
                limits.max_open_requests = 65536;
                limits.max_memory_bytes = 128 * 1024 * 1024;
                limits.max_steps = 10_000_000;
                let run = requests::run(&source, limits);
                assert_eq!(run.0, ULUA_OK, "{shape} bytes={bytes}: {}", run.1);
                numbers.push((compiled.luau.len(), now.elapsed().as_secs_f64()));
                continue;
            }
            let mut script = support::Script::new(&source);
            let mut limits = RunLimits::default();
            if shapes::SHAPES[..9].contains(shape) {
                limits.max_memory_bytes = 128 * 1024 * 1024;
                limits.max_steps = 10_000_000;
            }
            assert_eq!(script.instance(limits).0, ULUA_OK);
            let run = script.call(script.index("main"), &[]);
            assert_eq!(run.0, ULUA_OK, "{shape} bytes={bytes}: {run:?}");
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

#[test]
fn derived_drains_obey_steps_and_memory_limits() {
    let start = Instant::now();
    let source = "async function v():Promise<number>{return 1;}function plus(n:number):number{return n+1;}export async function chain(n:number):Promise<number>{let p=v();for(let i=0;i<n;i++){p=p.then(plus);}return p;}export async function all(n:number):Promise<number>{const ps:Promise<number>[]=[];for(let i=0;i<n;i++){ps.push(v());}return (await Promise.all(ps)).length;}export function good():number{return 42;}export function main():void{}";
    let mut script = support::Script::new(source);
    for kind in ["chain", "all"] {
        let mut limits = RunLimits::default();
        limits.max_memory_bytes = 16 * 1024 * 1024;
        limits.max_steps = 10_000_000;
        assert_eq!(script.instance(limits).0, ULUA_OK);
        let success = script.call(script.index(kind), &[support::number(5000)]);
        assert_eq!(success.0, ULUA_OK, "{kind}: {success:?}");
        assert_eq!(
            success.2.number,
            if kind == "chain" { 5001.0 } else { 5000.0 }
        );
        let stopped = script.call(script.index(kind), &[support::number(100000)]);
        assert_eq!(stopped.1, "limit:memory", "{kind}: {stopped:?}");
        assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
        limits.max_steps = 20_000;
        assert_eq!(script.instance(limits).0, ULUA_OK);
        assert_eq!(
            script.call(script.index(kind), &[support::number(5000)]).1,
            "limit:steps"
        );
        assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
    }
    eprintln!(
        "P4.2 drain limits forms=2 success_n=5000 memory_n=100000 memory_bytes=16777216 steps=20000 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
