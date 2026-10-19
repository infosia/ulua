//! Native hosts run through the CLI process with inherited standard streams.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn invoke(args: &[OsString]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_ulua"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}
struct Scratch(PathBuf);
impl Scratch {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("ulua-cli-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn script(&self, name: &str, text: &str) -> OsString {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path.into_os_string()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn build_hosts_and_link_flags() {
    let start = Instant::now();
    let scratch = Scratch::new("hosts");
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
    assert!(
        library.is_file(),
        "Cargo must emit the same static archive as the runtime example tests: {}",
        library.display()
    );
    let common = vec![
        OsString::from("--lib"),
        library.into_os_string(),
        "--include".into(),
        root().join("include").into_os_string(),
    ];
    for cc in ["unix", "msvc"] {
        let mut args = vec!["link-flags".into(), "--cc".into(), cc.into()];
        args.extend(common.clone());
        let (code, out, err) = invoke(&args);
        assert_eq!(code, 0, "{err}");
        assert!(err.is_empty());
        let lines: Vec<_> = out.lines().collect();
        assert!(lines[0].starts_with(if cc == "unix" { "-I" } else { "/I" }));
        assert!(Path::new(lines[1]).is_file());
        let libraries = ulua_toolchain::system_libraries().join(" ");
        assert_eq!(lines[2..].join(" "), libraries);
    }
    for example in ["hello", "run"] {
        let mut args = vec![
            "build".into(),
            "--host".into(),
            root()
                .join(format!("examples/{example}/host.c"))
                .into_os_string(),
            "-o".into(),
            scratch.0.join(example).into_os_string(),
        ];
        args.extend(common.clone());
        args.extend([
            "--run".into(),
            "--".into(),
            root()
                .join(format!("examples/{example}/hello.ts"))
                .into_os_string(),
        ]);
        let (code, out, err) = invoke(&args);
        assert_eq!(code, 0, "{err}");
        assert!(err.is_empty(), "{err}");
        let expected =
            std::fs::read_to_string(root().join(format!("examples/{example}/expected.txt")))
                .unwrap();
        let expected = if cfg!(windows) {
            expected.replace('\n', "\r\n")
        } else {
            expected
        };
        assert_eq!(out, expected);
    }
    let source = scratch.script("broken.c", "this is not C");
    let mut args = vec![
        "build".into(),
        "--host".into(),
        source,
        "-o".into(),
        scratch.0.join("bad").into_os_string(),
    ];
    args.extend(common.clone());
    let (code, out, err) = invoke(&args);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(!err.is_empty());
    let c = scratch.script(
        "first.c",
        "int answer(void); int main(void) {return answer();}\n",
    );
    let cpp = scratch.script("second.cpp", "extern \"C\" int answer(void) {return 7;}\n");
    let mut args = vec![
        "build".into(),
        "--host".into(),
        c,
        cpp,
        "-o".into(),
        scratch.0.join("mixed").into_os_string(),
    ];
    args.extend(common);
    assert_eq!(invoke(&args).0, 0);
    args.push("--run".into());
    assert_eq!(invoke(&args).0, 7);
    eprintln!(
        "cli host builds=5 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

fn host_options() -> Vec<OsString> {
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
    assert!(library.is_file());
    vec![
        "--lib".into(),
        library.into_os_string(),
        "--include".into(),
        root().join("include").into_os_string(),
    ]
}
#[test]
fn native_io_errors_and_compiler_arguments() {
    let scratch = Scratch::new("native-errors");
    let source = scratch.script("valid.c", "int main(void){return 0;}\n");
    let mut args = vec![
        OsString::from("build"),
        "--host".into(),
        source.clone(),
        "-o".into(),
        scratch.0.join("out").into_os_string(),
    ];
    args.extend(host_options());
    let missing = scratch.0.join("missing.c");
    args[2] = missing.clone().into_os_string();
    let (code, out, err) = invoke(&args);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains(&missing.display().to_string()), "{err}");
    assert!(
        err.contains(&std::fs::canonicalize(&missing).unwrap_err().to_string()),
        "{err}"
    );
    args[2] = source.clone();
    let blocked = scratch.script("blocked", "a regular file");
    args[4] = blocked.clone();
    let (code, _, err) = invoke(&args);
    assert_eq!(code, 2);
    assert!(err.contains(&blocked.display().to_string()));
    let os_error = std::fs::create_dir_all(Path::new(&blocked))
        .unwrap_err()
        .to_string();
    assert!(err.contains(&os_error), "{err}");
    args[4] = scratch.0.join("out").into_os_string();
    if !cfg!(target_env = "msvc") {
        let program = scratch.0.join("missing-compiler");
        let output = Command::new(env!("CARGO_BIN_EXE_ulua"))
            .args(&args)
            .env("CC", format!("{} -DVALUE=1", program.display()))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let err = String::from_utf8(output.stderr).unwrap();
        assert!(err.contains(&program.display().to_string()), "{err}");
        assert!(
            err.contains(&Command::new(&program).output().unwrap_err().to_string()),
            "{err}"
        );
        let c = scratch.script("flags.c", "#ifndef ULUA_CLI_C\n#error CC arguments missing\n#endif\nint answer(void); int main(void){return answer();}\n");
        let cpp = scratch.script("flags.cpp", "#ifndef ULUA_CLI_CPP\n#error CXX arguments missing\n#endif\nextern \"C\" int answer(void){return 7;}\n");
        let mut args = vec![
            "build".into(),
            "--host".into(),
            c,
            cpp,
            "-o".into(),
            scratch.0.join("flags").into_os_string(),
        ];
        args.extend(host_options());
        args.push("--run".into());
        let output = Command::new(env!("CARGO_BIN_EXE_ulua"))
            .args(args)
            .env("CC", "cc -DULUA_CLI_C=1")
            .env("CXX", "c++\t-DULUA_CLI_CPP=1")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(7),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
#[test]
#[cfg(unix)]
fn host_inherits_descriptors_and_signal_exit() {
    use std::process::Stdio;
    let scratch = Scratch::new("descriptors");
    let source = scratch.script(
        "descriptors.c",
        r#"#include <sys/stat.h>
#include <stdio.h>
int main(int argc, char **argv) {
    if (argc != 4) return 8;
    for (int fd=0; fd<3; ++fd) {
        struct stat actual, expected;
        if (fstat(fd, &actual) || stat(argv[fd+1], &expected)) return 9;
        if (actual.st_dev != expected.st_dev || actual.st_ino != expected.st_ino) return 10;
    }
    int c=getchar(); if(c != 'x') return 11;
    puts("host stdout"); fputs("host stderr\n", stderr); return 7;
}
"#,
    );
    let stdin = scratch.0.join("stdin");
    let stdout = scratch.0.join("stdout");
    let stderr = scratch.0.join("stderr");
    std::fs::write(&stdin, "x").unwrap();
    let mut args = vec![
        "build".into(),
        "--host".into(),
        source,
        "-o".into(),
        scratch.0.join("out").into_os_string(),
    ];
    args.extend(host_options());
    args.extend([
        "--run".into(),
        "--".into(),
        stdin.clone().into_os_string(),
        stdout.clone().into_os_string(),
        stderr.clone().into_os_string(),
    ]);
    let status = Command::new(env!("CARGO_BIN_EXE_ulua"))
        .args(args)
        .stdin(Stdio::from(std::fs::File::open(stdin).unwrap()))
        .stdout(Stdio::from(std::fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(&stderr).unwrap()))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(7));
    assert_eq!(std::fs::read_to_string(stdout).unwrap(), "host stdout\n");
    assert_eq!(std::fs::read_to_string(stderr).unwrap(), "host stderr\n");
    let source = scratch.script(
        "signal.c",
        "#include <signal.h>\nint main(void){raise(SIGTERM);return 0;}\n",
    );
    let mut args = vec![
        "build".into(),
        "--host".into(),
        source,
        "-o".into(),
        scratch.0.join("signal").into_os_string(),
    ];
    args.extend(host_options());
    args.push("--run".into());
    assert_eq!(invoke(&args), (1, String::new(), String::new()));
}

#[test]
fn compiler_error_filenames_are_escaped() {
    let scratch = Scratch::new("compiler-filenames");
    let suffix = if cfg!(unix) { "\u{202e}\n" } else { "\u{202e}" };
    let source = scratch.script(&format!("bad{suffix}.c"), "this is not C\n");
    let mut args = vec![
        "build".into(),
        "--host".into(),
        source,
        "-o".into(),
        scratch.0.join("out").into_os_string(),
    ];
    args.extend(host_options());
    let (code, out, err) = invoke(&args);
    assert_eq!(code, 1, "{err}");
    assert!(out.is_empty());
    assert!(!err.contains('\u{202e}'), "{err}");
    // Neither a raw name nor a raw direction override can forge another compiler line.
    assert!(!err.contains(&format!("bad{suffix}.c")), "{err}");
}
