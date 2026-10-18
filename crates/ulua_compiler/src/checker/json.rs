//! Static JSON targets and declaration-order reachability.
use super::*;
use crate::ir::{Expr, ExprKind};
use std::collections::HashSet;
impl Checker<'_> {
    pub(super) fn json_call(
        &mut self,
        call: &CallExpression<'_>,
        member: &StaticMemberExpression<'_>,
        expected: Option<Type>,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let parse = match member.property.name.as_str() {
            "parse" => true,
            "stringify" => false,
            _ => return Err(self.error(call.span, "C20", "Unknown JSON member")),
        };
        if call.optional || member.optional || call.type_arguments.is_some() {
            return Err(self.error(call.span, "C20", "JSON needs a direct call"));
        }
        if call.arguments.len() != 1 {
            return Err(self.error(
                call.span,
                "C20",
                if parse {
                    "JSON.parse takes exactly one argument"
                } else {
                    "JSON.stringify takes exactly one argument"
                },
            ));
        }
        let arg = call.arguments[0]
            .as_expression()
            .ok_or_else(|| self.error(call.span, "C20", "JSON spread is rejected"))?;
        let target =
            if parse {
                Some(expected.ok_or_else(|| {
                    self.error(call.span, "C20", "JSON.parse needs a target type")
                })?)
            } else {
                None
            };
        let value = self.expr_as(
            arg,
            depth,
            false,
            if parse { Some(Type::String) } else { None },
        )?;
        if parse {
            self.flow(&value.ty, Some(&Type::String), arg.span())?;
        }
        let ty = target.unwrap_or(value.ty);
        let mut pending = vec![ty];
        let mut seen = HashSet::new();
        while let Some(t) = pending.pop() {
            if !seen.insert(t) {
                continue;
            }
            let rejected = if self.types.is_absent(t) {
                Some("the absent type")
            } else if self.types.function(t).is_some() {
                Some("a function type")
            } else if let Some((_, v)) = self.types.table_parts(t) {
                Some(if v.is_some() { "a Map" } else { "a Set" })
            } else if self.types.promise_value(t).is_some() {
                Some("a promise")
            } else if t == Type::Error {
                Some("an Error-family value")
            } else if parse && self.types.is_class(t) {
                Some("a class")
            } else {
                None
            };
            if let Some(reason) = rejected {
                return Err(self.error(
                    call.span,
                    "C20",
                    &format!(
                        "JSON.{} of {reason} is rejected",
                        if parse { "parse" } else { "stringify" }
                    ),
                ));
            }
            if let Some(t) = self.types.inner(t).or_else(|| self.types.element(t)) {
                pending.push(t);
            } else if let Some(fields) = self.types.fields(t) {
                pending.extend(fields.iter().map(|f| f.1));
            } else if !matches!(t, Type::Number | Type::String | Type::Boolean | Type::Null) {
                return Err(self.error(call.span, "C20", "JSON target is not serializable"));
            }
        }
        self.uses_exceptions = true;
        let id = self
            .json
            .iter()
            .position(|e| *e == (parse, ty))
            .unwrap_or_else(|| {
                let id = self.json.len();
                self.json.push((parse, ty));
                id
            });
        Ok(Expr {
            flow: None,
            bare_true: false,
            index_form: crate::ir::IndexForm::Checked,
            start: call.span.start,
            ty: if parse { ty } else { Type::String },
            kind: ExprKind::Json(id, Box::new(value)),
        })
    }
    pub(super) fn json_order(&self) -> Result<(), Diagnostic> {
        let mut reachable = HashSet::new();
        let mut pending: Vec<_> = self.json.iter().filter(|e| !e.0).map(|e| e.1).collect();
        while let Some(t) = pending.pop() {
            if !reachable.insert(t) {
                continue;
            }
            if let Some(t) = self.types.inner(t).or_else(|| self.types.element(t)) {
                pending.push(t);
            }
            if let Some(fields) = self.types.fields(t) {
                pending.extend(fields.iter().map(|f| f.1));
            }
        }
        for (ty, keys, span) in &self.json_literals {
            for target in &reachable {
                if !self.types.equal(*ty, *target) {
                    continue;
                }
                if let Some(fields) = self.types.declaration_fields(*target)
                    && !keys.iter().copied().eq(fields.iter().map(|f| f.0))
                {
                    return Err(self.error(*span, "C20", "an object literal that reaches JSON.stringify lists its fields in declaration order"));
                }
            }
        }
        Ok(())
    }
}
