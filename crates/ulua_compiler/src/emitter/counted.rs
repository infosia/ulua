//! Numeric loop emission uses the shared eligibility decision.
use super::*;
use crate::counted::{self, Counted, literal};
use crate::ir::{Expr, ExprKind as E, Statement as S};

impl Emitter<'_> {
    pub(super) fn counted_loop<'e>(
        &self,
        init: &[S],
        test: Option<&'e Expr>,
        update: Option<&S>,
        body: &S,
    ) -> Option<Counted<'e>> {
        counted::select(&self.module.captured, init, test, update, body)
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
        let lengths = self.begin_invariant_lengths(body)?;
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
        for id in lengths {
            self.invariant_lengths.remove(&id);
        }
        self.generated_live = live;
        self.joins.leave();
        Ok(())
    }
}
