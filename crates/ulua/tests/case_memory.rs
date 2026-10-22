//! Cost: 0.37 s for all three tests on macOS arm64, Rust 1.95.0 debug.
//! A 64 KiB argument separates the old output charge from the Rust reservation.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use ulua::{RunLimits, ULUA_FAILED, ULUA_OK, ULUA_TYPE_STRING, Value};

// Cost: one ABI compilation and two calls with a 64 KiB input.
#[test]
fn upper_reserves_host_peak() {
    check("toUpperCase", 'ΐ');
}
// Cost: one ABI compilation and two calls with a 64 KiB input.
#[test]
fn lower_reserves_host_peak() {
    check("toLowerCase", 'İ');
}
fn check(method: &str, c: char) {
    let input = c.to_string().repeat(32768);
    let mut script = support::Script::new(&format!(
        "export function main(s:string):string{{return s.{method}();}}"
    ));
    let mut arg = Value::default();
    arg.r#type = ULUA_TYPE_STRING;
    arg.data = input.as_ptr().cast();
    arg.length = input.len();
    let mut limits = RunLimits::default();
    limits.max_memory_bytes = if method == "toUpperCase" {
        1024 * 1024
    } else {
        832 * 1024
    };
    assert_eq!(script.instance(limits).0, ULUA_OK);
    let (status, code, _) = script.call(script.index("main"), &[arg]);
    assert_eq!(
        (status, code),
        (ULUA_FAILED, "limit:memory".into()),
        "{method}"
    );
    limits.max_memory_bytes = 2 * 1024 * 1024;
    assert_eq!(script.instance(limits).0, ULUA_OK);
    let (status, code, value) = script.call(script.index("main"), &[arg]);
    assert_eq!((status, code), (ULUA_OK, String::new()));
    // SAFETY: The live instance owns the output until the next call.
    let output = unsafe { std::slice::from_raw_parts(value.data.cast::<u8>(), value.length) };
    let expected = if method == "toUpperCase" {
        input.to_uppercase()
    } else {
        input.to_lowercase()
    };
    assert_eq!(output, expected.as_bytes());
}

#[path = "support/host.rs"]
mod host;

// Cost: one C++ build and fourteen protected calls cover reservation accounting and release paths.
#[test]
fn reservation_tracks_only_host_temporary_and_releases_before_lua() {
    use std::{path::PathBuf, time::Instant};
    let now = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    let scratch = std::env::temp_dir().join(format!("ulua-case-budget-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let executable = scratch.join(if cfg!(windows) {
        "case-budget.exe"
    } else {
        "case-budget"
    });
    let original = host::compile(
        &root,
        &root.join("crates/ulua/tests/support/case_budget.cpp"),
        &library,
        &executable,
        true,
    );
    let luau = PathBuf::from(std::env::var_os("LUAU_SOURCE_DIR").expect("LUAU_SOURCE_DIR"));
    let mut extra: Vec<std::ffi::OsString> = vec![];
    for path in [
        root.join("native/shim"),
        luau.join("VM/include"),
        luau.join("Common/include"),
    ] {
        extra.push(
            if cfg!(target_env = "msvc") {
                "/I"
            } else {
                "-I"
            }
            .into(),
        );
        extra.push(path.into_os_string());
    }
    for name in ["ULUA_MEASUREMENTS", "ULUA_FAULT_INJECTION"] {
        extra.push(
            format!(
                "{}{name}",
                if cfg!(target_env = "msvc") {
                    "/D"
                } else {
                    "-D"
                }
            )
            .into(),
        );
    }
    if cfg!(target_env = "msvc") {
        // Luau headers contain UTF-8 text, including on hosts using code page 932.
        extra.push("/utf-8".into());
        // Complete production headers contain static helpers that this fixture does not call.
        extra.push("/wd4505".into());
    } else {
        extra.push("-Wno-unused-function".into());
        // Luau headers use the GNU variadic macro extension.
        extra.push("-Wno-pedantic".into());
    }
    // MSVC requires compiler options before /link. Preserve the host helper's SDK environment.
    let mut args: Vec<_> = original
        .get_args()
        .map(std::ffi::OsStr::to_os_string)
        .collect();
    let position = args
        .iter()
        .position(|arg| arg == "/link")
        .unwrap_or(args.len());
    args.splice(position..position, extra);
    let mut command = std::process::Command::new(original.get_program());
    command.args(args);
    if let Some(directory) = original.get_current_dir() {
        command.current_dir(directory);
    }
    for (name, value) in original.get_envs() {
        if let Some(value) = value {
            command.env(name, value);
        } else {
            command.env_remove(name);
        }
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(&executable).output().unwrap();
    let _ = std::fs::remove_dir_all(&scratch);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "{}build and fourteen calls: {:.3}s",
        String::from_utf8_lossy(&output.stdout),
        now.elapsed().as_secs_f64()
    );
}
