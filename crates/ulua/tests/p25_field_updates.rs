//! Record updates share the compound-store order and the C12.7 receiver check.
//! Each accepted source passes tsc, Node, the ABI, and luau-analyze. The test prints its Debug cost.
//! Measured Debug cost: 0.490 seconds for 41 accepted sources; restrictions cost 0.008 seconds for nine rejects.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
// Shared witnesses include constructors for other test targets.
#[allow(dead_code)]
#[path = "support/equality.rs"]
mod witness;
use std::time::Instant;
use witness::Suite;
fn source(body: &str) -> String {
    format!(
        "type B={{v:number}};type N={{v:number|null}};export function main():void{{function i(n:number):number{{print(`i${{n}}`);return n;}}{body}}}"
    )
}
fn update(target: &str, operator: &str, prefix: bool) -> String {
    if prefix {
        format!("{operator}{target}")
    } else {
        format!("{target}{operator}")
    }
}
#[test]
fn field_updates_match_node_and_keep_one_receiver_evaluation() {
    let start = Instant::now();
    let mut suite = Suite::named("P2.5 field updates");
    for operator in ["++", "--"] {
        for prefix in [false, true] {
            let value = if operator == "++" { 11 } else { 9 };
            let loop_value = if operator == "++" { 12 } else { 8 };
            let u = update("o.v", operator, prefix);
            suite.source(
                &source(&format!("const o:B={{v:10}};{u};print(`${{o.v}}`);")),
                Some(&format!("{value}\n")),
            );
            suite.source(&source(&format!("const o:B={{v:10}};for(let n=0;n<2;{u}){{print('body');n++;}}print(`${{o.v}}`);")), Some(&format!("body\nbody\n{loop_value}\n")));
            let u = update("rs[i(0)].v", operator, prefix);
            suite.source(
                &source(&format!(
                    "const rs:B[]=[{{v:10}}];const old=rs[0];{u};print(`${{old.v}}:${{rs[0].v}}`);"
                )),
                Some(&format!("i0\n{value}:{value}\n")),
            );
            suite.source(&source(&format!("const rs:B[]=[{{v:10}}];for(let n=0;n<2;{u}){{print('body');n++;}}print(`${{rs[0].v}}`);")), Some(&format!("body\ni0\nbody\ni0\n{loop_value}\n")));
            let u = update("pick().v", operator, prefix);
            suite.source(&source(&format!("const o:B={{v:10}};function pick():B{{print('pick');return o;}}{u};print(`${{o.v}}`);")), Some(&format!("pick\n{value}\n")));
            let u = update("rs[i(0)][i(0)].v", operator, prefix);
            suite.source(
                &source(&format!(
                    "const rs:B[][]=[[{{v:10}}]];{u};print(`${{rs[0][0].v}}`);"
                )),
                Some(&format!("i0\ni0\n{value}\n")),
            );
            let u = update("rs[i(0)].v", operator, prefix);
            suite.source_trap(
                &source(&format!("const rs:B[]=[];{u};print('after');")),
                "i0\n",
                "trap:index-out-of-range",
                "TypeError",
            );
            suite.source_trap(
                &source(&format!(
                    "const rs:B[]=[];for(let n=0;n<1;{u}){{print('body');n++;}}print('after');"
                )),
                "body\ni0\n",
                "trap:index-out-of-range",
                "TypeError",
            );
            let u = update("rs[i(0)][i(1)].v", operator, prefix);
            for entries in ["[]", "[[]]"] {
                suite.source_trap(
                    &source(&format!("const rs:B[][]={entries};{u};print('after');")),
                    "i0\ni1\n",
                    "trap:index-out-of-range",
                    "TypeError",
                );
            }
        }
    }
    suite.source(
        include_str!("../../../corpus/accept/a50-field-update.ts"),
        Some("2 3 11\n"),
    );
    suite.finish();
    eprintln!(
        "P2.5 field update cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
#[test]
fn field_updates_keep_target_and_value_restrictions() {
    let start = Instant::now();
    for (body, expected) in [
        ("const o={v:'x'};o.v++;", "C4"),
        ("const o={v:true};++o.v;", "C4"),
        ("const o:N={v:1};o.v--;", "C4"),
        ("const o:B={v:1};o.missing++;", "C1"),
        ("const o:B|null=null;o.v++;", "C16"),
        ("const xs:number[]=[];xs.length++;", "C6"),
        ("const xs:number[]=[1];xs[0]++;", "C11"),
        ("const o:B={v:1};const y=o.v++;", "C11"),
        ("const o:B={v:1};print(`${++o.v}`);", "C11"),
    ] {
        let d = ulua_compiler::compile(source(body).as_bytes(), ulua_compiler::Limits::default())
            .unwrap_err();
        assert_eq!(d.code, expected, "{body}: {d:?}");
    }
    eprintln!(
        "P2.5 field update restriction cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}
