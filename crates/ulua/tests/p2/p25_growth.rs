//! Condition wrappers reach the exact nesting bound.
#[allow(dead_code)]
#[path = "../support/p25_shapes.rs"]
mod shapes;
use shapes::{condition_depth, condition_source};
use ulua_compiler::{Limits, compile};

#[test]
fn condition_wrappers_reach_the_nesting_bound() {
    for shape in [
        "conditional-tests",
        "alternating-logical-tests",
        "conditional-wrapped-tests",
        "logical-wrapped-tests",
        "nullable-conditional-tests",
    ] {
        let depth = condition_depth(shape);
        compile(condition_source(shape, depth).as_bytes(), Limits::default()).unwrap();
        assert_eq!(
            compile(
                condition_source(shape, depth + 1).as_bytes(),
                Limits::default()
            )
            .unwrap_err()
            .code,
            "limit:nesting-depth",
            "{shape}"
        );
    }
}
