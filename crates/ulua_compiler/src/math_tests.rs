//! Math surface checks pin the closed member set, arities, and value rejection.
use crate::{Limits, compile};
#[test]
fn math_arities_and_rejections() {
    for (call, message) in [
        ("Math.max(1,2,3)", "Math.max takes exactly two arguments"),
        ("Math.sin", "a Math member is called or read as a constant"),
        ("Math.hypot(1)", "Math.hypot takes exactly two arguments"),
        ("Math.random(1)", "Math.random takes no arguments"),
    ] {
        let source = format!("export function main():void{{const x={call};}}");
        let e = compile(source.as_bytes(), Limits::default()).err().unwrap();
        assert_eq!(e.code, "C4");
        assert_eq!(e.message, message);
    }
    for expr in [
        "Math",
        "Math.f16round(1)",
        "Math['sin'](1)",
        "Math.sin('x')",
        "Math?.sin(1)",
        "Math.sin?.(1)",
        "2**3",
    ] {
        let s = format!("export function main():void{{const x={expr};}}");
        assert!(compile(s.as_bytes(), Limits::default()).is_err(), "{expr}");
    }
}
#[test]
fn math_constants_and_shadowed_namespace() {
    let source = "export function main():number{const Math={sin:(n:number):number=>n+1};return Math.sin(2);}";
    assert!(compile(source.as_bytes(), Limits::default()).is_ok());
    for constant in [
        "E", "LN2", "LN10", "LOG2E", "LOG10E", "PI", "SQRT1_2", "SQRT2",
    ] {
        let s = format!("export function main():number{{return Math.{constant};}}");
        assert!(compile(s.as_bytes(), Limits::default()).is_ok());
    }
}

#[test]
fn each_math_member_has_fixed_arity() {
    for name in "abs ceil floor sqrt round sign trunc max min clz32 imul fround acos acosh asin asinh atan atanh atan2 cbrt cos cosh exp expm1 hypot log log1p log10 log2 pow sin sinh tan tanh random".split_whitespace() {
        let expected = if name=="random" {0} else if ["max","min","imul","atan2","hypot","pow"].contains(&name) {2} else {1};
        for arity in 0..=3 {
            let args=vec!["1";arity].join(",");
            let s=format!("export function main():number{{return Math.{name}({args});}}");
            let result=compile(s.as_bytes(),Limits::default());
            if arity==expected { assert!(result.is_ok(),"{s}"); }
            else { assert_eq!(result.err().unwrap().code,"C4","{s}"); }
        }
    }
}
#[test]
fn native_math_registration_flag_follows_checked_members() {
    for (expression, expected) in [
        ("1", false),
        ("Math.PI", false),
        ("Math.floor(1)", false),
        ("Math.imul(1,2)", false),
        ("Math.sin(1)", true),
        ("Math.random()", true),
    ] {
        let s = format!("export function main():number{{return {expression};}}");
        let compiled = compile(s.as_bytes(), Limits::default()).unwrap();
        assert_eq!(compiled.native_math, expected, "{expression}");
    }
}
