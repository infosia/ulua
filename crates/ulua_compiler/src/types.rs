//! Interned type identities. Signatures store child identities, never child trees.
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Type(usize);

// These names preserve the primitive type vocabulary throughout the compiler.
#[allow(non_upper_case_globals)]
impl Type {
    pub(crate) const Number: Self = Self(0);
    pub(crate) const Boolean: Self = Self(1);
    pub(crate) const String: Self = Self(2);
    pub(crate) const Void: Self = Self(3);
    pub(crate) const StringParts: Self = Self(4);

    pub(crate) fn tag(&self) -> u32 {
        match *self {
            Self::Number => 1,
            Self::Boolean => 2,
            Self::String => 3,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_wide_signatures_share_one_node() {
        let mut types = Types::new();
        let inner = types.intern(vec![Type::Number; 31], Type::Boolean, 31);
        let outer = types.intern(vec![inner; 31], inner, 31);
        for _ in 0..1000 {
            let repeated_inner = types.intern(vec![Type::Number; 31], Type::Boolean, 31);
            assert_eq!(
                types.intern(vec![repeated_inner; 31], repeated_inner, 31),
                outer
            );
        }
        assert_eq!(types.nodes.len(), 7);
        assert_eq!(types.functions.len(), 2);
        assert_eq!(std::mem::size_of::<Type>(), std::mem::size_of::<usize>());
        let Node::Function(stored) = &types.nodes[outer.0] else {
            panic!("function node");
        };
        let (indexed, _) = types.functions.get_key_value(stored.as_ref()).unwrap();
        assert!(Rc::ptr_eq(stored, indexed));
    }

    #[test]
    fn signature_identity_includes_order_result_and_defaults() {
        let mut types = Types::new();
        let base = types.intern(vec![Type::Number, Type::Boolean], Type::String, 2);
        for (params, result, required) in [
            (vec![Type::Boolean, Type::Number], Type::String, 2),
            (vec![Type::Number], Type::String, 1),
            (vec![Type::Number, Type::Boolean], Type::Number, 2),
            (vec![Type::Number, Type::Boolean], Type::String, 1),
        ] {
            assert_ne!(base, types.intern(params, result, required));
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct FunctionType {
    pub(crate) params: Vec<Type>,
    pub(crate) result: Type,
    pub(crate) required: usize,
}

enum Node {
    Primitive(&'static str),
    Function(Rc<FunctionType>),
    Array(Type),
}

pub(crate) struct Types {
    nodes: Vec<Node>,
    // The index and table share one stored signature allocation.
    functions: HashMap<Rc<FunctionType>, Type>,
    arrays: HashMap<Type, Type>,
}

impl Types {
    pub(crate) fn new() -> Self {
        Self {
            nodes: ["number", "boolean", "string", "()", "{string}"]
                .into_iter()
                .map(Node::Primitive)
                .collect(),
            functions: HashMap::new(),
            arrays: HashMap::new(),
        }
    }

    pub(crate) fn intern(&mut self, params: Vec<Type>, result: Type, required: usize) -> Type {
        let signature = FunctionType {
            params,
            result,
            required,
        };
        if let Some(ty) = self.functions.get(&signature) {
            return *ty;
        }
        let ty = Type(self.nodes.len());
        let signature = Rc::new(signature);
        self.nodes.push(Node::Function(Rc::clone(&signature)));
        self.functions.insert(signature, ty);
        ty
    }

    pub(crate) fn array(&mut self, element: Type) -> Type {
        if let Some(ty) = self.arrays.get(&element) {
            return *ty;
        }
        let ty = Type(self.nodes.len());
        self.nodes.push(Node::Array(element));
        self.arrays.insert(element, ty);
        ty
    }

    pub(crate) fn element(&self, ty: Type) -> Option<Type> {
        match self.nodes.get(ty.0) {
            Some(Node::Array(element)) => Some(*element),
            _ => None,
        }
    }

    // The checker bounds the depth of every array chain.
    pub(crate) fn contains_function(&self, mut ty: Type) -> bool {
        while let Some(element) = self.element(ty) {
            ty = element;
        }
        self.function(ty).is_some()
    }

    pub(crate) fn function(&self, ty: Type) -> Option<&FunctionType> {
        match self.nodes.get(ty.0) {
            Some(Node::Function(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn signature(&self, ty: Type) -> Option<Rc<FunctionType>> {
        match self.nodes.get(ty.0) {
            Some(Node::Function(f)) => Some(Rc::clone(f)),
            _ => None,
        }
    }

    pub(crate) fn primitive(&self, ty: Type) -> Option<&'static str> {
        match self.nodes.get(ty.0) {
            Some(Node::Primitive(text)) => Some(text),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn luau(&self, ty: Type) -> String {
        enum Part {
            Ty(Type),
            Text(&'static str),
        }
        let mut pending = vec![Part::Ty(ty)];
        let mut out = String::new();
        while let Some(part) = pending.pop() {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Ty(ty) => {
                    if let Some(text) = self.primitive(ty) {
                        out.push_str(text);
                    } else if let Some(element) = self.element(ty) {
                        pending.push(Part::Text("}"));
                        pending.push(Part::Ty(element));
                        pending.push(Part::Text("{"));
                    } else if let Some(f) = self.function(ty) {
                        pending.push(Part::Ty(f.result));
                        pending.push(Part::Text(") -> "));
                        for (i, ty) in f.params.iter().enumerate().rev() {
                            let grouped = i >= f.required && self.function(*ty).is_some();
                            if i >= f.required {
                                pending.push(Part::Text("?"));
                            }
                            if grouped {
                                pending.push(Part::Text(")"));
                            }
                            pending.push(Part::Ty(*ty));
                            if grouped {
                                pending.push(Part::Text("("));
                            }
                            if i > 0 {
                                pending.push(Part::Text(", "));
                            }
                        }
                        pending.push(Part::Text("("));
                    }
                }
            }
        }
        out
    }
}
