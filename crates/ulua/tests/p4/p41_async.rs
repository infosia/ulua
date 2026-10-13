//! P4.1 direct acceptance, rejection, ordering, and suspension witnesses.
//! Costs are reported. No assertion reads wall-clock time.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::{ffi::CStr, time::Instant};
use ulua::*;

#[test]
fn closed_async_surface() {
    let start = Instant::now();
    let prelude = "async function f():Promise<number>{return 1;}";
    let bodies = [
        "f();",
        "const p=f();print(`${p}`);",
        "const p=f();if(p){}",
        "const p=f();print(typeof p);",
        "const p=f();print(`${p===p}`);",
        "const p=f();print(`${!p}`);",
        "const p=f();const x=p+1;",
        "const p=f();p.then();",
        "const p=f();const x=p[0];",
        "let p=f();p++;",
        "let p=f();p+=f();",
        "const p=f();const x=-p;",
        "const p=f();const x=p.state;",
        "const xs:(()=>Promise<number>)[]=[];xs.push(async():Promise<number>=>1);",
        "const xs:number[]=[1];xs.forEach(f);",
        "const p=f();const x=p instanceof Error;",
        "const p=f();const x=p&&true;",
        "const p:Promise<number>|null=f();const x=p??f();",
        "const p=f();const x=Promise;",
        "const p=Promise.resolve(1);",
        "const p=new Promise<number>();",
        "const p:Promise<number|null>=f();",
        "const p:Promise<Promise<number>>=f();",
        "const p:Promise<Promise<number>|null>=f();",
        "const p:Promise<void>=f();",
        "const p:Promise=f();",
        "const p:Promise<number,string>=f();",
        "const xs:number[]=[1];xs.forEach(async(x:number):Promise<void>=>{});",
    ];
    for body in bodies {
        support::Script::compile(
            &format!("{prelude}export function main():void{{{body}}}"),
            CompileLimits::default(),
            "C19",
        );
    }
    let forms = [
        "async function f():number{return 1;}export function main():void{}",
        "const f=async():number=>1;export function main():void{}",
        "const f=async()=>1;export function main():void{}",
        "const f=async()=>{};export function main():void{}",
        "async function f(){return 1;}export function main():void{}",
        "function* f():number{yield 1;}export function main():void{}",
        "async function* f():AsyncGenerator<number>{yield 1;}export function main():void{}",
        "async function f():Promise<number>{return 1;}export function main():void{const x=await f();}",
        "async function f():Promise<number>{return 1;}const x=await f();export function main():void{}",
        "export async function main():Promise<void>{const x=await 1;}",
        "class C{*m():number{yield 1;}}export function main():void{}",
        "export async function main():Promise<void>{function nested():void{await main();}}",
        "export async function main():Promise<void>{for await(const x of [1]){}}",
        "export async function main():Promise<void>{await using x=1;}",
        "class C{async m():number{return 1;}}export function main():void{}",
        "async function f():Promise<number>{return 1;}async function g():Promise<number|null>{return f();}export function main():void{}",
        "async function f():Promise<number>{return 1;}type R={f:()=>Promise<number>};export function main():void{const r:R|null={f:f};r?.f();}",
    ];
    for source in forms {
        support::Script::compile(source, CompileLimits::default(), "C19");
    }
    for source in [
        "async function f():Promise<void>{}export async function main():Promise<void>{const x=await f();}",
        "async function f():Promise<void>{}export async function main():Promise<void>{print(`${await f()}`);}",
    ] {
        support::Script::compile(source, CompileLimits::default(), "C2");
    }
    for source in [
        "async function f():Promise<number>{return 1;}export async function main():Promise<void>{let x:number|null=1;const w=async():Promise<number>=>{await f();x=null;return 0;};const p=w();if(x!==null){await p;print(`${x+1}`);}}",
        "async function f():Promise<number>{return 1;}export async function main():Promise<void>{let x:number|null=1;const w=():number=>{x=null;return 0;};if(x!==null){await f();print(`${x+1}`);}}",
    ] {
        support::Script::compile(source, CompileLimits::default(), "C16");
    }
    eprintln!(
        "P4.1 rejection cases={} cost={:.3}s",
        bodies.len() + forms.len() + 4,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn promise_jobs_and_values_match_node() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.1 direct");
    suite.source("export async function main():Promise<void>{}", Some(""));
    for file in [
        include_str!("../../../../corpus/accept/a58-async-order.ts"),
        include_str!("../../../../corpus/accept/a59-async-exceptions.ts"),
        include_str!("../../../../corpus/accept/a60-promise-values.ts"),
        include_str!("../../../../corpus/accept/a61-async-methods.ts"),
    ] {
        suite.source(file, None);
    }
    suite.source("async function v():Promise<number>{return 2;}export async function main():Promise<void>{let x:number|null=1;if(x!==null){await v();print(`${x+1}`);}const a=async():Promise<number>=>await v();const p=a();print(`${await p} ${await p}`);const nil=async():Promise<number|null>=>null;const no=async():Promise<void>=>{return;};await no();print(`${(await nil())===null}`);}", Some("2\n2 2\ntrue\n"));
    suite.source("async function v():Promise<number>{return 1;}async function fail():Promise<number>{throw new TypeError('late');}export async function main():Promise<void>{const p=fail();await v();try{await p;}catch(e){if(e instanceof Error){print(e.message);}}}", Some("late\n"));
    suite.source_trap("async function fail(s:string):Promise<number>{throw new TypeError(s);}export function main():void{const a=fail('first');const b=fail('second');print('done');}", "done\n", "trap:uncaught-exception", "TypeError");
    suite.source("async function v():Promise<number>{print('value');return 1;}async function adopt():Promise<number>{return v();}async function direct():Promise<number>{return await v();}export async function main():Promise<void>{const a=adopt();const b=direct();print('start');print(`${await b}`);print(`${await a}`);}",None);
    suite.source("async function v():Promise<number>{return 9;}function identity(p:Promise<number>):Promise<number>{return p;}export async function main():Promise<void>{print(`${await identity(v())}`);}",Some("9\n"));
    suite.source("async function v():Promise<number>{return 1;}class C{p:Promise<number>=v();async self():Promise<number>{await v();return this.p;}}export async function main():Promise<void>{const c=new C();c.p=c.self();try{await c.p;}catch(e){if(e instanceof Error){print(`${e.name}: ${e.message}`);}}}",Some("TypeError: Chaining cycle detected for promise #<Promise>\n"));
    suite.source("type R={n:number};async function a():Promise<number[]>{return [1,2];}async function b():Promise<R>{return {n:3};}async function c():Promise<()=>number>{return ():number=>4;}async function d():Promise<Error>{return new Error('value');}export async function main():Promise<void>{const xs=await a();const r=await b();const f=await c();const e=await d();print(`${xs[0]} ${r.n} ${f()} ${e.message}`);}",Some("1 3 4 value\n"));
    suite.source("function fail():number{throw new Error('default');}async function f(n:number=fail()):Promise<number>{return n;}export async function main():Promise<void>{let caught:number=0;const p=f();print('returned');try{await p;}catch{caught++;}print(`${caught}`);}",Some("returned\n1\n"));
    suite.source("async function tick():Promise<number>{return 0;}async function value(n:number):Promise<number>{print(`start${n}`);await tick();print(`end${n}`);return n;}export async function main():Promise<void>{const xs:number[]=[1,2];const ps=xs.map(value);print('mapped');print(`${await ps[1]} ${await ps[0]}`);}",Some("start1\nstart2\nmapped\nend1\nend2\n2 1\n"));
    suite.source("async function tick():Promise<number>{return 0;}async function done(n:number):Promise<void>{await tick();print(`${n}`);}async function value(n:number):Promise<void>{return done(n);}export async function main():Promise<void>{const xs:number[]=[1,2];const ps=xs.map(value);await ps[0];await ps[1];}",Some("1\n2\n"));
    suite.finish();
    eprintln!(
        "P4.1 direct cases={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn async_export_tags_results_and_recovery() {
    let source = "async function tick():Promise<number>{return 1;}class C{p:Promise<number>=tick();async wait():Promise<number>{await tick();return await this.p;}}export async function pending():Promise<number>{const c=new C();c.p=c.wait();return c.p;}export async function n(v:number=7):Promise<number>{await tick();return v;}export async function b():Promise<boolean>{return false;}export async function s():Promise<string>{await tick();return 'text';}export async function fail():Promise<number>{throw new RangeError('lost');}export function good():number{return 42;}export async function main():Promise<void>{await tick();return;}";
    let compiled =
        ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default()).unwrap();
    assert_eq!(
        compiled
            .exports
            .iter()
            .map(|e| e.result)
            .collect::<Vec<_>>(),
        [1, 1, 2, 3, 1, 1, 0]
    );
    let mut script = support::Script::new(source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(
        script
            .call(script.index("n"), &[support::number(7)])
            .2
            .number,
        7.0
    );
    let b = script.call(script.index("b"), &[]).2;
    assert_eq!((b.r#type, b.boolean), (ULUA_TYPE_BOOLEAN, 0));
    let s = script.call(script.index("s"), &[]).2;
    // SAFETY: The result bytes remain valid until the next call on the instance.
    unsafe {
        assert_eq!(
            std::slice::from_raw_parts(s.data.cast::<u8>(), s.length),
            b"text"
        );
    }
    assert_eq!(script.call(script.index("main"), &[]).0, ULUA_OK);
    let (status, d, _) = script.call_diagnostic(script.index("fail"), &[]);
    assert_eq!(status, ULUA_FAILED);
    // SAFETY: The ABI terminates both diagnostic arrays.
    unsafe {
        assert_eq!(
            CStr::from_ptr(d.code.as_ptr()).to_str().unwrap(),
            "trap:uncaught-exception"
        );
        assert_eq!(
            CStr::from_ptr(d.message.as_ptr()).to_str().unwrap(),
            "RangeError: lost"
        );
    }
    assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
    assert_eq!(script.call(script.index("pending"), &[]).1, "async:pending");
    assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
}

#[test]
fn await_in_each_protected_block_to_the_depth_ceiling() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.1 nesting");
    for kind in ["try", "catch", "finally"] {
        let mut ceiling = 0;
        for depth in 1..48 {
            let mut body = "await v();return 7;".to_owned();
            for _ in 0..depth {
                body = match kind {
                    "catch" => format!("try{{throw new Error();}}catch{{{body}}}"),
                    "finally" => format!("try{{}}finally{{{body}}}"),
                    _ => format!("try{{{body}}}finally{{await v();}}"),
                };
            }
            let source = format!(
                "async function v():Promise<number>{{return 1;}}async function f():Promise<number>{{{body}}}export async function main():Promise<void>{{print(`${{await f()}}`);}}"
            );
            match ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default()) {
                Ok(_) => {
                    suite.source(&source, Some("7\n"));
                    ceiling = depth;
                }
                Err(d) => {
                    assert!(
                        matches!(d.code, "limit:nesting-depth" | "limit:temporaries"),
                        "{d:?}"
                    );
                    break;
                }
            }
        }
        assert!(ceiling > 0);
        eprintln!("P4.1 await {kind} depth_ceiling={ceiling}");
    }
    for jump in ["break", "continue", "return 7"] {
        suite.source(&format!("async function v():Promise<number>{{return 1;}}async function f():Promise<number>{{for(let i=0;i<2;i++){{try{{await v();{jump};}}finally{{await v();print('final');}}}}return 8;}}export async function main():Promise<void>{{print(`${{await f()}}`);}}"),None);
    }
    suite.finish();
    eprintln!(
        "P4.1 await nesting programs={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn module_jobs_drain_and_unhandled_rejection_keeps_instance() {
    let mut script = support::Script::new(
        "let ready:number=0;async function tick():Promise<number>{return 1;}async function start():Promise<number>{await tick();ready=42;return ready;}const p=start();async function fail():Promise<number>{throw new TypeError('lost');}export function lost():void{const p=fail();}export function read():number{return ready;}export function main():void{}",
    );
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(script.call(script.index("read"), &[]).2.number, 42.0);
    let (status, d, _) = script.call_diagnostic(script.index("lost"), &[]);
    assert_eq!(status, ULUA_FAILED);
    // SAFETY: The ABI terminates both diagnostic arrays.
    unsafe {
        assert_eq!(
            CStr::from_ptr(d.code.as_ptr()).to_str().unwrap(),
            "trap:uncaught-exception"
        );
        assert_eq!(
            CStr::from_ptr(d.message.as_ptr()).to_str().unwrap(),
            "TypeError: lost"
        );
    }
    assert_eq!(script.call(script.index("read"), &[]).2.number, 42.0);
}
