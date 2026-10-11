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
                self.line(format!("__ulua_b{id} = {}", self.temp(v)));
                self.release(v);
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                self.line(format!("__ulua_b{id} = __ulua_arg{id}"));
                self.indent -= 1;
                self.line("end");
            }
        }
        Ok(())
    }
    pub(super) fn statements(&mut self, body: &[S], depth: u32) -> Result<(), Diagnostic> {
        let depth = self.guard(depth)?;
        for s in body {
            self.statement(s, depth)?;
        }
        Ok(())
    }
    fn statement(&mut self, s: &S, depth: u32) -> Result<(), Diagnostic> {
        let depth = self.guard(depth)?;
        match s {
            S::Declare(id, e) => {
                let v = self.expression(e, depth)?;
                self.line(format!("{} = {}", self.location(*id)?, self.temp(v)));
                self.release(v);
            }
            S::Assign(id, op, e) => {
                let old = if op.is_some() {
                    let ty = self.module.bindings[*id]
                        .ty
                        .as_ref()
                        .ok_or_else(|| self.limit("C11", "Missing assignment type"))?;
                    let v = self.slot(ty);
                    self.line(format!("{} = {}", self.temp(v), self.location(*id)?));
                    Some(v)
                } else {
                    None
                };
                let v = self.expression(e, depth)?;
                let value = if let (Some(op), Some(old)) = (op, old) {
                    self.binary(op, &self.temp(old), &self.temp(v))
                } else {
                    self.temp(v)
                };
                self.line(format!("{} = {value}", self.location(*id)?));
                self.release(v);
                if let Some(v) = old {
                    self.release(v);
                }
            }
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
            }
            S::While(e, b) => {
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
            }
            S::For(init, test, update, body, ids) => {
                self.statements(init, depth)?;
                let tail = self.generated("boolean", "true");
                let mut copies = vec![];
                let mut previous = vec![];
                for id in ids {
                    let ty = self.module.bindings[*id]
                        .ty
                        .as_ref()
                        .ok_or_else(|| self.limit("C11", "Missing loop type"))?;
                    let local = self.generated(&ty.luau(), initial(ty));
                    if let Some(generated) = self.generated.last_mut() {
                        generated.3 = false;
                    }
                    let head = self.location(*id)?;
                    copies.push((head, local.clone()));
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
                for (id, (head, local)) in ids.iter().zip(&copies) {
                    let ty = self.module.bindings[*id]
                        .ty
                        .as_ref()
                        .ok_or_else(|| self.limit("C11", "Missing loop type"))?;
                    self.line(format!("local {local}: {} = {head}", ty.luau()));
                }
                for (id, (_, local)) in ids.iter().zip(&copies) {
                    self.copies.insert(*id, local.clone());
                }
                self.line(format!("{tail} = true"));
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
                for (head, local) in &copies {
                    self.line(format!("{head} = {local}"));
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
            }
            S::Break => {
                if let Some(Loop {
                    tail: Some(tail), ..
                }) = self.loops.last()
                {
                    self.line(format!("{tail} = false"));
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
        Ok(())
    }
}
