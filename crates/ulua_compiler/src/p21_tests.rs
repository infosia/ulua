//! Direct P2.1 surface, declaration-order, depth, and callback-flow tests.
use crate::{Limits, compile};

fn accept(source: &str) {
    compile(source.as_bytes(), Limits::default()).unwrap();
}
fn reject(source: &str, code: &str) {
    assert_eq!(
        compile(source.as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        code,
        "{source}"
    );
}
fn body(source: &str) -> String {
    format!("export function main():void{{{source}}}")
}

#[test]
fn annotations_defaults_void_and_callee_forms_use_the_closed_surface() {
    for source in [
        "function make():()=>number{return ()=>1;}export function main():void{const f=make();print(`${f()}`);}",
        "function apply(f:(n:number)=>number):number{return f(4);}export function main():void{const f=(n:number):number=>n+1;print(`${apply(f)}`);}",
        "export function main():void{const f=():void=>print('a');f();((n:number):void=>print(`${n}`))(1);}",
        "export function main():void{const f=():void=>{return;};f();}",
        "export function main():void{function f(n:number=4):number{return n;}print(`${f()}`);}",
        "export function main():void{function f(n:number=4):number{return n;}const g:(n:number)=>number=f;g(1);}",
    ] {
        accept(source);
    }
    for source in [
        "const f=(n:number=1):number=>n;",
        "const f=()=>{return 1;};",
        "let f:()=>void=()=>{};",
        "const f=():number=>{if(true){return 1;}};",
        "const f=():void=>{print('a');};const x=f();",
        "function f(...xs:number[]):void{}",
    ] {
        reject(
            &body(source),
            if source.contains("const x=f()") {
                "C2"
            } else {
                "C11"
            },
        );
    }
    for source in [
        "const f=()=>1;print(`${f===f}`);",
        "const f=()=>1;print(`${f!=f}`);",
    ] {
        reject(&body(source), "C7");
    }
}
#[test]
fn function_valued_bindings_keep_their_declaration_tdz() {
    for source in [
        "const f=():number=>f();",
        "function f():number{return g();}f();const g=()=>1;",
        "const f=():number=>g();const g=()=>1;",
        "f();let n=1;function f():number{return g();}function g():number{return n;}",
        "const use=f;let n=1;function f():number{return n;}",
    ] {
        reject(&body(source), "C10");
    }
    accept(&body(
        "function outer():number{const n=1;function inner():number{return n;}return inner();}print(`${outer()}`);",
    ));
}
#[test]
fn callback_creation_preserves_parameter_let_and_const_tests() {
    for binding in ["", "let x:number=p+1;", "const x:number=p+1;"] {
        let name = if binding.is_empty() { "p" } else { "x" };
        for comparison in ["===", "==", "!==", "!="] {
            let test = if comparison.contains('!') {
                "!=="
            } else {
                "==="
            };
            let region = if comparison.contains('!') {
                "else"
            } else {
                "then"
            };
            let callback = format!("take(():boolean=>{name}{comparison}2);");
            let arms = if region == "then" {
                format!("{{{callback}}}")
            } else {
                format!("{{}}else{{{callback}}}")
            };
            reject(
                &format!(
                    "function take(f:()=>boolean):void{{}}function run(p:number):void{{{binding}if({name}{test}1){arms}}}export function main():void{{}}"
                ),
                "C7",
            );
        }
        reject(
            &format!(
                "function run(p:number):void{{{binding}if({name}===1){{function f():boolean{{return {name}===2;}}f();}}}}export function main():void{{}}"
            ),
            "C7",
        );
        accept(&format!(
            "function run(p:number):void{{{binding}const f=():boolean=>{name}===2;f();}}export function main():void{{}}"
        ));
    }
    // A nested test and a nested return do not change the continuation of the enclosing function.
    accept(
        "function run(p:number):void{const f=():boolean=>{if(p===1){return true;}return false;};print(`${p===2}`);}export function main():void{}",
    );
    // Function declarations are created at the entry of their block, before its later tests.
    accept(
        "function run(p:number):void{if(p===1){return;}function f():boolean{return p===2;}f();}export function main():void{}",
    );
}
#[test]
fn arrows_and_declarations_share_the_module_depth_measure() {
    for arrow in [false, true] {
        let mut source = String::from("export function main():void{");
        for i in 0..64 {
            source.push_str(&if arrow {
                format!("const f{i}=():void=>{{")
            } else {
                format!("function f{i}():void{{")
            });
        }
        source.push_str(&if arrow {
            "};".repeat(64)
        } else {
            "}".repeat(64)
        });
        source.push('}');
        reject(&source, "limit:nesting-depth");
    }
    let source = body("const f=():void=>{function g():void{print('a');}g();};f();");
    let boundary = (1..=crate::DEPTH_CEILING)
        .find(|limit| {
            compile(
                source.as_bytes(),
                Limits {
                    max_nesting_depth: *limit,
                    ..Limits::default()
                },
            )
            .is_ok()
        })
        .unwrap();
    assert!(boundary > 6);
    assert_eq!(
        compile(
            source.as_bytes(),
            Limits {
                max_nesting_depth: boundary - 1,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code,
        "limit:nesting-depth"
    );
}
#[test]
fn closures_in_each_for_head_position_give_c9_first() {
    for head in [
        "let i=0,f=()=>i;i<1;i++",
        "let i=0;take(()=>i);i++",
        "let i=0;i<1;i=take(()=>i)",
    ] {
        reject(&body(&format!("for({head}){{}}")), "C9");
    }
    reject(
        "const later:number[]=[];export function main():void{for(let i=0;take(()=>i);i++){}}",
        "C9",
    );
}

#[test]
fn assignments_do_not_remove_required_arguments_from_a_function_value() {
    reject(
        &body(
            "function f(n:number=1):number{return n;}function g(n:number):number{return n;}let call=f;call=g;call();",
        ),
        "C1",
    );
}
#[test]
fn each_nested_body_has_its_own_temporary_budget() {
    let loops = "for(;false;){}".repeat(100);
    accept(&body(&format!(
        "{loops}function f():void{{{loops}}}const g=():void=>{{{loops}}};f();g();"
    )));
    let loops = "for(;false;){}".repeat(crate::TEMPORARY_BOUND + 1);
    reject(
        &body(&format!("const g=():void=>{{{loops}}};g();")),
        "limit:temporaries",
    );
}

#[test]
fn hoisted_writes_obey_the_same_tdz_as_reads() {
    reject(&body("set();let x=1;function set():void{x=4;}"), "C10");
    accept(&body(
        "let x=1;set();function set():void{x=4;}print(`${x}`);",
    ));
}
