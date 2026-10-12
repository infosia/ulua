//! C16 edge facts use one undo log. Joins inspect only changed bindings.
use super::*;
use crate::ir::{Expr, ExprKind as E, Statement as S};
type Fact = (bool, u64);
type Undo = (usize, Option<Fact>, u64);
#[derive(Default)]
pub(super) struct Facts {
    values: HashMap<usize, Fact>,
    log: Vec<Undo>,
    pub(super) floor: u64,
    pub(super) serial: u64,
}
impl Facts {
    pub(super) fn mark(&self) -> usize {
        self.log.len()
    }
    pub(super) fn get(&self, id: usize) -> Option<bool> {
        self.values
            .get(&id)
            .filter(|value| value.1 >= self.floor)
            .map(|value| value.0)
    }
    pub(super) fn set(&mut self, id: usize, value: Option<bool>) {
        let old = self.get(id);
        if old == value {
            return;
        }
        self.serial += 1;
        self.log
            .push((id, self.values.get(&id).copied(), self.serial));
        if let Some(value) = value {
            self.values.insert(id, (value, self.serial));
        } else {
            self.values.remove(&id);
        }
    }
    pub(super) fn changes(&self, mark: usize) -> HashMap<usize, Option<bool>> {
        self.log[mark..]
            .iter()
            .map(|(id, _, _)| (*id, self.get(*id)))
            .collect()
    }
    pub(super) fn restore(&mut self, mark: usize) {
        while self.log.len() > mark {
            if let Some((id, old, _)) = self.log.pop() {
                if let Some(old) = old {
                    self.values.insert(id, old);
                } else {
                    self.values.remove(&id);
                }
            }
        }
    }
    pub(super) fn merge(
        &mut self,
        a: HashMap<usize, Option<bool>>,
        b: HashMap<usize, Option<bool>>,
    ) {
        let mut keys: std::collections::HashSet<_> = a.keys().copied().collect();
        keys.extend(b.keys().copied());
        for id in keys {
            let old = self.get(id);
            let a = a.get(&id).copied().unwrap_or(old);
            let b = b.get(&id).copied().unwrap_or(old);
            self.set(id, if a == b { a } else { None });
        }
    }
    pub(super) fn apply(&mut self, changes: HashMap<usize, Option<bool>>) {
        for (id, value) in changes {
            self.set(id, value);
        }
    }
}
pub(super) struct LoopFacts {
    mark: usize,
    exit: Option<HashMap<usize, Option<bool>>>,
    saved: Vec<Undo>,
    baseline: HashMap<usize, Option<bool>>,
    reachable: bool,
}
impl Checker<'_> {
    pub(super) fn enter_loop(&mut self) {
        self.loop_facts.push(LoopFacts {
            mark: self.facts.mark(),
            exit: None,
            saved: vec![],
            baseline: HashMap::new(),
            reachable: self.reachable,
        });
    }
    pub(super) fn break_edge(&mut self) {
        if !self.reachable {
            return;
        }
        let Some(context) = self.loop_facts.last_mut() else {
            return;
        };
        let path = &self.facts.log[context.mark..];
        // Active log prefixes share unique revision tags. Inspect only divergent tails.
        let mut low = 0;
        let mut high = path.len().min(context.saved.len());
        while low < high {
            let mid = low + (high - low).div_ceil(2);
            if path[mid - 1].2 == context.saved[mid - 1].2 {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        let common = low;
        let mut changed: std::collections::HashSet<_> =
            context.saved[common..].iter().map(|e| e.0).collect();
        for (id, old, _) in &path[common..] {
            changed.insert(*id);
            context
                .baseline
                .entry(*id)
                .or_insert(old.filter(|v| v.1 >= self.facts.floor).map(|v| v.0));
        }
        if let Some(exit) = context.exit.as_mut() {
            for id in changed {
                let before = context.baseline.get(&id).copied().unwrap_or(None);
                let value = exit.entry(id).or_insert(before);
                if *value != self.facts.get(id) {
                    *value = None;
                }
            }
        } else {
            context.exit = Some(self.facts.changes(context.mark));
        }
        context.saved.truncate(common);
        context.saved.extend_from_slice(&path[common..]);
    }
    pub(super) fn leave_loop(&mut self, test: Option<&Expr>) {
        let Some(context) = self.loop_facts.pop() else {
            return;
        };
        self.facts.restore(context.mark);
        let mut exit = context.exit;
        if let Some(test) = test
            && !test.bare_true
        {
            if context.reachable && self.edge(test, false) {
                let path = self.facts.changes(context.mark);
                self.facts.restore(context.mark);
                if let Some(previous) = exit.take() {
                    self.facts.merge(previous, path);
                    exit = Some(self.facts.changes(context.mark));
                    self.facts.restore(context.mark);
                } else {
                    exit = Some(path);
                }
            } else {
                self.facts.restore(context.mark);
            }
        }
        self.reachable = context.reachable && exit.is_some();
        if let Some(exit) = exit {
            self.facts.apply(exit);
        }
    }
    pub(super) fn loop_writes(&mut self, span: Span) {
        let begin = self.assignments.partition_point(|a| a.1 < span.start);
        let end = self.assignments.partition_point(|a| a.1 < span.end);
        for (start, _) in &self.assignments[begin..end] {
            if let Some(id) = self.binding_starts.get(start)
                && self.bindings[*id].mutable
            {
                self.facts.set(*id, None);
            }
        }
    }
    pub(super) fn read_type(&mut self, id: usize, ty: Type, _offset: u32) -> Type {
        let binding = &self.bindings[id];
        if binding.mutable
            && (binding.owner == 0
                || binding.owner != self.owner
                || self.captured_writes.contains(&binding.start))
        {
            return ty;
        }
        if let Some(inner) = self.types.inner(ty)
            && let Some(nonnull) = self.facts.get(id)
        {
            return if nonnull { inner } else { Type::Null };
        }
        ty
    }
    pub(super) fn original_type(&self, e: &Expr) -> Type {
        if let E::Read(id) = e.kind {
            self.bindings[id].ty.unwrap_or(e.ty)
        } else {
            e.ty
        }
    }
    pub(super) fn edge(&mut self, e: &Expr, truth: bool) -> bool {
        match &e.kind {
            E::Boolean(value) => *value == truth,
            E::Unary("not", a) => self.edge(a, !truth),
            E::Binary(op, a, b) if matches!(*op, "==" | "~=") => {
                let reference = if matches!(b.kind, E::Null) {
                    a
                } else if matches!(a.kind, E::Null) {
                    b
                } else {
                    return true;
                };
                if let E::Read(id) = reference.kind {
                    let binding = &self.bindings[id];
                    if !binding.mutable
                        || (binding.owner != 0
                            && binding.owner == self.owner
                            && !self.captured_writes.contains(&binding.start))
                    {
                        let nonnull = truth == (*op == "~=");
                        let reachable = self.facts.get(id).is_none_or(|old| old == nonnull);
                        self.facts.set(id, Some(nonnull));
                        return reachable;
                    }
                }
                true
            }
            E::Logical(and, a, b) => {
                if truth == *and {
                    let first = self.edge(a, truth);
                    let second = self.edge(b, truth);
                    first && second
                } else {
                    let mark = self.facts.mark();
                    let reachable_first = self.edge(a, truth);
                    let first = self.facts.changes(mark);
                    self.facts.restore(mark);
                    let left = self.edge(a, !truth);
                    let right = self.edge(b, truth);
                    let reachable_second = left && right;
                    let second = self.facts.changes(mark);
                    self.facts.restore(mark);
                    if !reachable_first {
                        self.facts.apply(second);
                    } else if !reachable_second {
                        self.facts.apply(first);
                    } else {
                        self.facts.merge(first, second);
                    }
                    reachable_first || reachable_second
                }
            }
            E::Conditional(t, a, b) => {
                let mark = self.facts.mark();
                let test_first = self.edge(t, true);
                let arm_first = self.edge(a, truth);
                let reachable_first = test_first && arm_first;
                let first = self.facts.changes(mark);
                self.facts.restore(mark);
                let test_second = self.edge(t, false);
                let arm_second = self.edge(b, truth);
                let reachable_second = test_second && arm_second;
                let second = self.facts.changes(mark);
                self.facts.restore(mark);
                if !reachable_first {
                    self.facts.apply(second);
                } else if !reachable_second {
                    self.facts.apply(first);
                } else {
                    self.facts.merge(first, second);
                }
                reachable_first || reachable_second
            }
            _ => true,
        }
    }
    pub(super) fn nullable_use(&self, span: Span, types: &[Type]) -> Result<(), Diagnostic> {
        if types.iter().any(|ty| self.types.contains_null(*ty)) {
            return Err(self.error(
                span,
                "C16",
                "Nullable value needs a stable null test or coalescing",
            ));
        }
        Ok(())
    }
}
// An exit removes that branch from the continuation join.
pub(super) fn exits(s: &S) -> bool {
    match s.form() {
        S::Return(_) | S::Break | S::Continue => true,
        S::Block(body) => body.iter().any(exits),
        S::If(_, a, Some(b)) => exits(a) && exits(b),
        _ => false,
    }
}
