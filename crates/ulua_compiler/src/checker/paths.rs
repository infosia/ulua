//! Persistent path heads share undo prefixes. A move visits only divergent tails.
use super::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Narrowing {
    Known(bool),
    // This reference cannot reach this edge. Reads retain its previous type.
    Contradicted(bool),
}
impl Narrowing {
    pub(super) fn value(self) -> bool {
        match self {
            Self::Known(value) | Self::Contradicted(value) => value,
        }
    }
    pub(super) fn test(old: Option<Self>, value: bool) -> Self {
        match old {
            Some(Self::Known(previous)) if previous != value => Self::Contradicted(previous),
            Some(value @ Self::Contradicted(_)) => value,
            _ => Self::Known(value),
        }
    }
    fn meet(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (Some(Self::Contradicted(_)), _) => b,
            (_, Some(Self::Contradicted(_))) => a,
            _ if a == b => a,
            _ => None,
        }
    }
}
type Fact = (Narrowing, u64);
// A join compares consecutive input paths and merges only the changed references.
#[derive(Default)]
pub(super) struct Join {
    previous: Option<usize>,
    merged: HashMap<usize, Option<Narrowing>>,
}
impl Join {
    pub(super) fn include(&mut self, facts: &Facts, head: usize) {
        if let Some(previous) = self.previous {
            for (id, a, b) in facts.differences(previous, head) {
                let value = self.merged.entry(id).or_insert(a);
                *value = Narrowing::meet(*value, b);
            }
        }
        self.previous = Some(head);
    }
    pub(super) fn finish(&self, facts: &mut Facts) -> Option<usize> {
        let head = self.previous?;
        facts.travel(head);
        for (id, value) in &self.merged {
            facts.set(*id, *value);
        }
        Some(facts.head())
    }
}
#[derive(Clone, Copy)]
struct Change {
    id: usize,
    old: Option<Fact>,
    value: Option<Fact>,
    parent: usize,
    depth: usize,
    kills: usize,
}
#[derive(Default)]
pub(super) struct Facts {
    values: HashMap<usize, Fact>,
    log: Vec<usize>,
    nodes: Vec<Change>,
    pub(super) floor: u64,
    pub(super) serial: u64,
}
impl Facts {
    pub(super) fn mark(&self) -> usize {
        self.log.len()
    }
    pub(super) fn head(&self) -> usize {
        self.log.last().copied().unwrap_or(0)
    }
    fn node(&self, head: usize) -> Option<&Change> {
        self.nodes.get(head.checked_sub(1)?)
    }
    fn depth(&self, head: usize) -> usize {
        self.node(head).map_or(0, |n| n.depth)
    }
    fn parent(&self, head: usize) -> usize {
        self.node(head).map_or(0, |n| n.parent)
    }
    fn kills(&self, head: usize) -> usize {
        self.node(head).map_or(0, |n| n.kills)
    }
    pub(super) fn get(&self, id: usize) -> Option<Narrowing> {
        self.values
            .get(&id)
            .filter(|v| v.1 >= self.floor)
            .map(|v| v.0)
    }
    fn put(&mut self, id: usize, value: Option<Fact>) {
        if let Some(value) = value {
            self.values.insert(id, value);
        } else {
            self.values.remove(&id);
        }
    }
    pub(super) fn set(&mut self, id: usize, value: Option<Narrowing>) {
        let old = self.get(id);
        if old == value {
            return;
        }
        self.serial += 1;
        let parent = self.head();
        self.nodes.push(Change {
            id,
            old: self.values.get(&id).copied(),
            value: value.map(|v| (v, self.serial)),
            parent,
            depth: self.log.len() + 1,
            kills: self.kills(parent) + usize::from(old.is_some()),
        });
        self.log.push(self.nodes.len());
        self.put(id, value.map(|v| (v, self.serial)));
    }
    fn undo(&mut self) {
        if let Some(head) = self.log.pop()
            && let Some(node) = self.node(head).copied()
        {
            self.put(node.id, node.old);
        }
    }
    pub(super) fn restore(&mut self, mark: usize) {
        while self.log.len() > mark {
            self.undo();
        }
    }
    pub(super) fn common(&self, mut a: usize, mut b: usize) -> usize {
        while self.depth(a) > self.depth(b) {
            a = self.parent(a);
        }
        while self.depth(b) > self.depth(a) {
            b = self.parent(b);
        }
        while a != b {
            a = self.parent(a);
            b = self.parent(b);
        }
        a
    }
    pub(super) fn extends(&self, ancestor: usize, head: usize) -> bool {
        self.kills(ancestor) == self.kills(head)
    }
    // Each divergent tail supplies its final value and its value before that tail.
    fn differences(
        &self,
        mut a: usize,
        mut b: usize,
    ) -> Vec<(usize, Option<Narrowing>, Option<Narrowing>)> {
        type Tail = HashMap<usize, (Option<Fact>, Option<Fact>)>;
        let mut first: Tail = HashMap::new();
        let mut second: Tail = HashMap::new();
        while a != b {
            let (head, tail) = if self.depth(a) >= self.depth(b) {
                (&mut a, &mut first)
            } else {
                (&mut b, &mut second)
            };
            let Some(node) = self.node(*head) else {
                break;
            };
            tail.entry(node.id)
                .and_modify(|value| value.1 = node.old)
                .or_insert((node.value, node.old));
            *head = node.parent;
        }
        let visible = |value: Option<Fact>| value.filter(|v| v.1 >= self.floor).map(|v| v.0);
        let mut changed = vec![];
        for (id, (value, old)) in &first {
            let other = second.get(id).map_or(*old, |v| v.0);
            if visible(*value) != visible(other) {
                changed.push((*id, visible(*value), visible(other)));
            }
        }
        for (id, (value, old)) in &second {
            if !first.contains_key(id) && visible(*old) != visible(*value) {
                changed.push((*id, visible(*old), visible(*value)));
            }
        }
        changed
    }
    pub(super) fn travel(&mut self, mut target: usize) {
        let mut pending = vec![];
        while self.depth(self.head()) > self.depth(target) {
            self.undo();
        }
        while self.depth(target) > self.depth(self.head()) {
            pending.push(target);
            target = self.parent(target);
        }
        while self.head() != target {
            self.undo();
            pending.push(target);
            target = self.parent(target);
        }
        for head in pending.into_iter().rev() {
            if let Some(node) = self.node(head).copied() {
                self.put(node.id, node.value);
                self.log.push(head);
            }
        }
    }
    pub(super) fn changes(&self, mark: usize) -> HashMap<usize, Option<Narrowing>> {
        self.log[mark..]
            .iter()
            .filter_map(|head| self.node(*head))
            .map(|node| (node.id, self.get(node.id)))
            .collect()
    }
    pub(super) fn merge(
        &mut self,
        a: HashMap<usize, Option<Narrowing>>,
        b: HashMap<usize, Option<Narrowing>>,
    ) {
        let mut keys: std::collections::HashSet<_> = a.keys().copied().collect();
        keys.extend(b.keys().copied());
        for id in keys {
            let old = self.get(id);
            let a = a.get(&id).copied().unwrap_or(old);
            let b = b.get(&id).copied().unwrap_or(old);
            self.set(id, Narrowing::meet(a, b));
        }
    }
    pub(super) fn apply(&mut self, changes: HashMap<usize, Option<Narrowing>>) {
        for (id, value) in changes {
            self.set(id, value);
        }
    }
}
