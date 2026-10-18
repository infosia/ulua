//! Callback growth uses the existing compiler and analyzer ratio bounds.
//! Measured Debug cost: 8.14 seconds for both tests with one analyzer worker.
#[path = "../support/analyzer.rs"]
mod analyzer;
#[allow(dead_code)]
#[path = "../../src/dense_inputs.rs"]
mod dense;
#[path = "../support/callback_growth.rs"]
mod shape;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{path::Path, time::Instant};

#[test]
fn inline_callbacks_keep_bounded_compiler_growth() {
    let _measurement = crate::measurement_lock();
    for key in shape::KEYS {
        let mut times = Vec::new();
        for bytes in [4096, 16384, 65536] {
            let source = shape::source(key, bytes);
            let compiled = ulua_compiler::compile(source.as_bytes(), Default::default()).unwrap();
            assert_eq!(
                compiled.luau.contains("__ulua_map_1("),
                *key == "block-callbacks"
            );
            let mut minimum = f64::INFINITY;
            for _ in 0..2 {
                let now = Instant::now();
                let script = support::Script::new(&source);
                minimum = minimum.min(now.elapsed().as_secs_f64());
                drop(script);
            }
            eprintln!("P6 compiler {key} bytes={bytes} seconds={minimum:.6}");
            times.push(minimum);
        }
        for pair in times.windows(2) {
            assert!(
                pair[1] / pair[0] <= 8.0,
                "{key} compiler ratio={:.3}",
                pair[1] / pair[0]
            );
        }
        eprintln!(
            "P6 compiler {key} ratios={:.3}/{:.3}",
            times[1] / times[0],
            times[2] / times[1]
        );
    }
}

fn analyze(directory: &Path, key: &str, source: &str) -> f64 {
    let compiled = ulua_compiler::compile(source.as_bytes(), Default::default()).unwrap();
    let path = directory.join(format!("{key}-{}.luau", source.len()));
    std::fs::write(&path, compiled.luau).unwrap();
    let executable =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/luau-tools/luau-analyze");
    let mut minimum = f64::INFINITY;
    for _ in 0..2 {
        let now = Instant::now();
        let output = analyzer::command(&executable).arg(&path).output().unwrap();
        minimum = minimum.min(now.elapsed().as_secs_f64());
        assert!(
            output.status.success(),
            "{key}: {}",
            analyzer::failure(&output)
        );
    }
    minimum
}

#[test]
fn inline_callbacks_keep_bounded_analyzer_growth() {
    let _measurement = crate::measurement_lock();
    let directory =
        std::env::temp_dir().join(format!("ulua-callback-growth-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let reference = analyze(&directory, "dense-assign", &dense::source("assign", 65536));
    eprintln!("P6 analyzer dense-assign bytes=65536 seconds={reference:.6}");
    for key in shape::KEYS {
        let mut times = Vec::new();
        for bytes in [4096, 16384, 65536] {
            let time = analyze(&directory, key, &shape::source(key, bytes));
            eprintln!("P6 analyzer {key} bytes={bytes} seconds={time:.6}");
            times.push(time);
        }
        let growth = times[1] / times[0];
        let ceiling = times[2] / reference;
        eprintln!(
            "P6 analyzer {key} ratios={growth:.3}/{:.3} ceiling_ratio={ceiling:.3}",
            times[2] / times[1]
        );
        assert!(growth <= 12.0, "{key} analyzer ratio={growth:.3}");
        assert!(ceiling <= 50.0, "{key} ceiling analyzer ratio={ceiling:.3}");
    }
    std::fs::remove_dir_all(directory).unwrap();
}
