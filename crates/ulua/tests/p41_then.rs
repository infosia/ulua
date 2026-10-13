//! C19 rejects a top-level then member, including forward and inferred types.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;
use ulua::CompileLimits;

#[test]
fn promise_results_have_no_then_member() {
    let start = Instant::now();
    let definitions = [
        "type T={then:(resolve:(n:number)=>void)=>void};",
        "type T={then:number};",
        "class T{then(resolve:(n:number)=>void):void{resolve(42);}}",
        "class T{then:()=>number=():number=>42;}",
        "class T{then:number=42;}",
        "class T{private then():number{return 42;}}",
    ];
    let uses = [
        "function f(p:Promise<T>):void{}",
        "function f(p:Promise<T|null>):void{}",
        "type Holder={p:Promise<T>};",
        "type Holder={ps:Promise<T>[]};",
        "type Holder={f:()=>Promise<T>};",
        "class Holder{async f():Promise<T>{throw new Error();}}",
        "async function f():Promise<T>{throw new Error();}",
        "const f=async():Promise<T>=>{throw new Error();};",
    ];
    for definition in definitions {
        for usage in uses {
            // Put each use before its definition to exercise the complete alias seal.
            support::Script::compile(
                &format!("{usage}{definition}export function main():void{{}}"),
                CompileLimits::default(),
                "C19",
            );
        }
    }
    let returns = [
        "type R={n:number};async function f():Promise<R>{const r={n:1,then:42};return r;}",
        "type R={n:number};async function f():Promise<R>{return {n:1,then:42};}",
        "type R={n:number};const f=async():Promise<R>=>({n:1,then:42});",
        "class T{then():void{}}async function f():Promise<number>{return new T();}",
        "async function f():Promise<number>{return {then:(n:number):number=>n};}",
        "class T{then:number=1;}async function f():Promise<number>{return new T();}",
    ];
    for source in returns {
        support::Script::compile(
            &format!("{source}export function main():void{{}}"),
            CompileLimits::default(),
            "C19",
        );
    }
    eprintln!(
        "P4.1 then rejection cases={} cost={:.3}s",
        definitions.len() * uses.len() + returns.len(),
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn nested_then_members_keep_their_values() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P4.1 nested then");
    suite.source("type Inner={then:number};type Outer={inner:Inner};async function f():Promise<Outer>{return {inner:{then:42}};}export async function main():Promise<void>{const p=f();const r=await p;print(`${r.inner.then} ${(await p).inner.then}`);}",Some("42 42\n"));
    suite.source("type Inner={then:()=>number};type Outer={inner:Inner};async function f():Promise<Outer>{return {inner:{then:():number=>42}};}export async function main():Promise<void>{const r=await f();print(`${r.inner.then()}`);}",Some("42\n"));
    suite.source("class Inner{then():number{return 42;}}type Outer={inner:Inner};async function f():Promise<Outer>{return {inner:new Inner()};}export async function main():Promise<void>{const r=await f();print(`${r.inner.then()}`);}",Some("42\n"));
    suite.source("type Inner={then:number};async function f():Promise<Inner[]>{return [{then:42}];}export async function main():Promise<void>{const xs=await f();print(`${xs[0].then}`);}",Some("42\n"));
    suite.finish();
    eprintln!(
        "P4.1 nested then cases={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
