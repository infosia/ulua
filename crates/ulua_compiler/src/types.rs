//! Interned type identities. Signatures store child identities, never child trees.
mod depths;
mod field_counts;
mod json;
mod links;
mod partition;
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
    pub(crate) const Null: Self = Self(5);
    pub(crate) const Error: Self = Self(6);
    pub(crate) const Unknown: Self = Self(7);

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
    fn promise_containment_follows_value_fields_only() {
        let mut types = Types::new();
        let number_promise = types.promise(Type::Number);
        let plain = types.reserve_record();
        types.define_record(plain, vec![(0, plain), (1, Type::Number)]);
        let first = types.reserve_record();
        let second = types.reserve_record();
        types.define_record(first, vec![(0, second)]);
        types.define_record(second, vec![(0, first), (1, number_promise)]);
        let callback = types.intern(vec![plain], number_promise, 1);
        let parameter = types.intern(vec![number_promise], Type::Void, 1);
        let class = types.reserve_class();
        types.define_class(class, vec![], vec![(0, callback)]);
        let field_class = types.reserve_class();
        types.define_class(field_class, vec![(0, second)], vec![(1, callback)]);
        let callable_class = types.reserve_class();
        types.define_class(callable_class, vec![(0, parameter)], vec![(1, callback)]);
        let callable_array = types.array(callback);
        let callable_nullable = types.nullable(callable_array);
        let callable_record = types.record(vec![(0, callable_nullable), (1, class)]);
        let field_array = types.array(field_class);
        let field_nullable = types.nullable(field_array);
        let field_record = types.record(vec![(0, field_nullable)]);
        // Check traversal before the seal, then the reverse-reachability cache after it.
        for sealed in [false, true] {
            if sealed {
                types.seal_aliases();
            }
            for ty in [
                plain,
                callback,
                parameter,
                class,
                callable_class,
                callable_array,
                callable_nullable,
                callable_record,
            ] {
                assert!(!types.contains_promise(ty), "sealed={sealed}, type={ty:?}");
            }
            for ty in [first, second, field_class, field_record, number_promise] {
                assert!(types.contains_promise(ty));
            }
        }
        let array = types.array(first);
        let nullable = types.nullable(array);
        let record = types.record(vec![(0, nullable)]);
        assert!(types.contains_promise(record));
        let plain_array = types.array(plain);
        assert!(!types.contains_promise(plain_array));
    }

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
        assert_eq!(types.nodes.len(), 10);
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
    Promise(Type),
    Table(Type, Option<Type>),
    Nullable(Type),
    Absent(Type),
    Record(Rc<Vec<(usize, Type)>>),
    Class(Rc<ClassType>),
}

struct ClassType {
    fields: Vec<(usize, Type)>,
    methods: Vec<(usize, Type)>,
}

pub(crate) struct Types {
    json_order_cache: std::cell::RefCell<HashMap<(Type, Type), bool>>,
    declaration_fields: HashMap<Type, Vec<(usize, Type)>>,
    tables: HashMap<(Type, Option<Type>), Type>,
    nodes: Vec<Node>,
    canonical: Vec<Type>,
    // Identity indexes use canonical child ids after the alias seal.
    functions: HashMap<Rc<FunctionType>, Type>,
    // Source views retain the depth and field-count contributions of their children.
    function_views: HashMap<Rc<FunctionType>, Type>,
    array_views: HashMap<Type, Type>,
    nullable_views: HashMap<Type, Type>,
    arrays: HashMap<Type, Type>,
    promises: HashMap<Type, Type>,
    promise_views: HashMap<Type, Type>,
    nullable: HashMap<Type, Type>,
    absent: HashMap<Type, Type>,
    records: HashMap<Rc<Vec<(usize, Type)>>, Type>,
    anonymous_records: HashMap<Rc<Vec<(usize, Type)>>, Type>,
    named_records: std::collections::HashSet<Type>,
    field_counts: Vec<usize>,
    depths: Vec<usize>,
    depths_ready: bool,
    function_content: std::cell::RefCell<HashMap<Type, bool>>,
    promise_content: std::cell::RefCell<HashMap<Type, bool>>,
}

impl Types {
    pub(crate) fn new() -> Self {
        Self {
            json_order_cache: Default::default(),
            declaration_fields: HashMap::new(),
            tables: HashMap::new(),
            nodes: [
                "number",
                "boolean",
                "string",
                "()",
                "{string}",
                "nil",
                "__ulua_Error",
                "unknown",
            ]
            .into_iter()
            .map(Node::Primitive)
            .collect(),
            functions: HashMap::new(),
            function_views: HashMap::new(),
            array_views: HashMap::new(),
            nullable_views: HashMap::new(),
            arrays: HashMap::new(),
            promises: HashMap::new(),
            promise_views: HashMap::new(),
            nullable: HashMap::new(),
            absent: HashMap::new(),
            records: HashMap::new(),
            anonymous_records: HashMap::new(),
            named_records: Default::default(),
            field_counts: vec![0; 8],
            depths: vec![0; 8],
            depths_ready: false,
            canonical: (0..8).map(Type).collect(),
            function_content: Default::default(),
            promise_content: Default::default(),
        }
    }

    pub(crate) fn table(&mut self, key: Type, value: Option<Type>) -> Type {
        let pair = (
            self.count_identity(key),
            value.map(|t| self.count_identity(t)),
        );
        if let Some(ty) = self.tables.get(&pair) {
            return *ty;
        }
        let canonical = (self.identity(key), value.map(|t| self.identity(t)));
        let identity = if let Some(ty) = self.tables.get(&canonical) {
            *ty
        } else {
            let ty = self.add_counted(Node::Table(canonical.0, canonical.1), None);
            self.tables.insert(canonical, ty);
            ty
        };
        let ty = if pair == canonical {
            identity
        } else {
            self.add_counted(Node::Table(pair.0, pair.1), Some(identity))
        };
        self.tables.insert(pair, ty);
        ty
    }
    pub(crate) fn table_parts(&self, ty: Type) -> Option<(Type, Option<Type>)> {
        match self.nodes.get(ty.0) {
            Some(Node::Table(k, v)) => Some((*k, *v)),
            _ => None,
        }
    }
    pub(crate) fn promise_value(&self, ty: Type) -> Option<Type> {
        match self.nodes.get(ty.0) {
            Some(Node::Promise(value)) => Some(*value),
            _ => None,
        }
    }

    // Follow stored values through containers. Function signatures and methods hold no promise value.
    pub(crate) fn contains_promise(&self, ty: Type) -> bool {
        if let Some(answer) = self.promise_content.borrow().get(&ty) {
            return *answer;
        }
        let root = ty;
        let mut pending = vec![ty];
        let mut seen = std::collections::HashSet::new();
        while let Some(ty) = pending.pop() {
            if !seen.insert(ty) {
                continue;
            }
            let cached = self.promise_content.borrow().get(&ty).copied();
            if cached == Some(true) || self.promise_value(ty).is_some() {
                self.promise_content.borrow_mut().insert(root, true);
                return true;
            }
            if cached == Some(false) {
                continue;
            }
            match self.nodes.get(ty.0) {
                Some(Node::Array(t) | Node::Nullable(t) | Node::Absent(t)) => pending.push(*t),
                Some(Node::Table(k, v)) => {
                    pending.push(*k);
                    pending.extend(*v);
                }
                Some(Node::Record(fields)) => pending.extend(fields.iter().map(|f| f.1)),
                Some(Node::Class(class)) => {
                    pending.extend(class.fields.iter().map(|f| f.1));
                }
                _ => {}
            }
        }
        let mut cache = self.promise_content.borrow_mut();
        for ty in seen {
            cache.insert(ty, false);
        }
        false
    }

    pub(crate) fn element(&self, ty: Type) -> Option<Type> {
        match self.nodes.get(ty.0) {
            Some(Node::Array(element)) => Some(*element),
            _ => None,
        }
    }

    pub(crate) fn contains_null(&self, ty: Type) -> bool {
        ty == Type::Null || self.inner(ty).is_some()
    }
    pub(crate) fn inner(&self, ty: Type) -> Option<Type> {
        match self.nodes.get(ty.0) {
            Some(Node::Nullable(inner) | Node::Absent(inner)) => Some(*inner),
            _ => None,
        }
    }
    pub(crate) fn is_absent(&self, ty: Type) -> bool {
        matches!(self.nodes.get(ty.0), Some(Node::Absent(_)))
    }
    pub(crate) fn absent(&mut self, inner: Type) -> Type {
        let inner = self.count_identity(inner);
        if let Some(ty) = self.absent.get(&inner) {
            return *ty;
        }
        let ty = self.add_counted(Node::Absent(inner), None);
        self.absent.insert(inner, ty);
        ty
    }
    pub(crate) fn reserve_record(&mut self) -> Type {
        let ty = Type(self.nodes.len());
        self.canonical.push(ty);
        self.nodes.push(Node::Record(Rc::new(vec![])));
        self.field_counts.push(0);
        self.depths.push(0);
        self.named_records.insert(ty);
        ty
    }
    pub(crate) fn define_record(&mut self, ty: Type, mut fields: Vec<(usize, Type)>) {
        if fields.windows(2).any(|p| p[0].0 > p[1].0) {
            self.declaration_fields.insert(ty, fields.clone());
        } else {
            self.declaration_fields.remove(&ty);
        }
        fields.sort_unstable_by_key(|f| f.0);
        let node = Node::Record(Rc::new(fields));
        self.field_counts[ty.0] = self.count_node(&node);
        self.nodes[ty.0] = node;
    }
    pub(crate) fn record(&mut self, mut fields: Vec<(usize, Type)>) -> Type {
        let declaration = fields.clone();
        for (_, ty) in &mut fields {
            *ty = self.count_identity(*ty);
        }
        fields.sort_unstable_by_key(|f| f.0);
        if let Some(ty) = self.anonymous_records.get(&fields) {
            return *ty;
        }
        let canonical: Vec<_> = fields
            .iter()
            .map(|(key, ty)| (*key, self.identity(*ty)))
            .collect();
        let identity = if let Some(ty) = self.records.get(&canonical) {
            *ty
        } else {
            let canonical = Rc::new(canonical);
            let ty = self.add_counted(Node::Record(Rc::clone(&canonical)), None);
            self.records.insert(canonical, ty);
            ty
        };
        let separate = self.named_records.contains(&identity)
            || self.fields(identity) != Some(fields.as_slice());
        let fields = Rc::new(fields);
        let ty = if separate {
            self.add_counted(Node::Record(Rc::clone(&fields)), Some(identity))
        } else {
            identity
        };
        self.anonymous_records.insert(fields, ty);
        if declaration.windows(2).any(|p| p[0].0 > p[1].0) {
            self.declaration_fields.entry(ty).or_insert(declaration);
        }
        ty
    }
    pub(crate) fn declaration_fields(&self, ty: Type) -> Option<&[(usize, Type)]> {
        self.declaration_fields
            .get(&ty)
            .map(Vec::as_slice)
            .or_else(|| self.fields(ty))
    }
    pub(crate) fn fields(&self, ty: Type) -> Option<&[(usize, Type)]> {
        match self.nodes.get(ty.0) {
            Some(Node::Record(fields)) => Some(fields),
            Some(Node::Class(class)) => Some(&class.fields),
            _ => None,
        }
    }
    pub(crate) fn reserve_class(&mut self) -> Type {
        let ty = self.add_counted(
            Node::Class(Rc::new(ClassType {
                fields: vec![],
                methods: vec![],
            })),
            None,
        );
        self.named_records.insert(ty);
        ty
    }
    pub(crate) fn define_class(
        &mut self,
        ty: Type,
        mut fields: Vec<(usize, Type)>,
        methods: Vec<(usize, Type)>,
    ) {
        if fields.windows(2).any(|p| p[0].0 > p[1].0) {
            self.declaration_fields.insert(ty, fields.clone());
        } else {
            self.declaration_fields.remove(&ty);
        }
        fields.sort_unstable_by_key(|f| f.0);
        let node = Node::Class(Rc::new(ClassType { fields, methods }));
        self.field_counts[ty.0] = self.count_node(&node);
        self.nodes[ty.0] = node;
    }
    pub(crate) fn is_class(&self, ty: Type) -> bool {
        matches!(self.nodes.get(ty.0), Some(Node::Class(_)))
    }
    pub(crate) fn has_member(&self, ty: Type, key: usize) -> bool {
        self.field(ty, key).is_some()
            || matches!(self.nodes.get(ty.0), Some(Node::Class(class))
                if class.methods.iter().any(|(name, _)| *name == key))
    }
    pub(crate) fn field(&self, ty: Type, key: usize) -> Option<Type> {
        let fields = self.fields(ty)?;
        fields
            .binary_search_by_key(&key, |f| f.0)
            .ok()
            .map(|i| fields[i].1)
    }
    pub(crate) fn identity(&self, ty: Type) -> Type {
        self.canonical[ty.0]
    }
    pub(crate) fn identities(&self) -> impl Iterator<Item = (Type, Type)> + '_ {
        self.canonical
            .iter()
            .enumerate()
            .map(|(id, identity)| (Type(id), *identity))
    }
    pub(crate) fn equal(&self, a: Type, b: Type) -> bool {
        self.canonical[a.0] == self.canonical[b.0]
    }
    pub(crate) fn assignable(&self, from: Type, to: Type) -> bool {
        self.equal(from, to)
            || (self.is_absent(from)
                && self.is_absent(to)
                && self.equal(self.inner(from).unwrap(), self.inner(to).unwrap()))
            || self.inner(to).is_some_and(|inner| {
                (!self.is_absent(to) && from == Type::Null) || self.equal(from, inner)
            })
    }
    pub(crate) fn join(&mut self, a: Type, b: Type) -> Option<Type> {
        if self.assignable(a, b) {
            Some(b)
        } else if self.assignable(b, a) {
            Some(a)
        } else if a == Type::Null {
            Some(self.nullable(b))
        } else if b == Type::Null {
            Some(self.nullable(a))
        } else {
            None
        }
    }

    // The checker bounds the depth of every array chain.
    pub(crate) fn contains_function(&self, ty: Type) -> bool {
        if let Some(answer) = self.function_content.borrow().get(&ty) {
            return *answer;
        }
        let root = ty;
        let mut pending = vec![ty];
        let mut seen = std::collections::HashSet::new();
        while let Some(ty) = pending.pop() {
            if !seen.insert(ty) {
                continue;
            }
            if self.function(ty).is_some() {
                self.function_content.borrow_mut().insert(root, true);
                return true;
            }
            if let Some(t) = self
                .element(ty)
                .or_else(|| self.inner(ty))
                .or_else(|| self.promise_value(ty))
            {
                pending.push(t);
            }
            if let Some((k, v)) = self.table_parts(ty) {
                pending.push(k);
                pending.extend(v);
            }
            if let Some(fields) = self.fields(ty) {
                pending.extend(fields.iter().map(|f| f.1));
            }
        }
        let mut cache = self.function_content.borrow_mut();
        for ty in seen {
            cache.insert(ty, false);
        }
        false
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
                    } else if let Some(Node::Nullable(inner) | Node::Absent(inner)) =
                        self.nodes.get(ty.0)
                    {
                        pending.push(Part::Text("?"));
                        pending.push(Part::Ty(*inner));
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
