//! Direct ABI tests exercise the product helpers, independent of the emitted source.
use crate::{tests::Fixture, *};
use std::ptr;
fn value(n: f64) -> Value {
    Value {
        r#type: ULUA_TYPE_NUMBER,
        number: n,
        ..Value::default()
    }
}
fn call(instance: *mut Instance, index: u32, args: &[Value]) -> f64 {
    let mut result = Value::default();
    let mut d = Diagnostic::default();
    // SAFETY: The fixture owns the instance and all argument and output storage.
    unsafe {
        assert_eq!(
            ulua_call(
                instance,
                index,
                args.as_ptr(),
                args.len(),
                &mut result,
                &mut d
            ),
            ULUA_OK
        );
    }
    result.number
}
fn equal(actual: f64, expected: f64) {
    if expected.is_nan() {
        assert!(actual.is_nan(), "{actual:?}");
    } else {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "{actual:?} != {expected:?}"
        );
    }
}
fn unary_check(name: &str, reference: impl Fn(f64) -> f64) {
    let f = Fixture::source(
        format!("export function main(x:number):number{{return Math.{name}(x);}}").as_bytes(),
    );
    let instance = f.instance(&Host::default());
    for &bits in oracle::UNARY {
        let x = f64::from_bits(bits);
        equal(call(instance, 0, &[value(x)]), reference(x));
    }
}
fn binary_check(name: &str, reference: impl Fn(f64, f64) -> f64) {
    let f = Fixture::source(
        format!("export function main(x:number,y:number):number{{return Math.{name}(x,y);}}")
            .as_bytes(),
    );
    let instance = f.instance(&Host::default());
    for &a in oracle::BINARY {
        for &b in oracle::BINARY {
            let (a, b) = (f64::from_bits(a), f64::from_bits(b));
            equal(call(instance, 0, &[value(a), value(b)]), reference(a, b));
        }
    }
}
macro_rules! unary {
    ($name:ident, [$($bits:expr),* $(,)?]) => {
        #[test] fn $name() {
            let expected: &[u64] = &[$($bits),*];
            let mut i = 0;
            let f = Fixture::source(concat!("export function main(x:number):number{return Math.",stringify!($name),"(x);}").as_bytes());
            let instance = f.instance(&Host::default());
            for (&x,&expected) in UNARY.iter().zip(expected) {
                equal(call(instance, 0, &[value(f64::from_bits(x))]), f64::from_bits(expected)); i+=1;
            }
            assert_eq!(i, UNARY.len());
        }
    };
}
macro_rules! binary {
    ($name:ident, [$($bits:expr),* $(,)?]) => {
        #[test] fn $name() {
            let expected: &[u64] = &[$($bits),*];
            let f = Fixture::source(concat!("export function main(x:number,y:number):number{return Math.",stringify!($name),"(x,y);}").as_bytes());
            let instance = f.instance(&Host::default());
            assert_eq!(expected.len(), BINARY.len()*BINARY.len());
            for (i, &a) in BINARY.iter().enumerate() { for (j, &b) in BINARY.iter().enumerate() {
                equal(call(instance, 0, &[value(f64::from_bits(a)),value(f64::from_bits(b))]), f64::from_bits(expected[i*BINARY.len()+j]));
            }}
        }
    };
}
mod oracle {
    use super::*;
    include!("math_oracle.rs");
}
macro_rules! libm_unary {
    ($($name:ident),*) => {$ (#[test] fn $name() { unary_check(stringify!($name), libm::$name); })*};
}
libm_unary!(
    acos, acosh, asin, asinh, atan, atanh, cbrt, cos, cosh, exp, expm1, log, log1p, log10, log2,
    sin, sinh, tan, tanh
);
#[test]
fn atan2() {
    binary_check("atan2", libm::atan2);
}
#[test]
fn hypot() {
    binary_check("hypot", libm::hypot);
}
#[test]
fn pow() {
    binary_check("pow", |a, b| {
        if b.is_nan() || (a.abs() == 1.0 && b.is_infinite()) {
            f64::NAN
        } else {
            libm::pow(a, b)
        }
    });
}
#[test]
fn seed_abi_default_set_reseed_and_instance_isolation() {
    let f = Fixture::source(b"const initial=Math.random();export function initialValue():number{return initial;}export function next():number{return Math.random();}");
    let first = f.instance(&Host::default());
    let second = f.instance(&Host::default());
    let expected = [
        0.7085450778517304,
        0.7056218027099065,
        0.990983775174846,
        0.11117172788539775,
        0.15235247969973897,
    ];
    for instance in [first, second] {
        equal(call(instance, 0, &[]), expected[0]);
        for &n in &expected[1..] {
            equal(call(instance, 1, &[]), n);
        }
    }
    let host = Host {
        random_seed_set: 1,
        random_seed: 42,
        ..Host::default()
    };
    let seeded = f.instance(&host);
    let initial = call(seeded, 0, &[]);
    assert_ne!(initial, expected[0]);
    // SAFETY: All three instances belong to this live fixture and calls are serialized.
    unsafe {
        assert_eq!(ulua_instance_seed_random(first, 42), ULUA_OK);
        assert_eq!(ulua_instance_seed_random(second, 42), ULUA_OK);
        equal(call(first, 1, &[]), initial);
        equal(call(second, 1, &[]), initial);
        for _ in 0..8 {
            equal(call(first, 1, &[]), call(seeded, 1, &[]));
        }
        assert_eq!(ulua_instance_seed_random(first, 42), ULUA_OK);
        equal(call(first, 1, &[]), initial);
        assert_eq!(
            ulua_instance_seed_random(ptr::null_mut(), 42),
            ULUA_INVALID_ARGUMENT
        );
    }
}
#[test]
fn seed_abi_invalid_host_fields() {
    let f = Fixture::new();
    for (set, reserved) in [(2, 0), (u32::MAX, 0), (0, 1), (1, u32::MAX)] {
        let host = Host {
            random_seed_set: set,
            reserved2: reserved,
            ..Host::default()
        };
        let mut instance = ptr::null_mut();
        // SAFETY: Descriptors and output storage are live; only validated scalar fields are invalid.
        unsafe {
            assert_eq!(
                ulua_instance_new(
                    f.context,
                    f.module,
                    &host,
                    ptr::null(),
                    &mut instance,
                    ptr::null_mut()
                ),
                ULUA_INVALID_ARGUMENT
            );
        }
        assert!(instance.is_null());
    }
}
#[test]
fn random_reference_seeds_cover_zero_and_full_width() {
    // Independent BigInt reference implements the unsigned splitmix64 and xoshiro256++ equations.
    let rows = [
        (
            0,
            [
                0.3245752680314067,
                0.38223929651167343,
                0.3596172076473553,
                0.011455508934653635,
                0.49527006868383106,
            ],
        ),
        (
            42,
            [
                0.8143051451229099,
                0.3188210400616611,
                0.9838941681774888,
                0.7011355981347556,
                0.793504489691729,
            ],
        ),
        (
            u64::MAX,
            [
                0.33906512301887703,
                0.9004750408188128,
                0.8902848745939088,
                0.2736678890261809,
                0.6556110533225108,
            ],
        ),
    ];
    let f = Fixture::source(b"export function main():number{return Math.random();}");
    for (seed, expected) in rows {
        let host = Host {
            random_seed_set: 1,
            random_seed: seed,
            ..Host::default()
        };
        let instance = f.instance(&host);
        for n in expected {
            equal(call(instance, 0, &[]), n);
        }
    }
    let ignored = f.instance(&Host {
        random_seed: 42,
        ..Host::default()
    });
    equal(call(ignored, 0, &[]), 0.7085450778517304);
}
#[test]
fn constants_have_ecmascript_bits() {
    for (name, bits) in [
        ("E", 0x4005bf0a8b145769),
        ("LN2", 0x3fe62e42fefa39ef),
        ("LN10", 0x40026bb1bbb55516),
        ("LOG2E", 0x3ff71547652b82fe),
        ("LOG10E", 0x3fdbcb7b1526e50e),
        ("PI", 0x400921fb54442d18),
        ("SQRT1_2", 0x3fe6a09e667f3bcd),
        ("SQRT2", 0x3ff6a09e667f3bcd),
    ] {
        let f = Fixture::source(
            format!("export function main():number{{return Math.{name};}}").as_bytes(),
        );
        equal(
            call(f.instance(&Host::default()), 0, &[]),
            f64::from_bits(bits),
        );
    }
}
#[test]
fn seed_abi_reentry_returns_busy() {
    unsafe extern "C" fn output(
        data: *mut std::ffi::c_void,
        _: *const std::ffi::c_char,
        _: usize,
    ) -> i32 {
        // SAFETY: The test keeps this pair live until the callback returns.
        unsafe {
            let pair = &mut *data.cast::<(*mut Instance, Status)>();
            pair.1 = ulua_instance_seed_random(pair.0, 42);
        }
        0
    }
    let f = Fixture::source(b"export function main():void{print('seed');}");
    let mut pair = (ptr::null_mut(), ULUA_INTERNAL);
    let host = Host {
        output: Some(output),
        userdata: ptr::from_mut(&mut pair).cast(),
        ..Host::default()
    };
    pair.0 = f.instance(&host);
    call(pair.0, 0, &[]);
    assert_eq!(pair.1, ULUA_BUSY);
}
#[test]
fn pow_ecmascript_edges_have_independent_values() {
    let f =
        Fixture::source(b"export function main(a:number,b:number):number{return Math.pow(a,b);}");
    let instance = f.instance(&Host::default());
    for (a, b, expected) in [
        (1.0, f64::NAN, f64::NAN),
        (-1.0, f64::NAN, f64::NAN),
        (1.0, f64::INFINITY, f64::NAN),
        (-1.0, f64::NEG_INFINITY, f64::NAN),
        (f64::NAN, 0.0, 1.0),
        (f64::NAN, -0.0, 1.0),
        (-0.0, 3.0, -0.0),
        (-0.0, -3.0, f64::NEG_INFINITY),
        (-0.0, 2.0, 0.0),
        (-1.0, 0.5, f64::NAN),
        (-2.0, 3.0, -8.0),
    ] {
        equal(call(instance, 0, &[value(a), value(b)]), expected);
    }
}

// SAFETY: Both probes write only the supplied four counters and 256-byte diagnostic.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_native_call_steps() -> u64;
    fn ulua_internal_probe_math_charge(
        budget: u64,
        random: i32,
        dispatch: extern "C" fn(u32, f64, f64) -> f64,
        counters: *mut u64,
        error: *mut std::ffi::c_char,
    ) -> i32;
}
#[test]
fn native_math_charge_stops_at_predicted_call_before_dispatch() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static DISPATCHES: AtomicU64 = AtomicU64::new(0);
    extern "C" fn counted_dispatch(op: u32, a: f64, b: f64) -> f64 {
        DISPATCHES.fetch_add(1, Ordering::Relaxed);
        crate::math::dispatch(op, a, b)
    }
    // SAFETY: The getter returns a fixed constant with no pointers or resource access.
    let charge = unsafe { ulua_internal_probe_native_call_steps() };
    assert!([1, 2, 4, 8].contains(&charge));
    // Each successful helper consumes one CALL interrupt plus the extra native charge.
    // After two calls, only the third CALL interrupt fits; its native charge must stop.
    let budget = 2 * (1 + charge) + 1;
    for random in [0, 1] {
        DISPATCHES.store(0, Ordering::Relaxed);
        let mut counters = [0; 4];
        let mut error = [0; 256];
        // SAFETY: Buffers cover their declared sizes; the static Rust dispatcher calls no VM API.
        let code = unsafe {
            ulua_internal_probe_math_charge(
                budget,
                random,
                counted_dispatch,
                counters.as_mut_ptr(),
                error.as_mut_ptr(),
            )
        };
        assert_eq!(code, 2, "SHIM_STEPS");
        assert_eq!(
            DISPATCHES.load(Ordering::Relaxed),
            if random == 0 { 2 } else { 0 }
        );
        assert_eq!(
            counters,
            [budget, 3, 0, 2],
            "steps, VM interrupts, interrupt refusals, completed helpers"
        );
        let mut diagnostic = Diagnostic::default();
        // SAFETY: The translator writes this live, initialized diagnostic descriptor.
        let status = unsafe { crate::objects::finish(code, &error, &mut diagnostic) };
        assert_eq!(status, ULUA_FAILED);
        // SAFETY: The ABI translator terminates the fixed diagnostic arrays.
        unsafe {
            assert_eq!(
                std::ffi::CStr::from_ptr(diagnostic.code.as_ptr()).to_bytes(),
                b"limit:steps"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(diagnostic.message.as_ptr()).to_bytes(),
                b"Step limit exceeded"
            );
        }
    }
}
#[test]
fn native_math_charge_exact_budget_and_interrupt_boundaries() {
    // SAFETY: The getter returns a fixed constant without resource access.
    let charge = unsafe { ulua_internal_probe_native_call_steps() };
    for random in [0, 1] {
        for (budget, status, expected) in [
            (0, 2, [0, 1, 1, 0]),
            (1, 2, [1, 1, 0, 0]),
            (2 * (1 + charge), 2, [2 * (1 + charge), 3, 1, 2]),
            // After the last native charge, the Lua return needs one interrupt.
            (3 * (1 + charge), 2, [3 * (1 + charge), 4, 1, 3]),
            (3 * (1 + charge) + 1, 0, [3 * (1 + charge) + 1, 4, 0, 3]),
        ] {
            let mut counters = [0; 4];
            let mut error = [0; 256];
            // SAFETY: All buffers cover the native probe's lengths and remain live through its return.
            let actual = unsafe {
                ulua_internal_probe_math_charge(
                    budget,
                    random,
                    crate::math::dispatch,
                    counters.as_mut_ptr(),
                    error.as_mut_ptr(),
                )
            };
            assert_eq!(actual, status, "budget {budget}, random {random}");
            assert_eq!(counters, expected, "budget {budget}, random {random}");
        }
    }
}
