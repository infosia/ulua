//! Shared function type identities bound analyzer work at repeated calls.
use crate::{
    Diagnostic,
    ir::{self, ExprKind as E, Statement as S, Type},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

pub(super) struct FunctionTypes {
    names: HashMap<Type, String>,
    identities: HashMap<Type, Type>,
    next_name: usize,
    definitions: HashMap<String, String>,
    pub(super) declarations: String,
}
impl FunctionTypes {
    pub(super) fn new(module: &ir::Module) -> Result<Self, Diagnostic> {
        let mut types = Self {
            identities: module.types.identities().collect(),
            names: [
                Type::Error,
                Type::Unknown,
                Type::Null,
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
            definitions: HashMap::new(),
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
            statements.extend(&function.initializers);
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
                S::Field(a, _, _, e) | S::ErrorField(a, _, _, e) => expressions.extend([a, e]),
                S::Element(a, i, _, e) => expressions.extend([a, i, e]),
                S::ForOf(_, a, b) => {
                    expressions.push(a);
                    statements.push(b);
                }
                S::Try(a, b, f) => {
                    statements.push(a);
                    if let Some((_, b)) = b {
                        statements.push(b);
                    }
                    statements.extend(f.iter().map(|s| s.as_ref()));
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
                S::Return(Some(e)) | S::Throw(e) => expressions.push(e),
                _ => {}
            }
        }
        while let Some(expression) = expressions.pop() {
            types.add(expression.ty, module, 0, expression.start)?;
            match &expression.kind {
                E::OptionalLength(a)
                | E::Error(_, a)
                | E::InstanceOf(a, _)
                | E::ErrorField(a, _)
                | E::OptionalErrorField(a, _)
                | E::Field(a, _)
                | E::OptionalField(a, _)
                | E::Json(_, a)
                | E::Await(a)
                | E::Unary(_, a)
                | E::Length(a) => expressions.push(a),
                E::OptionalPush(a, b)
                | E::Coalesce(a, b)
                | E::Binary(_, a, b)
                | E::Logical(_, a, b)
                | E::Index(a, b)
                | E::Push(a, b) => {
                    expressions.push(a);
                    expressions.push(b);
                }
                E::Conditional(t, a, b) => {
                    expressions.push(t);
                    expressions.push(a);
                    expressions.push(b);
                }
                E::OptionalMethod(_, _, c, args)
                | E::OptionalCall(c, _, args)
                | E::Method(_, _, c, args)
                | E::Call(c, args) => {
                    expressions.push(c);
                    expressions.extend(args);
                }
                E::Object(fields) => expressions.extend(fields.iter().map(|f| &f.1)),
                E::ClassCall(c, receiver, args, _) => {
                    expressions.extend([c.as_ref(), receiver.as_ref()]);
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
        _depth: u32,
        _offset: u32,
    ) -> Result<(), Diagnostic> {
        let mut pending = vec![(ty, false)];
        let mut active = HashSet::new();
        let mut back_edges = HashSet::new();
        while let Some((ty, emit)) = pending.pop() {
            // Count-specific views keep the same emitted identity as their structural type.
            let ty = module.types.identity(ty);
            if !emit {
                if self.names.contains_key(&ty) {
                    if active.contains(&ty) {
                        back_edges.insert(ty);
                    }
                    continue;
                }
                let name = format!("__ulua_fn{}", self.next_name);
                self.next_name += 1;
                self.names.insert(ty, name);
                active.insert(ty);
                pending.push((ty, true));
                if let Some(inner) = module
                    .types
                    .inner(ty)
                    .or_else(|| module.types.element(ty))
                    .or_else(|| module.types.promise_value(ty))
                {
                    pending.push((inner, false));
                }
                if let Some(fields) = module.types.fields(ty) {
                    pending.extend(fields.iter().map(|f| (f.1, false)));
                }
                if let Some(f) = module.types.function(ty) {
                    pending.push((f.result, false));
                    pending.extend(f.params.iter().map(|ty| (*ty, false)));
                }
                continue;
            }
            let name = self.luau(&ty);
            let definition = if module.types.table_parts(ty).is_some() {
                "__ulua_Table".to_owned()
            } else if let Some(value) = module.types.promise_value(ty) {
                format!(
                    "__ulua_Promise<{}>",
                    if value == Type::Void {
                        "nil".to_owned()
                    } else {
                        self.luau(&value)
                    }
                )
            } else if let Some(inner) = module.types.inner(ty) {
                let inner_name = self.luau(&inner);
                if module.types.function(inner).is_some() {
                    format!("({inner_name})?")
                } else {
                    format!("{inner_name}?")
                }
            } else if let Some(element) = module.types.element(ty) {
                format!("{{{}}}", self.luau(&element))
            } else if let Some(fields) = module.types.fields(ty) {
                format!(
                    "{{{}}}",
                    fields
                        .iter()
                        .map(|(key, ty)| format!("__ulua_k{key}: {}", self.luau(ty)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else if let Some(f) = module.types.function(ty) {
                let parameters = f
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| {
                        let name = self.luau(ty);
                        if i < f.required {
                            name
                        } else if module.types.function(*ty).is_some() {
                            format!("({name})?")
                        } else {
                            format!("{name}?")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({parameters}) -> {}", self.luau(&f.result))
            } else {
                continue;
            };
            // Luau classes have structural field types. The checker retains their nominal identities.
            // Share identical class shapes and signatures instead of separate aliases for each declaration.
            if let Some(existing) = self.definitions.get(&definition) {
                // A descendant can already name this pending ancestor. Preserve that name when it merges.
                if back_edges.contains(&ty) {
                    let _ = writeln!(self.declarations, "type {name} = {existing}");
                }
                self.names.insert(ty, existing.clone());
            } else {
                self.definitions.insert(definition.clone(), name.clone());
                let _ = writeln!(self.declarations, "type {name} = {definition}");
            }
            active.remove(&ty);
        }
        Ok(())
    }
    pub(super) fn luau(&self, ty: &Type) -> String {
        self.identities
            .get(ty)
            .and_then(|ty| self.names.get(ty))
            .cloned()
            .unwrap_or_default()
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
    fn type_alias_work_list_does_not_recurse_through_dependencies() {
        let source = "export function main():void{}function f(g:()=>()=>number):void{}";
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
        let mut module =
            crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING)
                .unwrap();
        module.depth = 2;
        let types = FunctionTypes::new(&module).unwrap();
        assert_eq!(types.declarations.lines().count(), 4);
        assert!(types.declarations.contains("() -> number"));
    }

    #[test]
    fn class_aliases_share_luau_shapes_and_keep_distinct_field_types() {
        let source = "class A{v:number=1;}class B{v:number=2;}class C{v:string='c';}\
            function f(a:A,b:B,c:C):void{}export function main():void{}";
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
        let module =
            crate::checker::check(&parsed.program, source.as_bytes(), crate::DEPTH_CEILING)
                .unwrap();
        let types = FunctionTypes::new(&module).unwrap();
        assert_eq!(types.declarations.matches("{__ulua_k0: number}").count(), 1);
        assert_eq!(types.declarations.matches("{__ulua_k0: string}").count(), 1);
        let class_types: Vec<_> = module
            .types
            .identities()
            .filter_map(|(ty, _)| module.types.is_class(ty).then_some(ty))
            .collect();
        assert_eq!(class_types.len(), 3);
        assert!(!module.types.equal(class_types[0], class_types[1]));
        assert_eq!(types.luau(&class_types[0]), types.luau(&class_types[1]));
        assert_ne!(types.luau(&class_types[0]), types.luau(&class_types[2]));
        assert_eq!(types.declarations.lines().count(), 6);
    }
}
