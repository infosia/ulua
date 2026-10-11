//! Round 7 flattened alias sets, declaration bounds, and hashbang diagnostics.
use crate::{ALIAS_TEST_BOUND, Limits, compile};

fn source(count: usize, op: &str, tail: &str) -> String {
    let parameters = (0..count)
        .map(|i| format!("x{i}:number"))
        .collect::<Vec<_>>()
        .join(",");
    let tests = (0..count)
        .map(|i| format!("x{i}{op}1"))
        .collect::<Vec<_>>()
        .join("||");
    format!(
        "function f({parameters}):boolean{{const a={tests};{tail}}}export function main():void{{}}"
    )
}

#[test]
fn each_alias_set_accepts_eight_and_rejects_nine_at_the_declaration() {
    assert_eq!(ALIAS_TEST_BOUND, 8);
    for op in ["===", "!==", "==", "!="] {
        let accepted = source(8, op, "const b=!a;const c=b;return c;");
        assert!(compile(accepted.as_bytes(), Limits::default()).is_ok());
        let rejected = source(9, op, "return a;");
        let diagnostic = compile(rejected.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(diagnostic.code, "C7");
        assert_eq!(diagnostic.offset as usize, rejected.find("a=").unwrap());
    }
    let accepted = source(8, "===", "return a;").replace("const a=", "let a:boolean=");
    assert!(compile(accepted.as_bytes(), Limits::default()).is_ok());
    let rejected = source(9, "!==", "return a;").replace("const a=", "let a:boolean=");
    let diagnostic = compile(rejected.as_bytes(), Limits::default()).unwrap_err();
    assert_eq!(diagnostic.code, "C7");
    assert_eq!(
        diagnostic.offset as usize,
        rejected.find("a:boolean=").unwrap()
    );
}

#[test]
fn bounds_count_distinct_references_and_keep_true_and_false_sets_separate() {
    let mut program = source(16, "===", "return a;");
    let a = program.find("const a=").unwrap();
    let b = program[a..].find(';').unwrap() + a;
    let tests = (0..16)
        .map(|i| format!("x{i}{}1", if i < 8 { "===" } else { "!==" }))
        .collect::<Vec<_>>()
        .join("||");
    program.replace_range(a..b, &format!("const a={tests}"));
    assert!(compile(program.as_bytes(), Limits::default()).is_ok());

    let accepted = source(8, "===", "const b=a||x0===1;const c=b||a;return c;");
    assert!(compile(accepted.as_bytes(), Limits::default()).is_ok());
    let rejected = source(9, "===", "return a;").replace("||x8===1", "");
    let rejected = rejected.replace("return a;", "const b=a||x8===1;return b;");
    let diagnostic = compile(rejected.as_bytes(), Limits::default()).unwrap_err();
    assert_eq!(diagnostic.code, "C7");
    assert_eq!(diagnostic.offset as usize, rejected.find("b=").unwrap());

    // Conditions without a stored alias do not have the alias bound.
    let condition = source(9, "===", "return true;")
        .replace("const a=", "if(")
        .replace(";return true;", "){return true;}return false;");
    assert!(compile(condition.as_bytes(), Limits::default()).is_ok());
}

#[test]
fn distinct_reference_chains_preserve_alias_tests_through_logical_operators() {
    let mut chain = String::new();
    for i in 0..1000 {
        chain.push_str(&format!(
            "const a{}=a{}||x{}===1;",
            i + 1,
            if i == 0 { String::new() } else { i.to_string() },
            i % 8
        ));
    }
    for (tail, rejected) in [
        ("return a1000;", false),
        ("if(a1000){return x7===2;}return false;", true),
        ("return a1000&&x7===2;", true),
        ("return !a1000||x7===2;", true),
        ("const b=!a1000;const c=b||false;return c||x7===2;", true),
    ] {
        let program = source(8, "===", &format!("{chain}{tail}"));
        let result = compile(program.as_bytes(), Limits::default());
        if rejected {
            assert_eq!(result.unwrap_err().code, "C7", "{tail}");
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }
}

#[test]
fn hashbang_diagnostic_names_the_hashbang() {
    let program = b"#!/usr/bin/env node\nexport function main():void{}";
    let diagnostic = compile(program, Limits::default()).unwrap_err();
    assert_eq!(diagnostic.code, "C11");
    assert_eq!(diagnostic.message, "Hashbang lines are rejected");
    assert_eq!(diagnostic.offset, 0);
    let diagnostic = compile(
        b"'use strict';export function main():void{}",
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(diagnostic.message, "Module directives are rejected");
}
