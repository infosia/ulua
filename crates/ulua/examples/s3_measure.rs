//! Reproduce S3 measurements with the pinned native compiler. This is not a gate.
//! Set LUAU_SOURCE_DIR and CARGO_NET_OFFLINE before each command.
//! Run `cargo run -p ulua --features measurements --example s3_measure -- MODE`.
//! MODE is stack, heap, heap-products, native, run, or emit (with an output directory). Add --release before -p for Release.
//! `stack --half` also verifies the compiler on half of its configured stack.
//! Native template emission is incremental: its probe reports a lower bound,
//! not an absolute maximum, when the next input still compiles.
// Each mode uses a subset of the shared generators.
#[allow(dead_code)]
#[path = "../src/p21_inputs.rs"]
mod functions;
#[allow(dead_code)]
#[path = "../src/s3_inputs.rs"]
mod inputs;
#[path = "../tests/support/growth_joins.rs"]
mod joins;
#[path = "../src/s6_inputs.rs"]
mod review;
#[path = "support/alloc.rs"]
mod s3_alloc;
use std::{
    ffi::{CStr, c_char},
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use ulua::*;
use ulua_compiler::{COMPILER_STACK_BYTES, Limits, SOURCE_CEILING};

// SAFETY: These declarations match the opt-in native probe and private shim.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_stack_start();
    fn ulua_internal_probe_stack_end() -> usize;
    fn ulua_internal_probe_heap() -> usize;
    fn ulua_internal_probe_ccalls(count: i32) -> i32;
    fn ulua_internal_probe_ccalls_protected(count: i32) -> i32;
    fn ulua_internal_shim_compile(
        source: *const c_char,
        length: usize,
        out: *mut *mut c_char,
        size: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(out: *mut c_char);
}
fn native(source: &str) -> Result<(), String> {
    let mut out = ptr::null_mut();
    let mut size = 0;
    let mut error = [0; 256];
    // SAFETY: Source and outputs cover their lengths. The shim catches every native compiler error.
    unsafe {
        let status = ulua_internal_shim_compile(
            source.as_ptr().cast(),
            source.len(),
            &mut out,
            &mut size,
            error.as_mut_ptr(),
        );
        ulua_internal_shim_bytecode_free(out);
        if status == 0 {
            Ok(())
        } else {
            Err(CStr::from_ptr(error.as_ptr())
                .to_string_lossy()
                .into_owned())
        }
    }
}
fn boundary(mut accepts: impl FnMut(usize) -> bool, maximum: usize) -> usize {
    let mut lo = 0;
    let mut hi = maximum + 1;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if accepts(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}
fn stages(form: &str, n: usize) {
    let source = inputs::nested(form, n);
    let mut values = Vec::new();
    let compiled =
        ulua_compiler::measure_compile(source.as_bytes(), Limits::default(), |stage, before| {
            // SAFETY: The probe touches only unused stack on this dedicated worker thread.
            unsafe {
                if before {
                    ulua_internal_probe_stack_start();
                } else {
                    values.push((stage, ulua_internal_probe_stack_end()));
                }
            }
        });
    let status = match compiled {
        Ok(c) => {
            // SAFETY: The probe uses this dedicated worker's unused stack only.
            unsafe {
                ulua_internal_probe_stack_start();
            }
            let result = native(&c.luau);
            // SAFETY: The matching start call initializes this thread's probe state.
            values.push(("luau", unsafe { ulua_internal_probe_stack_end() }));
            result.map_or_else(|e| e, |_| "ok".to_owned())
        }
        Err(e) => e.code.to_owned(),
    };
    for (_, bytes) in &values {
        assert!(*bytes <= COMPILER_STACK_BYTES / 2, "compiler stack margin");
    }
    println!("stack {form} levels={n} {values:?} status={status}");
}
struct HeapPeak {
    total: usize,
    rust: usize,
    native: usize,
}
fn heap(source: String, label: &str) -> HeapPeak {
    static STOP: AtomicBool = AtomicBool::new(false);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    static NATIVE_PEAK: AtomicUsize = AtomicUsize::new(0);
    // SAFETY: The platform malloc statistics function takes no arguments or ownership.
    let baseline = unsafe { ulua_internal_probe_heap() };
    PEAK.store(baseline, Ordering::Relaxed);
    STOP.store(false, Ordering::Relaxed);
    NATIVE_PEAK.store(0, Ordering::Relaxed);
    let rust_baseline = s3_alloc::start();
    let sampler = std::thread::spawn(move || {
        while !STOP.load(Ordering::Relaxed) {
            // SAFETY: malloc_zone_statistics supports concurrent allocator clients.
            let total = unsafe { ulua_internal_probe_heap() };
            PEAK.fetch_max(total, Ordering::Relaxed);
            // The sampled difference includes system allocator rounding of Rust requests.
            NATIVE_PEAK.fetch_max(
                total
                    .saturating_sub(baseline)
                    .saturating_sub(s3_alloc::current(rust_baseline)),
                Ordering::Relaxed,
            );
            std::thread::sleep(Duration::from_micros(100));
        }
    });
    let start = Instant::now();
    let compiled = ulua_compiler::compile(source.as_bytes(), Limits::default());
    let result = compiled
        .as_ref()
        .map_err(|e| e.code.to_owned())
        .and_then(|c| native(&c.luau));
    // SAFETY: The statistics query has no preconditions.
    PEAK.fetch_max(unsafe { ulua_internal_probe_heap() }, Ordering::Relaxed);
    STOP.store(true, Ordering::Relaxed);
    sampler.join().unwrap();
    let peak = HeapPeak {
        total: PEAK.load(Ordering::Relaxed).saturating_sub(baseline),
        rust: s3_alloc::peak(rust_baseline),
        native: NATIVE_PEAK.load(Ordering::Relaxed),
    };
    println!(
        "heap {label} bytes={} peak_delta={} peak_total={} rust_peak={} native_sample_peak={} elapsed={:?} status={result:?}",
        source.len(),
        peak.total,
        PEAK.load(Ordering::Relaxed),
        peak.rust,
        peak.native,
        start.elapsed()
    );
    assert_eq!(source.len(), SOURCE_CEILING as usize);
    assert_eq!(result, Ok(()), "heap shape {label}");
    peak
}
fn largest(label: &str, maximum: usize, make: impl Fn(usize) -> String) {
    let n = boundary(|n| native(&make(n)).is_ok(), maximum);
    println!(
        "native {label} accepted={n} next={:?}",
        native(&make(n + 1))
    );
}
fn run_stack() {
    for n in [197, 198, 199, 200] {
        // SAFETY: The native probe protects every recursive C call and uses no host objects.
        unsafe {
            ulua_internal_probe_stack_start();
            let status = ulua_internal_probe_ccalls(n);
            println!(
                "ccalls n={n} stack={} status={status}",
                ulua_internal_probe_stack_end()
            );
            ulua_internal_probe_stack_start();
            let status = ulua_internal_probe_ccalls_protected(n);
            println!(
                "protected ccalls n={n} stack={} status={status}",
                ulua_internal_probe_stack_end()
            );
        }
    }
    let source = inputs::recursion();
    let mut context = ptr::null_mut();
    let mut module = ptr::null_mut();
    let mut instance = ptr::null_mut();
    let mut d = Diagnostic::default();
    // SAFETY: Descriptors and output pointers are live for this serialized context.
    unsafe {
        assert_eq!(ulua_context_new(&mut context), ULUA_OK);
        assert_eq!(
            ulua_compile(
                context,
                source.as_ptr().cast(),
                source.len(),
                ptr::null(),
                &mut module,
                &mut d
            ),
            ULUA_OK
        );
        assert_eq!(
            ulua_instance_new(
                context,
                module,
                &Host::default(),
                ptr::null(),
                &mut instance,
                &mut d
            ),
            ULUA_OK
        );
        for depth in [19997, 19998, 19999, 20000] {
            let mut arg = Value::default();
            arg.r#type = ULUA_TYPE_NUMBER;
            arg.number = depth as f64;
            let mut result = Value::default();
            ulua_internal_probe_stack_start();
            let status = ulua_call(instance, 0, &arg, 1, &mut result, &mut d);
            println!(
                "run depth={depth} stack={} status={status} code={}",
                ulua_internal_probe_stack_end(),
                CStr::from_ptr(d.code.as_ptr()).to_string_lossy()
            );
        }
        ulua_context_free(context);
    }
}
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "stack".into());
    if !cfg!(target_os = "macos") && !matches!(mode.as_str(), "native" | "emit") {
        eprintln!("The stack watermark and native heap sampler need macOS.");
        std::process::exit(2);
    }
    let half = std::env::args().any(|arg| arg == "--half");
    let wide = std::env::args().any(|arg| arg == "--wide");
    let stack = if mode == "run" && !wide {
        ULUA_MIN_RUN_STACK
    } else if half {
        COMPILER_STACK_BYTES / 2
    } else {
        COMPILER_STACK_BYTES
    };
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || match mode.as_str() {
            "stack" => {
                for form in inputs::FORMS {
                    let n = boundary(
                        |n| match ulua_compiler::compile(
                            inputs::nested(form, n).as_bytes(),
                            Limits::default(),
                        ) {
                            Ok(_) => true,
                            Err(e) => *form == "assignment" && e.code == "C11",
                        },
                        128,
                    );
                    println!("form {form} boundary={n}");
                    for count in [8, 16, n, n + 1] {
                        stages(form, count);
                    }
                }
            }
            "heap-products" => {
                for shape in functions::SHAPES
                    .iter()
                    .filter(|s| s.starts_with("type-product-"))
                {
                    heap(functions::source(shape, SOURCE_CEILING as usize), shape);
                }
                heap(
                    review::source("spilled-mod-39", SOURCE_CEILING as usize),
                    "spilled-mod-39",
                );
                for links in [40, 41] {
                    let mut source = "export function main():void{".to_owned();
                    for i in 0..180 {
                        source.push_str(&format!("let v{i}=0;"));
                    }
                    source.push_str("let y=1;");
                    let unit = format!("y={};", vec!["y"; links + 1].join("%"));
                    // The review source ends near 65,518 bytes. Spaces fill the source ceiling.
                    while source.len() + unit.len() < 65_518 {
                        source.push_str(&unit);
                    }
                    source.push('}');
                    source.push_str(&" ".repeat(SOURCE_CEILING as usize - source.len()));
                    heap(source, &format!("spilled-mod-180-{links}-links"));
                }
            }
            "heap" => {
                let mut worst = [("", 0); 3];
                for label in inputs::SOURCE_SHAPES
                    .iter()
                    .chain(review::SHAPES)
                    .chain(joins::SHAPES)
                {
                    let source = if joins::SHAPES.contains(label) {
                        joins::source(label, SOURCE_CEILING as usize)
                    } else if review::SHAPES.contains(label) {
                        review::source(label, SOURCE_CEILING as usize)
                    } else if matches!(*label, "forward" | "backward" | "star") {
                        inputs::call_graph(label, SOURCE_CEILING as usize)
                    } else {
                        inputs::source_ceiling(label, SOURCE_CEILING as usize)
                    };
                    let peak = heap(source, label);
                    for (entry, bytes) in worst.iter_mut().zip([peak.total, peak.rust, peak.native])
                    {
                        if bytes > entry.1 {
                            *entry = (label, bytes);
                        }
                    }
                }
                println!(
                    "heap worst total={:?} rust={:?} native={:?}",
                    worst[0], worst[1], worst[2]
                );
            }
            "emit" => {
                let directory = std::env::args()
                    .nth(2)
                    .expect("emit requires an output directory");
                std::fs::create_dir_all(&directory).unwrap();
                for shape in inputs::SOURCE_SHAPES {
                    let source = if matches!(*shape, "forward" | "backward" | "star") {
                        inputs::call_graph(shape, SOURCE_CEILING as usize)
                    } else {
                        inputs::source_ceiling(shape, SOURCE_CEILING as usize)
                    };
                    let compiled =
                        ulua_compiler::compile(source.as_bytes(), Limits::default()).unwrap();
                    std::fs::write(
                        std::path::Path::new(&directory).join(format!("{shape}.luau")),
                        compiled.luau,
                    )
                    .unwrap();
                }
            }
            "native" => {
                largest("parameters", 256, |n| {
                    format!(
                        "local function f({}):number return 1 end return f",
                        (0..n)
                            .map(|i| format!("p{i}:number"))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                });
                largest("defaults", 128, |n| {
                    format!(
                        "local function f({}) {} return 1 end return f",
                        (0..n)
                            .map(|i| format!("p{i}:number?"))
                            .collect::<Vec<_>>()
                            .join(","),
                        (0..n)
                            .map(|i| format!(
                                "local b{i}:number=0 if p{i}==nil then b{i}=1 else b{i}=p{i} end "
                            ))
                            .collect::<String>()
                    )
                });
                largest("locals", 256, |n| {
                    format!(
                        "local function f() {} {} end return f",
                        (0..n)
                            .map(|i| format!("local t{i}=g({i}) "))
                            .collect::<String>(),
                        (0..n)
                            .map(|i| format!("if g then return t{i} end "))
                            .collect::<String>()
                    )
                });
                largest("temporaries-with-32-parameters", 256, |n| {
                    format!(
                        "local function f({}) {} {} end return f",
                        (0..32)
                            .map(|i| format!("p{i}:number"))
                            .collect::<Vec<_>>()
                            .join(","),
                        (0..n)
                            .map(|i| format!("local t{i}=g({i}) "))
                            .collect::<String>(),
                        (0..n)
                            .map(|i| format!("if g then return t{i} end "))
                            .collect::<String>()
                    )
                });
                largest("extra-temporaries-with-32-defaults", 256, |n| {
                    format!(
                        "local function f({}) {} {} {} end return f",
                        (0..32)
                            .map(|i| format!("p{i}:number?"))
                            .collect::<Vec<_>>()
                            .join(","),
                        (0..32)
                            .map(|i| format!(
                                "local b{i}:number=0 if p{i}==nil then b{i}=1 else b{i}=p{i} end "
                            ))
                            .collect::<String>(),
                        (0..n)
                            .map(|i| format!("local t{i}=g({i}) "))
                            .collect::<String>(),
                        (0..n)
                            .map(|i| format!("if g then return t{i} end "))
                            .collect::<String>()
                    )
                });
                largest("arguments-at-176-locals", 128, |n| {
                    format!(
                        "local function f(g,...) {} return g({}) end return f",
                        (0..175)
                            .map(|i| format!("local t{i}=i "))
                            .collect::<String>(),
                        (0..n)
                            .map(|i| format!("t{}", i % 175))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                });
                largest("template-substitutions (parts=2*n+1)", 100000, |n| {
                    ulua_compiler::measure_template(n).unwrap()
                });
            }
            "run" => run_stack(),
            _ => panic!("unknown mode"),
        })
        .unwrap()
        .join()
        .unwrap();
}
