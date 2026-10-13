//! C16.8 rejects every arrow at a point with no incoming edge.
//! Measured single-worker Debug cost: 0.82 seconds.
//! The target reports its cost and compares catch reads with stock tsc.
#[allow(dead_code)]
#[path = "../support/p25.rs"]
mod oracle;
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witness;
use std::time::Instant;
use ulua_compiler::{Limits, compile};

fn source(body: &str) -> String {
    format!(
        "function take(f:()=>number):void{{print(`${{f()}}`);}}export function main():void{{let a=0;{body}}}"
    )
}

#[test]
fn every_unreachable_arrow_position_is_rejected() {
    let start = Instant::now();
    let contexts = [
        "return;ARROW",
        "throw new Error();ARROW",
        "while(a<1){break;ARROW}",
        "while(a<1){a++;continue;ARROW}",
        "if(a>0){return;}else{return;}ARROW",
        "while(true){}ARROW",
        "while(true||a>0){}ARROW",
        "while(true){if(false){break;}}ARROW",
        "while(a<1){if(a>0){break;}else{continue;}ARROW}",
        "for(;;){}ARROW",
        "if(false){ARROW}",
        "if(true){}else{ARROW}",
        "while(false){ARROW}",
        "for(let i=0;i<1;ARROW){break;}",
        "for(let i=0;i<1;ARROW){return;}",
        "for(const n of [1]){continue;ARROW}",
        "try{throw new Error();ARROW}catch{}",
        "try{throw new Error();}catch{throw new Error();ARROW}",
        "try{}finally{return;ARROW}",
    ];
    // A call, array, template, and conditional all use the same arrow entry check.
    let statements = [
        "const f=():number=>1;",
        "take(():number=>1);",
        "print(`${(():number=>1)()+1}`);",
        "const fs=[():number=>1];",
        "const f=a>0?():number=>1:():number=>2;",
    ];
    let mut count = 0;
    for context in contexts {
        for statement in statements {
            if context.contains(";ARROW)") && statement != statements[1] {
                continue;
            }
            let statement = if context.contains(";ARROW)") {
                "a=(():number=>1)()"
            } else {
                statement
            };
            let program = source(&context.replace("ARROW", statement));
            let d = compile(program.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(d.code, "C16", "{program}: {d:?}");
            assert_eq!(
                d.message,
                "an arrow function at a point that no edge reaches"
            );
            assert_eq!(&program[d.offset as usize..][..1], "(", "{d:?}");
            count += 1;
        }
    }
    for expression in [
        "false&&(():boolean=>true)()",
        "true||(():boolean=>true)()",
        "false?(():number=>1)():0",
        "true?0:(():number=>1)()",
        "(true||a>0)?0:(():number=>1)()",
    ] {
        let program = source(&format!("print(`${{{expression}}}`);"));
        let d = compile(program.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(d.code, "C16", "{program}: {d:?}");
        assert_eq!(
            d.message,
            "an arrow function at a point that no edge reaches"
        );
        count += 1;
    }
    for body in [
        "return;for(let i=(():number=>0)();i<1;i++){}",
        "return;for(let i=0;(():boolean=>true)();i++){}",
        "for(let i=0;false&&(():boolean=>true)();i++){}",
        "for(let i=0;i<1;take(():number=>1)){break;}",
        "for(let i=0;i<1;(():number=>1)()){return;}",
        "for(let i=0;i<1;():number=>1){break;}",
    ] {
        let program = source(body);
        let d = compile(program.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(d.code, "C16", "{program}: {d:?}");
        assert_eq!(
            d.message,
            "an arrow function at a point that no edge reaches"
        );
        count += 1;
    }
    // C9 still rejects a reached closure in a for head.
    for body in [
        "for(let i=(():number=>0)();i<1;i++){}",
        "for(let i=0;(():boolean=>false)();i++){}",
        "for(let i=0;i<1;i=(():number=>1)()){}",
    ] {
        let program = source(body);
        assert_eq!(
            compile(program.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C9"
        );
    }
    for class in [
        "class A{v:number=false?(():number=>1)():0;}export function main():void{}",
        "class A{f:()=>number=false?():number=>1:():number=>2;}export function main():void{}",
    ] {
        let d = compile(class.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(d.code, "C16");
        assert_eq!(
            d.message,
            "an arrow function at a point that no edge reaches"
        );
        count += 1;
    }
    oracle::rejected_programs(
        "unreachable-arrow-corpus",
        "C16",
        &[
            (
                include_str!("../../../../corpus/reject/r124-unreachable-arrow.ts"),
                &[18046],
            ),
            (
                include_str!("../../../../corpus/reject/r125-unreachable-arrow-null.ts"),
                &[18047],
            ),
            (&source("return;const f=():number=>1;"), &[]),
        ],
    );
    eprintln!(
        "P2.7 unreachable arrows rejected={} cost={:.3}s",
        count + 3,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn reachable_arrow_controls_keep_their_edges() {
    let start = Instant::now();
    let mut suite = witness::Suite::named("P2.7 reachable arrows");
    for body in [
        "take(():number=>1);return;",
        "try{take(():number=>1);throw new Error();}catch{}",
        "while(a<1){take(():number=>1);break;}",
        "while(a<1){a++;take(():number=>1);continue;}",
        "if(a>0){return;}take(():number=>1);",
        "while(true){break;}take(():number=>1);",
        "for(;;){break;}take(():number=>1);",
        "if(true){take(():number=>1);}",
        "if(false){}else{take(():number=>1);}",
        "while(a<1){a++;take(():number=>1);}",
        "const f=():number=>1;for(let i=0;i<1;a=f()){i++;continue;}",
        "for(const n of [1]){take(():number=>1);continue;}",
        "try{throw new Error();}catch{take(():number=>1);}",
        "try{}finally{take(():number=>1);return;}",
        "const f=():number=>1;const fs=[f];const g=fs[0];print(`${g()}`);",
        "print(`${true&&(():boolean=>true)()}:${false||(():boolean=>true)()}`);",
        "print(`${true?(():number=>1)():0}:${false?0:(():number=>1)()}`);",
        // C16.9 gives both edges to a parenthesized literal leaf.
        "if((false)){take(():number=>1);}print('ok');",
    ] {
        suite.source(&source(body), None);
    }
    suite.source("class A{v:number=true?(():number=>1)():0;}export function main():void{print(`${new A().v}`);}", Some("1\n"));
    suite.source("class A{f:()=>number=():number=>1;}export function main():void{const f=new A().f;print(`${f()}`);}", Some("1\n"));
    suite.finish();
    eprintln!(
        "P2.7 reachable arrows accepted={} cost={:.3}s",
        suite.count,
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn unreachable_instanceof_does_not_narrow_a_catch_binding() {
    let start = Instant::now();
    let contexts = [
        "return;READ",
        "throw new Error();READ",
        "while(a<1){break;READ}",
        "while(a<1){a++;continue;READ}",
        "if(a>0){return;}else{return;}READ",
        "while(true){}READ",
        "for(;;){}READ",
        "if(false){READ}",
        "if(true){}else{READ}",
        "print(`${false&&(e instanceof Error&&e.message.length>0)}`);",
        "print(`${true||!(e instanceof Error&&e.message.length>0)}`);",
        "for(let i=0;i<1;a=e instanceof Error?e.message.length:0){break;}",
        "for(let i=0;i<1;a=e instanceof Error?e.message.length:0){return;}",
    ];
    let programs: Vec<_> = contexts
        .iter()
        .map(|context| {
            source(&format!(
                "try{{throw new Error('x');}}catch(e){{{}}}",
                context.replace("READ", "if(e instanceof Error){print(e.message);}")
            ))
        })
        .collect();
    let cases: Vec<_> = programs
        .iter()
        .map(|program| (program.as_str(), &[18046u32][..]))
        .collect();
    oracle::rejected_programs("unreachable-instanceof", "C17", &cases);
    eprintln!(
        "P2.7 unreachable instanceof rejected={} cost={:.3}s",
        cases.len(),
        start.elapsed().as_secs_f64()
    );
}
