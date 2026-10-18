//! Shared structural eligibility for numeric loop lowering and counter facts.
use crate::ir::{self, Expr, ExprKind as E, Statement as S};

pub(crate) struct Counted<'a> {
    pub(crate) id: usize,
    pub(crate) start: f64,
    pub(crate) step: f64,
    pub(crate) bound: &'a Expr,
    pub(crate) round: &'static str,
    pub(crate) adjustment: &'static str,
}

pub(crate) fn literal(expr: &Expr) -> Option<f64> {
    match &expr.kind {
        E::Number(n) => Some(*n),
        E::Unary("-", n) => match n.kind {
            E::Number(n) => Some(-n),
            _ => None,
        },
        _ => None,
    }
}

// Assignments exist only as statements. Captures exclude all indirect writes through calls.
fn writes(body: &S, id: usize) -> bool {
    let mut work = vec![body];
    while let Some(s) = work.pop() {
        match s.form() {
            S::Assign(target, _, _) if *target == id => return true,
            S::Block(b) => work.extend(b),
            S::If(_, a, b) => {
                work.push(a);
                work.extend(b.iter().map(|s| s.as_ref()));
            }
            S::While(_, b) | S::ForOf(_, _, b) => work.push(b),
            S::For(init, _, update, b, _) => {
                work.extend(init);
                work.extend(update.iter().map(|s| s.as_ref()));
                work.push(b);
            }
            S::Try(a, b, f) => {
                work.push(a);
                work.extend(b.iter().map(|(_, s)| s.as_ref()));
                work.extend(f.iter().map(|s| s.as_ref()));
            }
            _ => {}
        }
    }
    false
}

pub(crate) fn select<'e>(
    captured: &[bool],
    init: &[S],
    test: Option<&'e Expr>,
    update: Option<&S>,
    body: &S,
) -> Option<Counted<'e>> {
    let [init] = init else { return None };
    let S::Declare(id, start) = init.form() else {
        return None;
    };
    let start = literal(start)?;
    if !start.is_finite() || start.fract() != 0.0 || captured[*id] {
        return None;
    }
    let S::Assign(target, Some(op @ ("+" | "-")), amount) = update?.form() else {
        return None;
    };
    let E::Number(amount) = amount.kind else {
        return None;
    };
    if target != id || !amount.is_finite() || amount <= 0.0 || amount.fract() != 0.0 {
        return None;
    }
    let step = if *op == "+" { amount } else { -amount };
    let E::Binary(test, counter, bound) = &test?.kind else {
        return None;
    };
    if !matches!(counter.kind, E::Read(counter) if counter == *id) {
        return None;
    }
    let (round, adjustment) = match (*test, step > 0.0) {
        ("<", true) => ("__ulua_ceil", " - 1"),
        ("<=", true) => ("__ulua_floor", ""),
        (">", false) => ("__ulua_floor", " + 1"),
        (">=", false) => ("__ulua_ceil", ""),
        _ => return None,
    };
    if let E::Read(bound) = bound.kind {
        if bound == *id || captured[bound] || writes(body, bound) {
            return None;
        }
    } else {
        literal(bound)?;
    }
    if writes(body, *id) || ir::loop_continue(body) {
        return None;
    }
    Some(Counted {
        id: *id,
        start,
        step,
        bound,
        round,
        adjustment,
    })
}
