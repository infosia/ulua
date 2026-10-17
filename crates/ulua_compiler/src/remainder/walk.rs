//! One postorder walk visits all expression and statement forms.
use crate::{
    Diagnostic,
    ir::{Expr, ExprKind as E, Statement as S},
};
pub(super) trait Visitor {
    fn enter_expression(&mut self) {}
    fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic>;
    fn statement(&mut self, _: &mut S) -> Result<(), Diagnostic> {
        Ok(())
    }
}
pub(super) struct Walk<'a> {
    pub(super) source: &'a [u8],
    pub(super) limit: u32,
}
impl Walk<'_> {
    fn guard(&self, depth: u32, start: usize) -> Result<u32, Diagnostic> {
        if depth >= self.limit {
            return Err(Diagnostic::at(
                self.source,
                start,
                "limit:nesting-depth",
                "Remainder walk depth limit exceeded",
            ));
        }
        Ok(depth + 1)
    }
    pub(super) fn statements(
        &self,
        body: &mut [S],
        visitor: &mut impl Visitor,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        for s in body {
            self.statement(s, visitor, depth)?;
        }
        Ok(())
    }
    pub(super) fn statement(
        &self,
        mut s: &mut S,
        visitor: &mut impl Visitor,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let mut start = 0;
        while let S::Located(offset, inner) = s {
            start = *offset as usize;
            s = inner;
        }
        let depth = self.guard(depth, start)?;
        match s {
            S::Located(..) => {}
            S::Declare(_, e)
            | S::Assign(_, _, e)
            | S::Eval(e)
            | S::Return(Some(e))
            | S::Throw(e) => self.expression(e, visitor, depth)?,
            S::Field(a, _, _, e) | S::ErrorField(a, _, _, e) => {
                self.expression(a, visitor, depth)?;
                self.expression(e, visitor, depth)?;
            }
            S::Element(a, i, _, e) => {
                self.expression(a, visitor, depth)?;
                self.expression(i, visitor, depth)?;
                self.expression(e, visitor, depth)?;
            }
            S::ForOf(_, a, b) => {
                self.expression(a, visitor, depth)?;
                // The checker adds this storage block; the emitter also walks its parts directly.
                if let S::Block(parts) = b.as_mut() {
                    self.statements(parts, visitor, depth)?;
                    visitor.statement(b)?;
                } else {
                    self.statement(b, visitor, depth)?;
                }
            }
            S::While(a, b) => {
                self.expression(a, visitor, depth)?;
                self.statement(b, visitor, depth)?;
            }
            S::Block(b) => self.statements(b, visitor, depth)?,
            S::If(e, a, b) => {
                self.expression(e, visitor, depth)?;
                self.statement(a, visitor, depth)?;
                if let Some(b) = b {
                    self.statement(b, visitor, depth)?;
                }
            }
            S::For(a, t, u, b, _) => {
                self.statements(a, visitor, depth)?;
                if let Some(t) = t {
                    self.expression(t, visitor, depth)?;
                }
                if let Some(u) = u {
                    self.statement(u, visitor, depth)?;
                }
                self.statement(b, visitor, depth)?;
            }
            S::Try(a, b, f) => {
                self.statement(a, visitor, depth)?;
                if let Some((_, b)) = b {
                    self.statement(b, visitor, depth)?;
                }
                if let Some(f) = f {
                    self.statement(f, visitor, depth)?;
                }
            }
            S::Entry(_) | S::Function(_) | S::Return(None) | S::Break | S::Continue => {}
        }
        visitor.statement(s)
    }
    pub(super) fn expression(
        &self,
        e: &mut Expr,
        visitor: &mut impl Visitor,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let depth = self.guard(depth, e.start as usize)?;
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
            | E::Length(a) => self.expression(a, visitor, depth)?,
            E::OptionalPush(a, b)
            | E::Coalesce(a, b)
            | E::Binary(_, a, b)
            | E::Logical(_, a, b)
            | E::Index(a, b)
            | E::Push(a, b) => {
                self.expression(a, visitor, depth)?;
                self.expression(b, visitor, depth)?;
            }
            E::OptionalMethod(_, _, c, args)
            | E::OptionalCall(c, _, args)
            | E::Method(_, _, c, args)
            | E::Call(c, args) => {
                self.expression(c, visitor, depth)?;
                for a in args {
                    self.expression(a, visitor, depth)?;
                }
            }
            E::ClassCall(c, r, args, _) => {
                self.expression(c, visitor, depth)?;
                self.expression(r, visitor, depth)?;
                for a in args {
                    self.expression(a, visitor, depth)?;
                }
            }
            E::Conditional(t, a, b) => {
                self.expression(t, visitor, depth)?;
                self.expression(a, visitor, depth)?;
                self.expression(b, visitor, depth)?;
            }
            E::Object(fields) => {
                for (_, a) in fields {
                    self.expression(a, visitor, depth)?;
                }
            }
            E::Template(parts) | E::Array(parts) => {
                for a in parts {
                    self.expression(a, visitor, depth)?;
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
        visitor.expression(e)
    }
}
