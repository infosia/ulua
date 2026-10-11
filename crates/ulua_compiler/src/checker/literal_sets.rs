//! Shared binary tries avoid full-set copies through const initializer chains.
use std::collections::HashMap;

pub(super) type Set = usize;
const BITS: u32 = 32;

pub(super) struct Sets {
    nodes: Vec<(Set, Set)>,
    identities: HashMap<(Set, Set), Set>,
    unions: HashMap<(Set, Set), Set>,
    overlaps: HashMap<(Set, Set), bool>,
}
impl Sets {
    pub(super) fn new() -> Self {
        Self {
            // Zero is empty. One is the occupied leaf at depth zero.
            nodes: vec![(0, 0), (0, 0)],
            identities: HashMap::new(),
            unions: HashMap::new(),
            overlaps: HashMap::new(),
        }
    }
    fn node(&mut self, left: Set, right: Set) -> Set {
        if left == 0 && right == 0 {
            return 0;
        }
        let next = self.nodes.len();
        let id = *self.identities.entry((left, right)).or_insert(next);
        if id == next {
            self.nodes.push((left, right));
        }
        id
    }
    pub(super) fn singleton(&mut self, literal: u32) -> Set {
        let mut set = 1;
        for bit in 0..BITS {
            set = if literal & (1 << bit) == 0 {
                self.node(set, 0)
            } else {
                self.node(0, set)
            };
        }
        set
    }
    pub(super) fn union(&mut self, a: Set, b: Set) -> Set {
        if a == b || b == 0 {
            return a;
        }
        if a == 0 {
            return b;
        }
        let pair = (a.min(b), a.max(b));
        if let Some(set) = self.unions.get(&pair) {
            return *set;
        }
        let (al, ar) = self.nodes[a];
        let (bl, br) = self.nodes[b];
        let left = self.union(al, bl);
        let right = self.union(ar, br);
        let set = self.node(left, right);
        self.unions.insert(pair, set);
        set
    }
    pub(super) fn overlap(&mut self, a: Set, b: Set) -> bool {
        if a == 0 || b == 0 {
            return false;
        }
        if a == b {
            return true;
        }
        let pair = (a.min(b), a.max(b));
        if let Some(overlap) = self.overlaps.get(&pair) {
            return *overlap;
        }
        let (al, ar) = self.nodes[a];
        let (bl, br) = self.nodes[b];
        let overlap = self.overlap(al, bl) || self.overlap(ar, br);
        self.overlaps.insert(pair, overlap);
        overlap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unions_share_storage_and_compare_literal_identities() {
        let mut sets = Sets::new();
        let mut even = 0;
        let mut odd = 0;
        for literal in 0..4096 {
            let one = sets.singleton(literal);
            if literal % 2 == 0 {
                even = sets.union(even, one);
            } else {
                odd = sets.union(odd, one);
            }
            assert!(!sets.overlap(even, odd));
        }
        let all = sets.union(even, odd);
        assert!(sets.overlap(all, even));
        assert!(sets.overlap(all, odd));
        assert_eq!(sets.union(all, even), all);
        assert_eq!(sets.union(odd, even), all);
        // A full copy per prefix needs millions of entries. Shared paths stay bounded.
        assert!(sets.nodes.len() < 4096 * 3 * BITS as usize);
    }
}
