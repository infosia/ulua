//! Stops in resumed activations skip handlers, finalizers, and later jobs.
//! Costs are reported; each VM uses at most eight MiB per fixture.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{
    ffi::{CStr, c_char, c_void},
    time::Instant,
};
use ulua::*;
unsafe extern "C" fn output(data: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The fixture owns the counter; the shim supplies readable bytes.
    unsafe {
        *data.cast::<usize>() += 1;
        i32::from(std::slice::from_raw_parts(text.cast::<u8>(), length) == b"stop")
    }
}
fn run(stop: &str, protected: bool, limits: RunLimits) -> (Status, String, usize) {
    let action = if protected {
        format!(
            "try{{{stop}}}catch{{cleanup++;print('catch');}}finally{{cleanup++;print('finally');}}"
        )
    } else {
        stop.to_owned()
    };
    let source = format!(
        "let cleanup:number=0;async function tick():Promise<number>{{return 1;}}function recurse(n:number):number{{return recurse(n+1);}}async function later():Promise<number>{{await tick();cleanup++;print('later');return 0;}}async function task():Promise<number>{{await tick();{action}return 0;}}export function read():number{{return cleanup;}}export async function main():Promise<void>{{const p=task();const q=later();try{{await p;await q;}}catch{{cleanup++;}}finally{{cleanup++;}}}}"
    );
    let mut script = support::Script::new(&source);
    let mut outputs = 0usize;
    let mut host = Host::default();
    host.userdata = std::ptr::from_mut(&mut outputs).cast();
    host.output = Some(output);
    assert_eq!(script.instance_with_host(limits, &host).0, ULUA_OK);
    let (status, d, _) = script.call_diagnostic(script.index("main"), &[]);
    // SAFETY: The ABI terminates the diagnostic array.
    let code = unsafe { CStr::from_ptr(d.code.as_ptr()).to_str().unwrap().to_owned() };
    let read = script.call(script.index("read"), &[]);
    assert_eq!(read.0, ULUA_OK, "{stop}");
    assert_eq!(read.2.number, 0.0, "{stop}");
    (status, code, outputs)
}
#[test]
fn resumed_stops_keep_codes_and_instance() {
    let start = Instant::now();
    for (code, body) in [
        (
            "trap:index-out-of-range",
            "const xs:number[]=[];print(`${xs[0]}`);",
        ),
        (
            "trap:array-modified",
            "const xs:number[]=[1];xs.forEach((n:number):void=>{xs.push(n);});",
        ),
        ("trap:string-boundary", "print('é'.slice(1));"),
        ("trap:invalid-argument", "print('x'.repeat(-1));"),
        ("limit:memory", "let s:string='ab';while(true){s=s+s;}"),
        ("limit:steps", "while(true){}"),
        ("limit:stack", "recurse(0);"),
        ("host:output", "print('stop');"),
    ] {
        let mut limits = RunLimits::default();
        limits.max_memory_bytes = 8 * 1024 * 1024;
        if code == "limit:steps" {
            limits.max_steps = 20000;
        }
        let control = run(body, false, limits);
        assert_eq!(control.1, code);
        assert_eq!(run(body, true, limits), control);
    }
    eprintln!(
        "P4.1 resumed stops cases=16 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn immediate_starts_reach_c_resume_limit() {
    let mut script = support::Script::new(
        "async function f(n:number):Promise<number>{if(n===0){return 42;}return f(n-1);}export async function depth(n:number):Promise<number>{return f(n);}export function main():void{}export function good():number{return 42;}",
    );
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    let mut boundary = 0;
    for depth in 1..230 {
        let (status, code, value) = script.call(script.index("depth"), &[support::number(depth)]);
        if status == ULUA_FAILED {
            assert_eq!(code, "limit:stack");
            boundary = depth;
            break;
        }
        assert_eq!(value.number, 42.0);
    }
    assert!(boundary > 0);
    assert_eq!(
        script
            .call(script.index("depth"), &[support::number(boundary - 1)])
            .0,
        ULUA_OK
    );
    assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
    eprintln!(
        "P4.1 immediate resume depth success={} stop={boundary}",
        boundary - 1
    );
}

fn callback_stop(kind: &str, stop: &str, limits: RunLimits) -> (Status, String, usize) {
    let action = format!(
        "try{{{stop}}}catch{{cleanup++;print('catch');}}finally{{cleanup++;print('finally');}}"
    );
    let chained = ".catch(():number=>{cleanup++;print('catch job');return 0;}).finally(():void=>{cleanup++;print('finally job');})";
    let body = match kind {
        "then" => format!(
            "const p=tick().then(():number=>{{{action}return 0;}}){chained};const q=later();await p;await q;"
        ),
        "catch" => format!(
            "const p=fail().catch((e:unknown):number=>{{{action}return 0;}}){chained};const q=later();await p;await q;"
        ),
        "finally" => format!(
            "const p=tick().finally(():void=>{{{action}}}){chained};const q=later();await p;await q;"
        ),
        "map" => format!(
            "const q=later();const ps=[1].map(async(n:number):Promise<number>=>{{{action}return n;}});await Promise.all(ps);await q;"
        ),
        _ => stop.to_owned(),
    };
    let source = format!(
        "let cleanup:number=0;async function tick():Promise<number>{{return 1;}}async function fail():Promise<number>{{throw new Error();}}function recurse(n:number):number{{return recurse(n+1);}}function later():Promise<number>{{return tick().then(():number=>{{cleanup++;print('later');return 0;}});}}export function read():number{{return cleanup;}}export async function main():Promise<void>{{{body}}}"
    );
    let mut script = support::Script::new(&source);
    let mut outputs = 0usize;
    let mut host = Host::default();
    host.userdata = std::ptr::from_mut(&mut outputs).cast();
    host.output = Some(output);
    assert_eq!(script.instance_with_host(limits, &host).0, ULUA_OK);
    let (status, d, _) = script.call_diagnostic(script.index("main"), &[]);
    // SAFETY: The ABI terminates the diagnostic code array.
    let code = unsafe { CStr::from_ptr(d.code.as_ptr()).to_str().unwrap().to_owned() };
    for _ in 0..2 {
        let read = script.call(script.index("read"), &[]);
        assert_eq!((read.0, read.2.number), (ULUA_OK, 0.0), "{kind}: {stop}");
    }
    (status, code, outputs)
}
#[test]
fn callback_stops_keep_control_codes_and_clear_later_jobs() {
    let start = Instant::now();
    for (code, stop) in [
        (
            "trap:index-out-of-range",
            "const xs:number[]=[];print(`${xs[0]}`);",
        ),
        (
            "trap:array-modified",
            "const xs:number[]=[1];xs.forEach((n:number):void=>{xs.push(n);});",
        ),
        ("trap:string-boundary", "print('é'.slice(1));"),
        ("trap:invalid-argument", "print('x'.repeat(-1));"),
        ("limit:memory", "let s:string='ab';while(true){s=s+s;}"),
        ("limit:steps", "while(true){}"),
        ("limit:stack", "recurse(0);"),
        ("host:output", "print('stop');"),
    ] {
        let mut limits = RunLimits::default();
        limits.max_memory_bytes = 8 * 1024 * 1024;
        if code == "limit:steps" {
            limits.max_steps = 20000;
        }
        let control = callback_stop("control", stop, limits);
        assert_eq!(control.1, code);
        for kind in ["then", "catch", "finally", "map"] {
            assert_eq!(callback_stop(kind, stop, limits), control, "{kind}: {code}");
        }
    }
    eprintln!(
        "P4.2 callback stops controls=8 callbacks=32 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
