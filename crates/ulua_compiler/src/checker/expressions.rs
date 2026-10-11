//! Expression types and closed operator forms.
use super::*;
use crate::ir::{Expr, ExprKind};
impl Checker<'_> {
    pub(super) fn flow(
        &self,
        from: &Type,
        to: Option<&Type>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if !to.is_some_and(|to| from == to) {
            return Err(self.error(span, "C1", "Value types do not agree"));
        }
        Ok(())
    }
    pub(super) fn expr(
        &mut self,
        e: &Expression<'_>,
        depth: u32,
        statement: bool,
    ) -> Result<Expr, Diagnostic> {
        let depth = self.descend(depth, e.span())?;
        let (ty, kind) = match e {
            Expression::NumericLiteral(n) => {
                if strict::legacy_number(&self.source[n.span.start as usize..n.span.end as usize]) {
                    return Err(self.error(
                        n.span,
                        "C11",
                        "Legacy numeric literal in a strict module",
                    ));
                }
                (Type::Number, ExprKind::Number(n.value))
            }
            Expression::BooleanLiteral(b) => (Type::Boolean, ExprKind::Boolean(b.value)),
            Expression::StringLiteral(s) => {
                if strict::legacy_escape(&self.source[s.span.start as usize..s.span.end as usize]) {
                    return Err(self.error(
                        s.span,
                        "C11",
                        "Legacy string escape in a strict module",
                    ));
                }
                if s.lone_surrogates {
                    return Err(self.error(s.span, "C5", "Unpaired surrogate"));
                }
                (Type::String, ExprKind::String(s.value.as_str().into()))
            }
            Expression::BigIntLiteral(_) => {
                return Err(self.error(e.span(), "C4", "BigInt is rejected"));
            }
            Expression::Identifier(n) => {
                self.default_reference(n.name.as_str(), n.span)?;
                if self.scopes.iter().any(|s| s.contains_key(n.name.as_str())) {
                    let id = self.lookup(n.name.as_str(), n.span)?;
                    let ty = self.bindings[id]
                        .ty
                        .clone()
                        .ok_or_else(|| self.error(n.span, "C10", "Binding is not initialized"))?;
                    (ty, ExprKind::Read(id))
                } else {
                    match n.name.as_str() {
                        "Infinity" => (Type::Number, ExprKind::Number(f64::INFINITY)),
                        "NaN" => (Type::Number, ExprKind::Number(f64::NAN)),
                        "Utf8" => return Err(self.unsupported(n.span)),
                        _ => {
                            return Err(self.error(
                                n.span,
                                if n.name == "undefined" { "C2" } else { "C11" },
                                "Unknown value identifier",
                            ));
                        }
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => {
                let mut value = self.expr(&p.expression, depth, statement)?;
                value.bare_true = false;
                return Ok(value);
            }
            Expression::UnaryExpression(u) => {
                let op = u.operator.as_str();
                let required = match op {
                    "-" => Type::Number,
                    "!" => Type::Boolean,
                    "~" => return Err(self.error(u.span, "C4", "Bitwise operators are rejected")),
                    _ => return Err(self.error(u.span, "C8", "Unary operator has no lowering")),
                };
                let a = self.expr(&u.argument, depth, false)?;
                if a.ty != required {
                    return Err(self.error(
                        u.span,
                        if op == "!" { "C3" } else { "C4" },
                        "Invalid unary operand",
                    ));
                }
                (
                    required,
                    ExprKind::Unary(if op == "!" { "not" } else { "-" }, Box::new(a)),
                )
            }
            Expression::BinaryExpression(b) => {
                let op = b.operator.as_str();
                if matches!(op, "**" | "|" | "&" | "^" | "<<" | ">>" | ">>>") {
                    return Err(self.error(b.span, "C4", "Numeric operator has no lowering"));
                }
                let a = self.expr(&b.left, depth, false)?;
                let r = self.expr(&b.right, depth, false)?;
                let (ty, op) = match op {
                    "===" | "!==" | "==" | "!=" => {
                        self.equality_operands(b.span, &a, &r)?;
                        (
                            Type::Boolean,
                            if matches!(op, "===" | "==") {
                                "=="
                            } else {
                                "~="
                            },
                        )
                    }
                    "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" => {
                        if op == "+" && a.ty == Type::String && r.ty == Type::String {
                            return Err(self.unsupported(b.span));
                        }
                        if a.ty != Type::Number || r.ty != Type::Number {
                            return Err(self.error(b.span, "C4", "Numeric operands are required"));
                        }
                        let op = match op {
                            "+" => "+",
                            "-" => "-",
                            "*" => "*",
                            "/" => "/",
                            "%" => "%",
                            "<" => "<",
                            ">" => ">",
                            "<=" => "<=",
                            _ => ">=",
                        };
                        (
                            if matches!(op, "<" | ">" | "<=" | ">=") {
                                Type::Boolean
                            } else {
                                Type::Number
                            },
                            op,
                        )
                    }
                    _ => return Err(self.error(b.span, "C8", "Operator has no lowering")),
                };
                (ty, ExprKind::Binary(op, Box::new(a), Box::new(r)))
            }
            Expression::LogicalExpression(l) => {
                let op = l.operator.as_str();
                if op == "??" {
                    return Err(self.unsupported(l.span));
                }
                let a = self.expr(&l.left, depth, false)?;
                let [t, f] = self.equality.regions(&a);
                let (ids, skipped) = if op == "&&" { (t, f) } else { (f, t) };
                let checkpoint = self.equality.checkpoint();
                let entry = self.reachable;
                let mut join = self.equality.join();
                self.enter_region(&skipped);
                join.edge(&self.equality, self.reachable);
                self.equality.restore(checkpoint);
                self.reachable = entry;
                let b = self.edge_expr(&l.right, depth, &ids, &mut join)?;
                self.reachable = self.equality.finish(join);
                if a.ty != Type::Boolean || b.ty != Type::Boolean {
                    return Err(self.error(l.span, "C3", "Boolean operands are required"));
                }
                (
                    Type::Boolean,
                    ExprKind::Logical(op == "&&", Box::new(a), Box::new(b)),
                )
            }
            Expression::ConditionalExpression(c) => {
                let test = self.expr(&c.test, depth, false)?;
                if test.ty != Type::Boolean {
                    return Err(self.error(c.span, "C3", "Boolean condition is required"));
                }
                let [t, f] = self.equality.regions(&test);
                let mut join = self.equality.join();
                let a = self.edge_expr(&c.consequent, depth, &t, &mut join)?;
                let b = self.edge_expr(&c.alternate, depth, &f, &mut join)?;
                self.reachable = self.equality.finish(join);
                self.flow(&a.ty, Some(&b.ty), c.span)?;
                (
                    a.ty.clone(),
                    ExprKind::Conditional(Box::new(test), Box::new(a), Box::new(b)),
                )
            }
            Expression::CallExpression(call) => {
                if call.arguments.len() > crate::ARGUMENT_BOUND {
                    return Err(self.error(
                        call.span,
                        "limit:arguments",
                        "Argument limit exceeded",
                    ));
                }
                if call.optional || call.type_arguments.is_some() {
                    return Err(self.unsupported(call.span));
                }
                let callee = if let Expression::Identifier(n) = &call.callee {
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
                            wide: false,
                            bare_true: false,
                            start: call.callee.span().start,
                            ty: Type::Function(vec![p], Box::new(r), 1),
                            kind: ExprKind::Builtin(name),
                        }
                    } else {
                        self.expr(&call.callee, depth, false)?
                    }
                } else {
                    if let Expression::StaticMemberExpression(m) = &call.callee
                        && matches!(&m.object, Expression::Identifier(n) if n.name == "Math")
                    {
                        return Err(self.error(
                            m.span,
                            if m.property.name == "pow" {
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
                        Expression::StaticMemberExpression(_)
                        | Expression::ComputedMemberExpression(_) => {
                            return Err(self.unsupported(call.callee.span()));
                        }
                        _ => {
                            return Err(self.error(
                                call.callee.span(),
                                "C11",
                                "Invalid callee position",
                            ));
                        }
                    }
                };
                let Type::Function(params, result, required) = &callee.ty else {
                    return Err(self.error(
                        call.span,
                        "C11",
                        "Callee must be a function declaration",
                    ));
                };
                if call.arguments.len() < *required || call.arguments.len() > params.len() {
                    return Err(self.error(call.span, "C1", "Argument count does not agree"));
                }
                let mut args = vec![];
                for (arg, ty) in call.arguments.iter().zip(params) {
                    let a = arg.as_expression().ok_or_else(|| {
                        self.error(arg.span(), "C11", "Spread arguments are rejected")
                    })?;
                    let value = self.expr(a, depth, false)?;
                    self.flow(&value.ty, Some(ty), arg.span())?;
                    args.push(value);
                }
                (*result.clone(), ExprKind::Call(Box::new(callee), args))
            }
            Expression::TemplateLiteral(t) => {
                if t.quasis.len() + t.expressions.len() > crate::TEMPLATE_PART_BOUND {
                    return Err(self.error(
                        t.span,
                        "limit:template-parts",
                        "Template part limit exceeded",
                    ));
                }
                let mut parts = vec![];
                for (i, q) in t.quasis.iter().enumerate() {
                    if q.lone_surrogates {
                        return Err(self.error(q.span, "C5", "Unpaired surrogate"));
                    }
                    let text =
                        q.value.cooked.as_ref().ok_or_else(|| {
                            self.error(q.span, "syntax", "Invalid template escape")
                        })?;
                    parts.push(Expr {
                        wide: false,
                        bare_true: false,
                        start: q.span.start,
                        ty: Type::String,
                        kind: ExprKind::String(text.as_str().into()),
                    });
                    if let Some(e) = t.expressions.get(i) {
                        let value = self.expr(e, depth, false)?;
                        if !matches!(value.ty, Type::Number | Type::String | Type::Boolean) {
                            return Err(self.error(
                                e.span(),
                                "C4",
                                "Invalid template substitution type",
                            ));
                        }
                        parts.push(value);
                    }
                }
                (Type::String, ExprKind::Template(parts))
            }
            Expression::AssignmentExpression(_) | Expression::UpdateExpression(_) => {
                return Err(self.error(
                    e.span(),
                    "C11",
                    "Assignment and update values are rejected",
                ));
            }
            Expression::SequenceExpression(_) | Expression::TaggedTemplateExpression(_) => {
                return Err(self.error(e.span(), "C8", "Expression has no lowering"));
            }
            Expression::ArrowFunctionExpression(a) => self.arrow(a, depth)?,
            Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::NullLiteral(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::NewExpression(_)
            | Expression::ChainExpression(_)
            | Expression::ThisExpression(_) => return Err(self.unsupported(e.span())),
            _ => {
                return Err(self.error(
                    e.span(),
                    "C11",
                    "Expression is outside the closed surface",
                ));
            }
        };
        if ty == Type::Void && !statement {
            return Err(self.error(e.span(), "C2", "Void is a statement result only"));
        }
        let mut value = Expr {
            wide: false,
            bare_true: matches!(e, Expression::BooleanLiteral(b) if b.value),
            start: e.span().start,
            ty,
            kind,
        };
        value.wide = self.is_wide(e, &value);
        Ok(value)
    }
}
