//! Analyzer growth covers every shape of s4_growth and p2x_growth.
//! The default gate measures 4/16 KiB; the measurements feature reports 16/64 KiB.
//! Each size uses the minimum of two single-worker analyzer runs. Only ratios are asserted.
//! Measured Debug default cost: 69.593 seconds for 167 measured shapes and one C11 fixture.
//! Measured Debug full cost: 494.678 seconds; the prior full probe costs 627.296 seconds.
//! The full run reports eleven ratios above twelve; their forms need further lowering or contract bounds.
//! Stock tsc resource exceptions include TS2321 and TS2563 (CLAUDE.md invariant 3).
#[path = "support/analyzer.rs"]
mod analyzer;
#[path = "support/growth_shapes.rs"]
mod shapes;
use std::{path::PathBuf, process::Command, time::Instant};
use ulua_compiler::{Limits, compile};

#[test]
fn all_growth_shapes_have_bounded_analyzer_time() {
    let largest = measure("default", [4096, 16384]);
    assert!(largest <= 12.0, "Analyzer ratio: {largest:.3}");
}

#[cfg(feature = "measurements")]
#[test]
fn all_growth_shapes_report_full_analyzer_time() {
    measure("full", [16384, 65536]);
}

fn measure(label: &str, sizes: [usize; 2]) -> f64 {
    let start = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = std::env::temp_dir().join(format!(
        "ulua-growth-witness-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    let mut largest = 0.0f64;
    let mut measured = 0;
    let mut bounded = 0;
    let mut failures = vec![];
    for shape in shapes::all() {
        let mut times = vec![];
        for bytes in sizes {
            let source = (shape.make)(shape.key, bytes);
            assert_eq!(source.len(), bytes, "{}", shape.label);
            let compiled = match compile(source.as_bytes(), Limits::default()) {
                Ok(compiled) => {
                    assert!(shape.diagnostic.is_none(), "{}", shape.label);
                    compiled
                }
                Err(diagnostic) => {
                    assert!(
                        Some(diagnostic.code) == shape.diagnostic
                            || diagnostic.code.starts_with("limit:"),
                        "{}: {diagnostic:?}",
                        shape.label
                    );
                    eprintln!(
                        "analyzer {label} {}: bytes={bytes} bound={} offset={}",
                        shape.label, diagnostic.code, diagnostic.offset
                    );
                    continue;
                }
            };
            let path = directory.join(format!("{}-{bytes}.ts", shape.label));
            std::fs::write(&path, source).unwrap();
            std::fs::write(path.with_extension("luau"), compiled.luau).unwrap();
            let mut minimum = f64::INFINITY;
            let mut valid = true;
            for _ in 0..2 {
                let now = Instant::now();
                let output = analyzer::command(root.join("target/luau-tools/luau-analyze"))
                    .arg(path.with_extension("luau"))
                    .output()
                    .unwrap();
                minimum = minimum.min(now.elapsed().as_secs_f64());
                if !output.status.success() {
                    let failure = format!(
                        "{} bytes={bytes}: {}",
                        shape.label,
                        analyzer::failure(&output)
                    );
                    assert_eq!(label, "full", "{failure}");
                    eprintln!("analyzer {label} FAILURE: {failure}");
                    failures.push(failure);
                    valid = false;
                    break;
                }
            }
            eprintln!(
                "analyzer {label} {}: bytes={bytes} seconds={minimum:.6}",
                shape.label
            );
            if valid {
                times.push(minimum);
            }
            // Keep the existing tsc witnesses; the ABI growth gates cover the other generators.
            if shape.label.starts_with("P27-")
                || matches!(shape.key, "compound-five" | "store-thirty")
            {
                paths.push(path);
            }
        }
        if times.len() == 2 {
            let ratio = times[1] / times[0];
            largest = largest.max(ratio);
            measured += 1;
            eprintln!(
                "analyzer {label} {}: ratio={ratio:.3}{}",
                shape.label,
                if ratio > 12.0 { " EXCEEDS_12" } else { "" }
            );
        } else {
            bounded += 1;
        }
    }
    // Separate modules permit reused fixture names. Batches keep tsc resource costs bounded.
    for batch in paths.chunks(32) {
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
            .args(batch)
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // Invariant 3 excludes the stock tsc resource limits TS2321 and TS2563.
        assert!(
            output.status.success()
                || (!text.is_empty()
                    && text.lines().all(
                        |line| line.contains("error TS2321:") || line.contains("error TS2563:")
                    )),
            "{text}"
        );
        eprintln!("analyzer {label} tsc: {text}");
    }
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "analyzer {label} measured={measured} bounded={bounded} largest={largest:.3} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(
        failures.is_empty(),
        "{} analyzer failures: {}",
        failures.len(),
        failures.join("\n")
    );
    largest
}
