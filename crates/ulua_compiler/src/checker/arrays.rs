//! Array iteration uses one receiver and a fresh constant binding per step.
use super::*;
use crate::ir::{Expr, Statement as S};
impl Checker<'_> {
    pub(super) fn for_of(&mut self, f: &ForOfStatement<'_>, depth: u32) -> Result<S, Diagnostic> {
        let ForStatementLeft::VariableDeclaration(d) = &f.left else {
            return Err(self.error(f.span, "C11", "For-of requires a const declaration"));
        };
        if f.r#await {
            return Err(self.error(f.span, "C19", "for await is rejected"));
        }
        if d.kind != VariableDeclarationKind::Const || d.declare || d.declarations.len() != 1 {
            return Err(self.error(f.span, "C11", "Invalid for-of declaration"));
        }
        let v = &d.declarations[0];
        let BindingPattern::BindingIdentifier(n) = &v.id else {
            return Err(self.error(v.span, "C11", "For-of requires an identifier"));
        };
        if v.init.is_some() || v.definite || v.type_annotation.is_some() {
            return Err(self.error(v.span, "C11", "Invalid for-of binding"));
        }
        // The iteration binding also shadows outer names in the receiver's TDZ.
        self.scopes.push(HashMap::new());
        let id = self.bind(
            n.name.as_str(),
            ir::Binding {
                ty: None,
                owner: self.owner,
                start: v.span.start,
                end: f.right.span().end,
                mutable: false,
                declaration: false,
            },
            v.span,
        )?;
        let mut view = None;
        let receiver = if let Expression::CallExpression(call) = &f.right
            && let Expression::StaticMemberExpression(m) = &call.callee
            && matches!(m.property.name.as_str(), "keys" | "values")
        {
            let receiver = self.expr(&m.object, depth, false)?;
            if self
                .types
                .table_parts(receiver.ty)
                .is_some_and(|(_, v)| v.is_some())
                && !m.optional
                && !call.optional
                && call.arguments.is_empty()
                && call.type_arguments.is_none()
            {
                view = Some(m.property.name == "values");
                receiver
            } else {
                return Err(self.error(
                    f.span,
                    "C6",
                    "keys or values require a Map for-of subject",
                ));
            }
        } else {
            self.expr(&f.right, depth, false)?
        };
        let table = self.types.table_parts(receiver.ty);
        if table.is_some_and(|(_, v)| v.is_some()) && view.is_none() {
            return Err(self.error(
                f.span,
                "C6",
                "for...of over a Map needs a tuple type; iterate keys() or values()",
            ));
        }
        let mut table_binding = None;
        if table.is_some() {
            // Internal storage has no source name and cannot shadow the iteration binding.
            let binding = self.bindings.len();
            self.bindings.push(ir::Binding {
                ty: Some(receiver.ty),
                owner: self.owner,
                start: f.span.start,
                end: f.span.end,
                mutable: false,
                declaration: false,
            });
            self.facts.mutable.push(false);
            self.facts.owners.push(self.owner);
            table_binding = Some(binding);
        }
        let element = if let Some((key, value)) = table {
            if view == Some(true) {
                value.ok_or_else(|| self.error(f.span, "internal", "Missing Map value type"))?
            } else {
                key
            }
        } else if receiver.ty == Type::String {
            self.methods
                .insert((ir::Method::String(crate::strings::StringMethod::CharAt), 0));
            self.methods
                .insert((ir::Method::String(crate::strings::StringMethod::Next), 0));
            Type::String
        } else {
            self.types
                .element(receiver.ty)
                .ok_or_else(|| self.error(f.span, "C6", "For-of requires an array or string"))?
        };
        self.bindings[id].ty = Some(element);
        self.loop_writes(f.body.span());
        self.enter_loop();
        self.loops += 1;
        let body = self.statement(&f.body, depth)?;
        self.loops -= 1;
        let zero_or_exhausted = Expr {
            flow: None,
            bare_true: false,
            start: f.span.start,
            ty: Type::Boolean,
            kind: crate::ir::ExprKind::Boolean(false),
        };
        self.leave_loop(Some(&zero_or_exhausted));
        self.scopes.pop();
        // Entry makes captured storage local to each execution of the loop body.
        let body = S::Block(vec![S::Entry(vec![id]), body]);
        let statement = if let Some(binding) = table_binding {
            use crate::{ir::ExprKind as E, tables::TableMethod as M};
            self.uses_exceptions = true;
            let ty = receiver.ty;
            let read = || super::classes::value(ty, E::Read(binding), f.span.start);
            let method = |m| {
                super::classes::value(
                    Type::Void,
                    E::Method(ir::Method::Table(m), 0, Box::new(read()), vec![]),
                    f.span.start,
                )
            };
            for m in [M::Begin, M::End, M::Keys] {
                self.methods.insert((ir::Method::Table(m), 0));
            }
            let subject = super::classes::value(
                ty,
                E::Method(
                    ir::Method::Table(if view == Some(true) {
                        M::Values
                    } else {
                        M::Keys
                    }),
                    0,
                    Box::new(read()),
                    vec![],
                ),
                f.span.start,
            );
            S::Block(vec![
                S::Declare(binding, receiver),
                S::Eval(method(M::Begin)),
                S::Try(
                    Box::new(S::ForOf(id, subject, Box::new(body))),
                    None,
                    Some(Box::new(S::Eval(method(M::End)))),
                ),
            ])
        } else {
            S::ForOf(id, receiver, Box::new(body))
        };
        Ok(S::Located(f.span.start, Box::new(statement)))
    }
}
