//! C7 classes retain constant values and defer assigned-parameter classification.
use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Constant {
    Number(f64),
    Boolean(bool),
    String(usize),
}
#[derive(Clone, Copy)]
pub(super) enum Operand {
    Constant(Constant),
    Narrowable,
    Other,
    Binding(usize),
}
impl Checker<'_> {
    pub(super) fn operand(&mut self, source: &Expression<'_>, value: &ir::Expr) -> Operand {
        let mut source = source;
        while let Expression::ParenthesizedExpression(p) = source {
            source = &p.expression;
        }
        match source {
            Expression::NumericLiteral(n) => Operand::Constant(Constant::Number(n.value)),
            Expression::BooleanLiteral(b) => Operand::Constant(Constant::Boolean(b.value)),
            Expression::StringLiteral(s) => {
                // Intern bytes once. Repeated comparisons of long constants remain constant-time.
                let next = self.constant_strings.len();
                let id = *self
                    .constant_strings
                    .entry(s.value.as_str().into())
                    .or_insert(next);
                Operand::Constant(Constant::String(id))
            }
            Expression::UnaryExpression(u) if u.operator.as_str() == "-" => {
                let mut argument = &u.argument;
                while let Expression::ParenthesizedExpression(p) = argument {
                    argument = &p.expression;
                }
                if let Expression::NumericLiteral(n) = argument {
                    Operand::Constant(Constant::Number(-n.value))
                } else {
                    Operand::Other
                }
            }
            Expression::Identifier(_) => match value.kind {
                ir::ExprKind::Read(id) => Operand::Binding(id),
                _ => Operand::Other,
            },
            _ => Operand::Other,
        }
    }
    pub(super) fn resolve_operand(&self, operand: Operand) -> Operand {
        match operand {
            Operand::Binding(id) => self.operand_classes[id],
            _ => operand,
        }
    }
    pub(super) fn check_equalities(&self) -> Result<(), Diagnostic> {
        for &(span, left, right) in &self.equalities {
            let left = self.resolve_operand(left);
            let right = self.resolve_operand(right);
            if matches!(left, Operand::Other) || matches!(right, Operand::Other) {
                continue;
            }
            if let (Operand::Constant(a), Operand::Constant(b)) = (left, right)
                && a == b
            {
                continue;
            }
            return Err(self.error(
                span,
                "C7",
                "Equality requires an other operand or equal constants",
            ));
        }
        Ok(())
    }
}
