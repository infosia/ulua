//! Iterative AST depth checks. No parser visitor or recursive AST walk.
use crate::Diagnostic;
use oxc_ast::ast::*;
use oxc_span::GetSpan;

enum Node<'a> {
    Statement(&'a Statement<'a>),
    Declaration(&'a Declaration<'a>),
    Function(&'a Function<'a>),
    Class(&'a Class<'a>),
    Expression(&'a Expression<'a>),
}
impl Node<'_> {
    fn offset(&self) -> u32 {
        match self {
            Self::Statement(n) => n.span().start,
            Self::Declaration(n) => n.span().start,
            Self::Function(n) => n.span.start,
            Self::Class(n) => n.span.start,
            Self::Expression(n) => n.span().start,
        }
    }
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
        .rev()
        .map(|s| (Node::Statement(s), 1))
        .collect();
    while let Some((node, depth)) = pending.pop() {
        let offset = node.offset();
        if depth >= limit {
            return Err(Diagnostic::at(
                source,
                offset as usize,
                "limit:nesting-depth",
                "AST depth limit exceeded",
            ));
        }
        let next = depth + 1;
        let children = pending.len();
        match node {
            Node::Statement(s) => match s {
                Statement::ExportDeclaration(n) => {
                    pending.push((Node::Declaration(&n.declaration), next))
                }
                Statement::FunctionDeclaration(n) => pending.push((Node::Function(n), next)),
                Statement::ClassDeclaration(n) => pending.push((Node::Class(n), next)),
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
                Statement::ThrowStatement(n) => pending.push((Node::Expression(&n.argument), next)),
                Statement::TryStatement(n) => {
                    pending.extend(n.block.body.iter().map(|s| (Node::Statement(s), next + 1)));
                    if let Some(c) = &n.handler {
                        pending.extend(c.body.body.iter().map(|s| (Node::Statement(s), next + 1)));
                    }
                    if let Some(f) = &n.finalizer {
                        pending.extend(f.body.iter().map(|s| (Node::Statement(s), next + 1)));
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
                Statement::ForOfStatement(n) => {
                    pending.push((Node::Expression(&n.right), next));
                    pending.push((Node::Statement(&n.body), next));
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
            Node::Class(class) => {
                for member in &class.body.body {
                    match member {
                        ClassElement::MethodDefinition(m) => {
                            pending.push((Node::Function(&m.value), next))
                        }
                        ClassElement::PropertyDefinition(p) => {
                            if let Some(e) = &p.value {
                                pending.push((Node::Expression(e), next));
                            }
                        }
                        _ => {}
                    }
                }
            }
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
                Expression::NewExpression(n) => {
                    pending.push((Node::Expression(&n.callee), next));
                    for arg in &n.arguments {
                        if let Some(e) = arg.as_expression() {
                            pending.push((Node::Expression(e), next));
                        }
                    }
                }
                Expression::ArrowFunctionExpression(a) => {
                    for p in &a.params.items {
                        if let Some(e) = &p.initializer {
                            pending.push((Node::Expression(e), next));
                        }
                    }
                    if let Some(e) = a.body.as_expression() {
                        pending.push((Node::Expression(e), next));
                    } else if let ArrowFunctionBody::FunctionBody(b) = &a.body {
                        pending.extend(b.statements.iter().map(|s| (Node::Statement(s), next)));
                    }
                }
                Expression::AssignmentExpression(n) => {
                    pending.push((Node::Expression(&n.right), next));
                    match &n.left {
                        AssignmentTarget::ComputedMemberExpression(m) => {
                            pending.push((Node::Expression(&m.object), next));
                            pending.push((Node::Expression(&m.expression), next));
                        }
                        AssignmentTarget::StaticMemberExpression(m) => {
                            pending.push((Node::Expression(&m.object), next))
                        }
                        _ => {}
                    }
                }
                Expression::ObjectExpression(o) => {
                    for entry in &o.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = entry {
                            pending.push((Node::Expression(&p.value), next));
                        }
                    }
                }
                Expression::ChainExpression(c) => match &c.expression {
                    ChainElement::StaticMemberExpression(m) => {
                        pending.push((Node::Expression(&m.object), next))
                    }
                    ChainElement::CallExpression(c) => {
                        pending.push((Node::Expression(&c.callee), next));
                        for arg in &c.arguments {
                            if let Some(e) = arg.as_expression() {
                                pending.push((Node::Expression(e), next));
                            }
                        }
                    }
                    _ => {}
                },
                Expression::ArrayExpression(a) => {
                    for item in &a.elements {
                        if let Some(e) = item.as_expression() {
                            pending.push((Node::Expression(e), next));
                        }
                    }
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
        // A later sibling cannot hide the first form that reaches the depth bound.
        pending[children..].sort_by_key(|(node, _)| std::cmp::Reverse(node.offset()));
    }
    Ok(())
}

// The checker calls this after it determines the reachability of a head.
pub(crate) fn check_head(
    e: &Expression<'_>,
    source: &[u8],
    unreachable: bool,
) -> Result<(), Diagnostic> {
    let mut pending = vec![e];
    while let Some(e) = pending.pop() {
        match e {
            Expression::ArrowFunctionExpression(_) if unreachable => {
                return Err(Diagnostic::at(
                    source,
                    e.span().start as usize,
                    "C16",
                    crate::checker::UNREACHABLE_ARROW,
                ));
            }
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                return Err(Diagnostic::at(
                    source,
                    e.span().start as usize,
                    "C9",
                    "Closure in a for head",
                ));
            }
            Expression::ArrayExpression(a) => {
                for item in &a.elements {
                    if let Some(e) = item.as_expression() {
                        pending.push(e);
                    } else if let ArrayExpressionElement::SpreadElement(s) = item {
                        pending.push(&s.argument);
                    }
                }
            }
            Expression::ObjectExpression(o) => {
                for item in &o.properties {
                    match item {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            pending.push(&p.value);
                            if let Some(e) = p.key.as_expression() {
                                pending.push(e);
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(p) => pending.push(&p.argument),
                    }
                }
            }
            Expression::NewExpression(n) => {
                pending.push(&n.callee);
                for arg in &n.arguments {
                    if let Some(e) = arg.as_expression() {
                        pending.push(e);
                    } else if let Argument::SpreadElement(s) = arg {
                        pending.push(&s.argument);
                    }
                }
            }
            Expression::TSAsExpression(t) => pending.push(&t.expression),
            Expression::TSSatisfiesExpression(t) => pending.push(&t.expression),
            Expression::TSNonNullExpression(t) => pending.push(&t.expression),
            Expression::AwaitExpression(a) => pending.push(&a.argument),
            Expression::ParenthesizedExpression(p) => pending.push(&p.expression),
            Expression::CallExpression(c) => {
                pending.push(&c.callee);
                for arg in &c.arguments {
                    if let Some(e) = arg.as_expression() {
                        pending.push(e);
                    } else if let Argument::SpreadElement(s) = arg {
                        pending.push(&s.argument);
                    }
                }
            }
            Expression::BinaryExpression(b) => pending.extend([&b.left, &b.right]),
            Expression::LogicalExpression(b) => pending.extend([&b.left, &b.right]),
            Expression::AssignmentExpression(a) => {
                pending.push(&a.right);
                match &a.left {
                    AssignmentTarget::ComputedMemberExpression(m) => {
                        pending.extend([&m.object, &m.expression])
                    }
                    AssignmentTarget::StaticMemberExpression(m) => pending.push(&m.object),
                    _ => {}
                }
            }
            Expression::UpdateExpression(u) => match &u.argument {
                SimpleAssignmentTarget::ComputedMemberExpression(m) => {
                    pending.extend([&m.object, &m.expression])
                }
                SimpleAssignmentTarget::StaticMemberExpression(m) => pending.push(&m.object),
                _ => {}
            },
            Expression::ConditionalExpression(c) => {
                pending.extend([&c.test, &c.consequent, &c.alternate])
            }
            Expression::UnaryExpression(u) => pending.push(&u.argument),
            Expression::TemplateLiteral(t) => pending.extend(&t.expressions),
            Expression::SequenceExpression(s) => pending.extend(&s.expressions),
            Expression::StaticMemberExpression(m) => pending.push(&m.object),
            Expression::ComputedMemberExpression(m) => pending.extend([&m.object, &m.expression]),
            _ => {}
        }
    }
    Ok(())
}
