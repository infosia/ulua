//! A Release micro loop measures native Math calls against a builtin alias.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::time::Instant;
use ulua::{RunLimits, ULUA_OK};

#[test]
#[ignore = "run once in Release to record native call cost"]
fn native_math_call_cost() {
    assert!(
        std::hint::black_box(!cfg!(debug_assertions)),
        "run with --release"
    );
    let _measurement = crate::measurement_lock();
    let mut times = vec![];
    const DRAWS: usize = 200_000;
    for member in ["floor", "sin", "random"] {
        let expression = if member == "random" {
            "Math.random()".to_owned()
        } else {
            format!("Math.{member}(i*0.00001)")
        };
        let source = format!(
            "export function main():number{{let sum=0;for(let i=0;i<{DRAWS};i++){{sum=sum+{expression};}}return sum;}}"
        );
        let mut script = support::Script::new(&source);
        let mut limits = RunLimits::default();
        limits.max_steps = 100_000_000;
        assert_eq!(script.instance(limits).0, ULUA_OK);
        let index = script.index("main");
        assert_eq!(script.call(index, &[]).0, ULUA_OK);
        let mut samples = vec![];
        for _ in 0..5 {
            let now = Instant::now();
            let (status, _, result) = script.call(index, &[]);
            assert_eq!(status, ULUA_OK);
            std::hint::black_box(result.number);
            samples.push(now.elapsed().as_secs_f64() * 1e9 / DRAWS as f64);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "Math.{member}: {:.3} ns/call, median of 5 x {DRAWS}, loop included",
            samples[2]
        );
        times.push(samples[2]);
    }
    eprintln!(
        "native sin minus builtin floor: {:.3} ns/call; random minus floor: {:.3} ns/call",
        times[1] - times[0],
        times[2] - times[0]
    );
}
