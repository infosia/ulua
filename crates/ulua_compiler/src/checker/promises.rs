//! Promise callback signatures and homogeneous all inputs.
use super::*;
use crate::ir::{Expr, ExprKind as E};

impl Checker<'_> {
    // Unknown formal parameters have the same restricted uses as catch bindings.
    pub(super) fn parameter_type(
        &mut self,
        t: &TSType<'_>,
        depth: u32,
    ) -> Result<Type, Diagnostic> {
        if matches!(unparen(t), TSType::TSUnknownKeyword(_)) {
            Ok(Type::Unknown)
        } else {
            self.ty(t, false, depth)
        }
    }
    fn promise_argument<'a>(
        &self,
        call: &'a CallExpression<'a>,
    ) -> Result<&'a Expression<'a>, Diagnostic> {
        if call.optional || call.type_arguments.is_some() || call.arguments.len() != 1 {
            return Err(self.error(call.span, "C19", "A promise call takes one argument"));
        }
        call.arguments[0]
            .as_expression()
            .ok_or_else(|| self.error(call.span, "C19", "Spread promise arguments are rejected"))
    }
    fn promise_call(&mut self, name: &'static str, args: Vec<Expr>, ty: Type, start: u32) -> Expr {
        self.uses_async = true;
        let signature = self
            .types
            .intern(args.iter().map(|a| a.ty).collect(), ty, args.len());
        let callee = super::classes::value(signature, E::Builtin(name), start);
        super::classes::value(ty, E::Call(Box::new(callee), args), start)
    }
    pub(super) fn promise_method(
        &mut self,
        call: &CallExpression<'_>,
        member: &StaticMemberExpression<'_>,
        receiver: Expr,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let name = member.property.name.as_str();
        if member.optional || !matches!(name, "then" | "catch" | "finally") {
            return Err(self.error(
                member.span,
                "C19",
                "A promise member is only a callback callee",
            ));
        }
        let arg = self.promise_argument(call)?;
        let mut leaf = arg;
        while let Expression::ParenthesizedExpression(p) = leaf {
            leaf = &p.expression;
        }
        if name == "catch"
            && let Expression::ArrowFunctionExpression(a) = leaf
            && a.params.items.iter().any(|p| {
                !p.type_annotation.as_ref().is_some_and(|t| {
                    matches!(unparen(&t.type_annotation), TSType::TSUnknownKeyword(_))
                })
            })
        {
            return Err(self.error(
                arg.span(),
                "C19",
                "A catch callback parameter is annotated unknown",
            ));
        }
        let callback = self.expr(arg, depth, false)?;
        let signature = self
            .types
            .signature(callback.ty)
            .ok_or_else(|| self.error(arg.span(), "C19", "A promise callback is a function"))?;
        let source = self
            .types
            .promise_value(receiver.ty)
            .ok_or_else(|| self.error(call.span, "internal", "Missing promise source"))?;
        let arity = signature.params.len();
        if arity > usize::from(name != "finally") {
            return Err(self.error(
                arg.span(),
                "C19",
                "Promise callback parameter count is rejected",
            ));
        }
        if let Some(parameter) = signature.params.first() {
            let expected = if name == "catch" {
                Type::Unknown
            } else {
                source
            };
            if !self.types.equal(*parameter, expected) {
                return Err(self.error(
                    arg.span(),
                    "C19",
                    "Promise callback parameter type is rejected",
                ));
            }
        }
        let result = self
            .types
            .promise_value(signature.result)
            .unwrap_or(signature.result);
        self.promise_member(result, arg.span())?;
        if (name == "catch" && !self.types.equal(result, source))
            || (name == "finally" && result != Type::Void)
            || result == Type::Unknown
        {
            return Err(self.error(
                arg.span(),
                "C19",
                "Promise callback result type is rejected",
            ));
        }
        let ty = self
            .types
            .promise(if name == "then" { result } else { source });
        self.checked_type_depth(ty, call.span)?;
        let helper = match (name, arity) {
            ("then", 0) if signature.result == Type::Void => "__ulua_on_then0v",
            ("then", _) if signature.result == Type::Void => "__ulua_on_then1v",
            ("catch", 0) if signature.result == Type::Void => "__ulua_on_catch0v",
            ("catch", _) if signature.result == Type::Void => "__ulua_on_catch1v",
            ("finally", _) if signature.result == Type::Void => "__ulua_on_finallyv",
            ("then", 0) => "__ulua_on_then0",
            ("then", _) => "__ulua_on_then1",
            ("catch", 0) => "__ulua_on_catch0",
            ("catch", _) => "__ulua_on_catch1",
            _ => "__ulua_on_finally",
        };
        Ok(self.promise_call(helper, vec![receiver, callback], ty, call.span.start))
    }
    pub(super) fn promise_all(
        &mut self,
        call: &CallExpression<'_>,
        member: &StaticMemberExpression<'_>,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        if member.property.name != "all" || member.optional {
            return Err(self.error(member.span, "C19", "The Promise identifier is not a value"));
        }
        let arg = self.promise_argument(call)?;
        let values = self.expr(arg, depth, false)?;
        let element = self
            .types
            .element(values.ty)
            .and_then(|t| self.types.promise_value(t))
            .ok_or_else(|| {
                self.error(
                    arg.span(),
                    "C19",
                    "Promise.all needs an array of one promise type",
                )
            })?;
        if element == Type::Void || self.types.contains_null(element) {
            return Err(self.error(
                arg.span(),
                "C19",
                "Promise.all needs a promise type that is not nullable or void",
            ));
        }
        let array = self.types.array(element);
        let ty = self.types.promise(array);
        self.checked_type_depth(ty, call.span)?;
        Ok(self.promise_call("__ulua_all", vec![values], ty, call.span.start))
    }
}
