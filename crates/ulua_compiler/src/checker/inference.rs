//! D18 checks forms that can preserve a narrowed boolean reference type.
use super::*;

impl Checker<'_> {
    pub(super) fn inferred_reference(
        &self,
        e: &Expression<'_>,
        ty: Type,
    ) -> Result<(), Diagnostic> {
        if reference_form(e, ty) {
            return Err(self.error(
                e.span(),
                "C11",
                "An inferred type cannot come from a boolean reference; add a type annotation",
            ));
        }
        Ok(())
    }
}

// The depth check bounds this walk before the checker starts.
fn reference_form(e: &Expression<'_>, ty: Type) -> bool {
    if ty != Type::Boolean {
        return false;
    }
    // Checked logical operands and conditional arms have the result's boolean type.
    match e {
        Expression::Identifier(_) => true,
        Expression::ComputedMemberExpression(m) => reference(&m.object),
        Expression::StaticMemberExpression(m) => reference(&m.object),
        Expression::ParenthesizedExpression(p) => reference_form(&p.expression, ty),
        Expression::UnaryExpression(u) if u.operator.as_str() == "!" => {
            reference_form(&u.argument, ty)
        }
        Expression::ConditionalExpression(c) => {
            reference_form(&c.consequent, ty) || reference_form(&c.alternate, ty)
        }
        Expression::LogicalExpression(l) if matches!(l.operator.as_str(), "&&" | "||") => {
            reference_form(&l.left, ty) || reference_form(&l.right, ty)
        }
        _ => false,
    }
}

fn reference(e: &Expression<'_>) -> bool {
    match e {
        Expression::Identifier(_) => true,
        Expression::ComputedMemberExpression(m) => reference(&m.object),
        Expression::StaticMemberExpression(m) => reference(&m.object),
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
