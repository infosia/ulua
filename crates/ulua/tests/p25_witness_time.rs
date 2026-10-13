//! C12.7 witness time at 8 KiB and 32 KiB. Each size uses the minimum of two analyzer runs.
//! The witness ratio bound is twelve; the quadratic checks measured ratios from 29 to 36.
//! Five-level compound: 0.108/0.629 seconds; ratio 5.798.
//! Thirty-level store: 0.162/0.984 seconds; ratio 6.087.
//! P2.7 checks five dense try shapes at 4 KiB and 16 KiB, with two analyzer runs per size.
//! The measurements feature adds a report at 16 KiB and 64 KiB without a ratio assertion.
//! The 21-level nested shape also tests the depth ceiling.
//! Measured Debug costs: P2.5 9.281 seconds; default P2.7 14.642 seconds; default target 14.64 seconds.
//! The separate default P2.7 probe costs 13.765 seconds; its largest ratio is 7.133.
//! The full P2.7 measurement costs 86.168 seconds with the measurements feature.
//! Default ratios, finally/catch/both/nested/loop: 6.568 / 6.939 / 6.460 / 5.565 / 5.541.
//! Full ratios, finally/catch/both/nested/loop: 10.901 / 11.763 / 10.209 / 6.433 / 9.298.
//! The four P2.5 inputs pass stock tsc without diagnostics; the P2.7 inputs reach only TS2563 resource limits.
//! Each access keeps its original trap point.
//! The helper isolates the nil branch from the caller's flow graph.
#[path = "support/analyzer.rs"]
mod analyzer;
use std::{path::PathBuf, process::Command, time::Instant};
use ulua_compiler::{Limits, compile};

fn source(levels: usize, bytes: usize, compound: bool) -> String {
    let arrays = "[]".repeat(levels);
    let accesses = "[0]".repeat(levels);
    let unit = if compound {
        format!("x{accesses}.f+=1;")
    } else {
        format!("x{accesses}.f=1;")
    };
    let mut source = format!("type B={{f:number}};function f(x:B{arrays}):void{{");
    let tail = "}export function main():void{}";
    while source.len() + unit.len() + tail.len() <= bytes {
        source.push_str(&unit);
    }
    source.push_str(tail);
    source.extend(std::iter::repeat_n(' ', bytes - source.len()));
    source
}

#[test]
fn missing_element_checks_have_linear_analyzer_time() {
    let start = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory =
        std::env::temp_dir().join(format!("ulua-p25-witness-time-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut largest = 0.0f64;
    let mut paths = vec![];
    for (name, levels, compound) in [("compound-five", 5, true), ("store-thirty", 30, false)] {
        let mut times = vec![];
        for bytes in [8192, 32768] {
            let source = source(levels, bytes, compound);
            let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
            let path = directory.join(format!("{name}-{bytes}.ts"));
            std::fs::write(&path, source).unwrap();
            std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
            let mut seconds = f64::INFINITY;
            for _ in 0..2 {
                let now = Instant::now();
                let output = analyzer::command(root.join("target/luau-tools/luau-analyze"))
                    .arg(path.with_extension("luau"))
                    .output()
                    .unwrap();
                seconds = seconds.min(now.elapsed().as_secs_f64());
                assert!(
                    output.status.success(),
                    "luau-analyze {}",
                    analyzer::failure(&output)
                );
            }
            eprintln!("P2.5 analyzer {name}: bytes={bytes} seconds={seconds:.3}");
            times.push(seconds);
            paths.push(path);
        }
        let ratio = times[1] / times[0];
        largest = largest.max(ratio);
        eprintln!("P2.5 analyzer {name}: ratio={ratio:.3}");
    }
    let output = Command::new("node")
        .current_dir(&root)
        .args([
            "node_modules/typescript/bin/tsc",
            "--noEmit",
            "--strict",
            "--target",
            "es2022",
            "--module",
            "esnext",
            "--lib",
            "es2022",
            "prelude/ulua.d.ts",
        ])
        .args(&paths)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success()
            || (!text.is_empty()
                && text
                    .lines()
                    .all(|line| line.contains("error TS2321:") || line.contains("error TS2563:"))),
        "{text}"
    );
    eprintln!("P2.5 analyzer witness tsc: {text}");
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "P2.5 analyzer witness cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(largest <= 12.0, "Analyzer ratio: {largest:.3}");
}

#[path = "support/p27_time.rs"]
mod p27;

#[test]
fn protected_closures_have_bounded_analyzer_growth() {
    // The measured 4/16 KiB ratios are below ten, with margin below the bound of twelve.
    let largest = measure_protected_closures("default", [4096, 16384]);
    assert!(largest <= 12.0, "Analyzer ratio: {largest:.3}");
}

#[cfg(feature = "measurements")]
#[test]
fn protected_closures_report_full_analyzer_growth() {
    measure_protected_closures("full", [16384, 65536]);
}

fn measure_protected_closures(label: &str, sizes: [usize; 2]) -> f64 {
    let start = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = std::env::temp_dir().join(format!(
        "ulua-p27-witness-time-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    let mut largest = 0.0f64;
    for shape in p27::SHAPES {
        let mut times = vec![];
        for bytes in sizes {
            let source = p27::source(shape, bytes);
            let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
            let path = directory.join(format!("{shape}-{bytes}.ts"));
            std::fs::write(&path, source).unwrap();
            std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
            let mut minimum = f64::INFINITY;
            for _ in 0..2 {
                let now = Instant::now();
                let output = analyzer::command(root.join("target/luau-tools/luau-analyze"))
                    .arg(path.with_extension("luau"))
                    .output()
                    .unwrap();
                minimum = minimum.min(now.elapsed().as_secs_f64());
                assert!(output.status.success(), "{}", analyzer::failure(&output));
            }
            eprintln!("P2.7 {label} analyzer {shape}: bytes={bytes} seconds={minimum:.6}");
            times.push(minimum);
            paths.push(path);
        }
        let ratio = times[1] / times[0];
        largest = largest.max(ratio);
        eprintln!("P2.7 {label} analyzer {shape}: ratio={ratio:.3}");
    }
    let output = Command::new("node")
        .current_dir(&root)
        .args([
            "node_modules/typescript/bin/tsc",
            "--noEmit",
            "--strict",
            "--target",
            "es2022",
            "--module",
            "esnext",
            "--lib",
            "es2022",
            "prelude/ulua.d.ts",
        ])
        .args(&paths)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Invariant 3 excludes these stock tsc resource limits.
    assert!(
        output.status.success()
            || (!text.is_empty()
                && text
                    .lines()
                    .all(|line| line.contains("error TS2321:") || line.contains("error TS2563:"))),
        "{text}"
    );
    eprintln!("P2.7 {label} analyzer witness tsc: {text}");
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "P2.7 {label} analyzer witness cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    largest
}
