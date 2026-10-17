//! Deterministic type graphs use partition refinement before value flow.
use super::{FunctionType, Node, Type, Types};
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};
#[derive(Hash, PartialEq, Eq)]
enum Shape {
    Primitive(&'static str),
    Array,
    Promise,
    Nullable,
    Absent,
    Function(usize, usize),
    Record(Vec<usize>),
    Class(usize),
}
impl Types {
    pub(crate) fn seal_aliases(&mut self) {
        self.seal_depths();
        let mut shapes = HashMap::<Shape, usize>::new();
        let mut blocks: Vec<Vec<usize>> = vec![];
        let mut classes = vec![0; self.nodes.len()];
        let mut positions = vec![0; self.nodes.len()];
        let mut incoming = vec![vec![]; self.nodes.len()];
        for (id, node) in self.nodes.iter().enumerate() {
            let (shape, children) = match node {
                Node::Primitive(s) => (Shape::Primitive(s), vec![]),
                Node::Array(t) => (Shape::Array, vec![*t]),
                Node::Promise(t) => (Shape::Promise, vec![*t]),
                Node::Nullable(t) => (Shape::Nullable, vec![*t]),
                Node::Absent(t) => (Shape::Absent, vec![*t]),
                Node::Function(f) => {
                    let mut children = f.params.clone();
                    children.push(f.result);
                    (Shape::Function(f.params.len(), f.required), children)
                }
                Node::Record(fields) => (
                    Shape::Record(fields.iter().map(|f| f.0).collect()),
                    fields.iter().map(|f| f.1).collect(),
                ),
                Node::Class(class) => (
                    Shape::Class(id),
                    class
                        .fields
                        .iter()
                        .chain(&class.methods)
                        .map(|f| f.1)
                        .collect(),
                ),
            };
            let next = blocks.len();
            let class = *shapes.entry(shape).or_insert_with(|| {
                blocks.push(vec![]);
                next
            });
            classes[id] = class;
            positions[id] = blocks[class].len();
            blocks[class].push(id);
            for (label, child) in children.iter().enumerate() {
                incoming[child.0].push((id, label));
            }
        }
        let mut pending: VecDeque<_> = (0..blocks.len()).collect();
        let mut queued = vec![true; blocks.len()];
        while let Some(splitter) = pending.pop_front() {
            queued[splitter] = false;
            let mut predecessors = HashMap::<usize, Vec<usize>>::new();
            for target in &blocks[splitter] {
                for (source, label) in &incoming[*target] {
                    predecessors.entry(*label).or_default().push(*source);
                }
            }
            for sources in predecessors.into_values() {
                let mut touched = HashMap::<usize, Vec<usize>>::new();
                for source in sources {
                    touched.entry(classes[source]).or_default().push(source);
                }
                for (class, members) in touched {
                    if members.len() == blocks[class].len() {
                        continue;
                    }
                    let new = blocks.len();
                    blocks.push(vec![]);
                    queued.push(false);
                    // Swap removal visits the touched side, rather than the whole old block.
                    for id in members {
                        let position = positions[id];
                        blocks[class].swap_remove(position);
                        if let Some(moved) = blocks[class].get(position) {
                            positions[*moved] = position;
                        }
                        classes[id] = new;
                        positions[id] = blocks[new].len();
                        blocks[new].push(id);
                    }
                    let schedule = if queued[class] || blocks[new].len() < blocks[class].len() {
                        new
                    } else {
                        class
                    };
                    if !queued[schedule] {
                        queued[schedule] = true;
                        pending.push_back(schedule);
                    }
                }
            }
        }
        for block in blocks {
            if let Some(representative) = block.iter().min() {
                for id in &block {
                    self.canonical[*id] = Type(*representative);
                }
            }
        }
        // Keep source links for cached budgets. Rebuild every identity index with canonical child keys.
        self.records.clear();
        self.arrays.clear();
        self.promises.clear();
        self.nullable.clear();
        self.functions.clear();
        for (id, node) in self.nodes.iter().enumerate() {
            let identity = self.canonical[id];
            match node {
                Node::Array(t) => {
                    self.arrays.insert(self.canonical[t.0], identity);
                }
                Node::Promise(t) => {
                    self.promises.insert(self.canonical[t.0], identity);
                }
                Node::Nullable(t) => {
                    self.nullable.insert(self.canonical[t.0], identity);
                }
                Node::Absent(_) => {}
                Node::Function(f) => {
                    let signature = Rc::new(FunctionType {
                        params: f.params.iter().map(|t| self.canonical[t.0]).collect(),
                        result: self.canonical[f.result.0],
                        required: f.required,
                    });
                    self.functions.insert(signature, identity);
                }
                Node::Record(fields) => {
                    let fields = Rc::new(
                        fields
                            .iter()
                            .map(|(key, t)| (*key, self.canonical[t.0]))
                            .collect::<Vec<_>>(),
                    );
                    self.records.insert(fields, identity);
                }
                Node::Primitive(_) | Node::Class(_) => {}
            }
        }
        // Each containment cache uses reverse reachability through recursive records.
        for promises in [false, true] {
            let mut contains = vec![false; self.nodes.len()];
            let mut pending = VecDeque::new();
            for (id, node) in self.nodes.iter().enumerate() {
                if (promises && matches!(node, Node::Promise(_)))
                    || (!promises && matches!(node, Node::Function(_)))
                {
                    contains[id] = true;
                    pending.push_back(id);
                }
            }
            while let Some(id) = pending.pop_front() {
                for (source, label) in &incoming[id] {
                    // Containment follows fields; promises also stop at function signatures.
                    if matches!(&self.nodes[*source], Node::Class(class) if *label >= class.fields.len())
                        || (promises && matches!(&self.nodes[*source], Node::Function(_)))
                    {
                        continue;
                    }
                    if !contains[*source] {
                        contains[*source] = true;
                        pending.push_back(*source);
                    }
                }
            }
            let cache = if promises {
                &mut self.promise_content
            } else {
                &mut self.function_content
            };
            cache.get_mut().extend(
                contains
                    .into_iter()
                    .enumerate()
                    .map(|(id, value)| (Type(id), value)),
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cycle(types: &mut Types, n: usize, last: Type) -> Type {
        let records: Vec<_> = (0..n).map(|_| types.reserve_record()).collect();
        for (i, record) in records.iter().enumerate() {
            types.define_record(*record, vec![(0, records[(i + 1) % n]), (1, last)]);
        }
        records[0]
    }
    #[test]
    fn cycles_with_different_lengths_share_one_identity() {
        let mut types = Types::new();
        let a = cycle(&mut types, 127, Type::Number);
        let b = cycle(&mut types, 131, Type::Number);
        let c = cycle(&mut types, 127, Type::String);
        types.seal_aliases();
        assert!(types.equal(a, b));
        assert!(!types.equal(a, c));
        assert_eq!(types.array(a), types.array(b));
        assert_eq!(types.nullable(a), types.nullable(b));
    }
    #[test]
    fn one_different_leaf_splits_every_predecessor() {
        let mut types = Types::new();
        let a = cycle(&mut types, 129, Type::Number);
        let b = cycle(&mut types, 129, Type::Number);
        let child = types.field(b, 0).unwrap();
        let next = types.field(child, 0).unwrap();
        types.define_record(child, vec![(0, next), (1, Type::Boolean)]);
        types.seal_aliases();
        assert!(!types.equal(a, b));
        let signature = types.intern(vec![], Type::Void, 0);
        assert!(!types.equal(a, signature));
    }
}
