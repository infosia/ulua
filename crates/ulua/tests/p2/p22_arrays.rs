//! Array order, trap recovery, and iteration through the ABI. Cost: below one second.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
};
use support::Script;
use ulua::*;

fn result(body: &str, expected: f64) {
    let source = format!("export function score():number{{{body}}}export function main():void{{}}");
    source_result(&source, expected);
}

fn source_result(source: &str, expected: f64) {
    let mut script = Script::new(source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    let (status, code, value) = script.call(script.index("score"), &[]);
    assert_eq!((status, code), (ULUA_OK, String::new()));
    assert_eq!(value.number, expected);
    if let Ok(directory) = std::env::var("ULUA_P22_OUTPUT_DIR") {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hash);
        let path = std::path::Path::new(&directory);
        std::fs::create_dir_all(path).unwrap();
        let source = source.replace(
            "export function main():void{}",
            "export function main():void{print(`${score()}`);}",
        );
        let compiled =
            ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default()).unwrap();
        let name = format!("fixture-{}", hash.finish());
        std::fs::write(path.join(format!("{name}.ts")), source).unwrap();
        std::fs::write(path.join(format!("{name}.luau")), compiled.luau).unwrap();
        std::fs::write(path.join(format!("{name}.golden")), format!("{expected}\n")).unwrap();
    }
}

#[test]
fn wide_literals_keep_constants_reads_and_side_effects_in_source_order() {
    let entries = vec!["7,n,next()"; 150].join(",");
    result(
        &format!(
            "let n=0;function next():number{{n++;return n;}}const xs:number[]=[{entries}];let total=0;for(const x of xs){{total+=x;}}return total;"
        ),
        23550.0,
    );
    result(
        &format!(
            "let n=0;function next():number{{n++;return n;}}const make=():number[]=>[{entries}];let total=0;for(let i=0;i<3;i++){{n=0;const xs=[make(),make()];for(const row of xs){{for(const x of row){{total+=x;}}}}}}return total;"
        ),
        276300.0,
    );
    source_result(
        &format!(
            "let n=0;function next():number{{n++;return n;}}const xs:number[]=[{entries}];export function score():number{{let total=0;for(const x of xs){{total+=x;}}return total;}}export function main():void{{}}"
        ),
        23550.0,
    );
    result("const xs=[-0,-1,true?2:3];return 1/xs[0]<0?1:0;", 1.0);
}

#[test]
fn reads_writes_and_push_freeze_receivers_indices_values_and_lengths() {
    // Captured bindings change during the RHS; the write keeps both earlier values.
    result(
        "let xs=[10,20];const old=xs;let i=0;function rhs():number{xs=[50];i=1;return 3;}xs[i]+=rhs();return old[0]*100+old[1]*10+xs[0]+i;",
        1551.0,
    );
    // The index call replaces the receiver before the RHS replaces both bindings again.
    result(
        "let xs=[10,20];const old=xs;let i=0;function index():number{xs=[30,40];return i;}function rhs():number{i=1;xs=[50,60];return 3;}xs[index()]+=rhs();return old[0]*1000+xs[0]*10+i;",
        13501.0,
    );
    result(
        "const Math:number[]=[];const Utf8:number[]=[];const __ulua_trap:number[]=[];Math.push(1);Utf8.push(2);__ulua_trap.push(3);return Math[0]*100+Utf8[0]*10+__ulua_trap[0];",
        123.0,
    );
    result(
        "let xs=[10];const old=xs;function index():number{xs=[90];return 0;}const first=xs[index()];old[old.length]=20;function rhs():number{old.push(30);return 40;}old[3]=rhs();old.push(rhs());return first*10000+old.length*1000+old[3]*10+old[5];",
        106440.0,
    );
    result(
        "const xs=[10];function rhs():number{xs[0]=99;xs.push(20);return 3;}xs[0]+=rhs();xs[1]%=6;return xs[0]*10+xs[1];",
        132.0,
    );
    result(
        "let n=0;function next():number{n++;return n;}const xs=[next(),next(),next()];return xs[0]*100+xs[1]*10+xs[2];",
        123.0,
    );
    result(
        "const xs=[10];xs[0]+=3;xs[0]-=1;xs[0]*=2;xs[0]/=3;xs[0]%=3;return xs[0];",
        2.0,
    );
    result(
        "const xs=['a'];function rhs():string{xs[0]='x';xs.push('b');return 'c';}xs[0]+=rhs();function expected():string{return 'ac';}return xs[0]===expected()?xs.length:0;",
        2.0,
    );
    result(
        "const xs=[7];const a=xs[-0];xs[-0]=8;xs[xs.length]=9;return a*100+xs[0]*10+xs[1];",
        789.0,
    );
    result(
        "const xs=[1];function rhs():number{xs.push(2);return 3;}xs.push(rhs());return xs[0]*100+xs[1]*10+xs[2];",
        123.0,
    );
}

#[test]
fn iteration_keeps_one_array_live_length_and_per_step_closures() {
    result(
        "let xs=[1];const original=xs;let count=0;function receiver():number[]{count++;return xs;}let sum=0;for(const x of receiver()){xs=[99];sum+=x;if(x<4){original.push(x+1);}}return count*100+sum;",
        110.0,
    );
    result(
        "const xs=[[1],[2],[3]];const fs:(()=>number)[][]=[];for(const x of xs){fs.push([()=>x[0]]);}let sum=0;for(const group of fs){for(const f of group){sum=sum*10+f();}}return sum;",
        123.0,
    );
    result(
        "const xs=[1,2,3];let sum=0;for(const x of xs){if(x<2){continue;}for(const y of xs){if(y>2){break;}sum+=x*y;}if(x>2){break;}}return sum;",
        15.0,
    );
    for spill in [false, true] {
        let unused = if spill {
            (0..220)
                .map(|i| format!("let unused{i}={i};"))
                .collect::<String>()
        } else {
            String::new()
        };
        result(
            &format!(
                "{unused}const fs:(()=>number)[]=[];for(const x of [1,2,3]){{function read():number{{return x;}}fs.push(read);}}let sum=0;for(const f of fs){{sum=sum*10+f();}}return sum;"
            ),
            123.0,
        );
    }
    result(
        "const a=[1];const b=[1];const c=a;return a===c?(a===b?0:1):2;",
        1.0,
    );
}

unsafe extern "C" fn output(userdata: *mut c_void, text: *const c_char, length: usize) -> i32 {
    // SAFETY: The test keeps the Vec live. The shim provides readable output for this callback.
    unsafe {
        (*userdata.cast::<Vec<String>>()).push(
            String::from_utf8_lossy(std::slice::from_raw_parts(text.cast(), length)).into_owned(),
        );
    }
    0
}
#[test]
fn every_index_trap_delivers_output_and_keeps_the_instance_and_context_usable() {
    for index in [
        "0.5",
        "0.9999999999999999",
        "NaN",
        "Infinity",
        "-1",
        "1",
        "2",
        "9007199254740991",
        "1e100",
    ] {
        for operation in [
            format!("const value=xs[{index}];"),
            format!("xs[{index}]=rhs();"),
            format!("xs[{index}]+=rhs();"),
        ] {
            if index == "1" && operation.starts_with("xs[") && !operation.contains("+=") {
                continue;
            }
            let source = format!(
                "const xs=[10];function rhs():number{{print('rhs');return 20;}}export function bad():void{{print('before');{operation}print('after');}}export function good():number{{return 7;}}export function main():void{{}}"
            );
            let mut context = ptr::null_mut();
            let mut module = ptr::null_mut();
            let mut instance = ptr::null_mut();
            let mut diagnostic = Diagnostic::default();
            let mut lines = Vec::<String>::new();
            let mut host = Host::default();
            host.userdata = ptr::from_mut(&mut lines).cast();
            host.output = Some(output);
            // SAFETY: Test descriptors and buffers are live and cover their lengths. Calls are serialized.
            unsafe {
                assert_eq!(ulua_context_new(&mut context), ULUA_OK);
                assert_eq!(
                    ulua_compile(
                        context,
                        source.as_ptr().cast(),
                        source.len(),
                        std::ptr::null(),
                        0,
                        &CompileLimits::default(),
                        &mut module,
                        &mut diagnostic
                    ),
                    ULUA_OK
                );
                assert_eq!(
                    ulua_instance_new(
                        context,
                        module,
                        &host,
                        &RunLimits::default(),
                        &mut instance,
                        &mut diagnostic
                    ),
                    ULUA_OK
                );
                let mut bad = 0;
                let mut good = 0;
                assert_eq!(
                    ulua_module_find_export(module, c"bad".as_ptr(), 3, &mut bad),
                    ULUA_OK
                );
                assert_eq!(
                    ulua_module_find_export(module, c"good".as_ptr(), 4, &mut good),
                    ULUA_OK
                );
                for _ in 0..2 {
                    lines.clear();
                    let mut value = Value::default();
                    assert_eq!(
                        ulua_call(instance, bad, ptr::null(), 0, &mut value, &mut diagnostic),
                        ULUA_FAILED,
                        "{operation}"
                    );
                    assert_eq!(
                        CStr::from_ptr(diagnostic.code.as_ptr()).to_str().unwrap(),
                        "trap:index-out-of-range"
                    );
                    assert_eq!(
                        CStr::from_ptr(diagnostic.message.as_ptr())
                            .to_str()
                            .unwrap(),
                        "Index out of range"
                    );
                    let expected = if operation.starts_with("xs[") && !operation.contains("+=") {
                        vec!["before", "rhs"]
                    } else {
                        vec!["before"]
                    };
                    assert_eq!(lines, expected);
                    assert_eq!(
                        ulua_call(instance, good, ptr::null(), 0, &mut value, &mut diagnostic),
                        ULUA_OK
                    );
                    assert_eq!(value.number, 7.0);
                    let mut second = ptr::null_mut();
                    assert_eq!(
                        ulua_compile(
                            context,
                            source.as_ptr().cast(),
                            source.len(),
                            std::ptr::null(),
                            0,
                            &CompileLimits::default(),
                            &mut second,
                            &mut diagnostic
                        ),
                        ULUA_OK
                    );
                    assert_eq!(ulua_module_free(second), ULUA_OK);
                }
                assert_eq!(ulua_instance_free(instance), ULUA_OK);
                assert_eq!(ulua_module_free(module), ULUA_OK);
                assert_eq!(ulua_context_free(context), ULUA_OK);
            }
        }
    }
}
