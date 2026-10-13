//! Persistent path heads share undo prefixes. A move visits only divergent tails.
use super::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Narrowing {
    Known(bool),
    // This reference cannot reach this edge. Reads retain its previous type.
    Contradicted(bool),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn body_floor_hides_mutable_prefixes_but_keeps_fresh_tests_and_const_facts() {
        let mut facts = Facts::default();
        facts.mutable.extend([true, false]);
        facts.owners.extend([1, 1]);
        facts.set(0, Some(Narrowing::Known(true)));
        facts.set(1, Some(Narrowing::Known(true)));
        let mark = facts.mark();
        facts.mutable_floor = facts.serial + 1;
        assert_eq!(facts.get(0).map(Narrowing::value), None);
        assert_eq!(facts.get(1).map(Narrowing::value), Some(true));
        facts.set(0, Some(Narrowing::Known(true)));
        assert_eq!(facts.get(0).map(Narrowing::value), Some(true));
        facts.restore(mark);
        assert_eq!(facts.get(0).map(Narrowing::value), None);
        facts.mutable_floor = 0;
        assert_eq!(facts.get(0).map(Narrowing::value), Some(true));
    }
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
type Fact = (Narrowing, u64, u64);
type MutableFacts = HashMap<u64, HashMap<usize, std::collections::HashSet<usize>>>;
// A join compares consecutive input paths and merges only the changed references.
#[derive(Default)]
pub(super) struct Join {
    previous: Option<usize>,
    merged: HashMap<usize, Option<Narrowing>>,
    // A join belongs to one body, so all deferred suspensions use the same owner.
    suspended_owner: Option<usize>,
}
impl Join {
    pub(super) fn include(&mut self, facts: &mut Facts, head: usize) {
        if self.suspended_owner.is_some()
            && let Some(previous) = self.previous
        {
            // This move shares the cost of the divergent tails between consecutive inputs.
            let saved = facts.head();
            facts.travel(previous);
            self.apply_suspension(facts);
            facts.travel(saved);
        }
        if let Some(previous) = self.previous {
            for (id, a, b) in facts.differences(previous, head) {
                let value = self.merged.entry(id).or_insert(a);
                *value = Narrowing::meet(*value, b);
            }
        }
        self.previous = Some(head);
    }
    pub(super) fn invalidate(&mut self, id: usize) {
        if self.previous.is_some() {
            self.merged.insert(id, None);
        }
    }
    // Repeated finalizers update one marker without a move to an old jump head.
    pub(super) fn end_enclosing(&mut self, owner: usize) {
        if self.previous.is_some() {
            self.suspended_owner = Some(owner);
        }
    }
    fn apply_suspension(&mut self, facts: &Facts) {
        if let Some(owner) = self.suspended_owner.take() {
            for id in facts.enclosing(owner) {
                self.invalidate(id);
            }
        }
    }
    pub(super) fn finish(&mut self, facts: &mut Facts) -> Option<usize> {
        let head = self.previous?;
        facts.travel(head);
        self.apply_suspension(facts);
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
    pub(super) mutable_floor: u64,
    pub(super) mutable: Vec<bool>,
    pub(super) owners: Vec<usize>,
    // Entry floors index separate generations, so an await visits no hidden prefix.
    mutable_facts: MutableFacts,
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
    fn visible(&self, id: usize, value: Option<Fact>) -> Option<Narrowing> {
        let floor = if self.mutable.get(id).copied().unwrap_or(false) {
            self.floor.max(self.mutable_floor)
        } else {
            self.floor
        };
        value.filter(|v| v.1 >= floor).map(|v| v.0)
    }
    pub(super) fn get(&self, id: usize) -> Option<Narrowing> {
        self.visible(id, self.values.get(&id).copied())
    }
    fn put(&mut self, id: usize, value: Option<Fact>) {
        if self.mutable.get(id).copied().unwrap_or(false) {
            let owner = self.owners.get(id).copied().unwrap_or(0);
            if let Some(old) = self.values.get(&id)
                && let Some(generation) = self.mutable_facts.get_mut(&old.2)
            {
                if let Some(facts) = generation.get_mut(&owner) {
                    facts.remove(&id);
                    if facts.is_empty() {
                        generation.remove(&owner);
                    }
                }
                if generation.is_empty() {
                    self.mutable_facts.remove(&old.2);
                }
            }
            if let Some(value) = value {
                self.mutable_facts
                    .entry(value.2)
                    .or_default()
                    .entry(owner)
                    .or_default()
                    .insert(id);
            }
        }
        if let Some(value) = value {
            self.values.insert(id, value);
        } else {
            self.values.remove(&id);
        }
    }
    pub(super) fn enclosing(&self, owner: usize) -> Vec<usize> {
        self.mutable_facts
            .get(&self.floor.max(self.mutable_floor))
            .into_iter()
            .flat_map(|generation| generation.iter())
            .filter(|(key, _)| **key != owner)
            .flat_map(|(_, facts)| facts.iter().copied())
            .collect()
    }
    pub(super) fn end_enclosing(&mut self, owner: usize) {
        for id in self.enclosing(owner) {
            self.set(id, None);
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
            value: value.map(|v| (v, self.serial, self.floor.max(self.mutable_floor))),
            parent,
            depth: self.log.len() + 1,
            kills: self.kills(parent) + usize::from(old.is_some()),
        });
        self.log.push(self.nodes.len());
        self.put(
            id,
            value.map(|v| (v, self.serial, self.floor.max(self.mutable_floor))),
        );
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
        let mut changed = vec![];
        for (id, (value, old)) in &first {
            let other = second.get(id).map_or(*old, |v| v.0);
            if self.visible(*id, *value) != self.visible(*id, other) {
                changed.push((*id, self.visible(*id, *value), self.visible(*id, other)));
            }
        }
        for (id, (value, old)) in &second {
            if !first.contains_key(id) && self.visible(*id, *old) != self.visible(*id, *value) {
                changed.push((*id, self.visible(*id, *old), self.visible(*id, *value)));
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
