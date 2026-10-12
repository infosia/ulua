//! Canonical child keys identify composites. Source views retain each child's budget contribution.
use super::{FunctionType, Node, Type, Types};
use std::rc::Rc;
impl Types {
    pub(crate) fn intern(&mut self, params: Vec<Type>, result: Type, required: usize) -> Type {
        let signature = FunctionType {
            params: params.into_iter().map(|t| self.count_identity(t)).collect(),
            result: self.count_identity(result),
            required,
        };
        if let Some(ty) = self.function_views.get(&signature) {
            return *ty;
        }
        let canonical_params = signature.params.iter().map(|t| self.identity(*t)).collect();
        let canonical_result = self.identity(signature.result);
        let signature = Rc::new(signature);
        let canonical =
            if canonical_params == signature.params && canonical_result == signature.result {
                Rc::clone(&signature)
            } else {
                Rc::new(FunctionType {
                    params: canonical_params,
                    result: canonical_result,
                    required,
                })
            };
        let identity = if let Some(ty) = self.functions.get(&canonical) {
            *ty
        } else {
            let ty = self.add_counted(Node::Function(Rc::clone(&canonical)), None);
            self.functions.insert(canonical, ty);
            ty
        };
        let ty = if self.function(identity) == Some(signature.as_ref()) {
            identity
        } else {
            self.add_counted(Node::Function(Rc::clone(&signature)), Some(identity))
        };
        self.function_views.insert(signature, ty);
        ty
    }
    pub(crate) fn array(&mut self, element: Type) -> Type {
        let element = self.count_identity(element);
        if let Some(ty) = self.array_views.get(&element) {
            return *ty;
        }
        let canonical = self.identity(element);
        let identity = if let Some(ty) = self.arrays.get(&canonical) {
            *ty
        } else {
            let ty = self.add_counted(Node::Array(canonical), None);
            self.arrays.insert(canonical, ty);
            ty
        };
        let ty = if self.element(identity) == Some(element) {
            identity
        } else {
            self.add_counted(Node::Array(element), Some(identity))
        };
        self.array_views.insert(element, ty);
        ty
    }
    pub(crate) fn nullable(&mut self, inner: Type) -> Type {
        let inner = self.count_identity(inner);
        if inner == Type::Null || self.inner(inner).is_some() {
            return inner;
        }
        if let Some(ty) = self.nullable_views.get(&inner) {
            return *ty;
        }
        let canonical = self.identity(inner);
        let identity = if let Some(ty) = self.nullable.get(&canonical) {
            *ty
        } else {
            let ty = self.add_counted(Node::Nullable(canonical), None);
            self.nullable.insert(canonical, ty);
            ty
        };
        let ty = if self.inner(identity) == Some(inner) {
            identity
        } else {
            self.add_counted(Node::Nullable(inner), Some(identity))
        };
        self.nullable_views.insert(inner, ty);
        ty
    }
}

#[cfg(test)]
mod tests {
    //! Composite cache regression: measured Debug cost 0.029 seconds. The test prints its cost.
    use super::*;
    use std::time::Instant;

    fn links(types: &mut Types, a: Type, b: Type) -> [Type; 8] {
        let array = types.array(a);
        let grid = types.array(array);
        let nullable = types.nullable(a);
        let nullable_array = types.nullable(grid);
        let function = types.intern(vec![a], b, 1);
        let parameters = types.intern(vec![a, b], Type::Number, 2);
        let nested = types.intern(vec![function], function, 1);
        let record = types.record(vec![(1, array), (2, function)]);
        [
            array,
            grid,
            nullable,
            nullable_array,
            function,
            parameters,
            nested,
            record,
        ]
    }

    #[test]
    fn composites_created_after_the_seal_find_the_earlier_identity() {
        let start = Instant::now();
        for reverse in [false, true] {
            let mut types = Types::new();
            let p = types.reserve_record();
            let q = types.reserve_record();
            types.define_record(p, vec![(0, Type::Number)]);
            types.define_record(q, vec![(0, Type::Number)]);
            let (a, b) = if reverse { (p, q) } else { (q, p) };
            let earlier = links(&mut types, a, b);
            types.seal_aliases();
            let later = links(&mut types, b, a);
            for (old, new) in earlier.into_iter().zip(later) {
                assert_eq!(types.identity(old), types.identity(new));
            }
            let nodes = types.nodes.len();
            for _ in 0..1000 {
                assert_eq!(links(&mut types, b, a), later);
            }
            assert_eq!(types.nodes.len(), nodes);
        }
        eprintln!(
            "P2.5 composite cache unit cost={:.3}s",
            start.elapsed().as_secs_f64()
        );
    }
}
