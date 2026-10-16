//! Link the benchmark host to the same pinned VM as the product.
use std::{env, path::PathBuf};

fn run() -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=LUAU_SOURCE_DIR");
    println!("cargo:rerun-if-changed=luau_host.cpp");
    let source = PathBuf::from(env::var_os("LUAU_SOURCE_DIR").ok_or("Set LUAU_SOURCE_DIR")?);
    // The ulua dependency verifies the revision and builds the compiler and VM archives.
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .opt_level(3)
        .define("NDEBUG", None)
        .define("LUA_USE_LONGJMP", "1")
        .include(source.join("VM/include"))
        .include(source.join("Compiler/include"))
        .file("luau_host.cpp")
        .try_compile("ulua_bench_host")
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Benchmark host build failed: {error}");
        std::process::exit(1);
    }
}
