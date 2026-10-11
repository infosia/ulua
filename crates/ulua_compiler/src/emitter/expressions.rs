//! Reusable typed temporaries fix the time of each value read.
use super::*;
use crate::ir::{Expr, ExprKind as E};
impl Emitter<'_> {
    pub(super) fn expression(&mut self, expr: &Expr, depth: u32) -> Result<usize, Diagnostic> {
        let depth = self.guard(depth)?;
        let dest = self.slot(if expr.ty == Type::Void {
            &Type::Boolean
        } else {
            &expr.ty
        });
        let target = self.temp(dest);
        match &expr.kind {
            E::Number(n) => self.line(format!("{target} = {}", number(*n))),
            E::Boolean(b) => self.line(format!("{target} = {b}")),
            E::String(s) => self.line(format!("{target} = {}", quote(s))),
            E::Read(id) => self.line(format!("{target} = {}", self.location(*id)?)),
            E::Builtin(name) => self.line(format!("{target} = {name}")),
            E::Unary(op, a) => {
                let a = self.expression(a, depth)?;
                self.line(format!("{target} = {op} ({})", self.temp(a)));
                self.release(a);
            }
            E::Binary(op, a, b) => {
                let a = self.expression(a, depth)?;
                let b = self.expression(b, depth)?;
                let value = self.binary(op, &self.temp(a), &self.temp(b));
                self.line(format!("{target} = {value}"));
                self.release(a);
                self.release(b);
            }
            E::Logical(and, a, b) => {
                let a = self.expression(a, depth)?;
                self.line(format!("{target} = {}", self.temp(a)));
                self.release(a);
                self.line(format!(
                    "if {}{target} then",
                    if *and { "" } else { "not " }
                ));
                self.indent += 1;
                let b = self.expression(b, depth)?;
                self.line(format!(
                    "{target} = {target} {} {}",
                    if *and { "and" } else { "or" },
                    self.temp(b)
                ));
                self.release(b);
                self.indent -= 1;
                self.line("end");
            }
            E::Conditional(test, a, b) => {
                let t = self.expression(test, depth)?;
                if let (Some(a), Some(b)) = (self.pure(a, depth)?, self.pure(b, depth)?) {
                    self.line(format!("{target} = if {} then {a} else {b}", self.temp(t)));
                } else {
                    // Branch statements preserve short-circuit evaluation of calls.
                    self.line(format!("if {} then", self.temp(t)));
                    self.indent += 1;
                    let a = self.expression(a, depth)?;
                    self.indent -= 1;
                    self.line("else");
                    self.indent += 1;
                    let b = self.expression(b, depth)?;
                    self.indent -= 1;
                    self.line("end");
                    self.line(format!(
                        "{target} = if {} then {} else {}",
                        self.temp(t),
                        self.temp(a),
                        self.temp(b)
                    ));
                    self.release(a);
                    self.release(b);
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
                    self.line(format!("{target} = {call}"));
                }
                self.release(c);
                for arg in args {
                    self.release(arg);
                }
            }
            E::Template(parts) => {
                self.line(format!("{target} = \"\""));
                for part in parts {
                    let value = self.expression(part, depth)?;
                    let converted = match part.ty {
                        Type::Number => format!("__ulua_format({})", self.temp(value)),
                        Type::Boolean => format!("__ulua_tostring({})", self.temp(value)),
                        _ => self.temp(value),
                    };
                    self.line(format!("{target} = {target} .. {converted}"));
                    self.release(value);
                }
            }
        }
        Ok(dest)
    }
    fn pure(&self, expr: &Expr, depth: u32) -> Result<Option<String>, Diagnostic> {
        let depth = self.guard(depth)?;
        Ok(match &expr.kind {
            E::Number(n) => Some(number(*n)),
            E::Boolean(b) => Some(b.to_string()),
            E::String(s) => Some(quote(s)),
            E::Read(id) => Some(self.location(*id)?),
            E::Unary(op, a) => self.pure(a, depth)?.map(|a| format!("({op} ({a}))")),
            E::Binary(op, a, b) => match (self.pure(a, depth)?, self.pure(b, depth)?) {
                (Some(a), Some(b)) => Some(self.binary(op, &a, &b)),
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
