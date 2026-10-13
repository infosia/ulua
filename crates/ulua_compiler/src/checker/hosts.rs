//! Grants act as an outer scope. Only called grants enter the runtime module.
use super::*;
impl Checker<'_> {
    pub(super) fn is_host(&self, name: &str) -> bool {
        self.host_names.contains_key(name) && !self.scopes.iter().any(|s| s.contains_key(name))
    }
    pub(super) fn host_binding(
        &mut self,
        name: &IdentifierReference<'_>,
    ) -> Result<usize, Diagnostic> {
        let id = if let Some(&id) = self.host_ids.get(name.name.as_str()) {
            id
        } else {
            let index = *self
                .host_names
                .get(name.name.as_str())
                .ok_or_else(|| self.error(name.span, "internal", "Missing grant"))?;
            let grant = &self.host_grants[index];
            let scalar = |tag| match tag {
                0 => Some(Type::Void),
                1 => Some(Type::Number),
                2 => Some(Type::Boolean),
                3 => Some(Type::String),
                _ => None,
            };
            let parameters: Option<Vec<_>> = grant
                .parameters
                .iter()
                .map(|tag| scalar(*tag).filter(|t| *t != Type::Void))
                .collect();
            let parameters = parameters
                .ok_or_else(|| self.error(name.span, "C18", "Invalid grant parameters"))?;
            if parameters.len() > 16 {
                return Err(self.error(name.span, "C18", "Invalid grant parameter count"));
            }
            let result = scalar(grant.result)
                .ok_or_else(|| self.error(name.span, "C18", "Invalid grant result"))?;
            let count = parameters.len();
            let ty = self.types.intern(parameters, result, count);
            let id = self.bindings.len();
            self.bindings.push(ir::Binding {
                ty: Some(ty),
                owner: 0,
                start: name.span.start,
                end: 0,
                mutable: false,
                declaration: true,
            });
            self.facts.mutable.push(false);
            self.host_ids.insert(name.name.to_string(), id);
            self.hosts.push((id, name.name.to_string()));
            id
        };
        self.references.push((self.owner, id, name.span.start));
        Ok(id)
    }
    pub(super) fn host_value_form(&self, mut e: &Expression<'_>) -> Result<(), Diagnostic> {
        loop {
            e = match e {
                Expression::TSAsExpression(t) => &t.expression,
                Expression::TSSatisfiesExpression(t) => &t.expression,
                Expression::TSNonNullExpression(t) => &t.expression,
                Expression::ParenthesizedExpression(p) => &p.expression,
                Expression::NewExpression(n) => &n.callee,
                Expression::CallExpression(c)
                    if c.optional || !matches!(c.callee, Expression::Identifier(_)) =>
                {
                    &c.callee
                }
                _ => break,
            };
        }
        let reference = match e {
            Expression::Identifier(n) => Some(n.as_ref()),
            Expression::UnaryExpression(u) => {
                if let Expression::Identifier(n) = &u.argument {
                    Some(n.as_ref())
                } else {
                    None
                }
            }
            Expression::ChainExpression(c) => {
                if let ChainElement::CallExpression(call) = &c.expression
                    && let Expression::Identifier(n) = &call.callee
                {
                    Some(n.as_ref())
                } else {
                    None
                }
            }
            Expression::CallExpression(c) if c.optional => {
                if let Expression::Identifier(n) = &c.callee {
                    Some(n.as_ref())
                } else {
                    None
                }
            }
            _ => None,
        };
        if reference.is_some_and(|n| self.is_host(n.name.as_str())) {
            return Err(self.error(e.span(), "C18", "A host function is only called"));
        }
        Ok(())
    }
}

pub(super) fn known_global(name: &str) -> bool {
    name == "arguments"
        || crate::GLOBAL_NAMES.contains(&name)
        || crate::host_names::GLOBAL_TYPE_NAMES.contains(&name)
}
