//! Build the standalone native allocation probe only for the timing test target.
use std::{env, path::Path};

pub fn build(root: &Path, native: &cc::Build) -> Result<(), String> {
    if env::var_os("CARGO_FEATURE_TIMING").is_none() {
        return Ok(());
    }
    let out = env::var_os("OUT_DIR").ok_or("OUT_DIR is absent")?;
    let out = Path::new(&out);
    let compiler = native.get_compiler();
    let executable = out.join(if compiler.is_like_msvc() {
        "ulua-compile-memory-probe.exe"
    } else {
        "ulua-compile-memory-probe"
    });
    let mut command = compiler.to_command();
    if compiler.is_like_msvc() {
        // Keep the object, debug files, and linker outputs inside the build directory.
        command
            .current_dir(out)
            // The probe throws allocation errors and catches compiler exceptions.
            .arg("/EHsc")
            .arg(format!("/Fe:{}", executable.display()))
            .arg(format!("/Fo:{}", out.join("compile_memory.obj").display()))
            .arg(root.join("native/probes/compile_memory.cpp"))
            .arg("/link")
            .arg(out.join("ulua_native.lib"))
            .args([
                "advapi32.lib",
                "bcrypt.lib",
                "ntdll.lib",
                "userenv.lib",
                "ws2_32.lib",
            ]);
    } else {
        command
            .arg(root.join("native/probes/compile_memory.cpp"))
            .arg(out.join("libulua_native.a"))
            .arg("-o")
            .arg(&executable);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Compiler memory probe build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    println!(
        "cargo:rustc-env=ULUA_COMPILE_MEMORY_PROBE={}",
        executable.display()
    );
    Ok(())
}
