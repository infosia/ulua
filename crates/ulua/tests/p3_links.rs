//! Default-build C11/C++17 static/dynamic links and the exact dynamic symbol set.
//! Four compilations and four runs cost 7.181 seconds in the measured Debug build.
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
    let static_library = deps.join("libulua.a");
    let dynamic_library = deps.join(if cfg!(target_os = "macos") {
        "libulua.dylib"
    } else {
        "libulua.so"
    });
    assert!(static_library.is_file());
    assert!(dynamic_library.is_file());
    let scratch = std::env::temp_dir().join(format!("ulua-p3-links-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    for (lang, compiler, standard) in [("c", "cc", "c11"), ("c++", "c++", "c++17")] {
        for (kind, library) in [("static", &static_library), ("dynamic", &dynamic_library)] {
            let executable = scratch.join(format!("{standard}-{kind}"));
            let mut cmd = Command::new(compiler);
            cmd.arg(format!("-std={standard}"))
                .args(["-Wall", "-Wextra", "-Wpedantic", "-Werror", "-x", lang])
                .arg(root.join("native/host_link.c"))
                .arg("-I")
                .arg(root.join("include"))
                .args(["-x", "none"])
                .arg(library)
                .arg("-o")
                .arg(&executable);
            if cfg!(target_os = "macos") {
                cmd.args([
                    "-lc++",
                    "-liconv",
                    "-framework",
                    "Security",
                    "-framework",
                    "CoreFoundation",
                ]);
            } else {
                cmd.args(["-lstdc++", "-lpthread", "-ldl", "-lm"]);
            }
            if kind == "dynamic" {
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
