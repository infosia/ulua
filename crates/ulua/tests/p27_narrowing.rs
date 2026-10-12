//! Exceptional entries retain only facts that precede all writes in the protected block.
//! Measured Debug target cost: 0.42 seconds.
//! The witnesses report cost and include reject controls for both entry boundaries.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witnesses;
use std::time::Instant;
use ulua_compiler::{Limits, compile};
#[test]
fn exceptional_entry_facts_and_throw_edges() {
    let start = Instant::now();
    let mut suite = witnesses::Suite::named("P2.7 narrowing");
    suite.source("function f(x:number|null):number{if(x==null){throw new Error();}return x;}export function main():void{let a:number|null=1;let b:number|null=2;if(a!=null&&b!=null){try{a=null;throw new Error();}catch(e){print(`${b}`);if(e instanceof Error){print(e.name);}if(a!=null){print(`${a}`);}}finally{print(`${b}`);if(a!=null){print(`${a}`);}}}print(`${f(4)}`);}",Some("2\nError\n2\n4\n"));
    suite.source("export function main():void{try{throw new Error('x');}catch(e){if(!(e instanceof Error)){throw e;}print(e.message);const f=():void=>{print(e.message);};f();}}",Some("x\nx\n"));
    suite.source("class A{v:number;constructor(n:number){try{if(n==0){throw new Error();}this.v=n;}catch{throw new Error('constructor');}finally{print('final');}}}export function main():void{print(`${new A(2).v}`);}",Some("final\n2\n"));
    for body in [
        "throw 42;",
        "throw 'x';",
        "try{}catch(e:any){}",
        "try{}catch(e:Error){}",
        "try{}catch(e){print(e.message);}",
        "const e=new Error();print(e.stack);",
        "Error('x');",
        "new Error('x','y');",
        "try{}catch(e){const x=e;}",
    ] {
        let source = format!("export function main():void{{{body}}}");
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C17",
            "{source}"
        );
    }
    for body in [
        "if(x!=null){try{x=null;throw new Error();}catch{print(`${x+1}`);}}",
        "if(x!=null){try{}catch{x=null;}finally{print(`${x+1}`);}}",
        "if(x!=null){try{x=null;}finally{print(`${x+1}`);}}",
        "try{}catch(e){const f=():void=>{print(e.message);};}",
        "while(true){if(x!=null){try{break;}finally{x=null;}}}print(`${x+1}`);",
        "while(true){if(x!=null){try{continue;}finally{x=null;}}print(`${x+1}`);}",
    ] {
        let source = format!("export function main():void{{let x:number|null=1;{body}}}");
        assert!(
            compile(source.as_bytes(), Limits::default()).is_err(),
            "{source}"
        );
    }
    for body in [
        "this.v=1;throw new Error();",
        "try{throw this;}catch{}",
        "try{throw new Error();}finally{print(`${this.v}`);}",
    ] {
        let source =
            format!("class A{{v:number;constructor(){{{body}}}}}export function main():void{{}}");
        if body.starts_with("this.v") {
            assert!(compile(source.as_bytes(), Limits::default()).is_ok());
        } else {
            assert!(
                compile(source.as_bytes(), Limits::default()).is_err(),
                "{source}"
            );
        }
    }
    suite.source("export function main():void{const fs:(()=>string)[]=[];for(let i=0;i<3;i++){try{throw new Error(`${i}`);}catch(e){if(e instanceof Error){fs.push(():string=>e.message);}}}const a=fs[0];const b=fs[1];const c=fs[2];print(`${a()} ${b()} ${c()}`);}",Some("0 1 2\n"));
    suite.source("class A{v:number;constructor(){try{return;}finally{this.v=3;}}}export function main():void{print(`${new A().v}`);}",Some("3\n"));
    suite.source("export function main():void{let e:Error|null=null;print(e?.message??'empty');e=new Error('x');if(e!=null){e.message+=1;e.name+='!';}print(e?.message??'empty');if(e!=null){print(e.name);}}",Some("empty\nx1\nError!\n"));
    for call in [
        "fail();",
        "new B();",
        "const f:()=>void=fail;f();",
        "const xs:number[]=[1];xs.forEach((n:number):void=>fail());",
    ] {
        let source = format!(
            "function fail():void{{throw new Error();}}class B{{constructor(){{fail();}}}}class A{{v:number;constructor(){{try{{{call}this.v=1;}}finally{{print(`${{this.v}}`);}}}}}}export function main():void{{}}"
        );
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C13",
            "{source}"
        );
    }
    suite.finish();
    eprintln!(
        "P2.7 narrowing accepted={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}
