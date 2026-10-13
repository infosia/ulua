//! Error constructors, catch scopes, and exceptional entry facts.
use super::*;
use crate::ir::{ExprKind as E, Statement as S};

pub(super) fn family(name: &str) -> Option<&'static str> {
    [
        "Error",
        "TypeError",
        "RangeError",
        "SyntaxError",
        "ReferenceError",
        "EvalError",
        "URIError",
    ]
    .into_iter()
    .find(|kind| *kind == name)
}
pub(super) fn field(name: &str) -> Option<&'static str> {
    match name {
        "name" => Some("name"),
        "message" => Some("message"),
        _ => None,
    }
}
impl Checker<'_> {
    pub(super) fn catch_value(
        &mut self,
        e: &Expression<'_>,
        depth: u32,
    ) -> Result<ir::Expr, Diagnostic> {
        let mut leaf = e;
        while let Expression::ParenthesizedExpression(p) = leaf {
            leaf = &p.expression;
        }
        let old = self.catch_use;
        self.catch_use = matches!(leaf, Expression::Identifier(_));
        let value = self.expr(e, depth, false);
        self.catch_use = old;
        value
    }
    pub(super) fn new_error(
        &mut self,
        n: &NewExpression<'_>,
        depth: u32,
    ) -> Result<Option<(Type, E)>, Diagnostic> {
        let Expression::Identifier(id) = &n.callee else {
            return Ok(None);
        };
        if self.scopes.iter().any(|s| s.contains_key(id.name.as_str())) {
            return Ok(None);
        }
        let Some(kind) = family(id.name.as_str()) else {
            return Ok(None);
        };
        self.uses_exceptions = true;
        if n.type_arguments.is_some() || n.arguments.len() > 1 {
            return Err(self.error(n.span, "C17", "Error needs zero or one string argument"));
        }
        let message = if let Some(arg) = n.arguments.first() {
            let e = arg
                .as_expression()
                .ok_or_else(|| self.error(n.span, "C17", "Spread is rejected"))?;
            let value = self.expr(e, depth, false)?;
            self.flow(&value.ty, Some(&Type::String), e.span())?;
            value
        } else {
            super::classes::value(Type::String, E::String(String::new()), n.span.start)
        };
        Ok(Some((Type::Error, E::Error(kind, Box::new(message)))))
    }
    pub(super) fn instance_of(
        &mut self,
        e: &Expression<'_>,
        b: &BinaryExpression<'_>,
        depth: u32,
        statement: bool,
    ) -> Result<ir::Expr, Diagnostic> {
        let Expression::Identifier(id) = &b.right else {
            return Err(self.error(b.span, "C11", "Instanceof needs an Error class"));
        };
        let kind = family(id.name.as_str())
            .filter(|_| !self.scopes.iter().any(|s| s.contains_key(id.name.as_str())))
            .ok_or_else(|| self.error(b.span, "C11", "Instanceof needs an Error class"))?;
        let value = self.catch_value(&b.left, depth)?;
        self.promise_operator(value.ty, b.span)?;
        if !matches!(value.ty, Type::Error | Type::Unknown) {
            return Err(self.error(b.span, "C11", "Instanceof needs an Error or catch binding"));
        }
        self.finish_expr(
            e,
            statement,
            Type::Boolean,
            E::InstanceOf(Box::new(value), kind),
        )
    }
    pub(super) fn try_statement(
        &mut self,
        t: &TryStatement<'_>,
        depth: u32,
    ) -> Result<S, Diagnostic> {
        self.uses_exceptions = true;
        let entry = self.facts.head();
        let incoming = self.reachable;
        let block_depth = self.descend(depth, t.block.span)?;
        let body = S::Block(self.block(&t.block.body, block_depth)?);
        let first_reachable = self.reachable;
        let mut join = narrowing::Join::default();
        if first_reachable {
            join.include(&self.facts, self.facts.head());
        }
        self.facts.travel(entry);
        let handler = if let Some(catch) = &t.handler {
            self.loop_writes(t.block.span);
            self.reachable = incoming;
            self.scopes.push(HashMap::new());
            let id = if let Some(p) = &catch.param {
                if p.type_annotation
                    .as_ref()
                    .is_some_and(|t| !matches!(t.type_annotation, TSType::TSUnknownKeyword(_)))
                {
                    return Err(self.error(p.span, "C17", "Catch annotation must be unknown"));
                }
                let BindingPattern::BindingIdentifier(n) = &p.pattern else {
                    return Err(self.error(p.span, "C17", "Catch needs an identifier"));
                };
                Some(self.bind(
                    n.name.as_str(),
                    ir::Binding {
                        ty: Some(Type::Unknown),
                        owner: self.owner,
                        start: p.span.start,
                        end: p.span.end,
                        mutable: false,
                        declaration: false,
                    },
                    p.span,
                )?)
            } else {
                None
            };
            let catch_depth = self.descend(depth, catch.span)?;
            let catch_body = S::Block(self.block(&catch.body.body, catch_depth)?);
            self.scopes.pop();
            if self.reachable {
                join.include(&self.facts, self.facts.head());
            }
            Some((id, Box::new(catch_body)))
        } else {
            None
        };
        let reachable = join.finish(&mut self.facts).is_some();
        let completion = self.facts.head();
        let finalizer = if let Some(finalizer) = &t.finalizer {
            self.facts.travel(entry);
            self.loop_writes(t.block.span);
            if let Some(catch) = &t.handler {
                self.loop_writes(catch.span);
            }
            self.reachable = incoming;
            let final_depth = self.descend(depth, finalizer.span)?;
            let final_body = S::Block(self.block(&finalizer.body, final_depth)?);
            self.finalizer_jumps(finalizer.span);
            let final_reachable = self.reachable;
            // The finalizer facts apply to each normal outgoing completion.
            self.reachable = reachable && final_reachable;
            Some(Box::new(final_body))
        } else {
            self.facts.travel(completion);
            self.reachable = reachable;
            None
        };
        Ok(S::Try(Box::new(body), handler, finalizer))
    }
}
