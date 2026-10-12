//! D18 checks forms that preserve a narrowed boolean or nullable reference type.
use super::*;

impl Checker<'_> {
    pub(super) fn inferred_reference(
        &self,
        e: &Expression<'_>,
        ty: Type,
    ) -> Result<(), Diagnostic> {
        let nullable = self.nullable_reference(e);
        if reference_form(e, ty) || nullable {
            return Err(self.error(
                e.span(),
                "C11",
                if nullable {"An inferred type cannot come from a narrowable reference; add a type annotation"} else {"An inferred type cannot come from a boolean reference; add a type annotation"},
            ));
        }
        Ok(())
    }
    fn nullable_reference(&self, e: &Expression<'_>) -> bool {
        match e {
            Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_) => {
                reference(e)
                    && self
                        .reference_type(e)
                        .is_some_and(|ty| self.types.inner(ty).is_some())
            }
            Expression::ParenthesizedExpression(p) => self.nullable_reference(&p.expression),
            // Every accepted optional field read includes null in its result type.
            Expression::ChainExpression(c) => chain_reference(&c.expression),
            Expression::UnaryExpression(u) if u.operator.as_str() == "!" => {
                self.nullable_reference(&u.argument)
            }
            Expression::ConditionalExpression(c) => {
                self.nullable_reference(&c.consequent) || self.nullable_reference(&c.alternate)
            }
            Expression::LogicalExpression(l) => {
                self.nullable_reference(&l.left)
                    || self.nullable_reference(&l.right)
                    || (l.operator.as_str() == "??"
                        && (reference_form(&l.left, Type::Boolean)
                            || reference_form(&l.right, Type::Boolean)))
            }
            _ => false,
        }
    }
    fn reference_type(&self, e: &Expression<'_>) -> Option<Type> {
        match e {
            Expression::Identifier(n) => self
                .scopes
                .iter()
                .rev()
                .find_map(|s| s.get(n.name.as_str()))
                .and_then(|id| self.bindings[*id].ty),
            Expression::ThisExpression(_) => self.this_binding.and_then(|id| self.bindings[id].ty),
            Expression::ParenthesizedExpression(p) => self.reference_type(&p.expression),
            Expression::ComputedMemberExpression(m) => {
                let ty = self.reference_type(&m.object)?;
                self.types.element(self.types.inner(ty).unwrap_or(ty))
            }
            Expression::StaticMemberExpression(m) => {
                let ty = self.reference_type(&m.object)?;
                let ty = self.types.inner(ty).unwrap_or(ty);
                self.types
                    .field(ty, *self.fields.get(m.property.name.as_str())?)
            }
            _ => None,
        }
    }
}

// The depth check bounds this walk before the checker starts.
fn reference_form(e: &Expression<'_>, ty: Type) -> bool {
    if ty != Type::Boolean {
        return false;
    }
    // Checked logical operands and conditional arms have the result's boolean type.
    match e {
        Expression::Identifier(_) | Expression::ThisExpression(_) => true,
        Expression::ComputedMemberExpression(m) => reference(&m.object),
        Expression::StaticMemberExpression(m) => reference(&m.object),
        Expression::ChainExpression(c) => chain_reference(&c.expression),
        Expression::ParenthesizedExpression(p) => reference_form(&p.expression, ty),
        Expression::UnaryExpression(u) if u.operator.as_str() == "!" => {
            reference_form(&u.argument, ty)
        }
        Expression::ConditionalExpression(c) => {
            reference_form(&c.consequent, ty) || reference_form(&c.alternate, ty)
        }
        Expression::LogicalExpression(l) if matches!(l.operator.as_str(), "&&" | "||" | "??") => {
            reference_form(&l.left, ty) || reference_form(&l.right, ty)
        }
        _ => false,
    }
}

fn reference(e: &Expression<'_>) -> bool {
    match e {
        Expression::Identifier(_) | Expression::ThisExpression(_) => true,
        Expression::ComputedMemberExpression(m) => reference(&m.object),
        Expression::StaticMemberExpression(m) => reference(&m.object),
        Expression::ParenthesizedExpression(p) => reference(&p.expression),
        Expression::ChainExpression(c) => chain_reference(&c.expression),
        _ => false,
    }
}
fn chain_reference(c: &ChainElement<'_>) -> bool {
    match c {
        ChainElement::ComputedMemberExpression(m) => reference(&m.object),
        ChainElement::StaticMemberExpression(m) => reference(&m.object),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[test]
    fn boolean_member_paths_and_composites_define_reference_forms() {
        let mut types = ir::Types::new();
        let array = types.array(Type::Boolean);
        let function = types.intern(vec![], Type::Boolean, 0);
        for reference in [
            "b",
            "bs[k]",
            "o.f",
            "o.fields[k].flag",
            "os[k].child.flags[j]",
            "(o).f",
            "(o.fields)[k].flag",
            "(os[k]).child.flags[j]",
            "o?.f",
            "o?.[k]",
            "(o)?.f",
            "(os[k])?.flag",
            "(o?.child).flag",
        ] {
            for form in [
                reference.to_string(),
                format!("({reference})"),
                format!("!{reference}"),
                format!("c?{reference}:false"),
                format!("c?false:{reference}"),
                format!("{reference}&&true"),
                format!("true&&{reference}"),
                format!("{reference}||false"),
                format!("false||{reference}"),
                format!("c?(!{reference}&&true):(false||{reference})"),
            ] {
                check(&form, Type::Boolean, true);
                check(&form, Type::Number, false);
                check(&form, Type::String, false);
                check(&form, array, false);
                check(&form, function, false);
            }
        }
        check("xs.length", Type::Number, false);
        for form in [
            "true",
            "truth()",
            "make()[0]",
            "make().flag",
            "make()?.flag",
            "(make()).flag",
            "o?.f()",
            "c?true:false",
            "b===true",
            "true&&false",
            "true||false",
        ] {
            check(form, Type::Boolean, false);
        }
    }

    fn check(form: &str, ty: Type, expected: bool) {
        let allocator = Allocator::default();
        let source = format!("let result={form};");
        let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
        assert!(parsed.diagnostics.is_empty(), "{source}");
        let Statement::VariableDeclaration(declaration) = &parsed.program.body[0] else {
            panic!("Expected a variable declaration");
        };
        let expression = declaration.declarations[0].init.as_ref().unwrap();
        assert_eq!(reference_form(expression, ty), expected, "{source}");
    }
}
