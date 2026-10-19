//! Instruction gaps use lua_singlestep and the production compile options.
//! Measured Debug cost: 8.692 seconds for 26 source-ceiling shapes and both branch choices.
// Shared generators also serve the ratio and boundary fixtures.
#[allow(dead_code)]
#[path = "../support/interrupt_shapes.rs"]
mod shapes;
use std::{ffi::CStr, time::Instant};
use ulua_compiler::{COMPILER_STACK_BYTES, Limits};

// SAFETY: The probe reads source bytes and writes only the supplied counters and diagnostic.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_interrupts(
        source: *const u8,
        length: usize,
        branch: i32,
        largest: *mut u64,
        interrupts: *mut u64,
        statement: *mut u64,
        statement_line: *mut i32,
        total: *mut u64,
        run: *mut u64,
        result: *mut f64,
        error: *mut std::ffi::c_char,
        math: Option<extern "C" fn(u32, f64, f64) -> f64>,
    ) -> i32;
}

#[test]
fn source_ceiling_instruction_gaps() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    assert_eq!(ulua::ulua_abi_version(), 1);
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| {
            check_gaps(
                shapes::all()
                    .into_iter()
                    .chain(shapes::updates::all(65536))
                    .collect(),
            );
        })
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt instruction test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

/// Measured Debug cost: 0.898 seconds for seven source-ceiling conditions and both branch outcomes.
#[test]
fn wrapped_condition_instruction_gaps() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| check_gaps(shapes::conditions(65536)))
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt wrapped condition cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

fn check_gaps<S: AsRef<str>>(sources: Vec<(S, String)>) {
    let mut maximum = 0;
    let mut violations = vec![];
    for (name, source) in sources {
        let name = name.as_ref();
        assert_eq!(source.len(), 65536, "{name}");
        let luau = ulua_compiler::compile(source.as_bytes(), Limits::default())
            .unwrap_or_else(|error| panic!("{name}: {error:?}"))
            .luau;
        // Reuse this emission to check the statement form at no extra compile cost.
        assert!(
            !luau.lines().any(|line| line.trim_start().starts_with('(')),
            "{name}: a generated statement starts with a parenthesis"
        );
        if name == "moved-point" || name == "moved-nested" {
            let mut heads = luau.match_indices("    for __ulua_g");
            let head = if name == "moved-nested" {
                heads.nth(1)
            } else {
                heads.next()
            }
            .expect("numeric witness loop")
            .0;
            assert!(
                luau[..head].ends_with("    __ulua_interrupt()\n"),
                "{name}: the later point must move before the loop"
            );
            let body = luau[head..].split("\n    end\n").next().unwrap();
            assert_eq!(
                body.matches("__ulua_interrupt()").count(),
                1,
                "{name}: the body must also need a local point"
            );
        }
        let mut worst = 0;
        for branch in [0, 1] {
            if branch == 0 && (name.starts_with("exit-break") || name.starts_with("exit-nested")) {
                continue;
            }
            let mut gap = 0;
            let mut interrupts = 0;
            let mut statement = 0;
            let mut statement_line = 0;
            let mut total = 0;
            let mut run = 0;
            let mut result = f64::NAN;
            let mut error = [0; 256];
            // SAFETY: All pointers cover their declared lengths and remain live for this synchronous probe.
            let status = unsafe {
                ulua_internal_probe_interrupts(
                    luau.as_ptr(),
                    luau.len(),
                    branch,
                    &mut gap,
                    &mut interrupts,
                    &mut statement,
                    &mut statement_line,
                    &mut total,
                    &mut run,
                    &mut result,
                    error.as_mut_ptr(),
                    Some(math_reference),
                )
            };
            // SAFETY: The probe terminates this fixed diagnostic buffer.
            assert_eq!(
                status,
                0,
                "{name}: {}",
                unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
            );
            assert!(interrupts > 0);
            if name.starts_with("async-") {
                let expected = match name {
                    "async-prelude" => 1.0,
                    "async-resume-2" => 97.0,
                    _ => 49.0,
                };
                assert_eq!(
                    result, expected,
                    "{name}: every resume segment must execute"
                );
            }
            eprintln!(
                "interrupt shape {name} branch={branch}: gap={gap} statement={statement} line={statement_line} instructions={total} interrupts={interrupts} run={run}"
            );
            if run > ulua_compiler::INTERRUPT_STATEMENTS as u64 {
                violations.push(format!("{name}: executed statement run {run} exceeds 16"));
            }
            if statement > ulua_compiler::STATEMENT_INSTRUCTIONS {
                violations.push(format!(
                    "{name}: statement {statement} at line {statement_line} exceeds {}",
                    ulua_compiler::STATEMENT_INSTRUCTIONS
                ));
            }
            worst = worst.max(gap);
        }
        maximum = maximum.max(worst);
        eprintln!("interrupt gap {name}: {worst}");
    }
    eprintln!("interrupt maximum={maximum} violations={violations:?}");
    assert!(
        maximum <= ulua_compiler::INTERRUPT_INSTRUCTIONS,
        "instruction gap {maximum} exceeds {}",
        ulua_compiler::INTERRUPT_INSTRUCTIONS
    );
    assert!(violations.is_empty(), "statement bounds: {violations:?}");
}

/// Measured Debug cost: 3.976 seconds for twelve source-ceiling growth forms under the VM instruction probe.
#[test]
fn analyzer_growth_instruction_gaps() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| check_gaps(shapes::analyzer_growth(65536)))
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt analyzer growth cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

/// Measured Debug cost: 1.446 seconds for 195 source-ceiling cases and 356 VM executions.
/// One execution per branch exposes the review defects without wall-clock assertions.
#[test]
fn review_round2_instruction_bounds() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| check_gaps(shapes::round2::all(65536)))
        .unwrap()
        .join()
        .unwrap();
    eprintln!(
        "interrupt round2 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

/// Count prelude declarations in emitted text, separate from the emitter's flow state.
#[test]
fn async_prelude_statement_runs() {
    let source = "export async function main():Promise<void>{}";
    let luau = ulua_compiler::compile(source.as_bytes(), Limits::default())
        .unwrap()
        .luau;
    let mut count = 0;
    let mut largest = 0;
    for line in luau.lines() {
        // A direct top-level call fires an interrupt. Helper declarations only create a closure.
        if line.starts_with("__ulua_") && line.contains('(') {
            count = 0;
        } else if line.starts_with("local ") {
            count += 1;
            largest = largest.max(count);
        }
        if line.starts_with("__ulua_async_register(") {
            break;
        }
    }
    eprintln!("interrupt async prelude declaration run={largest}");
    assert!(
        largest <= ulua_compiler::INTERRUPT_STATEMENTS,
        "async prelude declaration run {largest} exceeds {}",
        ulua_compiler::INTERRUPT_STATEMENTS
    );
}

/// Hand-written Luau checks the VM counter with a legal call and a deliberately oversized call.
#[test]
fn statement_probe_separates_caller_and_callee() {
    assert_eq!(ulua::ulua_abi_version(), 1);
    for arguments in [32, 40] {
        let parameters = (0..arguments)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let arguments_text = vec!["x"; arguments].join(",");
        let source = format!(
            "local function g({parameters})\nlocal q=0\n{}return q\nend\nreturn {{main=function(b)\nlocal x=if b then 1 else 2\nlocal n=g({arguments_text})\nreturn n\nend}}\n",
            "q+=1\n".repeat(48)
        );
        let mut gap = 0;
        let mut interrupts = 0;
        let mut statement = 0;
        let mut line = 0;
        let mut total = 0;
        let mut run = 0;
        let mut result = f64::NAN;
        let mut error = [0; 256];
        // SAFETY: The source and counters remain live for this synchronous native probe.
        let status = unsafe {
            ulua_internal_probe_interrupts(
                source.as_ptr(),
                source.len(),
                1,
                &mut gap,
                &mut interrupts,
                &mut statement,
                &mut line,
                &mut total,
                &mut run,
                &mut result,
                error.as_mut_ptr(),
                Some(math_reference),
            )
        };
        assert_eq!(status, 0);
        assert_eq!(result, 48.0, "the callee must execute all additions");
        // This hand-written calibration bypasses the compiler's counted path:
        // its 48 additions intentionally have no points. Compiler-generated
        // shapes above always assert the run bound, with no shape exemptions.
        eprintln!("interrupt handwritten calibration arguments={arguments}: run={run}");
        assert!(run > ulua_compiler::INTERRUPT_STATEMENTS as u64);
        // O1 writes the result directly: argument moves, the callee move, and CALL.
        assert_eq!(
            statement,
            arguments as u64 + 2,
            "caller excludes the callee's 48 additions"
        );
        assert_eq!(
            statement <= ulua_compiler::STATEMENT_INSTRUCTIONS,
            arguments == 32
        );
    }
}

// The instruction probe uses the product C closure and an independent Rust libm reference.
extern "C" fn math_reference(op: u32, a: f64, _b: f64) -> f64 {
    if op == 18 { libm::sin(a) } else { f64::NAN }
}

#[test]
fn review_header_zero_trip_instruction_runs() {
    let _measurement = crate::measurement_lock();
    std::thread::Builder::new()
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(|| check_gaps(shapes::updates::headers(65536)))
        .unwrap()
        .join()
        .unwrap();
}
