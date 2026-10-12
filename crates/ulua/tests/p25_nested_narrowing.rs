//! C16.4 tests in nested bodies establish local facts for stable enclosing bindings.
//! Measured Debug cost: 0.49 seconds for 29 accepted and 17 rejected programs.
//! Accepted programs pass node, stock tsc, and luau-analyze; rejection witnesses give TS18047 or no diagnostic.
#[allow(dead_code)]
#[path = "support/p25.rs"]
mod oracle;
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use std::time::Instant;
use witness::Suite;

fn source(binding: &str, body: &str) -> String {
    let binding = if binding == "parameter" {
        ""
    } else {
        "let x:B|null=p;"
    };
    let parameter = if binding.is_empty() { "x" } else { "p" };
    format!(
        "type B={{v:number}};function run({parameter}:B|null):number{{{binding}{body}}}export function main():void{{print(`${{run(null)}}:${{run({{v:3}})}}`);}}"
    )
}

#[test]
fn nested_tests_narrow_stable_enclosing_values() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 nested body tests");
    suite.source(
        include_str!("../../../corpus/accept/a47-nested-body-null-test.ts"),
        Some("1 6 13\n"),
    );
    for binding in ["let", "parameter"] {
        for body in [
            "const f=():number=>x!=null?x.v:0;return f();",
            "const f=():number=>{if(x==null){return 0;}return x.v;};return f();",
            "function f():number{if(x!=null){return x.v;}return 0;}return f();",
            "const f=():number=>{if(x!=null){if(x.v==0){return 0;}else{return x.v;}}return 0;};return f();",
            "const f=():number=>{while(x!=null){return x.v;}return 0;};return f();",
            "const f=():number=>{for(let i=0;i<1;i++){if(x==null){continue;}return x.v;}return 0;};return f();",
            "const f=():number=>{if(x!=null){return x.v;}else{return 0;}};return f();",
            "const f=():number=>{const g=():number=>x==null?0:x.v;return g();};return f();",
            "function f():number{function g():number{if(x==null){return 0;}return x.v;}return g();}return f();",
            "if(x!=null){const f=():number=>{if(x==null){return 0;}return x.v;};return f();}return 0;",
            "const f=():number=>{if(x==null){return 0;}return x.v;};const n=f();x=null;return n;",
        ] {
            suite.source(&source(binding, body), Some("0:3\n"));
        }
        suite.source(
            &source(binding, "const f=():number=>x==null||x.v>0?1:0;return f();"),
            Some("1:1\n"),
        );
        suite.source(
            &source(binding, "const f=():number=>x!=null&&x.v>0?1:0;return f();"),
            Some("0:1\n"),
        );
        suite.source(
            &source(
                binding,
                "const f=(x:B|null):number=>{if(x==null){return 0;}return x.v;};return f(x);",
            ),
            Some("0:3\n"),
        );
    }
    suite.finish();
    eprintln!(
        "P2.5 nested body acceptance cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn inherited_mutable_facts_and_unreachable_reads_stay_rejected() {
    let start = Instant::now();
    let mut programs = vec![];
    for binding in ["let", "parameter"] {
        for body in [
            "if(x!=null){const f=():number=>x.v;return f();}return 0;",
            "if(x!=null){function f():number{return x.v;}return f();}return 0;",
            "const f=():number=>{if(x!=null){const g=():number=>x.v;return g();}return 0;};return f();",
            "const f=():number=>{if(x==null){return 0;}return 1;print(`${x.v}`);};return f();",
            "const f=():number=>{if(x!=null){}return x.v;};return f();",
        ] {
            programs.push(source(binding, body));
        }
    }
    let cases: Vec<_> = programs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                s.as_str(),
                if matches!(i % 5, 0 | 2) {
                    &[][..]
                } else {
                    &[18047u32][..]
                },
            )
        })
        .collect();
    oracle::rejected_programs("nested-body-limits", "C16", &cases);
    let mut programs = vec!["type B={v:number};let x:B|null={v:1};export function main():void{const f=():number=>{if(x==null){return 0;}return x.v;};print(`${f()}`);}".to_owned()];
    for binding in ["let", "parameter"] {
        for body in [
            "const clear=():void=>{x=null;};const f=():number=>{if(x==null){return 0;}return x.v;};return f();",
            "function clear():void{x=null;}function f():number{if(x==null){return 0;}return x.v;}return f();",
            "const f=():number=>{if(x==null){return 0;}x=null;return x.v;};return f();",
        ] {
            programs.push(source(binding, body));
        }
    }
    let cases: Vec<_> = programs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                s.as_str(),
                if i == 3 || i == 6 {
                    &[18047u32][..]
                } else {
                    &[][..]
                },
            )
        })
        .collect();
    oracle::rejected_programs("nested-body-writers", "C16", &cases);
    eprintln!(
        "P2.5 nested body rejection cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
