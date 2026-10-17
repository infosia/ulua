#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use ulua::{RunLimits, ULUA_FAILED, ULUA_OK};

#[test]
fn string_scalar_traps_cross_the_native_boundary() {
    for expression in [
        "String.fromCodePoint(-1)",
        "String.fromCodePoint(1.5)",
        "String.fromCodePoint(NaN)",
        "String.fromCodePoint(Infinity)",
        "String.fromCodePoint(-Infinity)",
        "String.fromCodePoint(1114112)",
        "String.fromCodePoint(55296)",
        "String.fromCodePoint(57343)",
        "String.fromCharCode(55296)",
        "String.fromCharCode(57343)",
        "String.fromCharCode(120832)",
        "String.fromCharCode(-10240)",
    ] {
        let mut script = support::Script::new(&format!(
            "const s={expression};export function main():void{{}}"
        ));
        assert_eq!(
            script.instance(RunLimits::default()),
            (ULUA_FAILED, "trap:invalid-argument".into()),
            "{expression}"
        );
    }
}
#[test]
fn string_case_expansion_context_and_uint16() {
    for (expression, expected) in [
        ("'Straße İ ﬀ ǅ'.toUpperCase()", "STRASSE İ FF Ǆ"),
        ("'ΟΣ ΟΣΑ İ'.toLowerCase()", "ος οσα i\u{307}"),
        ("''.toUpperCase()", ""),
        ("''.toLowerCase()", ""),
        ("String.fromCodePoint(128512)", "😀"),
        ("String.fromCodePoint(-0)", "\0"),
        ("String.fromCharCode(NaN)", "\0"),
        ("String.fromCharCode(Infinity)", "\0"),
        ("String.fromCharCode(-Infinity)", "\0"),
        ("String.fromCharCode(-1.9)", "\u{ffff}"),
        ("String.fromCharCode(65537.9)", "\u{1}"),
    ] {
        let mut script = support::Script::new(&format!(
            "export function main():string{{return {expression};}}"
        ));
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
        let (status, code, value) = script.call(script.index("main"), &[]);
        assert_eq!((status, code), (ULUA_OK, String::new()));
        // SAFETY: The instance owns the returned string until its next call.
        let actual = unsafe { std::slice::from_raw_parts(value.data.cast::<u8>(), value.length) };
        assert_eq!(actual, expected.as_bytes(), "{expression}");
    }
}
#[test]
fn nullable_case_receiver_uses_optional_method_path() {
    let mut script = support::Script::new(
        "export function main():void{const s:string|null=null;s?.toUpperCase();s?.toLowerCase();const t:string|null='Straße';t?.toUpperCase();t?.toLowerCase();}",
    );
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(script.call(script.index("main"), &[]).0, ULUA_OK);
}
#[test]
fn case_result_and_scratch_respect_memory_limit() {
    // A retained case expansion cannot allocate outside the VM memory limit.
    let mut control = support::Script::new(
        "export function main():string{let s='ΐ';for(let i=0;i<18;i++){s=s+s;}return s;}",
    );
    let mut control_limits = RunLimits::default();
    control_limits.max_memory_bytes = 2 * 1024 * 1024;
    assert_eq!(control.instance(control_limits).0, ULUA_OK);
    assert_eq!(control.call(control.index("main"), &[]).0, ULUA_OK);
    let mut script = support::Script::new(
        "export function main():string{let s='ΐ';for(let i=0;i<18;i++){s=s+s;}return s.toUpperCase();}",
    );
    let mut limits = RunLimits::default();
    limits.max_memory_bytes = 2 * 1024 * 1024;
    assert_eq!(script.instance(limits).0, ULUA_OK);
    let (status, code, _) = script.call(script.index("main"), &[]);
    assert_eq!((status, code), (ULUA_FAILED, "limit:memory".into()));
}
#[test]
#[ignore = "explicit Release P7.5 call cost measurement"]
fn native_case_call_cost() {
    assert!(
        std::hint::black_box(!cfg!(debug_assertions)),
        "run with --release"
    );
    let _lock = crate::measurement_lock();
    const CALLS: usize = 200_000;
    for input in ["Hello World", "Straße ΟΣ İ ﬀ"] {
        for member in ["toUpperCase", "toLowerCase"] {
            let mut script = support::Script::new(&format!(
                "export function main():number{{const s='{input}';let n=0;for(let i=0;i<{CALLS};i++){{n=n+s.{member}().length;}}return n;}}"
            ));
            let mut limits = RunLimits::default();
            limits.max_steps = 100_000_000;
            assert_eq!(script.instance(limits).0, ULUA_OK);
            let index = script.index("main");
            assert_eq!(script.call(index, &[]).0, ULUA_OK);
            let mut samples = vec![];
            for _ in 0..5 {
                let now = std::time::Instant::now();
                let (status, _, value) = script.call(index, &[]);
                assert_eq!(status, ULUA_OK);
                std::hint::black_box(value.number);
                samples.push(now.elapsed().as_secs_f64() * 1e9 / CALLS as f64);
            }
            samples.sort_by(f64::total_cmp);
            eprintln!(
                "case {input:?}.{member}: {:.3} ns/call; median of 5 x {CALLS}, loop and result length included",
                samples[2]
            );
        }
    }
}
