//! C16.9 checks coalescing operands and preserves facts from the other edges at contradictions.
//! Measured Debug cost: 0.75 seconds, including tsc, Node, the ABI, and luau-analyze.
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
    let (parameter, declaration) = match binding {
        "parameter" => ("x", "".to_owned()),
        _ => ("p", format!("{binding} x:B|null=p;")),
    };
    format!(
        "type B={{v:number}};function f({parameter}:B|null,c:boolean,bn:boolean|null):void{{{declaration}let out=0;{body}}}export function main():void{{f({{v:2}},false,false);f({{v:3}},true,true);f(null,false,null);f({{v:4}},false,null);}}"
    )
}

fn positions(test: &str) -> Vec<String> {
    vec![
        format!("if({test}){{return;}}out+=x.v;"),
        format!("while({test}){{out+=1;if(out>3){{return;}}}}out+=x.v;"),
        format!("for(;{test};){{return;}}out+=x.v;"),
        format!("if({test}){{}}else{{out+=x.v;}}"),
        format!("if(!({test})){{out+=x.v;}}"),
        format!("out+=({test})?0:x.v;"),
        format!("const t:boolean=({test})||x.v>0;print(`${{t}}`);"),
        format!("while(true){{if({test}){{out+=1;if(out>3){{return;}}continue;}}out+=x.v;break;}}"),
    ]
}

#[test]
fn coalescing_requires_both_right_operand_edges_in_every_position() {
    let start = Instant::now();
    let mut programs = vec![];
    for right in ["(true||c)", "!(false&&c)"] {
        for body in positions(&format!("bn??{right}")) {
            programs.push(source("const", &format!("if(x==null){{return;}}{body}")));
        }
    }
    let cases: Vec<_> = programs
        .iter()
        .map(|s| (s.as_str(), &[18047u32][..]))
        .chain([(
            include_str!("../../../corpus/reject/r115-coalesce-test-edge.ts"),
            &[18047][..],
        )])
        .collect();
    oracle::rejected_programs("coalesce-reviewer-edges", "C16", &cases);

    let mut programs = vec![];
    // The restriction also covers values without a nullable read, and either missing edge.
    for right in ["(true||c)", "!(false&&c)", "(false&&c)", "!(true||c)"] {
        for value in [
            format!("const t:boolean=bn??{right};print(`${{t}}`);"),
            format!("print(`${{!(bn??{right})}}`);"),
            format!("print(`${{(bn??{right})&&c}}`);"),
            format!("print(`${{c||(bn??{right})}}`);"),
            format!("print(`${{c&&(bn??{right})}}`);"),
            format!("out+=(bn??{right})?1:0;"),
            format!("const t:boolean|null=null;print(`${{t??(bn??{right})}}`);"),
        ] {
            programs.push(source("const", &value));
        }
    }
    let cases: Vec<_> = programs.iter().map(|s| (s.as_str(), &[][..])).collect();
    oracle::rejected_programs("coalesce-value-edges", "C16", &cases);
    eprintln!(
        "P2.5 coalesce edge rejects cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn coalescing_tests_keep_no_new_narrowing() {
    let programs = [
        source("const", "if(bn??(x!=null)){out+=x.v;}"),
        source("const", "print(`${(bn??(x!=null))&&x.v>0}`);"),
        // TypeScript copies the right operand's false-edge narrowing, although bn can be false.
        source("const", "if(bn??(x==null)){return;}out+=x.v;"),
        source("const", "out+=(bn??(x==null))?0:x.v;"),
    ];
    let cases = [
        (programs[0].as_str(), &[18047u32][..]),
        (programs[1].as_str(), &[18047][..]),
        (programs[2].as_str(), &[][..]),
        (programs[3].as_str(), &[][..]),
    ];
    oracle::rejected_programs("coalesce-leaf-narrowing", "C16", &cases);
}

#[test]
fn coalescing_literal_controls_match_all_witnesses() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 coalesce controls");
    for right in [
        "true",
        "false",
        "c",
        "(true)",
        "!(false)",
        "(true)||c",
        "c?true:true",
    ] {
        for body in positions(&format!("bn??({right})")) {
            suite.source(
                &source(
                    "const",
                    &format!("if(x==null){{return;}}{body}print(`${{out}}`);"),
                ),
                None,
            );
        }
        suite.source(
            &source(
                "const",
                &format!("const t:boolean=bn??({right});print(`${{t}}`);"),
            ),
            None,
        );
    }
    // Direct literal operands keep both edges, too.
    for right in ["true", "false"] {
        suite.source(
            &source(
                "const",
                &format!("if(x==null){{return;}}if(bn??{right}){{return;}}print(`${{x.v}}`);"),
            ),
            None,
        );
    }
    suite.finish();
    eprintln!(
        "P2.5 coalesce controls cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn contradicted_references_use_the_other_join_edges() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 contradicted references");
    for binding in ["const", "let", "parameter"] {
        for body in [
            "if(x!=null||x!=null){out+=x.v;}",
            "if(x==null&&x==null){}else{out+=x.v;}",
            "if(x==null&&x==null){return;}out+=x.v;",
            "if(!(x!=null||x!=null)){return;}out+=x.v;",
            "if(!(x==null&&x==null)){out+=x.v;}",
            "out+=(x!=null||x!=null)?x.v:0;",
            "out+=(x==null&&x==null)?0:x.v;",
            "print(`${(x!=null||x!=null)&&x.v>0}`);",
            "print(`${(x==null&&x==null)||x.v>0}`);",
            "while(x!=null||x!=null){out+=x.v;break;}",
            "while(x==null&&x==null){return;}out+=x.v;",
            "for(;x!=null||x!=null;){out+=x.v;break;}",
            "for(;x==null&&x==null;){return;}out+=x.v;",
            "if(x!=null||x!=null){for(let i=0;i<1;out+=x.v){i+=1;continue;}}",
            "if(x!=null||x!=null){x.v+=1;}",
            "if((x!=null||x!=null)||x!=null){out+=x.v;}",
            "if(x!=null||(x!=null||x!=null)){out+=x.v;}",
            "if((x==null&&x==null)&&x==null){return;}out+=x.v;",
            "if(x==null&&(x==null&&x==null)){return;}out+=x.v;",
            "if(x!==null||null!==x){out+=x.v;}",
            "if(x===null&&null===x){return;}out+=x.v;",
            // A contradicting read retains its old type. Other references keep their own facts.
            "if(x!=null){if(x==null){out+=x.v;}out+=x.v;}",
            "if(x!=null){if(x==null){}else{}out+=x.v;}",
            "if(x!=null){const y:B|null={v:3};if(y!=null){if(x==null){out+=y.v;}out+=x.v+y.v;}}",
            // Break and continue joins accept the fact from the possible edge.
            "while(true){if(x!=null){break;}if(x!=null){break;}return;}out+=x.v;",
            "if(x!=null){while(true){if(c){break;}if(x==null){break;}break;}out+=x.v;}",
            "if(x!=null){for(let i=0;i<1;out+=x.v){i+=1;if(c){continue;}if(x==null){continue;}}}",
        ] {
            suite.source(&source(binding, &format!("{body}print(`${{out}}`);")), None);
        }
    }
    suite.source(
        include_str!("../../../corpus/accept/a46-contradicted-null-test.ts"),
        Some("14 6 0\n"),
    );
    suite.finish();
    eprintln!(
        "P2.5 contradicted references cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn contradictions_do_not_hide_writes_on_other_edges() {
    let programs = [
        source(
            "let",
            "if(x!=null){if(c){x=null;}else if(x==null){}out+=x.v;}",
        ),
        source(
            "let",
            "if(x!=null){while(true){if(c){x=null;break;}if(x==null){break;}break;}out+=x.v;}",
        ),
        source(
            "let",
            "if(x!=null){for(let i=0;i<1;out+=x.v){i+=1;if(c){x=null;continue;}if(x==null){continue;}}}",
        ),
    ];
    let cases: Vec<_> = programs
        .iter()
        .map(|s| (s.as_str(), &[18047u32][..]))
        .collect();
    oracle::rejected_programs("contradicted-write-edges", "C16", &cases);
}
