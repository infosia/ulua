//! Checked P1 and P2.1 forms. Binding identities carry type, scope, and declaration order.
pub(crate) use crate::methods::Method;
pub(crate) use crate::types::{FunctionType, Type, Types};
pub(crate) struct Binding {
    pub(crate) ty: Option<Type>,
    pub(crate) owner: usize,
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) mutable: bool,
    pub(crate) declaration: bool,
}
pub(crate) struct Module {
    pub(crate) methods: std::collections::BTreeSet<(Method, usize)>,
    pub(crate) types: Types,
    pub(crate) bindings: Vec<Binding>,
    pub(crate) functions: Vec<Function>,
    pub(crate) body: Vec<Statement>,
    pub(crate) depth: u32,
    pub(crate) source: Vec<u8>,
    pub(crate) captured: Vec<bool>,
}
pub(crate) struct Function {
    pub(crate) binding: usize,
    pub(crate) name: String,
    pub(crate) exported: bool,
    pub(crate) parameters: Vec<(usize, Option<Expr>)>,
    pub(crate) result: Type,
    pub(crate) body: Vec<Statement>,
}
#[derive(Clone, Copy)]
pub(crate) struct Edge {
    pub(crate) head: usize,
    pub(crate) reachable: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Flow {
    // Logical tests retain child edge endings through parentheses and negation.
    pub(crate) logical: bool,
    pub(crate) yes: Edge,
    pub(crate) no: Edge,
}
pub(crate) struct Expr {
    // The checker caches both condition edges before a parent consumes them.
    pub(crate) flow: Option<Flow>,
    // Return coverage needs the original test syntax after parentheses disappear.
    pub(crate) bare_true: bool,
    pub(crate) start: u32,
    pub(crate) ty: Type,
    pub(crate) kind: ExprKind,
}
pub(crate) enum ExprKind {
    Null,
    Object(Vec<(usize, Expr)>),
    Field(Box<Expr>, usize),
    OptionalField(Box<Expr>, usize),
    OptionalLength(Box<Expr>),
    OptionalMethod(Method, usize, Box<Expr>, Vec<Expr>),
    OptionalPush(Box<Expr>, Box<Expr>),
    OptionalCall(Box<Expr>, usize, Vec<Expr>),
    Coalesce(Box<Expr>, Box<Expr>),
    Number(f64),
    Boolean(bool),
    String(String),
    Read(usize),
    Closure(usize),
    Builtin(&'static str),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Logical(bool, Box<Expr>, Box<Expr>),
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Template(Vec<Expr>),
    Array(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Length(Box<Expr>),
    Push(Box<Expr>, Box<Expr>),
    Method(Method, usize, Box<Expr>, Vec<Expr>),
}
pub(crate) enum Statement {
    Located(u32, Box<Statement>),
    Entry(Vec<usize>),
    Function(usize),
    Declare(usize, Expr),
    Assign(usize, Option<&'static str>, Expr),
    Field(Expr, usize, Option<&'static str>, Expr),
    Element(Expr, Expr, Option<&'static str>, Expr),
    ForOf(usize, Expr, Box<Statement>),
    Eval(Expr),
    Block(Vec<Statement>),
    If(Expr, Box<Statement>, Option<Box<Statement>>),
    While(Expr, Box<Statement>),
    For(
        Vec<Statement>,
        Option<Expr>,
        Option<Box<Statement>>,
        Box<Statement>,
        Vec<usize>,
    ),
    Break,
    Continue,
    Return(Option<Expr>),
}

impl Statement {
    pub(crate) fn form(&self) -> &Self {
        match self {
            Self::Located(_, form) => form,
            _ => self,
        }
    }
}

// Nested loop exits do not exit this loop. The work list avoids another recursive walk.
pub(crate) fn loop_break(body: &Statement) -> bool {
    let mut pending = vec![body];
    while let Some(s) = pending.pop() {
        match s {
            Statement::Located(_, s) => pending.push(s),
            Statement::Break => return true,
            Statement::Block(b) => pending.extend(b),
            Statement::If(_, a, b) => {
                pending.push(a);
                if let Some(b) = b {
                    pending.push(b);
                }
            }
            _ => {}
        }
    }
    false
}

// Read identities prove liveness when a pending join reaches the next statement.
pub(crate) fn reads(statement: &Statement) -> Vec<usize> {
    let mut statements = vec![statement];
    let mut expressions = vec![];
    let mut ids = Vec::new();
    while let Some(s) = statements.pop() {
        match s.form() {
            Statement::Declare(_, e) | Statement::Eval(e) => expressions.push(e),
            Statement::Assign(id, op, e) => {
                if op.is_some() {
                    ids.push(*id);
                }
                expressions.push(e);
            }
            Statement::Field(a, _, _, e) => expressions.extend([a, e]),
            Statement::Element(a, i, _, e) => expressions.extend([a, i, e]),
            Statement::ForOf(_, a, b) => {
                expressions.push(a);
                statements.push(b);
            }
            Statement::Block(b) => statements.extend(b),
            Statement::If(e, a, b) => {
                expressions.push(e);
                statements.push(a);
                statements.extend(b.iter().map(|s| s.as_ref()));
            }
            Statement::While(e, b) => {
                expressions.push(e);
                statements.push(b);
            }
            Statement::For(init, test, update, body, _) => {
                statements.extend(init);
                expressions.extend(test);
                statements.extend(update.iter().map(|s| s.as_ref()));
                statements.push(body);
            }
            Statement::Return(Some(e)) => expressions.push(e),
            _ => {}
        }
    }
    while let Some(e) = expressions.pop() {
        match &e.kind {
            ExprKind::Read(id) => {
                ids.push(*id);
            }
            ExprKind::Field(a, _)
            | ExprKind::OptionalField(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Length(a) => expressions.push(a),
            ExprKind::OptionalPush(a, b)
            | ExprKind::Coalesce(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Logical(_, a, b)
            | ExprKind::Index(a, b)
            | ExprKind::Push(a, b) => {
                expressions.push(a);
                expressions.push(b);
            }
            ExprKind::OptionalMethod(_, _, a, args)
            | ExprKind::OptionalCall(a, _, args)
            | ExprKind::Method(_, _, a, args) => {
                expressions.push(a);
                expressions.extend(args);
            }
            ExprKind::Conditional(t, a, b) => {
                expressions.push(t);
                expressions.push(a);
                expressions.push(b);
            }
            ExprKind::Call(c, args) => {
                expressions.push(c);
                expressions.extend(args);
            }
            ExprKind::Object(fields) => expressions.extend(fields.iter().map(|f| &f.1)),
            ExprKind::Template(parts) | ExprKind::Array(parts) => expressions.extend(parts),
            _ => {}
        }
    }
    ids
}
