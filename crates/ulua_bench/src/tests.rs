//! Cheap gate checks. The benchmark suite is never a cargo test.
use super::*;
use std::time::Instant;
#[path = "../../ulua/tests/support/tsc.rs"]
mod tsc;

// Cost: 0.201170 s, Debug, measured locally; ten ABI compiles and one tsc process, no execution.
#[test]
fn all_ulua_workloads_compile_and_typecheck() {
    let start = Instant::now();
    let directory = directory();
    let root = directory.join("../..");
    let mut paths = Vec::new();
    for id in IDS {
        let path = directory.join("workloads/ulua").join(format!("{id}.ts"));
        let compile_start = Instant::now();
        let source = std::fs::read(&path).unwrap();
        let _module =
            subjects::Ulua::compile(&source).unwrap_or_else(|error| panic!("{id}: {error}"));
        eprintln!(
            "compile {id} cost={:.6}s",
            compile_start.elapsed().as_secs_f64()
        );
        paths.push(path);
    }
    let scratch = std::env::temp_dir().join(format!("ulua-bench-tsc-{}", std::process::id()));
    std::fs::create_dir(&scratch).unwrap();
    let tsc_start = Instant::now();
    // Reuse the corpus witness test support, including its prelude and strict ES2022 options.
    let output = tsc::command(&root, &scratch, &paths).output().unwrap();
    std::fs::remove_dir_all(scratch).unwrap();
    assert!(
        output.status.success(),
        "tsc: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "tsc ten workloads cost={:.6}s",
        tsc_start.elapsed().as_secs_f64()
    );
    eprintln!(
        "all_ulua_workloads_compile_and_typecheck cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

// Cost: 0.000657 s, Debug, measured locally; temporary record files only, no benchmark execution.
#[test]
fn only_prints_without_writes() {
    let start = Instant::now();
    let directory = std::env::temp_dir().join(format!("ulua-bench-only-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("results.json"), "original record").unwrap();
    std::fs::write(directory.join("README.md"), "original readme").unwrap();
    let args = vec!["--only".into(), "queen".into()];
    let mode = Mode::parse(&args).unwrap();
    assert_eq!(mode.only(), Some("queen"));
    assert!(!report::publish(&directory, mode.only(), "new record", "new table").unwrap());
    assert_eq!(
        std::fs::read_to_string(directory.join("results.json")).unwrap(),
        "original record"
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("README.md")).unwrap(),
        "original readme"
    );
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
    std::fs::remove_dir_all(&directory).unwrap();
    // An empty destination also stays absent. --only does not need a README or a directory.
    assert!(!report::publish(&directory, mode.only(), "record", "table").unwrap());
    assert!(!directory.exists());
    assert!(Mode::parse(&["--only".into(), "unknown".into()]).is_err());
    assert!(Mode::parse(&["--only".into()]).is_err());
    eprintln!(
        "only_prints_without_writes cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
