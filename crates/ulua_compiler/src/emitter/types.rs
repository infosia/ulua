//! Shared function type identities bound analyzer work at repeated calls.
use crate::{
    Diagnostic,
    ir::{self, ExprKind as E, Statement as S, Type},
};
use std::{collections::HashMap, fmt::Write};

pub(super) struct FunctionTypes {
    names: HashMap<Type, String>,
    next_name: usize,
    pub(super) declarations: String,
}
impl FunctionTypes {
    pub(super) fn new(module: &ir::Module) -> Result<Self, Diagnostic> {
        let mut types = Self {
            names: [
                Type::Number,
                Type::Boolean,
                Type::String,
                Type::Void,
                Type::StringParts,
            ]
            .into_iter()
            .filter_map(|ty| module.types.primitive(ty).map(|text| (ty, text.to_owned())))
            .collect(),
            next_name: 0,
            declarations: String::new(),
        };
        for binding in &module.bindings {
            if let Some(ty) = &binding.ty {
                types.add(*ty, module, 0, binding.start)?;
            }
        }
        let mut statements: Vec<_> = module.body.iter().collect();
        let mut expressions = vec![];
        for function in &module.functions {
            statements.extend(&function.body);
            expressions.extend(
                function
                    .parameters
                    .iter()
                    .filter_map(|(_, default)| default.as_ref()),
            );
        }
        while let Some(statement) = statements.pop() {
            match statement.form() {
                S::Declare(_, e) | S::Assign(_, _, e) | S::Eval(e) => expressions.push(e),
                S::Element(a, i, _, e) => expressions.extend([a, i, e]),
                S::ForOf(_, a, b) => {
                    expressions.push(a);
                    statements.push(b);
                }
                S::Block(body) => statements.extend(body),
                S::If(e, a, b) => {
                    expressions.push(e);
                    statements.push(a);
                    statements.extend(b.iter().map(|s| s.as_ref()));
                }
                S::While(e, body) => {
                    expressions.push(e);
                    statements.push(body);
                }
                S::For(init, test, update, body, _) => {
                    statements.extend(init);
                    expressions.extend(test);
                    statements.extend(update.iter().map(|s| s.as_ref()));
                    statements.push(body);
                }
                S::Return(Some(e)) => expressions.push(e),
                _ => {}
            }
        }
        while let Some(expression) = expressions.pop() {
            types.add(expression.ty, module, 0, expression.start)?;
            match &expression.kind {
                E::Unary(_, a) | E::Length(a) => expressions.push(a),
                E::Binary(_, a, b) | E::Logical(_, a, b) | E::Index(a, b) | E::Push(a, b) => {
                    expressions.push(a);
                    expressions.push(b);
                }
                E::Conditional(t, a, b) => {
                    expressions.push(t);
                    expressions.push(a);
                    expressions.push(b);
                }
                E::Call(c, args) => {
                    expressions.push(c);
                    expressions.extend(args);
                }
                E::Template(parts) | E::Array(parts) => expressions.extend(parts),
                _ => {}
            }
        }
        Ok(types)
    }
    fn add(
        &mut self,
        ty: Type,
        module: &ir::Module,
        depth: u32,
        offset: u32,
    ) -> Result<(), Diagnostic> {
        if self.names.contains_key(&ty) {
            return Ok(());
        }
        if let Some(element) = module.types.element(ty) {
            let depth = depth + 1;
            if depth >= module.depth {
                return Err(Diagnostic::at(
                    &module.source,
                    offset as usize,
                    "internal",
                    "Emitter type depth limit exceeded after checked input",
                ));
            }
            self.add(element, module, depth, offset)?;
            // A composite alias keeps repeated array references constant-size too.
            let name = format!("__ulua_fn{}", self.next_name);
            let _ = writeln!(
                self.declarations,
                "type {name} = {{{}}}",
                self.luau(&element)
            );
            self.names.insert(ty, name);
            self.next_name += 1;
            return Ok(());
        }
        let Some(ir::FunctionType {
            params,
            result,
            required,
        }) = module.types.function(ty)
        else {
            return Ok(());
        };
        if self.names.contains_key(&ty) {
            return Ok(());
        }
        let depth = depth + 1;
        if depth >= module.depth {
            return Err(Diagnostic::at(
                &module.source,
                offset as usize,
                "internal",
                "Emitter type depth limit exceeded after checked input",
            ));
        }
        for parameter in params {
            self.add(*parameter, module, depth, offset)?;
        }
        self.add(*result, module, depth, offset)?;
        let parameters = params
            .iter()
            .enumerate()
            .map(|(i, parameter)| {
                let name = self.luau(parameter);
                if i < *required {
                    name
                } else if module.types.function(*parameter).is_some() {
                    format!("({name})?")
                } else {
                    format!("{name}?")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let name = format!("__ulua_fn{}", self.next_name);
        let _ = writeln!(
            self.declarations,
            "type {name} = ({parameters}) -> {}",
            self.luau(result)
        );
        self.names.insert(ty, name);
        self.next_name += 1;
        Ok(())
    }
    pub(super) fn luau(&self, ty: &Type) -> String {
        self.names.get(ty).cloned().unwrap_or_default()
    }
}

// Crowded functions use nested pages. Every constructor keeps at most 128 fields.
pub(super) fn spill_constructor(fields: &[(String, String, &str)]) -> (String, String) {
    fn page(fields: &[(String, String, &str)]) -> (String, String) {
        let ty = fields
            .iter()
            .map(|(name, ty, _)| format!("{name}: {ty}"))
            .collect::<Vec<_>>()
            .join(", ");
        let value = fields
            .iter()
            .map(|(name, ty, value)| format!("{name} = ({value}) :: {ty}"))
            .collect::<Vec<_>>()
            .join(", ");
        (format!("{{ {ty} }}"), format!("{{ {value} }}"))
    }
    if fields.len() <= super::SPILL_FIELDS {
        return page(fields);
    }
    let mut types = vec![];
    let mut values = vec![];
    for (index, fields) in fields.chunks(super::SPILL_FIELDS).enumerate() {
        let (ty, value) = page(fields);
        types.push(format!("__ulua_page{index}: {ty}"));
        values.push(format!("__ulua_page{index} = {value}"));
    }
    (
        format!("{{ {} }}", types.join(", ")),
        format!("{{ {} }}", values.join(", ")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchecked_type_depth_reports_internal_at_the_binding() {
        let source = "export function main():void{}function f(g:()=>()=>number):void{}";
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
        let mut module =
            crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING)
                .unwrap();
        module.depth = 2;
        let error = FunctionTypes::new(&module).err().unwrap();
        assert_eq!(error.code, "internal");
        assert_eq!(error.offset, source.find("function f").unwrap() as u64);
    }
}
