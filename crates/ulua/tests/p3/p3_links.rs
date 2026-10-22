//! Default-build C11/C++17 static/dynamic links and the exact dynamic symbol set.
//! A dedicated library build, four host compilations, and four runs verify the default features.
//! macOS aarch64 Debug link test: 29.750 seconds cold, 1.522 seconds with the dedicated build up to date.
//! The Cargo build costs 27.315 seconds cold and 0.181 seconds up to date.
#[path = "../support/host.rs"]
mod host;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
fn checked(command: &mut Command) -> std::process::Output {
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{command:?}\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
#[test]
fn c11_cpp17_static_dynamic_and_exports() {
    let now = Instant::now();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = std::env::temp_dir().join(format!(
        "ulua-p3-links-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&scratch).unwrap();
    let deps = build_libraries(&root);
    let static_library = deps.join(if cfg!(target_env = "msvc") {
        "ulua.lib"
    } else {
        "libulua.a"
    });
    let dynamic_library = deps.join(if cfg!(target_os = "windows") {
        "ulua.dll"
    } else if cfg!(target_os = "macos") {
        "libulua.dylib"
    } else {
        "libulua.so"
    });
    for library in [&static_library, &dynamic_library] {
        assert!(
            library.is_file(),
            "Missing test library: {}",
            library.display()
        );
    }
    if cfg!(target_os = "windows") {
        std::fs::copy(&dynamic_library, scratch.join("ulua.dll")).unwrap();
    }
    for (cpp, standard) in [(false, "c11"), (true, "c++17")] {
        for (kind, library) in [("static", &static_library), ("dynamic", &dynamic_library)] {
            let executable =
                scratch.join(format!("{standard}-{kind}{}", std::env::consts::EXE_SUFFIX));
            let link_library = if cfg!(target_env = "msvc") && kind == "dynamic" {
                deps.join("ulua.dll.lib")
            } else {
                library.clone()
            };
            assert!(
                link_library.is_file(),
                "Missing test library: {}",
                link_library.display()
            );
            let mut cmd = host::compile(
                &root,
                &root.join("native/host_link.c"),
                &link_library,
                &executable,
                cpp,
            );
            if kind == "dynamic" && !cfg!(target_env = "msvc") {
                cmd.arg(format!("-Wl,-rpath,{}", deps.display()));
            }
            checked(&mut cmd);
            checked(&mut Command::new(&executable));
            eprintln!("P3 link {standard} {kind}: PASS");
        }
    }
    let symbols = exported(&dynamic_library);
    let expected: BTreeSet<_> = [
        "ulua_abi_version",
        "ulua_default_compile_limits",
        "ulua_default_run_limits",
        "ulua_context_new",
        "ulua_context_free",
        "ulua_capability_new",
        "ulua_capability_revoke",
        "ulua_capability_free",
        "ulua_declarations",
        "ulua_compile",
        "ulua_module_free",
        "ulua_module_export_count",
        "ulua_module_export_name",
        "ulua_module_export_signature",
        "ulua_module_find_export",
        "ulua_instance_new",
        "ulua_instance_seed_random",
        "ulua_instance_free",
        "ulua_call",
        "ulua_call_start",
        "ulua_task_result",
        "ulua_task_discard",
        "ulua_request_complete",
        "ulua_request_fail",
        "ulua_pump",
        "ulua_instance_requests",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(symbols, expected);
    std::fs::remove_dir_all(scratch).unwrap();
    eprintln!(
        "P3 links symbols={} cost={:.3}s",
        symbols.len(),
        now.elapsed().as_secs_f64()
    );
}
fn build_libraries(root: &Path) -> PathBuf {
    let now = Instant::now();
    let executable = std::env::current_exe().unwrap();
    let profile_dir = executable.parent().unwrap().parent().unwrap();
    let target_dir = profile_dir.parent().unwrap().join("p3-links");
    let profile = profile_dir.file_name().unwrap().to_str().unwrap();
    // These tests execute native hosts. Use Cargo's host target explicitly, independent of build.target.
    let version = checked(Command::new(env!("CARGO")).arg("-vV"));
    let version = String::from_utf8(version.stdout).unwrap();
    let target = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .expect("cargo -vV must report its host target");
    // Only this test writes this target directory. No other command uses it.
    // Each Cargo build here selects default features, so shared outputs from other features or profiles are not inputs.
    checked(
        Command::new(env!("CARGO"))
            .current_dir(root)
            .args(["build", "--locked", "-p", "ulua", "--lib", "--profile"])
            .arg(if profile == "debug" { "dev" } else { profile })
            .arg("--target")
            .arg(target)
            .arg("--target-dir")
            .arg(&target_dir),
    );
    let libraries = target_dir.join(target).join(profile);
    eprintln!(
        "P3 default libraries: {} build={:.3}s",
        libraries.display(),
        now.elapsed().as_secs_f64()
    );
    libraries
}
fn exported(library: &Path) -> BTreeSet<String> {
    if cfg!(target_env = "msvc") {
        let tool = cc::windows_registry::find_tool("x86_64-pc-windows-msvc", "cl.exe").unwrap();
        let mut cmd = Command::new(tool.path().with_file_name("dumpbin.exe"));
        for (key, value) in tool.env() {
            cmd.env(key, value);
        }
        let output = checked(cmd.arg("/nologo").arg("/exports").arg(library));
        return String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                (fields.len() >= 4
                    && fields[0].parse::<u32>().is_ok()
                    && fields[1].chars().all(|c| c.is_ascii_hexdigit())
                    && fields[2].len() == 8
                    && fields[2].chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| fields[3].to_owned())
            })
            .collect();
    }
    let mut cmd = Command::new("nm");
    if cfg!(target_os = "macos") {
        cmd.arg("-gU");
    } else {
        cmd.args(["-D", "--defined-only"]);
    }
    cmd.arg(library);
    let output = checked(&mut cmd);
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(|s| s.strip_prefix('_').unwrap_or(s).to_owned())
        .collect()
}
