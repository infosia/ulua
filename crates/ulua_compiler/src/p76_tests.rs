use crate::{Limits, compile};
fn check(source: &str) -> Result<crate::Compiled, crate::Diagnostic> {
    compile(source.as_bytes(), Limits::default())
}
#[test]
fn json_members_and_c20_corpus() {
    for source in [
        include_str!("../../../corpus/accept/a83-json-stringify.ts"),
        include_str!("../../../corpus/accept/a84-json-parse.ts"),
        include_str!("../../../corpus/accept/a85-json-errors.ts"),
        include_str!("../../../corpus/accept/a86-json-mismatch.ts"),
    ] {
        check(source).unwrap();
    }
    for source in [
        include_str!("../../../corpus/reject/r165-json-map.ts"),
        include_str!("../../../corpus/reject/r166-json-function-field.ts"),
        include_str!("../../../corpus/reject/r167-json-parse-no-target.ts"),
        include_str!("../../../corpus/reject/r168-json-literal-order.ts"),
        include_str!("../../../corpus/reject/r169-json-arguments.ts"),
    ] {
        let expected = source
            .lines()
            .find_map(|s| s.strip_prefix("// expected-error: C20: "))
            .unwrap();
        let e = check(source).unwrap_err();
        assert_eq!((e.code, e.message.as_str()), ("C20", expected));
    }
}
#[test]
fn json_rejects_contained_values_and_class_parse_targets() {
    for body in [
        "JSON.stringify(new Set<number>());",
        "JSON.stringify(new Error());",
        "JSON.stringify([():number=>1]);",
        "const xs=[1];const x=xs.find((v:number):boolean=>true);JSON.stringify(x);",
        "const x:C=JSON.parse('{}');",
        "const x:R=JSON.parse('{}');",
        "JSON.parse('1');",
        "JSON.stringify();",
        "JSON.parse('1','2');",
    ] {
        let source =
            format!("class C{{x:number=1;}}type R={{x:C}};export function main():void{{{body}}}");
        assert_eq!(check(&source).unwrap_err().code, "C20", "{body}");
    }
}
#[test]
fn json_contexts_and_related_literal_declaration_orders() {
    check("type R={x:number};function f(x:R):R{return JSON.parse('{}');}class C{x:R=JSON.parse('{}');}export function main():void{f(JSON.parse('{}'));const r:R={x:1};r.x=JSON.parse('1');}").unwrap();
    let e = check("type A={z:number;a:number};type B={a:number;z:number};export function main():void{const b:B={a:1,z:2};const a:A=b;JSON.stringify(a);}").unwrap_err();
    assert_eq!(e.code, "C20");
    let c = check("type Other={a:number};type R={z:number;a:number};export function main():string{const r:R={z:1,a:2};return JSON.stringify(r);}").unwrap();
    assert!(!c.luau.contains("seen[v]"));
    check("type R={x:number;y:number};export function main():void{const r:R={y:1,x:2};}").unwrap();
}
#[test]
fn json_builtin_parameters_supply_targets_and_nested_calls_keep_depth_limits() {
    check(r#"export function main():void{Math.abs(JSON.parse('-1'));Number.isFinite(JSON.parse('1'));Number.parseInt(JSON.parse('\"10\"'),JSON.parse('10'));String.fromCodePoint(JSON.parse('65'));Utf8.byteLength(JSON.parse('\"s\"'));const n:number=JSON.parse(JSON.parse('\"1\"'));}"#).unwrap();
    let source = format!(
        "export function main():void{{JSON.stringify({}1{});}}",
        "JSON.stringify(".repeat(100),
        ")".repeat(100)
    );
    assert!(check(&source).unwrap_err().code.starts_with("limit:"));
}
