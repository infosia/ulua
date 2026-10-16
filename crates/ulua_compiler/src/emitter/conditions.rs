//! Condition chains retain short-circuit order and bound each inline refinement graph.
use super::*;
use crate::ir::{Expr, ExprKind as E};
use operands::{Operand, Operation};

impl Emitter<'_> {
    // A final call needs no preparation when its callee and arguments are single inline operands.
    fn condition_nodes(&self, expr: &Expr, last: bool, nodes: usize) -> Option<usize> {
        let remaining = nodes.checked_sub(1)?;
        match &expr.kind {
            E::Logical(_, a, b) => {
                let remaining = self.condition_nodes(a, false, remaining)?;
                self.condition_nodes(b, last, remaining)
            }
            E::Unary("not", a) => self.condition_nodes(a, last, remaining),
            E::Binary("<" | "<=" | ">" | ">=" | "==" | "~=", a, b)
                if a.ty == b.ty && matches!(a.ty, Type::Number | Type::String | Type::Boolean)
                    || (matches!(a.ty, Type::Null) || matches!(b.ty, Type::Null)) =>
            {
                let remaining = self.inline_tree(a, false, remaining)?;
                self.inline_tree(b, false, remaining)
            }
            E::Call(callee, args) if last => {
                let remaining = self.inline_tree(callee, false, remaining)?;
                // Nested calls need snapshots. The ordinary call emitter supplies those statements.
                args.iter().try_fold(remaining, |remaining, arg| {
                    self.inline_tree(arg, false, 1)?;
                    remaining.checked_sub(1)
                })
            }
            E::Boolean(_) | E::Read(_) if expr.ty == Type::Boolean => {
                self.inline_tree(expr, false, nodes)
            }
            _ => None,
        }
    }

    fn condition_text(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        Ok(match &expr.kind {
            E::Logical(and, a, b) => format!(
                "({} {} {})",
                self.condition_text(a)?,
                if *and { "and" } else { "or" },
                self.condition_text(b)?
            ),
            E::Unary("not", a) => format!("(not {})", self.condition_text(a)?),
            E::Call(callee, args) => {
                let callee = self.pure_operand(callee)?;
                let args = args
                    .iter()
                    .map(|arg| self.pure_operand(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                format!("{callee}({})", args.join(", "))
            }
            _ => self.pure_operand(expr)?,
        })
    }

    pub(super) fn condition(&mut self, expr: &Expr, depth: u32) -> Result<Operand, Diagnostic> {
        if !self.inline_enabled {
            return self.comparison(expr, depth);
        }
        if let Some(remaining) = self.condition_nodes(expr, true, crate::CONDITION_NODES) {
            let text = self.condition_text(expr)?;
            return Ok(Operand::Inline(Operation {
                text,
                cost: crate::CONDITION_NODES - remaining,
            }));
        }
        let E::Logical(and, a, b) = &expr.kind else {
            return self.comparison(expr, depth);
        };
        let depth = self.guard(depth)?;
        let dest = self.slot(&Type::Boolean)?;
        let a = self.condition(a, depth)?;
        // Fresh locals break refinement aliases at each split, as in the value form.
        let fresh = self.generated()?;
        self.line("do");
        self.line_weighted(
            format!("local {fresh}: boolean = {}", self.operand_text(&a)),
            self.operand_cost(&a),
        );
        self.write_slot(dest, &fresh);
        self.line("end");
        self.generated_live -= 1;
        self.release_operand(a);
        self.line(format!(
            "if {}{} then",
            if *and { "" } else { "not " },
            self.temp(dest)
        ));
        self.indent += 1;
        let b = self.condition(b, depth)?;
        self.write_weighted(
            &self.temp(dest),
            self.operand_text(&b),
            "boolean",
            self.operand_cost(&b),
        );
        self.release_operand(b);
        self.indent -= 1;
        self.line("end");
        Ok(Operand::Temporary(dest))
    }
}
