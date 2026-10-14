//! Seeded native defects must terminate a separate process with a sanitizer report.
use std::{process::Command, time::Instant};

fn probe(kind: &str, expected: &str, source_line: &str) {
    let start = Instant::now();
    let Some(executable) = option_env!("ULUA_SANITIZE_PROBE") else {
        panic!("ULUA_SANITIZE=1 is required");
    };
    let output = Command::new(executable)
        .args([kind, "8"])
        .env(
            "ASAN_OPTIONS",
            "halt_on_error=1:abort_on_error=1:detect_leaks=0",
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "seeded {kind} defect survived");
    assert!(report.contains(expected), "{}\n{report}", output.status);
    let summary = report.lines().find(|line| line.starts_with("SUMMARY:"));
    let source_matches = report.contains(source_line);
    let executable_matches = !report.contains("sanitize.cpp")
        && summary.is_some_and(|line| line.contains("ulua-sanitize-probe"));
    assert!(source_matches || executable_matches, "{report}");
    eprintln!(
        "sanitizer probe {kind}: child={} cost={:.3}s",
        output.status,
        start.elapsed().as_secs_f64()
    );
}

#[test]
#[cfg_attr(not(ulua_sanitize), ignore = "requires ULUA_SANITIZE=1")]
fn heap_write_reports_asan() {
    probe(
        "heap",
        "AddressSanitizer: heap-buffer-overflow",
        "sanitize.cpp:12",
    );
}

#[test]
#[cfg_attr(not(ulua_sanitize), ignore = "requires ULUA_SANITIZE=1")]
fn signed_overflow_reports_ubsan() {
    probe(
        "overflow",
        "runtime error: signed integer overflow",
        "sanitize.cpp:16",
    );
}
