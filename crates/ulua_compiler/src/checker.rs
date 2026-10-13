//! P1 declarations, scope, value flows, and transitive declaration order.
mod arrays;
mod asynchronous;
mod calls;
mod captures;
mod classes;
mod construction;
mod equality;
mod exceptions;
mod expressions;
mod functions;
mod hosts;
mod inference;
mod methods;
mod narrowing;
mod order;
mod paths;
mod records;
mod statements;
mod strict;
mod strings;
use crate::{
    Diagnostic,
    ir::{self, Type},
};
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use std::collections::HashMap;

pub(crate) const UNREACHABLE_ARROW: &str = "an arrow function at a point that no edge reaches";

pub(crate) struct Checker<'s> {
    source: &'s [u8],
    host_names: HashMap<String, usize>,
    host_grants: Vec<crate::HostSignature>,
    host_ids: HashMap<String, usize>,
    hosts: Vec<(usize, String)>,
    catch_use: bool,
    uses_exceptions: bool,
    aliases: HashMap<String, Type>,
    classes: HashMap<Type, classes::ClassInfo>,
    class_bindings: std::collections::HashSet<usize>,
    this_binding: Option<usize>,
    field_initializer: bool,
    class_context: Option<Type>,
    in_constructor: bool,
    fields: HashMap<String, usize>,
    facts: narrowing::Facts,
    assignments: Vec<(u32, u32)>,
    suspensions: HashMap<u32, Vec<u32>>,
    source_owner: u32,
    captured_writes: std::collections::HashSet<u32>,
    binding_starts: HashMap<u32, usize>,
    methods: std::collections::BTreeSet<(ir::Method, usize)>,
    pub(super) bindings: Vec<ir::Binding>,
    scopes: Vec<HashMap<String, usize>>,
    owner: usize,
    result: Type,
    async_body: bool,
    uses_async: bool,
    promise_annotations: Vec<(Type, Span)>,
    aliases_ready: bool,
    types: ir::Types,
    reachable: bool,
    loops: usize,
    loop_facts: Vec<narrowing::LoopFacts>,
    limit: u32,
    references: Vec<(usize, usize, u32)>,
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
    fn checked_type_depth(&self, ty: Type, span: Span) -> Result<Type, Diagnostic> {
        if self.types.depth(ty) > crate::TYPE_DEPTH_BOUND {
            return Err(self.error(span, "limit:type-depth", "Type-link depth limit exceeded"));
        }
        Ok(ty)
    }
    fn ty(&mut self, t: &TSType<'_>, result: bool, depth: u32) -> Result<Type, Diagnostic> {
        let depth = self.descend(depth, t.span())?;
        let ty = match t {
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
                        .any(|t| matches!(unparen(t), TSType::TSNullKeyword(_)))
                {
                    let inner = u
                        .types
                        .iter()
                        .find(|t| !matches!(unparen(t), TSType::TSNullKeyword(_)))
                        .ok_or_else(|| {
                            self.error(t.span(), "C2", "Union needs a non-null member")
                        })?;
                    if matches!(
                        unparen(inner),
                        TSType::TSUnionType(_) | TSType::TSVoidKeyword(_)
                    ) {
                        return Err(self.error(t.span(), "C2", "Invalid nullable member"));
                    }
                    let inner = self.ty(inner, false, depth)?;
                    let ty = self.types.nullable(inner);
                    return self.checked_type_depth(ty, t.span());
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
            TSType::TSTypeReference(r) => {
                let TSTypeName::IdentifierReference(n) = &r.type_name else {
                    return Err(self.error(r.span, "C11", "Qualified type name is rejected"));
                };
                if n.name == "Promise" {
                    return self.promise_type(r, result, depth);
                }
                if r.type_arguments.is_some() {
                    return Err(self.error(r.span, "C11", "Type arguments are rejected"));
                }
                if !self.aliases.contains_key(n.name.as_str())
                    && exceptions::family(n.name.as_str()).is_some()
                {
                    self.uses_exceptions = true;
                    return Ok(Type::Error);
                }
                *self
                    .aliases
                    .get(n.name.as_str())
                    .ok_or_else(|| self.error(r.span, "C11", "Unknown type name"))?
            }
            TSType::TSNullKeyword(_) => {
                return Err(self.error(t.span(), "C2", "Null type needs a union"));
            }
            TSType::TSTypeLiteral(literal) => {
                self.reject_proto_type(literal)?;
                return Err(self.error(t.span(), "C11", "Object type needs a module alias"));
            }
            _ => return Err(self.error(t.span(), "C11", "Type is outside the closed surface")),
        };
        self.checked_type_depth(ty, t.span())
    }
    fn bind(&mut self, name: &str, binding: ir::Binding, span: Span) -> Result<usize, Diagnostic> {
        self.promise_declaration(name, span)?;
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
        self.binding_starts.insert(binding.start, id);
        self.facts.mutable.push(binding.mutable);
        self.facts.owners.push(binding.owner);
        self.bindings.push(binding);
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
                    if name == "undefined" {
                        "C2"
                    } else if hosts::known_global(name) {
                        "C11"
                    } else {
                        "C18"
                    },
                    "Unknown identifier",
                )
            })?;
        let binding = &self.bindings[id];
        let inside_class = self.class_bindings.contains(&id)
            && span.start >= binding.start
            && span.start < binding.end;
        if !binding.declaration && span.start < binding.end && !inside_class {
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
        if d.declare {
            return Err(self.error(d.span, "C18", "Script declare forms are rejected"));
        }
        if d.kind == VariableDeclarationKind::AwaitUsing {
            return Err(self.error(d.span, "C19", "await using is rejected"));
        }
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

#[cfg(test)]
pub(crate) fn check(
    program: &Program<'_>,
    source: &[u8],
    limit: u32,
) -> Result<ir::Module, Diagnostic> {
    check_with_grants(program, source, limit, &[])
}

pub(crate) fn check_with_grants(
    program: &Program<'_>,
    source: &[u8],
    limit: u32,
    grants: &[crate::HostSignature],
) -> Result<ir::Module, Diagnostic> {
    let writes = captures::resolve(program);
    let mut c = Checker {
        source,
        host_names: grants
            .iter()
            .enumerate()
            .map(|(i, g)| (g.name.clone(), i))
            .collect(),
        host_grants: grants.to_vec(),
        host_ids: HashMap::new(),
        hosts: vec![],
        catch_use: false,
        uses_exceptions: false,
        aliases: HashMap::new(),
        classes: HashMap::new(),
        class_bindings: Default::default(),
        this_binding: None,
        field_initializer: false,
        class_context: None,
        in_constructor: false,
        fields: HashMap::new(),
        facts: Default::default(),
        assignments: writes.assignments,
        suspensions: writes.suspensions,
        source_owner: u32::MAX,
        captured_writes: writes.captured,
        binding_starts: HashMap::new(),
        methods: Default::default(),
        bindings: vec![],
        scopes: vec![HashMap::new()],
        owner: 0,
        result: Type::Void,
        async_body: false,
        uses_async: false,
        promise_annotations: vec![],
        aliases_ready: false,
        types: ir::Types::new(),
        reachable: true,
        loops: 0,
        loop_facts: vec![],
        limit,
        references: vec![],
        functions: vec![],
        default_scopes: vec![],
    };
    if let Some(hashbang) = &program.hashbang {
        return Err(c.error(hashbang.span, "C11", "Hashbang lines are rejected"));
    }
    if !program.directives.is_empty() {
        return Err(c.error(program.span, "C11", "Module directives are rejected"));
    }
    c.aliases(program)?;
    let mut declarations = Vec::new();
    for s in &program.body {
        match s {
            Statement::FunctionDeclaration(f) => {
                declarations.push((c.function_name(f, 0)?, f.as_ref(), false, s.span().start));
            }
            Statement::ExportDeclaration(e) => match &e.declaration {
                Declaration::FunctionDeclaration(f) if f.declare => {
                    return Err(c.error(f.span, "C18", "Script declare forms are rejected"));
                }
                Declaration::FunctionDeclaration(f) => {
                    declarations.push((c.function_name(f, 0)?, f.as_ref(), true, s.span().start));
                }
                _ => return Err(c.unsupported(e.span)),
            },
            Statement::VariableDeclaration(d) => c.variable_names(d)?,
            Statement::ExportDefaultDeclaration(e) => {
                if let Some(expression) = e.declaration.as_expression() {
                    c.host_value_form(expression)?;
                }
                return Err(c.unsupported(e.span));
            }
            Statement::ExportNamedDeclaration(e)
                if e.specifiers
                    .iter()
                    .any(|s| c.host_names.contains_key(s.local.name().as_str())) =>
            {
                return Err(c.error(e.span, "C18", "A host function is only called"));
            }
            Statement::TSInterfaceDeclaration(_) => {
                return Err(c.error(s.span(), "C8", "Interface declarations have no lowering"));
            }
            Statement::TSTypeAliasDeclaration(_) => {}
            Statement::ClassDeclaration(_) => {}
            _ => return Err(c.error(s.span(), "C11", "Module statements must be declarations")),
        }
    }
    if !declarations.iter().any(|(id, f, exported, _)| {
        *exported
            && f.id.as_ref().is_some_and(|n| n.name == "main")
            && c.bindings[*id]
                .ty
                .and_then(|ty| c.types.function(ty))
                .is_some_and(|f| {
                    f.params.is_empty()
                        && (f.result == Type::Void
                            || c.types.promise_value(f.result) == Some(Type::Void))
                })
    }) {
        return Err(c.unsupported(program.span));
    }
    let first_offset = program.body.first().map_or(0, |s| s.span().start);
    let mut body = vec![ir::Statement::Located(
        first_offset,
        Box::new(ir::Statement::Entry(
            c.bindings
                .iter()
                .enumerate()
                .filter_map(|(id, b)| (b.owner == 0).then_some(id))
                .collect(),
        )),
    )];
    for s in &program.body {
        if let Statement::VariableDeclaration(d) = s {
            body.extend(c.variables(d, 0)?);
        }
    }
    for s in &program.body {
        if let Statement::ClassDeclaration(class) = s {
            c.class_body(class)?;
        }
    }
    for (id, f, exported, offset) in declarations {
        c.function_body(id, f, exported, 0)?;
        body.insert(
            1,
            ir::Statement::Located(offset, Box::new(ir::Statement::Function(id))),
        );
    }
    for info in c.classes.values() {
        let constructor = c
            .bindings
            .get(info.constructor)
            .ok_or_else(|| c.error(program.span, "internal", "Missing constructor binding"))?;
        body.push(ir::Statement::Located(
            constructor.start,
            Box::new(ir::Statement::Function(info.constructor)),
        ));
        for id in info.methods.values() {
            let method = c
                .bindings
                .get(*id)
                .ok_or_else(|| c.error(program.span, "internal", "Missing method binding"))?;
            body.push(ir::Statement::Located(
                method.start,
                Box::new(ir::Statement::Function(*id)),
            ));
        }
    }
    // Restore source order after function bodies add their nested declarations.
    body[1..].sort_by_key(|s| match s.form() {
        ir::Statement::Function(id) => (0, c.bindings[*id].start),
        ir::Statement::Declare(id, _) => (1, c.bindings[*id].start),
        _ => (1, 0),
    });
    c.hosts.sort_by_key(|(_, name)| c.host_names[name]);
    if let Some(ir::Statement::Located(_, entry)) = body.first_mut()
        && let ir::Statement::Entry(ids) = entry.as_mut()
    {
        ids.extend(c.hosts.iter().map(|(id, _)| *id));
    }
    c.tdz()?;
    let mut captured: Vec<_> = c.bindings.iter().map(|b| b.declaration).collect();
    for &(owner, id, _) in &c.references {
        captured[id] |= owner != c.bindings[id].owner;
    }
    Ok(ir::Module {
        hosts: c.hosts,
        exceptions: c.uses_exceptions || c.uses_async,
        asynchronous: c.uses_async,
        methods: c.methods,
        types: c.types,
        captured,
        bindings: c.bindings,
        functions: c.functions,
        body,
        depth: limit,
        source: source.to_vec(),
    })
}
