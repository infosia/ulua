//! Typed forms that the emitter consumes.
pub(crate) struct Module {
    pub(crate) functions: Vec<Function>,
}
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) body: Vec<Statement>,
}
pub(crate) enum Statement {
    Print(String),
}
