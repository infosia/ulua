//! Growth measurements count checked nodes without expansion of shared type links.
use super::*;
pub(crate) fn nodes(module: &Module) -> usize {
    let mut statements: Vec<_> = module.body.iter().collect();
    let mut expressions = vec![];
    for f in &module.functions {
        statements.extend(&f.initializers);
        statements.extend(&f.body);
        expressions.extend(
            f.parameters
                .iter()
                .filter_map(|(_, default)| default.as_ref()),
        );
    }
    let mut count = 0;
    while let Some(s) = statements.pop() {
        count += 1;
        match s {
            Statement::Located(_, s) => statements.push(s),
            Statement::Declare(_, e) | Statement::Assign(_, _, e) | Statement::Eval(e) => {
                expressions.push(e)
            }
            Statement::Field(a, _, _, e) | Statement::ErrorField(a, _, _, e) => {
                expressions.extend([a, e])
            }
            Statement::Element(a, i, _, e) => expressions.extend([a, i, e]),
            Statement::ForOf(_, a, b) => {
                expressions.push(a);
                statements.push(b);
            }
            Statement::Try(a, b, f) => {
                statements.push(a);
                if let Some((_, b)) = b {
                    statements.push(b);
                }
                statements.extend(f.iter().map(|s| s.as_ref()));
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
            Statement::Return(Some(e)) | Statement::Throw(e) => expressions.push(e),
            _ => {}
        }
    }
    while let Some(e) = expressions.pop() {
        count += 1;
        match &e.kind {
            ExprKind::Error(_, a)
            | ExprKind::InstanceOf(a, _)
            | ExprKind::ErrorField(a, _)
            | ExprKind::OptionalErrorField(a, _)
            | ExprKind::Field(a, _)
            | ExprKind::OptionalField(a, _)
            | ExprKind::OptionalLength(a)
            | ExprKind::Unary(_, a)
            | ExprKind::Length(a) => expressions.push(a),
            ExprKind::OptionalPush(a, b)
            | ExprKind::Coalesce(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Logical(_, a, b)
            | ExprKind::Index(a, b)
            | ExprKind::Push(a, b) => expressions.extend([a.as_ref(), b.as_ref()]),
            ExprKind::OptionalMethod(_, _, c, args)
            | ExprKind::OptionalCall(c, _, args)
            | ExprKind::Method(_, _, c, args)
            | ExprKind::Call(c, args) => {
                expressions.push(c);
                expressions.extend(args);
            }
            ExprKind::ClassCall(c, receiver, args, _) => {
                expressions.extend([c.as_ref(), receiver.as_ref()]);
                expressions.extend(args);
            }
            ExprKind::Conditional(t, a, b) => {
                expressions.extend([t.as_ref(), a.as_ref(), b.as_ref()])
            }
            ExprKind::Object(fields) => expressions.extend(fields.iter().map(|f| &f.1)),
            ExprKind::Template(parts) | ExprKind::Array(parts) => expressions.extend(parts),
            _ => {}
        }
    }
    count
}
