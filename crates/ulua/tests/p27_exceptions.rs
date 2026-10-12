//! P2.7 witnesses include all completion replacements and call boundaries.
//! Measured Debug target cost: 0.58 seconds.
//! This target reports cost; no assertion reads a clock.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::{ffi::CStr, time::Instant};
use ulua::*;

#[test]
fn family_and_propagation_match_witnesses() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 direct");
    let kinds = [
        "Error",
        "TypeError",
        "RangeError",
        "SyntaxError",
        "ReferenceError",
        "EvalError",
        "URIError",
    ];
    for kind in kinds {
        let mut body = format!(
            "const x:URIError=new {kind}();print(`${{x.name}}:${{x.message}}`);x.name='changed';x.message='text';"
        );
        for test in kinds {
            body.push_str(&format!("print(`${{x instanceof {test}}}`);"));
        }
        body.push_str("try{throw x;}catch(e:unknown){if(e instanceof Error){print(`${e===x} ${e.name}: ${e.message}`);}else{throw e;}}");
        suite.source(&format!("export function main():void{{{body}}}"), None);
    }
    suite.source("function f():number{throw new Error('direct');}class A{m():number{throw new Error('method');}}export function main():void{try{f();}catch(e){if(e instanceof Error){print(e.message);}}try{new A().m();}catch(e){if(e instanceof Error){print(e.message);}}const a=():number=>{throw new Error('arrow');};const b:()=>number=a;try{a();}catch(e){if(e instanceof Error){print(e.message);}}try{b();}catch(e){if(e instanceof Error){print(e.message);}}}",Some("direct\nmethod\narrow\narrow\n"));
    for call in [
        "xs.forEach(cb)",
        "xs.map(cb)",
        "xs.filter(cb)",
        "xs.every(cb)",
        "xs.some(cb)",
        "xs.findLastIndex(cb)",
        "xs.findIndex(cb)",
        "xs.reduce(cb,0)",
        "xs.reduceRight(cb,0)",
        "xs.flatMap(cb)",
        "xs.sort(cb)",
    ] {
        let result = if call.contains("flatMap") {
            "number[]"
        } else if ["filter", "every", "some", "find"]
            .iter()
            .any(|m| call.contains(m))
        {
            "boolean"
        } else {
            "number"
        };
        let args = if call.contains("sort") || call.contains("reduce") {
            "a:number,b:number"
        } else {
            "a:number"
        };
        suite.source(&format!("export function main():void{{const x=new Error('callback');const xs:number[]=[2,1];const cb=({args}):{result}=>{{throw x;}};try{{{call};}}catch(e){{if(e instanceof Error){{print(`${{e===x}}`);}}else{{throw e;}}}}}}"), Some("true\n"));
    }
    suite.source("export function main():void{try{try{throw new TypeError('inner');}finally{print('inner final');}}catch{print('outer');}finally{print('outer final');}}",Some("inner final\nouter\nouter final\n"));
    suite.source("class A{v:number;constructor(n:number){if(n==0){throw new Error('empty');}this.v=n;}}export function main():void{try{new A(0);}catch{print('caught');}print(`${new A(3).v}`);}", Some("caught\n3\n"));
    suite.source("type R={f:()=>number};function fail():number{throw new EvalError('call');}function defaults(n:number=fail()):number{return n;}class A{constructor(){throw new Error('constructor');}m():void{throw new Error('method');}}export function main():void{const r:R|null={f:fail};try{r?.f();}catch(e){if(e instanceof Error){print(e.message);}}try{defaults();}catch(e){if(e instanceof Error){print(e.message);}}try{new A();}catch(e){if(e instanceof Error){print(e.message);}}}", Some("call\ncall\nconstructor\n"));
    suite.finish();
    eprintln!(
        "P2.7 direct count={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn twenty_finally_replacements_match_node() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 finally");
    for pending in [
        "return 11;",
        "throw new Error('pending');",
        "break;",
        "continue;",
    ] {
        for finalizer in [
            "print('final');",
            "return 22;",
            "throw new Error('final');",
            "break;",
            "continue;",
        ] {
            suite.source(&format!("function f():number{{for(let i=0;i<2;i++){{try{{print(`try${{i}}`);{pending}}}finally{{{finalizer}}}print('after');}}return 33;}}export function main():void{{try{{print(`${{f()}}`);}}catch(e){{if(e instanceof Error){{print(e.message);}}else{{throw e;}}}}}}"),None);
        }
    }
    suite.source("function f():number{let n:number=1;try{return n;}finally{n=2;}}export function main():void{print(`${f()}`);let i:number=0;while(i<3){i++;try{try{if(i==1){continue;}break;}finally{print('inner');}}finally{print('outer');}}}",Some("1\ninner\nouter\ninner\nouter\n"));
    suite.finish();
    eprintln!(
        "P2.7 finally count={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn uncaught_exception_leaves_instance_usable() {
    let mut script = support::Script::new(
        "export function fail():number{const e=new TypeError('message');e.name='custom';throw e;}export function good():number{return 7;}export function main():void{}",
    );
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    let (status, diagnostic, _) = script.call_diagnostic(script.index("fail"), &[]);
    assert_eq!(status, ULUA_FAILED);
    // SAFETY: The ABI terminates both diagnostic arrays.
    unsafe {
        assert_eq!(
            CStr::from_ptr(diagnostic.code.as_ptr()).to_str().unwrap(),
            "trap:uncaught-exception"
        );
        assert_eq!(
            CStr::from_ptr(diagnostic.message.as_ptr())
                .to_str()
                .unwrap(),
            "custom: message"
        );
    }
    let (status, _, value) = script.call(script.index("good"), &[]);
    assert_eq!(status, ULUA_OK);
    assert_eq!(value.number, 7.0);
}
