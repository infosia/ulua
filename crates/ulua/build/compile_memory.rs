//! Build the standalone native allocation probe only for the timing test target.
use std::{env, path::Path};

pub fn build(root: &Path, native: &cc::Build) -> Result<(), String> {
    if env::var_os("CARGO_FEATURE_TIMING").is_none() {
        return Ok(());
    }
    let out = env::var_os("OUT_DIR").ok_or("OUT_DIR is absent")?;
    let out = Path::new(&out);
    let executable = out.join("ulua-compile-memory-probe");
    let mut command = native.get_compiler().to_command();
    command
        .arg(root.join("native/probes/compile_memory.cpp"))
        .arg(out.join("libulua_native.a"))
        .arg("-o")
        .arg(&executable);
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
