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
fn function_type_equality_applies_at_each_value_flow() {
    let target = "(n:number)=>number";
    for (declaration, equal) in [
        ("function f(n:number):number{return n;}", true),
        ("function f(n:number=1):number{return n;}", false),
        ("function f(n:string):number{return 1;}", false),
        ("function f(n:number):boolean{return true;}", false),
        ("function f():number{return 1;}", false),
        ("function f(n:number,m:number):number{return n;}", false),
    ] {
        for flow in [
            format!("function take(g:{target}):void{{}}take(f);"),
            format!("const g:{target}=f;"),
            format!("function give():{target}{{return f;}}give();"),
            format!("const give=():{target}=>f;give();"),
            format!("let g:{target}=(n:number):number=>n;g=f;"),
            "const g=true?f:(n:number):number=>n;".into(),
            "const g=true?(n:number):number=>n:f;".into(),
            format!("function take(g:{target}=f):void{{}}take();"),
        ] {
            let source = body(&format!("{declaration}{flow}"));
            if equal {
                accept(&source);
            } else {
                reject(&source, "C1");
            }
        }
    }
}

#[test]
fn inferred_function_values_compare_defaults_in_both_directions() {
    for left_default in ["", "=1", "=2"] {
        for right_default in ["", "=3"] {
            let source = body(&format!(
                "function f(n:number{left_default}):number{{return n;}}function g(n:number{right_default}):number{{return n;}}const h=true?f:g;h(1);"
            ));
            if left_default.is_empty() == right_default.is_empty() {
                accept(&source);
            } else {
                reject(&source, "C1");
            }
            reject(
                &body(&format!(
                    "function f(n:number{left_default}):number{{return n;}}function g(n:number{right_default}):number{{return n;}}let h=f;h=g;h(1);"
                )),
                "C11",
            );
        }
    }
    // The result type contains another function signature with its own default count.
    reject(
        &body("function f(n:number=1):number{return n;}const make=():(n:number)=>number=>f;"),
        "C1",
    );
}

#[test]
fn function_type_equality_is_recursive_at_each_value_flow() {
    for (setup, target, matching) in [
        (
            "function f():(n:string)=>number{return (n:string):number=>1;}",
            "()=>(n:number)=>number",
            "():(n:number)=>number=>((n:number):number=>n)",
        ),
        (
            "function f(g:(n:string)=>number):number{return 1;}",
            "(g:(n:number)=>number)=>number",
            "(g:(n:number)=>number):number=>g(1)",
        ),
        (
            "function f():()=>boolean{return ()=>true;}",
            "()=>()=>number",
            "():()=>number=>()=>1",
        ),
    ] {
        for flow in [
            format!("function take(g:{target}):void{{}}take(f);"),
            format!("const g:{target}=f;"),
            format!("function give():{target}{{return f;}}give();"),
            format!("let g:{target}={matching};g=f;"),
            format!("const g=true?f:{matching};"),
            format!("const g=true?{matching}:f;"),
        ] {
            reject(&body(&format!("{setup}{flow}")), "C1");
        }
    }
}

#[test]
fn luau_function_types_keep_composite_boundaries() {
    use crate::ir::{Type, Types};
    let mut types = Types::new();
    let number = Type::Number;
    let boolean = Type::Boolean;
    let callback = types.intern(vec![number], boolean, 1);
    let nested = types.intern(vec![callback], number, 0);
    for (ty, expected) in [
        (
            types.intern(vec![callback, number], boolean, 2),
            "((number) -> boolean, number) -> boolean",
        ),
        (
            types.intern(vec![], callback, 0),
            "() -> (number) -> boolean",
        ),
        (
            types.intern(vec![number, callback], boolean, 1),
            "(number, ((number) -> boolean)?) -> boolean",
        ),
        (
            types.intern(vec![nested], boolean, 0),
            "(((((number) -> boolean)?) -> number)?) -> boolean",
        ),
    ] {
        assert_eq!(types.luau(ty), expected);
    }
}

#[test]
fn spilled_table_fields_use_function_type_aliases() {
    let declarations = (0..240)
        .map(|i| format!("const f{i}:(n:number)=>boolean=(n:number):boolean=>n>{i};"))
        .collect::<String>();
    let compiled = compile(
        body(&format!("{declarations}print(`${{f239(240)}}`);")).as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let spill = compiled
        .luau
        .lines()
        .find(|line| line.contains("local __ulua_spill"))
        .unwrap();
    assert!(spill.contains(": __ulua_fn"));
    assert!(!spill.contains(" -> "));
}

#[test]
fn parameter_defaults_form_one_trailing_suffix() {
    for parameters in [
        "a:number=1,b:number",
        "a:number,b:number=2,c:number",
        "a:number=1,b:number=2,c:number",
        "a:number=1,b:number,c:number=3",
    ] {
        reject(&body(&format!("function f({parameters}):void{{}}")), "C11");
    }
    for parameters in [
        "a:number,b:number",
        "a:number,b:number=2",
        "a:number=1,b:number=2",
    ] {
        accept(&body(&format!("function f({parameters}):void{{}}f(1,2);")));
    }
}

#[test]
fn optional_function_parameters_keep_the_function_result_required() {
    let source = "function test(n:number,f:(a:number)=>boolean=(a:number):boolean=>a>1):boolean{return f(n);}export function main():void{print(`${test(2)} ${test(0)} ${test(0,(a:number):boolean=>a===0)}`);}";
    let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
    assert!(compiled.luau.contains("(number, (__ulua_fn0)?) -> boolean"));
    accept(
        "function outer(f:(g:(n:number)=>number)=>((n:number)=>number)=(g:(n:number)=>number):(n:number)=>number=>g):(n:number)=>number{return f((n:number):number=>n);}export function main():void{const g=outer();print(`${g(1)}`);}",
    );
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
fn callback_equality_does_not_depend_on_enclosing_tests() {
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
            accept(&format!(
                "function take(f:()=>boolean):void{{}}function run(p:number):void{{{binding}if({name}{test}1){arms}}}export function main():void{{}}"
            ));
        }
        accept(&format!(
            "function run(p:number):void{{{binding}if({name}===1){{function f():boolean{{return {name}===2;}}f();}}}}export function main():void{{}}"
        ));
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
            "function f(n:number=1):number{return n;}function g(n:number):number{return n;}let call:(n:number)=>number=f;call=g;call();",
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
    let head = (0..32)
        .map(|i| format!("i{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    let loops = format!(
        "{}{}",
        format!("for(let {head};false;){{").repeat(5),
        "}".repeat(5)
    );
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

#[test]
fn declarations_reach_references_in_every_created_closure() {
    for source in [
        "const r=mk();r();let x=1;function mk():()=>number{return ():number=>x;}",
        "let r=()=>0;setup();r();let x=1;function setup():void{r=()=>x;}",
        "const r=mk();r();let x=1;function mk():()=>number{return ()=>read();}function read():number{return x;}",
        "const r=mk();r();let x=1;function mk():()=>number{function nested():()=>number{return ()=>x;}return nested();}",
        "const r=mk();r();let x=1;function mk():()=>()=>number{return ():()=>number=>()=>x;}",
        "f();let x=1;function f(g:()=>number=()=>x):number{return g();}",
    ] {
        reject(&body(source), "C10");
    }
    reject(
        "let r=()=>0;const ignored=setup();let x=1;function setup():number{r=()=>x;return 0;}export function main():void{r();}",
        "C10",
    );
    accept(&body(
        "function mk():()=>number{let x=1;return ()=>x;}const r=mk();r();",
    ));
    reject(
        "const ignored=set();let x=1;function set():number{x=4;return 0;}export function main():void{}",
        "C10",
    );
    accept(&body(
        "let x=1;const r=mk();function mk():()=>number{return ()=>x;}r();",
    ));
}

#[test]
fn function_valued_arrow_results_need_their_own_annotation() {
    for source in [
        "const make=()=>()=>1;",
        "const f=()=>1;const make=()=>f;",
        "function get():()=>number{return ()=>1;}const make=()=>get();",
        "const f=()=>1;const make=()=>true?f:f;",
        "const make:()=>()=>number=()=>()=>1;",
    ] {
        reject(&body(source), "C11");
    }
    accept(&body("const make=():()=>number=>()=>1;const f=make();f();"));
}

#[test]
fn defaults_cannot_refer_to_top_level_body_declarations() {
    for source in [
        "function g():number{return 1;}function f(a:number=g()):number{function g():number{return 2;}return a;}",
        "let x=1;function f(a:number=x):number{let x=2;return a;}",
        "let x=1;function f(a:()=>number=()=>x):number{const x=2;return a();}",
        "function f(a:number=Infinity):number{let Infinity=2;return a;}",
        "function f(a:string=String(1)):string{function String(n:number):string{return 'x';}return a;}",
    ] {
        reject(&body(source), "C11");
    }
    accept(&body(
        "let x=1;function f(a:number=x):number{{let x=2;}return a;}f();",
    ));
    accept(&body(
        "let x=1;function f(a:(x:number)=>number=(x:number)=>x):number{let x=2;return a(x);}f();",
    ));
}

#[test]
fn nested_aliases_have_one_function_signature_per_declaration() {
    let result = format!("{}number", "()=>".repeat(20));
    let mut parameter = String::from("number");
    for _ in 0..20 {
        parameter = format!("(g:{parameter})=>number");
    }
    let source = body(&format!(
        "function result(f:{result}):{result}{{return f;}}function parameter(f:{parameter}):{parameter}{{return f;}}"
    ));
    let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
    let aliases: Vec<_> = compiled
        .luau
        .lines()
        .filter(|line| line.starts_with("type "))
        .collect();
    assert_eq!(aliases.len(), 43);
    for alias in aliases {
        assert_eq!(alias.matches(" -> ").count(), 1, "{alias}");
    }
}
