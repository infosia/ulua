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
                if *op == "%" {
                    self.capture("__ulua_fmod");
                }
                let value = self.binary(op, &self.temp(a), &self.temp(b));
                self.write_slot(dest, value);
                self.release(a);
                self.release(b);
            }
            E::Logical(and, a, b) => {
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
            E::Method(name, arity, receiver, arguments) => {
                let a = self.expression(receiver, depth)?;
                let mut slots = vec![a];
                for arg in arguments {
                    slots.push(self.expression(arg, depth)?);
                }
                let helper = methods::name(name, *arity);
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
                let a = self.expression(a, depth)?;
                let i = self.expression(i, depth)?;
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
            E::Boolean(b) => Some(b.to_string()),
            E::String(s) => Some(quote(s)),
            E::Read(id) => Some(self.read_binding(*id)?),
            E::Unary(op, a) => self.pure(a, depth)?.map(|a| format!("({op} ({a}))")),
            E::Binary(op, a, b) => match (self.pure(a, depth)?, self.pure(b, depth)?) {
                (Some(a), Some(b)) => {
                    if *op == "%" {
                        self.capture("__ulua_fmod");
                    }
                    Some(self.binary(op, &a, &b))
                }
                _ => None,
            },
            _ => None,
        })
    }
    pub(super) fn binary(&self, op: &str, a: &str, b: &str) -> String {
        if op == "%" {
            format!("__ulua_fmod({a}, {b})")
        } else {
            format!("({a} {op} {b})")
        }
    }
}
