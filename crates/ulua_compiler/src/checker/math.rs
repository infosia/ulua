//! Closed Math calls retain numeric arguments and fixed arities.
use super::*;
use crate::ir::{Expr, ExprKind};
impl Checker<'_> {
    pub(super) fn math_call(
        &mut self,
        e: &Expression<'_>,
        call: &CallExpression<'_>,
        member: &StaticMemberExpression<'_>,
        depth: u32,
        statement: bool,
    ) -> Result<Expr, Diagnostic> {
        if member.optional || call.optional || call.type_arguments.is_some() {
            return Err(self.unsupported(call.span));
        }
        let (name, arity) = crate::math::member(member.property.name.as_str())
            .ok_or_else(|| self.error(member.span, "C11", "Unknown Math member"))?;
        if call.arguments.len() != arity {
            let count = match arity {
                0 => "no arguments",
                1 => "exactly one argument",
                _ => "exactly two arguments",
            };
            return Err(self.error(
                call.span,
                "C4",
                &format!("Math.{} takes {count}", member.property.name),
            ));
        }
        let mut args = Vec::with_capacity(arity);
        for arg in &call.arguments {
            let arg = arg
                .as_expression()
                .ok_or_else(|| self.unsupported(arg.span()))?;
            let value = self.expr_as(arg, depth, false, Some(Type::Number))?;
            self.flow(&value.ty, Some(&Type::Number), arg.span())?;
            args.push(value);
        }
        self.math.insert(name);
        let callee = Expr {
            flow: None,
            bare_true: false,
            start: member.span.start,
            ty: self
                .types
                .intern(vec![Type::Number; arity], Type::Number, arity),
            kind: ExprKind::Builtin(name),
        };
        self.finish_expr(
            e,
            statement,
            Type::Number,
            ExprKind::Call(Box::new(callee), args),
        )
    }
}
