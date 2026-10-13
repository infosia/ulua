//! Phase Review boundary tests. Measured Debug cost: 0.13 seconds (round 8 review).
// This target uses only zero-argument exports.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use support::Script;
use ulua::*;
use ulua_compiler::{DEPTH_CEILING, Limits};

#[test]
fn number_literals_round_trip_through_the_luau_vm() {
    for (literal, value) in [
        ("5e-324", f64::from_bits(1)),
        ("1.7976931348623157e308", f64::MAX),
        ("1e21", 1e21),
        ("1e-7", 1e-7),
        ("-0", -0.0),
        ("9007199254740993", 9007199254740992.0),
    ] {
        let mut script = Script::new(&format!(
            "export function f():number{{return {literal};}}export function main():void{{}}"
        ));
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
        let result = script.call(script.index("f"), &[]);
        assert_eq!(result.0, ULUA_OK);
        assert_eq!(result.2.number.to_bits(), value.to_bits(), "{literal}");
    }
}

#[test]
fn allocation_work_stops_at_the_default_step_limit() {
    let source = "export function main():void{let s=\"x\";for(let i=0;i<18;i++){s=`${s}${s}`;}let t=\"\";for(let i=0;i<1000;i++){t=`${s}${s}${s}${s}`;}}";
    let mut script = Script::new(source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    let result = script.call(0, &[]);
    assert_eq!((result.0, result.1.as_str()), (ULUA_FAILED, "limit:steps"));
    // The context and instance remain usable after the exhausted operation.
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
}

#[test]
fn a_large_template_uses_linear_copy_work() {
    let source = format!(
        "export function main():void {{const s=\"{}\";const t=`{}`;print(t);}}",
        "x".repeat(16 * 1024),
        "${s}".repeat(256)
    );
    let compiled = ulua_compiler::compile(source.as_bytes(), Limits::default()).unwrap();
    assert!(compiled.luau.contains("__ulua_concat("));
    let mut script = Script::new(&source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(script.call(0, &[]).0, ULUA_OK);
}

#[test]
fn statement_and_unary_depth_grid() {
    let mut pairs = 0;
    for form in ["if(true){", "{", "while(false){", "for(;false;){"] {
        for levels in 0..=DEPTH_CEILING as usize {
            let source = |unary| {
                format!(
                    "\nexport function main():void {{{}const x={}true;{}}}",
                    form.repeat(levels),
                    "!".repeat(unary),
                    "}".repeat(levels)
                )
            };
            let classify =
                |unary| match ulua_compiler::compile(source(unary).as_bytes(), Limits::default()) {
                    Ok(_) => true,
                    Err(d) => {
                        assert_eq!(
                            d.code, "limit:nesting-depth",
                            "{form} {levels} {unary}: {}",
                            d.message
                        );
                        assert!(d.offset > 0);
                        false
                    }
                };
            if !classify(0) {
                continue;
            }
            let (mut lo, mut hi) = (0, DEPTH_CEILING as usize + 1);
            while lo + 1 < hi {
                let mid = (lo + hi) / 2;
                if classify(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let mut at = Script::new(&source(lo));
            assert_eq!(at.instance(RunLimits::default()).0, ULUA_OK);
            assert_eq!(at.call(0, &[]).0, ULUA_OK);
            Script::compile(
                &source(lo + 1),
                CompileLimits::default(),
                "limit:nesting-depth",
            );
            pairs += 1;
        }
    }
    eprintln!("S4 depth grid: {pairs} accepted/rejected boundary pairs");
}

#[cfg(feature = "measurements")]
#[test]
fn allocation_charge_boundary_and_round_up() {
    // SAFETY: The private opt-in probe owns its allocator state and frees the allocation.
    #[link(name = "ulua_test_native", kind = "static")]
    unsafe extern "C" {
        fn ulua_internal_probe_allocation(bytes: usize, steps: u64) -> i32;
    }
    for bytes in [
        1,
        ALLOCATION_STEP_BYTES as usize,
        ALLOCATION_STEP_BYTES as usize + 1,
        4097,
    ] {
        let charge = (bytes as u64).div_ceil(ALLOCATION_STEP_BYTES);
        // SAFETY: The probe accepts sizes and budgets only; no borrowed pointers cross this call.
        unsafe {
            assert_eq!(ulua_internal_probe_allocation(bytes, charge), 0);
            assert_eq!(ulua_internal_probe_allocation(bytes, charge - 1), 2);
        }
    }
}
