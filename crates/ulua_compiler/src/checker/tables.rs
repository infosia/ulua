//! P7.4 collection admission; iterator views exist only as direct for-of subjects.
use super::*;
use crate::{
    ir::{Expr, ExprKind as E},
    tables::TableMethod as M,
};
impl Checker<'_> {
    pub(super) fn table_type(
        &mut self,
        name: &str,
        args: Option<&[TSType<'_>]>,
        span: Span,
        depth: u32,
    ) -> Result<Type, Diagnostic> {
        let args = args.ok_or_else(|| self.error(span, "C6", "Map and Set need type arguments"))?;
        if args.len() != if name == "Map" { 2 } else { 1 } {
            return Err(self.error(span, "C6", "Map or Set type argument count is rejected"));
        }
        if matches!(unparen(&args[0]), TSType::TSNullKeyword(_)) {
            return Err(self.error(span, "C6", "a Map or Set key type has a defined equality"));
        }
        let k = self.ty(&args[0], false, depth)?;
        if !(matches!(k, Type::Number | Type::String | Type::Boolean)
            || self.types.element(k).is_some()
            || self.types.fields(k).is_some())
        {
            return Err(self.error(span, "C6", "a Map or Set key type has a defined equality"));
        }
        let v = if name == "Map" {
            Some(if matches!(unparen(&args[1]), TSType::TSNullKeyword(_)) {
                Type::Null
            } else {
                self.ty(&args[1], false, depth)?
            })
        } else {
            None
        };
        self.methods
            .insert((ir::Method::Table(M::New), M::key_kind(k)));
        let ty = self.types.table(k, v);
        self.checked_type_depth(ty, span)
    }
    fn table_call(
        &mut self,
        method: M,
        args: Vec<Expr>,
        ty: Type,
        span: Span,
    ) -> Result<Expr, Diagnostic> {
        let (key, _) = self.types.table_parts(ty).ok_or_else(|| {
            self.error(span, "internal", "Missing collection constructor key type")
        })?;
        let kind = M::key_kind(key);
        let name = match (method, kind) {
            (M::New, 0) => "__ulua_table_new_0",
            (M::New, _) => "__ulua_table_new_1",
            (M::FromArray, 0) => "__ulua_table_fromArray_0",
            (M::FromArray, _) => "__ulua_table_fromArray_1",
            _ => return Err(self.error(span, "internal", "Missing collection constructor helper")),
        };
        self.methods.insert((ir::Method::Table(method), kind));
        let signature = self
            .types
            .intern(args.iter().map(|a| a.ty).collect(), ty, args.len());
        let callee = super::classes::value(signature, E::Builtin(name), span.start);
        Ok(super::classes::value(
            ty,
            E::Call(Box::new(callee), args),
            span.start,
        ))
    }
    pub(super) fn new_table(
        &mut self,
        new: &NewExpression<'_>,
        expected: Option<Type>,
        depth: u32,
    ) -> Result<(Type, E), Diagnostic> {
        let Expression::Identifier(n) = &new.callee else {
            return Err(self.error(
                new.span,
                "internal",
                "Missing collection constructor identifier",
            ));
        };
        let ty = if new.type_arguments.is_some() {
            self.table_type(
                n.name.as_str(),
                new.type_arguments.as_ref().map(|a| a.params.as_slice()),
                new.span,
                depth,
            )?
        } else {
            expected
                .map(|t| self.types.inner(t).unwrap_or(t))
                .filter(|t| {
                    self.types
                        .table_parts(*t)
                        .is_some_and(|(_, v)| v.is_some() == (n.name == "Map"))
                })
                .ok_or_else(|| self.error(new.span, "C6", "Map and Set need type arguments"))?
        };
        if new.arguments.len() > 1 {
            return Err(self.error(
                new.span,
                "C6",
                "Map or Set constructor argument count is rejected",
            ));
        }
        let (key, value) = self.types.table_parts(ty).ok_or_else(|| {
            self.error(new.span, "internal", "Missing collection constructor type")
        })?;
        let mut args = vec![];
        let mut method = M::New;
        if let Some(arg) = new.arguments.first() {
            let arg = arg
                .as_expression()
                .ok_or_else(|| self.error(new.span, "C6", "Spread is rejected"))?;
            let expected = if value.is_none() {
                Some(self.types.array(key))
            } else {
                None
            };
            let source = self.expr_as(arg, depth, false, expected)?;
            if value.is_none()
                && self
                    .types
                    .element(source.ty)
                    .is_some_and(|k| self.types.equal(k, key))
            {
                method = M::FromArray;
            } else {
                if !self.types.equal(source.ty, ty) {
                    return Err(self.error(
                        arg.span(),
                        "C6",
                        "Map or Set constructor needs a matching collection or Set array source",
                    ));
                }
            }
            args.push(source);
        }
        let result = self.table_call(method, args, ty, new.span)?;
        Ok((result.ty, result.kind))
    }
    pub(super) fn table_method(
        &mut self,
        call: &CallExpression<'_>,
        name: &str,
        receiver: Expr,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let (key, value) = self
            .types
            .table_parts(receiver.ty)
            .ok_or_else(|| self.error(call.span, "internal", "Missing collection receiver type"))?;
        if name == "entries" {
            return Err(self.error(call.span, "C6", "entries needs a tuple type"));
        }
        let (method, expected, result) = match (name, value) {
            ("get", Some(v)) => {
                if self.types.contains_null(v) {
                    return Err(self.error(
                        call.span,
                        "C2",
                        "get needs a value type without null; use has and getOr",
                    ));
                }
                (M::Get, vec![key], self.types.absent(v))
            }
            ("getOr", Some(v)) => (M::GetOr, vec![key, v], v),
            ("set", Some(v)) => (M::Set, vec![key, v], receiver.ty),
            ("add", None) => (M::Add, vec![key], receiver.ty),
            ("has", _) => (M::Has, vec![key], Type::Boolean),
            ("delete", _) => (M::Delete, vec![key], Type::Boolean),
            ("clear", _) => (M::Clear, vec![], Type::Void),
            ("forEach", _) => (M::Each, vec![Type::Unknown], Type::Void),
            _ => {
                return Err(self.error(
                    call.span,
                    "C6",
                    "Map or Set member is rejected; keys and values require a for-of subject",
                ));
            }
        };
        if call.arguments.len() != expected.len() {
            return Err(self.error(call.span, "C6", "Map or Set argument count is rejected"));
        }
        let mut args = vec![];
        for (arg, expected) in call.arguments.iter().zip(expected) {
            let arg = arg
                .as_expression()
                .ok_or_else(|| self.error(call.span, "C6", "Spread is rejected"))?;
            let v = self.expr_as(
                arg,
                depth,
                false,
                if method == M::Each {
                    None
                } else {
                    Some(expected)
                },
            )?;
            if method == M::Each {
                let sig = self.types.signature(v.ty).ok_or_else(|| {
                    self.error(call.span, "C6", "Collection callback must be a function")
                })?;
                if self.types.promise_value(sig.result).is_some() {
                    return Err(self.error(
                        arg.span(),
                        "C6",
                        "await in a Map or Set traversal is rejected",
                    ));
                }
                let max = if value.is_some() { 2 } else { 1 };
                if !(1..=max).contains(&sig.params.len()) {
                    return Err(self.error(
                        call.span,
                        "C6",
                        "Callback parameter count is rejected",
                    ));
                }
                self.flow(&sig.params[0], Some(&value.unwrap_or(key)), arg.span())?;
                if sig.params.len() == 2 {
                    self.flow(&sig.params[1], Some(&key), arg.span())?;
                }
                // Void callbacks can discard their result, as the array forEach rule does.
            } else {
                self.flow(&v.ty, Some(&expected), arg.span())?;
            }
            args.push(v);
        }
        let kind = if method.keyed() { M::key_kind(key) } else { 0 };
        self.methods.insert((ir::Method::Table(method), kind));
        Ok(super::classes::value(
            result,
            E::Method(ir::Method::Table(method), kind, Box::new(receiver), args),
            call.span.start,
        ))
    }
}
