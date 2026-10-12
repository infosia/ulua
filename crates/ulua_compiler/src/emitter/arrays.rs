//! Checked element operations and live-length iteration preserve C12 order.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement};
impl Emitter<'_> {
    pub(super) fn array_literal(
        &mut self,
        dest: usize,
        entries: &[Expr],
        depth: u32,
    ) -> Result<(), Diagnostic> {
        self.write_slot(dest, "{}");
        for (index, entry) in entries.iter().enumerate() {
            self.offset = entry.start as usize;
            let constant = match &entry.kind {
                E::Number(n) => {
                    if n.is_infinite() {
                        self.capture("__ulua_huge");
                    }
                    Some(number(*n))
                }
                E::Boolean(b) => Some(b.to_string()),
                E::String(s) => Some(quote(s)),
                E::Unary("-", a) => {
                    if let E::Number(n) = a.kind {
                        if n.is_infinite() {
                            self.capture("__ulua_huge");
                        }
                        Some(number(-n))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            // A one-element block avoids retained values across nested literals.
            // Append each block before the next source element can change a binding.
            let (value, slot) = if let Some(value) = constant {
                (value, None)
            } else {
                let slot = self.expression(entry, depth)?;
                (self.temp(slot), Some(slot))
            };
            self.write(
                &format!("{}[{}]", self.temp(dest), index + 1),
                value,
                &self.function_types.luau(&entry.ty),
            );
            if let Some(slot) = slot {
                self.release(slot);
            }
        }
        Ok(())
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
        let old = if op.is_some() {
            self.capture("__ulua_get");
            let old = self.slot(&e.ty)?;
            self.write_slot(
                old,
                format!("__ulua_get({}, {})", self.temp(a), self.temp(i)),
            );
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
        self.capture("__ulua_set");
        self.line(format!(
            "__ulua_set({}, {}, {result})",
            self.temp(a),
            self.temp(i)
        ));
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
