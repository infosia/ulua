//! One postorder walk visits all expression and statement forms.
use crate::ir::{Expr, ExprKind as E, Statement as S};
pub(super) trait Visitor {
    fn enter_expression(&mut self) {}
    fn expression(&mut self, e: &mut Expr);
    fn statement(&mut self, _: &mut S) {}
}
pub(super) fn statements(body: &mut [S], visitor: &mut impl Visitor) {
    for s in body {
        statement(s, visitor);
    }
}
pub(super) fn statement(s: &mut S, visitor: &mut impl Visitor) {
    match s {
        S::Located(_, s) => statement(s, visitor),
        S::Declare(_, e) | S::Assign(_, _, e) | S::Eval(e) | S::Return(Some(e)) | S::Throw(e) => {
            expression(e, visitor)
        }
        S::Field(a, _, _, e) | S::ErrorField(a, _, _, e) => {
            expression(a, visitor);
            expression(e, visitor);
        }
        S::Element(a, i, _, e) => {
            expression(a, visitor);
            expression(i, visitor);
            expression(e, visitor);
        }
        S::ForOf(_, a, b) | S::While(a, b) => {
            expression(a, visitor);
            statement(b, visitor);
        }
        S::Block(b) => statements(b, visitor),
        S::If(e, a, b) => {
            expression(e, visitor);
            statement(a, visitor);
            if let Some(b) = b {
                statement(b, visitor);
            }
        }
        S::For(a, t, u, b, _) => {
            statements(a, visitor);
            if let Some(t) = t {
                expression(t, visitor);
            }
            if let Some(u) = u {
                statement(u, visitor);
            }
            statement(b, visitor);
        }
        S::Try(a, b, f) => {
            statement(a, visitor);
            if let Some((_, b)) = b {
                statement(b, visitor);
            }
            if let Some(f) = f {
                statement(f, visitor);
            }
        }
        S::Entry(_) | S::Function(_) | S::Return(None) | S::Break | S::Continue => {}
    }
    visitor.statement(s);
}
pub(super) fn expression(e: &mut Expr, visitor: &mut impl Visitor) {
    visitor.enter_expression();
    match &mut e.kind {
        E::Error(_, a)
        | E::InstanceOf(a, _)
        | E::ErrorField(a, _)
        | E::OptionalErrorField(a, _)
        | E::Field(a, _)
        | E::OptionalField(a, _)
        | E::OptionalLength(a)
        | E::Await(a)
        | E::Unary(_, a)
        | E::Length(a) => expression(a, visitor),
        E::OptionalPush(a, b)
        | E::Coalesce(a, b)
        | E::Binary(_, a, b)
        | E::Logical(_, a, b)
        | E::Index(a, b)
        | E::Push(a, b) => {
            expression(a, visitor);
            expression(b, visitor);
        }
        E::OptionalMethod(_, _, c, args)
        | E::OptionalCall(c, _, args)
        | E::Method(_, _, c, args)
        | E::Call(c, args) => {
            expression(c, visitor);
            for a in args {
                expression(a, visitor);
            }
        }
        E::ClassCall(c, r, args, _) => {
            expression(c, visitor);
            expression(r, visitor);
            for a in args {
                expression(a, visitor);
            }
        }
        E::Conditional(t, a, b) => {
            expression(t, visitor);
            expression(a, visitor);
            expression(b, visitor);
        }
        E::Object(fields) => {
            for (_, a) in fields {
                expression(a, visitor);
            }
        }
        E::Template(parts) | E::Array(parts) => {
            for a in parts {
                expression(a, visitor);
            }
        }
        E::Null
        | E::Number(_)
        | E::Boolean(_)
        | E::String(_)
        | E::Read(_)
        | E::Closure(_)
        | E::Builtin(_) => {}
    }
    visitor.expression(e);
}
