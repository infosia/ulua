//! Confinement uses every read and write, including those in closures.
use super::*;
use crate::ir::{Method as M, Type};

pub(super) struct Callback {
    pub parameters: Vec<usize>,
    pub body: usize,
    pub accumulator: Option<usize>,
}
#[derive(Default)]
pub(super) struct Elements {
    pub confined: Vec<bool>,
    pub callbacks: HashMap<usize, Callback>,
    pub returns: HashMap<usize, Vec<usize>>,
    allowed: HashSet<usize>,
    reads: Vec<(usize, usize)>,
    declared: Vec<bool>,
}
impl Elements {
    pub fn callback(&self, args: &[Expr]) -> Option<&Callback> {
        let E::Closure(id) = args.first()?.kind else {
            return None;
        };
        self.callbacks.get(&id)
    }
    // A confined array uses its binding identity for its element fact.
    pub fn receiver(&self, e: &Expr) -> Option<usize> {
        if let E::Read(id) = e.kind {
            self.confined[id].then_some(id)
        } else {
            None
        }
    }
    fn allow(&mut self, e: &Expr) {
        if matches!(e.kind, E::Read(_)) {
            self.allowed.insert(address(e));
        }
    }
    fn array_write(&mut self, id: usize, e: &Expr) {
        match &e.kind {
            E::Array(_) => {}
            E::Method(M::Map | M::Filter, _, _, args) if self.callback(args).is_some() => {
                if let E::Method(M::Map, _, _, _) = e.kind {
                    let body = self.callback(args).map(|c| c.body);
                    if let Some(body) = body {
                        self.returns.entry(body).or_default().push(id);
                    }
                }
            }
            _ => self.confined[id] = false,
        }
    }
    pub fn collect(module: &mut Module) -> Result<Self, Diagnostic> {
        let mut info = Self {
            confined: module
                .bindings
                .iter()
                .map(|b| {
                    b.owner != 0 && b.ty.and_then(|t| module.types.element(t)) == Some(Type::Number)
                })
                .collect(),
            declared: vec![false; module.bindings.len()],
            ..Self::default()
        };
        for f in &module.functions {
            if f.inline_callback.is_some() {
                if f.parameters.is_empty() {
                    return Err(Diagnostic::at(
                        &module.source,
                        0,
                        "internal",
                        "Missing inline callback parameter",
                    ));
                }
                let body = f
                    .body
                    .first()
                    .and_then(|s| match s.form() {
                        S::Return(Some(e)) | S::Eval(e) => Some(address(e)),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        Diagnostic::at(
                            &module.source,
                            0,
                            "internal",
                            "Missing inline callback body",
                        )
                    })?;
                info.callbacks.insert(
                    f.binding,
                    Callback {
                        parameters: f.parameters.iter().map(|(id, _)| *id).collect(),
                        body,
                        accumulator: None,
                    },
                );
            }
        }
        visit_module(module, &mut info)?;
        for &(id, key) in &info.reads {
            if !info.allowed.contains(&key) {
                info.confined[id] = false;
            }
        }
        for (id, confined) in info.confined.iter_mut().enumerate() {
            *confined &= info.declared[id];
        }
        // Discard map destinations that failed confinement after all uses were checked.
        let accumulators: HashSet<_> = info
            .callbacks
            .values()
            .filter_map(|c| c.accumulator)
            .collect();
        for targets in info.returns.values_mut() {
            targets.retain(|id| accumulators.contains(id) || info.confined[*id]);
        }
        Ok(info)
    }
}
impl Visitor for Elements {
    fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
        match &e.kind {
            E::Read(id) => self.reads.push((*id, address(e))),
            E::Index(a, _) | E::Length(a) | E::Push(a, _) | E::Method(M::Pop, _, a, _) => {
                self.allow(a)
            }
            E::Method(M::ForEach | M::Map | M::Filter | M::Reduce, _, a, args)
                if self.callback(args).is_some() =>
            {
                self.allow(a);
                if let E::Method(M::Reduce, _, _, _) = e.kind
                    && let Some(Expr {
                        kind: E::Closure(id),
                        ..
                    }) = args.first()
                    && let Some(callback) = self.callbacks.get_mut(id)
                {
                    callback.accumulator = callback.parameters.first().copied();
                    if let Some(accumulator) = callback.accumulator {
                        self.returns
                            .entry(callback.body)
                            .or_default()
                            .push(accumulator);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
        match s {
            S::Declare(id, e) => {
                self.declared[*id] = true;
                self.array_write(*id, e);
            }
            S::Assign(id, op, e) => {
                if op.is_some() {
                    self.confined[*id] = false;
                } else {
                    self.array_write(*id, e);
                }
            }
            S::Element(a, _, _, _) | S::ForOf(_, a, _) => self.allow(a),
            _ => {}
        }
        Ok(())
    }
}

impl Build<'_> {
    pub(super) fn write_value(&mut self, binding: usize, e: &Expr) -> Result<(), Diagnostic> {
        self.graph.writes.push(Write {
            binding,
            value: self.index(e)?,
            start: self.lookup(&self.starts, e)?,
        });
        Ok(())
    }
    fn write_node(&mut self, binding: usize, node: Node) {
        let value = self.push(node);
        self.graph.writes.push(Write {
            binding,
            value,
            start: value,
        });
    }
    pub(super) fn array_write(&mut self, id: usize, e: &Expr) -> Result<(), Diagnostic> {
        match &e.kind {
            E::Array(entries) => {
                for e in entries {
                    self.write_value(id, e)?;
                }
            }
            E::Method(M::Filter, _, receiver, _) => {
                self.write_node(id, self.element_node(receiver))
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn element_node(&self, receiver: &Expr) -> Node {
        self.elements
            .receiver(receiver)
            .map_or(Node::None, Node::Read)
    }
    pub(super) fn callback_call(&mut self, e: &Expr) -> Result<Option<usize>, Diagnostic> {
        let E::Method(method @ (M::ForEach | M::Map | M::Filter | M::Reduce), _, receiver, args) =
            &e.kind
        else {
            return Ok(None);
        };
        let Some(callback) = self.elements.callback(args) else {
            return Ok(None);
        };
        let reduce = *method == M::Reduce;
        for (index, &id) in callback.parameters.iter().enumerate() {
            self.eligible[id] = true;
            self.graph.bindings.push(id);
            match (reduce, index) {
                (true, 0) => {}
                (false, 0) | (true, 1) => self.write_node(id, self.element_node(receiver)),
                (false, 1) | (true, 2) => self.write_node(id, Node::Range(0, (1 << 27) - 1)),
                _ => self.write_node(id, Node::None),
            }
        }
        if reduce {
            let accumulator = callback.accumulator.ok_or_else(|| {
                Diagnostic::at(
                    self.source,
                    e.start as usize,
                    "internal",
                    "Missing inline reduce accumulator",
                )
            })?;
            let init = args.get(1).ok_or_else(|| {
                Diagnostic::at(
                    self.source,
                    e.start as usize,
                    "internal",
                    "Missing reduce initial value",
                )
            })?;
            self.write_value(accumulator, init)?;
            Ok(Some(accumulator))
        } else {
            Ok(None)
        }
    }
}
