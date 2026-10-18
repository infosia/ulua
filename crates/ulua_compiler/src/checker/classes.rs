//! Nominal declarations retain each direct method target and one constructor target.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement as S};

pub(super) struct ClassInfo {
    pub(super) constructor: usize,
    pub(super) methods: HashMap<usize, usize>,
    private: std::collections::HashSet<usize>,
}

impl Checker<'_> {
    pub(super) fn reserve_class(&mut self, class: &Class<'_>) -> Result<(), Diagnostic> {
        if class.heritage.is_some()
            || !class.implements.is_empty()
            || class.type_parameters.is_some()
            || !class.decorators.is_empty()
            || class.r#abstract
            || class.declare
        {
            return Err(self.error(class.span, "C11", "Invalid class declaration"));
        }
        let name = class
            .id
            .as_ref()
            .ok_or_else(|| self.unsupported(class.span))?;
        self.promise_declaration(name.name.as_str(), class.span)?;
        let ty = self.types.reserve_class();
        if self.aliases.insert(name.name.to_string(), ty).is_some() {
            return Err(self.error(class.span, "C11", "Duplicate type declaration"));
        }
        let id = self.bind(
            name.name.as_str(),
            ir::Binding {
                ty: None,
                owner: 0,
                start: class.span.start,
                end: class.span.end,
                mutable: false,
                declaration: false,
            },
            class.span,
        )?;
        self.class_bindings.insert(id);
        self.classes.insert(
            ty,
            ClassInfo {
                constructor: id,
                methods: HashMap::new(),
                private: Default::default(),
            },
        );
        Ok(())
    }
    fn member_key(&mut self, key: &PropertyKey<'_>, span: Span) -> Result<usize, Diagnostic> {
        let PropertyKey::StaticIdentifier(n) = key else {
            return Err(self.error(span, "C11", "Class member key needs an identifier"));
        };
        if n.name == "__proto__" {
            return Err(self.error(span, "C1", "A class member named __proto__ is rejected"));
        }
        Ok(self.field_key(n.name.as_str()))
    }
    fn class_type(&self, class: &Class<'_>) -> Result<Type, Diagnostic> {
        class
            .id
            .as_ref()
            .and_then(|n| self.aliases.get(n.name.as_str()))
            .copied()
            .ok_or_else(|| self.error(class.span, "internal", "Missing class identity"))
    }
    fn internal_binding(
        &mut self,
        ty: Type,
        span: Span,
        declaration: bool,
    ) -> Result<usize, Diagnostic> {
        self.scopes.push(HashMap::new());
        let id = self.bind(
            "",
            ir::Binding {
                ty: Some(ty),
                owner: self.owner,
                start: span.start,
                end: 0,
                mutable: false,
                declaration,
            },
            span,
        )?;
        self.scopes.pop();
        Ok(id)
    }
    pub(super) fn class_signature(&mut self, class: &Class<'_>) -> Result<(), Diagnostic> {
        let ty = self.class_type(class)?;
        let mut fields = vec![];
        let mut methods = vec![];
        let mut targets = HashMap::new();
        let mut private = std::collections::HashSet::new();
        let mut seen = std::collections::HashSet::new();
        let mut constructor = None;
        for member in &class.body.body {
            let key = match member {
                ClassElement::PropertyDefinition(p) => {
                    let key = self.member_key(&p.key, p.span)?;
                    if p.accessibility == Some(TSAccessibility::Private) {
                        private.insert(key);
                    }
                    if p.computed
                        || p.r#static
                        || p.declare
                        || p.readonly
                        || p.optional
                        || p.definite
                        || p.r#override
                        || !p.decorators.is_empty()
                        || p.r#type != PropertyDefinitionType::PropertyDefinition
                        || p.accessibility
                            .is_some_and(|a| a != TSAccessibility::Private)
                    {
                        return Err(self.error(p.span, "C11", "Invalid class field"));
                    }
                    let annotation = p.type_annotation.as_ref().ok_or_else(|| {
                        self.error(p.span, "C13", "Class field needs an annotation")
                    })?;
                    fields.push((key, self.ty(&annotation.type_annotation, false, 0)?));
                    key
                }
                ClassElement::MethodDefinition(m) => {
                    let key = self.member_key(&m.key, m.span)?;
                    if m.accessibility == Some(TSAccessibility::Private) {
                        private.insert(key);
                    }
                    let f = &m.value;
                    if f.generator || (f.r#async && m.kind == MethodDefinitionKind::Constructor) {
                        return Err(self.error(
                            m.span,
                            "C19",
                            "async constructors and generators are rejected",
                        ));
                    }
                    if m.computed
                        || m.r#static
                        || m.optional
                        || m.r#override
                        || !m.decorators.is_empty()
                        || m.r#type != MethodDefinitionType::MethodDefinition
                        || m.accessibility
                            .is_some_and(|a| a != TSAccessibility::Private)
                        || f.generator
                        || f.declare
                        || f.this_param.is_some()
                        || f.type_parameters.is_some()
                        || f.body.is_none()
                        || !matches!(
                            m.kind,
                            MethodDefinitionKind::Constructor | MethodDefinitionKind::Method
                        )
                    {
                        return Err(self.error(m.span, "C11", "Invalid class method"));
                    }
                    let (params, required) = self.signature(&f.params, true, 0)?;
                    if m.kind == MethodDefinitionKind::Constructor {
                        if constructor.is_some()
                            || f.return_type.is_some()
                            || m.accessibility.is_some()
                        {
                            return Err(self.error(m.span, "C11", "Invalid constructor"));
                        }
                        constructor = Some(self.types.intern(params, ty, required));
                        continue;
                    }
                    let annotation = f.return_type.as_ref().ok_or_else(|| {
                        self.error(
                            m.span,
                            if f.r#async { "C19" } else { "C11" },
                            "Method result annotation is required",
                        )
                    })?;
                    let result = self.ty(&annotation.type_annotation, true, 0)?;
                    self.async_result(f.r#async, result, m.span)?;
                    methods.push((key, self.types.intern(params.clone(), result, required)));
                    let full = self.types.intern(
                        std::iter::once(ty).chain(params).collect(),
                        result,
                        required + 1,
                    );
                    targets.insert(key, self.internal_binding(full, m.span, true)?);
                    key
                }
                _ => return Err(self.error(member.span(), "C11", "Invalid class member")),
            };
            if !seen.insert(key) {
                return Err(self.error(member.span(), "C11", "Duplicate class member"));
            }
            if seen.len() > crate::FIELD_BOUND {
                return Err(self.error(
                    member.span(),
                    "limit:fields",
                    "Class field and method limit exceeded",
                ));
            }
        }
        self.types.define_class(ty, fields, methods);
        let ctor_ty = constructor.unwrap_or_else(|| self.types.intern(vec![], ty, 0));
        let info = self.classes.get_mut(&ty).ok_or_else(|| {
            Diagnostic::at(
                self.source,
                class.span.start as usize,
                "internal",
                "Missing class declaration",
            )
        })?;
        self.bindings[info.constructor].ty = Some(ctor_ty);
        info.methods = targets;
        info.private = private;
        Ok(())
    }
    pub(super) fn class_body(&mut self, class: &Class<'_>) -> Result<(), Diagnostic> {
        let ty = self.class_type(class)?;
        let previous_context = self.class_context.replace(ty);
        let ctor = self
            .classes
            .get(&ty)
            .ok_or_else(|| self.error(class.span, "internal", "Missing class declaration"))?
            .constructor;
        let state = self.enter_function(ctor, Type::Void, false, class.span.start);
        // Field initializers see module names, never constructor parameters or locals.
        self.field_initializer = true;
        let mut fields = vec![];
        let mut missing = std::collections::HashSet::new();
        for member in &class.body.body {
            if let ClassElement::PropertyDefinition(p) = member {
                let key = self.member_key(&p.key, p.span)?;
                let field_ty = self
                    .types
                    .field(ty, key)
                    .ok_or_else(|| self.error(p.span, "internal", "Missing field type"))?;
                if let Some(e) = &p.value {
                    // The generated constructor, declaration, and table initializer occupy three depth steps.
                    let value = self.expr_as(e, 3, false, Some(field_ty))?;
                    self.flow(&value.ty, Some(&field_ty), p.span)?;
                    fields.push((key, value));
                } else {
                    missing.insert(key);
                }
            }
        }
        self.field_initializer = false;
        self.in_constructor = true;
        let receiver = self.internal_binding(ty, class.span, false)?;
        self.this_binding = Some(receiver);
        let constructor = class.body.body.iter().find_map(|m| match m {
            ClassElement::MethodDefinition(m) if m.kind == MethodDefinitionKind::Constructor => {
                Some(m)
            }
            _ => None,
        });
        let (parameters, mut body) = if let Some(m) = constructor {
            let b = m
                .value
                .body
                .as_ref()
                .ok_or_else(|| self.unsupported(m.span))?;
            if !b.directives.is_empty() {
                return Err(self.error(b.span, "C11", "Constructor directives are rejected"));
            }
            let parameters = self.parameters(&m.value.params, &b.statements, 0)?;
            let body = self.block_contents(&b.statements, 1)?;
            (parameters, body)
        } else {
            (vec![], vec![])
        };
        construction::check(self, &body, &parameters, receiver, missing, class.span)?;
        construction::receiver_returns(&mut body, receiver, ty, class.span.start);
        body.push(S::Return(Some(value(
            ty,
            E::Read(receiver),
            class.span.start,
        ))));
        let initializers = vec![
            S::Entry(vec![receiver]),
            S::Declare(receiver, value(ty, E::Object(fields), class.span.start)),
        ];
        self.leave_function(state);
        self.functions.push(ir::Function {
            inline_callback: None,
            asynchronous: false,
            binding: ctor,
            name: format!("class{ctor}"),
            exported: false,
            parameters,
            result: ty,
            initializers,
            body,
        });
        for member in &class.body.body {
            if let ClassElement::MethodDefinition(m) = member {
                if m.kind == MethodDefinitionKind::Constructor {
                    continue;
                }
                let key = self.member_key(&m.key, m.span)?;
                let id = self
                    .classes
                    .get(&ty)
                    .and_then(|info| info.methods.get(&key))
                    .copied()
                    .ok_or_else(|| self.error(m.span, "internal", "Missing class method target"))?;
                let result = self
                    .types
                    .signature(
                        self.bindings[id]
                            .ty
                            .ok_or_else(|| self.unsupported(m.span))?,
                    )
                    .ok_or_else(|| self.unsupported(m.span))?
                    .result;
                let state = self.enter_function(id, result, false, m.value.span.start);
                self.async_body = m.value.r#async;
                self.result = self.body_result(result);
                let receiver = self.internal_binding(ty, m.span, false)?;
                self.this_binding = Some(receiver);
                let b = m
                    .value
                    .body
                    .as_ref()
                    .ok_or_else(|| self.unsupported(m.span))?;
                if !b.directives.is_empty() {
                    return Err(self.error(b.span, "C11", "Method directives are rejected"));
                }
                let mut parameters = vec![(receiver, None)];
                parameters.extend(self.parameters(&m.value.params, &b.statements, 0)?);
                let body = self.block_contents(&b.statements, 1)?;
                if self.result != Type::Void && !super::statements::returns(&body) {
                    return Err(self.error(m.span, "C11", "Method can finish without its result"));
                }
                self.leave_function(state);
                self.functions.push(ir::Function {
                    inline_callback: None,
                    asynchronous: m.value.r#async,
                    binding: id,
                    name: format!("method{id}"),
                    exported: false,
                    parameters,
                    result,
                    initializers: vec![],
                    body,
                });
            }
        }
        self.class_context = previous_context;
        Ok(())
    }
    pub(super) fn this(&mut self, span: Span) -> Result<(Type, E), Diagnostic> {
        if self.field_initializer {
            return Err(self.error(span, "C13", "A field initializer cannot use this"));
        }
        let id = self.this_binding.ok_or_else(|| {
            self.error(span, "C11", "This needs a method or constructor receiver")
        })?;
        self.references.push((self.owner, id, span.start));
        Ok((
            self.bindings[id].ty.ok_or_else(|| self.unsupported(span))?,
            E::Read(id),
        ))
    }
    pub(super) fn method_target(&self, ty: Type, key: usize) -> Option<usize> {
        self.classes.get(&ty)?.methods.get(&key).copied()
    }
    pub(super) fn reject_method_value(
        &self,
        ty: Type,
        key: usize,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.member_access(ty, key, span)?;
        if self.method_target(ty, key).is_some() {
            return Err(self.error(
                span,
                "C11",
                "A method cannot be a value or an assignment target",
            ));
        }
        Ok(())
    }
    pub(super) fn member_access(&self, ty: Type, key: usize, span: Span) -> Result<(), Diagnostic> {
        self.promise_operator(ty, span)?;
        if self
            .classes
            .get(&ty)
            .is_some_and(|info| info.private.contains(&key))
            && self.class_context != Some(ty)
        {
            return Err(self.error(span, "C11", "A private member needs its class body"));
        }
        Ok(())
    }
    pub(super) fn class_arguments(
        &mut self,
        id: usize,
        args: &[Argument<'_>],
        depth: u32,
        span: Span,
        method: bool,
    ) -> Result<Vec<Expr>, Diagnostic> {
        if args.len() > crate::ARGUMENT_BOUND {
            return Err(self.error(span, "limit:arguments", "Argument limit exceeded"));
        }
        let signature = self
            .types
            .signature(self.bindings[id].ty.ok_or_else(|| self.unsupported(span))?)
            .ok_or_else(|| self.unsupported(span))?;
        let skip = usize::from(method);
        if args.len() + skip < signature.required || args.len() + skip > signature.params.len() {
            return Err(self.error(span, "C1", "Argument count does not agree"));
        }
        let mut out = vec![];
        for (arg, ty) in args.iter().zip(&signature.params[skip..]) {
            let e = arg
                .as_expression()
                .ok_or_else(|| self.error(arg.span(), "C11", "Spread arguments are rejected"))?;
            let value = self.expr_as(e, depth, false, Some(*ty))?;
            self.flow(&value.ty, Some(ty), arg.span())?;
            out.push(value);
        }
        Ok(out)
    }
    pub(super) fn class_call(
        &mut self,
        id: usize,
        receiver: Expr,
        args: &[Argument<'_>],
        optional: bool,
        depth: u32,
        span: Span,
    ) -> Result<(Type, E), Diagnostic> {
        self.references.push((self.owner, id, span.start));
        let ty = self.bindings[id].ty.ok_or_else(|| self.unsupported(span))?;
        let result = self
            .types
            .signature(ty)
            .ok_or_else(|| self.unsupported(span))?
            .result;
        let args = self.class_arguments(id, args, depth, span, true)?;
        if optional && self.types.promise_value(result).is_some() {
            return Err(self.error(
                span,
                "C19",
                "a promise value is stored, passed, returned, or awaited",
            ));
        }
        Ok((
            if optional { Type::Void } else { result },
            E::ClassCall(
                Box::new(value(ty, E::Read(id), span.start)),
                Box::new(receiver),
                args,
                optional,
            ),
        ))
    }
    pub(super) fn new_instance(
        &mut self,
        new: &NewExpression<'_>,
        depth: u32,
    ) -> Result<(Type, E), Diagnostic> {
        let Expression::Identifier(n) = &new.callee else {
            return Err(self.error(new.span, "C11", "New needs a class identifier"));
        };
        if new.type_arguments.is_some() {
            return Err(self.error(new.span, "C11", "New type arguments are rejected"));
        }
        let id = self.lookup(n.name.as_str(), n.span)?;
        if !self.class_bindings.contains(&id) {
            return Err(self.error(new.span, "C11", "New needs a class declaration"));
        }
        let ty = self.bindings[id]
            .ty
            .ok_or_else(|| self.unsupported(new.span))?;
        let result = self
            .types
            .signature(ty)
            .ok_or_else(|| self.unsupported(new.span))?
            .result;
        let args = self.class_arguments(id, &new.arguments, depth, new.span, false)?;
        Ok((
            result,
            E::Call(Box::new(value(ty, E::Read(id), new.span.start)), args),
        ))
    }
}
pub(super) fn value(ty: Type, kind: E, start: u32) -> Expr {
    Expr {
        flow: None,
        bare_true: false,
        index_form: crate::ir::IndexForm::Checked,
        start,
        ty,
        kind,
    }
}
