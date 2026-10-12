//! C16 joins cached condition edges and loop exits through persistent path heads.
pub(super) use super::paths::Facts;
use super::*;
use crate::ir::{Edge, Expr, ExprKind as E, Flow, Statement as S};
pub(super) struct LoopFacts {
    mark: usize,
    exit: Option<Edge>,
    next: Option<Edge>,
    reachable: bool,
}
impl Checker<'_> {
    fn current_edge(&self, reachable: bool) -> Edge {
        Edge {
            head: self.facts.head(),
            reachable,
        }
    }
    // A meet preserves the caller's path and visits only divergent fact tails.
    fn meet(&mut self, a: Edge, b: Edge) -> Edge {
        if !a.reachable {
            return b;
        }
        if !b.reachable || a.head == b.head {
            return a;
        }
        let common = self.facts.common(a.head, b.head);
        if (common == a.head && self.facts.extends(common, b.head))
            || (common == b.head && self.facts.extends(common, a.head))
        {
            return Edge {
                head: common,
                reachable: true,
            };
        }
        let saved = self.facts.head();
        self.facts.travel(common);
        let mark = self.facts.mark();
        self.facts.travel(a.head);
        let first = self.facts.changes(mark);
        self.facts.travel(b.head);
        let second = self.facts.changes(mark);
        self.facts.travel(common);
        self.facts.merge(first, second);
        let result = self.current_edge(true);
        self.facts.travel(saved);
        result
    }
    pub(super) fn condition(&mut self, kind: &E) -> Flow {
        let base = self.current_edge(true);
        let unknown = Flow {
            yes: base,
            no: base,
        };
        match kind {
            E::Boolean(value) => Flow {
                yes: Edge {
                    reachable: *value,
                    ..base
                },
                no: Edge {
                    reachable: !*value,
                    ..base
                },
            },
            E::Unary("not", a) => {
                let a = a.flow.unwrap_or(unknown);
                Flow {
                    yes: a.no,
                    no: a.yes,
                }
            }
            E::Binary(op, a, b) if matches!(*op, "==" | "~=") => {
                let reference = if matches!(b.kind, E::Null) {
                    a
                } else if matches!(a.kind, E::Null) {
                    b
                } else {
                    return unknown;
                };
                if let E::Read(id) = reference.kind {
                    let binding = &self.bindings[id];
                    if !binding.mutable
                        || (binding.owner != 0
                            && binding.owner == self.owner
                            && !self.captured_writes.contains(&binding.start))
                    {
                        let old = self.facts.get(id);
                        let mark = self.facts.mark();
                        let yes_value = *op == "~=";
                        self.facts.set(id, Some(yes_value));
                        let yes = self.current_edge(old.is_none_or(|v| v == yes_value));
                        self.facts.restore(mark);
                        self.facts.set(id, Some(!yes_value));
                        let no = self.current_edge(old.is_none_or(|v| v != yes_value));
                        self.facts.restore(mark);
                        return Flow { yes, no };
                    }
                }
                unknown
            }
            E::Logical(and, a, b) => {
                let a = a.flow.unwrap_or(unknown);
                let b = b.flow.unwrap_or(unknown);
                if *and {
                    Flow {
                        yes: Edge {
                            reachable: a.yes.reachable && b.yes.reachable,
                            ..b.yes
                        },
                        no: self.meet(
                            a.no,
                            Edge {
                                reachable: a.yes.reachable && b.no.reachable,
                                ..b.no
                            },
                        ),
                    }
                } else {
                    Flow {
                        yes: self.meet(
                            a.yes,
                            Edge {
                                reachable: a.no.reachable && b.yes.reachable,
                                ..b.yes
                            },
                        ),
                        no: Edge {
                            reachable: a.no.reachable && b.no.reachable,
                            ..b.no
                        },
                    }
                }
            }
            E::Conditional(t, a, b) => {
                let t = t.flow.unwrap_or(unknown);
                let a = a.flow.unwrap_or(unknown);
                let b = b.flow.unwrap_or(unknown);
                Flow {
                    yes: self.meet(
                        Edge {
                            reachable: t.yes.reachable && a.yes.reachable,
                            ..a.yes
                        },
                        Edge {
                            reachable: t.no.reachable && b.yes.reachable,
                            ..b.yes
                        },
                    ),
                    no: self.meet(
                        Edge {
                            reachable: t.yes.reachable && a.no.reachable,
                            ..a.no
                        },
                        Edge {
                            reachable: t.no.reachable && b.no.reachable,
                            ..b.no
                        },
                    ),
                }
            }
            _ => unknown,
        }
    }
    pub(super) fn edge(&mut self, e: &Expr, truth: bool) -> bool {
        if let Some(flow) = e.flow {
            let path = if truth { flow.yes } else { flow.no };
            self.facts.travel(path.head);
            path.reachable
        } else {
            !matches!(e.kind,E::Boolean(value) if value!=truth)
        }
    }
    pub(super) fn enter_loop(&mut self) {
        self.loop_facts.push(LoopFacts {
            mark: self.facts.mark(),
            exit: None,
            next: None,
            reachable: self.reachable,
        });
    }
    fn loop_edge(&mut self, continuing: bool) {
        if !self.reachable {
            return;
        }
        let Some(context) = self.loop_facts.last() else {
            return;
        };
        let previous = if continuing {
            context.next
        } else {
            context.exit
        };
        let current = self.current_edge(true);
        let joined = previous.map_or(current, |previous| self.meet(previous, current));
        if let Some(context) = self.loop_facts.last_mut() {
            if continuing {
                context.next = Some(joined);
            } else {
                context.exit = Some(joined);
            }
        }
    }
    pub(super) fn break_edge(&mut self) {
        self.loop_edge(false);
    }
    pub(super) fn continue_edge(&mut self) {
        self.loop_edge(true);
    }
    pub(super) fn loop_update(&mut self) {
        self.continue_edge();
        let next = self.loop_facts.last().and_then(|c| c.next);
        if let Some(next) = next {
            self.facts.travel(next.head);
            self.reachable = next.reachable;
        } else {
            self.reachable = false;
        }
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
            let possible = self.edge(test, false);
            let path = self.current_edge(context.reachable && possible);
            if path.reachable {
                exit = Some(exit.map_or(path, |previous| self.meet(previous, path)));
            }
            self.facts.restore(context.mark);
        }
        self.reachable = context.reachable && exit.is_some();
        if let Some(exit) = exit {
            self.facts.travel(exit.head);
        }
    }
    // Initial entry joins every back edge. Any loop writer removes its entry fact.
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
pub(super) fn exits(s: &S) -> bool {
    match s.form() {
        S::Return(_) | S::Break | S::Continue => true,
        S::Block(body) => body.iter().any(exits),
        S::If(_, a, Some(b)) => exits(a) && exits(b),
        _ => false,
    }
}
