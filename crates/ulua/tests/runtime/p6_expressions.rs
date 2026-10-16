//! Two scaled workloads measure instruction costs without timing assertions.
use std::{ffi::CStr, time::Instant};

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
    ) -> i32;
}

fn instructions(source: &str, expected: f64) -> u64 {
    let (mut gap, mut interrupts, mut statement, mut total, mut run) = (0, 0, 0, 0, 0);
    let mut line = 0;
    let mut result = f64::NAN;
    let mut error = [0; 256];
    // SAFETY: All buffers remain live through this synchronous call and cover their declared lengths.
    let status = unsafe {
        ulua_internal_probe_interrupts(
            source.as_ptr(),
            source.len(),
            0,
            &mut gap,
            &mut interrupts,
            &mut statement,
            &mut line,
            &mut total,
            &mut run,
            &mut result,
            error.as_mut_ptr(),
        )
    };
    // SAFETY: The probe terminates the diagnostic buffer.
    assert_eq!(
        status,
        0,
        "{}",
        unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
    );
    assert_eq!(result, expected);
    assert!(gap <= ulua_compiler::INTERRUPT_INSTRUCTIONS, "gap={gap}");
    assert!(
        statement <= ulua_compiler::STATEMENT_INSTRUCTIONS,
        "statement={statement} line={line}"
    );
    total
}

#[test]
fn expression_workload_instruction_costs() {
    let _measurement = crate::measurement_lock();
    let start = Instant::now();
    // fib(20) makes 2*F(21)-1 calls. The loop executes 25*sum(0..39) inner iterations.
    for (id, source, units, expected) in [
        (
            "fib-recursive",
            include_str!("../../../ulua_bench/workloads/ulua/fib-recursive.ts").replace("31", "20"),
            21891,
            6765.0,
        ),
        (
            "fib-loop",
            include_str!("../../../ulua_bench/workloads/ulua/fib-loop.ts")
                .replace("1000000", "1000"),
            19500,
            4_139_503_500.0,
        ),
    ] {
        let luau = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
            .unwrap()
            .luau;
        let total = instructions(&luau, expected);
        eprintln!(
            "expression {id}: instructions={total} units={units} instructions/unit={:.6}",
            total as f64 / units as f64
        );
        // A supplied baseline is measurement input only. It never controls gate assertions.
        if let Some(directory) = std::env::var_os("ULUA_EXPRESSION_BEFORE") {
            let before = std::fs::read_to_string(
                std::path::PathBuf::from(directory).join(format!("{id}.luau")),
            )
            .unwrap()
            .replace("31.0", "20.0")
            .replace("1000000.0", "1000.0");
            let total = instructions(&before, expected);
            eprintln!(
                "expression before {id}: instructions={total} units={units} instructions/unit={:.6}",
                total as f64 / units as f64
            );
        }
    }
    eprintln!(
        "expression instruction test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

// Cost: seven small compiler calls and VM traces at the 32-argument bound.
#[test]
fn call_arguments_obey_the_statement_bound() {
    let parameters = (0..32)
        .map(|i| format!("p{i}:number"))
        .collect::<Vec<_>>()
        .join(",");
    for argument in ["n", "1", "n-1", "n*2", "n/3", "n+2", "-n"] {
        let args = vec![argument; 32].join(",");
        let source = format!(
            "function f({parameters}):number{{return p0;}}export function main(b:boolean):number{{const n=b?4:3;return f({args});}}"
        );
        let code = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
            .unwrap()
            .luau;
        let expected = match argument {
            "n" => 3.0,
            "1" => 1.0,
            "n-1" => 2.0,
            "n*2" => 6.0,
            "n/3" => 1.0,
            "n+2" => 5.0,
            _ => -3.0,
        };
        instructions(&code, expected);
    }
}
