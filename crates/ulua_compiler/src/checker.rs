//! P1 declarations, scope, value flows, and transitive declaration order.
mod arrays;
mod equality;
mod expressions;
use equality::Equality;
mod functions;
mod order;
mod statements;
mod strict;
use crate::{
    Diagnostic,
    ir::{self, Type},
};
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use std::collections::HashMap;

pub(crate) struct Checker<'s> {
    source: &'s [u8],
    pub(super) bindings: Vec<ir::Binding>,
    scopes: Vec<HashMap<String, usize>>,
    owner: usize,
    result: Type,
    types: ir::Types,
    loops: Vec<equality::LoopFlow>,
    reachable: bool,
    limit: u32,
    references: Vec<(usize, usize, u32)>,
    equality: Equality,
    functions: Vec<ir::Function>,
    default_scopes: Vec<(usize, std::collections::HashSet<String>)>,
}
impl Checker<'_> {
    fn error(&self, span: Span, code: &'static str, message: &str) -> Diagnostic {
        Diagnostic::at(self.source, span.start as usize, code, message)
    }
    fn unsupported(&self, span: Span) -> Diagnostic {
        self.error(
            span,
            "unsupported",
            "This form is outside the implemented slices",
        )
    }
    fn descend(&self, depth: u32, span: Span) -> Result<u32, Diagnostic> {
        let next = depth + 1;
        if next >= self.limit {
            return Err(self.error(span, "limit:nesting-depth", "Checker depth limit exceeded"));
        }
        Ok(next)
    }
    fn ty(&mut self, t: &TSType<'_>, result: bool, depth: u32) -> Result<Type, Diagnostic> {
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
            TSType::TSArrayType(a) => {
                let element = unparen(&a.element_type);
                if matches!(element, TSType::TSNullKeyword(_))
                    || matches!(element, TSType::TSUnionType(u) if u.types.iter().any(|t| matches!(unparen(t), TSType::TSNullKeyword(_))))
                {
                    return Err(self.error(a.span, "C6", "Array elements must exclude null"));
                }
                let element = self.ty(&a.element_type, false, depth)?;
                self.types.array(element)
            }
            TSType::TSFunctionType(f) => {
                if f.type_parameters.is_some() || f.this_param.is_some() {
                    return Err(self.error(f.span, "C11", "Invalid function type"));
                }
                let (params, required) = self.signature(&f.params, false, depth)?;
                let result = self.ty(&f.return_type.type_annotation, true, depth)?;
                self.types.intern(params, result, required)
            }
            TSType::TSParenthesizedType(p) => return self.ty(&p.type_annotation, result, depth),
            TSType::TSNullKeyword(_) | TSType::TSTypeReference(_) | TSType::TSTypeLiteral(_) => {
                return Err(self.unsupported(t.span()));
            }
            _ => return Err(self.error(t.span(), "C11", "Type is outside the closed surface")),
        })
    }
    fn bind(&mut self, name: &str, binding: ir::Binding, span: Span) -> Result<usize, Diagnostic> {
        if strict::binding(name) {
            return Err(self.error(span, "C11", "Binding name is forbidden in a strict module"));
        }
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
        self.equality.bindings.push(true);
        self.equality.aliases.push([vec![], vec![]]);
        self.equality.tested.push(false);
        Ok(id)
    }
    fn lookup(&mut self, name: &str, span: Span) -> Result<usize, Diagnostic> {
        self.default_reference(name, span)?;
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
        if !binding.declaration && span.start < binding.end {
            return Err(self.error(span, "C10", "Reference precedes the end of its declaration"));
        }
        self.references.push((self.owner, id, span.start));
        Ok(id)
    }
    fn default_reference(&self, name: &str, span: Span) -> Result<(), Diagnostic> {
        for (scope, names) in &self.default_scopes {
            if names.contains(name) && !self.scopes[*scope..].iter().any(|s| s.contains_key(name)) {
                return Err(self.error(
                    span,
                    "C11",
                    "Default refers to a top-level body declaration",
                ));
            }
        }
        Ok(())
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
                    declaration: false,
                },
                v.span,
            )?;
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
        types: ir::Types::new(),
        loops: vec![],
        reachable: true,
        limit,
        references: vec![],
        equality: Equality::new(),
        functions: vec![],
        default_scopes: vec![],
    };
    if let Some(hashbang) = &program.hashbang {
        return Err(c.error(hashbang.span, "C11", "Hashbang lines are rejected"));
    }
    if !program.directives.is_empty() {
        return Err(c.error(program.span, "C11", "Module directives are rejected"));
    }
    let mut declarations = Vec::new();
    for s in &program.body {
        match s {
            Statement::FunctionDeclaration(f) => {
                declarations.push((c.function_name(f, 0)?, f.as_ref(), false));
            }
            Statement::ExportDeclaration(e) => match &e.declaration {
                Declaration::FunctionDeclaration(f) => {
                    declarations.push((c.function_name(f, 0)?, f.as_ref(), true));
                }
                _ => return Err(c.unsupported(e.span)),
            },
            Statement::VariableDeclaration(d) => c.variable_names(d)?,
            Statement::TSInterfaceDeclaration(_) => {
                return Err(c.error(s.span(), "C8", "Interface declarations have no lowering"));
            }
            Statement::ClassDeclaration(_) | Statement::TSTypeAliasDeclaration(_) => {
                return Err(c.unsupported(s.span()));
            }
            _ => return Err(c.error(s.span(), "C11", "Module statements must be declarations")),
        }
    }
    if !declarations.iter().any(|(id, f, exported)| {
        *exported
            && f.id.as_ref().is_some_and(|n| n.name == "main")
            && c.bindings[*id]
                .ty
                .and_then(|ty| c.types.function(ty))
                .is_some_and(|f| f.params.is_empty() && f.result == Type::Void)
    }) {
        return Err(c.unsupported(program.span));
    }
    let mut body = vec![ir::Statement::Entry(
        c.scopes[0].values().copied().collect(),
    )];
    for s in &program.body {
        if let Statement::VariableDeclaration(d) = s {
            body.extend(c.variables(d, 0)?);
        }
    }
    c.equality.restore(0);
    c.reachable = true;
    for (id, f, exported) in declarations {
        c.function_body(id, f, exported, 0)?;
        body.insert(1, ir::Statement::Function(id));
    }
    // Restore source order after function bodies add their nested declarations.
    body[1..].sort_by_key(|s| match s.form() {
        ir::Statement::Function(id) => (0, c.bindings[*id].start),
        ir::Statement::Declare(id, _) => (1, c.bindings[*id].start),
        _ => (1, 0),
    });
    c.tdz()?;
    let mut captured: Vec<_> = c.bindings.iter().map(|b| b.declaration).collect();
    for &(owner, id, _) in &c.references {
        captured[id] |= owner != c.bindings[id].owner;
    }
    Ok(ir::Module {
        types: c.types,
        captured,
        bindings: c.bindings,
        functions: c.functions,
        body,
        depth: limit,
        source: source.to_vec(),
    })
}
