//! Function signatures and isolated function-body flow.
use super::*;
use crate::ir::{Expr, ExprKind, Statement as S};

impl Checker<'_> {
    pub(super) fn signature(
        &self,
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
            types.push(self.ty(&annotation.type_annotation, false, depth)?);
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
        if f.type_parameters.is_some() {
            return Err(self.error(f.span, "C8", "Generic declarations have no lowering"));
        }
        if f.r#async || f.generator || f.declare || f.this_param.is_some() {
            return Err(self.unsupported(f.span));
        }
        let name =
            f.id.as_ref()
                .ok_or_else(|| self.error(f.span, "C11", "Function name is required"))?;
        let (params, required) = self.signature(&f.params, true, depth)?;
        let result = self.ty(
            &f.return_type
                .as_ref()
                .ok_or_else(|| self.error(f.span, "C11", "Result annotation is required"))?
                .type_annotation,
            true,
            depth,
        )?;
        self.bind(
            name.name.as_str(),
            ir::Binding {
                ty: Some(Type::Function(params, Box::new(result), required)),
                owner: self.owner,
                start: f.span.start,
                end: 0,
                mutable: false,
                declaration: true,
            },
            f.span,
        )
    }

    fn parameters(
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
            let ty = self.ty(&annotation.type_annotation, false, depth)?;
            let id = self.bind(
                n.name.as_str(),
                ir::Binding {
                    ty: Some(ty),
                    owner: self.owner,
                    start: p.span.start,
                    end: p.span.end,
                    mutable: true,
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
        for (p, (id, default)) in params.items.iter().zip(&mut out) {
            if let Some(e) = &p.initializer {
                let value = self.expr(e, depth, false)?;
                self.flow(&value.ty, self.bindings[*id].ty.as_ref(), p.span)?;
                *default = Some(value);
            }
        }
        self.default_scopes.pop();
        Ok(out)
    }

    pub(super) fn function_body(
        &mut self,
        id: usize,
        f: &Function<'_>,
        exported: bool,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let Type::Function(_, result, _) = self.bindings[id]
            .ty
            .clone()
            .ok_or_else(|| self.unsupported(f.span))?
        else {
            return Err(self.unsupported(f.span));
        };
        let state = self.enter_function(id, *result.clone());
        let b = f.body.as_ref().ok_or_else(|| self.unsupported(f.span))?;
        let parameters = self.parameters(&f.params, &b.statements, depth)?;
        if !b.directives.is_empty() {
            return Err(self.error(b.span, "C11", "Function directives are rejected"));
        }
        let body = self.block_contents(&b.statements, self.descend(depth, b.span)?)?;
        if *result != Type::Void && !statements::returns(&body) {
            return Err(self.error(f.span, "C11", "Function can finish without its result"));
        }
        self.leave_function(state);
        self.functions.push(ir::Function {
            binding: id,
            name: f.id.as_ref().map_or(String::new(), |n| n.name.to_string()),
            exported,
            parameters,
            result: *result,
            body,
        });
        Ok(())
    }

    pub(super) fn arrow(
        &mut self,
        a: &ArrowFunctionExpression<'_>,
        depth: u32,
    ) -> Result<(Type, ExprKind), Diagnostic> {
        if a.r#async || a.type_parameters.is_some() {
            return Err(self.unsupported(a.span));
        }
        let (params, required) = self.signature(&a.params, false, depth)?;
        let result = a
            .return_type
            .as_ref()
            .map(|t| self.ty(&t.type_annotation, true, depth))
            .transpose()?;
        if a.body.as_expression().is_none() && result.is_none() {
            return Err(self.error(a.span, "C11", "Block arrow needs a result annotation"));
        }
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
        let state = self.enter_function(id, result.clone().unwrap_or(Type::Void));
        let parameters = self.parameters(&a.params, &[], depth)?;
        let body_depth = self.descend(depth, a.body.span())?;
        let (body, result) = if let Some(e) = a.body.as_expression() {
            let value = self.expr(e, body_depth, true)?;
            if value.ty == Type::Void && !matches!(value.kind, ExprKind::Call(..)) {
                return Err(self.error(e.span(), "C11", "Void arrow body must be a call"));
            }
            if let Some(result) = &result {
                self.flow(&value.ty, Some(result), a.span)?;
            }
            if result.is_none() && matches!(value.ty, Type::Function(..)) {
                return Err(self.error(a.span, "C11", "Function result needs an annotation"));
            }
            let result = result.unwrap_or_else(|| value.ty.clone());
            let body = if result == Type::Void {
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
            if result != Type::Void && !statements::returns(&body) {
                return Err(self.error(a.span, "C11", "Arrow can finish without its result"));
            }
            (body, result)
        };
        self.leave_function(state);
        let ty = Type::Function(params, Box::new(result.clone()), required);
        self.bindings[id].ty = Some(ty.clone());
        self.functions.push(ir::Function {
            binding: id,
            name: String::new(),
            exported: false,
            parameters,
            result,
            body,
        });
        Ok((ty, ExprKind::Closure(id)))
    }

    fn enter_function(&mut self, id: usize, result: Type) -> FunctionState {
        let state = FunctionState {
            owner: self.owner,
            result: std::mem::replace(&mut self.result, result),
            reachable: self.reachable,
            loops: std::mem::take(&mut self.loops),
            equality: self.equality.suspend(),
        };
        self.owner = id + 1;
        self.scopes.push(HashMap::new());
        state
    }
    fn leave_function(&mut self, state: FunctionState) {
        self.scopes.pop();
        self.owner = state.owner;
        self.result = state.result;
        self.reachable = state.reachable;
        self.loops = state.loops;
        self.equality.resume(state.equality);
    }
}
struct FunctionState {
    owner: usize,
    result: Type,
    reachable: bool,
    loops: Vec<equality::LoopFlow>,
    equality: equality::BodyState,
}
