//! Return coverage and boolean initializer rules retain their own consumers.
use crate::{Limits, compile};

fn check(body: &str, code: Option<&str>) {
    let source = format!(
        "function f(c:boolean,p:boolean,n:number,s:string):boolean{{{body}}}export function main():void{{}}"
    );
    let result = compile(source.as_bytes(), Limits::default());
    if let Some(code) = code {
        assert_eq!(result.unwrap_err().code, code, "{body}");
    } else {
        assert!(result.is_ok(), "{body}: {result:?}");
    }
}

#[test]
fn boolean_let_initializer_has_a_closed_form() {
    for body in [
        "if(c){let v=c;v=false;}return c;",
        "let v=c&&p;if(v){let w=v;w=p;}return false;",
        "while(c){let w=c;w=false;c=w;}return false;",
        "let v=!c;return v;",
        "let v=c?p:false;return v;",
        "let v=(c);return v;",
    ] {
        check(body, Some("C11"));
    }
    for body in [
        "let done=false;done=c;return done;",
        "let big=n>2;big=c;return big;",
        "let done=(false);return done;",
        "let big=((n>2));return big;",
        "if(c){let v:boolean=c;v=false;}return c;",
        "let v:boolean=c&&p;return v;",
        "const v=c&&p;return v;",
    ] {
        check(body, None);
    }
    for op in ["<", ">", "<=", ">=", "===", "!==", "==", "!="] {
        check(&format!("let v=n{op}2;v=c;return v;"), None);
    }
}

#[test]
fn return_coverage_preserves_parentheses_and_loop_breaks() {
    for (body, rejected) in [
        ("while((true)){return 1;}", true),
        ("while(((true))){return 1;}", true),
        ("for(;(true);){return 1;}", true),
        ("for(let i=0;(true);i++){return 1;}", true),
        ("while(true){continue;}", false),
        ("for(;;){continue;}", false),
        ("for(;true;){continue;}", false),
        ("while(true){if(false){break;}}", true),
        ("for(;;){break;}", true),
        ("while(true){while(true){break;}}", false),
        ("for(;;){for(;;){break;}}", false),
        ("while((true)){}return 1;", false),
        ("{return 1;}let n=2;", false),
        ("return 1;let n=2;", false),
        ("if(true){return 1;}else{{return 2;}}let n=3;", false),
        ("{let n=1;}", true),
        ("if(true){return 1;}", true),
    ] {
        let source = format!("function f():number{{{body}}}export function main():void{{}}");
        let result = compile(source.as_bytes(), Limits::default());
        if rejected {
            assert_eq!(result.unwrap_err().code, "C11", "{body}");
        } else {
            assert!(result.is_ok(), "{body}: {result:?}");
        }
    }
}

#[test]
fn loop_control_does_not_cross_function_boundaries() {
    for control in ["break", "continue"] {
        for loop_form in ["while(true)", "for(;;)", "for(const x of [1])"] {
            check(
                &format!("{loop_form}{{const f=():void=>{{{control};}};}}return false;"),
                Some("C11"),
            );
            check(
                &format!("{loop_form}{{function f():void{{{control};}}}}return false;"),
                Some("C11"),
            );
            check(
                &format!(
                    "{loop_form}{{const f=():void=>{{while(true){{break;}}}};{control};}}return false;"
                ),
                None,
            );
        }
    }
}
