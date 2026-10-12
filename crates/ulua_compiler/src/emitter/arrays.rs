//! Checked element operations and live-length iteration preserve C12 order.
use super::*;
use crate::ir::{Expr, Statement};
impl Emitter<'_> {
    pub(super) fn index_check(&mut self, array: usize, index: usize, write: bool) {
        self.capture("__ulua_fmod");
        self.capture("__ulua_trap");
        let i = self.temp(index);
        // Test the original index. Adding one can round a fractional value to an integer.
        self.line(format!(
            "if not ({i} >= 0 and {i} {} #{} and __ulua_fmod({i}, 1) == 0) then __ulua_trap(1) end",
            if write { "<=" } else { "<" },
            self.temp(array)
        ));
    }
    pub(super) fn element(
        &mut self,
        a: &Expr,
        i: &Expr,
        op: Option<&str>,
        e: &Expr,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let a = self.expression(a, depth)?;
        let i = self.expression(i, depth)?;
        let target = format!("{}[{} + 1]", self.temp(a), self.temp(i));
        let old = if op.is_some() {
            self.index_check(a, i, false);
            let old = self.slot(&e.ty)?;
            self.write_slot(old, &target);
            Some(old)
        } else {
            None
        };
        let value = self.expression(e, depth)?;
        let result = if let (Some(op), Some(old)) = (op, old) {
            if op == "%" {
                self.capture("__ulua_fmod");
            }
            self.binary(op, &self.temp(old), &self.temp(value))
        } else {
            self.temp(value)
        };
        self.index_check(a, i, true);
        self.write(&target, result, &self.function_types.luau(&e.ty));
        for slot in [Some(a), Some(i), Some(value), old].into_iter().flatten() {
            self.release(slot);
        }
        Ok(())
    }
    pub(super) fn for_of(
        &mut self,
        id: usize,
        a: &Expr,
        body: &Statement,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        self.joins.enter();
        let a = self.expression(a, depth)?;
        let counter = self.slot(&Type::Number)?;
        self.write_slot(counter, "1");
        self.line(format!(
            "while {} <= #{} do",
            self.temp(counter),
            self.temp(a)
        ));
        self.indent += 1;
        // Emit the binding's storage before its value, once per iteration.
        if let Statement::Block(parts) = body {
            if let Some(entry) = parts.first() {
                self.statement(entry, depth)?;
            }
            self.write_binding(id, format!("{}[{}]", self.temp(a), self.temp(counter)))?;
            // Increment before the body, so continue reaches the next step.
            self.write_slot(counter, format!("{} + 1", self.temp(counter)));
            self.loops.push(Loop { tail: None });
            for s in parts.iter().skip(1) {
                self.statement(s, depth)?;
            }
            self.loops.pop();
        }
        self.indent -= 1;
        self.line("end");
        self.release(counter);
        self.release(a);
        self.joins.leave();
        Ok(())
    }
}
