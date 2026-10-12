//! C7 compares one primitive type or arrays with identical element types.
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
            if matches!(operand, Expression::ArrayExpression(_)) {
                return Err(self.error(span, "C7", "Array literal equality operands are rejected"));
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
        if a.ty == b.ty
            && (matches!(a.ty, Type::Number | Type::String | Type::Boolean)
                || self.types.element(a.ty).is_some())
        {
            return Ok(());
        }
        Err(self.error(
            span,
            "C7",
            "Equality requires one primitive type or identical array types",
        ))
    }
}
