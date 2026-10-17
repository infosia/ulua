//! Structural equality ignores order; JSON source views retain ordered field graphs.
use super::{Node, Type, Types};
use std::collections::HashSet;
impl Types {
    pub(crate) fn same_json_order(&self, a: Type, b: Type) -> bool {
        if a == b {
            return true;
        }
        if self.fields(a).is_some_and(|f| f.is_empty()) && self.equal(a, b) {
            return true;
        }
        if let Some(&same) = self.json_order_cache.borrow().get(&(a, b)) {
            return same;
        }
        let mut seen = HashSet::new();
        let mut pending = vec![(a, b)];
        while let Some((a, b)) = pending.pop() {
            if a == b || !seen.insert((a, b)) {
                continue;
            }
            if !self.equal(a, b) {
                return false;
            }
            match (&self.nodes[a.0], &self.nodes[b.0]) {
                (Node::Array(a), Node::Array(b))
                | (Node::Nullable(a), Node::Nullable(b))
                | (Node::Absent(a), Node::Absent(b))
                | (Node::Promise(a), Node::Promise(b)) => pending.push((*a, *b)),
                (Node::Table(a, av), Node::Table(b, bv)) => {
                    pending.push((*a, *b));
                    pending.extend(av.iter().copied().zip(bv.iter().copied()));
                }
                (Node::Function(a), Node::Function(b)) => {
                    pending.extend(a.params.iter().copied().zip(b.params.iter().copied()));
                    pending.push((a.result, b.result));
                }
                _ => {
                    if let (Some(af), Some(bf)) =
                        (self.declaration_fields(a), self.declaration_fields(b))
                    {
                        if af.len() != bf.len()
                            || !af.iter().map(|f| f.0).eq(bf.iter().map(|f| f.0))
                        {
                            self.json_order_cache.borrow_mut().insert((a, b), false);
                            return false;
                        }
                        pending.extend(af.iter().zip(bf).map(|(a, b)| (a.1, b.1)));
                    }
                }
            }
        }
        let mut cache = self.json_order_cache.borrow_mut();
        for pair in seen {
            cache.insert(pair, true);
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_source_views_survive_array_nullable_and_function_interning() {
        let mut types = Types::new();
        let a = types.reserve_record();
        let b = types.reserve_record();
        types.define_record(a, vec![(0, Type::Number), (1, Type::String)]);
        types.define_record(b, vec![(1, Type::String), (0, Type::Number)]);
        types.seal_aliases();
        assert!(types.equal(a, b));
        assert!(!types.same_json_order(a, b));
        for (a, b) in [
            (types.array(a), types.array(b)),
            (types.nullable(a), types.nullable(b)),
            (types.intern(vec![a], a, 1), types.intern(vec![b], b, 1)),
        ] {
            assert_ne!(a, b);
            assert!(types.equal(a, b));
            assert!(!types.same_json_order(a, b));
        }
    }
}
