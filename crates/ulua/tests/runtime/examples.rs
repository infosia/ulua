//! Five C11 builds and twenty runs, plus one stock tsc batch for nineteen scripts.
//! The test prints build, run, tsc, and total costs. It has no time assertion.
//! Measured Debug cost: 3.356 seconds for five builds, twenty runs, and one tsc batch.
#[path = "../support/host.rs"]
mod host;
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

fn expected(path: &std::path::Path) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    // MSVC stdout in text mode expands every LF byte to CRLF.
    if cfg!(windows) {
        bytes
            .into_iter()
            .flat_map(|b| {
                if b == b'\n' {
                    vec![b'\r', b'\n']
                } else {
                    vec![b]
                }
            })
            .collect()
    } else {
        bytes
    }
}

// Check both directions so neither a script nor an expected file can escape the gate.
fn typescript_cases(directory: &std::path::Path) -> Vec<(PathBuf, PathBuf)> {
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if let Some(stem) = name.strip_suffix(".ts") {
            let expected = directory.join(format!("expected-{stem}.txt"));
            assert!(path.is_file(), "not a script file: {}", path.display());
            assert!(
                expected.is_file(),
                "missing expected file: {}",
                expected.display()
            );
            cases.push((path, expected));
        } else if let Some(stem) = name
            .strip_prefix("expected-")
            .and_then(|name| name.strip_suffix(".txt"))
        {
            let script = directory.join(format!("{stem}.ts"));
            assert!(path.is_file(), "not an expected file: {}", path.display());
            assert!(script.is_file(), "missing script: {}", script.display());
        }
    }
    cases.sort();
    assert!(!cases.is_empty(), "no scripts in {}", directory.display());
    cases
}

fn expected_exit(bytes: &[u8]) -> i32 {
    if bytes.starts_with(b"error ") {
        1
    } else if bytes
        .split(|&byte| byte == b'\n')
        .any(|line| line.starts_with(b"stopped "))
    {
        2
    } else {
        0
    }
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
    let library =
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(if cfg!(target_env = "msvc") {
                "ulua.lib"
            } else {
                "libulua.a"
            });
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

    let typescript_cases = typescript_cases(&root.join("examples/typescript"));
    let run_count = examples.len() + runner_cases.len() + typescript_cases.len();
    let tsc_count = examples.len()
        + runner_cases
            .iter()
            .filter(|(_, _, code)| *code != 1)
            .count()
        + typescript_cases.len();

    // All scripts are modules. One batch shares one load of the ES2022 library.
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
    // These include rejected.ts: Ulua rejects it, but stock TypeScript accepts it.
    for (script, _) in &typescript_cases {
        tsc.arg(script);
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
        let executable = scratch
            .0
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        let mut compiler = host::compile(
            &root,
            &directory.join("host.c"),
            &library,
            &executable,
            false,
        );
        let build_start = Instant::now();
        checked(&mut compiler);
        build_cost += build_start.elapsed().as_secs_f64();
        let run_start = Instant::now();
        let output = Command::new(&executable)
            .arg(directory.join(script))
            .output()
            .unwrap();
        run_cost += run_start.elapsed().as_secs_f64();
        let expected = expected(&directory.join("expected.txt"));
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
    let runner_cases = runner_cases
        .into_iter()
        .map(|(script, expected, code)| {
            let directory = root.join("examples/run");
            (directory.join(script), directory.join(expected), code)
        })
        .chain(typescript_cases.into_iter().map(|(script, expected)| {
            let code = expected_exit(&std::fs::read(&expected).unwrap());
            (script, expected, code)
        }));
    for (script, expected, code) in runner_cases {
        let run_start = Instant::now();
        let output = Command::new(
            scratch
                .0
                .join(format!("run{}", std::env::consts::EXE_SUFFIX)),
        )
        .arg(&script)
        .output()
        .unwrap();
        run_cost += run_start.elapsed().as_secs_f64();
        let expected = self::expected(&expected);
        let label = script.strip_prefix(&root).unwrap().display();
        if output.status.code() != Some(code)
            || !output.stderr.is_empty()
            || output.stdout != expected
        {
            failures.push(format!(
                "{label}: status={} expected_exit={code} stdout_matches={}\nstdout:\n{}stderr:\n{}",
                output.status,
                output.stdout == expected,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        } else {
            eprintln!("runner {label}: exit={code} stdout PASS");
        }
    }
    eprintln!(
        "examples hosts=5 runs={run_count} tsc_scripts={tsc_count} tsc_batches=1 build_cost={build_cost:.3}s \
         run_cost={run_cost:.3}s tsc_cost={tsc_cost:.3}s total_cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
