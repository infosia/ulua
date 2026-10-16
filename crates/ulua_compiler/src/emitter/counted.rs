//! Numeric loops require stable bounds and counters, proved from checked binding identities.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement as S};

pub(super) struct Counted<'a> {
    id: usize,
    start: f64,
    step: f64,
    bound: &'a Expr,
    round: &'static str,
    adjustment: &'static str,
}

fn literal(expr: &Expr) -> Option<f64> {
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

impl Emitter<'_> {
    pub(super) fn counted_loop<'e>(
        &self,
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
        if !start.is_finite() || start.fract() != 0.0 || self.module.captured[*id] {
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
            if bound == *id || self.module.captured[bound] || writes(body, bound) {
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

    pub(super) fn counted_bound_local(&self, form: &Counted<'_>) -> bool {
        match form.bound.kind {
            E::Read(id) => self.local_binding(id),
            _ => true,
        }
    }

    pub(super) fn reserve_counted(&mut self) -> Result<String, Diagnostic> {
        // The visible counter and three VM control registers coexist during the body.
        let counter = self.generated()?;
        self.generated()?;
        self.generated()?;
        self.generated()?;
        Ok(counter)
    }

    pub(super) fn emit_counted(
        &mut self,
        form: Counted<'_>,
        body: &S,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        self.joins.enter();
        let live = self.generated_live;
        self.line("do");
        let bound = self.operand(form.bound, depth, false, 2)?;
        let limit = self.generated()?;
        self.capture(form.round);
        self.line_weighted(
            format!(
                "local {limit}: number = {}({}){}",
                form.round,
                self.operand_text(&bound),
                form.adjustment
            ),
            2,
        );
        self.release_operand(bound);
        // Luau needs three control registers and one visible counter for FORNPREP/FORNLOOP.
        // Reserve all four in the storage probe, even when O1 reuses the limit register.
        let counter = self.reserve_counted()?;
        self.line_weighted(
            format!(
                "for {counter} = {}, {limit}, {} do",
                number(form.start),
                number(form.step)
            ),
            4,
        );
        self.indent += 1;
        // At large finite bounds, rounding ceil(b)-1 or floor(b)+1 can return b itself.
        // A strict comparison must exclude that endpoint, including when an update reaches infinity.
        let strict_endpoint = !form.adjustment.is_empty()
            && literal(form.bound).is_none_or(|b| {
                let limit = if form.step > 0.0 {
                    b.ceil() - 1.0
                } else {
                    b.floor() + 1.0
                };
                limit == b
            });
        if strict_endpoint {
            let bound = self.operand(form.bound, depth, false, 2)?;
            self.line_weighted(
                format!(
                    "if {counter} == {} then break end",
                    self.operand_text(&bound)
                ),
                2,
            );
            self.release_operand(bound);
        }
        let previous = self.copies.insert(form.id, counter);
        self.loops.push(Loop { tail: None });
        self.statement(body, depth)?;
        self.loops.pop();
        if let Some(previous) = previous {
            self.copies.insert(form.id, previous);
        } else {
            self.copies.remove(&form.id);
        }
        self.indent -= 1;
        self.line("end");
        self.line("end");
        self.generated_live = live;
        self.joins.leave();
        Ok(())
    }
}
