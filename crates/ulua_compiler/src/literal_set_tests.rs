//! Direct checker tests pin C7 forms and the S6 TypeScript measurements.
use super::*;

fn check(source: &str, rejected: bool) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert!(parsed.diagnostics.is_empty(), "{source}");
    let result = checker::check(&parsed.program, source.as_bytes(), DEPTH_CEILING);
    if rejected {
        assert_eq!(result.err().unwrap().code, "C7", "{source}");
    } else {
        if let Err(error) = result {
            panic!("{source}: {error:?}");
        }
    }
}

fn function(expression: &str, declarations: &str) -> String {
    format!(
        "function f(c:boolean,p:boolean,n:number,s:string):boolean{{{declarations}return {expression};}}export function main():void{{}}"
    )
}

#[test]
fn s6_all_fourteen_tsc_measurements() {
    // The declarations match the S6 record. Both strict operators need the same overlap check.
    for (expression, rejected) in [
        ("(!true) === true", true),
        ("(!true) === false", false),
        ("(c ? 1 : 2) === 3", true),
        ("(c ? 1 : 2) === 2", false),
        ("(p && true) === false", false),
        ("(true && false) === true", true),
        ("(p || true) === false", true),
        ("`a` === \"b\"", true),
        ("(-a) === 1", false),
        ("a === -1", true),
        ("(!p) === true", false),
        ("(c ? true : false) === true", false),
        ("(c ? \"x\" : \"y\") === \"z\"", true),
        ("(1 + 1) === 3", false),
    ] {
        for operator in ["===", "!=="] {
            let expression = expression.replace("===", operator);
            check(
                &format!(
                    "function f(c:boolean,p:boolean):boolean{{const a=1;return {expression};}}export function main():void{{}}"
                ),
                rejected,
            );
        }
    }
}

#[test]
fn each_literal_set_form_in_both_orders() {
    for (a, b, rejected) in [
        ("1", "2", true),
        ("'x'", "'y'", true),
        ("true", "false", true),
        ("-1", "1", true),
        ("-(1)", "-1", false),
        ("-a", "1", false),
        ("`x`", "'y'", true),
        (r"`\u{78}`", "'x'", false),
        (r"`\0`", r"'\u0000'", false),
        ("`x${a}`", "'y'", false),
        ("alias", "3", true),
        ("alias", "2", false),
        ("((a))", "2", true),
        ("!true", "true", true),
        ("!!true", "true", false),
        ("!(c?true:false)", "false", false),
        ("(c?1:2)", "(p?2:3)", false),
        ("(c?1:2)", "(p?3:4)", true),
        ("(c?'x':'y')", "(p?'y':'z')", false),
        ("(c?'x':'y')", "(p?'z':'w')", true),
        ("(true&&false)", "true", true),
        ("(false&&true)", "true", true),
        ("(p&&true)", "false", false),
        ("(p&&false)", "true", true),
        ("(false||true)", "false", true),
        ("(true||false)", "false", true),
        ("(p||false)", "true", false),
        ("(p||true)", "false", true),
        ("!(p||true)", "true", true),
        ("n", "1", false),
        ("s", "'x'", false),
        ("p", "true", false),
        ("!p", "true", false),
        ("(c?n:1)", "3", false),
        ("(c?1:n)", "3", false),
        ("(p&&c)", "true", false),
        ("(p||c)", "false", false),
        ("1+1", "3", false),
        ("Infinity", "1", false),
        ("NaN", "1", false),
    ] {
        for operator in ["===", "!=="] {
            for (left, right) in [(a, b), (b, a)] {
                check(
                    &function(
                        &format!("({left}){operator}({right})"),
                        "const a=1;const set=c?1:2;const alias=set;",
                    ),
                    rejected,
                );
            }
        }
    }
}

#[test]
fn short_circuit_excludes_a_finite_or_wide_right_value() {
    for (expression, literal, rejected) in [
        ("false&&true", "false", false),
        ("false&&p", "false", false),
        ("false&&p", "true", true),
        ("true||false", "true", false),
        ("true||p", "true", false),
        ("true||p", "false", true),
        ("(p||true)||c", "false", true),
        ("(p&&false)&&c", "true", true),
        ("!(false&&p)", "false", true),
        ("!(true||p)", "true", true),
        ("true&&p", "false", false),
        ("false||p", "true", false),
        ("(c?true:false)&&true", "false", false),
        ("(c?true:false)||false", "true", false),
        // Narrowable operands still propagate, even when the right value cannot enter.
        ("false&&q", "false", true),
        ("true||q", "true", true),
    ] {
        for operator in ["===", "!=="] {
            for (a, b) in [(expression, literal), (literal, expression)] {
                check(
                    &function(&format!("({a}){operator}({b})"), "let q=true;"),
                    rejected,
                );
            }
        }
    }
}

#[test]
fn const_initializers_preserve_wide_and_narrowable_facts() {
    for operator in ["===", "!=="] {
        for (declarations, expression, rejected) in [
            ("const a=p;const b=a;", format!("b{operator}true"), false),
            (
                "let q=true;const a=q;const b=a;",
                format!("b{operator}true"),
                true,
            ),
            (
                "let q=true;const a=q;const b=a;",
                format!("b{operator}p"),
                false,
            ),
            ("const a=!true;const b=a;", format!("b{operator}true"), true),
            (
                "const a=p||true;const b=a;",
                format!("b{operator}false"),
                true,
            ),
            ("const a=c?1:2;const b=a;", format!("b{operator}3"), true),
            ("let a=1;const b=a;", format!("b{operator}2"), false),
            ("let a='x';const b=a;", format!("b{operator}'y'"), false),
        ] {
            check(&function(&expression, declarations), rejected);
        }
    }
}

#[test]
fn narrowable_propagates_from_every_operand_and_value_part() {
    for (expression, finite, wide) in [
        ("((q))", "true", "p"),
        ("!q", "true", "p"),
        ("(c?q:false)", "true", "p"),
        ("(c?true:q)", "true", "p"),
        ("(q?true:false)", "true", "p"),
        ("(q?1:2)", "1", "n"),
        ("(q?'x':'y')", "'x'", "s"),
        ("(p&&q)", "true", "p"),
        ("(q&&p)", "true", "p"),
        ("(p||q)", "true", "p"),
        ("(q||p)", "true", "p"),
        ("!(c?(p&&q):(q||p))", "true", "p"),
    ] {
        for operator in ["===", "!=="] {
            for (other, rejected) in [(finite, true), (expression, true), (wide, false)] {
                for (a, b) in [(expression, other), (other, expression)] {
                    check(
                        &function(&format!("{a}{operator}{b}"), "let q=true;"),
                        rejected,
                    );
                }
            }
        }
    }
}

#[test]
fn later_parameter_assignment_propagates_through_const_forms() {
    for expression in [
        "!p",
        "c?p:false",
        "c?true:p",
        "p?true:false",
        "c&&p",
        "p&&c",
        "c||p",
        "p||c",
    ] {
        for operator in ["===", "!=="] {
            check(
                &format!(
                    "function f(c:boolean,p:boolean):boolean{{const a={expression};const alias=a;const result=alias{operator}true;p=false;return result;}}export function main():void{{}}"
                ),
                true,
            );
        }
    }
}

#[test]
fn finite_union_const_chain_stays_within_the_source_ceiling() {
    // Const aliases have no source depth. They must not add checker recursion or full-set copies.
    let mut source = String::from("function f(c:boolean):boolean{const a0=0;");
    for id in 1..1800 {
        source.push_str(&format!("const a{id}=c?a{}:{id};", id - 1));
    }
    source.push_str("return a1799===1799;}export function main():void{}");
    assert!(source.len() < SOURCE_CEILING as usize);
    check(&source, false);
    let disjoint = source.replace("a1799===1799", "a1799===1800");
    check(&disjoint, true);
}

#[test]
fn new_accepted_forms_in_inline_conditional_arms_compile() {
    for expression in [
        "(!true)===false",
        "(c?1:2)===2",
        "(p&&true)===false",
        "(-a)===1",
        "(!p)===true",
        "(c?true:false)===true",
        "(1+1)===3",
        "`x`==='x'",
        "alias===2",
    ] {
        let source = function(
            &format!("c?({expression}):false"),
            "const a=1;const alias=c?1:2;",
        );
        let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
        assert!(compiled.luau.contains(" :: boolean"));
    }
}
