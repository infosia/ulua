//! Cached type-link depths. Recursive components contribute one step.
use super::{Node, Type, Types};
use crate::TYPE_DEPTH_BOUND;

impl Node {
    fn children(&self) -> Vec<usize> {
        match self {
            Self::Primitive(_) => vec![],
            Self::Array(t) | Self::Nullable(t) => vec![t.0],
            Self::Function(f) => f
                .params
                .iter()
                .chain(std::iter::once(&f.result))
                .map(|t| t.0)
                .collect(),
            Self::Record(fields) => fields.iter().map(|(_, t)| t.0).collect(),
        }
    }
}
impl Types {
    pub(crate) fn depth(&self, ty: Type) -> usize {
        self.depths[ty.0]
    }
    pub(super) fn node_depth(&self, node: &Node) -> usize {
        node.children()
            .into_iter()
            .map(|id| (self.depths[id] + 1).min(TYPE_DEPTH_BOUND + 1))
            .max()
            .unwrap_or(0)
    }
    // Alias definitions are complete here. Both traversals use explicit stacks.
    pub(super) fn seal_depths(&mut self) {
        let edges: Vec<_> = self.nodes.iter().map(Node::children).collect();
        let mut incoming = vec![vec![]; edges.len()];
        for (id, children) in edges.iter().enumerate() {
            for child in children {
                incoming[*child].push(id);
            }
        }
        let mut seen = vec![false; edges.len()];
        let mut order = Vec::with_capacity(edges.len());
        for root in 0..edges.len() {
            if seen[root] {
                continue;
            }
            seen[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((id, next)) = stack.last_mut() {
                if let Some(child) = edges[*id].get(*next) {
                    *next += 1;
                    if !seen[*child] {
                        seen[*child] = true;
                        stack.push((*child, 0));
                    }
                } else {
                    order.push(*id);
                    stack.pop();
                }
            }
        }
        let mut component = vec![usize::MAX; edges.len()];
        let mut groups = vec![];
        for root in order.into_iter().rev() {
            if component[root] != usize::MAX {
                continue;
            }
            let group = groups.len();
            component[root] = group;
            let mut members = vec![];
            let mut stack = vec![root];
            while let Some(id) = stack.pop() {
                members.push(id);
                for parent in &incoming[id] {
                    if component[*parent] == usize::MAX {
                        component[*parent] = group;
                        stack.push(*parent);
                    }
                }
            }
            groups.push(members);
        }
        let mut depths = vec![0; groups.len()];
        // Components are in parent-first order. Every outgoing component already has its depth.
        for group in (0..groups.len()).rev() {
            let mut depth = usize::from(groups[group].len() > 1);
            for id in &groups[group] {
                for child in &edges[*id] {
                    let target = component[*child];
                    depth = depth.max(if target == group {
                        1
                    } else {
                        depths[target] + 1
                    });
                }
            }
            depths[group] = depth.min(TYPE_DEPTH_BOUND + 1);
        }
        self.depths = component.into_iter().map(|group| depths[group]).collect();
        self.depths_ready = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_recursive_types_keep_their_source_depths() {
        for reverse in [false, true] {
            let mut types = Types::new();
            let records: Vec<_> = (0..24).map(|_| types.reserve_record()).collect();
            let order: Vec<_> = if reverse {
                records.iter().rev().copied().collect()
            } else {
                records.clone()
            };
            let cycle = order[0];
            let nullable = types.nullable(cycle);
            types.define_record(cycle, vec![(0, nullable)]);
            let mut root = cycle;
            for parent in &order[1..] {
                let nullable = types.nullable(root);
                types.define_record(*parent, vec![(0, nullable)]);
                root = *parent;
            }
            types.seal_aliases();
            assert!(types.equal(root, cycle));
            assert_eq!(types.depth(root), 46);
            assert_eq!(types.depth(cycle), 1);
            let nullable = types.nullable(root);
            assert_eq!(types.depth(nullable), 47);
            let parameter = types.array(nullable);
            assert_eq!(types.depth(parameter), 48);
            let function = types.intern(vec![parameter], Type::Void, 1);
            assert_eq!(types.depth(function), 49);
            let array = types.array(cycle);
            assert_eq!(types.depth(array), 2);
            assert_eq!(
                types.depth(types.signature(function).unwrap().params[0]),
                48
            );
        }
    }
    #[test]
    fn recursive_components_and_shared_branches_have_cached_depths() {
        let mut types = Types::new();
        let ring: Vec<_> = (0..100).map(|_| types.reserve_record()).collect();
        for i in 0..ring.len() {
            let next = types.nullable(ring[(i + 1) % ring.len()]);
            types.define_record(ring[i], vec![(0, next)]);
        }
        let mut root = ring[0];
        for _ in 0..47 {
            let parent = types.reserve_record();
            types.define_record(parent, vec![(1, root), (2, root)]);
            root = parent;
        }
        types.seal_aliases();
        assert_eq!(types.depth(ring[0]), 1);
        assert_eq!(types.depth(root), 48);
        assert_eq!(types.depth(Type::Number), 0);
        let mut root = types.identity(root);
        for _ in 0..1000 {
            root = types.record(vec![(3, root), (4, root)]);
            assert_eq!(types.depth(root), 49);
        }
        let array = types.array(root);
        let nullable = types.nullable(array);
        let function = types.intern(vec![nullable], Type::Void, 1);
        assert_eq!(types.depth(function), 49);
    }
}
