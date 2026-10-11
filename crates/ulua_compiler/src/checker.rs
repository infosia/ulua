//! P1 declarations, scope, value flows, and transitive declaration order.
mod expressions;
mod statements;
use crate::{
    Diagnostic,
    ir::{self, Type},
};
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use std::collections::{HashMap, VecDeque};

pub(crate) struct Checker<'s> {
    source: &'s [u8],
    pub(super) bindings: Vec<ir::Binding>,
    scopes: Vec<HashMap<String, usize>>,
    owner: usize,
    result: Type,
    loops: usize,
    limit: u32,
    references: Vec<(usize, usize, u32)>,
}
impl Checker<'_> {
    fn error(&self, span: Span, code: &'static str, message: &str) -> Diagnostic {
        Diagnostic::at(self.source, span.start as usize, code, message)
    }
    fn unsupported(&self, span: Span) -> Diagnostic {
        self.error(span, "unsupported", "This form is outside P1")
    }
    fn descend(&self, depth: u32, span: Span) -> Result<u32, Diagnostic> {
        let next = depth + 1;
        if next >= self.limit {
            return Err(self.error(span, "limit:nesting-depth", "Checker depth limit exceeded"));
        }
        Ok(next)
    }
    fn ty(&self, t: &TSType<'_>, result: bool, depth: u32) -> Result<Type, Diagnostic> {
        let depth = self.descend(depth, t.span())?;
        Ok(match t {
            TSType::TSNumberKeyword(_) => Type::Number,
            TSType::TSBooleanKeyword(_) => Type::Boolean,
            TSType::TSStringKeyword(_) => Type::String,
            TSType::TSVoidKeyword(_) if result => Type::Void,
            TSType::TSVoidKeyword(_) | TSType::TSUndefinedKeyword(_) => {
                return Err(self.error(t.span(), "C2", "Undefined and void values are rejected"));
            }
            TSType::TSBigIntKeyword(_) => {
                return Err(self.error(t.span(), "C4", "BigInt is rejected"));
            }
            TSType::TSAnyKeyword(_)
            | TSType::TSUnknownKeyword(_)
            | TSType::TSObjectKeyword(_)
            | TSType::TSNeverKeyword(_) => {
                return Err(self.error(t.span(), "C8", "Type has no checked representation"));
            }
            TSType::TSUnionType(u) => {
                if u.types.len() == 2
                    && u.types
                        .iter()
                        .any(|t| matches!(t, TSType::TSNullKeyword(_)))
                {
                    return Err(self.unsupported(t.span()));
                }
                return Err(self.error(t.span(), "C2", "Only T | null is a union"));
            }
            TSType::TSParenthesizedType(p) => return self.ty(&p.type_annotation, result, depth),
            TSType::TSNullKeyword(_)
            | TSType::TSArrayType(_)
            | TSType::TSFunctionType(_)
            | TSType::TSTypeReference(_)
            | TSType::TSTypeLiteral(_) => return Err(self.unsupported(t.span())),
            _ => return Err(self.error(t.span(), "C11", "Type is outside the closed surface")),
        })
    }
    fn bind(&mut self, name: &str, binding: ir::Binding, span: Span) -> Result<usize, Diagnostic> {
        if name == "undefined" {
            return Err(self.error(span, "C2", "Undefined is rejected even as a binding name"));
        }
        let id = self.bindings.len();
        let scope = self.scopes.last_mut().ok_or_else(|| {
            Diagnostic::at(self.source, span.start as usize, "C11", "Missing scope")
        })?;
        if scope.insert(name.into(), id).is_some() {
            return Err(self.error(span, "C11", "Duplicate declaration"));
        }
        self.bindings.push(binding);
        Ok(id)
    }
    fn lookup(&mut self, name: &str, span: Span) -> Result<usize, Diagnostic> {
        let id = self
            .scopes
            .iter()
            .rev()
            .find_map(|s| s.get(name))
            .copied()
            .ok_or_else(|| {
                self.error(
                    span,
                    if name == "undefined" { "C2" } else { "C11" },
                    "Unknown identifier",
                )
            })?;
        let binding = &self.bindings[id];
        if !matches!(binding.ty, Some(Type::Function(..))) && span.start < binding.end {
            return Err(self.error(span, "C10", "Reference precedes the end of its declaration"));
        }
        self.references.push((self.owner, id, span.start));
        Ok(id)
    }
    fn variable_names(&mut self, d: &VariableDeclaration<'_>) -> Result<(), Diagnostic> {
        if d.kind == VariableDeclarationKind::Var {
            return Err(self.error(d.span, "C8", "Use let or const"));
        }
        if d.declare
            || !matches!(
                d.kind,
                VariableDeclarationKind::Let | VariableDeclarationKind::Const
            )
        {
            return Err(self.error(d.span, "C11", "Invalid declaration"));
        }
        for v in &d.declarations {
            let BindingPattern::BindingIdentifier(id) = &v.id else {
                return Err(self.error(v.span, "C11", "Binding patterns are rejected"));
            };
            if v.definite {
                return Err(self.error(v.span, "C11", "Definite assignment is rejected"));
            }
            if v.init.is_none() {
                return Err(self.error(v.span, "C8", "An initializer is required"));
            }
            let ty = v
                .type_annotation
                .as_ref()
                .map(|t| self.ty(&t.type_annotation, false, 0))
                .transpose()?;
            self.bind(
                id.name.as_str(),
                ir::Binding {
                    ty,
                    owner: self.owner,
                    start: v.span.start,
                    end: d.span.end,
                    mutable: d.kind == VariableDeclarationKind::Let,
                },
                v.span,
            )?;
        }
        Ok(())
    }
    fn tdz(&self) -> Result<(), Diagnostic> {
        // Only the latest declaration end matters for a reference's source offset.
        let mut latest = vec![0; self.bindings.len() + 1];
        let mut callers = vec![Vec::new(); latest.len()];
        for &(owner, id, _) in &self.references {
            if owner == 0 {
                continue;
            }
            if matches!(self.bindings[id].ty, Some(Type::Function(..))) {
                callers[id + 1].push(owner);
            } else if self.bindings[id].owner == 0 {
                latest[owner] = latest[owner].max(self.bindings[id].end);
            }
        }
        let mut pending: VecDeque<_> = latest
            .iter()
            .enumerate()
            .filter_map(|(i, end)| (*end > 0).then_some(i))
            .collect();
        let mut queued: Vec<_> = latest.iter().map(|end| *end > 0).collect();
        while let Some(function) = pending.pop_front() {
            queued[function] = false;
            for &caller in &callers[function] {
                if latest[caller] < latest[function] {
                    latest[caller] = latest[function];
                    if !queued[caller] {
                        queued[caller] = true;
                        pending.push_back(caller);
                    }
                }
            }
        }
        for &(_, id, offset) in &self.references {
            if matches!(self.bindings[id].ty, Some(Type::Function(..))) && offset < latest[id + 1] {
                return Err(Diagnostic::at(
                    self.source,
                    offset as usize,
                    "C10",
                    "Function reaches an undeclared binding",
                ));
            }
        }
        Ok(())
    }
}

fn unparen<'a, 'b>(mut ty: &'a TSType<'b>) -> &'a TSType<'b> {
    while let TSType::TSParenthesizedType(p) = ty {
        ty = &p.type_annotation;
    }
    ty
}

pub(crate) fn check(
    program: &Program<'_>,
    source: &[u8],
    limit: u32,
) -> Result<ir::Module, Diagnostic> {
    let mut c = Checker {
        source,
        bindings: vec![],
        scopes: vec![HashMap::new()],
        owner: 0,
        result: Type::Void,
        loops: 0,
        limit,
        references: vec![],
    };
    if program.hashbang.is_some() || !program.directives.is_empty() {
        return Err(c.error(program.span, "C11", "Module directives are rejected"));
    }
    let mut functions = Vec::new();
    for s in &program.body {
        let (f, exported) = match s {
            Statement::FunctionDeclaration(f) => (f.as_ref(), false),
            Statement::ExportDeclaration(e) => match &e.declaration {
                Declaration::FunctionDeclaration(f) => (f.as_ref(), true),
                _ => return Err(c.unsupported(e.span)),
            },
            Statement::VariableDeclaration(d) => {
                c.variable_names(d)?;
                continue;
            }
            Statement::TSInterfaceDeclaration(_) => {
                return Err(c.error(s.span(), "C8", "Interface declarations have no lowering"));
            }
            Statement::ClassDeclaration(_) | Statement::TSTypeAliasDeclaration(_) => {
                return Err(c.unsupported(s.span()));
            }
            _ => return Err(c.error(s.span(), "C11", "Module statements must be declarations")),
        };
        if f.type_parameters.is_some() {
            return Err(c.error(f.span, "C8", "Generic declarations have no lowering"));
        }
        if f.r#async || f.generator || f.declare || f.this_param.is_some() {
            return Err(c.unsupported(f.span));
        }
        let name =
            f.id.as_ref()
                .ok_or_else(|| c.error(f.span, "C11", "Function name is required"))?
                .name
                .as_str();
        if f.params.rest.is_some() {
            return Err(c.unsupported(f.params.span));
        }
        if f.params.items.len() > crate::PARAMETER_BOUND {
            return Err(c.error(
                f.params.span,
                "limit:parameters",
                "Parameter limit exceeded",
            ));
        }
        let mut params = vec![];
        let mut required = 0;
        for (i, p) in f.params.items.iter().enumerate() {
            if p.optional {
                return Err(c.error(p.span, "C2", "Optional parameters are rejected"));
            }
            if !p.decorators.is_empty()
                || p.accessibility.is_some()
                || p.readonly
                || p.r#override
                || !matches!(p.pattern, BindingPattern::BindingIdentifier(_))
            {
                return Err(c.unsupported(p.span));
            }
            if p.initializer.is_some() && p.type_annotation.as_ref().is_some_and(|t| matches!(unparen(&t.type_annotation), TSType::TSUnionType(u) if u.types.iter().any(|t| matches!(unparen(t), TSType::TSNullKeyword(_))))) {
                return Err(c.error(p.span, "C2", "A default parameter must exclude null"));
            }
            params.push(
                c.ty(
                    &p.type_annotation
                        .as_ref()
                        .ok_or_else(|| c.error(p.span, "C11", "Parameter annotation is required"))?
                        .type_annotation,
                    false,
                    0,
                )?,
            );
            if p.initializer.is_none() {
                required = i + 1;
            }
        }
        let result = c.ty(
            &f.return_type
                .as_ref()
                .ok_or_else(|| c.error(f.span, "C11", "Result annotation is required"))?
                .type_annotation,
            true,
            0,
        )?;
        let id = c.bind(
            name,
            ir::Binding {
                ty: Some(Type::Function(params, Box::new(result.clone()), required)),
                owner: 0,
                start: f.span.start,
                end: 0,
                mutable: false,
            },
            f.span,
        )?;
        functions.push((id, name.to_owned(), exported, result, f));
    }
    if !functions.iter().any(|(_, name, exported, result, f)| {
        name == "main" && *exported && *result == Type::Void && f.params.items.is_empty()
    }) {
        return Err(c.unsupported(program.span));
    }
    let mut body = vec![];
    for s in &program.body {
        if let Statement::VariableDeclaration(d) = s {
            body.extend(c.variables(d, 0)?);
        }
    }
    let mut checked = vec![];
    for (id, name, exported, result, f) in functions {
        c.owner = id + 1;
        c.result = result.clone();
        c.scopes.push(HashMap::new());
        let mut parameters = vec![];
        // Declare all parameters before defaults, so later parameters have a TDZ.
        for p in &f.params.items {
            let BindingPattern::BindingIdentifier(n) = &p.pattern else {
                return Err(c.unsupported(p.span));
            };
            let ty = c.ty(
                &p.type_annotation
                    .as_ref()
                    .ok_or_else(|| c.unsupported(p.span))?
                    .type_annotation,
                false,
                0,
            )?;
            let b = c.bind(
                n.name.as_str(),
                ir::Binding {
                    ty: Some(ty),
                    owner: c.owner,
                    start: p.span.start,
                    end: p.span.end,
                    mutable: true,
                },
                p.span,
            )?;
            parameters.push((b, None));
        }
        for (p, (b, default)) in f.params.items.iter().zip(&mut parameters) {
            if let Some(e) = &p.initializer {
                let value = c.expr(e, 0, false)?;
                c.flow(&value.ty, c.bindings[*b].ty.as_ref(), p.span)?;
                *default = Some(value);
            }
        }
        let b = f.body.as_ref().ok_or_else(|| c.unsupported(f.span))?;
        if !b.directives.is_empty() {
            return Err(c.error(b.span, "C11", "Function directives are rejected"));
        }
        let body = c.block_contents(&b.statements, 0)?;
        if result != Type::Void && !statements::returns(&body, 0, limit)? {
            return Err(c.error(f.span, "C1", "Function can finish without its result"));
        }
        c.scopes.pop();
        checked.push(ir::Function {
            binding: id,
            name,
            exported,
            parameters,
            result,
            body,
        });
    }
    c.tdz()?;
    Ok(ir::Module {
        bindings: c.bindings,
        functions: checked,
        body,
        depth: limit,
    })
}
