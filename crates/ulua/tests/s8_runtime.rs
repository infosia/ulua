//! Round 8 runtime errors cross the public ABI and preserve the instance after each limit.
//! Large string cases require measurements: each case can allocate more than 1 GiB.
//! Measured Debug cost: 0.03 seconds default; 0.80 seconds for the two opt-in string cases.
//! The default gate uses ABI budgets and depth, plus a private fixture for other VM errors.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
use std::ffi::CStr;
use support::{Script, number};
use ulua::*;

// SAFETY: The test-only entry owns its compiler outputs and VM and returns only a status.
#[link(name = "ulua_test_native", kind = "static")]
unsafe extern "C" {
    fn ulua_internal_probe_runtime_errors() -> i32;
}

#[test]
fn other_vm_limits_and_unrelated_errors_use_the_production_shim() {
    assert_eq!(ulua_abi_version(), 1);
    // SAFETY: The fixture takes no pointers and releases each VM before it returns.
    assert_eq!(unsafe { ulua_internal_probe_runtime_errors() }, 0);
}

fn text(bytes: &[std::ffi::c_char]) -> &str {
    // SAFETY: The diagnostic arrays contain NUL-terminated ASCII for these VM errors.
    unsafe { CStr::from_ptr(bytes.as_ptr()).to_str().unwrap() }
}

fn limit(source: &str, limits: RunLimits, code: &str, message: &str) {
    let mut script = Script::new(source);
    assert_eq!(script.instance(limits).0, ULUA_OK);
    let (status, diagnostic, _) = script.call_diagnostic(script.index("grow"), &[number(0)]);
    assert_eq!(status, ULUA_FAILED, "{}", text(&diagnostic.message));
    assert_eq!(text(&diagnostic.code), code);
    assert_eq!(text(&diagnostic.message), message);
    assert_eq!(diagnostic.offset, u64::MAX);
    assert_eq!((diagnostic.line, diagnostic.column), (0, 0));
    // This call uses the same instance and context after the VM unwinds and collects.
    assert_eq!(script.call(script.index("main"), &[]).0, ULUA_OK);
}

#[test]
fn budget_and_depth_errors_have_no_generated_location() {
    limit(
        "export function main():void{}export function grow(n:number):number{let s=\"ab\";while(true){s=`${s}${s}`;}return n;}",
        RunLimits::default(),
        "limit:memory",
        "not enough memory",
    );
    limit(
        "export function main():void{}export function grow(n:number):number{return grow(n+1);}",
        RunLimits::default(),
        "limit:stack",
        "stack overflow",
    );
}

#[cfg(feature = "measurements")]
#[test]
fn vm_string_size_limits_recover_with_large_host_budgets() {
    let start = std::time::Instant::now();
    let mut limits = RunLimits::default();
    limits.max_memory_bytes = 2 * 1024 * 1024 * 1024;
    limits.max_steps = 10_000_000;
    limit(
        "export function main():void{}export function grow(n:number):number{let s=\"ab\";let k=0;while(true){s=`${s}${s}`;k++;if(k>60){break;}}return k;}",
        limits,
        "limit:memory",
        "string length overflow",
    );
    limits.max_memory_bytes *= 2;
    limits.max_steps = 100_000_000;
    limit(
        "export function main():void{}export function grow(n:number):number{let s=\"ab\";let k=0;while(true){s=`${s}${s}${s}${s}${s}${s}${s}${s}`;k++;if(k>60){break;}}return k;}",
        limits,
        "limit:memory",
        "memory allocation error: block too big",
    );
    eprintln!("S8 large string cases: {:?}", start.elapsed());
}
