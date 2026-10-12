//! Module object aliases, generated field keys, and exact literal fields.
use super::*;
use crate::ir::{Expr, ExprKind};
impl Checker<'_> {
    fn reject_proto_key(&self, key: &PropertyKey<'_>, span: Span) -> Result<(), Diagnostic> {
        let proto = match key {
            PropertyKey::StaticIdentifier(n) => n.name == "__proto__",
            PropertyKey::StringLiteral(s) => s.value == "__proto__",
            _ => false,
        };
        if proto {
            return Err(self.error(span, "C1", "A field named __proto__ is rejected"));
        }
        Ok(())
    }
    pub(super) fn reject_proto_type(&self, literal: &TSTypeLiteral<'_>) -> Result<(), Diagnostic> {
        for member in &literal.members {
            if let TSSignature::TSPropertySignature(p) = member {
                self.reject_proto_key(&p.key, p.span)?;
            }
        }
        Ok(())
    }

    pub(super) fn field_key(&mut self, name: &str) -> usize {
        let next = self.fields.len();
        *self.fields.entry(name.to_owned()).or_insert(next)
    }
    pub(super) fn aliases(&mut self, program: &Program<'_>) -> Result<(), Diagnostic> {
        for s in &program.body {
            if let Statement::ClassDeclaration(class) = s {
                self.reserve_class(class)?;
            }
            if let Statement::TSTypeAliasDeclaration(a) = s {
                if strict::binding(a.id.name.as_str())
                    && !matches!(a.id.name.as_str(), "eval" | "arguments")
                {
                    return Err(self.error(
                        a.span,
                        "C11",
                        "Reserved type alias name in a strict module",
                    ));
                }
                if matches!(
                    a.id.name.as_str(),
                    "number"
                        | "boolean"
                        | "string"
                        | "symbol"
                        | "bigint"
                        | "void"
                        | "any"
                        | "unknown"
                        | "never"
                        | "object"
                        | "undefined"
                ) {
                    return Err(self.error(
                        a.span,
                        if a.id.name == "undefined" {
                            "C2"
                        } else {
                            "C11"
                        },
                        "A built-in type name cannot be redeclared",
                    ));
                }
                if a.type_parameters.is_some() || a.declare {
                    return Err(self.error(a.span, "C11", "Invalid object alias"));
                }
                let TSType::TSTypeLiteral(literal) = &a.type_annotation else {
                    return Err(self.error(a.span, "C11", "Alias needs an object type literal"));
                };
                if let Some(member) = literal.members.get(crate::FIELD_BOUND) {
                    return Err(self.error(member.span(), "limit:fields", "Field limit exceeded"));
                }
                self.reject_proto_type(literal)?;
                let ty = self.types.reserve_record();
                if self.aliases.insert(a.id.name.to_string(), ty).is_some() {
                    return Err(self.error(a.span, "C11", "Duplicate type alias"));
                }
            }
        }
        for s in &program.body {
            if let Statement::TSTypeAliasDeclaration(a) = s {
                let TSType::TSTypeLiteral(t) = &a.type_annotation else {
                    continue;
                };
                let mut fields = vec![];
                let mut seen = std::collections::HashSet::new();
                for member in &t.members {
                    let TSSignature::TSPropertySignature(p) = member else {
                        return Err(self.error(
                            member.span(),
                            if matches!(member, TSSignature::TSIndexSignature(_)) {
                                "C6"
                            } else {
                                "C11"
                            },
                            "Object type needs field signatures",
                        ));
                    };
                    if p.optional {
                        return Err(self.error(p.span, "C2", "Optional fields are rejected"));
                    }
                    let PropertyKey::StaticIdentifier(n) = &p.key else {
                        return Err(self.error(p.span, "C11", "Field key needs an identifier"));
                    };
                    if p.computed || p.readonly {
                        return Err(self.error(p.span, "C11", "Invalid field signature"));
                    }
                    let key = self.field_key(n.name.as_str());
                    if !seen.insert(key) {
                        return Err(self.error(p.span, "C11", "Duplicate field"));
                    }
                    let annotation = p
                        .type_annotation
                        .as_ref()
                        .ok_or_else(|| self.error(p.span, "C11", "Field needs an annotation"))?;
                    fields.push((key, self.ty(&annotation.type_annotation, false, 0)?));
                }
                let ty = self.aliases[a.id.name.as_str()];
                self.types.define_record(ty, fields);
            }
        }
        for s in &program.body {
            if let Statement::ClassDeclaration(class) = s {
                self.class_signature(class)?;
            }
        }
        self.types.seal_aliases();
        for s in &program.body {
            if let Statement::TSTypeAliasDeclaration(a) = s {
                self.checked_type_depth(self.aliases[a.id.name.as_str()], a.span)?;
            }
            if let Statement::ClassDeclaration(class) = s {
                let name = class
                    .id
                    .as_ref()
                    .ok_or_else(|| self.unsupported(class.span))?;
                let ty = self
                    .aliases
                    .get(name.name.as_str())
                    .copied()
                    .ok_or_else(|| self.error(class.span, "internal", "Missing class identity"))?;
                self.checked_type_depth(ty, class.span)?;
            }
        }
        for ty in self.aliases.values_mut() {
            let identity = self.types.identity(*ty);
            if self.types.depth(*ty) == self.types.depth(identity) {
                *ty = identity;
            }
        }
        Ok(())
    }
    pub(super) fn object(
        &mut self,
        object: &ObjectExpression<'_>,
        depth: u32,
        expected: Option<Type>,
    ) -> Result<(Type, ExprKind), Diagnostic> {
        if let Some(entry) = object.properties.get(crate::FIELD_BOUND) {
            return Err(self.error(entry.span(), "limit:fields", "Field limit exceeded"));
        }
        let expected = expected.map(|ty| self.types.inner(ty).unwrap_or(ty));
        if expected.is_some_and(|ty| self.types.is_class(ty)) {
            return Err(self.error(
                object.span,
                "C1",
                "An object literal cannot have a class type",
            ));
        }
        let mut entries = vec![];
        let mut fields = vec![];
        let mut seen = std::collections::HashSet::new();
        for entry in &object.properties {
            let ObjectPropertyKind::ObjectProperty(p) = entry else {
                return Err(self.error(entry.span(), "C11", "Object spread is rejected"));
            };
            self.reject_proto_key(&p.key, p.span)?;
            let PropertyKey::StaticIdentifier(n) = &p.key else {
                return Err(self.error(p.span, "C11", "Object key needs an identifier"));
            };
            if p.computed || p.shorthand || p.method || p.kind != PropertyKind::Init {
                return Err(self.error(p.span, "C11", "Object literal needs key: value entries"));
            }
            let key = self.field_key(n.name.as_str());
            if !seen.insert(key) {
                return Err(self.error(p.span, "C11", "Duplicate object key"));
            }
            let contextual = expected.and_then(|ty| self.types.field(ty, key));
            if contextual.is_none() {
                self.inferred_this(&p.value)?;
            }
            let value = self.expr_as(&p.value, depth, false, contextual)?;
            if let Some(ty) = contextual {
                self.flow(&value.ty, Some(&ty), p.span)?;
            } else {
                self.inferred_reference(&p.value, value.ty)?;
            }
            fields.push((key, contextual.unwrap_or(value.ty)));
            entries.push((key, value));
        }
        if let Some(expected) = expected {
            // A contextual literal uses that type, including its recursive component.
            let exact = self.types.fields(expected).is_some_and(|expected_fields| {
                expected_fields.len() == fields.len()
                    && fields
                        .iter()
                        .all(|(key, _)| self.types.field(expected, *key).is_some())
            });
            if !exact {
                return Err(self.error(object.span, "C1", "Value types do not agree"));
            }
            self.checked_type_depth(expected, object.span)?;
            return Ok((expected, ExprKind::Object(entries)));
        }
        let ty = self.types.record(fields);
        self.checked_type_depth(ty, object.span)?;
        if self.types.field_count(ty) > crate::FIELD_BOUND {
            return Err(self.error(
                object.span,
                "limit:fields",
                "Field limit includes nested anonymous object types",
            ));
        }
        Ok((ty, ExprKind::Object(entries)))
    }
    pub(super) fn optional_field(
        &mut self,
        chain: &ChainExpression<'_>,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let ChainElement::StaticMemberExpression(m) = &chain.expression else {
            return Err(self.error(chain.span, "C2", "Optional chain needs one terminal field"));
        };
        if !m.optional {
            return Err(self.error(chain.span, "C2", "Optional chain cannot continue"));
        }
        let mut receiver = self.expr(&m.object, depth, false)?;
        receiver.ty = self.original_type(&receiver);
        let inner = self
            .types
            .inner(receiver.ty)
            .ok_or_else(|| self.error(chain.span, "C2", "Optional receiver must include null"))?;
        if (inner == Type::String || self.types.element(inner).is_some())
            && m.property.name == "length"
        {
            let ty = self.types.nullable(Type::Number);
            return Ok(Expr {
                flow: None,
                bare_true: false,
                start: chain.span.start,
                ty,
                kind: ExprKind::OptionalLength(Box::new(receiver)),
            });
        }
        if inner == Type::Error {
            let key = exceptions::field(m.property.name.as_str())
                .ok_or_else(|| self.error(m.span, "C17", "Error has only name and message"))?;
            return Ok(Expr {
                flow: None,
                bare_true: false,
                start: chain.span.start,
                ty: self.types.nullable(Type::String),
                kind: ExprKind::OptionalErrorField(Box::new(receiver), key),
            });
        }
        let key = self.field_key(m.property.name.as_str());
        self.reject_method_value(inner, key, m.span)?;
        let field = self
            .types
            .field(inner, key)
            .ok_or_else(|| self.error(m.span, "C1", "Unknown optional field"))?;
        let ty = self.types.nullable(field);
        Ok(Expr {
            flow: None,
            bare_true: false,
            start: chain.span.start,
            ty,
            kind: ExprKind::OptionalField(Box::new(receiver), key),
        })
    }
    pub(super) fn optional_call(
        &mut self,
        chain: &ChainExpression<'_>,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let ChainElement::CallExpression(c) = &chain.expression else {
            return Err(self.error(chain.span, "C2", "Optional statement needs a terminal call"));
        };
        if c.arguments.len() > crate::ARGUMENT_BOUND {
            return Err(self.error(c.span, "limit:arguments", "Argument limit exceeded"));
        }
        let Expression::StaticMemberExpression(m) = &c.callee else {
            return Err(self.error(chain.span, "C2", "Optional function call is rejected"));
        };
        if !m.optional || c.optional || c.type_arguments.is_some() {
            return Err(self.error(chain.span, "C2", "Optional chain needs one terminal call"));
        }
        let mut receiver = self.expr(&m.object, depth, false)?;
        receiver.ty = self.original_type(&receiver);
        let inner = self
            .types
            .inner(receiver.ty)
            .ok_or_else(|| self.error(chain.span, "C2", "Optional receiver must include null"))?;
        if inner == Type::String || self.types.element(inner).is_some() {
            if let Some(element) = self.types.element(inner)
                && m.property.name == "push"
            {
                if c.arguments.len() != 1 {
                    return Err(self.error(c.span, "C1", "Push needs one argument"));
                }
                let arg = c.arguments[0]
                    .as_expression()
                    .ok_or_else(|| self.error(c.span, "C11", "Spread is rejected"))?;
                let value = self.expr_as(arg, depth, false, Some(element))?;
                self.flow(&value.ty, Some(&element), arg.span())?;
                return Ok(Expr {
                    flow: None,
                    bare_true: false,
                    start: chain.span.start,
                    ty: Type::Void,
                    kind: ExprKind::OptionalPush(Box::new(receiver), Box::new(value)),
                });
            }
            let element = self.types.element(inner);
            let nullable = receiver.ty;
            receiver.ty = inner;
            let method = if inner == Type::String {
                self.string_method(c, m.property.name.as_str(), receiver, depth)?
            } else {
                self.array_method(
                    c,
                    m.property.name.as_str(),
                    receiver,
                    element.ok_or_else(|| self.error(c.span, "C6", "Missing element type"))?,
                    depth,
                )?
            };
            let ExprKind::Method(method, arity, mut receiver, args) = method.kind else {
                return Err(self.error(c.span, "internal", "Missing method form"));
            };
            receiver.ty = nullable;
            return Ok(Expr {
                flow: None,
                bare_true: false,
                start: chain.span.start,
                ty: Type::Void,
                kind: ExprKind::OptionalMethod(method, arity, receiver, args),
            });
        }
        let key = self.field_key(m.property.name.as_str());
        self.member_access(inner, key, m.span)?;
        if let Some(id) = self.method_target(inner, key) {
            let (ty, kind) = self.class_call(id, receiver, &c.arguments, true, depth, c.span)?;
            return Ok(super::classes::value(ty, kind, chain.span.start));
        }
        let field = self
            .types
            .field(inner, key)
            .ok_or_else(|| self.error(m.span, "C1", "Unknown optional field"))?;
        let signature = self
            .types
            .signature(field)
            .ok_or_else(|| self.error(c.span, "C1", "Field must be a function"))?;
        if c.arguments.len() < signature.required || c.arguments.len() > signature.params.len() {
            return Err(self.error(c.span, "C1", "Argument count does not agree"));
        }
        let mut args = vec![];
        for (arg, ty) in c.arguments.iter().zip(&signature.params) {
            let e = arg
                .as_expression()
                .ok_or_else(|| self.error(arg.span(), "C11", "Spread is rejected"))?;
            let value = self.expr_as(e, depth, false, Some(*ty))?;
            self.flow(&value.ty, Some(ty), e.span())?;
            args.push(value);
        }
        Ok(Expr {
            flow: None,
            bare_true: false,
            start: chain.span.start,
            ty: Type::Void,
            kind: ExprKind::OptionalCall(Box::new(receiver), key, args),
        })
    }
}
