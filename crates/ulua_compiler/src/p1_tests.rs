//! Rule tests construct incompatible types and source forms directly.
use super::*;
fn accept(source: &str) -> Compiled {
    compile(source.as_bytes(), Limits::default()).unwrap()
}
fn reject(source: &str, code: &str) {
    let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
    assert_eq!(error.code, code, "{source}: {}", error.message);
}
fn body(text: &str) -> String {
    format!("export function main(): void {{ {text} }}")
}

#[test]
fn c1_each_p1_value_flow_checks_independent_types() {
    for s in [
        body("const n: number = true;"),
        body("let n = 1; n = false;"),
        "function f(): number { return true; } export function main(): void {}".into(),
        "function f(n: number): void {} export function main(): void { f(true); }".into(),
        "function f(n: number = true): void {} export function main(): void {}".into(),
        body("const x = true ? 1 : false;"),
        "function f(): number { if (true) { return 1; } } export function main(): void {}".into(),
        "function f(x: number): void {} export function main(): void { f(); }".into(),
        "function f(): void {} export function main(): void { f(1); }".into(),
    ] {
        reject(&s, "C1");
    }
}
#[test]
fn c2_defaults_void_and_undefined() {
    for s in [
        body("print(`${undefined}`);"),
        body("let undefined = 1;"),
        body("const x: void = 1;"),
        "function f(): void {} export function main(): void { const x = f(); }".into(),
        "function f(x?: number): void {} export function main(): void {}".into(),
        "function f(x: number | null = 1): void {} export function main(): void {}".into(),
        "function f(x: (number | (null)) = 1): void {} export function main(): void {}".into(),
    ] {
        reject(&s, "C2");
    }
    accept(
        "function f(x: boolean = false, y: string = 's'): string { return y; } export function main(): void { print(f()); }",
    );
    // A default before a required parameter does not reduce the required argument count.
    reject(
        "function f(x: number = 1, y: number): void {} export function main(): void { f(1); }",
        "C1",
    );
}
#[test]
fn c3_each_boolean_context() {
    for s in [
        "while (1) {}",
        "for (; 1;) {}",
        "const x = 1 ? true : false;",
        "const x = !1;",
        "const x = true && 1;",
        "const x = 1 || false;",
    ] {
        reject(&body(s), "C3");
    }
}
#[test]
fn c4_numbers_and_template_types() {
    for s in [
        "const x = -'x';",
        "const x = true * 1;",
        "const x = 'x' + 1;",
        "let x = 'x'; x += 1;",
        "const x = ~1;",
        "const x = 1 << 2;",
        "const x: bigint = 1;",
        "const x = Math.pow(1, NaN);",
    ] {
        reject(&body(s), "C4");
    }
    reject(
        "function f(): number { return 1; } export function main(): void { print(`${f}`); }",
        "C4",
    );
    reject(&body("print(String(true));"), "C1");
    let c = accept(&body("print(`${-0} ${NaN} ${Infinity} ${1e-7}`);"));
    assert!(c.luau.contains("e([+-])0*(%d)"));
    assert!(c.luau.contains("__ulua_format("));
}
#[test]
fn c7_equal_types_and_function_identity() {
    reject(&body("const x = 1 === true;"), "C7");
    reject(
        "function f(): void {} export function main(): void { const x = f === f; }",
        "C7",
    );
    reject(&body("const x = 1 != 1;"), "C7");
}
#[test]
fn c8_each_unimplemented_control_and_expression_form() {
    for s in [
        "const x: unknown = 1;",
        "const x: object = 1;",
        "const x: never = 1;",
        "print(void 1);",
        "print(`${1 in 2}`);",
        "print(`${delete x}`);",
        "print(`${(1, 2)}`);",
        "print`x`;",
        "do {} while (false);",
        "for (const x in y) {}",
        "label: while (true) { break label; }",
    ] {
        reject(&body(s), "C8");
    }
}
#[test]
fn c10_direct_block_default_and_transitive_references() {
    for s in [
        body("const x = x;"), body("print(`${x}`); const x = 1;"),
        body("const x = 1; { print(`${x}`); const x = 2; }"),
        "function f(x: number = x): void {} export function main(): void {}".into(),
        "function f(x: number = y, y: number = 1): void {} export function main(): void {}".into(),
        "const x: number = 1; function f(): number { return g(); } const b = 2; function g(): number { return b; } export function main(): void {}".into(),
    ] { reject(&s, "C10"); }
    // The chain includes a cycle and reaches a late module binding.
    reject(
        "const x: number = f(); function f(): number { return g(); } function g(): number { return true ? b : f(); } const b = 2; export function main(): void {}",
        "C10",
    );
    accept(
        "const b = 2; function f(): number { return g(); } function g(): number { return b; } export function main(): void { print(`${f()}`); }",
    );
}
#[test]
fn c11_closed_surface_positions_and_targets() {
    for s in [
        "let x = 1; const y = ++x;",
        "const x = 1; x = 2;",
        "print = 1;",
        "String = 1;",
        "NaN++;",
        "const x = Math;",
        "const x = print;",
        "1 + 2;",
        "for (const x = 1; false;) {}",
        "for (let x = 0; false; print('x')) {}",
        "const x = /x/;",
        "const x = function(): number { return 1; };",
        "const x = (1 as number);",
        "const x: 1 = 1;",
        "const x: [number] = [1];",
        "const x: number & string = 1;",
        "const { x } = 1;",
    ] {
        reject(&body(s), "C11");
    }
    for s in ["function f() {}", "function f(x) {}"] {
        reject(&format!("{s} export function main(): void {{}}"), "C11");
    }
}
#[test]
fn later_phase_forms_stay_unsupported() {
    for s in [
        "const x = null;",
        "const x = [];",
        "const x = {};",
        "const x = () => 1;",
        "function f(): void {}",
        "print('x' + 'y');",
        "let s = 'x'; s += 'y';",
        "print('x'.includes('x'));",
        "const x: number[] = [];",
        "const x: (() => number) = f;",
        "const x: number | null = 1;",
    ] {
        reject(&body(s), "unsupported");
    }
}
#[test]
fn c14_decoded_numbers_templates_and_generated_names() {
    let c = accept(&body(
        "const math = 7; const tostring = 8; print(`${0o17} ${0b101} ${1_000} ${.5} ${5.} ${0x10} ${1e400} ${math % 3}`); print(\"a\\\nb\"); print(`a\r\nb\rc`);",
    ));
    assert!(!c.luau.contains("0o17"));
    assert!(!c.luau.contains("0b101"));
    assert!(!c.luau.contains("1_000"));
    assert!(c.luau.contains("\"ab\""));
    assert!(c.luau.contains("\"a\\010b\\010c\""));
    for escape in [r"\uD800", r"\u{D800}"] {
        reject(&body(&format!("print(`{escape}`);")), "C5");
    }
    assert_eq!(
        emitter::quote("\0\r\n\"\\é9"),
        r#""\000\013\010\034\092\195\1699""#
    );
}
#[test]
fn export_signatures_follow_checked_types() {
    let c = accept(
        "export function f(n: number, b: boolean): number { return n; } export function main(): void {}",
    );
    assert_eq!(c.exports.len(), 2);
    assert_eq!(c.exports[0].parameters, [1, 2]);
    assert_eq!(c.exports[0].result, 1);
}
#[test]
fn c15_small_functions_avoid_spill_and_nested_bindings_share_one_table() {
    let small = accept(&body("let x = 1; x++;"));
    assert!(!small.luau.contains("__ulua_spill"));
    let mut text = String::new();
    for i in 0..220 {
        text.push_str(&format!("{{ let x{i} = {i}; x{i}++; }}"));
    }
    let large = accept(&body(&text));
    assert_eq!(large.luau.matches("local __ulua_spill").count(), 1);
    assert!(large.luau.contains("__ulua_spill1.__ulua_b"));
}
#[test]
fn depth_checks_cover_defaults_and_assignment_operands() {
    let chain = "1+".repeat(160) + "1";
    for s in [
        format!("function f(x: number = {chain}): void {{}} export function main(): void {{}}"),
        body(&format!("let x = 0; x = {chain};")),
    ] {
        reject(&s, "limit:nesting-depth");
    }
}

#[test]
fn c9_loop_copies_cover_continue_before_update() {
    let c = accept(&body(
        "for (let i = 0; i < 5; i++) { i++; if (i === 1) { continue; } }",
    ));
    let copy_back = c.luau.find("__ulua_b1 = (__ulua_g1_1) :: number").unwrap();
    let update = c.luau.rfind("__ulua_b1 = (").unwrap();
    assert!(copy_back < update);
    assert!(c.luau.contains("repeat\n"));
    assert!(c.luau.contains("until true"));
}

#[test]
fn c12_binary_call_and_compound_reads_precede_a_mutating_call() {
    for effect in ["print(`${n + inc()}`);", "pair(n, inc());", "n += inc();"] {
        let c = accept(&format!(
            "let n = 0; function inc(): number {{ n++; return n; }} function pair(a: number, b: number): void {{}} export function main(): void {{ {effect} }}"
        ));
        let main = c.luau.split("__ulua_b3 = (function").nth(1).unwrap();
        let read = main.find("= (__ulua_b0) :: number\n").unwrap();
        let call = main.find("= (__ulua_b1) :: ").unwrap();
        assert!(read < call, "{effect}");
    }
}

#[test]
fn c10_a_multi_declarator_statement_finishes_before_any_reference() {
    reject(&body("let a = 1, b = a;"), "C10");
    reject(
        "function f(x: number): void { let x = 1; } export function main(): void {}",
        "C11",
    );
}

#[test]
fn non_void_results_allow_loops_without_a_normal_exit() {
    for loop_body in [
        "while (true) { return 1; }",
        "for (;;) { continue; }",
        "while (true) { while (true) { break; } }",
    ] {
        accept(&format!(
            "function f(): number {{ {loop_body} }} export function main(): void {{}}"
        ));
    }
    reject(
        "function f(): number { while (true) { break; } } export function main(): void {}",
        "C1",
    );
}

#[test]
fn c8_module_interface_and_generic_declarations() {
    reject(
        "interface X { x: number; } export function main(): void {}",
        "C8",
    );
    reject(
        "function f<T>(): void {} export function main(): void {}",
        "C8",
    );
}

#[test]
fn joins_skip_dead_declarations_and_coalesce_nested_writes() {
    let source = format!(
        "export function main():void{{let gate=true;let x=0;{}{{let dead=0;x=1;}}x=x+1;}}",
        "if(gate)".repeat(40)
    );
    let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
    assert_eq!(
        compiled
            .luau
            .matches("__ulua_b2 = (__ulua_b2) :: number")
            .count(),
        1
    );
    assert!(!compiled.luau.contains("__ulua_b3 = (__ulua_b3)"));
    let overwrite = compile(
        b"export function main():void{let x=0;if(true){x=1;}x=2;print(String(x));}",
        Limits::default(),
    )
    .unwrap();
    assert!(!overwrite.luau.contains("__ulua_b1 = (__ulua_b1)"));
}

#[test]
fn demanded_join_precedes_the_next_branch() {
    let compiled = compile(
        b"export function main():void{let x=0;let gate=true;if(gate){x=1;}if(gate){x=x+1;}}",
        Limits::default(),
    )
    .unwrap();
    let reset = compiled
        .luau
        .find("__ulua_b1 = (__ulua_b1) :: number")
        .unwrap();
    let last_branch = compiled.luau.rfind("if __ulua_t").unwrap();
    assert!(reset < last_branch);
}
