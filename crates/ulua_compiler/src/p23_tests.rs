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
        "const xs=[1];xs.reduce((a:number):number=>a+1,0);",
        "const xs=[1];xs.reduceRight((a:number):number=>a+1,0);",
        "const xs=[1];xs.reduce((a:number,v:number,i:number,xs:number[]):number=>a+v,0);",
        "const xs=[1];xs.reduceRight((a:number,v:number,i:number,xs:number[]):number=>a+v,0);",
        "const xs=[1];xs.sort((a:number):number=>a);",
        "const xs=[1];xs.sort((a:number,b:number,i:number):number=>a-b);",
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
        let body = format!("const f=()=>1;const xs:(()=>number)[]=[f];xs.{method}(f);");
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
        "const xs=[1];xs.reduce((a:number,v:string,i:number):number=>a,0);",
        "const xs=[1];xs.reduceRight((a:number,v:number,i:string):number=>a+v,0);",
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
    let (prelude, _) = crate::emitter::methods::prelude(&checked.methods).unwrap();
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
fn helpers_reject_constructed_invalid_arities() {
    use crate::ir::Method;
    for (method, arity) in [
        (Method::Reduce, 1),
        (Method::Reduce, 4),
        (Method::ReduceRight, 1),
        (Method::ReduceRight, 4),
        (Method::Sort, 1),
        (Method::Sort, 3),
        (Method::Map, 0),
        (Method::Map, 3),
        (Method::Slice, 1),
    ] {
        let methods = [(method, arity)].into_iter().collect();
        let error = crate::emitter::methods::prelude(&methods).unwrap_err();
        assert_eq!(error.code, "internal", "{method:?}: {arity}");
    }
}

#[test]
fn nested_loop_local_boundary_counts_the_live_maximum() {
    let head = (0..32)
        .map(|i| format!("i{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    let params = (0..32)
        .map(|i| format!("p{i}:number{}", if i >= 28 { "=1" } else { "" }))
        .collect::<Vec<_>>()
        .join(",");
    let helper =
        format!("function one():number{{return 1;}}function f({params}):number{{return p0;}}");
    let mut call = format!("f({})", vec!["one()"; 28].join(","));
    for _ in 0..3 {
        call = format!("f({},{call})", vec!["one()"; 31].join(","));
    }
    for tails in [0, 1, 2, 3, 4] {
        // The call retains 129 slots. Four head tails and three more tails reach 136.
        let body = format!(
            "{}{}{call};{}",
            format!("for(let {head};false;){{").repeat(4),
            "for(;false;){".repeat(tails),
            "continue;}".repeat(4 + tails)
        );
        let result = compile(
            (helper.clone() + &module(&body)).as_bytes(),
            Limits::default(),
        );
        if tails <= 3 {
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
