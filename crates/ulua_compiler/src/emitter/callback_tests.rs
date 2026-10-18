//! Callback form tests use small compiler-only fixtures.
use crate::{Limits, compile};

fn emitted(method: &str, callback: &str, extra: &str) -> String {
    let init = if method == "reduce" { ",0" } else { "" };
    let source = format!(
        "{extra} export function main():void{{const a=[1,2];a.{method}({callback}{init});}}"
    );
    compile(source.as_bytes(), Limits::default()).unwrap().luau
}

#[test]
fn inline_callbacks_omit_helpers_and_safe_length_tests() {
    for method in ["forEach", "map", "filter", "reduce"] {
        let callback = match method {
            "reduce" => "(acc:number,v:number,i:number)=>acc+v+i",
            "filter" => "(v:number,i:number)=>v>i",
            _ => "(v:number,i:number)=>v+i",
        };
        let code = emitted(method, callback, "");
        assert!(!code.contains(&format!("__ulua_{method}_")), "{code}");
        assert!(!code.contains("__ulua_trap(2)"), "{code}");
        assert!(code.contains("for __ulua_g"), "{code}");
    }
}

#[test]
fn inline_callbacks_keep_helpers_for_other_forms() {
    for method in ["forEach", "map", "filter", "reduce"] {
        let (params, value, result) = if method == "reduce" {
            ("acc:number,v:number", "acc+v", "number")
        } else if method == "filter" {
            ("v:number", "v>0", "boolean")
        } else {
            ("v:number", "v+1", "number")
        };
        for callback in [
            "named".to_owned(),
            format!("({params}):{result}=>{{return {value};}}"),
            format!("({params}):{result}=>pass(()=>{value})"),
        ] {
            let extra = format!(
                "function named({params}):{result}{{return {value};}}function pass(f:()=>{result}):{result}{{return f();}}"
            );
            let code = emitted(method, &callback, &extra);
            assert!(code.contains(&format!("__ulua_{method}_")), "{code}");
        }
    }
}

#[test]
fn inline_callbacks_keep_length_tests_for_calls() {
    for method in ["forEach", "map", "filter", "reduce"] {
        for (value, extra) in [
            ("f(v)", "function f(v:number):number{return v;}"),
            ("[v].join('').length", ""),
            (
                "new Box(v).n",
                "class Box{n:number;constructor(n:number){this.n=n;}}",
            ),
            ("Math.abs(v)", ""),
        ] {
            let callback = match method {
                "reduce" => format!("(acc:number,v:number)=>acc+{value}"),
                "filter" => format!("(v:number)=>{value}>0"),
                _ => format!("(v:number)=>{value}"),
            };
            let code = emitted(method, &callback, extra);
            assert!(!code.contains(&format!("__ulua_{method}_")), "{code}");
            assert!(code.contains("__ulua_trap(2)"), "{code}");
        }
    }
}

#[test]
fn inline_callbacks_reject_unsupported_parameter_and_async_forms() {
    for callback in [
        "async (v:number)=>v",
        "(v:number=1)=>v",
        "([v]:number[])=>v",
        "({v}:{v:number})=>v",
    ] {
        let source = format!("export function main():void{{const a=[1];a.forEach({callback});}}");
        assert!(compile(source.as_bytes(), Limits::default()).is_err());
    }
}

#[test]
fn inline_callbacks_keep_length_tests_for_host_calls() {
    for method in ["forEach", "map", "filter", "reduce"] {
        let (params, value, init) = match method {
            "reduce" => ("acc:number,v:number", "acc+host(v)", ",0"),
            "filter" => ("v:number", "host(v)>0", ""),
            _ => ("v:number", "host(v)", ""),
        };
        let source = format!(
            "export function main():void{{const a=[1];a.{method}(({params})=>{value}{init});}}"
        );
        let code = crate::compile_with_grants(
            source.as_bytes(),
            Limits::default(),
            &[crate::HostSignature::new("host", vec![1], 1)],
        )
        .unwrap()
        .luau;
        assert!(!code.contains(&format!("__ulua_{method}_")), "{code}");
        assert!(code.contains("__ulua_trap(2)"), "{code}");
    }
}

#[test]
fn inline_callbacks_safe_fields_elements_and_templates() {
    let code = compile(b"class Box{n:number;constructor(n:number){this.n=n;}}export function main():void{const a=[new Box(1)];const r={n:2};const other=[3];a.map((v:Box,i:number)=>`${v.n+r.n+other[i]+a.length}`);}", Limits::default()).unwrap().luau;
    assert!(!code.contains("__ulua_map_"), "{code}");
    assert!(!code.contains("__ulua_trap(2)"), "{code}");
}

#[test]
fn inline_callbacks_optional_call_keeps_its_helper() {
    let code = compile(
        b"export function main():void{const a:number[]|null=[1];a?.forEach((v:number)=>v);}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(code.contains("__ulua_forEach_"), "{code}");
}

#[test]
fn inline_callbacks_check_length_before_map_store() {
    let code = emitted(
        "map",
        "(v:number)=>f(v)",
        "function f(v:number):number{return v;}",
    );
    let start = code.find("for __ulua_g").unwrap();
    let body = &code[start..];
    let trap = body.find("__ulua_trap(2)").unwrap();
    let store = body.lines().find(|line| line.contains("] = (")).unwrap();
    assert!(trap < body.find(store).unwrap(), "{code}");
}

#[test]
fn inline_callbacks_missing_reduce_initial_value_returns_internal() {
    use crate::ir::{Expr, ExprKind, Method, Statement};
    fn remove_initial(statement: &mut Statement) -> bool {
        match statement {
            Statement::Located(_, statement) => remove_initial(statement),
            Statement::Eval(Expr {
                kind: ExprKind::Method(Method::Reduce, _, _, args),
                ..
            }) => {
                args.pop();
                true
            }
            _ => false,
        }
    }
    let source = "export function main():void{const a=[1];a.reduce((s:number,v:number)=>s+v,0);}";
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    let mut checked =
        crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING).unwrap();
    assert!(
        checked
            .functions
            .iter_mut()
            .flat_map(|f| &mut f.body)
            .any(remove_initial)
    );
    assert_eq!(
        crate::emitter::emit(&checked, crate::OUTPUT_FACTOR)
            .unwrap_err()
            .code,
        "internal"
    );
}

#[test]
fn inline_callbacks_decide_before_functions_in_reduce_initial_value() {
    let code = compile(
        b"export function main():void{const a=[1];a.reduce((s:number,v:number)=>s+v,(()=>10)());}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(!code.contains("__ulua_reduce_"), "{code}");
    assert!(code.contains("for __ulua_g"), "{code}");
}
