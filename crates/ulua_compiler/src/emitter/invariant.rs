//! Loop-length eligibility is a query over checked IR, including nested loop heads.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement as S};
use std::collections::{BTreeMap, BTreeSet};

// These are library aliases of item 20, not generated helpers or native host functions.
fn builtin_alias(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        E::Builtin(
            "__ulua_fmod"
                | "__ulua_floor"
                | "__ulua_ceil"
                | "__ulua_math_abs"
                | "__ulua_math_sqrt"
                | "__ulua_tostring"
                | "__ulua_gsub"
                | "__ulua_print"
        )
    )
}

fn candidates(body: &S) -> BTreeMap<usize, &Expr> {
    let mut statements = vec![body];
    let mut expressions = vec![];
    let mut writes = BTreeSet::new();
    let mut arrays = BTreeMap::new();
    while let Some(s) = statements.pop() {
        match s.form() {
            S::Declare(id, e) | S::Assign(id, _, e) => {
                writes.insert(*id);
                expressions.push((e, true));
            }
            S::Eval(e) | S::Throw(e) => expressions.push((e, true)),
            S::Field(a, _, _, e) => expressions.extend([(a, false), (e, true)]),
            S::ErrorField(a, _, _, e) => expressions.extend([(a, true), (e, true)]),
            S::Element(..) => return BTreeMap::new(),
            S::ForOf(id, a, b) => {
                writes.insert(*id);
                expressions.push((a, true));
                statements.push(b);
            }
            S::Block(b) => statements.extend(b),
            S::If(e, a, b) => {
                expressions.push((e, true));
                statements.push(a);
                statements.extend(b.iter().map(|b| b.as_ref()));
            }
            S::While(e, b) => {
                expressions.push((e, true));
                statements.push(b);
            }
            S::For(init, test, update, b, _) => {
                statements.extend(init);
                expressions.extend(test.iter().map(|e| (e, true)));
                statements.extend(update.iter().map(|s| s.as_ref()));
                statements.push(b);
            }
            S::Try(a, b, f) => {
                statements.push(a);
                if let Some((id, b)) = b {
                    writes.extend(id);
                    statements.push(b);
                }
                statements.extend(f.iter().map(|s| s.as_ref()));
            }
            S::Return(e) => expressions.extend(e.iter().map(|e| (e, true))),
            S::Entry(_) | S::Function(_) | S::Break | S::Continue => {}
            S::Located(_, s) => statements.push(s),
        }
    }
    while let Some((e, checked)) = expressions.pop() {
        match &e.kind {
            E::Call(c, args) if builtin_alias(c) => {
                expressions.extend(args.iter().map(|e| (e, true)))
            }
            E::Call(..)
            | E::ClassCall(..)
            | E::OptionalCall(..)
            | E::Method(..)
            | E::OptionalMethod(..)
            | E::Push(..)
            | E::OptionalPush(..)
            | E::Json(..)
            | E::Await(..)
            | E::Error(..) => return BTreeMap::new(),
            E::Index(a, i) => {
                if checked && let E::Read(id) = a.kind {
                    arrays.insert(id, a.as_ref());
                }
                expressions.extend([(a.as_ref(), false), (i.as_ref(), true)]);
            }
            E::Field(a, _) => expressions.push((a, false)),
            E::InstanceOf(a, _)
            | E::ErrorField(a, _)
            | E::OptionalErrorField(a, _)
            | E::OptionalField(a, _)
            | E::OptionalLength(a)
            | E::Unary(_, a)
            | E::Length(a) => expressions.push((a, true)),
            E::Coalesce(a, b) | E::Binary(_, a, b) | E::Logical(_, a, b) => {
                expressions.extend([(a.as_ref(), true), (b.as_ref(), true)]);
            }
            E::Conditional(t, a, b) => {
                expressions.extend([(t.as_ref(), true), (a.as_ref(), true), (b.as_ref(), true)])
            }
            E::Object(fields) => expressions.extend(fields.iter().map(|(_, e)| (e, true))),
            E::Template(parts) | E::Array(parts) => {
                expressions.extend(parts.iter().map(|e| (e, true)))
            }
            E::Null
            | E::Number(_)
            | E::Boolean(_)
            | E::String(_)
            | E::Read(_)
            | E::Closure(_)
            | E::Builtin(_) => {}
        }
    }
    arrays.retain(|id, _| !writes.contains(id));
    arrays
}

impl Emitter<'_> {
    pub(super) fn invariant_length(&mut self, array: &Expr) -> Option<String> {
        if let E::Read(id) = array.kind {
            let length = self.invariant_lengths.get(&id).cloned();
            if let Some(length) = &length {
                self.capture(length);
            }
            length
        } else {
            None
        }
    }

    pub(super) fn begin_invariant_lengths(&mut self, body: &S) -> Result<Vec<usize>, Diagnostic> {
        // The ordinary storage probe fixes spills first. Optional locals use only its spare capacity.
        let capacity = self
            .context
            .and_then(|c| c.layouts[self.owner].as_ref())
            .map_or(0, |l| l.length_capacity);
        if self.invariant_lengths.len() >= capacity {
            return Ok(vec![]);
        }
        let mut added = vec![];
        for (id, expr) in candidates(body) {
            if self.invariant_lengths.len() >= capacity {
                break;
            }
            if self.module.bindings[id]
                .ty
                .is_none_or(|ty| self.module.types.element(ty).is_none())
            {
                continue;
            }
            if self.invariant_lengths.contains_key(&id) {
                continue;
            }
            let array = self.pure_operand(expr)?;
            let length = self.generated()?;
            self.line(format!("local {length}: number = #{array}"));
            self.invariant_lengths.insert(id, length);
            added.push(id);
        }
        Ok(added)
    }
}

#[cfg(test)]
#[path = "invariant_review_tests.rs"]
mod review_tests;
