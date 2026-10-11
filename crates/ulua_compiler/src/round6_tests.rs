//! Round 6 condition aliases, literal edges, and strict module errors.
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
fn aliases_carry_initializer_tests_transitively() {
    for body in [
        "const isOne=x===1;if(isOne){return x===2;}return false;",
        "const b=x!==1;if(!b){return x===2;}return false;",
        "const b=x===1&&y===3;if(b){return y===2;}return false;",
        "const b=x===1;while(b){return x===2;}return false;",
        "const b=x===1;return b?x===2:false;",
        "const b=x===1;if(b&&y===3){return x===2;}return false;",
        "let z:number=y+1;const b=z===1;if(b){return z===2;}return false;",
        "const b=x===1;const d=b;const e=!d;if(!e){return x===2;}return false;",
        "let b=(x===1);if(b){return x===2;}return false;",
        "let b:boolean=x===1;const d=b;if(d){return x===2;}return false;",
        "const b=x!==1;for(;b;){}return x===2;",
        "const b=x===1;return b&&x===2;",
        "const b=x!==1;return b||x===2;",
    ] {
        check(body, Some("C7"));
    }
    check("const zero=x*0;const v=zero===0;return v?true:false;", None);
    check("const b=x===1;return b?true:false;", None);
    check("let zero=0;const v=zero===0;return v?false:true;", None);
    check(
        "const b=x===1;{const b=y===3;if(b){return x===2;}}return false;",
        None,
    );
    check(
        "const b=x===1;{const d=b;if(d){return x===2;}}return false;",
        Some("C7"),
    );
}

#[test]
fn long_alias_chains_share_repeated_operands() {
    let mut body = "const b0=x===1;".to_owned();
    for i in 1..=1000 {
        body.push_str(&format!("const b{i}=b{}||b{};", i - 1, i - 1));
    }
    check(
        &format!("{body}if(b1000){{return x===2;}}return false;"),
        Some("C7"),
    );
    check(&format!("{body}return b1000?true:false;"), None);
}

#[test]
fn literal_edges_and_unreachable_references() {
    for body in [
        "if(x!==1){if(true){return false;}}if(x===2){return true;}return false;",
        "if(x!==1){if(false){}else{return false;}}if(x===2){return true;}return false;",
        "if(true||x===x){if(x!==3){return false;}}return x===10;",
        "if(false&&x===x){}return false;",
        "if((false)){return x===2;}return false;",
        "return true?false:x===2;",
        "return false?x===2:false;",
        "while((false)){return x===2;}return false;",
        "for(;false;){return x===2;}return false;",
        "while((true)){}return x===2;",
        "for(;;){}return x===2;",
        "for(;true;){}return x===2;",
        "return false;return x===2;",
        "if(x!==1){if((true)){return false;}}return x===2;",
        "if(x!==1){let b:boolean=true&&c;return false;}return x===2;",
    ] {
        check(body, Some("C7"));
    }
    for body in [
        "if(false){}return x===2;",
        "while(false){}return x===2;",
        "for(;false;){}return x===2;",
        "return true||x+1===2;",
        "return false&&x+1===2;",
        "return false?x+1===2:true;",
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
