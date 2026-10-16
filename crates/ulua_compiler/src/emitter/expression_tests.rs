//! Each fixture costs one compiler call; no native VM or external process.
use crate::{Limits, compile};

#[test]
fn literals_and_local_operations_target_bindings() {
    let code = compile(b"export function main(n:number):number{let x=1;let b=true;x=n+2;if(n<3){x=4;}while(x<5){x++;}for(let i=0;i<1;i++){}return x;}", Limits::default()).unwrap().luau;
    assert!(code.contains("__ulua_b2 = (1.0) :: number"), "{code}");
    assert!(code.contains("__ulua_b3 = (true) :: boolean"), "{code}");
    assert!(
        code.contains("__ulua_b2 = ((__ulua_b1 + 2.0)) :: number"),
        "{code}"
    );
    assert!(code.contains("if (__ulua_b1 < 3.0) then"), "{code}");
    assert!(
        code.contains("if not (__ulua_b2 < 5.0) then break end"),
        "{code}"
    );
    assert!(
        code.contains("for __ulua_g1_1 = 0.0, __ulua_g1_0, 1.0 do"),
        "{code}"
    );
}

#[test]
fn call_operands_inline_stable_reads_and_literals() {
    let code = compile(b"function f(a:number,b:boolean):number{return a;} export function main(n:number):number{return f(n,true);}", Limits::default()).unwrap().luau;
    assert!(code.contains("__ulua_b0(__ulua_b4, true)"), "{code}");
}

#[test]
fn shared_reads_without_later_calls_stay_inline() {
    let code = compile(
        b"let n=1; export function main():number{let x=2;const read=()=>x;return n+x;}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(code.contains("(__ulua_b0 + __ulua_b2)"), "{code}");
}

#[test]
fn captured_read_before_call_keeps_its_snapshot() {
    let code = compile(b"export function main():number{let n=1;const inc=():number=>{n++;return n;};return n+inc();}", Limits::default()).unwrap().luau;
    let snapshot = code.find("= (__ulua_b1) :: number").unwrap();
    let call = code.find("= (__ulua_b2()) :: number").unwrap();
    assert!(snapshot < call, "{code}");
    assert!(!code.contains("(__ulua_b1 +"), "{code}");
}

#[test]
fn module_read_and_callee_before_call_keep_snapshots() {
    let code = compile(b"let n=1;function inc():number{n++;return n;}function f(a:number,b:number):number{return a+b;}export function main():number{return f(n,inc());}", Limits::default()).unwrap().luau;
    let snapshot = code.rfind("= (__ulua_access0(false").unwrap();
    let callee = code.find("= (__ulua_b2) :: ").unwrap();
    let call = code.find("= (__ulua_b1()) :: number").unwrap();
    assert!(snapshot < call && callee < snapshot, "{code}");
}

#[test]
fn uncaptured_read_before_call_stays_inline() {
    let code = compile(
        b"function f():number{return 2;}export function main(n:number):number{return n+f();}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(code.contains("(__ulua_b2 + __ulua_t2_"), "{code}");
    assert!(!code.contains("= (__ulua_b2) :: number"), "{code}");
}

#[test]
fn four_node_operator_tree_keeps_a_temporary() {
    let code = compile(
        b"export function main(n:number):number{let x=-(n+1);return x;}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(code.contains("= ((__ulua_b1 + 1.0)) :: number"), "{code}");
    assert!(code.contains("__ulua_b2 = (- (__ulua_t1_"), "{code}");
    assert!(!code.contains("- ((__ulua_b1 + 1.0))"), "{code}");
}

#[test]
fn compound_assignment_freezes_the_old_value() {
    let code = compile(b"let n=1;function inc():number{n=10;return 2;}export function main():number{n+=inc();return n;}", Limits::default()).unwrap().luau;
    assert!(
        code.find("= (__ulua_access0(false").unwrap()
            < code.find("= (__ulua_b1()) :: number").unwrap(),
        "{code}"
    );
    assert!(!code.contains("(__ulua_b0 +"), "{code}");
}

#[test]
fn spilled_reads_keep_temporaries() {
    let declarations = (0..200)
        .map(|i| format!("let v{i}={i};"))
        .collect::<String>();
    let code = compile(
        format!("export function main():number{{{declarations}return v199+1;}}").as_bytes(),
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(
        code.contains("= (__ulua_spill1.__ulua_b200) :: number"),
        "{code}"
    );
    assert!(!code.contains("(__ulua_spill1.__ulua_b200 +"), "{code}");
}

#[test]
fn module_read_before_call_keeps_a_plain_local_snapshot() {
    let code = compile(
        b"const n=1;function f():number{return 2;}export function main():number{return n+f();}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    let snapshot = code.find("= (__ulua_b0) :: number").unwrap();
    let call = code.find("= (__ulua_b1()) :: number").unwrap();
    assert!(snapshot < call, "{code}");
}

#[test]
fn closure_construction_targets_the_binding() {
    let code = compile(
        b"export function main():void{const n=1;while(true){const g=()=>n;}}",
        Limits::default(),
    )
    .unwrap()
    .luau;
    assert!(code.contains("__ulua_b2 = (function"), "{code}");
    assert!(!code.contains("__ulua_t1_0 = (function"), "{code}");
}
