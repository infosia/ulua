//! Iterative AST depth checks. No parser visitor or recursive AST walk.
use crate::Diagnostic;
use oxc_ast::ast::*;
use oxc_span::GetSpan;

enum Node<'a> {
    Statement(&'a Statement<'a>),
    Declaration(&'a Declaration<'a>),
    Function(&'a Function<'a>),
    Expression(&'a Expression<'a>),
}

pub(crate) fn check<'a>(
    program: &'a Program<'a>,
    limit: u32,
    source: &[u8],
) -> Result<(), Diagnostic> {
    if limit == 0 {
        return Err(Diagnostic::at(
            source,
            0,
            "limit:nesting-depth",
            "AST depth limit exceeded",
        ));
    }
    let mut pending: Vec<_> = program
        .body
        .iter()
        .map(|s| (Node::Statement(s), 1))
        .collect();
    while let Some((node, depth)) = pending.pop() {
        let offset = match &node {
            Node::Statement(n) => n.span().start,
            Node::Declaration(n) => n.span().start,
            Node::Function(n) => n.span.start,
            Node::Expression(n) => n.span().start,
        };
        if depth >= limit {
            return Err(Diagnostic::at(
                source,
                offset as usize,
                "limit:nesting-depth",
                "AST depth limit exceeded",
            ));
        }
        let next = depth + 1;
        match node {
            Node::Statement(s) => match s {
                Statement::ExportDeclaration(n) => {
                    pending.push((Node::Declaration(&n.declaration), next))
                }
                Statement::FunctionDeclaration(n) => pending.push((Node::Function(n), next)),
                Statement::ExpressionStatement(n) => {
                    pending.push((Node::Expression(&n.expression), next))
                }
                Statement::BlockStatement(n) => {
                    pending.extend(n.body.iter().map(|s| (Node::Statement(s), next)))
                }
                Statement::VariableDeclaration(n) => {
                    for d in &n.declarations {
                        if let Some(e) = &d.init {
                            pending.push((Node::Expression(e), next));
                        }
                    }
                }
                Statement::ReturnStatement(n) => {
                    if let Some(e) = &n.argument {
                        pending.push((Node::Expression(e), next));
                    }
                }
                Statement::IfStatement(n) => {
                    pending.push((Node::Expression(&n.test), next));
                    pending.push((Node::Statement(&n.consequent), next));
                    if let Some(s) = &n.alternate {
                        pending.push((Node::Statement(s), next));
                    }
                }
                Statement::WhileStatement(n) => {
                    pending.push((Node::Expression(&n.test), next));
                    pending.push((Node::Statement(&n.body), next));
                }
                Statement::ForStatement(n) => {
                    pending.push((Node::Statement(&n.body), next));
                    for e in [&n.test, &n.update].into_iter().flatten() {
                        pending.push((Node::Expression(e), next));
                    }
                    if let Some(init) = &n.init {
                        if let Some(e) = init.as_expression() {
                            pending.push((Node::Expression(e), next));
                        } else if let ForStatementInit::VariableDeclaration(d) = init {
                            for d in &d.declarations {
                                if let Some(e) = &d.init {
                                    pending.push((Node::Expression(e), next));
                                }
                            }
                        }
                    }
                }
                // Later-phase nodes stop before the recursive checker and emitter.
                _ => {}
            },
            Node::Declaration(d) => match d {
                Declaration::FunctionDeclaration(n) => pending.push((Node::Function(n), next)),
                Declaration::VariableDeclaration(n) => {
                    for d in &n.declarations {
                        if let Some(e) = &d.init {
                            pending.push((Node::Expression(e), next));
                        }
                    }
                }
                _ => {}
            },
            Node::Function(f) => {
                for p in &f.params.items {
                    if let Some(e) = &p.initializer {
                        pending.push((Node::Expression(e), next));
                    }
                }
                if let Some(body) = &f.body {
                    pending.extend(body.statements.iter().map(|s| (Node::Statement(s), next)));
                }
            }
            Node::Expression(e) => match e {
                Expression::AssignmentExpression(n) => {
                    pending.push((Node::Expression(&n.right), next));
                }
                Expression::BinaryExpression(n) => {
                    pending.push((Node::Expression(&n.left), next));
                    pending.push((Node::Expression(&n.right), next));
                }
                Expression::LogicalExpression(n) => {
                    pending.push((Node::Expression(&n.left), next));
                    pending.push((Node::Expression(&n.right), next));
                }
                Expression::CallExpression(n) => {
                    pending.push((Node::Expression(&n.callee), next));
                    for a in &n.arguments {
                        if let Some(e) = a.as_expression() {
                            pending.push((Node::Expression(e), next));
                        }
                    }
                }
                Expression::StaticMemberExpression(n) => {
                    pending.push((Node::Expression(&n.object), next))
                }
                Expression::ComputedMemberExpression(n) => {
                    pending.push((Node::Expression(&n.object), next));
                    pending.push((Node::Expression(&n.expression), next));
                }
                Expression::ParenthesizedExpression(n) => {
                    pending.push((Node::Expression(&n.expression), next))
                }
                Expression::UnaryExpression(n) => {
                    pending.push((Node::Expression(&n.argument), next))
                }
                Expression::ConditionalExpression(n) => {
                    for e in [&n.test, &n.consequent, &n.alternate] {
                        pending.push((Node::Expression(e), next));
                    }
                }
                Expression::SequenceExpression(n) => {
                    pending.extend(n.expressions.iter().map(|e| (Node::Expression(e), next)))
                }
                Expression::TemplateLiteral(n) => {
                    pending.extend(n.expressions.iter().map(|e| (Node::Expression(e), next)))
                }
                _ => {}
            },
        }
    }
    Ok(())
}
