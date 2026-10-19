//! IR witnesses cover bindings that the surface syntax cannot type as arrays.
use super::candidates;
use crate::ir::{Expr, ExprKind as E, IndexForm, Statement as S, Type};

fn expr(kind: E) -> Expr {
    Expr {
        flow: None,
        bare_true: false,
        index_form: IndexForm::Checked,
        start: 0,
        ty: Type::Number,
        kind,
    }
}
fn read() -> S {
    S::Eval(expr(E::Index(
        Box::new(expr(E::Read(0))),
        Box::new(expr(E::Number(0.0))),
    )))
}
#[test]
fn length_try_catch_binding_write() {
    // A typed source catch is Error. The query must also record this IR binding write.
    let body = S::Block(vec![
        read(),
        S::Try(
            Box::new(S::Block(vec![])),
            Some((Some(0), Box::new(S::Block(vec![])))),
            None,
        ),
    ]);
    assert!(candidates(&body).is_empty());
}
#[test]
fn located_wrappers_are_traversed() {
    let body = S::Located(0, Box::new(S::Located(0, Box::new(read()))));
    assert!(candidates(&body).contains_key(&0));
}
