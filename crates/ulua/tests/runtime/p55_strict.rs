//! Small instances cross the interrupt and split boundaries in the strict Luau analyzer.
//! Measured Debug cost: 2.697 seconds for 80 small boundary instances with one analyzer worker.
#[path = "../support/analyzer.rs"]
mod analyzer;
// Shared generators also serve the source-ceiling probes.
#[allow(dead_code)]
#[path = "../support/interrupt_shapes.rs"]
mod shapes;
use std::{path::PathBuf, time::Instant};

#[test]
fn interrupt_shapes_are_strict_clean() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn(|| {
            let directory =
                std::env::temp_dir().join(format!("ulua-interrupt-strict-{}", std::process::id()));
            std::fs::create_dir_all(&directory).unwrap();
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            for (name, source) in shapes::small() {
                let compile = if name.starts_with("export-loop-boundary") {
                    ulua_compiler::test_compile_export_loop
                } else {
                    ulua_compiler::compile
                };
                let luau = compile(source.as_bytes(), ulua_compiler::Limits::default())
                    .unwrap()
                    .luau;
                if name == "paged-spill" {
                    assert!(
                        luau.contains(".__ulua_page0 ="),
                        "fixture must reach the paged form"
                    );
                }
                if name == "nested-boundaries" {
                    assert!(luau.matches("__ulua_interrupt()").count() >= 6);
                }
                let path = directory.join(format!("{name}.luau"));
                std::fs::write(&path, luau).unwrap();
                let now = Instant::now();
                let output = analyzer::command(root.join("target/luau-tools/luau-analyze"))
                    .arg(&path)
                    .output()
                    .unwrap();
                eprintln!(
                    "interrupt strict {name}: {:.3}s",
                    now.elapsed().as_secs_f64()
                );
                assert!(
                    output.status.success(),
                    "{name}: {}",
                    analyzer::failure(&output)
                );
            }
            std::fs::remove_dir_all(directory).unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt strict test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
