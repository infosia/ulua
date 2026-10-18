//! The call site decides callback eligibility and length safety once.
use super::*;
use crate::ir::{Expr, ExprKind as E, Method as M, Statement as S};

impl Checker<'_> {
    pub(super) fn inline_callback(
        &mut self,
        call: &CallExpression<'_>,
        method: M,
        args: &[Expr],
    ) -> bool {
        if !matches!(method, M::ForEach | M::Map | M::Filter | M::Reduce) {
            return false;
        }
        if matches!(&call.callee, Expression::StaticMemberExpression(m) if m.optional) {
            return false;
        }
        let Some(Expression::ArrowFunctionExpression(arrow)) =
            call.arguments.first().and_then(|a| a.as_expression())
        else {
            return false;
        };
        // Arrow parameter checks reject defaults and destructuring before this decision.
        if arrow.r#async || arrow.body.as_expression().is_none() {
            return false;
        }
        let Some(Expr {
            kind: E::Closure(id),
            ..
        }) = args.first()
        else {
            return false;
        };
        // The checker appends the literal after every function in its body.
        let Some(function) = self.functions.last_mut().filter(|f| f.binding == *id) else {
            return false;
        };
        let [statement] = function.body.as_slice() else {
            return false;
        };
        let (S::Return(Some(body)) | S::Eval(body)) = statement.form() else {
            return false;
        };
        let mut safe = true;
        let mut work = vec![body];
        while let Some(e) = work.pop() {
            match &e.kind {
                E::Closure(_) => return false,
                E::Null | E::Number(_) | E::Boolean(_) | E::String(_) | E::Read(_) => {}
                E::Unary(_, a) | E::Field(a, _) | E::Length(a) => work.push(a),
                E::Binary(_, a, b) | E::Logical(_, a, b) | E::Coalesce(a, b) | E::Index(a, b) => {
                    work.extend([a.as_ref(), b.as_ref()])
                }
                E::Conditional(t, a, b) => work.extend([t.as_ref(), a.as_ref(), b.as_ref()]),
                E::Template(parts) => work.extend(parts),
                E::Call(c, args)
                | E::Method(_, _, c, args)
                | E::OptionalCall(c, _, args)
                | E::OptionalMethod(_, _, c, args) => {
                    safe = false;
                    work.push(c);
                    work.extend(args);
                }
                E::ClassCall(c, r, args, _) => {
                    safe = false;
                    work.extend([c.as_ref(), r.as_ref()]);
                    work.extend(args);
                }
                E::Object(fields) => {
                    safe = false;
                    work.extend(fields.iter().map(|(_, e)| e));
                }
                E::Array(parts) => {
                    safe = false;
                    work.extend(parts);
                }
                E::Push(a, b) | E::OptionalPush(a, b) => {
                    safe = false;
                    work.extend([a.as_ref(), b.as_ref()]);
                }
                E::Error(_, a)
                | E::InstanceOf(a, _)
                | E::ErrorField(a, _)
                | E::OptionalErrorField(a, _)
                | E::OptionalField(a, _)
                | E::OptionalLength(a)
                | E::Json(_, a)
                | E::Await(a) => {
                    safe = false;
                    work.push(a);
                }
                E::Builtin(_) => safe = false,
            }
        }
        function.inline_callback = Some(safe);
        true
    }
}
