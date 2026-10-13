//! Statements and return coverage.
use super::*;
use crate::ir::{Expr, ExprKind, Statement as S};
impl Checker<'_> {
    fn for_head_check(
        &self,
        e: &Expression<'_>,
        reachable: bool,
        diagnostic: Option<&Diagnostic>,
    ) -> Result<(), Diagnostic> {
        if let Err(head) = crate::depth::check_head(e, self.source, !reachable) {
            // C16.8 wins at the first head arrow, but not inside a later nested body.
            if diagnostic.is_some_and(|d| {
                d.code == "C16" && d.message == UNREACHABLE_ARROW && d.offset <= head.offset
            }) {
                return Ok(());
            }
            return Err(head);
        }
        Ok(())
    }

    pub(super) fn variables(
        &mut self,
        d: &VariableDeclaration<'_>,
        depth: u32,
    ) -> Result<Vec<S>, Diagnostic> {
        let mut out = vec![];
        for v in &d.declarations {
            let BindingPattern::BindingIdentifier(n) = &v.id else {
                return Err(self.unsupported(v.span));
            };
            let id = self
                .scopes
                .last()
                .and_then(|s| s.get(n.name.as_str()))
                .copied()
                .ok_or_else(|| self.error(v.span, "C11", "Missing declaration"))?;
            let depth = self.descend(depth, v.span)?;
            let initializer = v
                .init
                .as_ref()
                .ok_or_else(|| self.error(v.span, "C8", "Missing initializer"))?;
            if v.type_annotation.is_none() {
                self.inferred_this(initializer)?;
            }
            let value = self.expr_as(initializer, depth, false, self.bindings[id].ty)?;
            if d.kind == VariableDeclarationKind::Let && v.type_annotation.is_none() {
                self.inferred_reference(initializer, value.ty)?;
            }
            if d.kind == VariableDeclarationKind::Let
                && v.type_annotation.is_none()
                && value.ty == Type::Boolean
                && !boolean_initializer(initializer)
            {
                return Err(self.error(
                    v.span,
                    "C11",
                    "An unannotated boolean let needs a literal or comparison initializer",
                ));
            }
            if self.bindings[id].ty.is_some() {
                self.flow(&value.ty, self.bindings[id].ty.as_ref(), v.span)?;
            } else {
                self.bindings[id].ty = Some(value.ty);
            }
            out.push(S::Located(d.span.start, Box::new(S::Declare(id, value))));
        }
        Ok(out)
    }
    pub(super) fn block(
        &mut self,
        statements: &[Statement<'_>],
        depth: u32,
    ) -> Result<Vec<S>, Diagnostic> {
        self.scopes.push(HashMap::new());
        let out = self.block_contents(statements, depth)?;
        self.scopes.pop();
        Ok(out)
    }
    pub(super) fn block_contents(
        &mut self,
        statements: &[Statement<'_>],
        depth: u32,
    ) -> Result<Vec<S>, Diagnostic> {
        let entry_mark = self.facts.mark();
        let entry_reachable = self.reachable;
        let mut declarations = vec![];
        for s in statements {
            match s {
                Statement::VariableDeclaration(d) => self.variable_names(d)?,
                Statement::FunctionDeclaration(f) => {
                    declarations.push((self.function_name(f, depth)?, f.as_ref()))
                }
                _ => {}
            }
        }
        let ids = self
            .scopes
            .last()
            .map(|s| s.values().copied().collect())
            .unwrap_or_default();
        let mut out = vec![S::Entry(ids)];
        out.extend(
            declarations
                .iter()
                .map(|(id, f)| S::Located(f.span.start, Box::new(S::Function(*id)))),
        );
        for s in statements {
            if !matches!(s, Statement::FunctionDeclaration(_)) {
                out.push(self.statement(s, depth)?);
            }
        }
        let final_reachable = self.reachable;
        let final_facts = self.facts.changes(entry_mark);
        self.facts.restore(entry_mark);
        self.reachable = entry_reachable;
        for (id, f) in declarations {
            self.function_body(id, f, false, depth)?;
        }
        self.facts.apply(final_facts);
        self.reachable = final_reachable;
        Ok(out)
    }
    fn test(&mut self, e: &Expression<'_>, depth: u32) -> Result<Expr, Diagnostic> {
        let test = self.expr(e, depth, false)?;
        self.promise_operator(test.ty, e.span())?;
        if test.ty != Type::Boolean {
            return Err(self.error(e.span(), "C3", "Condition must be boolean"));
        }
        Ok(test)
    }
    fn target(&mut self, n: &IdentifierReference<'_>) -> Result<usize, Diagnostic> {
        if self.is_host(n.name.as_str()) {
            return Err(self.error(n.span, "C18", "A host function is only called"));
        }
        let id = self.lookup(n.name.as_str(), n.span)?;
        if !self.bindings[id].mutable {
            return Err(self.error(n.span, "C11", "Target is not mutable"));
        }
        Ok(id)
    }
    fn effect(&mut self, e: &Expression<'_>, depth: u32, update: bool) -> Result<S, Diagnostic> {
        let depth = self.descend(depth, e.span())?;
        match e {
            Expression::AssignmentExpression(a) => {
                if let AssignmentTarget::ComputedMemberExpression(m) = &a.left {
                    if m.optional {
                        return Err(self.unsupported(m.span));
                    }
                    let (array, index, element) =
                        self.index(&m.object, &m.expression, depth, m.span)?;
                    let mut value = self.expr_as(&a.right, depth, false, Some(element))?;
                    let mut op = match a.operator.as_str() {
                        "=" => None,
                        "+=" => Some("+"),
                        "-=" => Some("-"),
                        "*=" => Some("*"),
                        "/=" => Some("/"),
                        "%=" => Some("%"),
                        _ => {
                            return Err(self.error(
                                a.span,
                                "C4",
                                "Assignment operator is rejected",
                            ));
                        }
                    };
                    if op == Some("+")
                        && element == Type::String
                        && matches!(value.ty, Type::String | Type::Number | Type::Boolean)
                    {
                        value = string_value(value);
                        op = Some("..");
                    }
                    if op.is_some()
                        && op != Some("..")
                        && (element != Type::Number || value.ty != Type::Number)
                    {
                        return Err(self.error(
                            a.span,
                            "C4",
                            "Compound assignment requires numbers",
                        ));
                    }
                    self.flow(&value.ty, Some(&element), a.span)?;
                    return Ok(S::Element(array, index, op, value));
                }
                if let AssignmentTarget::StaticMemberExpression(m) = &a.left {
                    if m.optional {
                        return Err(self.error(m.span, "C2", "Optional write is rejected"));
                    }
                    let receiver = self.expr(&m.object, depth, false)?;
                    if self.types.element(receiver.ty).is_some() {
                        if m.property.name == "length"
                            && a.operator.as_str() == "="
                            && matches!(&a.right, Expression::NumericLiteral(n) if n.value == 0.0)
                        {
                            // Check the literal's strict-module syntax before this special lowering.
                            self.expr(&a.right, depth, false)?;
                            self.methods.insert((ir::Method::Clear, 0));
                            return Ok(S::Eval(Expr {
                                flow: None,
                                bare_true: false,
                                start: a.span.start,
                                ty: Type::Void,
                                kind: ExprKind::Method(
                                    ir::Method::Clear,
                                    0,
                                    Box::new(receiver),
                                    vec![],
                                ),
                            }));
                        }
                        return Err(self.error(
                            m.span,
                            "C6",
                            "Only literal zero can clear array length",
                        ));
                    }
                    if receiver.ty == Type::Error {
                        let key = exceptions::field(m.property.name.as_str()).ok_or_else(|| {
                            self.error(m.span, "C17", "Error has only name and message")
                        })?;
                        let mut value = self.expr_as(&a.right, depth, false, Some(Type::String))?;
                        if a.operator.as_str() == "+="
                            && matches!(value.ty, Type::String | Type::Number | Type::Boolean)
                        {
                            value = string_value(value);
                        }
                        self.flow(&value.ty, Some(&Type::String), a.span)?;
                        let op = match a.operator.as_str() {
                            "=" => None,
                            "+=" => Some(".."),
                            _ => {
                                return Err(self.error(
                                    a.span,
                                    "C17",
                                    "Error field assignment needs a string",
                                ));
                            }
                        };
                        return Ok(S::ErrorField(receiver, key, op, value));
                    }
                    self.nullable_use(m.span, &[receiver.ty])?;
                    let key = self.field_key(m.property.name.as_str());
                    self.reject_method_value(receiver.ty, key, m.span)?;
                    let field_ty = self
                        .types
                        .field(receiver.ty, key)
                        .ok_or_else(|| self.error(m.span, "C1", "Unknown object field"))?;
                    let mut value = self.expr_as(&a.right, depth, false, Some(field_ty))?;
                    let mut op = match a.operator.as_str() {
                        "=" => None,
                        "+=" => Some("+"),
                        "-=" => Some("-"),
                        "*=" => Some("*"),
                        "/=" => Some("/"),
                        "%=" => Some("%"),
                        _ => {
                            return Err(self.error(
                                a.span,
                                "C4",
                                "Assignment operator is rejected",
                            ));
                        }
                    };
                    if op == Some("+")
                        && field_ty == Type::String
                        && matches!(value.ty, Type::String | Type::Number | Type::Boolean)
                    {
                        value = string_value(value);
                        op = Some("..");
                    }
                    if op.is_some()
                        && op != Some("..")
                        && (field_ty != Type::Number || value.ty != Type::Number)
                    {
                        return Err(self.error(
                            a.span,
                            "C4",
                            "Compound assignment requires numbers",
                        ));
                    }
                    self.flow(&value.ty, Some(&field_ty), a.span)?;
                    return Ok(S::Field(receiver, key, op, value));
                }
                let AssignmentTarget::AssignmentTargetIdentifier(n) = &a.left else {
                    return Err(match &a.left {
                        AssignmentTarget::StaticMemberExpression(_)
                        | AssignmentTarget::ComputedMemberExpression(_) => {
                            self.unsupported(a.left.span())
                        }
                        _ => self.error(a.left.span(), "C11", "Invalid assignment target"),
                    });
                };
                let id = self.target(n)?;
                let mut value = self.expr_as(&a.right, depth, false, self.bindings[id].ty)?;
                let mut op = match a.operator.as_str() {
                    "=" => None,
                    "+=" => Some("+"),
                    "-=" => Some("-"),
                    "*=" => Some("*"),
                    "/=" => Some("/"),
                    "%=" => Some("%"),
                    _ => return Err(self.error(a.span, "C4", "Assignment operator is rejected")),
                };
                if op == Some("+")
                    && self.bindings[id].ty == Some(Type::String)
                    && matches!(value.ty, Type::String | Type::Number | Type::Boolean)
                {
                    value = string_value(value);
                    op = Some("..");
                }
                if op.is_some() {
                    self.promise_operator(value.ty, a.span)?;
                    if let Some(ty) = self.bindings[id].ty {
                        self.promise_operator(ty, a.span)?;
                    }
                }
                if op.is_some()
                    && op != Some("..")
                    && (value.ty != Type::Number || self.bindings[id].ty != Some(Type::Number))
                {
                    return Err(self.error(a.span, "C4", "Compound assignment requires numbers"));
                }
                self.flow(&value.ty, self.bindings[id].ty.as_ref(), a.span)?;
                self.facts.set(id, None);
                Ok(S::Assign(id, op, value))
            }
            Expression::UpdateExpression(u) => {
                let op = Some(if u.operator.as_str() == "++" {
                    "+"
                } else {
                    "-"
                });
                let value = Expr {
                    flow: None,
                    bare_true: false,
                    start: u.span.start,
                    ty: Type::Number,
                    kind: ExprKind::Number(1.0),
                };
                if let SimpleAssignmentTarget::StaticMemberExpression(m) = &u.argument {
                    if m.optional {
                        return Err(self.error(m.span, "C2", "Optional write is rejected"));
                    }
                    let receiver = self.expr(&m.object, depth, false)?;
                    if self.types.element(receiver.ty).is_some() {
                        return Err(self.error(m.span, "C6", "Array properties are read-only"));
                    }
                    self.nullable_use(m.span, &[receiver.ty])?;
                    let key = self.field_key(m.property.name.as_str());
                    self.reject_method_value(receiver.ty, key, m.span)?;
                    let field_ty = self
                        .types
                        .field(receiver.ty, key)
                        .ok_or_else(|| self.error(m.span, "C1", "Unknown object field"))?;
                    if field_ty != Type::Number {
                        return Err(self.error(u.span, "C4", "Update requires number"));
                    }
                    return Ok(S::Field(receiver, key, op, value));
                }
                if matches!(
                    u.argument,
                    SimpleAssignmentTarget::ComputedMemberExpression(_)
                ) {
                    return Err(self.error(
                        u.span,
                        "C11",
                        "Element updates are outside the surface",
                    ));
                }
                let SimpleAssignmentTarget::AssignmentTargetIdentifier(n) = &u.argument else {
                    return Err(self.error(u.argument.span(), "C11", "Invalid update target"));
                };
                let id = self.target(n)?;
                if let Some(ty) = self.bindings[id].ty {
                    self.promise_operator(ty, u.span)?;
                }
                if self.bindings[id].ty != Some(Type::Number) {
                    return Err(self.error(u.span, "C4", "Update requires number"));
                }
                Ok(S::Assign(id, op, value))
            }
            Expression::AwaitExpression(_)
            | Expression::CallExpression(_)
            | Expression::ChainExpression(_)
            | Expression::NewExpression(_)
                if !update =>
            {
                let value = self.expr(e, depth, true)?;
                if self.types.promise_value(value.ty).is_some() {
                    return Err(self.error(
                        e.span(),
                        "C19",
                        "a promise value is stored, passed, returned, or awaited",
                    ));
                }
                Ok(S::Eval(value))
            }
            Expression::TaggedTemplateExpression(_) | Expression::SequenceExpression(_) => {
                Err(self.error(e.span(), "C8", "Expression has no lowering"))
            }
            _ => Err(self.error(e.span(), "C11", "Invalid statement expression")),
        }
    }
    pub(super) fn statement(&mut self, s: &Statement<'_>, depth: u32) -> Result<S, Diagnostic> {
        let depth = self.descend(depth, s.span())?;
        let form = match s {
            Statement::VariableDeclaration(d) => S::Block(self.variables(d, depth)?),
            Statement::BlockStatement(b) => S::Block(self.block(&b.body, depth)?),
            Statement::ExpressionStatement(e) => self.effect(&e.expression, depth, false)?,
            Statement::IfStatement(i) => {
                let test = self.test(&i.test, depth)?;
                let mark = self.facts.mark();
                let incoming = self.reachable;
                let true_edge = self.edge(&test, true);
                self.reachable = incoming && true_edge;
                let a = self.statement(&i.consequent, depth)?;
                let first_reachable = self.reachable && !narrowing::exits(&a);
                let mut join = narrowing::Join::default();
                if first_reachable {
                    let head = self.facts.head();
                    join.include(&mut self.facts, head);
                }
                self.facts.restore(mark);
                let false_edge = self.edge(&test, false);
                self.reachable = incoming && false_edge;
                let b = i
                    .alternate
                    .as_ref()
                    .map(|s| self.statement(s, depth))
                    .transpose()?;
                let second_reachable = self.reachable && !b.as_ref().is_some_and(narrowing::exits);
                if second_reachable {
                    let head = self.facts.head();
                    join.include(&mut self.facts, head);
                }
                if join.finish(&mut self.facts).is_none() {
                    self.facts.restore(mark);
                }
                self.reachable = first_reachable || second_reachable;
                S::If(test, Box::new(a), b.map(Box::new))
            }
            Statement::WhileStatement(w) => {
                self.loop_writes(w.span);
                let test = self.test(&w.test, depth)?;
                self.enter_loop();
                self.reachable = self.reachable && self.edge(&test, true);
                self.loops += 1;
                let body = self.statement(&w.body, depth)?;
                self.loops -= 1;
                self.leave_loop(Some(&test));
                S::While(test, Box::new(body))
            }
            Statement::ForStatement(f) => {
                self.scopes.push(HashMap::new());
                let init = if let Some(ForStatementInit::VariableDeclaration(d)) = &f.init {
                    if d.kind != VariableDeclarationKind::Let {
                        return Err(self.error(d.span, "C11", "For initializer requires let"));
                    }
                    self.variable_names(d)?;
                    let reachable = self.reachable;
                    let result = self.variables(d, depth);
                    for e in d.declarations.iter().filter_map(|d| d.init.as_ref()) {
                        self.for_head_check(e, reachable, result.as_ref().err())?;
                    }
                    result?
                } else if f.init.is_none() {
                    vec![]
                } else {
                    return Err(self.error(f.span, "C11", "For initializer must be a declaration"));
                };
                let ids = init
                    .iter()
                    .filter_map(|s| {
                        if let S::Declare(id, _) = s.form() {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();
                self.loop_writes(f.span);
                let test = f
                    .test
                    .as_ref()
                    .map(|e| {
                        let reachable = self.reachable;
                        let result = self.test(e, depth);
                        self.for_head_check(e, reachable, result.as_ref().err())?;
                        result
                    })
                    .transpose()?;
                self.enter_loop();
                if let Some(test) = &test {
                    self.reachable = self.reachable && self.edge(test, true);
                }
                self.loops += 1;
                let body = self.statement(&f.body, depth)?;
                self.loops -= 1;
                self.loop_update();
                let update = f
                    .update
                    .as_ref()
                    .map(|e| {
                        let reachable = self.reachable;
                        let result = self.effect(e, depth, true).map(Box::new);
                        self.for_head_check(e, reachable, result.as_ref().err())?;
                        result
                    })
                    .transpose()?;
                self.leave_loop(test.as_ref());
                self.scopes.pop();
                S::For(init, test, update, Box::new(body), ids)
            }
            Statement::BreakStatement(b) if b.label.is_none() && self.loops != 0 => {
                self.break_edge();
                self.reachable = false;
                S::Break
            }
            Statement::ContinueStatement(c) if c.label.is_none() && self.loops != 0 => {
                self.continue_edge();
                self.reachable = false;
                S::Continue
            }
            Statement::ThrowStatement(t) => {
                let value = self.catch_value(&t.argument, depth)?;
                if !matches!(value.ty, Type::Error | Type::Unknown) {
                    return Err(self.error(t.span, "C17", "Throw needs an Error value"));
                }
                self.reachable = false;
                S::Throw(value)
            }
            Statement::TryStatement(t) => self.try_statement(t, depth)?,
            Statement::ReturnStatement(r) => {
                if self.in_constructor && r.argument.is_some() {
                    return Err(self.error(r.span, "C11", "A constructor return has no argument"));
                }
                let value = r
                    .argument
                    .as_ref()
                    .map(|e| {
                        self.async_return_literal(e)?;
                        self.expr_as(e, depth, false, Some(self.result))
                    })
                    .transpose()?;
                let ty = value.as_ref().map_or(&Type::Void, |v| &v.ty);
                self.return_flow(*ty, self.result, r.span)?;
                self.reachable = false;
                S::Return(value)
            }
            Statement::ForOfStatement(f) => return self.for_of(f, depth),
            Statement::SwitchStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForInStatement(_)
            | Statement::LabeledStatement(_)
            | Statement::TSInterfaceDeclaration(_) => {
                return Err(self.error(s.span(), "C8", "Statement has no lowering"));
            }
            _ => return Err(self.error(s.span(), "C11", "Statement is outside the surface")),
        };
        Ok(S::Located(s.span().start, Box::new(form)))
    }
}
// Return coverage uses an explicit postorder stack. It adds no compiler recursion.
pub(super) fn returns(body: &[S]) -> bool {
    enum Work<'a> {
        Body(&'a [S]),
        Form(&'a S),
        Any(usize),
        Both,
    }
    let mut pending = vec![Work::Body(body)];
    let mut values = vec![];
    while let Some(work) = pending.pop() {
        match work {
            Work::Body(body) => {
                pending.push(Work::Any(body.len()));
                pending.extend(body.iter().map(Work::Form));
            }
            Work::Form(s) => match s.form() {
                S::Return(_) | S::Throw(_) => values.push(true),
                S::Try(_, _, _) => values.push(ir::abrupt(s, false)),
                S::Block(b) => pending.push(Work::Body(b)),
                S::If(_, a, Some(b)) => {
                    pending.push(Work::Both);
                    pending.push(Work::Form(a));
                    pending.push(Work::Form(b));
                }
                S::While(test, body) => values.push(test.bare_true && !ir::loop_break(body)),
                S::For(_, test, _, body, _) => {
                    values.push(test.as_ref().is_none_or(|t| t.bare_true) && !ir::loop_break(body))
                }
                _ => values.push(false),
            },
            Work::Any(n) => {
                let start = values.len().saturating_sub(n);
                let complete = values.drain(start..).any(|v| v);
                values.push(complete);
            }
            Work::Both => {
                let a = values.pop().unwrap_or(false);
                let b = values.pop().unwrap_or(false);
                values.push(a && b);
            }
        }
    }
    values.pop().unwrap_or(false)
}

// Parentheses preserve the initializer form.
fn boolean_initializer(mut e: &Expression<'_>) -> bool {
    while let Expression::ParenthesizedExpression(p) = e {
        e = &p.expression;
    }
    matches!(e, Expression::BooleanLiteral(_))
        || matches!(e, Expression::BinaryExpression(b) if matches!(b.operator.as_str(), "<" | ">" | "<=" | ">=" | "===" | "!==" | "==" | "!="))
}

fn string_value(value: Expr) -> Expr {
    Expr {
        flow: None,
        start: value.start,
        bare_true: false,
        ty: Type::String,
        kind: ExprKind::Template(vec![value]),
    }
}
