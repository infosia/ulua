//! Closed method signatures carry callback arity into the helper call.
use super::*;
use crate::ir::{Expr, ExprKind};

impl Checker<'_> {
    pub(super) fn array_method(
        &mut self,
        call: &CallExpression<'_>,
        name: &str,
        receiver: Expr,
        element: Type,
        depth: u32,
    ) -> Result<Expr, Diagnostic> {
        let (name, min, max) = match name {
            "indexOf" => ("indexOf", 1, 2),
            "lastIndexOf" => ("lastIndexOf", 1, 2),
            "includes" => ("includes", 1, 2),
            "join" => ("join", 0, 1),
            "slice" => ("slice", 0, 2),
            "concat" => ("concat", 1, 1),
            "fill" => ("fill", 1, 3),
            "reverse" => ("reverse", 0, 0),
            "splice" => ("splice", 1, 2),
            "unshift" => ("unshift", 1, 1),
            "copyWithin" => ("copyWithin", 2, 3),
            "at" => ("at", 1, 1),
            "pop" => ("pop", 0, 0),
            "shift" => ("shift", 0, 0),
            "forEach" => ("forEach", 1, 1),
            "map" => ("map", 1, 1),
            "filter" => ("filter", 1, 1),
            "some" => ("some", 1, 1),
            "every" => ("every", 1, 1),
            "find" => ("find", 1, 1),
            "findLast" => ("findLast", 1, 1),
            "findIndex" => ("findIndex", 1, 1),
            "findLastIndex" => ("findLastIndex", 1, 1),
            "flatMap" => ("flatMap", 1, 1),
            "reduce" => ("reduce", 2, 2),
            "reduceRight" => ("reduceRight", 2, 2),
            "sort" => ("sort", 1, 1),
            _ => return Err(self.error(call.span, "C6", "Array method is rejected")),
        };
        let method = ir::Method::parse(name)
            .ok_or_else(|| self.error(call.span, "internal", "Missing array method kind"))?;
        if !(min..=max).contains(&call.arguments.len()) {
            return Err(self.error(call.span, "C6", "Array method argument count is rejected"));
        }
        if matches!(name, "indexOf" | "lastIndexOf" | "includes")
            && self
                .types
                .promise_value(self.types.inner(element).unwrap_or(element))
                .is_some()
        {
            return Err(self.error(call.span, "C19", "Search of promise elements is rejected"));
        }
        if matches!(name, "indexOf" | "lastIndexOf" | "includes")
            && self.types.function(element).is_some()
        {
            return Err(self.error(call.span, "C6", "Search of function elements is rejected"));
        }
        if name == "join" && !matches!(element, Type::Number | Type::String | Type::Boolean) {
            return Err(self.error(call.span, "C6", "Join requires primitive elements"));
        }
        let callback = matches!(
            name,
            "forEach"
                | "map"
                | "filter"
                | "some"
                | "every"
                | "find"
                | "findLast"
                | "findIndex"
                | "findLastIndex"
                | "flatMap"
                | "reduce"
                | "reduceRight"
                | "sort"
        );
        let mut args: Vec<Expr> = vec![];
        let mut arity = 0;
        for (index, arg) in call.arguments.iter().enumerate() {
            let arg = arg.as_expression().ok_or_else(|| {
                self.error(call.span, "C6", "Variadic array methods are rejected")
            })?;
            if name != "map" {
                self.async_array_argument(arg)?;
            }
            let expected = if callback && index == 0 {
                None
            } else if callback {
                args.first()
                    .and_then(|a| self.types.function(a.ty))
                    .and_then(|s| s.params.first().copied())
            } else {
                Some(match (name, index) {
                    ("concat", _) => receiver.ty,
                    ("join", _) => Type::String,
                    ("indexOf" | "lastIndexOf" | "includes" | "fill" | "unshift", 0) => element,
                    _ => Type::Number,
                })
            };
            let value = self.expr_as(arg, depth, false, expected)?;
            if name != "map" {
                self.promise_array_argument(value.ty, arg.span())?;
            }
            if let Some(ty) = expected {
                self.flow(&value.ty, Some(&ty), arg.span())?;
            }
            if callback && index == 0 {
                let signature = self.types.signature(value.ty).ok_or_else(|| {
                    self.error(arg.span(), "C6", "Array callback must be a function")
                })?;
                arity = signature.params.len();
                let reduce = matches!(name, "reduce" | "reduceRight");
                if !method.arities().contains(&arity) {
                    return Err(self.error(
                        arg.span(),
                        "C6",
                        "Callback parameter count is rejected",
                    ));
                }
                let expected = if reduce { signature.params[0] } else { element };
                self.flow(&signature.params[0], Some(&expected), arg.span())?;
                if arity >= 2 {
                    let second = if reduce || name == "sort" {
                        element
                    } else {
                        Type::Number
                    };
                    self.flow(&signature.params[1], Some(&second), arg.span())?;
                }
                if reduce && arity == 3 {
                    self.flow(&signature.params[2], Some(&Type::Number), arg.span())?;
                }
                if matches!(
                    name,
                    "filter"
                        | "some"
                        | "every"
                        | "find"
                        | "findLast"
                        | "findIndex"
                        | "findLastIndex"
                ) {
                    self.flow(&signature.result, Some(&Type::Boolean), arg.span())?;
                } else if name == "sort" {
                    self.flow(&signature.result, Some(&Type::Number), arg.span())?;
                } else if reduce {
                    self.flow(&signature.result, Some(&expected), arg.span())?;
                } else if name == "flatMap" && self.types.element(signature.result).is_none() {
                    return Err(self.error(
                        arg.span(),
                        "C6",
                        "FlatMap callback must return an array",
                    ));
                } else if name == "map" && signature.result == Type::Void {
                    return Err(self.error(arg.span(), "C2", "Map elements must exclude void"));
                }
                if matches!(name, "map" | "flatMap") {
                    let element = if name == "map" {
                        signature.result
                    } else {
                        self.types.element(signature.result).ok_or_else(|| {
                            self.error(arg.span(), "C6", "FlatMap callback must return an array")
                        })?
                    };
                    self.reject_absent(element, arg.span())?;
                    if self.types.contains_null(element) {
                        return Err(self.error(
                            arg.span(),
                            "C6",
                            "Array elements must exclude null",
                        ));
                    }
                }
            }
            args.push(value);
        }
        let ty = match name {
            "indexOf" | "lastIndexOf" | "unshift" | "findIndex" | "findLastIndex" => Type::Number,
            "includes" | "some" | "every" => Type::Boolean,
            "join" => Type::String,
            "at" | "pop" | "shift" => element,
            "find" | "findLast" => self.types.absent(element),
            "forEach" => Type::Void,
            "map" | "flatMap" | "reduce" | "reduceRight" => {
                let result = self
                    .types
                    .signature(args[0].ty)
                    .ok_or_else(|| {
                        self.error(call.span, "internal", "Missing checked callback signature")
                    })?
                    .result;
                if name == "map" {
                    self.types.array(result)
                } else {
                    result
                }
            }
            _ => receiver.ty,
        };
        self.methods.insert((method, arity));
        Ok(Expr {
            flow: None,
            bare_true: false,
            start: call.span.start,
            ty,
            kind: ExprKind::Method(method, arity, Box::new(receiver), args),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_array_elements_reject_constructed_null_types() {
        for method in ["map", "flatMap"] {
            for shape in ["null", "nullable", "number"] {
                let source = format!("xs.{method}(f);");
                let allocator = oxc_allocator::Allocator::default();
                let parsed =
                    oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::ts())
                        .parse();
                assert!(parsed.diagnostics.is_empty());
                let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
                    panic!("expression statement");
                };
                let Expression::CallExpression(call) = &statement.expression else {
                    panic!("method call");
                };
                let mut c = Checker {
                    host_names: HashMap::new(),
                    host_grants: vec![],
                    host_ids: HashMap::new(),
                    hosts: vec![],
                    catch_use: false,
                    uses_exceptions: false,
                    source: source.as_bytes(),
                    aliases: Default::default(),
                    classes: Default::default(),
                    class_bindings: Default::default(),
                    this_binding: None,
                    field_initializer: false,
                    class_context: None,
                    in_constructor: false,
                    fields: Default::default(),
                    facts: Default::default(),
                    assignments: vec![],
                    suspensions: HashMap::new(),
                    source_owner: u32::MAX,
                    captured_writes: Default::default(),
                    binding_starts: Default::default(),
                    json: vec![],
                    json_literals: vec![],
                    math: Default::default(),
                    methods: Default::default(),
                    bindings: vec![],
                    scopes: vec![HashMap::new()],
                    owner: 0,
                    result: Type::Void,
                    async_body: false,
                    uses_async: false,
                    promise_annotations: vec![],
                    aliases_ready: false,
                    types: ir::Types::new(),
                    reachable: true,
                    loops: 0,
                    loop_facts: vec![],
                    limit: crate::DEPTH_CEILING,
                    references: vec![],
                    functions: vec![],
                    default_scopes: vec![],
                };
                // Build the violating signature directly, independent of array admission.
                let element = match shape {
                    "null" => Type::Null,
                    "nullable" => c.types.nullable(Type::Number),
                    "number" => Type::Number,
                    _ => panic!("type shape"),
                };
                let result = if method == "flatMap" {
                    c.types.array(element)
                } else {
                    element
                };
                let callback = c.types.intern(vec![Type::Number], result, 1);
                c.bind(
                    "f",
                    ir::Binding {
                        ty: Some(callback),
                        owner: 0,
                        start: 0,
                        end: 0,
                        mutable: false,
                        declaration: true,
                    },
                    call.span,
                )
                .unwrap();
                let receiver = Expr {
                    flow: None,
                    bare_true: false,
                    start: 0,
                    ty: c.types.array(Type::Number),
                    kind: ExprKind::Array(vec![]),
                };
                let checked = c.array_method(call, method, receiver, Type::Number, 0);
                if shape == "number" {
                    assert!(checked.is_ok(), "{method}");
                } else {
                    let error = checked.err().expect("nullable array result must fail");
                    assert_eq!(error.code, "C6", "{method}: {shape}");
                    assert_eq!(error.message, "Array elements must exclude null");
                    assert!(c.methods.is_empty());
                }
            }
        }
    }
}
