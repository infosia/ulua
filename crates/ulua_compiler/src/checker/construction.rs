//! Constructor paths use bounded bit sets. Every return and every receiver use needs complete fields.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement as S};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct State([u64; crate::FIELD_BOUND / 64]);
impl State {
    fn complete(self) -> bool {
        self.0.iter().all(|word| *word == 0)
    }
    fn assign(&mut self, index: usize) {
        self.0[index / 64] &= !(1 << (index % 64));
    }
    fn meet(&mut self, other: Self) {
        for (a, b) in self.0.iter_mut().zip(other.0) {
            *a |= b;
        }
    }
}
fn join(states: impl IntoIterator<Item = State>) -> Option<State> {
    let mut states = states.into_iter();
    let mut first = states.next()?;
    for state in states {
        first.meet(state);
    }
    Some(first)
}
struct Paths<'a, 's> {
    checker: &'a Checker<'s>,
    receiver: usize,
    fields: HashMap<usize, usize>,
    closures: HashSet<usize>,
    breaks: Vec<Vec<State>>,
    continues: Vec<Vec<State>>,
    pending: Vec<Pending>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Return,
    Throw,
    Break,
    Continue,
}
#[derive(Clone, Copy)]
struct Exit {
    kind: Kind,
    state: State,
    offset: u32,
}
struct Pending {
    loops: usize,
    exits: Vec<Exit>,
}
pub(super) fn check(
    c: &Checker<'_>,
    body: &[S],
    params: &[(usize, Option<Expr>)],
    receiver: usize,
    missing: HashSet<usize>,
    span: Span,
) -> Result<(), Diagnostic> {
    let mut state = State([0; crate::FIELD_BOUND / 64]);
    let fields: HashMap<_, _> = missing
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            state.0[i / 64] |= 1 << (i % 64);
            (key, i)
        })
        .collect();
    // Each closure carries transitive receiver reads, including arrows nested inside arrows.
    let mut closures = HashSet::new();
    let owner = c.bindings[receiver].owner;
    for &(mut reader, id, _) in &c.references {
        if id != receiver {
            continue;
        }
        while reader != owner && reader != 0 {
            let id = reader - 1;
            if !closures.insert(id) {
                break;
            }
            reader = c.bindings[id].owner;
        }
    }
    let mut paths = Paths {
        checker: c,
        receiver,
        fields,
        closures,
        breaks: vec![],
        continues: vec![],
        pending: vec![],
    };
    for (_, default) in params {
        if let Some(e) = default {
            paths.expression(e, state)?;
        }
    }
    if let Some(state) = paths.body(body, state)? {
        paths.complete(state, span.start)?;
    }
    Ok(())
}
impl Paths<'_, '_> {
    fn complete(&self, state: State, offset: u32) -> Result<(), Diagnostic> {
        if !state.complete() {
            return Err(Diagnostic::at(
                self.checker.source,
                offset as usize,
                "C13",
                "This needs every field assigned on every path",
            ));
        }
        Ok(())
    }
    fn expression(&mut self, expression: &Expr, state: State) -> Result<(), Diagnostic> {
        if state.complete() {
            return Ok(());
        }
        let mut pending = vec![expression];
        while let Some(e) = pending.pop() {
            let raises = match &e.kind {
                E::Call(c, _) => !matches!(c.kind, E::Builtin(_)),
                E::ClassCall(_, _, _, _) | E::OptionalCall(_, _, _) => true,
                E::Method(m, _, _, _) | E::OptionalMethod(m, _, _, _) => matches!(
                    m,
                    ir::Method::Table(crate::tables::TableMethod::Each)
                        | ir::Method::ForEach
                        | ir::Method::Map
                        | ir::Method::Filter
                        | ir::Method::Some
                        | ir::Method::Every
                        | ir::Method::Find
                        | ir::Method::FindLast
                        | ir::Method::FindIndex
                        | ir::Method::FindLastIndex
                        | ir::Method::FlatMap
                        | ir::Method::Reduce
                        | ir::Method::ReduceRight
                        | ir::Method::Sort
                ),
                _ => false,
            };
            if raises {
                self.exit(Exit {
                    kind: Kind::Throw,
                    state,
                    offset: e.start,
                })?;
            }
            match &e.kind {
                E::Read(id) if *id == self.receiver => self.complete(state, e.start)?,
                E::Closure(id) if self.closures.contains(id) => self.complete(state, e.start)?,
                E::Error(_, a)
                | E::InstanceOf(a, _)
                | E::ErrorField(a, _)
                | E::OptionalErrorField(a, _)
                | E::Field(a, _)
                | E::OptionalField(a, _)
                | E::OptionalLength(a)
                | E::Length(a)
                | E::Json(_, a)
                | E::Await(a)
                | E::Unary(_, a) => pending.push(a),
                E::Coalesce(a, b)
                | E::Binary(_, a, b)
                | E::Logical(_, a, b)
                | E::Index(a, b, _)
                | E::Push(a, b)
                | E::OptionalPush(a, b) => pending.extend([a.as_ref(), b.as_ref()]),
                E::Conditional(t, a, b) => pending.extend([t.as_ref(), a.as_ref(), b.as_ref()]),
                E::Call(c, args)
                | E::OptionalCall(c, _, args)
                | E::Method(_, _, c, args)
                | E::OptionalMethod(_, _, c, args) => {
                    pending.push(c);
                    pending.extend(args);
                }
                E::ClassCall(c, receiver, args, _) => {
                    pending.extend([c.as_ref(), receiver.as_ref()]);
                    pending.extend(args);
                }
                E::Object(fields) => pending.extend(fields.iter().map(|f| &f.1)),
                E::Array(parts) | E::Template(parts) => pending.extend(parts),
                _ => {}
            }
        }
        Ok(())
    }
    fn body(&mut self, body: &[S], mut state: State) -> Result<Option<State>, Diagnostic> {
        for statement in body {
            let Some(next) = self.statement(statement, state)? else {
                return Ok(None);
            };
            state = next;
        }
        Ok(Some(state))
    }
    fn statement(&mut self, s: &S, mut state: State) -> Result<Option<State>, Diagnostic> {
        let offset = match s {
            S::Located(offset, _) => *offset,
            _ => self.checker.bindings[self.receiver].start,
        };
        match s.form() {
            S::Field(receiver, key, None, value) if matches!(receiver.kind, E::Read(id) if id == self.receiver) =>
            {
                self.expression(value, state)?;
                if let Some(index) = self.fields.get(key) {
                    state.assign(*index);
                }
            }
            S::Field(a, _, _, b) => {
                self.expression(a, state)?;
                self.expression(b, state)?;
            }
            S::Element(a, i, _, b, _) => {
                for e in [a, i, b] {
                    self.expression(e, state)?;
                }
            }
            S::Declare(_, e) | S::Assign(_, _, e) | S::Eval(e) => self.expression(e, state)?,
            S::Throw(e) => {
                self.expression(e, state)?;
                self.exit(Exit {
                    kind: Kind::Throw,
                    state,
                    offset,
                })?;
                return Ok(None);
            }
            S::Try(a, b, f) => return self.protected(a, b.as_ref(), f.as_deref(), state),
            S::Return(value) => {
                if let Some(e) = value {
                    self.expression(e, state)?;
                }
                self.exit(Exit {
                    kind: Kind::Return,
                    state,
                    offset,
                })?;
                return Ok(None);
            }
            S::Block(body) => return self.body(body, state),
            S::If(test, a, b) => {
                self.expression(test, state)?;
                let a = self.statement(a, state)?;
                let b = if let Some(b) = b {
                    self.statement(b, state)?
                } else {
                    Some(state)
                };
                return Ok(join(a.into_iter().chain(b)));
            }
            S::While(test, body) => return self.loop_body(Some(test), body, None, state),
            S::For(init, test, update, body, _) => {
                let Some(state) = self.body(init, state)? else {
                    return Ok(None);
                };
                return self.loop_body(test.as_ref(), body, update.as_deref(), state);
            }
            S::ForOf(_, receiver, body) => {
                self.expression(receiver, state)?;
                self.breaks.push(vec![]);
                self.continues.push(vec![]);
                self.statement(body, state)?;
                self.breaks.pop();
                self.continues.pop();
            }
            S::Break => {
                self.exit(Exit {
                    kind: Kind::Break,
                    state,
                    offset,
                })?;
                return Ok(None);
            }
            S::Continue => {
                self.exit(Exit {
                    kind: Kind::Continue,
                    state,
                    offset,
                })?;
                return Ok(None);
            }
            _ => {}
        }
        Ok(Some(state))
    }
    fn exit(&mut self, exit: Exit) -> Result<(), Diagnostic> {
        let crossing = self.pending.last().is_some_and(|p| {
            matches!(exit.kind, Kind::Return | Kind::Throw) || self.breaks.len() <= p.loops
        });
        if crossing {
            if let Some(p) = self.pending.last_mut() {
                // Merge equal completion kinds. A nested finalizer visits at most five incoming states.
                if let Some(old) = p.exits.iter_mut().find(|old| old.kind == exit.kind) {
                    old.state.meet(exit.state);
                } else {
                    p.exits.push(exit);
                }
            }
        } else {
            match exit.kind {
                Kind::Return => self.complete(exit.state, exit.offset)?,
                Kind::Throw => {}
                Kind::Break => {
                    if let Some(states) = self.breaks.last_mut() {
                        states.push(exit.state);
                    }
                }
                Kind::Continue => {
                    if let Some(states) = self.continues.last_mut() {
                        states.push(exit.state);
                    }
                }
            }
        }
        Ok(())
    }
    fn protected(
        &mut self,
        a: &S,
        b: Option<&(Option<usize>, Box<S>)>,
        f: Option<&S>,
        state: State,
    ) -> Result<Option<State>, Diagnostic> {
        self.pending.push(Pending {
            loops: self.breaks.len(),
            exits: vec![],
        });
        let first = self.statement(a, state)?;
        let mut exits = self.pending.pop().map(|p| p.exits).unwrap_or_default();
        let second = if let Some((_, b)) = b {
            exits.retain(|e| e.kind != Kind::Throw);
            self.pending.push(Pending {
                loops: self.breaks.len(),
                exits: vec![],
            });
            // Any call can raise before a field write. The catch entry uses the incoming field state.
            let normal = self.statement(b, state)?;
            exits.extend(self.pending.pop().map(|p| p.exits).unwrap_or_default());
            normal
        } else {
            None
        };
        let normal = join(first.into_iter().chain(second));
        let normal = if let Some(f) = f {
            let incoming =
                join(normal.into_iter().chain(exits.iter().map(|e| e.state))).unwrap_or(state);
            self.pending.push(Pending {
                loops: self.breaks.len(),
                exits: vec![],
            });
            let final_state = self.statement(f, incoming)?;
            let replaced = self.pending.pop().map(|p| p.exits).unwrap_or_default();
            let normal = if let Some(final_state) = final_state {
                for exit in &mut exits {
                    for (old, final_word) in exit.state.0.iter_mut().zip(final_state.0) {
                        *old &= final_word;
                    }
                }
                normal.map(|mut state| {
                    for (old, final_word) in state.0.iter_mut().zip(final_state.0) {
                        *old &= final_word;
                    }
                    state
                })
            } else {
                exits.clear();
                None
            };
            exits.extend(replaced);
            normal
        } else {
            normal
        };
        for exit in exits {
            self.exit(exit)?;
        }
        Ok(normal)
    }
    fn loop_body(
        &mut self,
        test: Option<&Expr>,
        body: &S,
        update: Option<&S>,
        state: State,
    ) -> Result<Option<State>, Diagnostic> {
        if let Some(e) = test {
            self.expression(e, state)?;
        }
        self.breaks.push(vec![]);
        self.continues.push(vec![]);
        let tail = self.statement(body, state)?;
        let continues = self.continues.pop().unwrap_or_default();
        if let Some(tail) = join(tail.into_iter().chain(continues))
            && let Some(update) = update
        {
            self.statement(update, tail)?;
        }
        let mut exits = self.breaks.pop().unwrap_or_default();
        if test.is_some_and(|test| !test.bare_true) {
            exits.push(state);
        }
        Ok(join(exits))
    }
}
// Empty constructor returns preserve the instance. Nested functions retain their own returns.
pub(super) fn receiver_returns(body: &mut [S], receiver: usize, ty: Type, start: u32) {
    let mut pending: Vec<_> = body.iter_mut().collect();
    while let Some(s) = pending.pop() {
        match s {
            S::Located(_, s) => pending.push(s),
            S::Return(None) => {
                *s = S::Return(Some(super::classes::value(ty, E::Read(receiver), start)))
            }
            S::Try(a, b, f) => {
                pending.push(a);
                if let Some((_, b)) = b {
                    pending.push(b);
                }
                pending.extend(f.iter_mut().map(|s| s.as_mut()));
            }
            S::Block(body) => pending.extend(body),
            S::If(_, a, b) => {
                pending.push(a);
                pending.extend(b.iter_mut().map(|s| s.as_mut()));
            }
            S::While(_, body) | S::ForOf(_, _, body) => pending.push(body),
            S::For(_, _, _, body, _) => pending.push(body),
            _ => {}
        }
    }
}
