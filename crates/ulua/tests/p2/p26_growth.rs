//! Class growth runs the full ABI compiler at n and 4n bytes and analyzes ceiling outputs.
//! Single-worker Debug cost: 3.58 seconds for this test target.
//! Compiler unit tests assert deterministic state ratios. This test only reports elapsed cost.
#[path = "../support/analyzer.rs"]
mod analyzer;
#[path = "../../../ulua_compiler/src/p26_shapes.rs"]
mod shapes;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{path::PathBuf, time::Instant};
use ulua_compiler::{Limits, OUTPUT_FACTOR, OUTPUT_PRELUDE_BYTES, compile};

#[test]
fn class_full_pipeline_growth() {
    let start = Instant::now();
    let dir = std::env::temp_dir().join(format!("ulua-P2.6-growth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut files = vec![];
    for shape in shapes::SHAPES {
        let mut times = vec![];
        for bytes in [8192, 32768] {
            let source = shapes::source(shape, bytes);
            let now = Instant::now();
            let script = support::Script::new(&source);
            times.push(now.elapsed().as_secs_f64());
            drop(script);
        }
        eprintln!(
            "P2.6 ABI growth {shape} bytes=8192/32768 ms={:.3}/{:.3} observed_time_ratio={:.3}",
            times[0] * 1000.0,
            times[1] * 1000.0,
            times[1] / times[0]
        );
        for bytes in [8192, 32768, 65536] {
            let source = shapes::source(shape, bytes);
            let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
            assert!(compiled.luau.len() <= OUTPUT_FACTOR * bytes + OUTPUT_PRELUDE_BYTES);
            eprintln!(
                "P2.6 output {shape} bytes={bytes} output={} factor={:.3}",
                compiled.luau.len(),
                compiled.luau.len().saturating_sub(OUTPUT_PRELUDE_BYTES) as f64 / bytes as f64
            );
            let path = dir.join(format!("{shape}-{bytes}.luau"));
            std::fs::write(&path, compiled.luau).unwrap();
            files.push(path);
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let now = Instant::now();
    let out = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(&files)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "luau-analyze {}",
        analyzer::failure(&out)
    );
    eprintln!(
        "P2.6 growth analyzer={} cost={:.3}s total_cost={:.3}s",
        files.len(),
        now.elapsed().as_secs_f64(),
        start.elapsed().as_secs_f64()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
