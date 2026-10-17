//! Checked element operations and live-length iteration preserve C12 order.
use super::*;
use crate::ir::{Expr, ExprKind as E, IndexForm, Statement};
pub(super) enum ElementTarget {
    Slot(usize),
    Binding(usize),
}
impl Emitter<'_> {
    // An element that supplies an access object keeps its value until that access checks it.
    pub(super) fn access_receiver(
        &mut self,
        receiver: &Expr,
        depth: u32,
    ) -> Result<(usize, bool), Diagnostic> {
        if let E::Index(a, i, _) = &receiver.kind {
            self.offset = receiver.start as usize;
            let depth = self.guard(depth)?;
            let (a, deferred) = self.access_receiver(a, depth)?;
            let i = self.expression(i, depth)?;
            self.access_receiver_check(a, deferred);
            self.offset = receiver.start as usize;
            let slot = self.slot(&receiver.ty)?;
            self.capture("__ulua_get");
            self.write_slot(
                slot,
                format!("__ulua_get({}, {}, true)", self.temp(a), self.temp(i)),
            );
            self.release(a);
            self.release(i);
            return Ok((slot, true));
        }
        Ok((self.expression(receiver, depth)?, false))
    }
    pub(super) fn access_receiver_check(&mut self, receiver: usize, deferred: bool) {
        self.access_receiver_check_text(&self.temp(receiver), deferred);
    }
    fn access_receiver_check_text(&mut self, receiver: &str, deferred: bool) {
        if deferred {
            self.capture("__ulua_receiver");
            self.line(format!("__ulua_receiver({receiver})"));
        }
    }
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
            // Each literal owns a fresh array; these integral keys are consecutive and non-nil.
            // The helper keeps field writes out of the caller's flow graph.
            self.capture("__ulua_literal_set");
            self.line(format!(
                "__ulua_literal_set({}, {}, ({value}) :: {})",
                self.temp(dest),
                index + 1,
                self.function_types.luau(&entry.ty)
            ));
            if let Some(slot) = slot {
                self.release(slot);
            }
        }
        Ok(())
    }
    pub(super) fn element_read(
        &mut self,
        dest: ElementTarget,
        ty: Type,
        array: usize,
        index: &str,
        form: IndexForm,
    ) -> Result<(), Diagnostic> {
        self.element_read_text(dest, ty, &self.temp(array), index, form)
    }
    fn element_read_text(
        &mut self,
        dest: ElementTarget,
        ty: Type,
        array: &str,
        index: &str,
        form: IndexForm,
    ) -> Result<(), Diagnostic> {
        let test = self.index_test(array, index, "<", form);
        self.capture("__ulua_trap");
        // Reject unsafe numeric keys before the VM converts a table index to an integer.
        // A fresh scoped local bounds analyzer growth across repeated element checks.
        let read = self.generated()?;
        let ty = self.function_types.luau(&ty);
        self.line("do");
        // Keep the full check weight; this also bounds every proven check.
        self.line_weighted(format!("if not ({test}) then __ulua_trap(1) end"), 4);
        self.line(format!("local {read}: {ty} = {array}[{index} + 1]"));
        match dest {
            ElementTarget::Slot(dest) => self.write_slot(dest, read),
            ElementTarget::Binding(id) => self.write_binding(id, read)?,
        }
        self.line("end");
        self.generated_live -= 1;
        Ok(())
    }
    fn index_test(&mut self, array: &str, index: &str, bound: &str, form: IndexForm) -> String {
        if form == IndexForm::Proven {
            #[cfg(feature = "remainder-shadow")]
            if self.remainder_shadow {
                self.capture("__ulua_floor");
                self.line_weighted(format!("if not ({index} >= 0 and __ulua_floor({index}) == {index}) then error(\"proven index shadow mismatch\") end"), 4);
            }
            format!("{index} {bound} #{array}")
        } else {
            self.capture("__ulua_floor");
            format!(
                "{index} >= 0 and {index} {bound} #{array} and __ulua_floor({index}) == {index}"
            )
        }
    }
    fn element_check(
        &mut self,
        array: &str,
        index: &str,
        bound: &str,
        form: IndexForm,
    ) -> Result<(), Diagnostic> {
        let test = self.index_test(array, index, bound, form);
        let checked = self.generated()?;
        self.line("do");
        self.line_weighted(format!("local {checked}: boolean = not ({test})"), 4);
        self.line(format!("if {checked} then __ulua_trap(1) end"));
        self.line("end");
        self.generated_live -= 1;
        Ok(())
    }
    pub(super) fn element(
        &mut self,
        a: &Expr,
        i: &Expr,
        op: Option<&str>,
        e: &Expr,
        form: IndexForm,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        // Stable receivers need no mutable snapshot across index and RHS calls.
        let receiver_type = self.function_types.luau(&a.ty);
        let (a, deferred) = if self.inline_enabled && self.inline_tree(a, true, 1).is_some() {
            (self.operand(a, depth, true, 1)?, false)
        } else {
            let (slot, deferred) = self.access_receiver(a, depth)?;
            (operands::Operand::Temporary(slot), deferred)
        };
        // Each write gets a receiver definition; repeated stores cannot join its refinement graph.
        let array = self.generated()?;
        self.line("do");
        self.line(format!(
            "local {array}: {receiver_type} = {}",
            self.operand_text(&a)
        ));
        // Keep stable operands inline; snapshot an index that a later call can change.
        let i = self.operand(i, depth, true, 1)?;
        let index = self.operand_text(&i);
        let old = if op.is_some() {
            self.access_receiver_check_text(&array, deferred);
            let old = self.slot(&e.ty)?;
            self.element_read_text(ElementTarget::Slot(old), e.ty, &array, &index, form)?;
            Some(old)
        } else {
            None
        };
        let value = self.expression(e, depth)?;
        if op.is_none() {
            self.access_receiver_check_text(&array, deferred);
        }
        let result = if let (Some(op), Some(old)) = (op, old) {
            if op == "%" {
                self.capture("__ulua_fmod");
            }
            let result = self.binary(op, &self.temp(old), &self.temp(value));
            // Freeze concatenation and its allocation before the write checks.
            self.write_slot(value, result);
            self.temp(value)
        } else {
            self.temp(value)
        };
        self.capture("__ulua_trap");
        self.element_check(&array, &index, "<=", form)?;
        self.write(
            &format!("{array}[{index} + 1]"),
            result,
            &self.function_types.luau(&e.ty),
        );
        self.line("end");
        self.generated_live -= 1;
        self.release_operand(i);
        self.release_operand(a);
        for slot in [Some(value), old].into_iter().flatten() {
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
        let table = self.module.types.table_parts(a.ty).is_some();
        let values = matches!(
            a.kind,
            crate::ir::ExprKind::Method(
                crate::ir::Method::Table(crate::tables::TableMethod::Values),
                ..
            )
        );
        let a = if table {
            if let crate::ir::ExprKind::Method(_, _, receiver, _) = &a.kind {
                receiver
            } else {
                a
            }
        } else {
            a
        };
        let string = a.ty == Type::String;
        let a = self.expression(a, depth)?;
        let counter = self.slot(&Type::Number)?;
        self.write_slot(counter, "1");
        self.line(format!(
            "while {} <= {} do",
            self.temp(counter),
            if table {
                format!("{}.slots", self.temp(a))
            } else {
                format!("#{}", self.temp(a))
            }
        ));
        self.indent += 1;
        // Emit the binding's storage before its value, once per iteration.
        if let Statement::Block(parts) = body {
            if let Some(entry) = parts.first() {
                self.statement(entry, depth)?;
            }
            if table {
                self.line(format!(
                    "if {}.keys[{}] == nil then",
                    self.temp(a),
                    self.temp(counter)
                ));
                self.indent += 1;
                self.write_slot(counter, format!("{} + 1", self.temp(counter)));
                self.line("continue");
                self.indent -= 1;
                self.line("end");
                self.write_binding(
                    id,
                    format!(
                        "{}.{}[{}]",
                        self.temp(a),
                        if values { "values" } else { "keys" },
                        self.temp(counter)
                    ),
                )?;
                self.write_slot(counter, format!("{} + 1", self.temp(counter)));
            } else if string {
                self.capture("__ulua_str_charAt_0");
                self.capture("__ulua_str_next_0");
                self.write_binding(
                    id,
                    format!(
                        "__ulua_str_charAt_0({}, {} - 1)",
                        self.temp(a),
                        self.temp(counter)
                    ),
                )?;
                self.write_slot(
                    counter,
                    format!(
                        "__ulua_str_next_0({}, {} - 1) + 1",
                        self.temp(a),
                        self.temp(counter)
                    ),
                );
            } else {
                self.write_binding(id, format!("{}[{}]", self.temp(a), self.temp(counter)))?;
                self.write_slot(counter, format!("{} + 1", self.temp(counter)));
            }
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
