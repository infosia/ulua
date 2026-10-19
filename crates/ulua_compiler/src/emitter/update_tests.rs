//! P6.S14: fixed source fixtures, no VM or timing dependency.
use crate::{Limits, compile};

#[test]
fn primitive_update_runs_use_two_instead_of_eight() {
    let mut failures = vec![];
    for (decl, store) in [
        ("const a={x:0};", "a.x=1;"),
        ("const a={x:false};", "a.x=true;"),
        ("const a={x:0};", "a.x+=1;"),
        ("class A{x:number=0;}const a=new A();", "a.x=1;"),
        ("class A{x:boolean=false;}const a=new A();", "a.x=true;"),
    ] {
        let source = format!("{decl}export function main():void{{{}}}", store.repeat(32));
        let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
        let body = code
            .split("function(): ()")
            .last()
            .unwrap()
            .split("end) ::")
            .next()
            .unwrap();
        let points = body.matches("__ulua_interrupt()").count();
        eprintln!("update {store} points={points}");
        // Each store has one RHS temporary and costs two: five fit per interval.
        if points != if store.contains("+=") { 10 } else { 6 } {
            failures.push((decl, store, points));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

fn store_runs(source: &str) -> Vec<usize> {
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    let mut runs = vec![];
    let mut count = 0;
    for line in code.lines() {
        if line.trim() == "__ulua_interrupt()" || line.contains("end) ::") {
            if count > 0 {
                runs.push(count);
            }
            count = 0;
        }
        if line.contains(".__ulua_k") && line.contains(" = (") && !line.contains("local ") {
            let target = line.split_once(" = ").unwrap().0;
            if target.contains(".__ulua_k") {
                count += 1;
            }
        }
    }
    runs
}

#[test]
fn construction_and_nonprimitive_fields_keep_eight() {
    for (ty, initial, value) in [
        ("number|null", "null", "1"),
        ("string", "'a'", "'b'"),
        ("number[]", "[]", "[]"),
    ] {
        let source = format!(
            "type R={{x:{ty}}};const a:R={{x:{initial}}};export function main():void{{{}}}",
            format!("a.x={value};").repeat(32)
        );
        let runs = store_runs(&source);
        assert_eq!(runs.iter().max(), Some(&1), "{ty}: {runs:?}");
    }
}

#[test]
fn this_in_constructor_keeps_eight() {
    let source = format!(
        "class A{{x:number=0;constructor(){{{}}}}}export function main():void{{const a=new A();}}",
        "this.x=1;".repeat(32)
    );
    let runs = store_runs(&source);
    assert_eq!(runs.iter().max(), Some(&1), "{runs:?}\n{source}");
}

#[test]
fn this_in_method_is_an_update() {
    let source = format!(
        "class A{{x:number=0;run():void{{{}}}}}export function main():void{{const a=new A();a.run();}}",
        "this.x=1;".repeat(32)
    );
    assert_eq!(store_runs(&source).iter().max(), Some(&5));
}

#[test]
fn other_receivers_in_constructor_remain_updates() {
    let source = format!(
        "type R={{x:number}};class A{{x:number=0;constructor(a:R){{{}}}}}export function main():void{{const a=new A({{x:0}});}}",
        "a.x=1;".repeat(32)
    );
    assert_eq!(store_runs(&source).iter().max(), Some(&5));
}

#[test]
fn particles_update_loop_has_one_local_point_and_a_point_before_the_head() {
    let code = compile(
        include_bytes!("../../../ulua_bench/workloads/ulua/particles.ts"),
        Limits::default(),
    )
    .unwrap()
    .luau;
    let receiver = code.find("__ulua_b9 = (").unwrap();
    let head = code[..receiver].rfind("    for ").unwrap();
    assert!(code[..head].ends_with("    __ulua_interrupt()\n"));
    let end = code[receiver..].find("__ulua_b9.__ulua_k1 = ").unwrap() + receiver;
    assert_eq!(code[head..end].matches("__ulua_interrupt()").count(), 1);
}

#[test]
fn object_literal_fields_keep_eight() {
    let fields = (0..32).map(|i| format!("x{i}:{i},")).collect::<String>();
    let source = format!("export function main():void{{const a={{{fields}}};}}");
    assert_eq!(store_runs(&source).iter().max(), Some(&1));
}

#[test]
fn field_initializers_keep_eight() {
    let fields = (0..32)
        .map(|i| format!("x{i}:number={i};"))
        .collect::<String>();
    let source = format!("class A{{{fields}}}export function main():void{{const a=new A();}}");
    assert_eq!(store_runs(&source).iter().max(), Some(&1));
}

#[test]
fn block_headers_move_a_point_before_an_active_loop() {
    let source = format!(
        "export function main(b:boolean):void{{let x=0;const n=b?0:2;{}for(let i=0;i<n;i++){{let c0=x;let c1=x;const f=():number=>c0+c1;x=x+f();}}{}}}",
        "x=x+1;".repeat(12),
        "x=x+1;".repeat(20)
    );
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    let head = code.find("    for __ulua_g").unwrap();
    assert!(code[..head].ends_with("    __ulua_interrupt()\n"), "{code}");
}

#[test]
fn remainder_update_keeps_the_operator_cost() {
    let source = format!(
        "const a={{x:7}};export function main():void{{{}}}",
        "a.x%=3;".repeat(32)
    );
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    let body = code
        .split("function(): ()")
        .last()
        .unwrap()
        .split("end) ::")
        .next()
        .unwrap();
    let points = body.matches("__ulua_interrupt()").count();
    eprintln!("remainder update points={points}");
    assert_eq!(points, 12, "{code}");
}

#[test]
fn spilled_binding_receiver_is_loaded_but_keeps_update_weight() {
    let declarations = (0..11)
        .map(|i| format!("const a{i}={{x:0}};"))
        .collect::<String>();
    let captures = (0..11)
        .map(|i| format!("a{i}.x"))
        .collect::<Vec<_>>()
        .join("+");
    let source = format!(
        "export function main():void{{{declarations}const capture=():number=>{captures};capture();{}}}",
        "a10.x=1;".repeat(32)
    );
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    let body = code
        .rsplit_once("= (__ulua_b12())")
        .expect(&code)
        .1
        .split("end) ::")
        .next()
        .unwrap();
    let loads = body
        .lines()
        .filter(|l| l.contains("= (__ulua_spill1.__ulua_b11) ::"))
        .count();
    assert_eq!(loads, 32, "{code}");
    let points = body.matches("__ulua_interrupt()").count();
    eprintln!("spilled receiver points={points}, loads={loads}");
    assert_eq!(points, 10, "{body}");
}
