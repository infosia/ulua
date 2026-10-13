//! Call checks stay outside the recursive expression frame to bound the Debug stack.
use super::*;
use crate::ir::{Expr, ExprKind};
impl Checker<'_> {
    pub(super) fn call_expression(
        &mut self,
        e: &Expression<'_>,
        call: &CallExpression<'_>,
        depth: u32,
        statement: bool,
    ) -> Result<Expr, Diagnostic> {
        if matches!(call.callee, Expression::StaticMemberExpression(_)) {
            self.member_call(e, call, depth, statement)
        } else {
            self.plain_call(e, call, None, depth, statement)
        }
    }
    fn member_call(
        &mut self,
        e: &Expression<'_>,
        call: &CallExpression<'_>,
        depth: u32,
        statement: bool,
    ) -> Result<Expr, Diagnostic> {
        if let Expression::StaticMemberExpression(m) = &call.callee
            && matches!(&m.object, Expression::Identifier(n) if n.name == "Promise" && !self.scopes.iter().any(|s| s.contains_key(n.name.as_str())))
        {
            let value = self.promise_all(call, m, depth)?;
            return self.finish_expr(e, statement, value.ty, value.kind);
        }
        let mut field_callee = None;
        if let Expression::StaticMemberExpression(m) = &call.callee
            && !m.optional
            && !call.optional
            && call.type_arguments.is_none()
        {
            // Freeze an object function field before the arguments.
            if !matches!(&m.object, Expression::Identifier(n) if matches!(n.name.as_str(), "Math" | "Utf8" | "String") && !self.scopes.iter().any(|s| s.contains_key(n.name.as_str())))
            {
                let receiver = self.expr(&m.object, depth, false)?;
                if self.types.promise_value(receiver.ty).is_some() {
                    let value = self.promise_method(call, m, receiver, depth)?;
                    return self.finish_expr(e, statement, value.ty, value.kind);
                }
                self.promise_operator(receiver.ty, m.span)?;
                if receiver.ty == Type::String {
                    let method =
                        self.string_method(call, m.property.name.as_str(), receiver, depth)?;
                    return self.finish_expr(e, statement, method.ty, method.kind);
                }
                if let Some(element) = self.types.element(receiver.ty) {
                    for arg in &call.arguments {
                        if let Some(e) = arg.as_expression()
                            && m.property.name != "map"
                        {
                            self.async_array_argument(e)?;
                        }
                    }
                    if m.property.name != "push" {
                        let method = self.array_method(
                            call,
                            m.property.name.as_str(),
                            receiver,
                            element,
                            depth,
                        )?;
                        return self.finish_expr(e, statement, method.ty, method.kind);
                    }
                    if !statement {
                        return Err(self.error(call.span, "C11", "Push is a statement only"));
                    }
                    if call.arguments.len() != 1 {
                        return Err(self.error(call.span, "C1", "Push needs one argument"));
                    }
                    let arg = call.arguments[0].as_expression().ok_or_else(|| {
                        self.error(call.span, "C11", "Spread arguments are rejected")
                    })?;
                    let value = self.expr_as(arg, depth, false, Some(element))?;
                    self.promise_array_argument(value.ty, arg.span())?;
                    self.flow(&value.ty, Some(&element), arg.span())?;
                    return Ok(Expr {
                        flow: None,
                        bare_true: false,
                        start: call.span.start,
                        ty: Type::Void,
                        kind: ExprKind::Push(Box::new(receiver), Box::new(value)),
                    });
                }
                self.nullable_use(m.span, &[receiver.ty])?;
                let key = self.field_key(m.property.name.as_str());
                self.member_access(receiver.ty, key, m.span)?;
                if let Some(id) = self.method_target(receiver.ty, key) {
                    let (ty, kind) =
                        self.class_call(id, receiver, &call.arguments, false, depth, call.span)?;
                    return self.finish_expr(e, statement, ty, kind);
                }
                let ty = self
                    .types
                    .field(receiver.ty, key)
                    .ok_or_else(|| self.error(m.span, "C1", "Unknown object field"))?;
                field_callee = Some(Expr {
                    flow: None,
                    bare_true: false,
                    start: m.span.start,
                    ty,
                    kind: ExprKind::Field(Box::new(receiver), key),
                });
            }
        }
        if let Expression::StaticMemberExpression(m) = &call.callee
            && matches!(&m.object, Expression::Identifier(n) if n.name == "Utf8" && !self.scopes.iter().any(|s| s.contains_key(n.name.as_str())))
        {
            if m.property.name != "byteLength" || call.arguments.len() != 1 {
                return Err(self.error(call.span, "C11", "Unknown Utf8 call"));
            }
            let arg = call.arguments[0]
                .as_expression()
                .ok_or_else(|| self.error(call.span, "C11", "Spread is rejected"))?;
            let value = self.expr(arg, depth, false)?;
            self.flow(&value.ty, Some(&Type::String), arg.span())?;
            return self.finish_expr(
                e,
                statement,
                Type::Number,
                ExprKind::Length(Box::new(value)),
            );
        }
        self.plain_call(e, call, field_callee, depth, statement)
    }
    fn plain_call(
        &mut self,
        e: &Expression<'_>,
        call: &CallExpression<'_>,
        field_callee: Option<Expr>,
        depth: u32,
        statement: bool,
    ) -> Result<Expr, Diagnostic> {
        if matches!(&call.callee, Expression::Identifier(n) if exceptions::family(n.name.as_str()).is_some() && !self.scopes.iter().any(|s| s.contains_key(n.name.as_str())))
        {
            return Err(self.error(call.span, "C17", "Error construction needs new"));
        }
        if call.arguments.len() > crate::ARGUMENT_BOUND {
            return Err(self.error(call.span, "limit:arguments", "Argument limit exceeded"));
        }
        if call.optional || call.type_arguments.is_some() {
            return Err(self.unsupported(call.span));
        }
        let callee = if let Some(callee) = field_callee {
            callee
        } else if let Expression::Identifier(n) = &call.callee {
            self.default_reference(n.name.as_str(), n.span)?;
            if matches!(n.name.as_str(), "print" | "String")
                && !self.scopes.iter().any(|s| s.contains_key(n.name.as_str()))
            {
                let (name, p, r) = if n.name == "print" {
                    ("__ulua_print", Type::String, Type::Void)
                } else {
                    ("__ulua_format", Type::Number, Type::String)
                };
                Expr {
                    flow: None,
                    bare_true: false,
                    start: call.callee.span().start,
                    ty: self.types.intern(vec![p], r, 1),
                    kind: ExprKind::Builtin(name),
                }
            } else if self.is_host(n.name.as_str()) {
                let id = self.host_binding(n)?;
                Expr {
                    flow: None,
                    bare_true: false,
                    start: n.span.start,
                    ty: self.bindings[id]
                        .ty
                        .ok_or_else(|| self.error(n.span, "internal", "Missing host type"))?,
                    kind: ExprKind::Read(id),
                }
            } else {
                self.expr(&call.callee, depth, false)?
            }
        } else {
            if let Expression::StaticMemberExpression(m) = &call.callee
                && matches!(&m.object, Expression::Identifier(n) if matches!(n.name.as_str(), "Math" | "String"))
            {
                return Err(self.error(
                    m.span,
                    if matches!(&m.object, Expression::Identifier(n) if n.name == "String") {
                        "C5"
                    } else if m.property.name == "pow" {
                        "C4"
                    } else {
                        "C11"
                    },
                    "Math member has no lowering",
                ));
            }
            match &call.callee {
                Expression::ParenthesizedExpression(p)
                    if matches!(p.expression, Expression::ArrowFunctionExpression(_)) =>
                {
                    self.expr(&call.callee, depth, false)?
                }
                Expression::StaticMemberExpression(_) => self.expr(&call.callee, depth, false)?,
                Expression::ComputedMemberExpression(_) => {
                    return Err(self.unsupported(call.callee.span()));
                }
                _ => {
                    return Err(self.error(call.callee.span(), "C11", "Invalid callee position"));
                }
            }
        };
        self.nullable_use(call.span, &[callee.ty])?;
        let Some(signature) = self.types.signature(callee.ty) else {
            return Err(self.error(call.span, "C11", "Callee must be a function declaration"));
        };
        let ir::FunctionType {
            params,
            result,
            required,
        } = signature.as_ref();
        if call.arguments.len() < *required || call.arguments.len() > params.len() {
            return Err(self.error(call.span, "C1", "Argument count does not agree"));
        }
        let mut args = vec![];
        for (arg, ty) in call.arguments.iter().zip(params) {
            let a = arg
                .as_expression()
                .ok_or_else(|| self.error(arg.span(), "C11", "Spread arguments are rejected"))?;
            let value = self.expr_as(a, depth, false, Some(*ty))?;
            self.flow(&value.ty, Some(ty), arg.span())?;
            args.push(value);
        }
        self.finish_expr(
            e,
            statement,
            *result,
            ExprKind::Call(Box::new(callee), args),
        )
    }
}
