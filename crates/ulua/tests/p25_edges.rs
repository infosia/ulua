//! C16.9 keeps contradictory null-test edges and checks the original literal syntax.
//! Measured Debug cost: 0.48 seconds, including tsc, Node, and Luau analysis.
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
const PRELUDE: &str = "type B={v:number};function mk():B|null{return {v:2};}";
fn source(body: &str) -> String {
    format!(
        "{PRELUDE}export function main():void{{let out=0;let a:B|null=mk();const k:B|null=mk();const c2:B|null=mk();{body}}}"
    )
}
#[test]
fn contradictory_tests_keep_all_join_edges() {
    let start = Instant::now();
    let bodies = [
        "if(k!=null){if(c2!=null){}else if(k!=null){return;}out+=c2.v;}",
        "if(k!=null){if(a!=null){if(k==null){a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if(k===null){a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if(k!=null){out+=0;}else{a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if(k==null&&out>0){a=null;}out+=a.v;}}",
        "if(k!=null){if((k==null&&c2==null)||c2!=null){out+=c2.v;}}",
        // This conditional is a value; its logical test still joins both c2 edges.
        "if(k!=null){out+=((k==null&&c2==null)||c2!=null)?c2.v:0;}",
        "if(k!==null){if(a!==null){if(null===k){a=null;}out+=a.v;}}",
        "if(k!==null){if(a!==null){if(k!==null){}else{a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if((false)){a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if(!true){a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if((true)){}else{a=null;}out+=a.v;}}",
        "if(k!=null){if(a!=null){if(!false){}else{a=null;}out+=a.v;}}",
    ];
    let programs: Vec<_> = bodies.iter().map(|body| source(body)).chain([
        format!("{PRELUDE}function g(p:B|null,q:B|null):number{{if(p!=null){{if(q!=null){{if(p==null){{q=null;}}return q.v;}}}}return 0;}}export function main():void{{print(`${{g(mk(),mk())}}`);}}"),
    ]).collect();
    let cases: Vec<_> = programs
        .iter()
        .map(|s| (s.as_str(), &[18047u32][..]))
        .chain([(
            include_str!("../../../corpus/reject/r109-contradicting-null-test.ts"),
            &[18047][..],
        )])
        .collect();
    oracle::rejected_programs("contradictory-edges", "C16", &cases);
    eprintln!(
        "P2.5 contradictory edges cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn wrapped_literals_and_other_facts_match_all_witnesses() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 literal edges");
    for body in [
        "if(k==null){return;}if((false)){out+=k.v;}print(`${out}`);",
        "if(k==null){return;}if(!true){out+=k.v;}print(`${out}`);",
        "if(k==null){return;}if(!false){}else{out+=k.v;}print(`${out}`);",
        "if(k==null){return;}if((true)){}else{out+=k.v;}print(`${out}`);",
        "print('start');if(k==null){return;}while((true)){return;}out+=k.v;",
        "print('start');if(k==null){return;}while(!false){return;}out+=k.v;",
        "print('start');if(k==null){return;}for(;(true);){return;}out+=k.v;",
        "print('start');if(k==null){return;}for(;!false;){return;}out+=k.v;",
        "if(k==null){return;}out+=(false)?k.v:0;print(`${out}`);",
        "if(k==null){return;}print(`${(false)&&k.v>0}`);",
        "if(k==null){return;}print(`${(true)||k.v>0}`);",
        "if(k==null){return;}print(`${!true&&k.v>0}`);",
        "if(k==null){return;}print(`${!false||k.v>0}`);",
        "if(k==null){return;}out+=(true?false:true)?k.v:0;print(`${out}`);",
        "if(k==null){return;}if(false?true:false){out+=k.v;}print(`${out}`);",
        "if(k==null){return;}print(`${!((true?true:false))&&k.v>0}`);",
        // Contradictory tests keep the other facts, and do not erase their own known type.
        "if(k!=null&&c2!=null){if(k==null){out+=c2.v;}out+=k.v;}print(`${out}`);",
        "if(k!=null&&c2!=null){if(k==null){out+=k?.v??0;out+=c2.v;}out+=k.v;}print(`${out}`);",
        "if(k!=null&&a!=null){if(k==null){out+=a.v;}out+=k.v;}print(`${out}`);",
        "if(k!=null&&c2!=null){out+=(k==null)?c2.v:k.v;}print(`${out}`);",
        "if(k!=null&&a!=null){if(false){a=null;}out+=a.v;}print(`${out}`);",
        "print('start');if(k!=null&&c2!=null){while(true){if(k==null){break;}return;}out+=c2.v;}",
        "print('start');if(k!=null&&c2!=null){for(;;){if(k==null){break;}return;}out+=c2.v;}",
        "print('start');if(k!=null&&c2!=null){for(let i=0;i<1;out+=c2.v){if(k==null){continue;}return;}}",
        // A wrapped false makes the break reachable for C16, but the runtime returns.
        "print('start');if(k!=null){while(true){if((false)){break;}return;}out+=k.v;}",
    ] {
        suite.source(&source(body), None);
    }
    suite.finish();
    eprintln!(
        "P2.5 literal edges cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn result_completion_stays_stricter_than_reachability() {
    let cases = [
        (
            "function f():number{while((true)){return 1;}}export function main():void{}",
            &[2366u32][..],
        ),
        (
            "function f():number{while(!false){return 1;}}export function main():void{}",
            &[2366][..],
        ),
        (
            "function f():number{for(;(true);){return 1;}}export function main():void{}",
            &[2366][..],
        ),
        (
            "function f():number{while(true){if(false){break;}return 1;}}export function main():void{}",
            &[][..],
        ),
        (
            "function f():number{for(;;){if(false){break;}return 1;}}export function main():void{}",
            &[][..],
        ),
    ];
    oracle::rejected_programs("result-completion", "C11", &cases);
}
