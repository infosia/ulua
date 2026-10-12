//! Closed array surface, type flows, and resource boundaries.
use crate::{Limits, compile};
fn check(body: &str) -> Result<crate::Compiled, crate::Diagnostic> {
    compile(
        format!("export function main():void{{{body}}}").as_bytes(),
        Limits::default(),
    )
}
fn temporary_count(out: &str) -> usize {
    out.lines()
        .filter(|line| {
            line.trim()
                .strip_prefix("local __ulua_t")
                .is_some_and(|tail| tail.as_bytes().first().is_some_and(u8::is_ascii_digit))
        })
        .count()
}
#[test]
fn array_flows_are_invariant_at_every_depth() {
    for body in [
        "const xs:number[][]=[[],[1]];xs[0]=[2];xs.push([]);",
        "const xs:number[]=true?[]:[1];",
        "const xs=['a'];xs[0]+='b';",
        "function take(xs:number[]):number[]{return xs;}const ys=take([]);",
        "function f(xs:number[]=[]):number[]{return [];}const ys=f();",
        "const f=():number[][]=>[[],[1]];const xs=f();",
        "const f=():(()=>number)[][]=>[[()=>1]];const xs=f();",
    ] {
        check(body).unwrap();
    }
    for (body, code) in [
        ("const xs=[];", "C1"),
        ("const xs:number[]=['x'];", "C1"),
        ("const xs:number[][]=[[1],['x']];", "C1"),
        ("const xs=[1];xs.push('x');", "C1"),
        ("const xs=[1];xs[0]='x';", "C1"),
        ("const xs:(number|null)[]=[];", "C6"),
        ("const xs=[1,,2];", "C6"),
        ("const a=[1];const xs=[...a];", "C6"),
        ("const xs=[1];print(`${xs}`);", "C4"),
        ("const f=()=>[()=>1];", "C11"),
        ("const f=()=>[[()=>1]];", "C11"),
        ("const xs=[1];for(let x of xs){}", "C11"),
        ("const xs=[1];for(const xs of xs){}", "C10"),
        ("const xs=[1];xs[0]++;", "C11"),
        ("const xs=[1];xs.length++;", "C6"),
        ("const xs=[1];xs.push();", "C1"),
        ("const xs=[1];const push=xs.push;", "C6"),
        ("const xs=[1];xs['length'];", "C11"),
    ] {
        assert_eq!(check(body).unwrap_err().code, code, "{body}");
    }
    for source in [
        "export function main(xs:number[]):void{}",
        "export function main():number[]{return [];}",
    ] {
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "unsupported"
        );
    }
}
#[test]
fn array_literal_width_does_not_consume_the_temporary_budget() {
    for entry in ["1", "-1", "1+1", "x", "[x]", "f()"] {
        let body = format!(
            "const x=1;function f():number{{return x;}}const xs=[{}];",
            vec![entry; 1000].join(",")
        );
        for source in [
            format!("export function main():void{{{body}}}"),
            format!("{body}export function main():void{{}}"),
            format!("export function main():void{{const f=():void=>{{{body}}};f();}}"),
        ] {
            let out = compile(source.as_bytes(), Limits::default()).unwrap().luau;
            let temporaries = temporary_count(&out);
            assert!(temporaries <= 33, "{entry}: {temporaries}");
        }
    }
    for entry in ["1", "-1", "true", "'text'"] {
        let out = check(&format!("const xs=[{}];", vec![entry; 200].join(",")))
            .unwrap()
            .luau;
        assert_eq!(temporary_count(&out), 1, "{entry}");
    }
}

#[test]
fn rejected_array_members_keep_c6() {
    for method in [
        "find", "findLast", "flat", "unknown", "entries", "keys", "values", "toString",
    ] {
        assert_eq!(
            check(&format!("const xs=[1];xs.{method}();"))
                .unwrap_err()
                .code,
            "C6"
        );
    }
    for method in ["push", "map", "pop", "slice", "sort", "reduce"] {
        assert_eq!(
            check(&format!("const xs=[1];const m=xs.{method};"))
                .unwrap_err()
                .code,
            "C6"
        );
    }
}
#[test]
fn array_function_types_use_shared_aliases_and_reads_check_before_addition() {
    let out = check(
        "const fs:(()=>number)[][]=[[()=>1]];const xs=[1];print(`${xs[0.9999999999999999]}`);",
    )
    .unwrap()
    .luau;
    assert!(out.contains("= {__ulua_fn"));
    let guard = out.find("__ulua_fmod(i, 1)").unwrap();
    let read = out[guard..].find(" + 1]").unwrap();
    assert!(read > 0);
    assert!(out.contains("local __ulua_trap: (number) -> () = (...)"));
    assert!(!out.contains("getfenv"));
    assert!(out.contains("__ulua_get(__ulua_t"));
    assert_eq!(out.matches("then __ulua_trap(1) end").count(), 2);
}

#[test]
fn equality_rejects_literal_arrays_and_accepts_repeated_iteration_tests() {
    for op in ["===", "!==", "==", "!="] {
        for rhs in ["[]", "([])", "[1]", "([1])", "(([1]))"] {
            assert_eq!(
                check(&format!("const a=[1];if(a{op}{rhs}){{}}"))
                    .unwrap_err()
                    .code,
                "C7"
            );
        }
    }
    assert_eq!(check("const a=[1];if(([])===a){}").unwrap_err().code, "C7");
    check("const a=[1];for(const x of a){if(x===6){}} ").unwrap();
    check("const a=[1];for(const x of a){if(x===6){if(x===7){}}}").unwrap();
}
