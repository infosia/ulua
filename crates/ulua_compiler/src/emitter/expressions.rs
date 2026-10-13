//! Reusable typed temporaries fix the time of each value read.
use super::*;
use crate::ir::{Expr, ExprKind as E};
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
        match &expr.kind {
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
                    format!("if {0} ~= nil then #{0} else nil", self.temp(slot)),
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
                self.write_slot(dest, value);
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
                let a = self.expression(a, depth)?;
                let b = self.expression(b, depth)?;
                self.capture_binary(op);
                let value = self.binary(op, &self.temp(a), &self.temp(b));
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
                self.write_slot(dest, self.temp(a));
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
                if let Some((id, tests)) = same_conditional_read(expr) {
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
            E::Index(a, i) => {
                let (a, deferred) = self.access_receiver(a, depth)?;
                let i = self.expression(i, depth)?;
                self.access_receiver_check(a, deferred);
                self.capture("__ulua_get");
                self.write_slot(
                    dest,
                    format!("__ulua_get({}, {})", self.temp(a), self.temp(i)),
                );
                self.release(a);
                self.release(i);
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
                        self.write_slot(frozen, converted);
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
            E::Unary(op, a) => self.pure(a, depth)?.map(|a| format!("({op} ({a}))")),
            E::Logical(and, a, b) => match (self.pure(a, depth)?, self.pure(b, depth)?) {
                (Some(a), Some(b)) => {
                    Some(format!("({a} {} {b})", if *and { "and" } else { "or" }))
                }
                _ => None,
            },
            E::Binary(op, a, b) => match (self.pure(a, depth)?, self.pure(b, depth)?) {
                (Some(a), Some(b)) => {
                    self.capture_binary(op);
                    Some(self.binary(op, &a, &b))
                }
                _ => None,
            },
            _ => None,
        })
    }
    pub(super) fn binary(&self, op: &str, a: &str, b: &str) -> String {
        match op {
            "%" => format!("__ulua_fmod({a}, {b})"),
            "==" => format!("__ulua_equal({a}, {b})"),
            "~=" => format!("__ulua_unequal({a}, {b})"),
            _ => format!("({a} {op} {b})"),
        }
    }
    fn capture_binary(&mut self, op: &str) {
        match op {
            "%" => self.capture("__ulua_fmod"),
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
