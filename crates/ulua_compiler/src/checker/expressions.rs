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
        if !to.is_some_and(|to| self.types.assignable(*from, *to)) {
            if self.types.promise_value(*from).is_some()
                || to.is_some_and(|t| self.types.promise_value(*t).is_some())
            {
                return Err(self.error(
                    span,
                    "C19",
                    "a promise relates only to an identical promise type",
                ));
            }
            if self.types.contains_null(*from)
                && to.is_some_and(|ty| !self.types.contains_null(*ty))
            {
                return Err(self.error(span, "C16", "Nullable value needs a stable null test"));
            }
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
        self.expr_as(e, depth, statement, None)
    }
    pub(super) fn expr_as(
        &mut self,
        e: &Expression<'_>,
        depth: u32,
        statement: bool,
        expected: Option<Type>,
    ) -> Result<Expr, Diagnostic> {
        self.host_value_form(e)?;
        if matches!(e, Expression::Identifier(n) if n.name == "Promise") {
            return Err(self.error(e.span(), "C19", "the Promise identifier is not a value"));
        }
        let depth = self.descend(depth, e.span())?;
        let (ty, kind) = match e {
            Expression::AwaitExpression(a) => self.await_expression(a, depth)?,
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
                    if self.class_bindings.contains(&id) {
                        return Err(self.error(
                            n.span,
                            "C11",
                            "A class identifier only precedes new arguments",
                        ));
                    }
                    let ty = self.bindings[id]
                        .ty
                        .ok_or_else(|| self.error(n.span, "C10", "Binding is not initialized"))?;
                    let ty = self.read_type(id, ty, n.span.start);
                    if ty == Type::Unknown && !self.catch_use {
                        return Err(self.error(
                            n.span,
                            "C17",
                            "Catch binding needs an instanceof test",
                        ));
                    }
                    (ty, ExprKind::Read(id))
                } else {
                    match n.name.as_str() {
                        "Infinity" => (Type::Number, ExprKind::Number(f64::INFINITY)),
                        "NaN" => (Type::Number, ExprKind::Number(f64::NAN)),
                        "Utf8" => return Err(self.unsupported(n.span)),
                        _ => {
                            return Err(self.error(
                                n.span,
                                if n.name == "undefined" {
                                    "C2"
                                } else if hosts::known_global(n.name.as_str()) {
                                    "C11"
                                } else {
                                    "C18"
                                },
                                "Unknown value identifier",
                            ));
                        }
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => {
                let mut value = self.expr_as(&p.expression, depth, statement, expected)?;
                value.bare_true = false;
                if let Some(flow) = &mut value.flow
                    && !flow.logical
                {
                    flow.yes.reachable = true;
                    flow.no.reachable = true;
                }
                return Ok(value);
            }
            Expression::UnaryExpression(u) => {
                if u.operator.as_str() == "typeof" {
                    let value = self.expr(&u.argument, depth, false)?;
                    self.promise_operator(value.ty, u.span)?;
                }
                let op = u.operator.as_str();
                let required = match op {
                    "-" => Type::Number,
                    "!" => Type::Boolean,
                    "~" => {
                        return Err(self.error(u.span, "C4", "Bitwise operators are rejected"));
                    }
                    _ => {
                        return Err(self.error(u.span, "C8", "Unary operator has no lowering"));
                    }
                };
                let a = self.expr(&u.argument, depth, false)?;
                self.promise_operator(a.ty, u.span)?;
                if a.ty != required {
                    self.nullable_use(u.span, &[a.ty])?;
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
            Expression::BinaryExpression(b) if b.operator.as_str() == "instanceof" => {
                return self.instance_of(e, b, depth, statement);
            }
            Expression::BinaryExpression(b) => {
                let op = b.operator.as_str();
                if matches!(op, "**" | "|" | "&" | "^" | "<<" | ">>" | ">>>") {
                    return Err(self.error(b.span, "C4", "Numeric operator has no lowering"));
                }
                if matches!(op, "===" | "!==" | "==" | "!=") {
                    self.equality_syntax(b.span, [&b.left, &b.right])?;
                }
                let a = self.expr(&b.left, depth, false)?;
                let r = self.expr(&b.right, depth, false)?;
                let null_test = matches!(op, "===" | "!==")
                    && ((a.ty == Type::Null && self.types.inner(self.original_type(&r)).is_some())
                        || (r.ty == Type::Null
                            && self.types.inner(self.original_type(&a)).is_some()));
                if !null_test {
                    self.promise_operator(a.ty, b.span)?;
                    self.promise_operator(r.ty, b.span)?;
                }
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
                        if op == "+"
                            && (a.ty == Type::String || r.ty == Type::String)
                            && matches!(a.ty, Type::String | Type::Number | Type::Boolean)
                            && matches!(r.ty, Type::String | Type::Number | Type::Boolean)
                        {
                            return self.finish_expr(
                                e,
                                statement,
                                Type::String,
                                ExprKind::Template(vec![a, r]),
                            );
                        }
                        if matches!(op, "<" | ">" | "<=" | ">=")
                            && a.ty == Type::String
                            && r.ty == Type::String
                        {
                            return Err(self.error(b.span, "C5", "String ordering is rejected"));
                        }
                        if a.ty != Type::Number || r.ty != Type::Number {
                            self.nullable_use(b.span, &[a.ty, r.ty])?;
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
                let a = if op == "??" {
                    if let Expression::ChainExpression(chain) = &l.left {
                        self.optional_field(chain, depth)?
                    } else {
                        self.expr(&l.left, depth, false)?
                    }
                } else {
                    self.expr(&l.left, depth, false)?
                };
                self.promise_operator(self.original_type(&a), l.span)?;
                if op == "??" {
                    let original = self.original_type(&a);
                    let inner = self.types.inner(original).ok_or_else(|| {
                        self.error(l.span, "C2", "Coalescing requires a nullable left operand")
                    })?;
                    let b = self.expr_as(&l.right, depth, false, expected.or(Some(inner)))?;
                    let nullable = self.types.nullable(inner);
                    self.flow(&b.ty, Some(&nullable), l.span)?;
                    let ty = if self.types.contains_null(b.ty) {
                        nullable
                    } else {
                        inner
                    };
                    // Literal operands keep both edges. Logical operands use their cached test edges.
                    if ty == Type::Boolean
                        && b.flow.is_some_and(|flow| {
                            flow.logical && (!flow.yes.reachable || !flow.no.reachable)
                        })
                    {
                        return Err(self.error(
                            l.span,
                            "C16",
                            "The right operand of boolean coalescing needs both test edges",
                        ));
                    }
                    return self.finish_expr(
                        e,
                        statement,
                        ty,
                        ExprKind::Coalesce(Box::new(a), Box::new(b)),
                    );
                }
                let mark = self.facts.mark();
                let incoming = self.reachable;
                let right_edge = self.edge(&a, op == "&&");
                self.reachable = incoming && right_edge;
                let b = self.expr(&l.right, depth, false)?;
                self.facts.restore(mark);
                self.reachable = incoming;
                self.promise_operator(b.ty, l.span)?;
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
                let mark = self.facts.mark();
                let incoming = self.reachable;
                let true_edge = self.edge(&test, true);
                self.reachable = incoming && true_edge;
                let a = self.expr_as(&c.consequent, depth, false, expected)?;
                let mut join = narrowing::Join::default();
                if self.reachable {
                    join.include(&self.facts, self.facts.head());
                }
                self.facts.restore(mark);
                let false_edge = self.edge(&test, false);
                self.reachable = incoming && false_edge;
                let b = self.expr_as(&c.alternate, depth, false, expected)?;
                if self.reachable {
                    join.include(&self.facts, self.facts.head());
                }
                if join.finish(&mut self.facts).is_none() {
                    self.facts.restore(mark);
                }
                self.reachable = incoming;
                let ty = self
                    .types
                    .join(a.ty, b.ty)
                    .ok_or_else(|| self.error(c.span, "C1", "Conditional types do not agree"))?;
                (
                    ty,
                    ExprKind::Conditional(Box::new(test), Box::new(a), Box::new(b)),
                )
            }
            Expression::ArrayExpression(a) => {
                let contextual_element = expected.and_then(|ty| self.types.element(ty));
                let mut element = contextual_element;
                let mut entries = vec![];
                for entry in &a.elements {
                    let e = entry.as_expression().ok_or_else(|| {
                        self.error(entry.span(), "C6", "Array holes and spreads are rejected")
                    })?;
                    if contextual_element.is_none() {
                        self.inferred_this(e)?;
                    }
                    let value = self.expr_as(e, depth, false, contextual_element)?;
                    if self.types.contains_null(value.ty) {
                        return Err(self.error(e.span(), "C6", "Array elements must exclude null"));
                    }
                    if contextual_element.is_none() {
                        self.inferred_reference(e, value.ty)?;
                    }
                    if let Some(element) = element {
                        self.flow(&value.ty, Some(&element), e.span())?;
                    } else {
                        element = Some(value.ty);
                    }
                    entries.push(value);
                }
                let element = element.ok_or_else(|| {
                    self.error(a.span, "C1", "An empty array needs an element type")
                })?;
                (self.types.array(element), ExprKind::Array(entries))
            }
            Expression::ComputedMemberExpression(m) => {
                if m.optional {
                    return Err(self.error(
                        m.span,
                        "C2",
                        "Optional step needs a terminal chain position",
                    ));
                }
                let (a, i, element) = self.index(&m.object, &m.expression, depth, m.span)?;
                (element, ExprKind::Index(Box::new(a), Box::new(i)))
            }
            Expression::StaticMemberExpression(m) => {
                if m.optional {
                    return Err(self.error(
                        m.span,
                        "C2",
                        "Optional step needs a terminal chain position",
                    ));
                }
                let a = self.expr(&m.object, depth, false)?;
                self.promise_operator(a.ty, m.span)?;
                if a.ty == Type::Error {
                    let key = exceptions::field(m.property.name.as_str()).ok_or_else(|| {
                        self.error(m.span, "C17", "Error has only name and message")
                    })?;
                    return self.finish_expr(
                        e,
                        statement,
                        Type::String,
                        ExprKind::ErrorField(Box::new(a), key),
                    );
                }
                if self.types.element(a.ty).is_none() && a.ty != Type::String {
                    if self.types.contains_null(a.ty) {
                        return Err(self.error(
                            m.span,
                            "C16",
                            "Nullable receiver needs a stable null test",
                        ));
                    }
                    let key = self.field_key(m.property.name.as_str());
                    self.reject_method_value(a.ty, key, m.span)?;
                    let ty = self
                        .types
                        .field(a.ty, key)
                        .ok_or_else(|| self.error(m.span, "C1", "Unknown object field"))?;
                    return self.finish_expr(e, statement, ty, ExprKind::Field(Box::new(a), key));
                }
                if m.property.name != "length" {
                    return Err(self.error(
                        m.span,
                        if a.ty == Type::String { "C5" } else { "C6" },
                        "Member has no value lowering",
                    ));
                }
                (Type::Number, ExprKind::Length(Box::new(a)))
            }
            Expression::CallExpression(call) => {
                return self.call_expression(e, call, depth, statement);
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
                        flow: None,
                        bare_true: false,
                        start: q.span.start,
                        ty: Type::String,
                        kind: ExprKind::String(text.as_str().into()),
                    });
                    if let Some(e) = t.expressions.get(i) {
                        let value = self.expr(e, depth, false)?;
                        self.promise_operator(value.ty, e.span())?;
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
            Expression::ArrowFunctionExpression(a) => self.arrow(a, depth, expected)?,
            Expression::ObjectExpression(o) => self.object(o, depth, expected)?,
            Expression::NullLiteral(_) => (Type::Null, ExprKind::Null),
            Expression::ChainExpression(c) if statement => {
                let value = self.optional_call(c, depth)?;
                return self.finish_expr(e, statement, value.ty, value.kind);
            }
            Expression::ChainExpression(_) => {
                return Err(self.error(e.span(), "C2", "Optional field only precedes coalescing"));
            }
            Expression::NewExpression(new) => {
                if matches!(&new.callee, Expression::Identifier(n) if n.name == "Promise") {
                    return Err(self.error(
                        new.span,
                        "C19",
                        "the Promise identifier is not a value",
                    ));
                }
                if let Some(value) = self.new_error(new, depth)? {
                    value
                } else {
                    self.new_instance(new, depth)?
                }
            }
            Expression::ThisExpression(this) => self.this(this.span)?,
            _ => {
                return Err(self.error(
                    e.span(),
                    "C11",
                    "Expression is outside the closed surface",
                ));
            }
        };
        self.finish_expr(e, statement, ty, kind)
    }
    pub(super) fn finish_expr(
        &mut self,
        e: &Expression<'_>,
        statement: bool,
        ty: Type,
        kind: ExprKind,
    ) -> Result<Expr, Diagnostic> {
        self.checked_type_depth(ty, e.span())?;
        if ty == Type::Void && !statement {
            return Err(self.error(e.span(), "C2", "Void is a statement result only"));
        }
        let flow = if ty == Type::Boolean {
            Some(self.condition(&kind))
        } else {
            None
        };
        if matches!(kind, ExprKind::Logical(..))
            && let Some(flow) = flow
        {
            self.expression_join(flow);
        }
        let value = Expr {
            flow,
            bare_true: matches!(e, Expression::BooleanLiteral(b) if b.value),
            start: e.span().start,
            ty,
            kind,
        };
        Ok(value)
    }
    pub(super) fn index(
        &mut self,
        object: &Expression<'_>,
        index: &Expression<'_>,
        depth: u32,
        span: Span,
    ) -> Result<(Expr, Expr, Type), Diagnostic> {
        let a = self.expr(object, depth, false)?;
        self.promise_operator(a.ty, span)?;
        if a.ty == Type::String {
            return Err(self.error(span, "C5", "String index is rejected"));
        }
        let element = self
            .types
            .element(a.ty)
            .ok_or_else(|| self.error(span, "C6", "Index requires an array"))?;
        let i = self.expr(index, depth, false)?;
        if i.ty != Type::Number {
            return Err(self.error(span, "C6", "Array index requires a number"));
        }
        Ok((a, i, element))
    }
}
