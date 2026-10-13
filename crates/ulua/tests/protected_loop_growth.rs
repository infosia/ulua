//! Each protected loop form compiles through its exact nesting ceiling.
//! ABI and analyzer checks use the ceiling; separate witnesses measure dense repetitions.
//! Each test reports its measured cost.
#[path = "support/analyzer.rs"]
mod analyzer;
#[allow(dead_code)]
#[path = "support/protected_loops.rs"]
mod shapes;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::{path::PathBuf, time::Instant};
use ulua_compiler::{Limits, compile};

#[test]
fn protected_loops_reach_each_nesting_ceiling() {
    let start = Instant::now();
    let directory =
        std::env::temp_dir().join(format!("ulua-protected-loops-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = vec![];
    let mut count = 0;
    for asynchronous in [false, true] {
        for kind in ["finally", "catch", "nested"] {
            for edge in ["break", "none", "continue", "inner-break", "inner-continue"] {
                let ceiling = shapes::ceiling(asynchronous, kind, edge);
                for depth in usize::from(edge.starts_with("inner-"))..=ceiling {
                    let source = shapes::at_depth(asynchronous, kind, edge, depth, 0);
                    let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
                    assert!(
                        compiled.luau.len()
                            <= ulua_compiler::OUTPUT_PRELUDE_BYTES
                                + source.len() * ulua_compiler::OUTPUT_FACTOR
                    );
                    count += 1;
                    if depth == ceiling {
                        let mut script = support::Script::new(&source);
                        assert_eq!(script.instance(ulua::RunLimits::default()).0, ulua::ULUA_OK);
                        assert_eq!(script.call(script.index("main"), &[]).0, ulua::ULUA_OK);
                        let path = directory.join(format!("{asynchronous}-{kind}-{edge}.luau"));
                        std::fs::write(&path, compiled.luau).unwrap();
                        paths.push(path);
                    }
                }
                let source = shapes::at_depth(asynchronous, kind, edge, ceiling + 1, 0);
                assert_eq!(
                    compile(source.as_bytes(), Limits::default())
                        .unwrap_err()
                        .code,
                    "limit:nesting-depth"
                );
                eprintln!(
                    "protected loops async={asynchronous} kind={kind} edge={edge} ceiling={ceiling}"
                );
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = analyzer::command(root.join("target/luau-tools/luau-analyze"))
        .args(&paths)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", analyzer::failure(&output));
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "protected loop ceilings compiled={count} ABI/analyzer={} cost={:.3}s",
        paths.len(),
        start.elapsed().as_secs_f64()
    );
}
