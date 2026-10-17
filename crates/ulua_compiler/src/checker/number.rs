//! Number calls use the same private native table as Math.
use super::*;
use crate::ir::{Expr, ExprKind};
impl Checker<'_> {
    pub(super) fn number_call(
        &mut self,
        e: &Expression<'_>,
        call: &CallExpression<'_>,
        name: &str,
        receiver: Option<Expr>,
        depth: u32,
        statement: bool,
    ) -> Result<Expr, Diagnostic> {
        let format = receiver.is_some();
        let (helper, min, max, parse) = crate::number::member(name)
            .filter(|_| format == name.starts_with("to"))
            .ok_or_else(|| self.error(call.span, "C4", "Number member is rejected"))?;
        if call.optional || call.type_arguments.is_some() {
            return Err(self.unsupported(call.span));
        }
        if call.arguments.len() < min || call.arguments.len() > max {
            let message = if name == "toString" {
                "toString takes a radix"
            } else if name == "toPrecision" {
                "toPrecision takes digits"
            } else if name == "parseInt" {
                "parseInt takes a string and a radix"
            } else {
                "Number argument count is rejected"
            };
            return Err(self.error(call.span, "C4", message));
        }
        let mut args: Vec<Expr> = receiver.into_iter().collect();
        for (i, arg) in call.arguments.iter().enumerate() {
            let arg = arg
                .as_expression()
                .ok_or_else(|| self.unsupported(arg.span()))?;
            let value = self.expr(arg, depth, false)?;
            self.flow(
                &value.ty,
                Some(&if parse && i == 0 {
                    Type::String
                } else {
                    Type::Number
                }),
                arg.span(),
            )?;
            args.push(value);
        }
        let result = if format {
            Type::String
        } else if parse {
            Type::Number
        } else {
            Type::Boolean
        };
        self.math.insert(helper);
        let callee = Expr {
            flow: None,
            bare_true: false,
            start: call.span.start,
            ty: self
                .types
                .intern(args.iter().map(|a| a.ty).collect(), result, args.len()),
            kind: ExprKind::Builtin(helper),
        };
        self.finish_expr(e, statement, result, ExprKind::Call(Box::new(callee), args))
    }
}
