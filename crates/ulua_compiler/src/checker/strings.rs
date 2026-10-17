//! String calls keep receiver and argument evaluation order in the checked form.
use super::*;
use crate::{
    ir::{Expr, ExprKind, Method},
    strings::StringMethod as M,
};
impl Checker<'_> {
    pub(super) fn string_method(
        &mut self,
        call: &CallExpression<'_>,
        name: &str,
        receiver: Expr,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        if matches!(
            name,
            "toLocaleUpperCase" | "toLocaleLowerCase" | "localeCompare"
        ) {
            return Err(self.error(call.span, "C5", &format!("{name} depends on locale data")));
        }
        let method = M::parse(name)
            .ok_or_else(|| self.error(call.span, "C5", "String member is rejected"))?;
        let (min, max) = match method {
            M::Slice => (0, 2),
            M::Trim | M::TrimStart | M::TrimEnd | M::Upper | M::Lower => (0, 0),
            M::Replace | M::ReplaceAll => (2, 2),
            M::CharAt | M::CharCodeAt | M::CodePointAt | M::At | M::Repeat | M::Concat => (1, 1),
            _ => (1, 2),
        };
        if !(min..=max).contains(&call.arguments.len()) {
            return Err(self.error(call.span, "C5", "String argument count is rejected"));
        }
        let mut args = vec![];
        for (i, arg) in call.arguments.iter().enumerate() {
            let expr = arg
                .as_expression()
                .ok_or_else(|| self.error(call.span, "C5", "String spread is rejected"))?;
            if matches!(expr, Expression::RegExpLiteral(_)) {
                return Err(self.error(expr.span(), "C5", "String RegExp arguments are rejected"));
            }
            let ty = match method {
                M::Concat | M::Replace | M::ReplaceAll => Type::String,
                M::IndexOf
                | M::LastIndexOf
                | M::Includes
                | M::StartsWith
                | M::EndsWith
                | M::Split
                    if i == 0 =>
                {
                    Type::String
                }
                M::PadStart | M::PadEnd if i == 1 => Type::String,
                _ => Type::Number,
            };
            let value = self.expr_as(expr, depth, false, Some(ty))?;
            if value.ty != ty {
                return Err(self.error(expr.span(), "C5", "String argument type is rejected"));
            }
            args.push(value);
        }
        let ty = match method {
            M::IndexOf | M::LastIndexOf | M::CharCodeAt | M::CodePointAt => Type::Number,
            M::Includes | M::StartsWith | M::EndsWith => Type::Boolean,
            M::Split => self.types.array(Type::String),
            _ => Type::String,
        };
        match method {
            M::Upper => {
                self.math.insert("__ulua_case_upper");
            }
            M::Lower => {
                self.math.insert("__ulua_case_lower");
            }
            _ => {}
        }
        let method = Method::String(method);
        self.methods.insert((method, 0));
        Ok(Expr {
            flow: None,
            bare_true: false,
            start: call.span.start,
            ty,
            kind: ExprKind::Method(method, 0, Box::new(receiver), args),
        })
    }
    fn string_native(
        &mut self,
        start: u32,
        helper: &'static str,
        argument: Expr,
    ) -> Result<Expr, Diagnostic> {
        self.math.insert(helper);
        let callee = Expr {
            flow: None,
            bare_true: false,
            start,
            ty: self.types.intern(vec![argument.ty], Type::String, 1),
            kind: ExprKind::Builtin(helper),
        };
        Ok(Expr {
            flow: None,
            bare_true: false,
            start,
            ty: Type::String,
            kind: ExprKind::Call(Box::new(callee), vec![argument]),
        })
    }
    pub(super) fn string_static(
        &mut self,
        call: &CallExpression<'_>,
        name: &str,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let helper = match name {
            "fromCodePoint" => "__ulua_case_fromCodePoint",
            "fromCharCode" => "__ulua_case_fromCharCode",
            _ => return Err(self.error(call.span, "C5", "String member is rejected")),
        };
        if call.optional || call.type_arguments.is_some() {
            return Err(self.unsupported(call.span));
        }
        if call.arguments.len() != 1 {
            return Err(self.error(
                call.span,
                "C5",
                &format!("String.{name} takes exactly one argument"),
            ));
        }
        let arg = call.arguments[0]
            .as_expression()
            .ok_or_else(|| self.error(call.span, "C5", "String spread is rejected"))?;
        let value = self.expr_as(arg, depth, false, Some(Type::Number))?;
        if value.ty != Type::Number {
            return Err(self.error(arg.span(), "C5", "String argument type is rejected"));
        }
        self.string_native(call.span.start, helper, value)
    }
}
