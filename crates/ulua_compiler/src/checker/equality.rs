//! C7 compares one primitive type, identical arrays, or identical object types.
use super::*;
use crate::ir::Expr;

impl Checker<'_> {
    pub(super) fn equality_syntax(
        &self,
        span: Span,
        operands: [&Expression<'_>; 2],
    ) -> Result<(), Diagnostic> {
        for mut operand in operands {
            while let Expression::ParenthesizedExpression(p) = operand {
                operand = &p.expression;
            }
            if matches!(
                operand,
                Expression::ArrayExpression(_) | Expression::ObjectExpression(_)
            ) {
                return Err(self.error(
                    span,
                    "C7",
                    "Array and object literal equality operands are rejected",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn equality_operands(
        &self,
        span: Span,
        a: &Expr,
        b: &Expr,
    ) -> Result<(), Diagnostic> {
        if matches!(a.kind, ir::ExprKind::Null) || matches!(b.kind, ir::ExprKind::Null) {
            let other = if matches!(a.kind, ir::ExprKind::Null) {
                b
            } else {
                a
            };
            if self.types.inner(self.original_type(other)).is_some() {
                return Ok(());
            }
            return Err(self.error(span, "C2", "Null test requires a nullable operand"));
        }
        if self.types.equal(a.ty, b.ty)
            && (matches!(
                a.ty,
                Type::Number | Type::String | Type::Boolean | Type::Error
            ) || self.types.element(a.ty).is_some()
                || self.types.fields(a.ty).is_some())
        {
            return Ok(());
        }
        Err(self.error(
            span,
            "C7",
            "Equality requires one primitive type, identical array types, or identical object types",
        ))
    }
}
