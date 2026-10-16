//! HEAD observations pin output and trap kind. Cost: two small compiles and 39 short ABI calls.
//! Measured Release cost: 0.006 seconds, including compilation and all output comparisons.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{
    ffi::{c_char, c_void},
    ptr,
    time::Instant,
};
use ulua::*;

unsafe extern "C" fn output(data: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test keeps the vector live; the ABI supplies readable bytes for this callback.
    unsafe {
        (*data.cast::<Vec<String>>()).push(
            String::from_utf8_lossy(std::slice::from_raw_parts(text.cast(), length)).into_owned(),
        );
    }
    0
}

#[test]
fn index_edges_keep_head_output_and_trap_kind() {
    let start = Instant::now();
    let source = r#"
function rhs():number{print('rhs');return 30;}
function side():number{print('index');return 0;}
export function read(i:number):void{const a=[10,20];print('before');print(`${a[i]}`);print('after');}
export function set(i:number):void{const a=[10,20];print('before');a[i]=rhs();print(`${a.length}:${a[0]}:${a[1]}:${a[a.length-1]}`);print('after');}
export function deferred(i:number):void{const a=[[10],[20]];print('before');print(`${a[i][side()]}`);print('after');}
export function main():void{}
"#;
    let luau = ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
        .unwrap()
        .luau;
    assert!(
        luau.lines()
            .any(|line| line.contains("__ulua_get(") && line.contains(", true)"))
    );
    let mut script = support::Script::new(source);
    let mut lines = Vec::<String>::new();
    let mut host = Host::default();
    host.userdata = ptr::from_mut(&mut lines).cast();
    host.output = Some(output);
    assert_eq!(
        script.instance_with_host(RunLimits::default(), &host).0,
        ULUA_OK
    );
    // The array length is two. The booleans and values are independent, hand-checked expectations.
    let cases = [
        ("-1", -1.0, None, None),
        ("-0", -0.0, Some("10"), Some("2:30:20:20")),
        ("0", 0.0, Some("10"), Some("2:30:20:20")),
        ("#a-1", 1.0, Some("20"), Some("2:10:30:30")),
        ("#a", 2.0, None, Some("3:10:20:30")),
        ("#a+1", 3.0, None, None),
        ("0.5", 0.5, None, None),
        ("below-one", 0.9999999999999999, None, None),
        ("negative-epsilon", -1e-17, None, None),
        ("NaN", f64::NAN, None, None),
        ("Infinity", f64::INFINITY, None, None),
        ("-Infinity", f64::NEG_INFINITY, None, None),
        ("2^53", 9007199254740992.0, None, None),
    ];
    for (name, number, read, set) in cases {
        for (operation, expected) in [("read", read), ("set", set), ("deferred", read)] {
            lines.clear();
            let mut argument = Value::default();
            argument.r#type = ULUA_TYPE_NUMBER;
            argument.number = number;
            let (status, code, _) = script.call(script.index(operation), &[argument]);
            let mut expected_lines = vec!["before"];
            if operation == "set" {
                expected_lines.push("rhs");
            }
            if operation == "deferred" {
                expected_lines.push("index");
            }
            let (expected_status, expected_code) = if let Some(value) = expected {
                expected_lines.extend([value, "after"]);
                (ULUA_OK, "")
            } else {
                (ULUA_FAILED, "trap:index-out-of-range")
            };
            assert_eq!(
                (status, code.as_str()),
                (expected_status, expected_code),
                "{operation} {name}"
            );
            assert_eq!(lines, expected_lines, "{operation} {name}");
            eprintln!("index {operation} {name}: {code:?} {lines:?}");
        }
    }
    eprintln!("index edge test cost={:.3}s", start.elapsed().as_secs_f64());
}
