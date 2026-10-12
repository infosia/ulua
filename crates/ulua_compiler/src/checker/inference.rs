//! D18 checks only forms that can preserve a narrowed reference type.
use super::*;

impl Checker<'_> {
    pub(super) fn inferred_reference(&self, e: &Expression<'_>) -> Result<(), Diagnostic> {
        if reference_form(e) {
            return Err(self.error(
                e.span(),
                "C11",
                "An inferred type cannot come from a reference; add a type annotation",
            ));
        }
        Ok(())
    }
}

// The depth check bounds this walk before the checker starts.
fn reference_form(e: &Expression<'_>) -> bool {
    match e {
        Expression::Identifier(_) => true,
        Expression::ParenthesizedExpression(p) => reference_form(&p.expression),
        Expression::UnaryExpression(u) if u.operator.as_str() == "!" => reference_form(&u.argument),
        Expression::ConditionalExpression(c) => {
            reference_form(&c.consequent) || reference_form(&c.alternate)
        }
        Expression::LogicalExpression(l) if matches!(l.operator.as_str(), "&&" | "||") => {
            reference_form(&l.left) || reference_form(&l.right)
        }
        _ => false,
    }
}
