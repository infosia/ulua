//! A child process checks the analyzer worker option and both process failure reports.
//! Single-worker Debug cost: 0.24 seconds for this test target.
//! The test prints its cost without a wall-clock assertion.
#[cfg(unix)]
#[path = "../support/analyzer.rs"]
mod analyzer;

#[cfg(unix)]
#[test]
fn analyzer_failures_report_exit_codes_and_signals() {
    use std::{
        os::unix::{fs::PermissionsExt, process::ExitStatusExt},
        time::Instant,
    };
    let start = Instant::now();
    let directory =
        std::env::temp_dir().join(format!("ulua-analyzer-status-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("analyzer");
    std::fs::write(&executable, "#!/bin/sh\n[ \"$1\" = \"-j1\" ] || exit 97\nif [ \"$2\" = \"exit\" ]; then exit 3; fi\nkill -TERM $$\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    for (mode, code, signal) in [("exit", Some(3), None), ("signal", None, Some(15))] {
        let output = analyzer::command(&executable).arg(mode).output().unwrap();
        assert_eq!(output.status.code(), code);
        assert_eq!(output.status.signal(), signal);
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
        let report = analyzer::failure(&output);
        assert!(
            report.contains(&format!("status={}", output.status)),
            "{report}"
        );
        assert!(report.contains(&format!("code={code:?}")), "{report}");
        assert!(report.contains(&format!("signal={signal:?}")), "{report}");
    }
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!("Analyzer status cost={:.3}s", start.elapsed().as_secs_f64());
}
