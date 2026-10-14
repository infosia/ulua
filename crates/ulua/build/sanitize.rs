//! Shared sanitizer runtime links for workspace binaries and tests.
use std::{env, path::PathBuf, process::Command};

pub fn enabled() -> bool {
    env::var("ULUA_SANITIZE").as_deref() == Ok("1")
}

pub fn link() -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=ULUA_SANITIZE");
    println!("cargo:rustc-check-cfg=cfg(ulua_sanitize)");
    if !enabled() {
        return Ok(());
    }
    println!("cargo:rustc-cfg=ulua_sanitize");
    println!("cargo:rustc-link-arg=-fsanitize=address,undefined");
    // Rust uses -nodefaultlibs. Clang then omits its sanitizer runtime links.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let compiler = env::var_os("CC").unwrap_or_else(|| "cc".into());
        let output = Command::new(compiler)
            .arg("-print-resource-dir")
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("Cannot locate the Clang sanitizer runtime".into());
        }
        let directory =
            PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()).join("lib/darwin");
        let runtime = directory.join("libclang_rt.asan_osx_dynamic.dylib");
        if !runtime.is_file() {
            return Err("The Clang ASan runtime is absent".into());
        }
        // The Apple ASan runtime also exports the UBSan handlers.
        println!("cargo:rustc-link-arg={}", runtime.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", directory.display());
    } else {
        println!("cargo:rustc-link-lib=asan");
        println!("cargo:rustc-link-lib=ubsan");
    }
    Ok(())
}
