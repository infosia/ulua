//! Default-build C11/C++17 static/dynamic links and the exact dynamic symbol set.
//! Four compilations and four runs cost 7.181 seconds in the measured Debug build.
#[path = "../support/host.rs"]
mod host;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
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
    let deps = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
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
    assert!(static_library.is_file());
    assert!(dynamic_library.is_file());
    let scratch = std::env::temp_dir().join(format!("ulua-p3-links-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
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
