//! P5.4 measures every growth shape at 16 KiB and 64 KiB.
//! Final Debug cost, including the native probe: 60.459 seconds.
//! Measured Rust-only cost: Debug 59.744 seconds; Release 10.381 seconds, for 276 shapes.
//! Final native cost: 0.660 seconds for the five largest outputs.
//! This test runs in witnesses with feature timing, because Debug exceeds ten seconds.
//! The assertion uses byte counts only. Borrowed inputs and stack space are excluded.
#[path = "../support/compile_alloc.rs"]
mod alloc;
#[path = "../support/growth_shapes.rs"]
mod shapes;

use std::time::Instant;
use ulua_compiler::{COMPILE_MEMORY_BASE, COMPILE_MEMORY_FACTOR, Limits, compile_with_grants};

#[test]
fn compile_memory_bound() {
    let _measurement = crate::measurement_lock();
    std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn(measure)
        .unwrap()
        .join()
        .unwrap();
}

fn measure() {
    let start = Instant::now();
    let (empty, base) = alloc::measure(|| compile_with_grants(b"", Limits::default(), &[]));
    assert!(empty.is_ok());
    assert!(base <= COMPILE_MEMORY_BASE, "empty peak={base}");
    drop(empty);
    eprintln!("P5.4 empty peak={base}");
    let mut largest = (0.0_f64, String::new(), 0);
    let mut outputs = Vec::new();
    for shape in shapes::all() {
        let mut maximum = 0.0_f64;
        for bytes in [16384, 65536] {
            let source = (shape.make)(shape.key, bytes);
            assert_eq!(source.len(), bytes, "{}", shape.label);
            let grants = shapes::grants(&source);
            let (compiled, peak) = alloc::measure(|| {
                compile_with_grants(source.as_bytes(), Limits::default(), &grants)
            });
            let ratio = peak as f64 / bytes as f64;
            maximum = maximum.max(ratio);
            if ratio > largest.0 {
                largest = (ratio, shape.label.clone(), bytes);
            }
            eprintln!(
                "P5.4 {} bytes={bytes} peak={peak} ratio={ratio:.6}",
                shape.label
            );
            assert!(
                peak <= COMPILE_MEMORY_FACTOR * bytes + COMPILE_MEMORY_BASE,
                "{} bytes={bytes} peak={peak}",
                shape.label
            );
            match compiled {
                Ok(compiled) => {
                    assert!(shape.diagnostic.is_none(), "{}", shape.label);
                    // Retain only the five largest outputs for the native probe.
                    outputs.push((
                        compiled.luau.len(),
                        shape.label.clone(),
                        bytes,
                        compiled.luau,
                    ));
                    outputs.sort_by_key(|output| std::cmp::Reverse(output.0));
                    outputs.truncate(5);
                }
                Err(diagnostic) => {
                    eprintln!(
                        "P5.4 {} bytes={bytes} diagnostic={}",
                        shape.label, diagnostic.code
                    );
                    assert!(
                        Some(diagnostic.code) == shape.diagnostic
                            || diagnostic.code.starts_with("limit:"),
                        "{}: {diagnostic:?}",
                        shape.label
                    );
                }
            }
        }
        eprintln!("P5.4 shape={} maximum={maximum:.6}", shape.label);
    }
    eprintln!(
        "P5.4 largest={:.6} shape={} bytes={} base={base} cost={:.3}s",
        largest.0,
        largest.1,
        largest.2,
        start.elapsed().as_secs_f64()
    );
    let native_start = Instant::now();
    let directory =
        std::env::temp_dir().join(format!("ulua-compile-memory-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = Vec::new();
    for (size, shape, bytes, output) in outputs {
        let path = directory.join(format!("{shape}-{bytes}.luau"));
        std::fs::write(&path, output).unwrap();
        paths.push(path);
        eprintln!("P5.4 native-input shape={shape} bytes={bytes} output={size}");
    }
    let result = std::process::Command::new(env!("ULUA_COMPILE_MEMORY_PROBE"))
        .args(paths)
        .output()
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!("{}", String::from_utf8_lossy(&result.stdout));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    eprintln!(
        "P5.4 native cost={:.3}s total cost={:.3}s",
        native_start.elapsed().as_secs_f64(),
        start.elapsed().as_secs_f64()
    );
}
