//! State ratios use independent counts. Timing is a report, never an assertion.
//! Measured Debug target cost: 0.16 seconds.
//! Head diagnostic tests compile 37 small inputs without external witnesses and report their cost.
use crate::{checker, emitter, ir, p27_shapes as shapes, *};
use std::time::Instant;
#[test]
fn protected_growth_is_linear() {
    let start = Instant::now();
    for shape in shapes::SHAPES {
        let n = if shape == "nested" { 5 } else { 128 };
        let mut values = vec![];
        for count in [n, 4 * n] {
            let source = shapes::source(shape, count);
            let allocator = Allocator::default();
            let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
            assert!(parsed.diagnostics.is_empty());
            let module = checker::check(&parsed.program, source.as_bytes(), DEPTH_CEILING).unwrap();
            let state = module.types.identities().count()
                + module.bindings.len()
                + module.functions.len()
                + ir::nodes(&module);
            let output = emitter::emit(&module, OUTPUT_FACTOR).unwrap().len();
            values.push((state, output));
            eprintln!(
                "P2.7 state {shape} n={count} source={} state={state} output={output}",
                source.len()
            );
        }
        assert!(values[1].0 <= 6 * values[0].0);
        assert!(values[1].1 <= 6 * values[0].1);
        eprintln!(
            "P2.7 growth {shape} state_ratio={:.3} output_ratio={:.3}",
            values[1].0 as f64 / values[0].0 as f64,
            values[1].1 as f64 / values[0].1 as f64
        );
    }
    eprintln!("P2.7 state cost={:.3}s", start.elapsed().as_secs_f64());
}

fn head_diagnostic(body: &str, code: &str) {
    let source = format!(
        "function f(): number {{ {body} return 0; }}\nexport function main(): void {{ print(`${{f()}}`); }}"
    );
    let diagnostic = compile(source.as_bytes(), Limits::default()).unwrap_err();
    let offset = source.find("(): number =>").unwrap();
    assert_eq!(diagnostic.code, code, "{source}: {diagnostic:?}");
    assert_eq!(diagnostic.offset, offset as u64, "{source}");
    assert_eq!((diagnostic.line, diagnostic.column), (1, offset as u32 + 1));
    assert_eq!(
        diagnostic.message,
        if code == "C9" {
            "Closure in a for head"
        } else {
            checker::UNREACHABLE_ARROW
        }
    );
}

#[test]
fn for_head_outer_arrow_precedes_nested_unreachable_arrow() {
    let start = Instant::now();
    for expression in [
        "((): number => { return 1; const h = (): number => 2; })()",
        "((): number => { throw new Error(); const h = (): number => 2; })()",
        "((): number => { return ((): number => { return 1; const h = (): number => 2; })(); })()",
        "((): number => 1)()",
    ] {
        for head in [
            format!("let i = {expression}; i < 2; i++"),
            format!("let i = 0; {expression} < 2; i++"),
            format!("let i = 0; i < 2; i = {expression}"),
        ] {
            head_diagnostic(&format!("for ({head}) {{ }}"), "C9");
        }
    }
    eprintln!(
        "P2.7 head outer arrows cases=12 cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn for_head_c16_wins_at_the_first_arrow() {
    let start = Instant::now();
    let arrow = "((): number => { return 1; const h = (): number => 2; })()";
    for body in [
        format!("return 0; for (let i = {arrow}; i < 2; i++) {{ }}"),
        format!("return 0; for (let i = 0; {arrow} < 2; i++) {{ }}"),
        format!("for (let i = 0; i < 2; i = {arrow}) {{ break; }}"),
        format!("for (let i = 0; i < 2; i = {arrow}) {{ return 0; }}"),
        "for (let i = 0; false && ((): number => 1)() < 2; i++) { }".into(),
        "for (let i = 0; true || ((): number => 1)() < 2; i++) { }".into(),
    ] {
        head_diagnostic(&body, "C16");
    }
    for expression in [
        "false ? ((): number => 1)() : 0",
        "true ? 0 : ((): number => 1)()",
        "false ? ((): number => 1)() : ((): number => 2)()",
    ] {
        for head in [
            format!("let i = {expression}; i < 2; i++"),
            format!("let i = 0; ({expression}) < 2; i++"),
            format!("let i = 0; i < 2; i = {expression}"),
        ] {
            head_diagnostic(&format!("for ({head}) {{ }}"), "C16");
        }
    }
    head_diagnostic(
        "for (let i = false ? ((): number => 1)() : 0, j = ((): number => 2)(); i < 2; i++) { }",
        "C16",
    );
    eprintln!(
        "P2.7 head first C16 cases=16 cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn for_head_arrows_follow_source_order_across_siblings() {
    let start = Instant::now();
    for expression in [
        "((): number => 1)() + (false ? ((): number => 2)() : 0)",
        "((): number => 1)() + ((): number => { return 1; const h = (): number => 2; })()",
        "[((): number => 1)(), ((): number => 2)()].length",
    ] {
        for head in [
            format!("let i = {expression}; i < 2; i++"),
            format!("let i = 0; {expression} < 2; i++"),
            format!("let i = 0; i < 2; i = {expression}"),
        ] {
            head_diagnostic(&format!("for ({head}) {{ }}"), "C9");
        }
    }
    eprintln!(
        "P2.7 head sibling arrows cases=9 cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}
