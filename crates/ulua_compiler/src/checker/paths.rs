//! Persistent path heads share undo prefixes. A move visits only divergent tails.
use super::*;
type Fact = (bool, u64);
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
    pub(super) fn get(&self, id: usize) -> Option<bool> {
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
    pub(super) fn set(&mut self, id: usize, value: Option<bool>) {
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
    pub(super) fn changes(&self, mark: usize) -> HashMap<usize, Option<bool>> {
        self.log[mark..]
            .iter()
            .filter_map(|head| self.node(*head))
            .map(|node| (node.id, self.get(node.id)))
            .collect()
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
