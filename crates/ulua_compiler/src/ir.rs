//! Checked P1 and P2.1 forms. Binding identities carry type, scope, and declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Number,
    Boolean,
    String,
    Void,
    StringParts,
    Function(Vec<Type>, Box<Type>, usize),
}
impl Type {
    pub(crate) fn luau(&self) -> String {
        enum Part<'a> {
            Ty(&'a Type),
            Text(String),
        }
        let mut pending = vec![Part::Ty(self)];
        let mut out = String::new();
        while let Some(part) = pending.pop() {
            match part {
                Part::Text(text) => out.push_str(&text),
                Part::Ty(ty) => match ty {
                    Self::Number => out.push_str("number"),
                    Self::Boolean => out.push_str("boolean"),
                    Self::String => out.push_str("string"),
                    Self::Void => out.push_str("()"),
                    Self::StringParts => out.push_str("{string}"),
                    Self::Function(params, result, required) => {
                        pending.push(Part::Ty(result));
                        pending.push(Part::Text(") -> ".into()));
                        for (i, t) in params.iter().enumerate().rev() {
                            if i >= *required {
                                pending.push(Part::Text("?".into()));
                            }
                            let grouped = i >= *required && matches!(t, Self::Function(..));
                            if grouped {
                                pending.push(Part::Text(")".into()));
                            }
                            pending.push(Part::Ty(t));
                            if grouped {
                                pending.push(Part::Text("(".into()));
                            }
                            if i > 0 {
                                pending.push(Part::Text(", ".into()));
                            }
                        }
                        pending.push(Part::Text("(".into()));
                    }
                },
            }
        }
        out
    }
    pub(crate) fn tag(&self) -> u32 {
        match self {
            Self::Number => 1,
            Self::Boolean => 2,
            Self::String => 3,
            _ => 0,
        }
    }
}
pub(crate) struct Binding {
    pub(crate) ty: Option<Type>,
    pub(crate) owner: usize,
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) mutable: bool,
    pub(crate) declaration: bool,
}
pub(crate) struct Module {
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
pub(crate) struct Expr {
    // Structural width belongs to this node, even when parent and child share a source offset.
    pub(crate) wide: bool,
    // Return coverage needs the original test syntax after parentheses disappear.
    pub(crate) bare_true: bool,
    pub(crate) start: u32,
    pub(crate) ty: Type,
    pub(crate) kind: ExprKind,
}
pub(crate) enum ExprKind {
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
}
pub(crate) enum Statement {
    Located(u32, Box<Statement>),
    Entry(Vec<usize>),
    Function(usize),
    Declare(usize, Expr),
    Assign(usize, Option<&'static str>, Expr),
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
            ExprKind::Unary(_, a) => expressions.push(a),
            ExprKind::Binary(_, a, b) | ExprKind::Logical(_, a, b) => {
                expressions.push(a);
                expressions.push(b);
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
            ExprKind::Template(parts) => expressions.extend(parts),
            _ => {}
        }
    }
    ids
}
