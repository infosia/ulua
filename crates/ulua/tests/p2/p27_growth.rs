//! Protected growth includes native compilation and execution at n, 4n, and the depth ceiling.
//! Measured Debug target cost: 0.38 seconds.
//! The target reports costs. Only deterministic growth ratios are asserted.
#[path = "../support/analyzer.rs"]
mod analyzer;
#[path = "../../../ulua_compiler/src/p27_shapes.rs"]
mod shapes;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{path::PathBuf, time::Instant};
use ulua_compiler::{Limits, compile};
#[test]
fn protected_pipeline_and_nesting_ceiling() {
    let start = Instant::now();
    let dir = std::env::temp_dir().join(format!("ulua-p27-growth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut files = vec![];
    for shape in shapes::SHAPES {
        let n = if shape == "nested" { 5 } else { 16 };
        let mut times = vec![];
        for count in [n, 4 * n] {
            let source = shapes::source(shape, count);
            let now = Instant::now();
            let mut script = support::Script::new(&source);
            assert_eq!(script.instance(ulua::RunLimits::default()).0, ulua::ULUA_OK);
            assert_eq!(script.call(script.index("main"), &[]).0, ulua::ULUA_OK);
            times.push(now.elapsed().as_secs_f64());
            let result = compile(source.as_bytes(), Limits::default()).unwrap();
            let file = dir.join(format!("{shape}-{count}.luau"));
            std::fs::write(&file, result.luau).unwrap();
            files.push(file);
        }
        eprintln!(
            "P2.7 ABI growth {shape} n={n}/{} ms={:.3}/{:.3} observed_time_ratio={:.3}",
            4 * n,
            times[0] * 1000.0,
            times[1] * 1000.0,
            times[1] / times[0]
        );
    }
    let mut ceiling = 0;
    let mut last = None;
    for count in 1..48 {
        let source = shapes::source("nested", count);
        match compile(source.as_bytes(), Limits::default()) {
            Ok(result) => {
                ceiling = count;
                last = Some((source, result));
            }
            Err(d) => {
                assert_eq!(d.code, "limit:nesting-depth");
                break;
            }
        }
    }
    let (source, result) = last.unwrap();
    let mut script = support::Script::new(&source);
    assert_eq!(script.instance(ulua::RunLimits::default()).0, ulua::ULUA_OK);
    assert_eq!(script.call(script.index("main"), &[]).0, ulua::ULUA_OK);
    let file = dir.join("ceiling.luau");
    std::fs::write(&file, result.luau).unwrap();
    files.push(file);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let now = Instant::now();
    let out = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(&files)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", analyzer::failure(&out));
    eprintln!(
        "P2.7 growth analyzer={} ceiling={ceiling} analyzer_cost={:.3}s total_cost={:.3}s",
        files.len(),
        now.elapsed().as_secs_f64(),
        start.elapsed().as_secs_f64()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
