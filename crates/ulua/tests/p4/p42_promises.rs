//! Callback and all witnesses share one node, tsc, and single-worker analyzer batch.
//! Each test reports its cost. The stop target covers resource stops separately.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::time::Instant;
use ulua::*;
const PRELUDE: &str = "async function v(n:number=1):Promise<number>{return n;}async function fail():Promise<number>{throw new RangeError('bad');}async function done():Promise<void>{}";
#[test]
fn callback_rules_and_derived_operators_are_closed() {
    let start = Instant::now();
    let bodies = [
        "const p=v().then();",
        "const p=v().then(1);",
        "const p=v().then((n:number):number=>n,():number=>0);",
        "const p=v().then((s:string):number=>1);",
        "const p=v().then((n:number,k:number):number=>n);",
        "const p=v().catch((e):number=>0);",
        "const p=v().catch((e:Error):number=>0);",
        "const p=v().catch((e:unknown,k:number):number=>0);",
        "const p=v().catch(():string=>'x');",
        "const p=v().catch(async():Promise<string>=>'x');",
        "const p=v().finally((n:number):void=>{});",
        "const p=v().finally(():number=>1);",
        "const p=v().finally(async():Promise<number>=>1);",
        "const p=Promise.all([done()]);",
        "const p=Promise.all([v(),done()]);",
        "const p=Promise.all([1]);",
        "const p=Promise.all(v());",
        "const p=Promise.all([v()], [v()]);",
        "const p=Promise.race([v()]);",
        "const p=Promise.reject(new Error());",
        "const p=Promise.any([v()]);",
        "const p=Promise.allSettled([v()]);",
        "const p=Promise.resolve(1);",
        "async function f():Promise<number>{return 1;}const xs:(()=>Promise<number>)[]=[];xs.push(f);",
        "async function f():Promise<number>{return 1;}const xs:(()=>Promise<number>)[]=[];xs.unshift(f);",
        "async function f():Promise<number>{return 1;}const xs:(()=>Promise<number>)[]=[];xs.fill(f);",
        "const p=v().then(()=>null);const xs=Promise.all([p]);",
        "const p=v().then(():number=>1);const f=p.then;",
        "const p=v().catch(():number=>1);const x=p.state;",
        "const p=v().finally(():void=>{});const x=p[0];",
        "const p=Promise.all([v()]);const x=p+1;",
        "[1].forEach((n:number):Promise<number>=>v(n));",
        "const xs=[1].filter((n:number):Promise<number>=>v(n));",
        "const xs=[1].some(async(n:number):Promise<boolean>=>true);",
        "const xs=[1].sort(async(n:number,k:number):Promise<number>=>n-k);",
        "async function nil():Promise<number|null>{return null;}const p=Promise.all([nil()]);",
    ];
    for body in bodies {
        support::Script::compile(
            &format!("{PRELUDE}export function main():void{{{body}}}"),
            CompileLimits::default(),
            "C19",
        );
    }
    support::Script::compile(
        "type R={then:number};async function v():Promise<number>{return 1;}export function main():void{const p=v().then(():R=>({then:1}));}",
        CompileLimits::default(),
        "C19",
    );
    let derived = [
        "v().then(():number=>1)",
        "v().catch(():number=>1)",
        "v().finally(():void=>{})",
        "Promise.all([v()])",
    ];
    let uses = [
        "p;",
        "print(`${p}`);",
        "if(p){}",
        "const b=p===p;",
        "const b=!p;",
        "const s=typeof p;",
        "const b=p&&true;",
        "const n=-p;",
        "const b=p>p;",
        "const xs=[p];const n=xs.indexOf(p);",
    ];
    for promise in derived {
        for usage in uses {
            let body = if usage == "p;" {
                format!("{promise};")
            } else {
                format!("const p={promise};{usage}")
            };
            support::Script::compile(
                &format!("{PRELUDE}export function main():void{{{body}}}"),
                CompileLimits::default(),
                "C19",
            );
        }
    }
    for (body, code) in [
        (
            "const p=v().catch((e:unknown):number=>{print(e.message);return 0;});",
            "C17",
        ),
        (
            "const p=v().catch((e:unknown):number=>{e=new Error();return 0;});",
            "C11",
        ),
        (
            "const p=v().catch((e:unknown):number=>{const x=e;return 0;});",
            "C17",
        ),
    ] {
        support::Script::compile(
            &format!("{PRELUDE}export function main():void{{{body}}}"),
            CompileLimits::default(),
            code,
        );
    }
    // C6.5 requires an element type for the empty array; an annotated empty input is accepted below.
    support::Script::compile(
        &format!("{PRELUDE}export function main():void{{const p=Promise.all([]);}}"),
        CompileLimits::default(),
        "C1",
    );
    eprintln!(
        "P4.2 rejection cases={} cost={:.3}s",
        bodies.len() + derived.len() * uses.len() + 5,
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn callbacks_and_all_match_es_jobs() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.2 direct");
    for file in [
        include_str!("../../../../corpus/accept/a62-promise-then.ts"),
        include_str!("../../../../corpus/accept/a63-promise-all.ts"),
    ] {
        suite.source(file, None);
    }
    let bodies = [
        "const a=v().then((n:number):number=>{print('then');return n+1;});const b=v().then(():Promise<number>=>v(3));const c=v().then(async(n:number):Promise<number>=>{await v();print('async');return n+3;});print('sync');print(`${await a} ${await b} ${await c}`);",
        "const a=v().catch((e:unknown):number=>{print('unexpected');throw e;});const b=fail().catch((e:unknown):number=>{if(e instanceof Error){print(e.message);}return 9;});const c=fail().catch(async(e:unknown):Promise<number>=>{await v();return 4;});print(`${await a} ${await b} ${await c}`);",
        "const a=v().then(():void=>{print('void');});await a;const b=fail().then(():number=>{print('unexpected');return 1;});try{await b;}catch(e){if(e instanceof Error)print(e.message);}",
        "const a=v().finally(():void=>{print('final value');});const b=fail().finally(():void=>{print('final reason');});print(`${await a}`);try{await b;}catch(e){if(e instanceof Error)print(e.message);}",
        "const a=v().finally(():void=>{throw new TypeError('override');});const b=fail().finally(async():Promise<void>=>{await done();throw new URIError('async override');});for(const p of [a,b]){try{await p;}catch(e){if(e instanceof Error)print(e.name);}}",
        "const a=v().finally(():Promise<void>=>done());const b=fail().finally(async():Promise<void>=>{await done();print('waited');});print(`${await a}`);try{await b;}catch(e){if(e instanceof Error)print(e.name);}",
        "const xs:Promise<number>[]=[];const a=Promise.all(xs);print('sync');print(`${(await a).length}`);",
        "function list():Promise<number>[]{return [v(3),v(4)];}const a=Promise.all(list());print((await a).join(','));",
        "const a=Promise.all([fail(),fail(),v()]);try{await a;}catch(e){if(e instanceof Error)print(e.message);}await v();print('end');",
        "function plus(n:number):number{return n+2;}function recover(e:unknown):number{if(e instanceof Error)print(e.name);return 3;}function final():void{print('final');}const a=v().then(plus);const b=fail().catch(recover).finally(final);print(`${await a} ${await b}`);",
        "const xs=[1,2,3].map(async(n:number,i:number):Promise<number>=>{print(`start${i}`);await v();return n+i;});print('mapped');print((await Promise.all(xs)).join(','));",
        "const a=v().then(():number|null=>null);const b=v().then(():boolean=>false);const c=v().then(():string=>'text');const d=v().then(():number[]=>[1,2]);print(`${(await a)===null} ${await b} ${await c} ${(await d).length}`);",
        "let p:Promise<number>=v();p=p.then(():Promise<number>=>p);try{await p;}catch(e){if(e instanceof Error)print(e.message);}",
        "const p=fail().catch((e:unknown):number=>{throw e;});try{await p;}catch(e){if(e instanceof Error)print(e.message);}",
    ];
    for body in bodies {
        suite.source(
            &format!("{PRELUDE}export async function main():Promise<void>{{{body}}}"),
            None,
        );
    }
    for expression in [
        "fail().then(():number=>1)",
        "v().then(():number=>{throw new TypeError('derived');})",
        "fail().catch((e:unknown):number=>{throw e;})",
        "fail().finally(():void=>{})",
        "Promise.all([fail(),fail()])",
    ] {
        suite.source_async_trap(
            &format!("{PRELUDE}export function main():void{{const p={expression};print('end');}}"),
            if expression.contains("derived") {
                "TypeError"
            } else {
                "RangeError"
            },
        );
    }
    for body in [
        "function show(n:number|null):void{print(`${n===null}`);}const p=v().then(()=>null);show(await p);",
        "function f(n:number=9):number{return n+1;}const p=v().then(f);print(`${await p}`);",
        "function f():Promise<void>{return done();}const p=v().finally(f);print(`${await p}`);",
        "async function bad():Promise<void>{throw new Error('void');}function recover(e:unknown):void{if(e instanceof Error)print(e.message);}const a=bad().catch(recover);const b=bad().catch(():void=>{});await a;await b;",
        "function f():number{return 7;}const a=v().then(f);const b=fail().catch(f);print(`${await a} ${await b}`);",
        "const p=v().then(():number=>{throw new TypeError('later handled');});await v();const q=p.catch((e:unknown):number=>{if(e instanceof Error)print(e.message);return 2;});print(`${await q}`);",
        "let p=v().finally(():Promise<void>=>done());for(let i=0;i<8;i++){p=p.then((n:number):number=>{print(`then${n}`);return n+1;});}async function ticks():Promise<void>{for(let i=0;i<12;i++){await v();print(`tick${i}`);}}const t=ticks();print(`${await p}`);await t;",
    ] {
        suite.source(
            &format!("{PRELUDE}export async function main():Promise<void>{{{body}}}"),
            None,
        );
    }
    suite.source_async_trap("async function first():Promise<number>{throw new RangeError('early');}async function second():Promise<number>{throw new TypeError('late');}export function main():void{const early=first();const late=second();const p=Promise.all([late,early]);}","TypeError");
    suite.finish();
    eprintln!(
        "P4.2 direct programs={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn derived_unhandled_rejections_keep_reason_order_and_instance() {
    let start = Instant::now();
    let source = "async function first():Promise<number>{throw new RangeError('first');}async function second():Promise<number>{throw new TypeError('second');}export function lostThen():void{const a=first().then(():number=>1);const b=second().then(():number=>2);}export function lostCatch():void{const a=first().catch((e:unknown):number=>{throw new URIError('first derived');});const b=second().catch((e:unknown):number=>{throw new TypeError('second derived');});}export function lostFinally():void{const a=first().finally(():void=>{});const b=second().finally(():void=>{});}export function lostAll():void{const a=Promise.all([first(),second()]);const b=Promise.all([second()]);}export function reversedAll():void{const early=first();const late=second();const p=Promise.all([late,early]);}export function good():number{return 42;}export function main():void{}";
    let mut script = support::Script::new(source);
    assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
    for (name, expected) in [
        ("lostThen", "RangeError: first"),
        ("lostCatch", "URIError: first derived"),
        ("lostFinally", "RangeError: first"),
        ("lostAll", "RangeError: first"),
        // C19.6 queues reactions to settled inputs in registration order.
        ("reversedAll", "TypeError: second"),
    ] {
        let (status, d, _) = script.call_diagnostic(script.index(name), &[]);
        assert_eq!(status, ULUA_FAILED);
        // SAFETY: The ABI terminates the diagnostic code and message arrays.
        unsafe {
            assert_eq!(
                std::ffi::CStr::from_ptr(d.code.as_ptr()).to_str().unwrap(),
                "trap:uncaught-exception"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(d.message.as_ptr())
                    .to_str()
                    .unwrap(),
                expected
            );
        }
        assert_eq!(script.call(script.index("good"), &[]).2.number, 42.0);
    }
    eprintln!(
        "P4.2 derived rejection diagnostics=5 recovery_calls=5 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
