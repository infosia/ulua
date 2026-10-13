//! Every reachable stop crosses try, catch, and finally without script cleanup.
//! Measured Debug target cost: 0.04 seconds.
//! Cost is reported. This target allocates at most an eight MiB VM budget per case.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{
    ffi::{CStr, c_char, c_void},
    time::Instant,
};
use ulua::*;

unsafe extern "C" fn output(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test keeps the counter alive, and the shim supplies readable bytes.
    unsafe {
        *userdata.cast::<usize>() += 1;
        let bytes = std::slice::from_raw_parts(text.cast::<u8>(), length);
        i32::from(bytes == b"stop")
    }
}
fn run(body: &str, stop: &str, limits: RunLimits) -> (Status, String, usize) {
    let source = format!(
        "let cleanup:number=0;function recurse(n:number):number{{return recurse(n+1);}}export function read():number{{return cleanup;}}export function main():void{{{body}}}"
    );
    let mut script = support::Script::new(&source);
    let mut outputs = 0usize;
    let mut host = Host::default();
    host.userdata = std::ptr::from_mut(&mut outputs).cast();
    host.output = Some(output);
    assert_eq!(script.instance_with_host(limits, &host).0, ULUA_OK);
    let (status, d, _) = script.call_diagnostic(script.index("main"), &[]);
    // SAFETY: The ABI terminates the diagnostic code.
    let code = unsafe { CStr::from_ptr(d.code.as_ptr()).to_str().unwrap().to_owned() };
    let (read_status, _, value) = script.call(script.index("read"), &[]);
    assert_eq!(read_status, ULUA_OK, "{stop}");
    assert_eq!(value.number, 0.0, "cleanup ran: {stop} {body}");
    (status, code, outputs)
}
#[test]
fn stops_skip_all_enclosing_handlers_and_finalizers() {
    let start = Instant::now();
    let mut cases = 0;
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
        let control = run(stop, stop, limits);
        assert_eq!(control.1, code, "control {stop}");
        for block in [
            format!(
                "try{{{stop}}}catch{{cleanup++;print('catch');}}finally{{cleanup++;print('finally');}}"
            ),
            format!(
                "try{{throw new Error();}}catch{{{stop}}}finally{{cleanup++;print('finally');}}"
            ),
            format!("try{{}}finally{{{stop}}}"),
        ] {
            let body = format!(
                "try{{{block}}}catch{{cleanup++;print('outer catch');}}finally{{cleanup++;print('outer finally');}}"
            );
            let actual = run(&body, stop, limits);
            assert_eq!(actual, control, "{code}: {body}");
            cases += 1;
        }
    }
    eprintln!(
        "P2.7 stops controls=8 protected={cases} cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
