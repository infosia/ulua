//! Strict module errors and equality through conditions.
use crate::{Limits, compile};

fn check(body: &str, code: Option<&str>) {
    let source = format!(
        "function f(x:number,y:number,c:boolean):boolean{{{body}}}export function main():void{{}}"
    );
    let result = compile(source.as_bytes(), Limits::default());
    if let Some(code) = code {
        assert_eq!(result.unwrap_err().code, code, "{body}");
    } else {
        assert!(result.is_ok(), "{body}: {result:?}");
    }
}

#[test]
fn conditions_aliases_and_unreachable_code_do_not_restrict_equality() {
    for body in [
        "const b=x===1;const d=b;const e=!d;if(!e){return x===2;}return false;",
        "let b=(x===1);if(b){return x===2;}return false;",
        "const b=x!==1;for(;b;){break;}return x===2;",
        "const b=x===1;return b&&x===2;",
        "const b=x!==1;return b||x===2;",
        "if(x!==1){if(true){return false;}}return x===2;",
        "if(x!==1){if(false){}else{return false;}}return x===2;",
        "return false?x===2:false;",
        "while(false){return x===2;}return false;",
        "for(;false;){return x===2;}return false;",
        "return false;return x===2;",
        "if(x===1){x=2;return x===2;}return false;",
        "if(c){if(x!==1){return false;}}else{if(x!==2){return false;}}return x===3;",
        "for(let i=0;i===0;i=x===2?5:6){if(c){continue;}if(x!==1){return false;}}return false;",
    ] {
        check(body, None);
    }
}

#[test]
fn strict_bindings_in_each_declaration_position() {
    for name in [
        "eval",
        "arguments",
        "await",
        "implements",
        "interface",
        "let",
        "package",
        "private",
        "protected",
        "public",
        "static",
        "yield",
    ] {
        let escaped = format!("\\u{:04x}{}", name.as_bytes()[0], &name[1..]);
        for declaration in [
            format!("let {name}=1;"),
            format!("const {name}=1;"),
            format!("function {name}():void{{}}"),
            format!("function f({name}:number):void{{}}"),
            format!("function f():void{{let {name}=1;}}"),
            format!("let {escaped}=1;"),
        ] {
            let source = format!("{declaration}export function main():void{{}}");
            assert_eq!(
                compile(source.as_bytes(), Limits::default())
                    .unwrap_err()
                    .code,
                "C11",
                "{source}"
            );
        }
    }
}

#[test]
fn strict_numeric_and_string_escapes() {
    for value in [
        "010", "08", "09", "00", r#""\01""#, r#""\1""#, r#""\7""#, r#""\8""#, r#""\9""#,
        r#""\00""#, r#""\08""#,
    ] {
        let source = format!("const v={value};export function main():void{{}}");
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C11",
            "{source}"
        );
    }
    for value in [
        "0",
        "0o10",
        "0O10",
        "0x10",
        "0b10",
        r#""\0""#,
        r#""\\01""#,
        r#""\\8""#,
        r#""\u0031""#,
    ] {
        let source = format!("const v={value};export function main():void{{}}");
        assert!(
            compile(source.as_bytes(), Limits::default()).is_ok(),
            "{source}"
        );
    }
}
