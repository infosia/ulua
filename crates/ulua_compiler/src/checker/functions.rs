//! Function signatures and isolated function-body flow.
use super::*;
use crate::ir::{Expr, ExprKind, Statement as S};

impl Checker<'_> {
    pub(super) fn signature(
        &mut self,
        params: &FormalParameters<'_>,
        defaults: bool,
        depth: u32,
    ) -> Result<(Vec<Type>, usize), Diagnostic> {
        if params.rest.is_some() {
            return Err(self.error(params.span, "C11", "Rest parameters are rejected"));
        }
        if params.items.len() > crate::PARAMETER_BOUND {
            return Err(self.error(params.span, "limit:parameters", "Parameter limit exceeded"));
        }
        let mut types = vec![];
        let mut required = 0;
        let mut default_seen = false;
        for (i, p) in params.items.iter().enumerate() {
            if p.optional {
                return Err(self.error(p.span, "C2", "Optional parameters are rejected"));
            }
            if !p.decorators.is_empty()
                || p.accessibility.is_some()
                || p.readonly
                || p.r#override
                || !matches!(p.pattern, BindingPattern::BindingIdentifier(_))
                || (!defaults && p.initializer.is_some())
            {
                return Err(self.error(p.span, "C11", "Invalid parameter form"));
            }
            let annotation = p
                .type_annotation
                .as_ref()
                .ok_or_else(|| self.error(p.span, "C11", "Parameter annotation is required"))?;
            if p.initializer.is_some()
                && matches!(unparen(&annotation.type_annotation), TSType::TSUnionType(u) if u.types.iter().any(|t| matches!(unparen(t), TSType::TSNullKeyword(_))))
            {
                return Err(self.error(p.span, "C2", "A default parameter must exclude null"));
            }
            types.push(self.parameter_type(&annotation.type_annotation, depth)?);
            if p.initializer.is_none() {
                if default_seen {
                    return Err(self.error(p.span, "C11", "Parameter defaults must be trailing"));
                }
                required = i + 1;
            } else {
                default_seen = true;
            }
        }
        Ok((types, required))
    }

    pub(super) fn function_name(
        &mut self,
        f: &Function<'_>,
        depth: u32,
    ) -> Result<usize, Diagnostic> {
        if f.declare {
            return Err(self.error(f.span, "C18", "Script declare forms are rejected"));
        }
        if f.type_parameters.is_some() {
            return Err(self.error(f.span, "C8", "Generic declarations have no lowering"));
        }
        if f.generator {
            return Err(self.error(f.span, "C19", "generators are rejected"));
        }
        if f.this_param.is_some() {
            return Err(self.unsupported(f.span));
        }
        let name =
            f.id.as_ref()
                .ok_or_else(|| self.error(f.span, "C11", "Function name is required"))?;
        let (params, required) = self.signature(&f.params, true, depth)?;
        let result = self.ty(
            &f.return_type
                .as_ref()
                .ok_or_else(|| {
                    self.error(
                        f.span,
                        if f.r#async { "C19" } else { "C11" },
                        "Result annotation is required",
                    )
                })?
                .type_annotation,
            true,
            depth,
        )?;
        self.async_result(f.r#async, result, f.span)?;
        let ty = self.types.intern(params, result, required);
        self.checked_type_depth(ty, f.span)?;
        self.bind(
            name.name.as_str(),
            ir::Binding {
                ty: Some(ty),
                owner: self.owner,
                start: f.span.start,
                end: 0,
                mutable: false,
                declaration: true,
            },
            f.span,
        )
    }

    pub(super) fn parameters(
        &mut self,
        params: &FormalParameters<'_>,
        body: &[Statement<'_>],
        depth: u32,
    ) -> Result<Vec<(usize, Option<Expr>)>, Diagnostic> {
        let mut out = vec![];
        for p in &params.items {
            let BindingPattern::BindingIdentifier(n) = &p.pattern else {
                return Err(self.unsupported(p.span));
            };
            let annotation = p
                .type_annotation
                .as_ref()
                .ok_or_else(|| self.error(p.span, "C11", "Missing parameter type"))?;
            let ty = self.parameter_type(&annotation.type_annotation, depth)?;
            let id = self.bind(
                n.name.as_str(),
                ir::Binding {
                    ty: Some(ty),
                    owner: self.owner,
                    start: p.span.start,
                    end: p.span.end,
                    mutable: ty != Type::Unknown,
                    declaration: false,
                },
                p.span,
            )?;
            out.push((id, None));
        }
        let mut names = std::collections::HashSet::new();
        for statement in body {
            match statement {
                Statement::FunctionDeclaration(f) => {
                    if let Some(n) = &f.id {
                        names.insert(n.name.to_string());
                    }
                }
                Statement::VariableDeclaration(d) => {
                    for v in &d.declarations {
                        if let BindingPattern::BindingIdentifier(n) = &v.id {
                            names.insert(n.name.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        self.default_scopes.push((self.scopes.len(), names));
        let async_body = std::mem::replace(&mut self.async_body, false);
        for (p, (id, default)) in params.items.iter().zip(&mut out) {
            if let Some(e) = &p.initializer {
                let value = self.expr_as(e, depth, false, self.bindings[*id].ty)?;
                self.flow(&value.ty, self.bindings[*id].ty.as_ref(), p.span)?;
                *default = Some(value);
            }
        }
        self.default_scopes.pop();
        self.async_body = async_body;
        Ok(out)
    }

    pub(super) fn function_body(
        &mut self,
        id: usize,
        f: &Function<'_>,
        exported: bool,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let result = self.bindings[id]
            .ty
            .and_then(|ty| self.types.function(ty))
            .map(|f| f.result)
            .ok_or_else(|| self.unsupported(f.span))?;
        let state = self.enter_function(id, result, false, f.span.start);
        self.async_body = f.r#async;
        self.result = self.body_result(result);
        let b = f.body.as_ref().ok_or_else(|| self.unsupported(f.span))?;
        let parameters = self.parameters(&f.params, &b.statements, depth)?;
        if !b.directives.is_empty() {
            return Err(self.error(b.span, "C11", "Function directives are rejected"));
        }
        let body = self.block_contents(&b.statements, self.descend(depth, b.span)?)?;
        if self.result != Type::Void && !statements::returns(&body) {
            return Err(self.error(f.span, "C11", "Function can finish without its result"));
        }
        self.leave_function(state);
        self.functions.push(ir::Function {
            declaration: true,
            inline_callback: None,
            asynchronous: f.r#async,
            initializers: vec![],
            binding: id,
            name: f.id.as_ref().map_or(String::new(), |n| n.name.to_string()),
            exported,
            parameters,
            result,
            body,
        });
        Ok(())
    }

    pub(super) fn arrow(
        &mut self,
        a: &ArrowFunctionExpression<'_>,
        depth: u32,
        expected: Option<Type>,
    ) -> Result<(Type, ExprKind), Diagnostic> {
        if !self.reachable {
            return Err(self.error(a.span, "C16", UNREACHABLE_ARROW));
        }
        if a.type_parameters.is_some() {
            return Err(self.unsupported(a.span));
        }
        let (params, required) = self.signature(&a.params, false, depth)?;
        let result = a
            .return_type
            .as_ref()
            .map(|t| self.ty(&t.type_annotation, true, depth))
            .transpose()?;
        if a.r#async && a.return_type.is_none() {
            return Err(self.error(a.span, "C19", "async result annotation is required"));
        }
        if a.body.as_expression().is_none() && result.is_none() {
            return Err(self.error(a.span, "C11", "Block arrow needs a result annotation"));
        }
        if let Some(result) = result {
            self.async_result(a.r#async, result, a.span)?;
        }
        let result = result.or_else(|| {
            expected.and_then(|ty| self.types.function(ty).map(|signature| signature.result))
        });
        // Anonymous identities never enter a source scope or the parent's local budget.
        self.scopes.push(HashMap::new());
        let id = self.bind(
            "",
            ir::Binding {
                ty: None,
                owner: self.owner,
                start: a.span.start,
                end: a.span.end,
                mutable: false,
                declaration: false,
            },
            a.span,
        )?;
        self.scopes.pop();
        let state = self.enter_function(id, result.unwrap_or(Type::Void), true, a.span.start);
        self.async_body = a.r#async;
        self.result = self.body_result(result.unwrap_or(Type::Void));
        let parameters = self.parameters(&a.params, &[], depth)?;
        let body_depth = self.descend(depth, a.body.span())?;
        let (body, result) = if let Some(e) = a.body.as_expression() {
            let mut body_expression = e;
            while let Expression::ParenthesizedExpression(p) = body_expression {
                body_expression = &p.expression;
            }
            if matches!(body_expression, Expression::ChainExpression(c) if matches!(c.expression, ChainElement::CallExpression(_)))
            {
                return Err(self.error(
                    e.span(),
                    "C2",
                    "An optional call is only an expression statement",
                ));
            }
            if result.is_none() {
                self.inferred_this(e)?;
            }
            self.async_return_literal(e)?;
            let value = self.expr_as(e, body_depth, true, result.map(|r| self.body_result(r)))?;
            if result.is_none() {
                self.inferred_reference(e, value.ty)?;
            }
            self.reject_absent(value.ty, e.span())?;
            if value.ty == Type::Void
                && !matches!(
                    value.kind,
                    ExprKind::Call(..)
                        | ExprKind::Method(..)
                        | ExprKind::ClassCall(..)
                        | ExprKind::Await(..)
                )
            {
                return Err(self.error(e.span(), "C11", "Void arrow body must be a call"));
            }
            if let Some(result) = &result {
                self.return_flow(value.ty, *result, a.span)?;
            }
            if a.return_type.is_none() && self.types.contains_function(value.ty) {
                return Err(self.error(a.span, "C11", "Function result needs an annotation"));
            }
            let result = result.unwrap_or(value.ty);
            let body = if value.ty == Type::Void {
                vec![S::Eval(value)]
            } else {
                vec![S::Return(Some(value))]
            };
            (body, result)
        } else {
            let ArrowFunctionBody::FunctionBody(b) = &a.body else {
                return Err(self.unsupported(a.span));
            };
            if !b.directives.is_empty() {
                return Err(self.error(b.span, "C11", "Arrow directives are rejected"));
            }
            let body = self.block_contents(&b.statements, body_depth)?;
            let result = result.unwrap_or(Type::Void);
            if self.result != Type::Void && !statements::returns(&body) {
                return Err(self.error(a.span, "C11", "Arrow can finish without its result"));
            }
            (body, result)
        };
        self.async_result(a.r#async, result, a.span)?;
        self.leave_function(state);
        let ty = self.types.intern(params, result, required);
        self.checked_type_depth(ty, a.span)?;
        self.bindings[id].ty = Some(ty);
        self.functions.push(ir::Function {
            declaration: false,
            inline_callback: None,
            asynchronous: a.r#async,
            initializers: vec![],
            binding: id,
            name: String::new(),
            exported: false,
            parameters,
            result,
            body,
        });
        Ok((ty, ExprKind::Closure(id)))
    }

    pub(super) fn enter_function(
        &mut self,
        id: usize,
        result: Type,
        inherit: bool,
        source_owner: u32,
    ) -> FunctionState {
        let state = FunctionState {
            reachable: std::mem::replace(&mut self.reachable, true),
            fact_mark: self.facts.mark(),
            fact_floor: self.facts.floor,
            mutable_fact_floor: self.facts.mutable_floor,
            loop_facts: std::mem::take(&mut self.loop_facts),
            owner: self.owner,
            source_owner: std::mem::replace(&mut self.source_owner, source_owner),
            result: std::mem::replace(&mut self.result, result),
            async_body: std::mem::replace(&mut self.async_body, false),
            loops: std::mem::replace(&mut self.loops, 0),
            this_binding: self.this_binding,
            in_constructor: std::mem::replace(&mut self.in_constructor, false),
        };
        if !inherit {
            self.this_binding = None;
        }
        // Named declarations have no outer const facts (TS18047).
        if !inherit || !state.reachable {
            self.facts.floor = self.facts.serial + 1;
        }
        self.facts.mutable_floor = self.facts.serial + 1;
        self.owner = id + 1;
        self.scopes.push(HashMap::new());
        state
    }
    pub(super) fn leave_function(&mut self, state: FunctionState) {
        self.reachable = state.reachable;
        self.facts.restore(state.fact_mark);
        self.facts.floor = state.fact_floor;
        self.facts.mutable_floor = state.mutable_fact_floor;
        self.loop_facts = state.loop_facts;
        self.scopes.pop();
        self.owner = state.owner;
        self.source_owner = state.source_owner;
        self.result = state.result;
        self.async_body = state.async_body;
        self.loops = state.loops;
        self.this_binding = state.this_binding;
        self.in_constructor = state.in_constructor;
    }
}
pub(super) struct FunctionState {
    reachable: bool,
    fact_mark: usize,
    fact_floor: u64,
    mutable_fact_floor: u64,
    loop_facts: Vec<narrowing::LoopFacts>,
    owner: usize,
    source_owner: u32,
    result: Type,
    async_body: bool,
    loops: usize,
    this_binding: Option<usize>,
    in_constructor: bool,
}
