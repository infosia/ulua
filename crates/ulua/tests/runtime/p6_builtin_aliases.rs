//! Inspect each alias call in production bytecode. Cost: two small native compiles; no VM execution.
//! Measured Debug cost: 0.010 seconds, including the compiler and the assertion mutation.
use std::{ffi::CStr, time::Instant};

// SAFETY: The probe reads source bytes and writes the supplied line arrays and 256-byte diagnostic.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_builtin_aliases(
        source: *const u8,
        length: usize,
        fastcalls: *mut u32,
        calls: *mut u32,
        lines: usize,
        error: *mut std::ffi::c_char,
    ) -> i32;
}

fn inspect(source: &str) -> (Vec<u32>, Vec<u32>) {
    let mut fastcalls = vec![0; source.lines().count() + 1];
    let mut calls = fastcalls.clone();
    let mut error = [0; 256];
    // SAFETY: Every buffer covers its declared length and stays live through this synchronous call.
    let status = unsafe {
        ulua_internal_probe_builtin_aliases(
            source.as_ptr(),
            source.len(),
            fastcalls.as_mut_ptr(),
            calls.as_mut_ptr(),
            fastcalls.len(),
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
    (fastcalls, calls)
}

#[test]
fn builtin_alias_calls_use_fastcall_with_production_options() {
    let start = Instant::now();
    let source =
        "export function main(x:number):void{const a=[1,2];a[0]=x%3;print(`${a[0]}:${true}`);}";
    let luau = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
        .unwrap()
        .luau;
    let (body, exports) = luau.rsplit_once("\nreturn ").unwrap();
    let luau = format!(
        r#"{body}
local function alias_witness(x: number): ()
    __ulua_print(x)
    local f = __ulua_floor(x)
    local n = __ulua_fmod(f, 3)
    local s = __ulua_tostring(n)
    __ulua_gsub(s, "x", "y")
end
return {{module = {exports}, witness = alias_witness}}
"#
    );
    let (fastcalls, calls) = inspect(&luau);
    let aliases = [
        ("fmod", 1),
        ("tostring", 2),
        ("floor", 4),
        ("print", 0),
        ("gsub", 0),
    ];
    let mut failures = Vec::new();
    for (alias, builtin) in aliases {
        let needle = format!("__ulua_{alias}(");
        let mut sites = 0;
        for (index, line) in luau.lines().enumerate() {
            if line.contains(&needle) {
                sites += 1;
                if builtin != 0 && fastcalls[index + 1] & builtin == 0 {
                    failures.push(format!("{alias} at line {} lacks FASTCALL", index + 1));
                }
                // print and gsub have no FASTCALL in the pinned VM. Their calls must remain visible.
                assert!(calls[index + 1] > 0, "missing CALL fallback: {line}");
                if builtin == 0 {
                    let nested = if line.contains("__ulua_tostring(") {
                        2
                    } else {
                        0
                    };
                    assert_eq!(fastcalls[index + 1] & !nested, 0, "{line}");
                }
            }
        }
        eprintln!("alias {alias}: sites={sites}, FASTCALL bit={builtin}");
        if sites == 0 {
            failures.push(format!("{alias}: no call site"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");

    // The violating form must defeat the bytecode check even when the source still names each alias.
    let mut cast = luau.clone();
    for (alias, library) in [
        ("fmod", "math.fmod"),
        ("tostring", "tostring"),
        ("floor", "math.floor"),
    ] {
        cast = cast.replace(
            &format!("local __ulua_{alias}: typeof({library}) = {library}"),
            &format!("local __ulua_{alias}: typeof({library}) = ({library}) :: typeof({library})"),
        );
    }
    let (cast_fastcalls, _) = inspect(&cast);
    assert!(cast_fastcalls.iter().all(|bits| bits & 7 == 0));
    eprintln!(
        "builtin alias test cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
