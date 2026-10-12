//! Method admission, selected preludes, and the live loop-local boundary.
use crate::{Limits, compile};
fn module(body: &str) -> String {
    format!("export function main():void{{{body}}}")
}

#[test]
fn methods_keep_exact_argument_and_callback_types() {
    for body in [
        "const xs=[1];xs.sort();",
        "const xs=[1];xs.reduce((a:number,v:number):number=>a+v);",
        "const xs=[1];xs.map((v:number,i:number,a:number[]):number=>v);",
        "const xs=[1];xs.map(():number=>1);",
        "const xs=[1];xs.flat();",
        "const xs=[1];xs.find((v:number):boolean=>v>0);",
        "const xs=[1];xs.findLast((v:number):boolean=>v>0);",
        "const xs=[1];xs.entries();",
        "const xs=[1];xs.keys();",
        "const xs=[1];xs.values();",
        "const xs=[1];xs.toString();",
        "const xs=[1];xs.splice(0,1,2);",
        "const xs=[1];xs.concat([1],[2]);",
        "const xs=[1];xs.unshift(1,2);",
        "const xs=[1];xs.map((v:number):number=>v,1);",
        "const xs=[1];xs.length=1;",
        "const xs=[1];const zero=0;xs.length=zero;",
        "const xs=[1];xs.length+=0;",
        "const xs=[1];xs.length=-0;",
        "const xs=[1];const method=xs.map;",
        "const xs=[[1]];xs.join();",
        "const xs=[()=>1];xs.join();",
    ] {
        assert_eq!(
            compile(module(body).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C6",
            "{body}"
        );
    }
    for body in [
        "const xs=[1];xs.length=00;",
        "const xs=[1];const f=():void=>xs.push(1);",
    ] {
        assert_eq!(
            compile(module(body).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C11"
        );
    }
    for body in [
        "const xs=[1];const result=xs.forEach((v:number):void=>{});",
        "const xs=[1];return xs.forEach((v:number):void=>{});",
    ] {
        assert_eq!(
            compile(module(body).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C2"
        );
    }
    compile(
        module("const xs=[1];if(xs.indexOf(1)===0){}const n=xs.indexOf(1);if(n===0){}").as_bytes(),
        Limits::default(),
    )
    .unwrap();
    for method in ["indexOf", "lastIndexOf", "includes"] {
        let body = format!("const f=()=>1;const xs=[f];xs.{method}(f);");
        assert_eq!(
            compile(module(&body).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C6"
        );
    }
    for body in [
        "const xs=[1];xs.indexOf('a');",
        "const xs=[1];xs.slice('a');",
        "const xs=[1];xs.concat(['a']);",
        "const xs=[1];xs.map((v:string):number=>1);",
        "const xs=[1];xs.filter((v:number):number=>v);",
        "const xs=[1];xs.map((v:number,i:string):number=>v);",
        "const xs=[1];xs.sort((a:number,b:string):number=>a);",
        "const xs=[1];xs.reduce((a:string,v:number):string=>a,0);",
        "const xs=[1];xs.reduce((a:string,v:number):number=>v,'');",
    ] {
        assert_eq!(
            compile(module(body).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C1",
            "{body}"
        );
    }
}

#[test]
fn helpers_appear_once_only_when_used_and_the_output_budget_includes_them() {
    let source = module(&format!(
        "const xs=[1];{}",
        "xs.slice().slice().slice();xs.map((v:number):number=>v).filter((v:number):boolean=>v>0);"
            .repeat(10)
    ));
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::ts()).parse();
    let checked =
        crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING).unwrap();
    let (prelude, _) = crate::emitter::methods::prelude(&checked.methods);
    let output = crate::emitter::emit(&checked, crate::OUTPUT_FACTOR).unwrap();
    for helper in ["slice_0<T>", "map_1<T, U>", "filter_1<T>"] {
        assert_eq!(
            output
                .matches(&format!("local function __ulua_{helper}"))
                .count(),
            1
        );
    }
    assert!(!output.contains("local function __ulua_sort"));
    assert!(!output.contains("table.sort"));
    let base = crate::OUTPUT_PRELUDE_BYTES + prelude.len();
    assert!(output.len() <= base + crate::OUTPUT_FACTOR * source.len());
    assert_eq!(
        crate::emitter::emit(&checked, 0).unwrap_err().code,
        "limit:output"
    );
    let empty = compile(module("").as_bytes(), Limits::default())
        .unwrap()
        .luau;
    assert!(!empty.contains("__ulua_position"));
    assert!(!empty.contains("__ulua_map"));
    for method in ["at", "lastIndexOf"] {
        let output = compile(
            module(&format!("const xs=[1];xs.{method}(0);")).as_bytes(),
            Limits::default(),
        )
        .unwrap()
        .luau;
        assert!(output.contains("local function __ulua_integer"));
        assert!(!output.contains("local function __ulua_position"));
    }
}

#[test]
fn nested_loop_local_boundary_counts_the_live_maximum() {
    let head = (0..32)
        .map(|i| format!("i{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    for tails in [0, 1, 2, 3] {
        // Four heads need 132 locals. Number and boolean slots need two more.
        let body = format!(
            "{}{}{}",
            format!("for(let {head};false;){{").repeat(4),
            "for(;false;){".repeat(tails),
            "}".repeat(4 + tails)
        );
        let result = compile(module(&body).as_bytes(), Limits::default());
        if tails <= 2 {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().code, "limit:temporaries");
        }
    }
    compile(
        module(&"for(let i=0;i<3;i++){}".repeat(200)).as_bytes(),
        Limits::default(),
    )
    .unwrap();
}
