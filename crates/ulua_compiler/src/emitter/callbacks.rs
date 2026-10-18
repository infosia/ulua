//! The emitter reads the call-site decision from the checked callback function.
use super::*;
use crate::ir::{Expr, ExprKind as E, Function, Method as M, Statement as S};

impl<'a> Emitter<'a> {
    pub(super) fn inline_callback(&self, args: &[Expr]) -> Option<&'a Function> {
        let E::Closure(id) = args.first()?.kind else {
            return None;
        };
        let index = self.function_indices.get(id).copied().flatten()?;
        self.module
            .functions
            .get(index)
            .filter(|f| f.inline_callback.is_some())
    }

    pub(super) fn callback_loop(
        &mut self,
        method: M,
        receiver: &Expr,
        args: &[Expr],
        dest: usize,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let function = self
            .inline_callback(args)
            .ok_or_else(|| self.limit("internal", "Missing inline callback decision"))?;
        let [statement] = function.body.as_slice() else {
            return Err(self.limit("internal", "Missing inline callback body"));
        };
        let (S::Return(Some(body)) | S::Eval(body)) = statement.form() else {
            return Err(self.limit("internal", "Invalid inline callback body"));
        };
        self.joins.enter();
        let live = self.generated_live;
        self.line("do");
        let array = self.expression(receiver, depth)?;
        // The initial value follows the receiver and precedes the length snapshot.
        let initial = if method == M::Reduce {
            let initial = args
                .get(1)
                .ok_or_else(|| self.limit("internal", "Missing inline reduce initial value"))?;
            Some(self.expression(initial, depth)?)
        } else {
            None
        };
        let n = self.generated()?;
        self.line(format!("local {n}: number = #{}", self.temp(array)));
        if let Some(initial) = initial {
            self.write_slot(dest, self.temp(initial));
            self.release(initial);
        } else if matches!(method, M::Map | M::Filter) {
            self.write_slot(dest, "{}");
        }
        let counter = self.reserve_counted()?;
        self.line_weighted(format!("for {counter} = 1, {n} do"), 4);
        self.indent += 1;
        let element = self.generated()?;
        let element_type = self
            .module
            .types
            .element(receiver.ty)
            .ok_or_else(|| self.limit("internal", "Missing inline receiver element type"))?;
        self.line(format!(
            "local {element}: {} = {}[{counter}]",
            self.function_types.luau(&element_type),
            self.temp(array)
        ));
        let mut previous = Vec::new();
        for (index, (id, _)) in function.parameters.iter().enumerate() {
            let local = self.generated()?;
            let value = match (method == M::Reduce, index) {
                (true, 0) => self.temp(dest),
                (false, 0) | (true, 1) => element.clone(),
                _ => format!("{counter} - 1"),
            };
            let ty = self.module.bindings[*id]
                .ty
                .ok_or_else(|| self.limit("internal", "Missing inline parameter type"))?;
            self.line(format!(
                "local {local}: {} = {value}",
                self.function_types.luau(&ty)
            ));
            previous.push((*id, self.copies.insert(*id, local)));
        }
        let value = self.expression(body, depth)?;
        if function.inline_callback == Some(false) {
            self.capture("__ulua_trap");
            self.line(format!(
                "if #{} ~= {n} then __ulua_trap(2) end",
                self.temp(array)
            ));
        }
        match method {
            M::Map => self.write(
                &format!("{}[{counter}]", self.temp(dest)),
                self.temp(value),
                &self.function_types.luau(&body.ty),
            ),
            M::Filter => {
                self.line(format!("if {} then", self.temp(value)));
                self.indent += 1;
                self.write(
                    &format!("{0}[#{0} + 1]", self.temp(dest)),
                    &element,
                    &self.function_types.luau(&element_type),
                );
                self.indent -= 1;
                self.line("end");
            }
            M::Reduce => self.write_slot(dest, self.temp(value)),
            M::ForEach => {}
            _ => return Err(self.limit("internal", "Invalid inline callback method")),
        }
        self.release(value);
        for (id, old) in previous {
            if let Some(old) = old {
                self.copies.insert(id, old);
            } else {
                self.copies.remove(&id);
            }
        }
        self.indent -= 1;
        self.line("end");
        self.release(array);
        self.line("end");
        self.generated_live = live;
        self.joins.leave();
        Ok(())
    }
}
