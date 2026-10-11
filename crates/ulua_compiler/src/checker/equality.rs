//! C7 structural operands and scoped tested references. No literal types are modeled.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement as S};

pub(super) struct Equality {
    pub(super) bindings: Vec<bool>,
    pub(super) tested: Vec<bool>,
    undo: Vec<usize>,
}
impl Equality {
    pub(super) fn new() -> Self {
        Self {
            bindings: vec![],
            tested: vec![],
            undo: vec![],
        }
    }
    pub(super) fn checkpoint(&self) -> usize {
        self.undo.len()
    }
    pub(super) fn restore(&mut self, checkpoint: usize) {
        while self.undo.len() > checkpoint {
            if let Some(id) = self.undo.pop() {
                self.tested[id] = false;
            }
        }
    }
    pub(super) fn enter(&mut self, ids: &[usize]) {
        for &id in ids {
            if !self.tested[id] {
                self.tested[id] = true;
                self.undo.push(id);
            }
        }
    }
    // Union uses a work list. No region copies the active tested set.
    pub(super) fn region(&self, condition: &Expr, truth: bool) -> Vec<usize> {
        let mut work = vec![(condition, truth)];
        let mut ids = vec![];
        while let Some((e, truth)) = work.pop() {
            match &e.kind {
                E::Unary("not", g) => work.push((g, !truth)),
                E::Logical(_, a, b) => {
                    work.push((a, truth));
                    work.push((b, truth));
                }
                E::Binary(op, a, b) if (*op == "==" && truth) || (*op == "~=" && !truth) => {
                    for (reference, other) in [(a, b), (b, a)] {
                        if let E::Read(id) = reference.kind
                            && !other.wide
                        {
                            ids.push(id);
                        }
                    }
                }
                _ => {}
            }
        }
        ids
    }
}
impl Checker<'_> {
    pub(super) fn is_wide(&self, source: &Expression<'_>, value: &Expr) -> bool {
        match source {
            Expression::CallExpression(_) => true,
            Expression::BinaryExpression(b) => {
                matches!(b.operator.as_str(), "+" | "-" | "*" | "/" | "%")
            }
            Expression::UnaryExpression(u) if u.operator.as_str() == "-" => {
                let mut argument = &u.argument;
                while let Expression::ParenthesizedExpression(p) = argument {
                    argument = &p.expression;
                }
                !matches!(argument, Expression::NumericLiteral(_))
            }
            Expression::Identifier(_) => match value.kind {
                E::Read(id) => self.equality.bindings[id] && !self.equality.tested[id],
                _ => false,
            },
            _ => false,
        }
    }
    pub(super) fn equality_operands(
        &self,
        span: Span,
        a: &Expr,
        b: &Expr,
    ) -> Result<(), Diagnostic> {
        if a.ty != b.ty || matches!(a.ty, Type::Function(..)) {
            return Err(self.error(span, "C7", "Equality requires identical value types"));
        }
        if a.ty == Type::Boolean {
            return Err(self.error(span, "C7", "Boolean equality operands are rejected"));
        }
        if [a, b]
            .iter()
            .any(|e| matches!(e.kind, E::Number(n) if n.is_nan()))
        {
            return Err(self.error(span, "C7", "NaN equality operands are rejected"));
        }
        if !a.wide && !b.wide {
            return Err(self.error(span, "C7", "An equality needs a wide operand"));
        }
        Ok(())
    }
    pub(super) fn region_statement(
        &mut self,
        s: &Statement<'_>,
        depth: u32,
        ids: &[usize],
    ) -> Result<S, Diagnostic> {
        let checkpoint = self.equality.checkpoint();
        self.equality.enter(ids);
        let out = self.statement(s, depth);
        self.equality.restore(checkpoint);
        out
    }
    pub(super) fn region_expr(
        &mut self,
        e: &Expression<'_>,
        depth: u32,
        ids: &[usize],
    ) -> Result<Expr, Diagnostic> {
        let checkpoint = self.equality.checkpoint();
        self.equality.enter(ids);
        let out = self.expr(e, depth, false);
        self.equality.restore(checkpoint);
        out
    }
}

pub(super) fn holds_exit(statement: &S) -> bool {
    let mut pending = vec![statement];
    while let Some(s) = pending.pop() {
        match s.form() {
            S::Return(_) | S::Break | S::Continue => return true,
            S::Block(body) => pending.extend(body),
            S::If(_, a, b) => {
                pending.push(a);
                pending.extend(b.iter().map(|s| s.as_ref()));
            }
            S::While(_, body) | S::For(_, _, _, body, _) => pending.push(body),
            _ => {}
        }
    }
    false
}
