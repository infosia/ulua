//! Round 5 joins, transfer edges, and boolean initializer restrictions.
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
fn reviewer_reproductions() {
    for body in [
        "if(c){return false;}else{if(n!==1){return false;}}return n===2;",
        "{if(n!==1){return false;}}return n===2;",
        "while(true){if(n!==1){return false;}break;}return n===2;",
        "let r=false;for(let i=0;i===0;i=i===1?5:6){r=c;}return r;",
        "if(c){if(n===1){}else{return false;}}else{return true;}return n===2;",
    ] {
        check(body, Some("C7"));
    }
}

#[test]
fn branch_and_block_joins_intersect_only_reaching_edges() {
    for (body, rejected) in [
        (
            "if(c){if(n!==1){return false;}}else{if(n!==2){return false;}}return n===3;",
            true,
        ),
        ("if(c){if(n!==1){return false;}}else{}return n===3;", false),
        ("if(c){if(p){return false;}}return n===3;", false),
        ("{if(n!==1){return false;}n=5;}return n===3;", true),
        (
            "if(c){if(s!=='a'){return false;}}else{if(s!=='b'){return false;}}return s==='c';",
            true,
        ),
        ("if(c){if(s!=='a'){return false;}}return s==='c';", false),
        (
            "if(c){return false; if(n!==1){return false;}}return n===3;",
            false,
        ),
        (
            "if(c){{if(n!==1){return false;}}}else{return false;}return n===3;",
            true,
        ),
    ] {
        check(body, rejected.then_some("C7"));
    }
}

#[test]
fn loop_exit_intersects_false_and_each_break_edge() {
    for (body, rejected) in [
        ("while(n!==1){}return n===2;", true),
        ("while(n!==1){break;}return n===2;", false),
        (
            "while(true){if(c){if(n!==1){return false;}break;}if(n!==2){return false;}break;}return n===3;",
            true,
        ),
        (
            "while(true){if(c){break;}if(n!==1){return false;}break;}return n===3;",
            false,
        ),
        (
            "while(true){if(n!==1){return false;}while(c){break;}break;}return n===3;",
            true,
        ),
        (
            "while(c){if(n!==1){return false;}continue;}return n===3;",
            false,
        ),
        ("for(;n!==1;){}return n===2;", true),
        ("for(;n!==1;){break;}return n===2;", false),
        ("for(;;){if(n!==1){return false;}break;}return n===3;", true),
        (
            "for(;;){if(c){break;}if(n!==1){return false;}break;}return n===3;",
            false,
        ),
        (
            "while(true){if(n!==1){return false;}break;if(n!==2){return false;}}return n===3;",
            true,
        ),
    ] {
        check(body, rejected.then_some("C7"));
    }
}

#[test]
fn for_update_intersects_body_end_and_each_continue() {
    for (body, rejected) in [
        (
            "for(let i=0;i===0;i=n===2?5:6){if(n!==1){return false;}}return false;",
            true,
        ),
        (
            "for(let i=0;i===0;i=n===2?5:6){if(c){continue;}if(n!==1){return false;}}return false;",
            false,
        ),
        (
            "for(let i=0;i===0;i=n===2?5:6){if(c){if(n!==1){return false;}continue;}if(n!==2){return false;}}return false;",
            true,
        ),
        (
            "for(let i=0;i===0;i=n===2?5:6){if(c){return false;}if(n!==1){return false;}continue;}return false;",
            true,
        ),
        (
            "for(let i=0;i===0;i=n===2?5:6){if(c){break;}if(n!==1){return false;}}return false;",
            true,
        ),
        (
            "for(let i=0;i===0;i=n===2?5:6){if(c){if(n!==1){return false;}continue;}continue;}return false;",
            false,
        ),
        (
            "for(let i=0;i===0;i=s==='b'?5:6){if(s!=='a'){return false;}continue;}return false;",
            true,
        ),
        (
            "for(let i=0;i===0;i=n===2?5:6){if(n!==1){return false;}while(c){continue;}continue;}return false;",
            true,
        ),
    ] {
        check(body, rejected.then_some("C7"));
    }
}

#[test]
fn loop_test_uses_entry_only_and_assignments_preserve_tests() {
    for (body, rejected) in [
        ("while(n===1){n=2;}return n===3;", false),
        (
            "while(n>0){if(n!==1){return false;}n=0;}return n===3;",
            false,
        ),
        ("for(let i=0;n===1;i++){n=2;}return n===3;", false),
        (
            "for(let i=0;i<2;i=n===2?5:6){if(n!==1){return false;}n=2;}return false;",
            true,
        ),
        ("if(n!==1){return false;}n=2;return n===3;", true),
        ("if(n===1){}if(n===2){}return false;", false),
        (
            "for(let i=0;i<10;i++){if(i===3){continue;}if(i===7){break;}}return false;",
            false,
        ),
    ] {
        check(body, rejected.then_some("C7"));
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
fn expression_edges_join_before_the_next_statement() {
    for (body, rejected) in [
        (
            "let v:boolean=(n===1||n!==2)?true:false;return n===3;",
            true,
        ),
        ("let v:boolean=(n===1||n!==2)&&c;return n===3;", true),
        ("let v:boolean=(n===1||n!==2)||c;return n===3;", true),
        ("let v:boolean=n===1?true:false;return n===3;", false),
        ("let v:boolean=n===1&&c;return n===3;", false),
        ("let v:boolean=n!==1||c;return n===3;", false),
    ] {
        check(body, rejected.then_some("C7"));
    }
}

#[test]
fn many_exit_edges_share_a_large_tested_prefix() {
    let mut body = String::new();
    for i in 0..700 {
        body.push_str(&format!("let a{i}=0;"));
    }
    body.push_str("while(true){");
    for i in 0..700 {
        body.push_str(&format!("if(a{i}!==1){{return false;}}"));
    }
    body.push_str(&"if(c){break;}".repeat(2000));
    body.push_str("break;}return a0===2;");
    check(&body, Some("C7"));
}
