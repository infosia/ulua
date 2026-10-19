//! Each interrupt shape repeats in a script loop at the default one-million-step budget.
//! Measured cost: Debug 17.046 seconds, Release 3.911 seconds, for 64 shapes and a 100-call addition cost sample.
#[path = "../support/interrupt_shapes.rs"]
#[allow(dead_code)]
mod shapes;
#[path = "../support/script.rs"]
#[allow(dead_code)]
mod support;
use std::time::Instant;
use ulua::{RunLimits, ULUA_FAILED};

fn run(source: &str) -> f64 {
    let mut script = support::Script::new(source);
    let now = Instant::now();
    assert_eq!(
        script.instance(RunLimits::default()),
        (ULUA_FAILED, "limit:steps".to_owned())
    );
    now.elapsed().as_secs_f64()
}

#[test]
fn all_interrupt_shape_step_time_ratios() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    addition_call_cost();
    let sources = std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn(|| {
            shapes::all_at(65536 - 256)
                .into_iter()
                .chain(shapes::conditions(65536 - 256))
                .chain(shapes::updates::all(65536 - 256))
                .map(|(name, source)| {
                    (
                        name.to_owned(),
                        if name == "exports" {
                            let mut source = source;
                            source.extend(std::iter::repeat_n(' ', 65536 - source.len()));
                            source
                        } else {
                            shapes::loop_source(&source)
                        },
                    )
                })
                .chain(shapes::growth_loops(65536 - 256))
                .chain(shapes::round2::all(65536 - 256).into_iter()
                    .filter(|(name, _)| !name.contains("-boundary-"))
                    .map(|(name, source)| {
                        let source = if name.starts_with("async-") {
                            let work = source.trim_end().replace("export async function main(b:boolean)", "async function work(b:boolean)");
                            let mut out = work + "async function start():Promise<number>{while(true){await work(true);await work(false);}return 0;}const stop=start();export function main():void{}";
                            assert!(out.len() <= 65536);
                            out.extend(std::iter::repeat_n(' ', 65536 - out.len()));
                            out
                        } else { shapes::loop_source(&source) };
                        (name, source)
                    }))
                .collect::<Vec<_>>()
        })
        .unwrap()
        .join()
        .unwrap();
    let baseline = run(
        "function start():number{let i=0;while(true){i++;}return 0;}const stop=start();export function main():void{}",
    );
    let mut failures = vec![];
    for (name, source) in sources {
        assert_eq!(source.len() as u64, ulua_compiler::SOURCE_CEILING, "{name}");
        let elapsed = if name == "exports" {
            run_exports(&source)
        } else {
            run(&source)
        };
        let ratio = elapsed / baseline;
        eprintln!(
            "interrupt ratio {name}: baseline={baseline:.6}s time={elapsed:.6}s ratio={ratio:.3}"
        );
        if ratio > 6.0 {
            failures.push((name, ratio));
        }
    }
    eprintln!(
        "interrupt all ratios cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
    assert!(failures.is_empty(), "ratios exceed 6: {failures:?}");
}

// SAFETY: The private probe accepts live source bytes, a limits descriptor, and a fixed diagnostic buffer.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_shim_compile(
        source: *const std::ffi::c_char,
        length: usize,
        out: *mut *mut std::ffi::c_char,
        size: *mut usize,
        error: *mut std::ffi::c_char,
    ) -> i32;
    fn ulua_internal_shim_bytecode_free(out: *mut std::ffi::c_char);
    fn ulua_internal_probe_entry(
        bytecode: *const std::ffi::c_char,
        length: usize,
        limits: *const std::ffi::c_void,
        steps: *mut u64,
        seconds: *mut f64,
        error: *mut std::ffi::c_char,
    ) -> i32;
    fn ulua_internal_probe_step_loop(
        source: *const u8,
        length: usize,
        limits: *const std::ffi::c_void,
        error: *mut std::ffi::c_char,
    ) -> i32;
}
fn run_exports(source: &str) -> f64 {
    let source = source.to_owned();
    let luau = std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn(move || {
            ulua_compiler::test_compile_export_loop(
                source.as_bytes(),
                ulua_compiler::Limits::default(),
            )
            .unwrap()
            .luau
        })
        .unwrap()
        .join()
        .unwrap();
    let mut error = [0; 256];
    let mut bytecode = std::ptr::null_mut();
    let mut size = 0;
    // SAFETY: These buffers cover their declared lengths. The native output has one owner below.
    assert_eq!(
        unsafe {
            ulua_internal_shim_compile(
                luau.as_ptr().cast(),
                luau.len(),
                &mut bytecode,
                &mut size,
                error.as_mut_ptr(),
            )
        },
        0
    );
    let limits = RunLimits::default();
    let now = Instant::now();
    // SAFETY: The probe reads the live bytecode and descriptor, and owns its VM until the call returns.
    let status = unsafe {
        ulua_internal_probe_step_loop(
            bytecode.cast(),
            size,
            (&limits as *const RunLimits).cast(),
            error.as_mut_ptr(),
        )
    };
    let elapsed = now.elapsed().as_secs_f64();
    // SAFETY: The compiler returned this owned buffer. Free it exactly once after the probe.
    unsafe {
        ulua_internal_shim_bytecode_free(bytecode);
    }
    assert_eq!(status, 2, "export loop must reach SHIM_STEPS");
    // SAFETY: The native shim terminates this fixed diagnostic buffer.
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(error.as_ptr()) }
            .to_str()
            .unwrap(),
        "Step limit exceeded"
    );
    elapsed
}

fn addition_call_cost() {
    let source = std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn(|| {
            let (_, source) = shapes::all_at(65536 - 256).into_iter().next().unwrap();
            let work = source.trim_end().replace(
                "export function main(b:boolean):void{",
                "function work(b:boolean):void{",
            );
            work + "export function main():void{for(let i=0;i<100;i++){work(true);}}"
        })
        .unwrap()
        .join()
        .unwrap();
    let mut script = support::Script::new(&source);
    assert_eq!(
        script.instance(RunLimits::default()),
        (ulua::ULUA_OK, String::new())
    );
    let index = script.index("main");
    let now = Instant::now();
    let (status, code, _) = script.call(index, &[]);
    assert_eq!((status, code.as_str()), (ulua::ULUA_OK, ""));
    eprintln!(
        "interrupt addition call: samples=100 us={:.3}",
        now.elapsed().as_secs_f64() * 1e6 / 100.0
    );
}

/// One call measures charged steps. A 100-call batch amortizes the clock cost; no time assertion applies.
#[test]
fn spilled_header_entry_cost() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    // 174 direct bindings and 128 fields fill one spill header under the C15 local budget.
    let body = (0..302).map(|i| format!("let a{i}=0;")).collect::<String>();
    let source = format!("export function main():number{{{body}return 0;}}");
    let luau = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
        .unwrap()
        .luau;
    let header = luau
        .lines()
        .find(|line| line.trim_start().starts_with("local __ulua_spill"))
        .unwrap();
    let fields = header
        .split_once(" = (")
        .unwrap()
        .0
        .matches("__ulua_b")
        .count();
    assert_eq!(
        fields, 128,
        "the entry must contain one 128-field spill header"
    );
    let mut error = [0; 256];
    let mut bytecode = std::ptr::null_mut();
    let mut size = 0;
    let mut steps = 0;
    let mut seconds = 0.0;
    // SAFETY: All buffers cover their declared lengths. Free the owned bytecode after the synchronous calls.
    let status = unsafe {
        assert_eq!(
            ulua_internal_shim_compile(
                luau.as_ptr().cast(),
                luau.len(),
                &mut bytecode,
                &mut size,
                error.as_mut_ptr()
            ),
            0
        );
        let status = ulua_internal_probe_entry(
            bytecode,
            size,
            (&RunLimits::default() as *const RunLimits).cast(),
            &mut steps,
            &mut seconds,
            error.as_mut_ptr(),
        );
        ulua_internal_shim_bytecode_free(bytecode);
        status
    };
    // SAFETY: The native probe terminates this fixed diagnostic buffer.
    assert_eq!(
        status,
        0,
        "{}",
        unsafe { std::ffi::CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
    );
    eprintln!(
        "interrupt spill entry: fields=128 steps={steps} us={:.3} samples=100 test_cost={:.3}s",
        seconds * 1e6,
        start.elapsed().as_secs_f64()
    );
}

// Only the three Math shapes need four candidate measurements. The final gate checks every shape.
#[test]
#[ignore = "select NATIVE_CALL_STEPS with the four measurements of stdlib section 5 rule 5"]
fn math_native_charge_selection_ratios() {
    let _measurement = crate::measurement_lock();
    // SAFETY: The test-only getter has no arguments and returns the compiled native constant.
    #[link(name = "ulua_test_native", kind = "static")]
    unsafe extern "C" {
        fn ulua_internal_probe_native_call_steps() -> u64;
    }
    // SAFETY: This getter reads one fixed constant and owns no resources.
    let charge = unsafe { ulua_internal_probe_native_call_steps() };
    let sources = std::thread::Builder::new()
        .stack_size(ulua_compiler::COMPILER_STACK_BYTES)
        .spawn(|| {
            shapes::all_at(65536 - 256)
                .into_iter()
                .filter(|(name, _)| name.starts_with("math-"))
                .map(|(name, source)| (name, shapes::loop_source(&source)))
                .collect::<Vec<_>>()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(sources.len(), 3);
    let baseline = run(
        "function start():number{let i=0;while(true){i++;}return 0;}const stop=start();export function main():void{}",
    );
    let mut largest = 0.0f64;
    for (name, source) in sources {
        let elapsed = run(&source);
        let ratio = elapsed / baseline;
        largest = largest.max(ratio);
        eprintln!(
            "Math selection NATIVE_CALL_STEPS={charge} {name}: baseline={baseline:.6}s time={elapsed:.6}s ratio={ratio:.6}"
        );
    }
    eprintln!("Math selection NATIVE_CALL_STEPS={charge} largest={largest:.6}");
}
