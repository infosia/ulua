//! Loop tails implement copy-back and update on each continue path.
use super::*;
use crate::ir::Statement as S;
impl Emitter<'_> {
    pub(super) fn defaults(&mut self, f: &ir::Function) -> Result<(), Diagnostic> {
        for (id, value) in &f.parameters {
            if let Some(value) = value {
                self.line(format!("if __ulua_arg{id} == nil then"));
                self.indent += 1;
                let v = self.expression(value, 0)?;
                self.write_binding(*id, self.temp(v))?;
                self.release(v);
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                self.write_binding(*id, format!("__ulua_arg{id}"))?;
                self.indent -= 1;
                self.line("end");
            }
        }
        Ok(())
    }
    pub(super) fn statements(&mut self, body: &[S], depth: u32) -> Result<(), Diagnostic> {
        for s in body {
            self.statement(s, depth)?;
        }
        Ok(())
    }
    pub(super) fn statement(&mut self, s: &S, depth: u32) -> Result<(), Diagnostic> {
        self.output_check()?;
        let parent_offset = self.statement_offset;
        if let S::Located(offset, _) = s {
            self.statement_offset = *offset as usize;
            self.offset = *offset as usize;
        }
        if !self.joins.is_empty() {
            // Hoist a demanded join before control flow. A reset inside one arm
            // does not cut the analyzer dependency of the other arm.
            for id in ir::reads(s) {
                self.read_binding(id)?;
            }
        }
        let depth = self.guard(depth)?;
        match s.form() {
            S::Entry(ids) => {
                if let Some(context) = self.context {
                    let key = ids.iter().min().map_or(0, |id| id + 1);
                    if let Some(headers) = context.layouts[self.owner]
                        .as_ref()
                        .and_then(|l| l.entries.get(&key))
                    {
                        let headers = self.headers(headers);
                        if let Some(output) = self.output {
                            output.charge(headers.len(), self.statement_offset);
                            output.check(self.module)?;
                        }
                        self.out.push_str(&headers);
                    }
                }
            }
            S::Function(id) => {
                let value = self.closure(*id, depth)?;
                self.write_binding(*id, value)?;
            }
            S::Located(_, _) => {
                return Err(self.limit("internal", "Nested source location wrapper"));
            }
            S::Declare(id, e) => {
                let v = self.expression(e, depth)?;
                self.write_binding(*id, self.temp(v))?;
                self.release(v);
            }
            S::Assign(id, op, e) => {
                let old = if op.is_some() {
                    let ty = self.module.bindings[*id]
                        .ty
                        .as_ref()
                        .ok_or_else(|| self.limit("C11", "Missing assignment type"))?;
                    let v = self.slot(ty)?;
                    let value = self.read_binding(*id)?;
                    self.write_slot(v, value);
                    Some(v)
                } else {
                    None
                };
                let v = self.expression(e, depth)?;
                let value = if let (Some(op), Some(old)) = (op, old) {
                    if *op == "%" {
                        self.capture("__ulua_fmod");
                    }
                    self.binary(op, &self.temp(old), &self.temp(v))
                } else {
                    self.temp(v)
                };
                self.write_binding(*id, value)?;
                self.joins.assigned(*id);
                self.release(v);
                if let Some(v) = old {
                    self.release(v);
                }
            }
            S::Element(a, i, op, e) => self.element(a, i, *op, e, depth)?,
            S::ForOf(id, a, body) => self.for_of(*id, a, body, depth)?,
            S::Eval(e) => {
                let v = self.expression(e, depth)?;
                self.release(v);
            }
            S::Block(b) => {
                self.line("do");
                self.indent += 1;
                self.statements(b, depth)?;
                self.indent -= 1;
                self.line("end");
            }
            S::If(e, a, b) => {
                self.joins.enter();
                let t = self.expression(e, depth)?;
                self.line(format!("if {} then", self.temp(t)));
                self.release(t);
                self.indent += 1;
                self.statement(a, depth)?;
                self.indent -= 1;
                if let Some(b) = b {
                    self.line("else");
                    self.indent += 1;
                    self.statement(b, depth)?;
                    self.indent -= 1;
                }
                self.line("end");
                self.joins.leave();
            }
            S::While(e, b) => {
                self.joins.enter();
                self.line("while true do");
                self.indent += 1;
                if !matches!(e.kind, ir::ExprKind::Boolean(true)) {
                    let t = self.expression(e, depth)?;
                    self.line(format!("if not {} then break end", self.temp(t)));
                    self.release(t);
                }
                self.loops.push(Loop { tail: None });
                self.statement(b, depth)?;
                self.loops.pop();
                self.indent -= 1;
                self.line("end");
                self.joins.leave();
            }
            S::For(init, test, update, body, ids) => {
                self.joins.enter();
                self.statements(init, depth)?;
                self.offset = self.statement_offset;
                let tail = self.generated("boolean", "true")?;
                let mut copies = vec![];
                let mut previous = vec![];
                for id in ids {
                    let ty = self.module.bindings[*id]
                        .ty
                        .as_ref()
                        .ok_or_else(|| self.limit("C11", "Missing loop type"))?;
                    let local = self.generated(&self.function_types.luau(ty), initial(ty))?;
                    if let Some(generated) = self.generated.last_mut() {
                        generated.3 = false;
                    }
                    let head = self.location(*id)?;
                    copies.push((head, local.clone(), self.function_types.luau(ty)));
                    previous.push((*id, self.copies.insert(*id, local)));
                }
                // The condition and update use head bindings; only the body uses copies.
                for (id, old) in &previous {
                    if let Some(old) = old {
                        self.copies.insert(*id, old.clone());
                    } else {
                        self.copies.remove(id);
                    }
                }
                self.line("while true do");
                self.indent += 1;
                if let Some(e) = test
                    && !matches!(e.kind, ir::ExprKind::Boolean(true))
                {
                    let t = self.expression(e, depth)?;
                    self.line(format!("if not {} then break end", self.temp(t)));
                    self.release(t);
                }
                for (id, (head, local, _)) in ids.iter().zip(&copies) {
                    let ty = self.module.bindings[*id]
                        .ty
                        .as_ref()
                        .ok_or_else(|| self.limit("C11", "Missing loop type"))?;
                    self.line(format!(
                        "local {local}: {0} = ({head}) :: {0}",
                        self.function_types.luau(ty)
                    ));
                }
                for (id, (_, local, _)) in ids.iter().zip(&copies) {
                    self.copies.insert(*id, local.clone());
                }
                self.write(&tail, "true", "boolean");
                self.line("repeat");
                self.indent += 1;
                self.loops.push(Loop {
                    tail: Some(tail.clone()),
                });
                self.statement(body, depth)?;
                self.loops.pop();
                self.indent -= 1;
                self.line("until true");
                if ir::loop_break(body) {
                    self.line(format!("if not {tail} then break end"));
                }
                for (head, local, ty) in &copies {
                    self.write(head, local, ty);
                }
                for (id, old) in previous {
                    if let Some(old) = old {
                        self.copies.insert(id, old);
                    } else {
                        self.copies.remove(&id);
                    }
                }
                if let Some(update) = update {
                    self.statement(update, depth)?;
                }
                self.indent -= 1;
                self.line("end");
                self.joins.leave();
            }
            S::Break => {
                if let Some(Loop {
                    tail: Some(tail), ..
                }) = self.loops.last()
                {
                    let tail = tail.clone();
                    self.write(&tail, "false", "boolean");
                }
                // A do block allows a following source statement after a terminator.
                self.line("do break end");
            }
            S::Continue => {
                let context = self
                    .loops
                    .last()
                    .cloned()
                    .ok_or_else(|| self.limit("C11", "Missing loop"))?;
                if context.tail.is_some() {
                    self.line("do break end");
                } else {
                    self.line("do continue end");
                }
            }
            S::Return(value) => {
                if let Some(e) = value {
                    let v = self.expression(e, depth)?;
                    self.line(format!("do return {} end", self.temp(v)));
                    self.release(v);
                } else {
                    self.line("do return end");
                }
            }
        }
        self.statement_offset = parent_offset;
        self.output_check()
    }
}
