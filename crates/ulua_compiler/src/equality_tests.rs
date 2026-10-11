//! Direct C7 structural and region tests. Each case uses the default compiler API.
use crate::{Limits, compile};
fn check(body: &str, reject: bool) {
    let source = format!(
        "function value():number{{return 1;}}function text():string{{return 'x';}}function f(n:number,m:number,s:string,c:boolean,p:boolean):boolean{{let x=0;const k=1;const wide=n+1;{body}}}export function main():void{{}}"
    );
    let result = compile(source.as_bytes(), Limits::default());
    if reject {
        assert_eq!(result.unwrap_err().code, "C7", "{body}");
    } else {
        assert!(result.is_ok(), "{body}: {result:?}");
    }
}
#[test]
fn each_wide_and_non_wide_operand_in_both_orders() {
    for op in ["===", "!==", "==", "!="] {
        for (form, literal, rejected) in [
            ("value()", "1", false),
            ("text()", "'x'", false),
            ("String(n)", "'x'", false),
            ("1+2", "1", false),
            ("1-2", "1", false),
            ("1*2", "1", false),
            ("1/2", "1", false),
            ("1%2", "1", false),
            ("-n", "1", false),
            ("-k", "1", false),
            ("-(1+2)", "1", false),
            ("x", "1", false),
            ("n", "1", false),
            ("s", "'x'", false),
            ("wide", "1", false),
            ("((wide))", "1", false),
            ("1", "1", true),
            ("'x'", "'x'", true),
            ("-1", "1", true),
            ("-((1))", "1", true),
            ("k", "1", true),
            ("`x`", "'x'", true),
            ("`${1}`", "'x'", true),
            ("c?1:2", "1", true),
            ("true", "false", true),
            ("!c", "false", true),
            ("c&&p", "false", true),
            ("c||p", "false", true),
        ] {
            for (a, b) in [(form, literal), (literal, form)] {
                check(&format!("return ({a}){op}({b});"), rejected);
            }
            check(
                &format!("const a=({form});const b=a;return b{op}({literal});"),
                rejected,
            );
        }
        check(&format!("const a=wide;const b=a;return b{op}1;"), false);
        for nan in ["NaN", "((NaN))"] {
            for (a, b) in [(nan, "n"), ("n", nan)] {
                check(&format!("return {a}{op}{b};"), true);
            }
        }
        for a in ["c", "p", "true", "!p", "c?p:false", "c&&p", "c||p"] {
            check(&format!("return ({a}){op}p;"), true);
        }
    }
}
#[test]
fn true_and_false_sets_follow_operators_and_nesting() {
    for (g, t, f) in [
        ("n===1", true, false),
        ("1===n", true, false),
        ("n!==1", false, true),
        ("1!==n", false, true),
        ("!(n===1)", false, true),
        ("!(1!==n)", true, false),
        ("((n===1))", true, false),
        ("!!(n!==1)", false, true),
        ("n===1&&c", true, false),
        ("c&&n!==1", false, true),
        ("n===1||c", true, false),
        ("c||n!==1", false, true),
        ("(n===1)||(n!==2)", true, true),
        ("!((n!==1)&&!(n===2))", true, false),
        ("n===m", false, false),
        ("n===1+2", false, false),
    ] {
        for g in [g.to_owned(), g.replace("===", "==").replace("!==", "!=")] {
            check(&format!("if({g}){{return n===3;}}return false;"), t);
            check(&format!("if({g}){{return false;}}else{{return n===3;}}"), f);
            check(&format!("return ({g})&&(n===3);"), t);
            check(&format!("return ({g})||(n===3);"), f);
            check(&format!("return ({g})?(n===3):false;"), t);
            check(&format!("return ({g})?false:(n===3);"), f);
            check(&format!("while({g}){{return n===3;}}return false;"), t);
            check(&format!("for(;{g};){{return n===3;}}return false;"), t);
            check(&format!("while({g}){{break;}}return n===3;"), t && f);
            check(&format!("for(;{g};){{break;}}return n===3;"), t && f);
            check(&format!("if({g}){{return false;}}return n===3;"), f);
            check(&format!("if({g}){{}}else{{return false;}}return n===3;"), t);
            for exit in ["break;", "continue;"] {
                check(
                    &format!("while(c){{if({g}){{{exit}}}return n===3;}}return false;"),
                    f,
                );
                check(
                    &format!("while(c){{if({g}){{}}else{{{exit}}}return n===3;}}return false;"),
                    t,
                );
            }
        }
    }
}
#[test]
fn each_statement_region_and_block_remainder() {
    for body in [
        "{if(n!==1){return false;}}return n===2;",
        "while(n===1){return n===2;}return false;",
        "for(;n===1;){return n===2;}return false;",
        "if(n!==1){return false;}return n===2;",
        "if(n===1){}else{return false;}return n===2;",
        "while(n!==1){}return n===2;",
        "for(;n!==1;){}return n===2;",
        "while(c){if(n!==1){break;}return n===2;}return false;",
        "while(c){if(n===1){}else{continue;}return n===2;}return false;",
        "if(n===1){if(c){return n===2;}}return false;",
        "if(n===1){n=2;return n===2;}return false;",
        "if(n===1){while(c){return n===2;}}return false;",
        "if(n===1){return c?(p&&(n===2)):false;}return false;",
        "if(n===1){const a=n;return a===2;}return false;",
        "if(wide===1){return wide===2;}return false;",
        "if(n===1){return (c?1:2)===n;}return false;",
    ] {
        check(body, true);
    }
    for body in [
        "if(n===1){}return n===2;",
        "if(n!==1){}else{}return n===2;",
        "while(n===1){break;}return n===2;",
        "for(;n===1;){break;}return n===2;",
        "if(n===1){return m===2;}return n===2;",
        "if(n===1){return n===m;}return false;",
        "if(n===m){return m===2;}return false;",
        "if(n!==m){}else{return m===2;}return false;",
        "if(n===m){return n===2;}return false;",
        "if(value()===n){return n===2;}return false;",
        "if(n===1){return (n+0)===2;}return false;",
        "if(n===1){let n=3;return n===2;}return false;",
        "const alias=n;if(alias===1){return n===2;}return false;",
    ] {
        check(body, false);
    }
}
#[test]
fn documented_reviewer_narrowing_forms_are_rejected() {
    // The tracking note supplies these three forms. It does not list the other twelve.
    for body in [
        "if(p){return p===false;}return false;",
        "return n===1&&n===2;",
        "if(n!==1){return true;}return n===2;",
    ] {
        check(body, true);
    }
}
#[test]
fn tested_regions_do_not_copy_the_active_set_per_statement() {
    let mut body = String::new();
    for i in 0..700 {
        body.push_str(&format!("let a{i}=0;"));
    }
    for i in 0..700 {
        body.push_str(&format!("if(a{i}!==1){{return false;}}"));
    }
    body.push_str(&"if(c){}".repeat(2000));
    body.push_str("return a0===2;");
    check(&body, true);
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
