//! C16 uses the same cached test edges in statements, expressions, and loop exits.
//! Single-worker Debug cost: 0.57 seconds for this test target.
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
const PRELUDE: &str = "type R={v:number};function mk(n:number):R|null{return n>0?{v:n}:null;}";
fn source(body: &str) -> String {
    format!(
        "{PRELUDE}export function main():void{{const x=mk(1);const y=mk(2);const c=mk(3)!=null;let out=0;{body}}}"
    )
}

#[test]
fn logical_test_endings_apply_in_every_position() {
    let start = Instant::now();
    let bodies = [
        "if(x==null){return;}while(true||c){out+=1;if(out>3){return;}}out+=x.v;",
        "if(x==null){return;}for(;true||c;){return;}out+=x.v;",
        "if(x==null){return;}while(!(false&&c)){return;}out+=x.v;",
        "if(x==null){return;}while((true||c)){return;}out+=x.v;",
        "if(x==null){return;}while(c||true){return;}out+=x.v;",
        "if(x==null){return;}for(let i=0;true||c;i+=1){return;}out+=x.v;",
    ];
    let mut programs: Vec<_> = bodies.iter().map(|body| source(body)).collect();
    for test in ["true||c", "!(false&&c)", "(true||c)", "c||true"] {
        for body in [
            format!("if(x==null){{return;}}if({test}){{return;}}out+=x.v;"),
            format!("if(x==null){{return;}}out+=({test})?0:x.v;"),
            format!("if(x==null){{return;}}print(`${{({test})||x.v>0}}`);"),
            format!("if(x==null){{return;}}while({test}){{continue;}}out+=x.v;"),
            // Nested breaks do not make an exit of this loop.
            format!("if(x==null){{return;}}while({test}){{while(true){{break;}}return;}}out+=x.v;"),
        ] {
            programs.push(source(&body));
        }
    }
    let cases: Vec<_> = programs
        .iter()
        .map(|s| (s.as_str(), &[18047u32][..]))
        .chain([(
            include_str!("../../../corpus/reject/r110-logical-literal-loop-exit.ts"),
            &[18047][..],
        )])
        .collect();
    oracle::rejected_programs("logical-test-positions", "C16", &cases);
    eprintln!(
        "P2.5 logical test positions cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn conditional_tests_narrow_nothing() {
    let start = Instant::now();
    let mut programs = vec![];
    for test in [
        "c?x!=null:false",
        "c?x!=null:x!=null",
        "x!=null?true:false",
        "x==null?false:true",
        "!(x==null?true:false)",
        "x!=null?y!=null:false",
        "x==null?false:y!=null",
        "c?false:x!=null",
    ] {
        let read = if test.contains("y!=null") {
            "x.v+y.v"
        } else {
            "x.v"
        };
        programs.push(source(&format!("if({test}){{out+={read};}}")));
    }
    for body in [
        "while(c?x!=null:false){out+=x.v;break;}",
        "for(;x!=null?true:false;){out+=x.v;break;}",
        "out+=(c?x!=null:false)?x.v:0;",
        "print(`${(c?x!=null:false)&&x.v>0}`);",
        "print(`${!(c?x==null:true)||x.v>0}`);",
        "if((c?x!=null:false)&&y!=null){out+=x.v+y.v;}",
        "if(c?x==null:true){return;}out+=x.v;",
        // The old nested nullable growth test needs a separate null guard.
        "if((x!=null?true:false)?true:false){out+=x.v;}",
    ] {
        programs.push(source(body));
    }
    let cases: Vec<_> = programs
        .iter()
        .map(|s| (s.as_str(), &[18047u32][..]))
        .chain([(
            include_str!("../../../corpus/reject/r111-conditional-test-narrowing.ts"),
            &[18047][..],
        )])
        .collect();
    oracle::rejected_programs("conditional-test-positions", "C16", &cases);
    // TypeScript narrows these literal initializers; Ulua still requires a direct null test.
    let former_controls = [
        source("const z:R|null={v:2};if(true?z!=null:false){out+=z.v;}"),
        source("const z:R|null={v:2};out+=(z==null?false:true)?z.v:0;"),
        source(
            "const z:R|null={v:2};const w:R|null={v:3};if(z===null?false:w===null?false:true){out+=z.v+w.v;}",
        ),
    ];
    let cases: Vec<_> = former_controls
        .iter()
        .map(|s| (s.as_str(), &[][..]))
        .collect();
    oracle::rejected_programs("conditional-literal-initializers", "C16", &cases);
    eprintln!(
        "P2.5 conditional test positions cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn logical_loop_exits_and_conditional_values_match_all_witnesses() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 test position controls");
    for test in ["(true)", "!(false)", "!!true", "(true)||c", "c?true:true"] {
        for body in [
            format!("print('start');if(x==null){{return;}}while({test}){{return;}}out+=x.v;"),
            format!(
                "print('start');if(x==null){{return;}}for(let i=0;{test};i+=1){{return;}}out+=x.v;"
            ),
            format!("print('start');if(x==null){{return;}}if({test}){{return;}}out+=x.v;"),
            format!("if(x==null){{return;}}out+=({test})?0:x.v;print(`${{out}}`);"),
            format!("if(x==null){{return;}}print(`${{({test})||x.v>0}}`);"),
        ] {
            suite.source(&source(&body), None);
        }
    }
    for body in [
        "out+=(x!=null?x.v:0);print(`${out}`);",
        "out+=(x==null?0:x.v);print(`${out}`);",
        "out+=(x!=null&&y!=null)?x.v+y.v:0;print(`${out}`);",
        "out+=c?(x!=null?x.v:0):(y!=null?y.v:0);print(`${out}`);",
        "if(x==null){return;}if(c?x!=null:false){out+=x.v;}print(`${out}`);",
        // A reachable break restores the loop exit even when the test has no false edge.
        "if(x==null){return;}while(true||c){out+=1;if(out>3){break;}}out+=x.v;print(`${out}`);",
        "if(x==null){return;}for(let i=0;true||c;i+=1){if(i>2){break;}}out+=x.v;print(`${out}`);",
        "if(x==null){return;}while(!(false&&c)){break;}out+=x.v;print(`${out}`);",
    ] {
        suite.source(&source(body), None);
    }
    suite.finish();
    eprintln!(
        "P2.5 test position controls cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
