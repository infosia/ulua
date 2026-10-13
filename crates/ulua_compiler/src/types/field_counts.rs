//! Counts visit direct type edges once. Alias references end the count; shared edges count at each position.
use super::{Node, Type, Types};
use crate::FIELD_BOUND;
impl Types {
    // Use the identity only when both budgets retain the same contribution.
    pub(super) fn count_identity(&self, ty: Type) -> Type {
        if self.nested_field_count(ty) == 0 && self.depth(ty) == self.depth(self.identity(ty)) {
            self.identity(ty)
        } else {
            ty
        }
    }
    pub(crate) fn field_count(&self, ty: Type) -> usize {
        self.field_counts[ty.0]
    }
    pub(crate) fn nested_field_count(&self, ty: Type) -> usize {
        if self.named_records.contains(&ty) {
            0
        } else {
            self.field_count(ty)
        }
    }
    pub(super) fn count_node(&self, node: &Node) -> usize {
        let add = |count: usize, ty: Type| {
            count
                .saturating_add(self.nested_field_count(ty))
                .min(FIELD_BOUND + 1)
        };
        match node {
            Node::Primitive(_) => 0,
            Node::Array(child) | Node::Nullable(child) | Node::Promise(child) => {
                self.nested_field_count(*child)
            }
            Node::Function(f) => f
                .params
                .iter()
                .fold(self.nested_field_count(f.result), |count, ty| {
                    add(count, *ty)
                }),
            Node::Record(fields) => fields
                .iter()
                .fold(fields.len().min(FIELD_BOUND + 1), |count, (_, ty)| {
                    add(count, *ty)
                }),
            Node::Class(class) => class.fields.iter().chain(&class.methods).fold(
                (class.fields.len() + class.methods.len()).min(FIELD_BOUND + 1),
                |count, (_, ty)| add(count, *ty),
            ),
        }
    }
    pub(super) fn add_counted(&mut self, node: Node, identity: Option<Type>) -> Type {
        let count = self.count_node(&node);
        let ty = Type(self.nodes.len());
        self.field_counts.push(count);
        self.depths.push(if self.depths_ready {
            self.node_depth(&node)
        } else {
            0
        });
        self.canonical.push(identity.unwrap_or(ty));
        self.nodes.push(node);
        ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_counts_preserve_alias_boundaries_and_saturate_shared_edges() {
        let mut types = Types::new();
        let named = types.reserve_record();
        let fields = (0..600).map(|i| (i, Type::Number)).collect::<Vec<_>>();
        types.define_record(named, fields.clone());
        types.seal_aliases();
        let anonymous = types.record(fields);
        assert!(types.equal(named, anonymous));
        assert_eq!(types.field_count(named), 600);
        assert_eq!(types.nested_field_count(named), 0);
        assert_eq!(types.nested_field_count(anonymous), 600);
        let doubled = types.record(vec![(1000, anonymous), (1001, anonymous)]);
        assert_eq!(types.field_count(doubled), FIELD_BOUND + 1);
        let array = types.array(anonymous);
        let nullable = types.nullable(array);
        let function = types.intern(vec![named], nullable, 1);
        assert_eq!(types.field_count(function), 600);
        assert_eq!(
            types.field_count(types.signature(function).unwrap().result),
            600
        );
        let both = types.intern(vec![anonymous], anonymous, 1);
        assert_eq!(types.field_count(both), FIELD_BOUND + 1);
        let mut chain = types.record(vec![(0, Type::Number)]);
        for _ in 0..64 {
            chain = types.record(vec![(1000, chain), (1001, chain)]);
        }
        assert_eq!(types.field_count(chain), FIELD_BOUND + 1);
        assert_eq!(
            types.record(vec![(1000, chain), (1001, chain)]),
            types.record(vec![(1000, chain), (1001, chain)])
        );
    }
}
