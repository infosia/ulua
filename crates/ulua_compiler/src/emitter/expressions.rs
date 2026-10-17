//! Reusable typed temporaries fix the time of each value read.
use super::*;
use crate::ir::{Expr, ExprKind as E};

// Fixed pure-expression work also keeps short-circuit analyzer graphs small.
// Three nodes permit one binary operation. Larger trees use interruptible slot statements.
pub(super) const PURE_EXPRESSION_NODES: usize = 3;
impl Emitter<'_> {
    pub(super) fn expression(&mut self, expr: &Expr, depth: u32) -> Result<usize, Diagnostic> {
        self.offset = expr.start as usize;
        let depth = self.guard(depth)?;
        let dest = self.slot(if expr.ty == Type::Void {
            &Type::Boolean
        } else {
            &expr.ty
        })?;
        let target = self.temp(dest);
        if self.inline_enabled
            && let Some(value) = self.last_value(expr, depth)?
        {
            if expr.ty == Type::Void {
                self.line_weighted(value.text, value.cost);
            } else {
                let ty = self.function_types.luau(&self.slots[dest].ty);
                self.write_weighted(&target, value.text, &ty, value.cost);
            }
            return Ok(dest);
        }
        match &expr.kind {
            E::Json(id, value) => {
                let slot = self.expression(value, depth)?;
                self.capture("__ulua_json");
                self.write_slot(dest, format!("__ulua_json.c{id}({})", self.temp(slot)));
                self.release(slot);
            }
            E::Await(value) => {
                self.capture("__ulua_await");
                let slot = self.expression(value, depth)?;
                if expr.ty == Type::Void {
                    self.line(format!("__ulua_await({})", self.temp(slot)));
                } else {
                    self.write_slot(dest, format!("__ulua_await({})", self.temp(slot)));
                }
                self.release(slot);
            }
            E::Error(kind, message) => {
                self.capture("__ulua_error");
                let value = self.expression(message, depth)?;
                self.write_slot(
                    dest,
                    format!("__ulua_error({}, {})", quote(kind), self.temp(value)),
                );
                self.release(value);
            }
            E::InstanceOf(value, kind) => {
                self.capture("__ulua_instance");
                let value = self.expression(value, depth)?;
                self.write_slot(
                    dest,
                    format!("__ulua_instance({}, {})", self.temp(value), quote(kind)),
                );
                self.release(value);
            }
            E::ErrorField(value, key) => {
                let value = self.expression(value, depth)?;
                self.write_slot(
                    dest,
                    format!("({} :: __ulua_Error).{key}", self.temp(value)),
                );
                self.release(value);
            }
            E::OptionalErrorField(value, key) => {
                let value = self.expression(value, depth)?;
                self.write_slot(
                    dest,
                    format!(
                        "if {0} ~= nil then ({0} :: __ulua_Error).{key} else nil",
                        self.temp(value)
                    ),
                );
                self.release(value);
            }
            E::Null => self.write_slot(dest, "nil"),
            E::Object(fields) => {
                // The checked constructor assigns every field; its empty table has no inferred field graph.
                self.write_slot(dest, "{} :: any");
                for (key, entry) in fields {
                    let slot = self.expression(entry, depth)?;
                    self.write(
                        &format!("{}.__ulua_k{key}", self.temp(dest)),
                        self.temp(slot),
                        &self.function_types.luau(&entry.ty),
                    );
                    self.release(slot);
                }
                // Restore the complete declared type after the individually typed stores.
                self.write_slot(dest, format!("{} :: any", self.temp(dest)));
            }
            E::Field(receiver, key) => {
                let (slot, deferred) = self.access_receiver(receiver, depth)?;
                self.access_receiver_check(slot, deferred);
                self.write_slot(dest, format!("{}.__ulua_k{key}", self.temp(slot)));
                self.release(slot);
            }
            E::OptionalField(receiver, key) => {
                let slot = self.expression(receiver, depth)?;
                self.write_slot(
                    dest,
                    format!(
                        "if {0} ~= nil then {0}.__ulua_k{key} else nil",
                        self.temp(slot)
                    ),
                );
                self.release(slot);
            }
            E::OptionalLength(receiver) => {
                let slot = self.expression(receiver, depth)?;
                self.write_slot(
                    dest,
                    if self
                        .module
                        .types
                        .inner(receiver.ty)
                        .is_some_and(|t| self.module.types.table_parts(t).is_some())
                    {
                        format!("if {0} ~= nil then {0}.live else nil", self.temp(slot))
                    } else {
                        format!("if {0} ~= nil then #{0} else nil", self.temp(slot))
                    },
                );
                self.release(slot);
            }
            E::OptionalMethod(method, arity, receiver, args) => {
                let slot = self.expression(receiver, depth)?;
                self.line(format!("if {} ~= nil then", self.temp(slot)));
                self.indent += 1;
                let inner = self
                    .module
                    .types
                    .inner(receiver.ty)
                    .ok_or_else(|| self.limit("internal", "Missing optional receiver type"))?;
                let narrowed = self.slot(&inner)?;
                self.write_slot(narrowed, self.temp(slot));
                let mut slots = vec![narrowed];
                for arg in args {
                    slots.push(self.expression(arg, depth)?);
                }
                let helper = methods::name(*method, *arity);
                self.capture(&helper);
                self.line(format!(
                    "{helper}({})",
                    slots
                        .iter()
                        .map(|s| self.temp(*s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                for slot in slots {
                    self.release(slot);
                }
                self.release(slot);
                self.indent -= 1;
                self.line("end");
            }
            E::OptionalPush(receiver, value) => {
                let slot = self.expression(receiver, depth)?;
                self.line(format!("if {} ~= nil then", self.temp(slot)));
                self.indent += 1;
                let value = self.expression(value, depth)?;
                self.write(
                    &format!("{0}[#{0}+1]", self.temp(slot)),
                    self.temp(value),
                    &self.function_types.luau(&self.slots[value].ty),
                );
                self.release(value);
                self.release(slot);
                self.indent -= 1;
                self.line("end");
            }
            E::OptionalCall(receiver, key, args) => {
                let slot = self.expression(receiver, depth)?;
                self.line(format!("if {} ~= nil then", self.temp(slot)));
                self.indent += 1;
                let ty = self
                    .module
                    .types
                    .inner(receiver.ty)
                    .and_then(|ty| self.module.types.field(ty, *key))
                    .ok_or_else(|| self.limit("internal", "Missing optional function type"))?;
                let callee = self.slot(&ty)?;
                self.write_slot(callee, format!("{}.__ulua_k{key}", self.temp(slot)));
                let mut slots = vec![];
                for arg in args {
                    slots.push(self.expression(arg, depth)?);
                }
                self.line(format!(
                    "{}({})",
                    self.temp(callee),
                    slots
                        .iter()
                        .map(|s| self.temp(*s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                for slot in slots {
                    self.release(slot);
                }
                self.release(callee);
                self.release(slot);
                self.indent -= 1;
                self.line("end");
            }
            E::Coalesce(a, b) => {
                let slot = self.expression(a, depth)?;
                self.line(format!("if {} ~= nil then", self.temp(slot)));
                self.indent += 1;
                self.write_slot(dest, self.temp(slot));
                self.indent -= 1;
                self.line("else");
                self.indent += 1;
                let fallback = self.expression(b, depth)?;
                self.write_slot(dest, self.temp(fallback));
                self.release(fallback);
                self.indent -= 1;
                self.line("end");
                self.release(slot);
            }
            E::Number(n) => {
                if n.is_infinite() {
                    self.capture("__ulua_huge");
                }
                self.write_slot(dest, number(*n));
            }
            E::Boolean(b) => self.write_slot(dest, b.to_string()),
            E::String(s) => self.write_slot(dest, quote(s)),
            E::Read(id) => {
                let value = self.read_binding(*id)?;
                self.write_slot(dest, value);
            }
            E::Closure(id) => {
                let value = self.closure(*id, depth)?;
                let ty = self.function_types.luau(&self.slots[dest].ty);
                self.write_weighted(&target, value.text, &ty, value.cost);
            }
            E::Builtin(name) => {
                self.capture(name);
                self.write_slot(dest, name);
            }
            E::Unary(op, a) => {
                let a = self.expression(a, depth)?;
                self.write_slot(dest, format!("{op} ({})", self.temp(a)));
                self.release(a);
            }
            E::Binary(op, a, b) => {
                let types = [a.ty, b.ty];
                let a = self.expression(a, depth)?;
                let b = self.expression(b, depth)?;
                let value = self.expression_binary(op, types, &self.temp(a), &self.temp(b));
                self.write_slot(dest, value);
                self.release(a);
                self.release(b);
            }
            E::Logical(and, a, b) => {
                // Pure boolean operands use Luau's short-circuit expression without temporary joins.
                if let Some(value) = self.pure(expr, depth)? {
                    self.write_slot(dest, value);
                    return Ok(dest);
                }
                let a = self.expression(a, depth)?;
                // Keep a bounded pure expression on the right. This avoids a refinement join for each split operation.
                if let Some(b) = self.pure(b, depth)? {
                    self.write_slot(
                        dest,
                        format!("({} {} {b})", self.temp(a), if *and { "and" } else { "or" }),
                    );
                    self.release(a);
                    return Ok(dest);
                }
                // A fresh local breaks refinement aliases before the short-circuit join.
                let fresh = self.generated()?;
                self.line("do");
                self.line(format!("local {fresh}: boolean = {}", self.temp(a)));
                self.write_slot(dest, &fresh);
                self.line("end");
                self.generated_live -= 1;
                self.release(a);
                self.line(format!(
                    "if {}{target} then",
                    if *and { "" } else { "not " }
                ));
                self.indent += 1;
                let b = self.expression(b, depth)?;
                // The branch already selects the right operand of this boolean operation.
                self.write_slot(dest, self.temp(b));
                self.release(b);
                self.indent -= 1;
                self.line("end");
            }
            E::Conditional(test, a, b) => {
                if let Some((id, tests)) = same_conditional_read(expr)
                    && tests.len() <= 1
                {
                    // The disjunction evaluates exactly the tests on the selected false-arm chain.
                    let tests = tests
                        .into_iter()
                        .map(|test| self.pure(test, depth))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| self.limit("internal", "Missing pure conditional test"))?;
                    let test = self.slot(&Type::Boolean)?;
                    self.write_slot(test, tests.join(" or "));
                    self.release(test);
                    let value = self.read_binding(id)?;
                    self.write_slot(dest, value);
                    return Ok(dest);
                }
                let t = self.expression(test, depth)?;
                if let (Some(a), Some(b)) = (self.pure(a, depth)?, self.pure(b, depth)?) {
                    self.write_slot(dest, format!("if {} then {a} else {b}", self.temp(t)));
                } else {
                    // Branch statements preserve short-circuit evaluation of calls.
                    self.line(format!("if {} then", self.temp(t)));
                    self.indent += 1;
                    let a = self.expression(a, depth)?;
                    self.write_slot(dest, self.temp(a));
                    self.release(a);
                    self.indent -= 1;
                    self.line("else");
                    self.indent += 1;
                    let b = self.expression(b, depth)?;
                    self.write_slot(dest, self.temp(b));
                    self.release(b);
                    self.indent -= 1;
                    self.line("end");
                }
                self.release(t);
            }
            E::Call(callee, arguments) => {
                if let E::Builtin(name) = callee.kind
                    && (name.starts_with("__ulua_on_") || name == "__ulua_all")
                {
                    self.promise_expression(expr, name, arguments, dest, depth)?;
                    return Ok(dest);
                }
                let c = self.expression(callee, depth)?;
                let mut args = vec![];
                for arg in arguments {
                    args.push(self.expression(arg, depth)?);
                }
                let call = format!(
                    "{}({})",
                    self.temp(c),
                    args.iter()
                        .map(|i| self.temp(*i))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if expr.ty == Type::Void {
                    self.line(call);
                } else {
                    self.write_slot(dest, call);
                }
                self.release(c);
                for arg in args {
                    self.release(arg);
                }
            }
            E::ClassCall(callee, receiver, arguments, optional) => {
                let c = self.expression(callee, depth)?;
                let receiver_slot = self.expression(receiver, depth)?;
                if *optional {
                    self.line(format!("if {} ~= nil then", self.temp(receiver_slot)));
                    self.indent += 1;
                }
                let receiver_slot = if *optional {
                    let inner = self
                        .module
                        .types
                        .inner(receiver.ty)
                        .ok_or_else(|| self.limit("internal", "Missing optional class type"))?;
                    let narrowed = self.slot(&inner)?;
                    self.write_slot(narrowed, self.temp(receiver_slot));
                    self.release(receiver_slot);
                    narrowed
                } else {
                    receiver_slot
                };
                let mut args = vec![receiver_slot];
                for arg in arguments {
                    args.push(self.expression(arg, depth)?);
                }
                let call = format!(
                    "{}({})",
                    self.temp(c),
                    args.iter()
                        .map(|slot| self.temp(*slot))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if expr.ty == Type::Void {
                    self.line(call);
                } else {
                    self.write_slot(dest, call);
                }
                for slot in args {
                    self.release(slot);
                }
                self.release(c);
                if *optional {
                    self.indent -= 1;
                    self.line("end");
                }
            }
            E::Method(name, arity, receiver, arguments) => {
                let a = self.expression(receiver, depth)?;
                let mut slots = vec![a];
                for arg in arguments {
                    slots.push(self.expression(arg, depth)?);
                }
                let helper = methods::name(*name, *arity);
                self.capture(&helper);
                let call = format!(
                    "{helper}({})",
                    slots
                        .iter()
                        .map(|s| self.temp(*s))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if expr.ty == Type::Void {
                    self.line(call);
                } else {
                    self.write_slot(dest, call);
                }
                for slot in slots {
                    self.release(slot);
                }
            }
            E::Array(entries) => {
                self.array_literal(dest, entries, depth)?;
            }
            E::Index(a, i, form) => {
                let (a, deferred) = self.access_receiver(a, depth)?;
                let i = self.expression(i, depth)?;
                self.access_receiver_check(a, deferred);
                self.element_read(
                    super::arrays::ElementTarget::Slot(dest),
                    expr.ty,
                    a,
                    &self.temp(i),
                    *form,
                )?;
                self.release(a);
                self.release(i);
            }
            E::Length(a) if self.module.types.table_parts(a.ty).is_some() => {
                let a = self.expression(a, depth)?;
                self.write_slot(dest, format!("{}.live", self.temp(a)));
                self.release(a);
            }
            E::Length(a) => {
                let a = self.expression(a, depth)?;
                self.write_slot(dest, format!("#{}", self.temp(a)));
                self.release(a);
            }
            E::Push(a, e) => {
                let a = self.expression(a, depth)?;
                let e = self.expression(e, depth)?;
                self.write(
                    &format!("{0}[#{0} + 1]", self.temp(a)),
                    self.temp(e),
                    &self.function_types.luau(&self.slots[e].ty),
                );
                self.release(a);
                self.release(e);
            }
            E::Template(parts) => {
                if parts.len() > 16 {
                    self.capture("__ulua_concat");
                }
                let table = if parts.len() > 16 {
                    let table = self.slot(&Type::StringParts)?;
                    self.write_slot(table, "{}");
                    Some(table)
                } else {
                    None
                };
                let mut values = vec![];
                for (index, part) in parts.iter().enumerate() {
                    let value = self.expression(part, depth)?;
                    match part.ty {
                        Type::Number => self.capture("__ulua_format"),
                        Type::Boolean => self.capture("__ulua_tostring"),
                        _ => {}
                    }
                    let converted = match part.ty {
                        Type::Number => format!("__ulua_format({})", self.temp(value)),
                        Type::Boolean => format!("__ulua_tostring({})", self.temp(value)),
                        _ => self.temp(value),
                    };
                    if let Some(table) = table {
                        // Freeze each converted value before the next part can change a binding.
                        self.write(
                            &format!("{}[{}]", self.temp(table), index + 1),
                            converted,
                            "string",
                        );
                        self.release(value);
                    } else {
                        // Conversion also occurs before the next source expression.
                        let frozen = self.slot(&Type::String)?;
                        let ty = self.function_types.luau(&self.slots[frozen].ty);
                        let cost = if part.ty == Type::Boolean { 4 } else { 1 };
                        self.write_weighted(&self.temp(frozen), converted, &ty, cost);
                        self.release(value);
                        values.push(frozen);
                    }
                }
                if let Some(table) = table {
                    self.write_slot(dest, format!("__ulua_concat({})", self.temp(table)));
                    self.release(table);
                } else {
                    self.write_slot(
                        dest,
                        values
                            .iter()
                            .map(|v| self.temp(*v))
                            .collect::<Vec<_>>()
                            .join(" .. "),
                    );
                    for value in values {
                        self.release(value);
                    }
                }
            }
        }
        self.offset = expr.start as usize;
        Ok(dest)
    }
    fn pure(&mut self, expr: &Expr, depth: u32) -> Result<Option<String>, Diagnostic> {
        // A balanced pure tree can grow without a larger depth. Bound its node count before aggregation.
        let mut work = vec![expr];
        let mut nodes = 0;
        while let Some(expr) = work.pop() {
            nodes += 1;
            if nodes > PURE_EXPRESSION_NODES {
                return Ok(None);
            }
            match &expr.kind {
                E::Unary(_, a) => work.push(a),
                E::Logical(_, a, b) | E::Binary(_, a, b) => {
                    work.push(a);
                    work.push(b);
                }
                _ => {}
            }
        }
        self.pure_bounded(expr, depth)
    }
    fn pure_bounded(&mut self, expr: &Expr, depth: u32) -> Result<Option<String>, Diagnostic> {
        let depth = self.guard(depth)?;
        Ok(match &expr.kind {
            E::Number(n) => {
                if n.is_infinite() {
                    self.capture("__ulua_huge");
                }
                Some(number(*n))
            }
            E::Null => Some("nil".into()),
            E::Boolean(b) => Some(b.to_string()),
            E::String(s) => Some(quote(s)),
            E::Read(id) => Some(self.read_binding(*id)?),
            E::Unary(op, a) => self
                .pure_bounded(a, depth)?
                .map(|a| format!("({op} ({a}))")),
            E::Logical(_, a, b) if matches!((&a.kind, &b.kind), (E::Read(a), E::Read(b)) if a == b) =>
            {
                // Boolean reads have no effects. One read also avoids a duplicate-condition lint after a direct literal write.
                self.pure_bounded(a, depth)?
            }
            E::Logical(and, a, b) => {
                match (self.pure_bounded(a, depth)?, self.pure_bounded(b, depth)?) {
                    (Some(a), Some(b)) => {
                        Some(format!("({a} {} {b})", if *and { "and" } else { "or" }))
                    }
                    _ => None,
                }
            }
            E::Binary(op, a, b) => {
                let types = [a.ty, b.ty];
                match (self.pure_bounded(a, depth)?, self.pure_bounded(b, depth)?) {
                    (Some(a), Some(b)) => Some(self.expression_binary(op, types, &a, &b)),
                    _ => None,
                }
            }
            _ => None,
        })
    }
    pub(super) fn expression_binary(
        &mut self,
        op: &str,
        types: [Type; 2],
        a: &str,
        b: &str,
    ) -> String {
        // C2 validates a null operand before flow narrowing can remove the other's nullable type.
        let native = (types[0] == types[1]
            && matches!(types[0], Type::Number | Type::String | Type::Boolean))
            || types.contains(&Type::Null);
        if matches!(op, "==" | "~=") && native {
            if types[0] == types[1] && matches!(types[0], Type::Boolean | Type::String) {
                // Strict Luau rejects distinct singleton refinements; restore the checked primitive.
                let ty = self.function_types.luau(&types[0]);
                format!("((({a}) :: {ty}) {op} (({b}) :: {ty}))")
            } else {
                format!("({a} {op} {b})")
            }
        } else {
            self.capture_binary(op);
            self.binary(op, a, b)
        }
    }
    pub(super) fn binary(&self, op: &str, a: &str, b: &str) -> String {
        match op {
            "%" => format!("__ulua_fmod({a}, {b})"),
            ir::NATIVE_REMAINDER => {
                #[cfg(feature = "remainder-shadow")]
                if self.remainder_shadow {
                    return format!("__ulua_native_remainder({a}, {b})");
                }
                format!("({a} % {b})")
            }
            "==" => format!("__ulua_equal({a}, {b})"),
            "~=" => format!("__ulua_unequal({a}, {b})"),
            _ => format!("({a} {op} {b})"),
        }
    }
    pub(super) fn capture_binary(&mut self, op: &str) {
        match op {
            "%" => self.capture("__ulua_fmod"),
            #[cfg(feature = "remainder-shadow")]
            ir::NATIVE_REMAINDER if self.remainder_shadow => {
                self.capture("__ulua_native_remainder")
            }
            "==" => self.capture("__ulua_equal"),
            "~=" => self.capture("__ulua_unequal"),
            _ => {}
        }
    }
}

// Equal read arms share the final read, after all tests of the selected path.
fn same_conditional_read(mut expr: &Expr) -> Option<(usize, Vec<&Expr>)> {
    let mut id = None;
    let mut tests = vec![];
    loop {
        match &expr.kind {
            E::Read(last) => return (id == Some(*last)).then_some((*last, tests)),
            E::Conditional(t, a, b)
                if t.ty == Type::Boolean && matches!(t.kind, E::Read(_) | E::Boolean(_)) =>
            {
                let E::Read(read) = a.kind else {
                    return None;
                };
                if id.is_some_and(|id| id != read) {
                    return None;
                }
                id = Some(read);
                tests.push(t.as_ref());
                expr = b;
            }
            _ => return None,
        }
    }
}
