//! Direct C7 type pairs and expression forms.
use crate::{Limits, compile};

fn check(expression: &str, reject: bool) {
    let source = format!(
        "function value():number{{return 1;}}function text():string{{return 'x';}}function f(n:number,m:number,s:string,c:boolean,p:boolean):boolean{{let x=0;const k=1;const wide=n+1;const na=[1];const nb=[2];const sa=['x'];const ba=[true];const nested=[[1]];const funcs=[value];return {expression};}}export function main():void{{}}"
    );
    let result = compile(source.as_bytes(), Limits::default());
    if reject {
        assert_eq!(result.unwrap_err().code, "C7", "{expression}");
    } else {
        assert!(result.is_ok(), "{expression}: {result:?}");
    }
}

#[test]
fn every_operator_checks_every_operand_type_pair() {
    let operands = ["n", "s", "c", "na", "sa", "ba", "nested", "funcs", "value"];
    for op in ["===", "!==", "==", "!="] {
        for (i, a) in operands.iter().enumerate() {
            for (j, b) in operands.iter().enumerate() {
                check(&format!("{a}{op}{b}"), i != j || *a == "value");
            }
        }
        check(&format!("na{op}nb"), false);
        check(&format!("value{op}text"), true);
    }
}

#[test]
fn primitive_acceptance_does_not_depend_on_operand_shape() {
    for op in ["===", "!==", "==", "!="] {
        for (form, other) in [
            ("value()", "1"),
            ("text()", "'x'"),
            ("String(n)", "'x'"),
            ("1+2", "1"),
            ("1-2", "1"),
            ("1*2", "1"),
            ("1/2", "1"),
            ("1%2", "1"),
            ("-n", "1"),
            ("-k", "1"),
            ("-(1+2)", "1"),
            ("x", "1"),
            ("n", "1"),
            ("s", "'x'"),
            ("wide", "1"),
            ("((wide))", "1"),
            ("1", "2"),
            ("'x'", "'y'"),
            ("-1", "1"),
            ("-((1))", "1"),
            ("k", "1"),
            ("`x`", "'x'"),
            ("`${1}`", "'x'"),
            ("c?1:2", "1"),
            ("true", "false"),
            ("!c", "false"),
            ("c&&p", "false"),
            ("c||p", "false"),
            ("NaN", "NaN"),
            ("((NaN))", "n"),
            ("0/0", "NaN"),
            ("Infinity", "n"),
            ("-0", "0"),
        ] {
            for (a, b) in [(form, other), (other, form)] {
                check(&format!("({a}){op}({b})"), false);
            }
        }
        for a in ["c", "p", "true", "!p", "c?p:false", "c&&p", "c||p"] {
            check(&format!("({a}){op}p"), false);
        }
        for literal in ["[]", "([])", "[1]", "(([1]))"] {
            for (a, b) in [(literal, "na"), ("na", literal), (literal, literal)] {
                check(&format!("{a}{op}{b}"), true);
            }
        }
        for function in ["value", "(()=>1)"] {
            check(&format!("{function}{op}value"), true);
            check(&format!("value{op}{function}"), true);
        }
    }
}
