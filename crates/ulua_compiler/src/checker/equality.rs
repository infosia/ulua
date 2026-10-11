//! C7 structural operands and forward tested facts. No literal types are modeled.
use super::*;
use crate::ir::{Expr, ExprKind as E};

pub(super) struct Equality {
    pub(super) bindings: Vec<bool>,
    pub(super) tested: Vec<bool>,
    undo: Vec<usize>,
    nodes: Vec<(usize, usize)>,
}
impl Equality {
    pub(super) fn new() -> Self {
        Self {
            bindings: vec![],
            tested: vec![],
            undo: vec![],
            nodes: vec![(0, 0)],
        }
    }
    pub(super) fn checkpoint(&self) -> usize {
        self.undo.len()
    }
    pub(super) fn restore(&mut self, checkpoint: usize) {
        while self.undo.len() > checkpoint {
            if let Some(id) = self.undo.pop() {
                self.tested[self.nodes[id].1] = false;
            }
        }
    }
    pub(super) fn enter(&mut self, ids: &[usize]) {
        for &id in ids {
            if !self.tested[id] {
                self.tested[id] = true;
                let parent = self.tip();
                self.nodes.push((parent, id));
                self.undo.push(self.nodes.len() - 1);
            }
        }
    }
    fn tip(&self) -> usize {
        self.undo.last().copied().unwrap_or(0)
    }
    pub(super) fn join(&self) -> Join {
        Join {
            checkpoint: self.checkpoint(),
            anchor: self.tip(),
            arrivals: vec![],
        }
    }
    pub(super) fn finish(&mut self, join: Join) -> bool {
        self.restore(join.checkpoint);
        let reachable = !join.arrivals.is_empty();
        let ids = join.intersection(&self.nodes);
        self.enter(&ids);
        reachable
    }
    // Union uses a work list. No region copies the active tested set.
    pub(super) fn region(&self, condition: &Expr, truth: bool) -> Vec<usize> {
        let mut work = vec![(condition, truth)];
        let mut ids = vec![];
        while let Some((e, truth)) = work.pop() {
            match &e.kind {
                E::Unary("not", g) => work.push((g, !truth)),
                E::Logical(_, a, b) => {
                    work.push((a, truth));
                    work.push((b, truth));
                }
                E::Binary(op, a, b) if (*op == "==" && truth) || (*op == "~=" && !truth) => {
                    for (reference, other) in [(a, b), (b, a)] {
                        if let E::Read(id) = reference.kind
                            && !other.wide
                        {
                            ids.push(id);
                        }
                    }
                }
                _ => {}
            }
        }
        ids
    }
}
// Each edge records one log tip. Shared prefixes are visited once per join, not once per edge.
// Each node is a new tested reference. Parents precede children; restores retain the tree.
// Work is linear in the distinct reference nodes and edges of a join, independent of its baseline.
// A node crosses at most the fixed nesting bound of joins. No statement copies the active set.
pub(super) struct Join {
    checkpoint: usize,
    anchor: usize,
    arrivals: Vec<usize>,
}
impl Join {
    pub(super) fn edge(&mut self, equality: &Equality, reachable: bool) {
        if reachable {
            self.arrivals.push(equality.tip());
        }
    }
    fn intersection(&self, nodes: &[(usize, usize)]) -> Vec<usize> {
        if self.arrivals.is_empty() || self.arrivals.contains(&self.anchor) {
            return vec![];
        }
        if self.arrivals.iter().all(|tip| *tip == self.arrivals[0]) {
            let mut node = self.arrivals[0];
            let mut ids = vec![];
            while node != self.anchor {
                ids.push(nodes[node].1);
                node = nodes[node].0;
            }
            return ids;
        }
        // Counts combine shared paths before they propagate to the parent.
        let mut paths: HashMap<usize, (usize, usize)> = HashMap::new();
        for &tip in &self.arrivals {
            let mut node = tip;
            while node != self.anchor && !paths.contains_key(&node) {
                paths.insert(node, (0, 0));
                node = nodes[node].0;
            }
            if tip != self.anchor {
                paths.entry(tip).or_default().0 += 1;
            }
        }
        let keys: Vec<_> = paths.keys().copied().collect();
        for &node in &keys {
            if let Some(parent) = paths.get_mut(&nodes[node].0) {
                parent.1 += 1;
            }
        }
        let mut ready: Vec<_> = keys
            .into_iter()
            .filter(|n| paths.get(n).is_some_and(|p| p.1 == 0))
            .collect();
        let mut counts: HashMap<usize, usize> = HashMap::new();
        while let Some(node) = ready.pop() {
            let count = paths.get(&node).map_or(0, |p| p.0);
            *counts.entry(nodes[node].1).or_default() += count;
            if let Some(parent) = paths.get_mut(&nodes[node].0) {
                parent.0 += count;
                parent.1 -= 1;
                if parent.1 == 0 {
                    ready.push(nodes[node].0);
                }
            }
        }
        counts
            .into_iter()
            .filter_map(|(id, n)| (n == self.arrivals.len()).then_some(id))
            .collect()
    }
}
pub(super) struct LoopFlow {
    pub(super) exit: Join,
    pub(super) update: Join,
}
impl Checker<'_> {
    pub(super) fn is_wide(&self, source: &Expression<'_>, value: &Expr) -> bool {
        match source {
            Expression::CallExpression(_) => true,
            Expression::BinaryExpression(b) => {
                matches!(b.operator.as_str(), "+" | "-" | "*" | "/" | "%")
            }
            Expression::UnaryExpression(u) if u.operator.as_str() == "-" => {
                let mut argument = &u.argument;
                while let Expression::ParenthesizedExpression(p) = argument {
                    argument = &p.expression;
                }
                !matches!(argument, Expression::NumericLiteral(_))
            }
            Expression::Identifier(_) => match value.kind {
                E::Read(id) => self.equality.bindings[id] && !self.equality.tested[id],
                _ => false,
            },
            _ => false,
        }
    }
    pub(super) fn equality_operands(
        &self,
        span: Span,
        a: &Expr,
        b: &Expr,
    ) -> Result<(), Diagnostic> {
        if a.ty != b.ty || matches!(a.ty, Type::Function(..)) {
            return Err(self.error(span, "C7", "Equality requires identical value types"));
        }
        if a.ty == Type::Boolean {
            return Err(self.error(span, "C7", "Boolean equality operands are rejected"));
        }
        if [a, b]
            .iter()
            .any(|e| matches!(e.kind, E::Number(n) if n.is_nan()))
        {
            return Err(self.error(span, "C7", "NaN equality operands are rejected"));
        }
        if !a.wide && !b.wide {
            return Err(self.error(span, "C7", "An equality needs a wide operand"));
        }
        Ok(())
    }
    pub(super) fn edge_expr(
        &mut self,
        e: &Expression<'_>,
        depth: u32,
        ids: &[usize],
        join: &mut Join,
    ) -> Result<Expr, Diagnostic> {
        let checkpoint = self.equality.checkpoint();
        self.equality.enter(ids);
        let out = self.expr(e, depth, false);
        join.edge(&self.equality, self.reachable);
        self.equality.restore(checkpoint);
        out
    }
}
