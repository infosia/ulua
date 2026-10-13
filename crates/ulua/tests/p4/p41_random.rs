//! Fixed-seed async tasks compare their full job order with three witnesses.
//! Each run awaits async main and records unhandledRejection in the node witness.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::time::Instant;
struct Random(u64);
impl Random {
    fn next(&mut self) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as usize
    }
}
#[test]
fn random_async_programs_match_three_witnesses() {
    let start = Instant::now();
    let mut r = Random(0x0041_c124_0026);
    let mut suite = witnesses::Suite::named("P4.1 random");
    for i in 0..240 {
        let mut functions = String::from(
            "async function value(n:number):Promise<number>{return n;}async function failure(n:number):Promise<number>{await value(n);throw new RangeError('bad');}",
        );
        for task in 0..3 {
            let count = 1 + r.next() % 4;
            let fail = r.next().is_multiple_of(3);
            let pending = match r.next() % 4 {
                0 => "return await value(sum);",
                1 => "return value(sum);",
                2 => "throw new TypeError('return');",
                _ => "sum+=await value(1);",
            };
            let caught = match r.next() % 4 {
                0 => "return await value(9);",
                1 => "throw e;",
                2 => "return value(11);",
                _ => "sum+=await value(3);",
            };
            let finalizer = match r.next() % 4 {
                0 => "return await value(13);",
                1 => "return value(17);",
                2 => "throw new URIError('final');",
                _ => "sum+=await value(5);",
            };
            functions.push_str(&format!("async function task{task}():Promise<number>{{let sum:number=0;print('t{task} start');for(let k=0;k<{count};k++){{try{{sum+=await {}(k);print(`t{task} value ${{sum}}`);{pending}}}catch(e){{print('t{task} catch');if(e instanceof Error){{print(e.name);}}{caught}}}finally{{await value(k);print('t{task} finally');{finalizer}}}}}return sum;}}",if fail {"failure"} else {"value"}));
        }
        let order = if r.next().is_multiple_of(2) {
            [0, 1, 2]
        } else {
            [2, 0, 1]
        };
        let mut body =
            String::from("const p0=task0();const p1=task1();const p2=task2();print('started');");
        for task in order {
            body.push_str(&format!("try{{print(`p{task} ${{await p{task}}} ${{await p{task}}}`);}}catch(e){{if(e instanceof Error){{print(`p{task} ${{e.name}}`);}}}}"));
        }
        body.push_str(&format!("print('end{i}');"));
        if i % 24 == 0 {
            body.push_str("const lost=failure(1);");
            suite.source_async_trap(
                &format!("{functions}export async function main():Promise<void>{{{body}}}"),
                "RangeError",
            );
        } else {
            suite.source(
                &format!("{functions}export async function main():Promise<void>{{{body}}}"),
                None,
            );
        }
    }
    for i in 0..120 {
        let seed = 1 + r.next() % 9;
        let assigned = if r.next().is_multiple_of(2) {
            "null".to_owned()
        } else {
            (10 + r.next() % 9).to_string()
        };
        let suspend = match r.next() % 8 {
            0 => "await value(0);",
            1 => "for(let k=0;k<2;k++){await value(k);}",
            2 => "try{await value(0);}finally{await value(1);}",
            3 => "try{throw new Error();}catch{await value(0);}",
            4 => "const n=(await flag())?await value(0):await value(1);",
            5 => "const n=(await flag())&&await flag();",
            6 => "const n=(await flag())||await flag();",
            _ => "const g=async():Promise<number>=>{await value(0);return 1;};await g();",
        };
        let (parameter, declaration) = if r.next().is_multiple_of(2) {
            ("x:number|null", String::new())
        } else {
            ("seed:number", "let x:number|null=seed;".to_owned())
        };
        let arrow = r.next().is_multiple_of(2);
        let build = |read: &str| {
            let body = format!(
                "if(x!==null){{const before=x;{suspend}print(`capture{i} ${{before}} ${{x??0}}`);return {read};}}return 0;"
            );
            let nested = if arrow {
                format!("const f=async():Promise<number>=>{{{body}}};")
            } else {
                format!("async function f():Promise<number>{{{body}}}")
            };
            format!(
                "async function value(n:number):Promise<number>{{return n;}}async function flag():Promise<boolean>{{return true;}}async function parent({parameter}):Promise<number>{{{declaration}{nested}const p=f();x={assigned};return p;}}export async function main():Promise<void>{{print(`${{await parent({seed})}}`);}}"
            )
        };
        let unsafe_source = build("x+1");
        let d = ulua_compiler::compile(unsafe_source.as_bytes(), ulua_compiler::Limits::default())
            .unwrap_err();
        assert_eq!(d.code, "C16", "{unsafe_source}: {d:?}");
        suite.source(&build("(x??0)+1"), None);
    }
    // Mix derived callbacks, all, and async map with pending P4.1 activations.
    for i in 0..120 {
        let mut body = String::new();
        for task in 0..3 {
            let delay = r.next() % 5;
            body.push_str(&format!(
                "let p{task}=delay({task},{delay},{});",
                r.next().is_multiple_of(4)
            ));
            for step in 0..1 + r.next() % 5 {
                let callback = match r.next() % 8 {
                    0 => format!(
                        ".then((n:number):number=>{{print('then{task}-{step}');return n+1;}})"
                    ),
                    1 => {
                        format!(
                            ".then((n:number):Promise<number>=>{{print('adopt{task}-{step}');return delay(n,{},false);}}",
                            r.next() % 3
                        ) + ")"
                    }
                    2 => format!(
                        ".then(async(n:number):Promise<number>=>{{await value(n);print('async{task}-{step}');return n+2;}})"
                    ),
                    3 => format!(
                        ".catch((e:unknown):number=>{{if(e instanceof Error)print(e.name);print('catch{task}-{step}');return 9;}})"
                    ),
                    4 => ".catch(async(e:unknown):Promise<number>=>{await value(0);throw e;})"
                        .to_owned(),
                    5 => format!(".finally(():void=>{{print('finally{task}-{step}');}})"),
                    6 => format!(
                        ".finally(async():Promise<void>=>{{await value(0);print('final await{task}-{step}');}})"
                    ),
                    _ => ".finally(():void=>{throw new URIError('final');})".to_owned(),
                };
                body.push_str(&format!("p{task}=p{task}{callback};"));
            }
        }
        body.push_str("const mapped=[1,2,3].map(async(n:number):Promise<number>=>{await value(n);print(`map${n}`);return n*2;});const all=Promise.all([p0,p1,p2]);const shared=Promise.all([p2,p0,p2]);print('sync');for(const p of [all,shared,Promise.all(mapped)]){try{print((await p).join(','));}catch(e){if(e instanceof Error)print(e.name);}}await value(0);print('end');");
        if i % 12 == 0 {
            body.push_str(
                "const lost=value(0).then(():number=>{throw new RangeError('lost derived');});",
            );
        }
        let source = format!(
            "async function value(n:number):Promise<number>{{return n;}}async function delay(n:number,steps:number,fail:boolean):Promise<number>{{print(`start${{n}}`);for(let i=0;i<steps;i++){{await value(i);}}if(fail)throw new TypeError('task');return n;}}export async function main():Promise<void>{{{body}}}"
        );
        if i % 12 == 0 {
            suite.source_async_trap(&source, "RangeError");
        } else {
            suite.source(&source, None);
        }
    }
    // Nullable values cross all helper signatures, with both fulfilled and rejected sources.
    for i in 0..120 {
        let (ty, value, declaration) = [
            ("number", "7", ""),
            ("string", "'value'", ""),
            ("boolean", "false", ""),
            ("number[]", "[7]", ""),
            ("R", "{n:7}", "type R={n:number};"),
            ("Box", "new Box()", "class Box{n:number=7;}"),
            ("(()=>number)", "():number=>7", ""),
            ("Error", "new Error('value')", ""),
        ][i % 8];
        let nullable = format!("{ty}|null");
        let mut body = format!("let p:Promise<{nullable}>=source({});", r.next() % 3);
        for step in 0..2 + r.next() % 6 {
            let callback = match r.next() % 10 {
                0 => format!("then((v:{nullable}):{nullable}=>{{print('then{step}');return v;}})"),
                1 => format!("then(():{nullable}=>{{print('null{step}');return null;}})"),
                2 => format!(
                    "then((v:{nullable}):Promise<{nullable}>=>source({}))",
                    r.next() % 3
                ),
                3 => format!("then(():Promise<{nullable}>=>source({}))", r.next() % 3),
                4 => format!(
                    "catch((e:unknown):{nullable}=>{{print('catch{step}');return {value};}})"
                ),
                5 => format!("catch(():{nullable}=>null)"),
                6 => format!(
                    "catch(async(e:unknown):Promise<{nullable}>=>{{await tick();print('recover{step}');return null;}})"
                ),
                7 => format!("catch(():Promise<{nullable}>=>source({}))", r.next() % 3),
                8 => format!("finally(():void=>{{print('final{step}');}})"),
                _ => format!(
                    "finally(async():Promise<void>=>{{await tick();print('wait{step}');{}}})",
                    if r.next().is_multiple_of(4) {
                        "throw new TypeError('final');"
                    } else {
                        ""
                    }
                ),
            };
            body.push_str(&format!("p=p.{callback};"));
        }
        body.push_str("try{print(`${(await p)===null}`);}catch(e){if(e instanceof Error)print(e.name);}print('end');");
        suite.source(&format!("{declaration}async function tick():Promise<void>{{}}async function source(mode:number):Promise<{nullable}>{{await tick();if(mode===0)return null;if(mode===1)return {value};throw new RangeError('source');}}export async function main():Promise<void>{{{body}}}"),None);
    }
    suite.finish();
    eprintln!("P4.2 nullable random programs=120");
    eprintln!(
        "P4.1 random seed=0x41c1240026 programs={} p42_mixed=120 suspended_capture_programs=120 unsafe_rejects=120 unhandled=20 cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
