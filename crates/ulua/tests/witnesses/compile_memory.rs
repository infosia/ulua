//! P5.4 measures the exported C ABI for each growth shape at 16 KiB and 64 KiB.
//! Measured total cost: Debug 80.701 seconds; Release 21.990 seconds, for 277 shapes and 166 small sources.
//! Debug costs: shape ABI 73.431 seconds; small sources 0.049 seconds; native probe 5.883 seconds.
//! Debug shape ABI costs: Rust 67.693 seconds; native compilation 5.544 seconds; output capture 0.046 seconds.
//! The observer writes borrowed output to preopened files without a Rust heap buffer or a second Rust compiler call.
//! The native probe measures all 276 emitted outputs at 64 KiB. One rejected shape has no output.
//! This timing witness asserts byte counts. Borrowed inputs, stack space, and native allocations are excluded.
// External ABI descriptors are non-exhaustive and require field assignments.
#![allow(clippy::field_reassign_with_default)]
#[path = "../support/compile_alloc.rs"]
mod alloc;
#[path = "../support/growth_shapes.rs"]
mod shapes;

use std::{
    collections::BTreeSet,
    ffi::CStr,
    fs::File,
    io::Write,
    path::Path,
    ptr,
    sync::Mutex,
    time::{Duration, Instant},
};
use ulua::compile_measurement::{CompileEvent, replace_observer};
use ulua::*;
use ulua_compiler::{COMPILE_MEMORY_BASE, COMPILE_MEMORY_FACTOR};

#[derive(Default)]
struct Costs {
    rust: Duration,
    native: Duration,
    capture: Duration,
}
struct Profile {
    file: Option<File>,
    boundary: Instant,
    costs: Costs,
}
static PROFILE: Mutex<Option<Profile>> = Mutex::new(None);

fn observe(event: CompileEvent<'_>) {
    let mut profile = PROFILE.lock().unwrap();
    let profile = profile.as_mut().unwrap();
    match event {
        CompileEvent::Start => profile.boundary = Instant::now(),
        CompileEvent::Rust(source) => {
            profile.costs.rust += profile.boundary.elapsed();
            let start = Instant::now();
            if let Some(file) = &mut profile.file {
                // The preopened file borrows the output. This write allocates no Rust buffer.
                file.write_all(source.as_bytes()).unwrap();
            }
            profile.costs.capture += start.elapsed();
            profile.boundary = Instant::now();
        }
        CompileEvent::Rejected => profile.costs.rust += profile.boundary.elapsed(),
        CompileEvent::Native => profile.costs.native += profile.boundary.elapsed(),
        _ => {}
    }
}

unsafe extern "C" fn unused(
    _: *mut std::ffi::c_void,
    _: *const Value,
    _: usize,
    _: *mut Value,
) -> i32 {
    1
}
unsafe extern "C" fn unused_start(
    _: *mut std::ffi::c_void,
    _: *const Value,
    _: usize,
    _: u64,
) -> i32 {
    1
}

fn compile(source: &str, output: Option<&Path>) -> (Status, String, usize, Costs) {
    let signatures = shapes::grants(source);
    let mut context = ptr::null_mut();
    let mut module = ptr::null_mut();
    let mut diagnostic = Diagnostic::default();
    // SAFETY: The test owns all inputs and output storage and serializes this context.
    unsafe {
        assert_eq!(ulua_context_new(&mut context), ULUA_OK);
        let mut grants = Vec::new();
        for signature in &signatures {
            let mut descriptor = CapabilityDesc::default();
            descriptor.name = signature.name.as_ptr().cast();
            descriptor.name_length = signature.name.len();
            descriptor.parameter_types = signature.parameters.as_ptr();
            descriptor.parameter_count = signature.parameters.len();
            descriptor.result_type = signature.result;
            if signature.asynchronous {
                descriptor.start = Some(unused_start);
            } else {
                descriptor.function = Some(unused);
            }
            let mut grant = ptr::null_mut();
            assert_eq!(
                ulua_capability_new(context, &descriptor, &mut grant),
                ULUA_OK
            );
            grants.push(grant);
        }
        *PROFILE.lock().unwrap() = Some(Profile {
            file: output.map(|path| File::create(path).unwrap()),
            boundary: Instant::now(),
            costs: Costs::default(),
        });
        let prior = replace_observer(Some(observe));
        let (status, peak) = alloc::measure(|| {
            ulua_compile(
                context,
                source.as_ptr().cast(),
                source.len(),
                grants.as_ptr(),
                grants.len(),
                ptr::null(),
                &mut module,
                &mut diagnostic,
            )
        });
        replace_observer(prior);
        let profile = PROFILE.lock().unwrap().take().unwrap();
        let code = CStr::from_ptr(diagnostic.code.as_ptr())
            .to_string_lossy()
            .into_owned();
        assert_eq!(module.is_null(), status != ULUA_OK);
        assert_eq!(ulua_context_free(context), ULUA_OK);
        (status, code, peak, profile.costs)
    }
}

fn nested_blocks(_: &str, bytes: usize) -> String {
    let mut source = "export function main():void{".to_owned();
    let unit = format!("{}{}", "{".repeat(44), "}".repeat(44));
    while source.len() + unit.len() < bytes {
        source.push_str(&unit);
    }
    source.push('}');
    source.extend(std::iter::repeat_n(' ', bytes - source.len()));
    source
}

fn small_sources() -> BTreeSet<String> {
    let mut sources = BTreeSet::from([String::new()]);
    for unit in [
        ";",
        "let a=1",
        "class A{}",
        "{}",
        "{let a=1;}",
        "class A{a=1;}",
        "class A{f():void{}}",
        "function f():void{}",
        "export function main():void{}",
        "type A={a:number};",
        "let a=[1,2,3];",
        "let a={x:1,y:2};",
        "class A{constructor(){}}",
        "class A{}class B extends A{}",
    ] {
        for repetitions in 1..=64 / unit.len() {
            sources.insert(unit.repeat(repetitions));
        }
    }
    // Distinct names keep repeated declarations accepted.
    for unit in [
        "class @{}",
        "class @{a=1;}",
        "function @():void{}",
        "let @=1;",
    ] {
        let mut source = String::new();
        for name in 'A'..='Z' {
            source.push_str(&unit.replace('@', &name.to_string()));
            if source.len() > 64 {
                break;
            }
            sources.insert(source.clone());
        }
    }
    sources
}

#[test]
fn compile_memory_bound() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    // Pin cross-thread accounting with a known allocation, not a compiler-derived count.
    let (buffer, peak) = alloc::measure(|| {
        std::thread::Builder::new()
            .name("ulua-compiler".into())
            .spawn(|| vec![0_u8; 4096])
            .unwrap()
            .join()
            .unwrap()
    });
    assert!(peak >= 4096, "worker allocation missing: {peak}");
    drop(buffer);
    let borrowed = vec![0_u8; 8192];
    let (_, peak) = alloc::measure(|| drop(borrowed));
    assert_eq!(peak, 0, "a prior allocation must not affect this count");
    let (_, peak) = alloc::measure(|| {
        let layout = std::alloc::Layout::from_size_align(4096, 64).unwrap();
        // SAFETY: The layouts match each allocation. The test releases the resized allocation once.
        unsafe {
            let pointer = std::alloc::alloc(layout);
            assert!(!pointer.is_null());
            assert_eq!(pointer.addr() % 64, 0);
            let pointer = std::alloc::realloc(pointer, layout, 8192);
            assert!(!pointer.is_null());
            assert_eq!(pointer.addr() % 64, 0);
            std::alloc::dealloc(
                pointer,
                std::alloc::Layout::from_size_align(8192, 64).unwrap(),
            );
        }
    });
    assert_eq!(peak, 8192, "count payload bytes without allocator metadata");

    let mut violations = Vec::new();
    let mut small_largest = (0, String::new());
    let sources = small_sources();
    let small_start = Instant::now();
    for source in &sources {
        let (status, code, peak, _) = compile(source, None);
        assert!(
            matches!(status, ULUA_OK | ULUA_REJECTED),
            "{source:?}: {code}"
        );
        if peak > small_largest.0 {
            small_largest = (peak, source.clone());
        }
        eprintln!(
            "P5.4 small source={source:?} bytes={} peak={peak} status={status} code={code}",
            source.len()
        );
        if peak > COMPILE_MEMORY_FACTOR * source.len() + COMPILE_MEMORY_BASE {
            violations.push(format!("small {source:?} peak={peak}"));
        }
    }
    let small_cost = small_start.elapsed();
    let mut abi_cost = Duration::ZERO;
    let mut costs = Costs::default();
    let mut largest = (0.0_f64, String::new(), 0, 0);
    let directory =
        std::env::temp_dir().join(format!("ulua-compile-memory-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut paths = Vec::new();
    let mut growth = shapes::all();
    growth.push(shapes::Shape {
        label: "nested-empty-blocks".into(),
        key: "",
        make: nested_blocks,
        diagnostic: None,
    });
    for shape in &growth {
        for bytes in [16384, 65536] {
            let source = (shape.make)(shape.key, bytes);
            assert_eq!(source.len(), bytes, "{}", shape.label);
            let path = (bytes == 65536).then(|| directory.join(format!("{}.luau", shape.label)));
            let now = Instant::now();
            let (status, code, peak, cost) = compile(&source, path.as_deref());
            abi_cost += now.elapsed();
            costs.rust += cost.rust;
            costs.native += cost.native;
            costs.capture += cost.capture;
            let ratio = peak as f64 / bytes as f64;
            if ratio > largest.0 {
                largest = (ratio, shape.label.clone(), bytes, peak);
            }
            eprintln!(
                "P5.4 {} bytes={bytes} peak={peak} ratio={ratio:.6} status={status} code={code}",
                shape.label
            );
            if peak > COMPILE_MEMORY_FACTOR * bytes + COMPILE_MEMORY_BASE {
                violations.push(format!("{} bytes={bytes} peak={peak}", shape.label));
            }
            if status == ULUA_OK {
                assert!(shape.diagnostic.is_none(), "{}", shape.label);
                if let Some(path) = path {
                    paths.push(path);
                }
            } else {
                assert_eq!(status, ULUA_REJECTED, "{}: {code}", shape.label);
                assert!(
                    Some(code.as_str()) == shape.diagnostic || code.starts_with("limit:"),
                    "{}: {code}",
                    shape.label
                );
            }
        }
    }
    eprintln!(
        "P5.4 breakdown small={:.3}s shapes-ABI={:.3}s ABI-Rust={:.3}s ABI-native={:.3}s output-capture={:.3}s duplicate-input=0s",
        small_cost.as_secs_f64(),
        abi_cost.as_secs_f64(),
        costs.rust.as_secs_f64(),
        costs.native.as_secs_f64(),
        costs.capture.as_secs_f64()
    );
    let derived_factor = (largest.3 * 5).div_ceil(largest.2 * 4);
    let derived_base = (small_largest.0 * 5).div_ceil(4);
    eprintln!(
        "P5.4 largest={:.6} shape={} bytes={} peak={} derived-factor={} small-peak={} small-source={:?} derived-base={} shapes={} small-sources={} Rust/input cost={:.3}s",
        largest.0,
        largest.1,
        largest.2,
        largest.3,
        derived_factor,
        small_largest.0,
        small_largest.1,
        derived_base,
        growth.len(),
        sources.len(),
        start.elapsed().as_secs_f64()
    );
    assert!(
        derived_factor <= COMPILE_MEMORY_FACTOR,
        "derived factor {derived_factor} exceeds {COMPILE_MEMORY_FACTOR}"
    );
    assert!(
        derived_base <= COMPILE_MEMORY_BASE,
        "derived base {derived_base} exceeds {COMPILE_MEMORY_BASE}"
    );
    native_probe(&paths);
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!("P5.4 total cost={:.3}s", start.elapsed().as_secs_f64());
    assert!(violations.is_empty(), "bound violations: {violations:?}");
}

fn native_probe(paths: &[std::path::PathBuf]) {
    let start = Instant::now();
    let result = std::process::Command::new(env!("ULUA_COMPILE_MEMORY_PROBE"))
        .args(paths)
        .output()
        .unwrap();
    eprintln!("{}", String::from_utf8_lossy(&result.stdout));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    eprintln!(
        "P5.4 native shapes={} cost={:.3}s",
        paths.len(),
        start.elapsed().as_secs_f64()
    );
}
