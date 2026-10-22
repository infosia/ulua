//! Product compiler/VM measurements, using p55_ratio's exact baseline source.
//! The caller supplies the bound: the shared section 5.19 bound for the timing
//! gate, or the separate target 5 for explicit charge measurements.
#[allow(dead_code)]
#[path = "script.rs"]
mod support;
use std::time::Instant;
use ulua::{RunLimits, ULUA_FAILED};
const BASELINE: &str = "function start():number{let i=0;while(true){i++;}return 0;}const stop=start();export function main():void{}";
fn measure_source(source: &str) -> f64 {
    let mut script = support::Script::new(source);
    let mut samples = vec![];
    for _ in 0..5 {
        let start = Instant::now();
        assert_eq!(
            script.instance(RunLimits::default()),
            (ULUA_FAILED, "limit:steps".into())
        );
        samples.push(start.elapsed().as_secs_f64());
    }
    samples.sort_by(f64::total_cmp);
    samples[2]
}
fn measure(body: &str) -> f64 {
    measure_source(&format!(
        "function start():number{{{body}return 0;}}const stop=start();export function main():void{{}}"
    ))
}
fn baseline() -> f64 {
    // p55_ratio warms the VM before its baseline. Repeat the same product loop
    // here and retain the fastest median, so cold-start variance cannot hide
    // an undercharge (the initial fixed-charge calibration did exactly that).
    (0..3)
        .map(|_| measure_source(BASELINE))
        .fold(f64::INFINITY, f64::min)
}
pub fn parse_verify(bound: f64) {
    let baseline = baseline();
    for (name, s) in [
        ("space", format!("{}1", " ".repeat(30000))),
        ("digits", "1".repeat(30000)),
        ("leading-zero", format!("{}1", "0".repeat(30000))),
        ("unicode", format!("{}1", "\u{feff}".repeat(10000))),
    ] {
        for call in ["parseInt(s,10)", "parseInt(s,3)", "parseFloat(s)"] {
            let elapsed = measure(&format!("const s='{s}';while(true){{const x={call};}}"));
            let ratio = elapsed / baseline;
            eprintln!(
                "Number bytes=1 {name} {call}: baseline={baseline:.6}s time={elapsed:.6}s ratio={ratio:.3}"
            );
            assert!(ratio <= bound, "{name} {call}: {ratio}");
        }
    }
}
const CLASSES: &[(&str, &[&str])] = &[
    (
        "subnormal",
        &[
            "5e-324",
            "-5e-324",
            "2.225073858507201e-308",
            "2.2250738585072014e-308",
        ],
    ),
    (
        "near-one",
        &[
            "0.9999999999999999",
            "1",
            "1.0000000000000002",
            "0.3333333333333333",
        ],
    ),
    (
        "near-2^53",
        &[
            "9007199254740991",
            "9007199254740992",
            "9007199254740994",
            "-9007199254740991",
        ],
    ),
    (
        "near-max",
        &["1.7976931348623157e308", "1e308", "-1.7976931348623157e308"],
    ),
];
/// All sweep shapes are also gate shapes, including each measured worst case.
pub fn format_verify(bound: f64) {
    let baseline = baseline();
    let mut largest = (0.0, String::new());
    let mut failures = vec![];
    for &(class, values) in CLASSES {
        for member in ["toFixed", "toExponential", "toPrecision", "toString"] {
            let args: &[&str] = match member {
                "toString" => &["2", "3", "10", "36"],
                // toPrecision(0) traps and is tested by number_trap_edges.
                "toPrecision" => &["1", "20", "100"],
                "toExponential" => &["0", "20", "100", ""],
                "toFixed" => &["0", "20", "100", ""],
                _ => unreachable!(),
            };
            for arg in args {
                let mut worst = (0.0, String::new());
                for value in values {
                    let call = format!("({value}).{member}({arg})");
                    let elapsed = measure(&format!("while(true){{const x={call};}}"));
                    let ratio = elapsed / baseline;
                    eprintln!(
                        "Number sweep {class} {member} arg={arg:?} value={value} baseline={baseline:.6}s time={elapsed:.6}s ratio={ratio:.3}"
                    );
                    if ratio > worst.0 {
                        worst = (ratio, call.clone());
                    }
                    if ratio > largest.0 {
                        largest = (ratio, call.clone());
                    }
                    if ratio > bound {
                        failures.push((ratio, call));
                    }
                }
                eprintln!(
                    "Number sweep-max {class} {member} arg={arg:?}: ratio={:.3} shape={}",
                    worst.0, worst.1
                );
            }
        }
    }
    eprintln!(
        "Number format sweep largest={:.3} shape={} formula=32+3*output_bytes+ceil(abs(binary_exponent)/2)",
        largest.0, largest.1
    );
    assert!(
        failures.is_empty(),
        "Number format ratios exceed {bound}: {failures:?}"
    );
}
