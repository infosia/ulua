//! Nullable promise values retain their types through every derived helper.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::time::Instant;

#[test]
fn nullable_promise_helpers_match_all_witnesses() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.2 nullable direct");
    for (ty, value, declaration) in [
        ("number", "7", ""),
        ("string", "'value'", ""),
        ("boolean", "false", ""),
        ("number[]", "[7]", ""),
        ("R", "{n:7}", "type R={n:number};"),
        ("Box", "new Box()", "class Box{n:number=7;}"),
        ("(()=>number)", "():number=>7", ""),
        ("Error", "new Error('value')", ""),
    ] {
        let nullable = format!("{ty}|null");
        let functions = format!(
            "{declaration}async function source(mode:number):Promise<{nullable}>{{if(mode===0)return null;if(mode===1)return {value};throw new RangeError('source');}}async function done():Promise<void>{{}}"
        );
        for mode in 0..3 {
            for (callback, golden) in [
                (
                    format!("then((v:{nullable}):{nullable}=>v)"),
                    if mode == 2 {
                        "RangeError\n"
                    } else if mode == 0 {
                        "true\n"
                    } else {
                        "false\n"
                    },
                ),
                (
                    format!("then(():{nullable}=>null)"),
                    if mode == 2 { "RangeError\n" } else { "true\n" },
                ),
                (
                    format!("then((v:{nullable}):Promise<{nullable}>=>source(0))"),
                    if mode == 2 { "RangeError\n" } else { "true\n" },
                ),
                (
                    format!("then(():Promise<{nullable}>=>source(1))"),
                    if mode == 2 { "RangeError\n" } else { "false\n" },
                ),
                (
                    format!("catch((e:unknown):{nullable}=>null)"),
                    if mode == 1 { "false\n" } else { "true\n" },
                ),
                (
                    format!("catch(():{nullable}=>({value}))"),
                    if mode == 0 { "true\n" } else { "false\n" },
                ),
                (
                    format!("catch((e:unknown):Promise<{nullable}>=>source(0))"),
                    if mode == 1 { "false\n" } else { "true\n" },
                ),
                (
                    format!("catch(():Promise<{nullable}>=>source(1))"),
                    if mode == 0 { "true\n" } else { "false\n" },
                ),
                (
                    format!(
                        "catch(async(e:unknown):Promise<{nullable}>=>{{await done();return null;}})"
                    ),
                    if mode == 1 { "false\n" } else { "true\n" },
                ),
                (
                    format!("catch(async():Promise<{nullable}>=>{{await done();return {value};}})"),
                    if mode == 0 { "true\n" } else { "false\n" },
                ),
                (
                    "finally(():void=>{})".to_owned(),
                    if mode == 2 {
                        "RangeError\n"
                    } else if mode == 0 {
                        "true\n"
                    } else {
                        "false\n"
                    },
                ),
                (
                    "finally(():Promise<void>=>done())".to_owned(),
                    if mode == 2 {
                        "RangeError\n"
                    } else if mode == 0 {
                        "true\n"
                    } else {
                        "false\n"
                    },
                ),
                (
                    "finally(async():Promise<void>=>{await done();})".to_owned(),
                    if mode == 2 {
                        "RangeError\n"
                    } else if mode == 0 {
                        "true\n"
                    } else {
                        "false\n"
                    },
                ),
            ] {
                suite.source(&format!("{functions}export async function main():Promise<void>{{const p=source({mode}).{callback};try{{print(`${{(await p)===null}}`);}}catch(e){{if(e instanceof Error)print(e.name);}}}}"),Some(golden));
            }
            for callback in [
                format!("then((v:{nullable}):void=>{{}})"),
                "then(():void=>{})".to_owned(),
            ] {
                suite.source(&format!("{functions}export async function main():Promise<void>{{const p=source({mode}).{callback};try{{await p;print('void');}}catch(e){{if(e instanceof Error)print(e.name);}}}}"), Some(if mode == 2 {"RangeError\n"} else {"void\n"}));
            }
        }
    }
    // An inferred Promise<null> uses the nil constructor and the same derived helpers.
    suite.source("async function nul():Promise<number|null>{return null;}async function done():Promise<void>{}function show(n:number|null):void{print(`${n===null}`);}export async function main():Promise<void>{const a=nul().then(()=>null);const b=a.catch((e:unknown)=>null);const c=b.catch(()=>a);const d=c.finally(():void=>{});const e=d.finally(():Promise<void>=>done());show(await e);}",Some("true\n"));
    suite.finish();
    eprintln!(
        "P4.2 nullable direct programs={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn floating_promise_containers_are_rejected() {
    let start = Instant::now();
    for body in [
        "v();",
        "[1].map(async(n:number):Promise<number>=>n);",
        "[1].map((n:number):Promise<number>=>v());",
        "list();",
        "record();",
        "nested();",
        "optional();",
        "new Box();",
        "new RecordBox();",
        "await container();",
    ] {
        let source = format!(
            "async function v():Promise<number>{{return 1;}}function list():Promise<number>[]{{return [v()];}}type R={{p:Promise<number>}};function record():R{{return {{p:v()}};}}type N={{items:R[]}};function nested():N{{return {{items:[record()]}};}}function optional():R|null{{return null;}}class Box{{p:Promise<number>=v();}}class RecordBox{{r:R=record();}}async function container():Promise<R>{{return record();}}export async function main():Promise<void>{{{body}}}"
        );
        support::Script::compile(&source, ulua::CompileLimits::default(), "C19");
    }
    let mut suite = witnesses::Suite::named("P4.2 floating controls");
    suite.source("async function v():Promise<number>{return 1;}function callable():()=>Promise<number>{print('callable');return v;}export function main():void{callable();}",Some("callable\n"));
    suite.source("class Game{score:number=0;constructor(){print('start');}async load():Promise<number>{return 1;}}export function main():void{new Game();}",Some("start\n"));
    suite.source("function factory():()=>Promise<number>{print('factory');return async():Promise<number>=>1;}export function main():void{factory();}",Some("factory\n"));
    suite.source("async function v():Promise<number>{return 1;}function list():Promise<number>[]{return [v()];}type R={p:Promise<number>};function record():R{return {p:v()};}function numbers():number[]{return [1];}async function done():Promise<void>{}export async function main():Promise<void>{numbers();const xs=[1].map(async(n:number):Promise<number>=>n);const ys=list();const r=record();print(`${await xs[0]} ${await ys[0]} ${await r.p}`);await done();}",Some("1 1 1\n"));
    suite.finish();
    eprintln!(
        "P4.2 floating containers rejections=10 controls={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
