//! Path precedence uses isolated child environments, with no nested library build.
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn command(directory: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ulua"));
    cmd.current_dir(directory)
        .env_remove("ULUA_LIB")
        .env_remove("ULUA_INCLUDE");
    cmd
}
fn paths(output: Output) -> Vec<String> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}
#[test]
fn options_environment_and_repository() {
    let start = std::time::Instant::now();
    let scratch =
        Scratch(std::env::temp_dir().join(format!("ulua-cli-env-{}", std::process::id())));
    std::fs::create_dir_all(&scratch.0).unwrap();
    let library = scratch.0.join("chosen.a");
    std::fs::write(&library, b"link-flags does not read the archive").unwrap();
    let include = scratch.0.join("headers");
    std::fs::create_dir_all(&include).unwrap();
    std::fs::write(include.join("ulua.h"), b"").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let header_arg = format!("-I{}", std::fs::canonicalize(&include).unwrap().display());
    let library_arg = std::fs::canonicalize(&library)
        .unwrap()
        .display()
        .to_string();
    let result = paths(
        command(&scratch.0)
            .args(["link-flags", "--cc", "unix", "--lib"])
            .arg(&library)
            .env("ULUA_LIB", "missing.a")
            .env("ULUA_INCLUDE", &include)
            .output()
            .unwrap(),
    );
    assert_eq!(result[0], header_arg);
    assert_eq!(result[1], library_arg);
    let result = paths(
        command(&scratch.0)
            .args(["link-flags", "--cc", "unix", "--include"])
            .arg(&include)
            .env("ULUA_LIB", &library)
            .env("ULUA_INCLUDE", "missing-headers")
            .output()
            .unwrap(),
    );
    assert_eq!(result[0], header_arg);
    assert_eq!(result[1], library_arg);
    let result = paths(
        command(&scratch.0)
            .args(["link-flags", "--cc", "unix", "--lib"])
            .arg(&library)
            .output()
            .unwrap(),
    );
    assert_eq!(
        result[0],
        format!(
            "-I{}",
            std::fs::canonicalize(root.join("include"))
                .unwrap()
                .display()
        )
    );
    let result = command(&scratch.0)
        .arg("link-flags")
        .env("ULUA_LIB", "missing.a")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    let message = String::from_utf8(result.stderr).unwrap();
    for text in [
        "--lib",
        "ULUA_LIB",
        "target/release",
        "cargo build --release -p ulua",
    ] {
        assert!(message.contains(text));
    }
    let result = command(&scratch.0)
        .args(["link-flags", "--lib"])
        .arg(&library)
        .args(["--include", "missing"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    let default = root
        .join("target/release")
        .join(if cfg!(target_env = "msvc") {
            "ulua.lib"
        } else {
            "libulua.a"
        });
    let result = command(&scratch.0).arg("link-flags").output().unwrap();
    if default.is_file() {
        assert_eq!(
            paths(result)[1],
            std::fs::canonicalize(default)
                .unwrap()
                .display()
                .to_string()
        );
    } else {
        assert_eq!(result.status.code(), Some(2));
    }
    eprintln!(
        "cli environment processes=6 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
