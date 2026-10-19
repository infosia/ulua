//! Existing primitive keys have bounded update cost; construction keeps the store cost.
use crate::{
    STORE_WEIGHT, UPDATE_WEIGHT,
    ir::{Type, Types},
};

pub(super) fn weight(types: &Types, receiver: Type, key: usize, construction: bool) -> usize {
    if !construction
        && matches!(
            types.field(receiver, key),
            Some(Type::Number | Type::Boolean)
        )
    {
        UPDATE_WEIGHT
    } else {
        STORE_WEIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_types_and_construction_choose_the_weight() {
        let mut types = Types::new();
        let nullable = types.nullable(Type::Number);
        let absent = types.absent(Type::Number);
        let array = types.array(Type::Number);
        for class in [false, true] {
            for (field, expected) in [
                (Type::Number, 2),
                (Type::Boolean, 2),
                (nullable, 8),
                (absent, 8),
                (Type::String, 8),
                (array, 8),
            ] {
                let receiver = if class {
                    let ty = types.reserve_class();
                    types.define_class(ty, vec![(0, field)], vec![]);
                    ty
                } else {
                    types.record(vec![(0, field)])
                };
                assert_eq!(
                    weight(&types, receiver, 0, false),
                    expected,
                    "class={class}, field={field:?}"
                );
                assert_eq!(weight(&types, receiver, 0, true), 8);
                assert_eq!(weight(&types, receiver, 1, false), 8);
                let nullable_receiver = types.nullable(receiver);
                assert_eq!(weight(&types, nullable_receiver, 0, false), 8);
            }
        }
        assert_eq!(weight(&types, array, 0, false), 8);
    }
}
