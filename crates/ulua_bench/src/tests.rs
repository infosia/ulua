//! Cheap gate checks. The benchmark suite is never a cargo test.
use super::*;
#[cfg(feature = "remainder-shadow")]
use crate::measurement::Workload;
use std::time::Instant;
#[path = "../../ulua/tests/support/tsc.rs"]
mod tsc;

// Cost: 0.225657 s, Debug, measured locally; ten ABI compiles and one tsc process, no execution.
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

// Cost: 0.000460 s, Debug, measured locally; temporary record files only, no benchmark execution.
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

// Cost: 0.120270 s, Debug, measured locally; a temporary Git index and synthetic records, no commits or workload execution.
#[test]
fn changed_crates_prevent_full_run_records() {
    let start = Instant::now();
    let directory = std::env::temp_dir().join(format!("ulua-bench-dirty-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&directory)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet", "--template="]);
    std::fs::write(
        directory.join("README.md"),
        "<!-- results:start --><!-- results:end -->",
    )
    .unwrap();
    // Untracked files outside crates/ do not block a record.
    assert!(report::publish(&directory, None, "committed-code record", "table").unwrap());
    let old_readme = std::fs::read(directory.join("README.md")).unwrap();
    let old_record = std::fs::read(directory.join("results.json")).unwrap();
    std::fs::create_dir_all(directory.join("crates/fixture")).unwrap();
    let source = directory.join("crates/fixture/workload.ts");
    std::fs::write(&source, "untracked source").unwrap();
    let mode = Mode::parse(&[]).unwrap();
    assert_eq!(mode, Mode::Full);
    let reject = || {
        let error = report::publish(
            &directory,
            mode.only(),
            "changed-code record",
            "changed table",
        )
        .unwrap_err();
        assert!(error.contains("no file is written"));
        assert_eq!(
            std::fs::read(directory.join("README.md")).unwrap(),
            old_readme
        );
        assert_eq!(
            std::fs::read(directory.join("results.json")).unwrap(),
            old_record
        );
        // The root pathspec also works from the runner's nested crate directory.
        assert!(report::require_committed_crates(&directory.join("crates/fixture")).is_err());
    };
    reject();
    git(&["add", "--", "crates/fixture/workload.ts"]);
    reject();
    std::fs::write(&source, "unstaged source").unwrap();
    reject();
    // Move the indexed source outside crates/. Its deletion still blocks publication.
    std::fs::rename(&source, directory.join("moved.ts")).unwrap();
    reject();
    git(&[
        "rm",
        "--cached",
        "--force",
        "--",
        "crates/fixture/workload.ts",
    ]);
    assert!(report::publish(&directory, None, "outside-crates changes", "table").unwrap());
    std::fs::remove_dir_all(&directory).unwrap();
    eprintln!(
        "changed_crates_prevent_full_run_records cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

// Cost: 0.002310 s, Debug, measured locally; ten Luau module compiles, no workload execution.
#[test]
fn all_luau_workloads_compile() {
    let start = Instant::now();
    for id in IDS {
        let source = std::fs::read(
            directory()
                .join("workloads/luau")
                .join(format!("{id}.luau")),
        )
        .unwrap();
        let _vm = subjects::Luau::new(&source).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    eprintln!(
        "all_luau_workloads_compile cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

// Cost: 5.521 s, Debug; one Ulua and one reference run per workload.
#[test]
#[cfg(feature = "remainder-shadow")]
fn shadow_ten_workloads() {
    assert!(std::env::var_os("ULUA_REMAINDER_SHADOW").is_some());
    let start = Instant::now();
    for id in IDS {
        let directory = directory().join("workloads");
        let source = std::fs::read(directory.join("ulua").join(format!("{id}.ts"))).unwrap();
        let reference = std::fs::read(directory.join("luau").join(format!("{id}.luau"))).unwrap();
        let mut ulua = subjects::Ulua::new(&source).unwrap();
        let mut luau = subjects::Luau::new(&reference).unwrap();
        ulua.prepare();
        luau.prepare();
        let status = ulua.call();
        let result = ulua.result(status);
        let runtime_stops = usize::from(result.is_err());
        eprintln!("shadow workload={id} result={result:?} runtime_stops={runtime_stops}");
        let actual = result.unwrap_or_else(|e| panic!("{id}: {e}"));
        let status = luau.call();
        let expected = luau.result(status).unwrap();
        assert_eq!(actual, expected, "{id}");
    }
    eprintln!(
        "shadow workloads=10 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
