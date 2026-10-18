//! Direct calls and body writes jointly constrain parameter facts.
use crate::{Limits, compile};

fn forms(source: &str, proven: bool) {
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    assert_eq!(code.matches(" % ").count(), usize::from(proven), "{code}");
    assert_eq!(
        code.matches("__ulua_fmod(").count(),
        usize::from(!proven),
        "{code}"
    );
    let tests: Vec<_> = code
        .lines()
        .filter(|line| line.contains(" < #__ulua_") && line.contains("then __ulua_trap(1) end"))
        .collect();
    assert_eq!(tests.len(), 1, "{code}");
    assert_eq!(tests[0].contains("__ulua_floor"), !proven, "{code}");
}

const BODY: &str = "const a=[1];return a[p]+p%3;";
fn case(prefix: &str, parameter: &str, body: &str, calls: &str, proven: bool) {
    forms(
        &format!(
            "{prefix}function f({parameter}):number{{{body}}}export function main():number{{{calls}}}"
        ),
        proven,
    );
}

#[test]
fn direct_parameter() {
    case("", "p:number", BODY, "return f(0);", true);
}
#[test]
fn exported_parameter() {
    case("export ", "p:number", BODY, "return f(0);", false);
}
#[test]
fn async_parameter() {
    forms(
        "async function f(p:number):Promise<number>{const a=[1];return a[p]+p%3;}export async function main():Promise<number>{return await f(0);}",
        false,
    );
}
#[test]
fn value_use_parameter() {
    case("", "p:number", BODY, "const g=f;return f(0)+g(-1);", false);
}
#[test]
fn uncalled_parameter() {
    case("", "p:number", BODY, "return 0;", false);
}
#[test]
fn default_parameter() {
    case("", "p:number=0", BODY, "return f(0);", false);
}
#[test]
fn body_write_parameter() {
    for write in ["p=1;", "p+=1;", "p++;", "const g=():void=>{p=1;};g();"] {
        case(
            "",
            "p:number",
            &(write.to_owned() + BODY),
            "return f(0);",
            true,
        );
    }
    for write in ["p=-1;", "p-=1;", "p--;", "const g=():void=>{p=-1;};g();"] {
        case(
            "",
            "p:number",
            &(write.to_owned() + BODY),
            "return f(0);",
            false,
        );
    }
}
#[test]
fn recursive_parameter() {
    case(
        "",
        "p:number",
        &("if(p<3){f(p+1);}".to_owned() + BODY),
        "return f(0);",
        true,
    );
}
#[test]
fn recursive_argument_without_a_fact_disables_parameter_facts() {
    case(
        "",
        "p:number",
        &("if(p<3){f(p-5);}".to_owned() + BODY),
        "return f(0);",
        false,
    );
}
#[test]
fn constructor_and_method_parameters_stay_checked() {
    forms(
        "class C{value:number;constructor(p:number){const a=[1];this.value=a[p]+p%3;}}export function main():number{return new C(0).value;}",
        false,
    );
    forms(
        "class C{m(p:number):number{const a=[1];return a[p]+p%3;}}export function main():number{const c=new C();return c.m(0);}",
        false,
    );
}
#[test]
fn argument_across_outermost_functions() {
    forms(
        "function f(p:number):number{const a=[1];return a[p]+p%3;}function g(q:number):number{let x=q;return f(x);}export function main():number{return g(0);}",
        true,
    );
}

#[test]
fn every_call_contributes_an_argument_write() {
    case("", "p:number", BODY, "return f(0)+f(-1);", false);
    case("", "p:number", BODY, "return f(0)+f(0.5);", false);
    forms(
        "let x=0;function f(p:number):number{const a=[1];return a[p]+p%3;}export function main():number{return f(x);}",
        false,
    );
}

#[test]
fn value_use_in_another_function_disables_parameter_facts() {
    forms(
        "function f(p:number):number{const a=[1];return a[p]+p%3;}function g():number{const h=f;return h(-1);}export function main():number{return f(0)+g();}",
        false,
    );
}

#[test]
fn uncalled_parameter_body_write_has_no_fact() {
    case(
        "",
        "p:number",
        &("p=0;".to_owned() + BODY),
        "return 0;",
        false,
    );
}

#[test]
fn default_does_not_disable_other_parameters() {
    forms(
        "function f(p:number,q:number=-1):number{const a=[1];return a[p]+p%3;}export function main():number{return f(0);}",
        true,
    );
}

#[test]
fn recursive_component_without_a_seed_has_no_fact() {
    case(
        "",
        "p:number",
        &("f(p);".to_owned() + BODY),
        "return 0;",
        false,
    );
}

#[test]
fn literal_argument_preserves_finite_interval() {
    let mut module = super::tests::checked(
        "function f(p:number):number{f(p%4);return (7-p)%3;}export function main():number{return f(7);}",
    );
    // The literal argument and recursive write keep the upper bound at seven.
    let report = super::analyze(&mut module).unwrap();
    assert_eq!(report.sites.len(), 2);
    assert!(report.rounds <= super::FACT_ROUNDS);
}

#[test]
fn parameter_growth_shapes_keep_one_native_site() {
    for shape in [
        "parameter-call-chain",
        "parameter-many-calls",
        "parameter-nested-calls",
    ] {
        for bytes in [4096, 16384, 65536] {
            let source = super::tests::growth::source(shape, bytes);
            assert_eq!(source.len(), bytes);
            let mut module = super::tests::checked(&source);
            let report = super::analyze(&mut module).unwrap();
            assert_eq!(report.sites.len(), 1, "{shape} {bytes}");
            assert_eq!(report.rounds, 2, "{shape} {bytes}");
            if shape == "parameter-nested-calls" {
                let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
                assert_eq!(code.matches(" % ").count(), 1);
                assert_eq!(code.matches("__ulua_fmod(").count(), 1);
            }
        }
    }
}
