//! Shared function type identities bound analyzer work at repeated calls.
use crate::ir::{self, ExprKind as E, Statement as S, Type};
use std::{collections::HashMap, fmt::Write};

pub(super) struct FunctionTypes {
    names: HashMap<String, String>,
    pub(super) declarations: String,
}
impl FunctionTypes {
    pub(super) fn new(module: &ir::Module) -> Self {
        let mut types = Self {
            names: HashMap::new(),
            declarations: String::new(),
        };
        for binding in &module.bindings {
            if let Some(ty) = &binding.ty {
                types.add(ty);
            }
        }
        let mut statements: Vec<_> = module.body.iter().collect();
        let mut expressions = vec![];
        for function in &module.functions {
            statements.extend(&function.body);
            expressions.extend(
                function
                    .parameters
                    .iter()
                    .filter_map(|(_, default)| default.as_ref()),
            );
        }
        while let Some(statement) = statements.pop() {
            match statement.form() {
                S::Declare(_, e) | S::Assign(_, _, e) | S::Eval(e) => expressions.push(e),
                S::Block(body) => statements.extend(body),
                S::If(e, a, b) => {
                    expressions.push(e);
                    statements.push(a);
                    statements.extend(b.iter().map(|s| s.as_ref()));
                }
                S::While(e, body) => {
                    expressions.push(e);
                    statements.push(body);
                }
                S::For(init, test, update, body, _) => {
                    statements.extend(init);
                    expressions.extend(test);
                    statements.extend(update.iter().map(|s| s.as_ref()));
                    statements.push(body);
                }
                S::Return(Some(e)) => expressions.push(e),
                _ => {}
            }
        }
        while let Some(expression) = expressions.pop() {
            types.add(&expression.ty);
            match &expression.kind {
                E::Unary(_, a) => expressions.push(a),
                E::Binary(_, a, b) | E::Logical(_, a, b) => {
                    expressions.push(a);
                    expressions.push(b);
                }
                E::Conditional(t, a, b) => {
                    expressions.push(t);
                    expressions.push(a);
                    expressions.push(b);
                }
                E::Call(c, args) => {
                    expressions.push(c);
                    expressions.extend(args);
                }
                E::Template(parts) => expressions.extend(parts),
                _ => {}
            }
        }
        types
    }
    fn add(&mut self, ty: &Type) {
        if !matches!(ty, Type::Function(..)) {
            return;
        }
        let signature = ty.luau();
        if !self.names.contains_key(&signature) {
            let name = format!("__ulua_fn{}", self.names.len());
            let _ = writeln!(self.declarations, "type {name} = {signature}");
            self.names.insert(signature, name);
        }
    }
    pub(super) fn luau(&self, ty: &Type) -> String {
        let signature = ty.luau();
        self.names.get(&signature).cloned().unwrap_or(signature)
    }
}
