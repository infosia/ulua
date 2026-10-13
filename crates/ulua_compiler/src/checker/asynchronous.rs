//! Async body rules and the private suspension expression.
use super::*;
use crate::ir::{ExprKind, Type};

impl Checker<'_> {
    pub(super) fn promise_declaration(&self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if name == "Promise" {
            return Err(self.error(
                span,
                "C19",
                "A class or type alias named Promise is rejected",
            ));
        }
        Ok(())
    }
    pub(super) fn suspends(&self, span: Span) -> bool {
        self.suspensions
            .get(&self.source_owner)
            .is_some_and(|offsets| {
                let index = offsets.partition_point(|offset| *offset < span.start);
                offsets.get(index).is_some_and(|offset| *offset < span.end)
            })
    }
    pub(super) fn promise_type(
        &mut self,
        r: &TSTypeReference<'_>,
        result: bool,
        depth: u32,
    ) -> Result<Type, Diagnostic> {
        let args = r
            .type_arguments
            .as_ref()
            .ok_or_else(|| self.error(r.span, "C19", "Promise needs one type argument"))?;
        if args.params.len() != 1 {
            return Err(self.error(r.span, "C19", "Promise needs one type argument"));
        }
        if !result && matches!(unparen(&args.params[0]), TSType::TSVoidKeyword(_)) {
            return Err(self.error(r.span, "C19", "Promise<void> is an async result only"));
        }
        let value = self.ty(&args.params[0], result, depth)?;
        if self
            .types
            .promise_value(self.types.inner(value).unwrap_or(value))
            .is_some()
        {
            return Err(self.error(
                r.span,
                "C19",
                "a Promise type cannot contain a Promise type",
            ));
        }
        self.uses_async = true;
        if self.aliases_ready {
            self.promise_member(value, r.span)?;
        } else {
            // Forward record and class members are complete after the alias seal.
            self.promise_annotations.push((value, r.span));
        }
        let ty = self.types.promise(value);
        self.checked_type_depth(ty, r.span)
    }
    pub(super) fn async_result(
        &mut self,
        asynchronous: bool,
        result: Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if asynchronous {
            let value = self
                .types
                .promise_value(result)
                .ok_or_else(|| self.error(span, "C19", "async result has a Promise type"))?;
            if self.aliases_ready {
                self.promise_member(value, span)?;
            }
            self.uses_async = true;
        } else if self.types.promise_value(result) == Some(Type::Void) {
            return Err(self.error(span, "C19", "Promise<void> is an async result only"));
        }
        Ok(())
    }
    pub(super) fn body_result(&self, result: Type) -> Type {
        if self.async_body {
            self.types.promise_value(result).unwrap_or(result)
        } else {
            result
        }
    }
    pub(super) fn return_flow(
        &self,
        from: Type,
        result: Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let result = self.body_result(result);
        let from = if self.async_body {
            self.promise_member(self.types.promise_value(from).unwrap_or(from), span)?;
            if let Some(value) = self.types.promise_value(from) {
                if !self.types.equal(value, result) {
                    return Err(self.error(
                        span,
                        "C19",
                        "a promise relates only to an identical promise type",
                    ));
                }
                value
            } else {
                from
            }
        } else {
            from
        };
        self.flow(&from, Some(&result), span)
    }
    pub(super) fn promise_member(&self, value: Type, span: Span) -> Result<(), Diagnostic> {
        let value = self.types.inner(value).unwrap_or(value);
        if self
            .fields
            .get("then")
            .is_some_and(|key| self.types.has_member(value, *key))
        {
            return Err(self.error(span, "C19", "a Promise result has no member named then"));
        }
        Ok(())
    }
    pub(super) fn async_return_literal(&self, mut e: &Expression<'_>) -> Result<(), Diagnostic> {
        if !self.async_body {
            return Ok(());
        }
        while let Expression::ParenthesizedExpression(p) = e {
            e = &p.expression;
        }
        // Check the value before a contextual record shape can reject an extra member.
        if let Expression::ObjectExpression(object) = e {
            for entry in &object.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = entry
                    && matches!(&p.key, PropertyKey::StaticIdentifier(n) if n.name == "then")
                {
                    return Err(self.error(
                        p.span,
                        "C19",
                        "a Promise result has no member named then",
                    ));
                }
            }
        }
        Ok(())
    }
    pub(super) fn async_array_argument(&self, mut e: &Expression<'_>) -> Result<(), Diagnostic> {
        while let Expression::ParenthesizedExpression(p) = e {
            e = &p.expression;
        }
        if matches!(e, Expression::ArrowFunctionExpression(a) if a.r#async) {
            return Err(self.error(
                e.span(),
                "C19",
                "an async arrow is not an array method argument",
            ));
        }
        Ok(())
    }
    pub(super) fn promise_operator(&self, ty: Type, span: Span) -> Result<(), Diagnostic> {
        let ty = self.types.inner(ty).unwrap_or(ty);
        if self.types.promise_value(ty).is_some() {
            return Err(self.error(span, "C19", "a promise value has no operator or member"));
        }
        Ok(())
    }
    pub(super) fn await_expression(
        &mut self,
        a: &AwaitExpression<'_>,
        depth: u32,
    ) -> Result<(Type, ExprKind), Diagnostic> {
        if !self.async_body {
            return Err(self.error(a.span, "C19", "await is only accepted in an async body"));
        }
        let value = self.expr(&a.argument, depth, false)?;
        let ty = self
            .types
            .promise_value(value.ty)
            .ok_or_else(|| self.error(a.span, "C19", "the operand of await has a Promise type"))?;
        self.facts.end_enclosing(self.owner);
        Ok((ty, ExprKind::Await(Box::new(value))))
    }
}
