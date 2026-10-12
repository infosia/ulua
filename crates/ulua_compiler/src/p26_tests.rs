//! P2.6 growth asserts compiler state and output ratios, never elapsed-time ratios.
//! Review round 1 Debug cost: 0.323 seconds; three unit tests, including ten growth modules.
//! Ten checked modules and ten emitted modules cost one measured test invocation.
use crate::{checker, emitter, p26_shapes as shapes, *};
use std::time::Instant;

#[test]
fn class_state_and_output_growth() {
    let start = Instant::now();
    for shape in shapes::SHAPES {
        let mut measurements = vec![];
        for bytes in [8192, 32768] {
            let source = shapes::source(shape, bytes);
            let allocator = Allocator::default();
            let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
            assert!(parsed.diagnostics.is_empty(), "{shape}");
            let module = checker::check(&parsed.program, source.as_bytes(), DEPTH_CEILING).unwrap();
            let types = module.types.identities().count();
            let bindings = module.bindings.len();
            let functions = module.functions.len();
            let nodes = ir::nodes(&module);
            let output = emitter::emit(&module, OUTPUT_FACTOR).unwrap().len();
            // Immutable type ids, checked bindings, function identities, and emitted bytes are separate state measures.
            let state = types + bindings + functions + nodes;
            measurements.push((state, output));
            eprintln!(
                "P2.6 state {shape} bytes={bytes} types={types} bindings={bindings} functions={functions} nodes={nodes} state={state} output={output}"
            );
        }
        let a = measurements[0];
        let b = measurements[1];
        let state_ratio = b.0 as f64 / a.0 as f64;
        let output_ratio = b.1 as f64 / a.1 as f64;
        assert!(b.0 <= 6 * a.0, "{shape} state {state_ratio}");
        assert!(b.1 <= 6 * a.1, "{shape} output {output_ratio}");
        eprintln!(
            "P2.6 growth {shape} state_ratio={state_ratio:.3} output_ratio={output_ratio:.3}"
        );
    }
    eprintln!(
        "P2.6 compiler growth cost={:.3}s",
        start.elapsed().as_secs_f64()
    );
}

#[test]
fn class_identity_and_source_type_links() {
    let mut types = ir::Types::new();
    let a = types.reserve_class();
    let b = types.reserve_class();
    let record = types.reserve_record();
    types.define_class(a, vec![(0, ir::Type::Number)], vec![]);
    types.define_class(b, vec![(0, ir::Type::Number)], vec![]);
    types.define_record(record, vec![(0, ir::Type::Number)]);
    types.seal_aliases();
    for other in [b, record] {
        assert!(!types.assignable(a, other));
        assert!(!types.assignable(other, a));
        let aa = types.array(a);
        let bb = types.array(other);
        assert!(!types.equal(aa, bb));
    }
    let mut types = ir::Types::new();
    let classes: Vec<_> = (0..49).map(|_| types.reserve_class()).collect();
    let mut next = ir::Type::Number;
    for ty in classes.iter().rev() {
        types.define_class(*ty, vec![(0, next)], vec![]);
        next = *ty;
    }
    types.seal_aliases();
    assert_eq!(types.depth(classes[0]), 49);
    assert_eq!(types.depth(classes[1]), 48);
}

#[test]
fn class_initializer_depth_has_a_limit_diagnostic() {
    for depth in 0..56 {
        for expression in [
            format!("{}0{}", "(".repeat(depth), ")".repeat(depth)),
            format!("{}0", "- ".repeat(depth)),
        ] {
            let source =
                format!("class C{{v:number={expression};}}export function main():void{{new C();}}");
            if let Err(d) = compile(source.as_bytes(), Limits::default()) {
                assert_eq!(d.code, "limit:nesting-depth", "depth={depth}: {d:?}");
            }
        }
    }
}
