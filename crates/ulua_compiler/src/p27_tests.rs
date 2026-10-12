//! State ratios use independent counts. Timing is a report, never an assertion.
//! Measured Debug target cost: 0.16 seconds.
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
