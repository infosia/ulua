//! Operand forms carry temporary ownership and the bounded inline tree.
use super::*;
use crate::ir::{Expr, ExprKind as E};

pub(super) struct Operation {
    pub(super) text: String,
    pub(super) cost: usize,
}
impl Operation {
    fn new(expr: &Expr, text: String) -> Self {
        Self {
            text,
            // Inline forms remove statements. Reserve earlier checkpoints for their remaining work.
            cost: match expr.kind {
                E::Binary("%", _, _) => 3,
                E::Binary("<" | "<=" | ">" | ">=" | "==" | "~=", _, _) => 2,
                _ => 1,
            },
        }
    }
}

pub(super) enum Operand {
    Inline(Operation),
    Temporary(usize),
}
impl Emitter<'_> {
    pub(super) fn local_binding(&self, id: usize) -> bool {
        self.inline_enabled
            && !self
                .context
                .map_or(&self.spilled, |context| &context.spilled)
                .contains(&id)
            && !self.probe_spills.is_some_and(|spills| spills.contains(&id))
            && !self.uses_accessor(id)
    }

    pub(super) fn operand_text(&self, value: &Operand) -> String {
        match value {
            Operand::Inline(value) => value.text.clone(),
            Operand::Temporary(slot) => self.temp(*slot),
        }
    }

    pub(super) fn operand_cost(&self, value: &Operand) -> usize {
        match value {
            Operand::Inline(value) => value.cost,
            Operand::Temporary(_) => 1,
        }
    }

    pub(super) fn release_operand(&mut self, value: Operand) {
        if let Operand::Temporary(slot) = value {
            self.release(slot);
        }
    }

    pub(super) fn operand(
        &mut self,
        expr: &Expr,
        depth: u32,
        later_call: bool,
        nodes: usize,
    ) -> Result<Operand, Diagnostic> {
        if self.inline_enabled && self.inline_tree(expr, later_call, nodes).is_some() {
            let value = self.pure_operand(expr)?;
            return Ok(Operand::Inline(Operation::new(expr, value)));
        }
        Ok(Operand::Temporary(self.expression(expr, depth)?))
    }

    // None explicitly requires a temporary. No decision depends on generated source text.
    pub(super) fn inline_tree(&self, expr: &Expr, later_call: bool, nodes: usize) -> Option<usize> {
        let remaining = nodes.checked_sub(1)?;
        match &expr.kind {
            E::Null | E::Number(_) | E::Boolean(_) | E::String(_) | E::Builtin(_) => {
                Some(remaining)
            }
            E::Read(id)
                if self.local_binding(*id)
                    && (!later_call
                        || (self.module.bindings[*id].owner == self.owner
                            && self.owner != 0
                            && !self.module.captured[*id])) =>
            {
                Some(remaining)
            }
            E::Unary(_, a) => self.inline_tree(a, later_call, remaining),
            E::Binary(_, a, b) => {
                let remaining = self.inline_tree(a, later_call, remaining)?;
                self.inline_tree(b, later_call, remaining)
            }
            _ => None,
        }
    }

    pub(super) fn pure_operand(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        Ok(match &expr.kind {
            E::Null => "nil".into(),
            E::Number(n) => {
                if n.is_infinite() {
                    self.capture("__ulua_huge");
                }
                number(*n)
            }
            E::Boolean(b) => b.to_string(),
            E::String(s) => quote(s),
            E::Builtin(name) => {
                self.capture(name);
                (*name).into()
            }
            E::Read(id) => {
                let value = self.read_binding(*id)?;
                if self.module.bindings[*id].ty != Some(expr.ty) {
                    format!("({value} :: {})", self.function_types.luau(&expr.ty))
                } else {
                    value
                }
            }
            E::Unary(op, a) => format!("({op} ({}))", self.pure_operand(a)?),
            E::Binary(op, a, b) => {
                let types = [a.ty, b.ty];
                let a = self.pure_operand(a)?;
                let b = self.pure_operand(b)?;
                self.expression_binary(op, types, &a, &b)
            }
            _ => return Err(self.limit("internal", "Invalid inline operand form")),
        })
    }

    // The final operation can target either a binding or a reusable typed slot.
    pub(super) fn last_value(
        &mut self,
        expr: &Expr,
        depth: u32,
    ) -> Result<Option<Operation>, Diagnostic> {
        self.offset = expr.start as usize;
        if let E::Closure(id) = expr.kind {
            return self.closure(id, depth).map(Some);
        }
        if self
            .inline_tree(expr, false, expressions::PURE_EXPRESSION_NODES)
            .is_some()
        {
            return self
                .pure_operand(expr)
                .map(|text| Some(Operation::new(expr, text)));
        }
        let text = match &expr.kind {
            E::Read(id) => Some(self.read_binding(*id)?),
            E::Unary(op, a) => {
                let a = self.operand(a, depth, false, 2)?;
                let text = format!("{op} ({})", self.operand_text(&a));
                self.release_operand(a);
                Some(text)
            }
            E::Binary(op, a, b) => {
                let types = [a.ty, b.ty];
                let a = self.operand(a, depth, contains_call(b), 1)?;
                let b = self.operand(b, depth, false, 1)?;
                let text = self.expression_binary(
                    op,
                    types,
                    &self.operand_text(&a),
                    &self.operand_text(&b),
                );
                self.release_operand(a);
                self.release_operand(b);
                Some(text)
            }
            E::Field(receiver, key) => {
                let (slot, deferred) = self.access_receiver(receiver, depth)?;
                self.access_receiver_check(slot, deferred);
                let value = format!("{}.__ulua_k{key}", self.temp(slot));
                self.release(slot);
                Some(value)
            }
            E::Length(receiver) => {
                let slot = self.expression(receiver, depth)?;
                let value = format!("#{}", self.temp(slot));
                self.release(slot);
                Some(value)
            }
            E::Method(method, arity, receiver, args) => {
                let helper = methods::name(*method, *arity);
                self.capture(&helper);
                let values =
                    self.call_operands(std::iter::once(receiver.as_ref()).chain(args), depth)?;
                Some(format!("{helper}({values})"))
            }
            E::ClassCall(callee, receiver, args, false) => {
                let callee = self.operand(
                    callee,
                    depth,
                    contains_call(receiver) || args.iter().any(contains_call),
                    1,
                )?;
                let values =
                    self.call_operands(std::iter::once(receiver.as_ref()).chain(args), depth)?;
                let value = format!("{}({values})", self.operand_text(&callee));
                self.release_operand(callee);
                Some(value)
            }
            E::Call(callee, args)
                if !matches!(callee.kind, E::Builtin(name)
                if name.starts_with("__ulua_on_") || name == "__ulua_all") =>
            {
                let callee = self.operand(callee, depth, args.iter().any(contains_call), 1)?;
                let values = self.call_operands(args.iter(), depth)?;
                let text = format!("{}({values})", self.operand_text(&callee));
                self.release_operand(callee);
                Some(text)
            }
            _ => None,
        };
        Ok(text.map(|text| Operation::new(expr, text)))
    }

    fn call_operands<'e>(
        &mut self,
        args: impl Iterator<Item = &'e Expr>,
        depth: u32,
    ) -> Result<String, Diagnostic> {
        let args: Vec<_> = args.collect();
        let mut later = false;
        let mut suffix = vec![false; args.len()];
        for (i, arg) in args.iter().enumerate().rev() {
            suffix[i] = later;
            later |= contains_call(arg);
        }
        let mut values = Vec::with_capacity(args.len());
        for (arg, later) in args.into_iter().zip(suffix) {
            values.push(self.operand(arg, depth, later, 1)?);
        }
        let text = values
            .iter()
            .map(|value| self.operand_text(value))
            .collect::<Vec<_>>()
            .join(", ");
        for value in values {
            self.release_operand(value);
        }
        Ok(text)
    }

    pub(super) fn binding_expression(
        &mut self,
        id: usize,
        expr: &Expr,
        depth: u32,
        declaration: bool,
    ) -> Result<(), Diagnostic> {
        // The checked local value can target the binding without a reusable result slot.
        if self.local_binding(id)
            && let E::Index(a, i) = &expr.kind
        {
            self.offset = expr.start as usize;
            let depth = self.guard(depth)?;
            let (a, deferred) = self.access_receiver(a, depth)?;
            let i = self.operand(i, depth, false, 1)?;
            self.access_receiver_check(a, deferred);
            self.element_read(
                super::arrays::ElementTarget::Binding(id),
                expr.ty,
                a,
                &self.operand_text(&i),
            )?;
            self.release(a);
            self.release_operand(i);
            return Ok(());
        }
        if self.local_binding(id)
            && let Some(value) = self.last_value(expr, depth)?
        {
            self.write_binding_weighted(id, value.text, value.cost)
        } else {
            let value = self.expression(expr, depth)?;
            if declaration {
                self.write_binding_slot(id, value)?;
            } else {
                self.write_binding(id, self.temp(value))?;
            }
            self.release(value);
            Ok(())
        }
    }

    pub(super) fn comparison(&mut self, expr: &Expr, depth: u32) -> Result<Operand, Diagnostic> {
        if self.inline_enabled
            && matches!(
                expr.kind,
                E::Binary("<" | "<=" | ">" | ">=" | "==" | "~=", _, _)
            )
            && let Some(value) = self.last_value(expr, depth)?
        {
            return Ok(Operand::Inline(value));
        }
        self.operand(expr, depth, false, 1)
    }
}

// Calls in any later subtree can change a captured or module binding.
fn contains_call(expr: &Expr) -> bool {
    let mut work = vec![expr];
    while let Some(expr) = work.pop() {
        match &expr.kind {
            E::Call(..)
            | E::ClassCall(..)
            | E::Method(..)
            | E::OptionalCall(..)
            | E::OptionalMethod(..)
            | E::Await(..) => return true,
            E::Error(_, a)
            | E::InstanceOf(a, _)
            | E::ErrorField(a, _)
            | E::OptionalErrorField(a, _)
            | E::Field(a, _)
            | E::OptionalField(a, _)
            | E::OptionalLength(a)
            | E::Unary(_, a)
            | E::Length(a) => work.push(a),
            E::OptionalPush(a, b)
            | E::Coalesce(a, b)
            | E::Binary(_, a, b)
            | E::Logical(_, a, b)
            | E::Index(a, b)
            | E::Push(a, b) => work.extend([a.as_ref(), b.as_ref()]),
            E::Conditional(t, a, b) => work.extend([t.as_ref(), a.as_ref(), b.as_ref()]),
            E::Object(fields) => work.extend(fields.iter().map(|(_, e)| e)),
            E::Template(parts) | E::Array(parts) => work.extend(parts),
            E::Null
            | E::Number(_)
            | E::Boolean(_)
            | E::String(_)
            | E::Read(_)
            | E::Closure(_)
            | E::Builtin(_) => {}
        }
    }
    false
}
