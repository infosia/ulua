//! P6.S4 fixtures check emitted forms against checked operand types.
use crate::{Limits, compile};

// Cost: twelve small compiles, with no VM or external process.
#[test]
fn primitive_equalities_use_native_operators() {
    for ty in ["number", "string", "boolean"] {
        for (op, native) in [
            ("===", " == "),
            ("==", " == "),
            ("!==", " ~= "),
            ("!=", " ~= "),
        ] {
            let source = format!("export function main(a:{ty},b:{ty}):boolean{{return a{op}b;}}");
            let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
            assert!(code.contains(native), "{code}");
            assert_eq!(code.matches("__ulua_equal(").count(), 1, "{code}");
            assert_eq!(code.matches("__ulua_unequal(").count(), 1, "{code}");
        }
    }
}

// Cost: eight small compiles. The same operands keep reference identity helpers.
#[test]
fn reference_equalities_keep_helpers() {
    for ty in ["number[]", "R", "Box", "Error"] {
        for (op, helper) in [("===", "__ulua_equal("), ("!==", "__ulua_unequal(")] {
            let source = format!(
                "type R={{n:number}};class Box{{n:number;constructor(n:number){{this.n=n;}}}}function compare(a:{ty},b:{ty}):boolean{{return a{op}b;}}export function main():void{{}}"
            );
            let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
            assert_eq!(code.matches(helper).count(), 2, "{code}");
        }
    }
}

// Cost: two small compiles. A narrowed null test still has the checked C2 meaning.
#[test]
fn nullable_and_narrowed_null_tests_use_native_operators() {
    for test in ["a===null", "null===a"] {
        let source = format!(
            "type R={{n:number}};function compare(a:R|null):boolean{{if(a!==null){{return {test};}}return true;}}export function main():void{{}}"
        );
        let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
        assert!(
            code.contains(" == nil") || code.contains("nil == "),
            "{code}"
        );
        assert_eq!(code.matches("__ulua_equal(").count(), 1, "{code}");
        assert_eq!(code.matches("__ulua_unequal(").count(), 1, "{code}");
    }
}

// Cost: one small compile. Ordinary reads and writes retain no index helper call.
#[test]
fn element_checks_are_inline_and_keep_the_rhs_before_the_write_check() {
    let source = b"function rhs():number{return 3;}function work(a:number[],i:number):number{a[i]=rhs();return a[i];}export function main():void{}";
    let code = compile(source, Limits::default()).unwrap().luau;
    assert!(!code.contains("__ulua_get("), "{code}");
    assert!(!code.contains("__ulua_set("), "{code}");
    let check = code.find(": boolean = not (").unwrap();
    assert!(code[..check].contains("__ulua_b0()"), "{code}");
    assert!(code[check..].contains(" <= #"), "{code}");
    assert!(code[check..].contains(" and __ulua_floor("), "{code}");
}

// Cost: one small compile. A loop back-edge does not pay the full capture cost by itself.
#[test]
fn costly_first_closure_keeps_a_point_after_creation() {
    let mut source = String::from("function work():number{");
    for index in 0..10 {
        source.push_str(&format!("const v{index}={index};"));
    }
    source.push_str("while(true){const g=():number=>{let s=0;");
    for index in 0..10 {
        source.push_str(&format!("s+=v{index};"));
    }
    source.push_str("return s;};}return 0;}export function main():void{}");
    let code = compile(source.as_bytes(), Limits::default()).unwrap().luau;
    let head = code.find("while true do\n").unwrap();
    assert!(code[..head].ends_with("__ulua_interrupt()\n    "), "{code}");
    let body = &code[head..];
    let closure_end = body.find("end) :: ").unwrap();
    let next = body[closure_end..].split_once('\n').unwrap().1;
    assert!(next.starts_with("    __ulua_interrupt()\n"), "{code}");
}
