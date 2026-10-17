use crate::{Limits, compile};
#[test]
fn number_surface_and_exact_rejections() {
    for (body, message) in [
        (
            "Number('1');",
            "Number converts its argument; use parseFloat",
        ),
        (
            "new Number(1);",
            "Number converts its argument; use parseFloat",
        ),
        (
            "isNaN(1);",
            "the global isNaN converts its argument; use Number.isNaN",
        ),
        ("(1).toString();", "toString takes a radix"),
        ("(1).toPrecision();", "toPrecision takes digits"),
        ("parseInt('1');", "parseInt takes a string and a radix"),
    ] {
        let source = format!("export function main():void{{{body}}}");
        let err = compile(source.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(err.code, "C4");
        assert_eq!(err.message, message);
    }
    for expression in [
        "Number.isNaN('1')",
        "Number.isFinite(true)",
        "Number.isInteger(null)",
        "Number.isSafeInteger('2')",
        "Number.isNaN",
        "(1).toFixed",
        "(1).toLocaleString()",
        "Number.toFixed(2)",
        "parseFloat(1)",
        "parseInt('1', '2')",
    ] {
        let source = format!("export function main():void{{const x={expression};}}");
        assert!(
            compile(source.as_bytes(), Limits::default()).is_err(),
            "{expression}"
        );
    }
}
#[test]
fn number_native_helpers_are_private_calls_and_constants_are_literals() {
    let compiled=compile(b"export function main():string{const x=Number.MAX_SAFE_INTEGER;const n=Number.parseInt('12',10);return n.toFixed();}",Limits::default()).unwrap();
    assert!(compiled.native_math);
    assert!(compiled.luau.contains("9007199254740991"));
    assert!(compiled.luau.contains("__ulua_math_native.number_parseInt"));
    assert!(compiled.luau.contains("__ulua_math_native.number_toFixed"));
}
