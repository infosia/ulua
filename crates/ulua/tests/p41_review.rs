//! Review regressions cover suspension paths and drain recovery.
//! Costs are reported. No assertion reads wall-clock time.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::{ffi::CStr, time::Instant};
use ulua::*;

#[test]
fn enclosing_narrowings_end_on_suspension_paths() {
    let start = Instant::now();
    let bodies = [
        "if(x!==null){await v();return x+1;}",
        "if(x!==null){while(true){await v();return x+1;}}",
        "if(x!==null){for(let i=0;i<2;i++){await v();return x+1;}}",
        "if(x!==null){for(const i of [1]){await v();return x+1;}}",
        "if(x!==null){while(await b()){return x+1;}}",
        "if(x!==null){for(let i=0;await b();i++){return x+1;}}",
        "if(x!==null){for(let i=0;i<2;await v()){return x+1;}}",
        "if(x!==null){while(true){print(`${x+1}`);await v();}}",
        "if(x!==null){for(let i=0;i<2;i++){print(`${x+1}`);await v();}}",
        "if(x!==null){try{await v();return x+1;}catch{}}",
        "if(x!==null){try{await v();throw new Error();}catch{return x+1;}}",
        "if(x!==null){try{throw new Error();}catch{await v();return x+1;}}",
        "if(x!==null){try{await v();}finally{return x+1;}}",
        "if(x!==null){try{throw new Error();}catch{await v();}finally{return x+1;}}",
        "if(x!==null){try{}finally{await v();return x+1;}}",
        "if(x!==null){for(let i=0;i<2;i++){try{break;}finally{await v();}}return x+1;}",
        "if(x!==null){for(let i=0;i<2;i++){try{continue;}finally{await v();}}return x+1;}",
        "if(x!==null){return (await b())?x+1:0;}",
        "if(x!==null){return true?(await v())+x:0;}",
        "if(x!==null){const n=true?await v():0;return x+1;}",
        "if(x!==null){const c:boolean=true;const n=c?0:await v();return x+1;}",
        "if(x!==null){if((await b())&&x+1>0){return 1;}}",
        "if(x!==null){if((await b())||x+1>0){return 1;}}",
        "if(x!==null){const n=true&&await b();return x+1;}",
        "if(x!==null){const n=false||await b();return x+1;}",
        "if(x!==null){const n=true?await b():false;if(n){return x+1;}}",
        "if(x!==null){const n=false?false:await b();if(n){return x+1;}}",
        "if(x!==null){const n: number|null=null;const k=n??await v();return x+1;}",
        "if(x!==null){const k=[await v(),x+1];}",
        "if(x!==null){print(`${await v()} ${x+1}`);}",
        "if(x!==null){return add(await v(),x+1);}",
        "if(x!==null){const o={n:await v(),m:x+1};}",
        "if(x!==null&&await b()){return x+1;}",
        "if(x===null||await b()){}else{return x+1;}",
    ];
    let prelude = "async function v():Promise<number>{return 0;}async function b():Promise<boolean>{return true;}function add(a:number,b:number):number{return a+b;}";
    for declaration in ["let x:number|null=1;", ""] {
        let argument = if declaration.is_empty() { "1" } else { "" };
        let parameter = if declaration.is_empty() {
            "x:number|null"
        } else {
            ""
        };
        for body in bodies {
            for nested in [
                format!("const f=async():Promise<number>=>{{{body}return 0;}};"),
                format!("async function f():Promise<number>{{{body}return 0;}}"),
            ] {
                let source = format!(
                    "{prelude}async function parent({parameter}):Promise<void>{{{declaration}{nested}const p=f();x=null;await p;}}export async function main():Promise<void>{{await parent({argument});}}"
                );
                let diagnostic =
                    ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default())
                        .err();
                assert_eq!(
                    diagnostic.as_ref().map(|d| d.code),
                    Some("C16"),
                    "{source}: {diagnostic:?}"
                );
                support::Script::compile(&source, CompileLimits::default(), "C16");
            }
        }
    }
    for source in [
        "async function v():Promise<number>{return 0;}export function main():void{let s:string|null='a';async function inner():Promise<number>{if(s!==null){await v();return s.length;}return 0;}const p=inner();s=null;}",
        "async function v():Promise<number>{return 0;}export async function main():Promise<void>{let x:number|null=1;const outer=async():Promise<number>=>{let y:number|null=1;const inner=async():Promise<number>=>{if(x!==null&&y!==null){await v();return x+y;}return 0;};const p=inner();y=null;return p;};const p=outer();x=null;await p;}",
        "async function v():Promise<number>{return 0;}class C{async m(x:number|null):Promise<number>{const f=async():Promise<number>=>{if(x!==null){await v();return x+1;}return 0;};const p=f();x=null;return p;}}export function main():void{}",
    ] {
        support::Script::compile(source, CompileLimits::default(), "C16");
    }
    eprintln!(
        "P4.1 suspension rejects={} cost={:.3}s",
        bodies.len() * 4 + 3,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn const_local_and_fresh_narrowings_continue() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.1 narrowing review");
    suite.source("async function v():Promise<number>{return 0;}export async function main():Promise<void>{const x:number|null=1;if(x!==null){const f=async():Promise<number>=>{await v();return x+1;};print(`${await f()}`);}const g=async():Promise<number>=>{let y:number|null=2;if(y!==null){await v();return y+1;}return 0;};print(`${await g()}`);}", Some("2\n3\n"));
    suite.source("async function v():Promise<number>{return 0;}export async function main():Promise<void>{let x:number|null=1;const f=async():Promise<number>=>{if(x!==null){await v();if(x!==null){return x+1;}}return 0;};const p=f();x=null;print(`${await p}`);}", Some("0\n"));
    suite.source("async function v():Promise<number>{return 0;}export async function main():Promise<void>{let x:number|null=1;const f=async():Promise<number>=>{if(x!==null){const y=x;await v();return y+1;}return 0;};const p=f();x=null;print(`${await p}`);}", Some("2\n"));
    suite.source("async function v():Promise<number>{return 0;}export async function main():Promise<void>{let x:number|null=1;const f=async():Promise<number>=>{if(x!==null){const g=async():Promise<number>=>{await v();return 0;};const p=g();return x+1;}return 0;};print(`${await f()}`);}", Some("2\n"));
    suite.source("async function v():Promise<number>{return 0;}async function f(p:Promise<number>=(async():Promise<number>=>{await v();return 2;})()):Promise<number>{return p;}export async function main():Promise<void>{print(`${await f()}`);}", Some("2\n"));
    suite.finish();
    eprintln!(
        "P4.1 narrowing accepts={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn async_defaults_and_promise_declarations_are_rejected() {
    let start = Instant::now();
    let prelude = "async function v():Promise<number>{return 0;}";
    let forms = [
        "async function f(n:number=await v()):Promise<number>{return n;}",
        "class C{async m(n:number=await v()):Promise<number>{return n;}}",
        "async function f(n:number=1+(await v())):Promise<number>{return n;}",
        "class C{async m(n:number=true?await v():1):Promise<number>{return n;}}",
        "class Promise{}",
        "type Promise={n:number};",
        "type Promise=number;",
        "type Promise<T>={n:T};",
        "const Promise=1;",
        "function Promise():number{return 1;}",
    ];
    for form in forms {
        support::Script::compile(
            &format!("{prelude}{form}export function main():void{{}}"),
            CompileLimits::default(),
            "C19",
        );
    }
    for method in ["indexOf", "lastIndexOf", "includes"] {
        for items in ["p", ""] {
            support::Script::compile(
                &format!(
                    "{prelude}export function main():void{{const p=v();const xs:Promise<number>[]=[{items}];const n=xs.{method}(p);}}"
                ),
                CompileLimits::default(),
                "C19",
            );
        }
    }
    eprintln!(
        "P4.1 surface review rejects=16 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn drain_memory_stop_releases_jobs_before_collection() {
    let start = Instant::now();
    let mut script = support::Script::new(
        "async function v(n:number):Promise<number>{return n;}export async function spin(n:number):Promise<number>{await v(n);return spin(n+1);}export function good():number{return 7;}export function push():number{const xs:number[]=[];for(let i=0;i<1000000;i++){xs.push(i);}return xs.length;}export function main():void{}",
    );
    for _ in 0..50 {
        let mut limits = RunLimits::default();
        limits.max_memory_bytes = 64 * 1024 * 1024;
        limits.max_steps = 100_000_000;
        assert_eq!(script.instance(limits).0, ULUA_OK);
        let stopped = script.call(script.index("spin"), &[support::number(0)]);
        assert_eq!(
            (stopped.0, stopped.1.as_str()),
            (ULUA_FAILED, "limit:memory")
        );
        let good = script.call(script.index("good"), &[]);
        assert_eq!((good.0, good.2.number), (ULUA_OK, 7.0), "{good:?}");
        let pushed = script.call(script.index("push"), &[]);
        assert_eq!((pushed.0, pushed.2.number), (ULUA_OK, 1_000_000.0));
    }
    eprintln!(
        "P4.1 drain memory recovery fresh_instances=50 recovery_calls=100 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn export_rejection_reason_precedes_unhandled_reason() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.1 rejection review");
    for body in [
        "const p=other();throw new RangeError('own');",
        "const p=other();await v();throw new RangeError('own');",
        "const p=other();return own();",
    ] {
        let source = format!(
            "async function v():Promise<number>{{return 1;}}async function other():Promise<number>{{throw new TypeError('other');}}async function own():Promise<number>{{await v();throw new RangeError('own');}}export async function fail():Promise<number>{{{body}}}export function good():number{{return 7;}}export function main():void{{}}"
        );
        let mut script = support::Script::new(&source);
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
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
                "RangeError: own"
            );
        }
        assert_eq!(script.call(script.index("good"), &[]).2.number, 7.0);
        suite.source_async_trap(
            &source.replace(
                "export function main():void{}",
                "export async function main():Promise<void>{await fail();}",
            ),
            "RangeError",
        );
    }
    suite.finish();
    eprintln!(
        "P4.1 rejection precedence cases=3 cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn drain_stop_precedes_export_rejection() {
    for (body, code) in [
        (
            "const xs:number[]=[];return xs[0];",
            "trap:index-out-of-range",
        ),
        ("while(true){}", "limit:steps"),
    ] {
        let source = format!(
            "async function v():Promise<number>{{return 1;}}async function stop():Promise<number>{{await v();{body}}}export async function fail():Promise<number>{{const p=stop();throw new RangeError('own');}}export function good():number{{return 7;}}export function main():void{{}}"
        );
        let mut script = support::Script::new(&source);
        let mut limits = RunLimits::default();
        limits.max_steps = 20000;
        assert_eq!(script.instance(limits).0, ULUA_OK);
        let stopped = script.call(script.index("fail"), &[]);
        assert_eq!((stopped.0, stopped.1.as_str()), (ULUA_FAILED, code));
        let good = script.call(script.index("good"), &[]);
        assert_eq!((good.0, good.2.number), (ULUA_OK, 7.0));
    }
}
