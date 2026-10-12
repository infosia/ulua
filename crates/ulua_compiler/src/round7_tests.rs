//! Equality aliases have no separate reference bound.
use crate::{Limits, compile};

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
fn alias_reference_count_uses_only_the_existing_parameter_bound() {
    for count in [8, 9, 16, crate::PARAMETER_BOUND] {
        for op in ["===", "!==", "==", "!="] {
            for declaration in ["const a=", "let a:boolean="] {
                let program = source(count, op, "const b=!a;const c=b;return c;")
                    .replace("const a=", declaration);
                assert!(compile(program.as_bytes(), Limits::default()).is_ok());
            }
        }
    }
    let program = source(crate::PARAMETER_BOUND + 1, "===", "return a;");
    assert_eq!(
        compile(program.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        "limit:parameters"
    );
}

#[test]
fn long_alias_chains_do_not_restrict_later_comparisons() {
    let mut chain = String::new();
    for i in 0..1000 {
        chain.push_str(&format!(
            "const a{}=a{}||x{}===1;",
            i + 1,
            if i == 0 { String::new() } else { i.to_string() },
            i % 8
        ));
    }
    let program = source(
        8,
        "===",
        &format!("{chain}if(a1000){{return x7===2;}}return false;"),
    );
    assert!(compile(program.as_bytes(), Limits::default()).is_ok());
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
