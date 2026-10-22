//! Empty and minimal forms exercise emitter guards through the public compiler.
//! Measured Debug cost: 1.56 seconds for 81 shapes at depths 1 through 64 on macOS arm64.
//! No assertion depends on elapsed time.
use crate::{Limits, compile};

struct Shape {
    name: &'static str,
    head: &'static str,
    open: &'static str,
    seed: &'static str,
    close: &'static str,
    tail: &'static str,
}

impl Shape {
    fn source(&self, depth: usize) -> String {
        format!(
            "{}{}{}{}{}",
            self.head,
            self.open.repeat(depth),
            self.seed,
            self.close.repeat(depth),
            self.tail,
        )
    }
}

const TRIES: &[Shape] = &[
    statement("try_catch", "try{", "", "}catch(e){}"),
    statement("try_finally", "try{", "", "}finally{}"),
    statement("catch_body", "try{}catch(e){", "", "}"),
];

const fn statement(
    name: &'static str,
    open: &'static str,
    seed: &'static str,
    close: &'static str,
) -> Shape {
    Shape {
        name,
        head: "function f():void{",
        open,
        seed,
        close,
        tail: "}",
    }
}

const fn expression(
    name: &'static str,
    open: &'static str,
    seed: &'static str,
    close: &'static str,
) -> Shape {
    Shape {
        name,
        head: "function id(x:number):number{return x;} function f():void{const v=",
        open,
        seed,
        close,
        tail: ";}",
    }
}

#[test]
fn empty_try_at_23_reports_nesting_limit() {
    on_compiler_thread(|| {
        let mut failures = vec![];
        for shape in TRIES {
            let source = shape.source(23);
            let result = compile(source.as_bytes(), Limits::default());
            match result {
                Err(d) if d.code == "limit:nesting-depth" => {}
                other => failures.push(format!("{}: {other:?}", shape.name)),
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}

#[test]
fn guarded_forms_depth_sweep() {
    on_compiler_thread(sweep);
}

fn on_compiler_thread(test: fn()) {
    std::thread::Builder::new()
        .stack_size(crate::COMPILER_STACK_BYTES)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

fn sweep() {
    // Statements: statement() guards the node, including empty blocks and operandless exits.
    // Each leaf also sits inside a block chain, so its own guard reaches the boundary.
    let statements = [
        statement("block", "{", "", "}"),
        statement("if", "if(true){", "", "}"),
        statement("else_if", "if(false){}else ", "if(true){}", ""),
        statement("while", "while(true){", "", "}"),
        statement("for", "for(;;){", "", "}"),
        statement("for_of_array", "for(const x of [0]){", "", "}"),
        statement("for_of_string", "for(const x of 'x'){", "", "}"),
        Shape {
            head: "function value():number{return 0;}function f():void{",
            ..statement("for_call_init", "for(let i=value();i<1;i++){", "", "}")
        },
        statement("function", "function g():void{", "", "}"),
        statement("arrow", "const g=():void=>{", "", "};"),
        Shape {
            head: "class C{m():void{",
            tail: "}}",
            ..statement("class_method", "{", "", "}")
        },
        Shape {
            head: "class C{constructor(){",
            tail: "}}",
            ..statement("class_constructor", "{", "", "}")
        },
        statement("try_catch_finally", "try{", "", "}catch(e){}finally{}"),
        statement("finally_body", "try{}finally{", "", "}"),
        statement("return", "{", "return;", "}"),
        statement("throw", "{", "throw new Error('x');", "}"),
        statement("break", "while(true){", "break;", "}"),
        statement("continue", "while(true){", "continue;", "}"),
        statement("variable", "{", "const x=0;", "}"),
        statement("assignment", "{", "let x=0;x=1;", "}"),
        statement("update", "{", "let x=0;x++;", "}"),
        statement("empty_array", "{", "const x:number[]=[];", "}"),
        statement("empty_object", "{", "const x={};", "}"),
        statement("empty_template", "{", "const x=``;", "}"),
        statement("logical_leaf", "{", "const x=true&&false;", "}"),
        statement("conditional_leaf", "{", "const x=true?false:true;", "}"),
        statement("error_no_arguments", "{", "const x=new Error();", "}"),
        Shape {
            head: "function get():number[]|null{return null;}function f():void{",
            ..statement("optional_call", "{", "get()?.push(0);", "}")
        },
        statement("empty_arrow", "{", "const x=():void=>{};", "}"),
        statement("empty_function", "{", "function g():void{}", "}"),
        statement("element_assignment", "{", "const a=[0];a[0]=1;", "}"),
        statement(
            "element_compound_assignment",
            "{",
            "const a=[0];a[0]+=1;",
            "}",
        ),
        statement("field_assignment", "{", "const a={x:0};a.x=1;", "}"),
        statement("field_update", "{", "const a={x:0};a.x++;", "}"),
        statement("zero_argument_call", "{", "f();", "}"),
        statement("array_push", "{", "const a:number[]=[];a.push(0);", "}"),
        statement("json_parse", "{", "const x:number=JSON.parse('0');", "}"),
        Shape {
            head: "function f():void{try{}catch(e){if(e instanceof Error){",
            tail: "}}}",
            ..statement("error_field_store", "{", "e.message='y';", "}")
        },
        Shape {
            head: "function f():void{try{}catch(e){if(e instanceof Error){",
            tail: "}}}",
            ..statement("error_field_read", "{", "const x=e.message;", "}")
        },
    ];
    // Expressions: expression(), pure_bounded(), condition(), access_receiver(), and binding_expression().
    let expressions = [
        expression("parentheses", "(", "0", ")"),
        expression("unary", "- ", "0", ""),
        expression("binary_left", "", "0", "+1"),
        expression("binary_right", "1+", "0", ""),
        expression("logical", "true&&", "true", ""),
        expression("logical_or", "false||", "false", ""),
        expression("logical_parentheses", "(true)&&(", "true", ")"),
        expression("logical_unary", "true&&!", "true", ""),
        expression("logical_binary", "true&&0===0&&", "true", ""),
        expression("conditional", "true?", "0", ":1"),
        expression("conditional_alternate", "false?1:", "0", ""),
        expression("call", "id(", "0", ")"),
        expression("builtin_call", "Math.abs(", "0", ")"),
        expression("array", "[", "0", "]"),
        expression("object", "{x:", "0", "}"),
        expression("template", "`${", "0", "}`"),
        expression("index", "[", "0", "][0]"),
        expression("field", "{x:", "0", "}.x"),
        Shape {
            head: "type R={x:number};function get(x:number):R|null{return null;}function f():void{const v=",
            ..expression("optional_field", "(get(", "0", ")?.x??0)")
        },
        expression("method_call", "", "'x'", ".toLowerCase()"),
        expression("arrow_expression", "((():number=>", "0", ")())"),
        expression("json_stringify", "JSON.stringify(", "0", ")"),
        expression("string_length", "`${", "'x'", "}`.length"),
        expression("array_length", "[", "0", "].length"),
        expression(
            "reduce_callback",
            "[",
            "0",
            "].reduce((a:number,x:number):number=>a+x,0)",
        ),
        expression(
            "filter_callback",
            "[",
            "0",
            "].filter((x:number):boolean=>true)[0]",
        ),
    ];
    let special = [
        Shape {
            head: "function id():number{return 0;}function f():number{",
            ..statement("return_expression", "{", "return id();", "}")
        },
        Shape {
            head: "function f():void{if(",
            tail: "){} }",
            ..expression("condition_logical", "true&&", "true", "")
        },
        Shape {
            head: "class C{m(x:number):number{return x;}}function f():void{const c=new C();const v=",
            ..expression("class_method_call", "c.m(", "0", ")")
        },
        Shape {
            head: "class C{constructor(x:C|null){}}function f():void{const v=",
            ..expression("new", "new C(", "null", ")")
        },
        Shape {
            head: "async function p(x:number):Promise<number>{return x;}async function f():Promise<void>{const v=",
            ..expression("await", "await p(", "0", ")")
        },
        Shape {
            head: "function f():void{const v=",
            ..expression("callback", "[", "0", "].map((x:number):number=>x)[0]")
        },
        Shape {
            head: "export function f():void{",
            ..statement("export", "{", "", "}")
        },
        Shape {
            head: "class C{x:number=",
            tail: ";}",
            ..expression("class_field", "(", "0", ")")
        },
        Shape {
            head: "function f(x:number=",
            tail: "):void{}",
            ..expression("parameter_default", "(", "0", ")")
        },
        Shape {
            head: "async function f():Promise<void>{await (async():Promise<void>=>{",
            tail: "})();}",
            ..statement("await_try", "try{", "", "}catch(e){}")
        },
        Shape {
            head: "type R={x:number};function f():void{((():R=>{",
            tail: "return {x:0};})()).x++;}",
            ..statement("update_receiver_try", "try{", "", "}catch(e){}")
        },
        Shape {
            head: "function f():void{const e=new Error();",
            ..statement(
                "logical_instanceof",
                "{",
                "const x=true&&(e instanceof Error);",
                "}",
            )
        },
        Shape {
            head: "function g():boolean|null{return null;}function f():void{",
            ..statement("logical_coalesce", "{", "const x=true&&(g()??false);", "}")
        },
    ];
    let start = std::time::Instant::now();
    let mut failures = vec![];
    for shape in TRIES
        .iter()
        .chain(&statements)
        .chain(&expressions)
        .chain(&special)
    {
        let mut accepted = 0;
        for depth in 1..=64 {
            let source = shape.source(depth);
            match compile(source.as_bytes(), Limits::default()) {
                Ok(_) => accepted = depth,
                Err(d) if d.code.starts_with("limit:") => {}
                Err(d) => failures.push(format!(
                    "{} n={depth}: {}: {}",
                    shape.name, d.code, d.message
                )),
            }
        }
        if accepted == 0 {
            failures.push(format!("{} never reaches the emitter", shape.name));
        }
        eprintln!("depth sweep {} accepted={accepted}", shape.name);
    }
    eprintln!("depth sweep cost={:.3}s", start.elapsed().as_secs_f64());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
