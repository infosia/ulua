//! Build the pinned Luau sources, the protected shim, and a separate archive for native tests.
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
#[path = "build/sanitize.rs"]
mod sanitize;

fn instrument(build: &mut cc::Build) {
    if sanitize::enabled() {
        build
            .opt_level(1)
            .flag("-fsanitize=address,undefined")
            .flag("-fno-omit-frame-pointer")
            .flag("-fno-sanitize-recover=undefined");
    }
}

fn files(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    files.retain(|p| p.extension().is_some_and(|e| e == "cpp"));
    files.sort();
    Ok(files)
}
fn run() -> Result<(), String> {
    sanitize::link()?;
    println!("cargo:rerun-if-env-changed=LUAU_SOURCE_DIR");
    let source = PathBuf::from(
        env::var_os("LUAU_SOURCE_DIR").ok_or("Set LUAU_SOURCE_DIR to the pinned Luau checkout")?,
    );
    let root =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR is absent")?)
            .join("../..");
    let wanted = fs::read_to_string(root.join("luau-revision.txt")).map_err(|e| e.to_string())?;
    let revision = Command::new("git")
        .arg("-C")
        .arg(&source)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    let have = String::from_utf8_lossy(&revision.stdout);
    if !revision.status.success() || have.trim() != wanted.trim() {
        return Err(format!(
            "LUAU_SOURCE_DIR revision {}; luau-revision.txt requires {}",
            have.trim(),
            wanted.trim()
        ));
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join("luau-revision.txt").display()
    );
    let configure = || {
        let mut b = cc::Build::new();
        b.cpp(true)
            .std("c++17")
            .opt_level(3)
            .warnings(false)
            .define("NDEBUG", None)
            .define("LUA_USE_LONGJMP", "1");
        instrument(&mut b);
        for dir in [
            "Common/include",
            "Ast/include",
            "Bytecode/include",
            "Compiler/include",
            "VM/include",
            "VM/src",
            "Bytecode/src",
            "Compiler/src",
        ] {
            b.include(source.join(dir));
        }
        b.include(root.join("include"));
        b
    };
    let mut native = configure();
    for dir in ["Common/src", "Ast/src", "Bytecode/src", "Compiler/src"] {
        println!("cargo:rerun-if-changed={}", source.join(dir).display());
        native.files(files(&source.join(dir))?);
    }
    native.files(files(&root.join("native/shim"))?);
    if env::var_os("CARGO_FEATURE_MEASUREMENTS").is_some() {
        native.define("ULUA_MEASUREMENTS", None);

        println!(
            "cargo:rerun-if-changed={}",
            root.join("native/probes").display()
        );
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join("native/shim").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("include/ulua.h").display()
    );
    native
        .try_compile("ulua_native")
        .map_err(|e| e.to_string())?;
    let mut vm = configure();
    vm.flag("-fno-math-errno")
        .files(files(&source.join("VM/src"))?);
    println!("cargo:rerun-if-changed={}", source.join("VM").display());
    vm.try_compile("ulua_vm").map_err(|e| e.to_string())?;
    println!(
        "cargo:rerun-if-changed={}",
        root.join("native/layout.c").display()
    );
    let mut layout = cc::Build::new();
    instrument(&mut layout);
    layout
        .cargo_metadata(false)
        .include(root.join("include"))
        .file(root.join("native/layout.c"))
        .try_compile("ulua_test_layout")
        .map_err(|e| e.to_string())?;
    let mut probes = configure();
    probes
        .cargo_metadata(false)
        .file(root.join("native/probes/allocator.cpp"))
        .file(root.join("native/probes/runtime_errors.cpp"))
        .file(root.join("native/probes/stack_shrink.cpp"))
        .file(root.join("native/probes/host_steps.cpp"))
        .file(root.join("native/probes/async.cpp"));
    if env::var_os("CARGO_FEATURE_MEASUREMENTS").is_some() {
        probes
            .define("ULUA_MEASUREMENTS", None)
            .file(root.join("native/probes/s3.cpp"));
    }
    probes
        .try_compile("ulua_test_native")
        .map_err(|e| e.to_string())?;
    println!(
        "cargo:rerun-if-changed={}",
        root.join("native/probes").display()
    );
    if sanitize::enabled() {
        let executable = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is absent")?)
            .join("ulua-sanitize-probe");
        // Compile a standalone child. Neither library contains the seeded defects.
        let mut command = configure().get_compiler().to_command();
        command
            .arg(root.join("native/probes/sanitize.cpp"))
            .arg("-o")
            .arg(&executable);
        let output = command.output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Sanitizer probe build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        println!(
            "cargo:rustc-env=ULUA_SANITIZE_PROBE={}",
            executable.display()
        );
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Ulua build failed: {error}");
        std::process::exit(1);
    }
}
