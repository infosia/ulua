//! Five C11 builds and ten runs, plus one stock tsc batch for nine accepted scripts.
//! The test prints build, run, tsc, and total costs. It has no time assertion.
//! Measured Debug cost: 3.242 seconds for all five hosts and ten runs.
use std::{
    path::PathBuf,
    process::{Command, Output},
    time::Instant,
};

fn checked(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\nstatus: {}\n{}{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn c_hosts_match_expected_and_scripts_pass_tsc() {
    let start = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Use the archive from this test's profile, as the link tests do.
    let library = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("libulua.a");
    assert!(library.is_file(), "{}", library.display());
    let scratch =
        Scratch(std::env::temp_dir().join(format!("ulua-example-hosts-{}", std::process::id())));
    std::fs::create_dir_all(&scratch.0).unwrap();
    let examples = [
        ("hello", "hello.ts", None),
        ("capabilities", "game.ts", Some("game.d.ts")),
        ("async", "scores.ts", Some("scores.d.ts")),
        ("limits", "limits.ts", None),
        ("run", "hello.ts", None),
    ];
    let runner_cases = [
        ("reject.ts", "expected-reject.txt", 1),
        ("trap.ts", "expected-trap.txt", 2),
        ("async.ts", "expected-async.txt", 0),
        ("no-main.ts", "expected-no-main.txt", 0),
        ("init-trap.ts", "expected-init-trap.txt", 2),
    ];

    // All scripts are modules. One batch avoids four loads of the ES2022 library.
    let mut tsc = Command::new("node");
    tsc.current_dir(&root).args([
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
    ]);
    for (name, script, declarations) in examples {
        let directory = root.join("examples").join(name);
        tsc.arg(directory.join(script));
        if let Some(declarations) = declarations {
            tsc.arg(directory.join(declarations));
        }
    }
    for (script, _, code) in runner_cases {
        // The rejection fixture deliberately fails tsc. All other scripts must pass.
        if code != 1 {
            tsc.arg(root.join("examples/run").join(script));
        }
    }
    let tsc_start = Instant::now();
    let output = checked(&mut tsc);
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let tsc_cost = tsc_start.elapsed().as_secs_f64();

    let mut build_cost = 0.0;
    let mut run_cost = 0.0;
    let mut failures = Vec::new();
    for (name, script, _) in examples {
        let directory = root.join("examples").join(name);
        let executable = scratch.0.join(name);
        let mut compiler = Command::new("cc");
        if cfg!(ulua_sanitize) {
            compiler.arg("-fsanitize=address,undefined");
        }
        compiler
            .args(["-std=c11", "-Wall", "-Wextra", "-Wpedantic", "-Werror"])
            .arg(directory.join("host.c"))
            .arg("-I")
            .arg(root.join("include"))
            .arg(&library)
            .arg("-o")
            .arg(&executable);
        if cfg!(target_os = "macos") {
            compiler.args([
                "-lc++",
                "-liconv",
                "-framework",
                "Security",
                "-framework",
                "CoreFoundation",
            ]);
        } else {
            compiler.args(["-lstdc++", "-lpthread", "-ldl", "-lm"]);
        }
        let build_start = Instant::now();
        checked(&mut compiler);
        build_cost += build_start.elapsed().as_secs_f64();
        let run_start = Instant::now();
        let output = Command::new(&executable)
            .arg(directory.join(script))
            .output()
            .unwrap();
        run_cost += run_start.elapsed().as_secs_f64();
        let expected = std::fs::read(directory.join("expected.txt")).unwrap();
        if !output.status.success() || !output.stderr.is_empty() || output.stdout != expected {
            failures.push(format!(
                "{name}: status={} stdout_matches={}\nstdout:\n{}stderr:\n{}",
                output.status,
                output.stdout == expected,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        } else {
            eprintln!("example {name}: C11 static host and tsc PASS");
        }
    }
    for (script, expected, code) in runner_cases {
        let directory = root.join("examples/run");
        let run_start = Instant::now();
        let output = Command::new(scratch.0.join("run"))
            .arg(directory.join(script))
            .output()
            .unwrap();
        run_cost += run_start.elapsed().as_secs_f64();
        let expected = std::fs::read(directory.join(expected)).unwrap();
        if output.status.code() != Some(code)
            || !output.stderr.is_empty()
            || output.stdout != expected
        {
            failures.push(format!(
                "run/{script}: status={} expected_exit={code} stdout_matches={}\nstdout:\n{}stderr:\n{}",
                output.status,
                output.stdout == expected,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        } else {
            eprintln!("runner {script}: exit={code} stdout PASS");
        }
    }
    eprintln!(
        "examples hosts=5 runs=10 tsc_scripts=9 tsc_batches=1 build_cost={build_cost:.3}s \
         run_cost={run_cost:.3}s tsc_cost={tsc_cost:.3}s total_cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
