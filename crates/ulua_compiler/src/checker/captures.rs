//! Resolve write locations before type flow. Scope events cost O(n log n).
use super::*;
#[derive(Clone, Copy)]
struct Scope {
    span: Span,
    owner: u32,
}
enum Work<'a> {
    Statement(&'a Statement<'a>, Scope),
    Function(&'a Function<'a>, Scope),
    Expr(&'a Expression<'a>, Scope),
}
struct Name {
    name: String,
    scope: Scope,
    start: u32,
    mutable: bool,
}
struct Write {
    name: String,
    offset: u32,
    owner: u32,
}
pub(super) struct Writes {
    pub(super) captured: std::collections::HashSet<u32>,
    pub(super) assignments: Vec<(u32, u32)>,
}
pub(super) fn resolve<'a>(program: &'a Program<'a>) -> Writes {
    let module = Scope {
        span: program.span,
        owner: u32::MAX,
    };
    let mut pending: Vec<_> = program
        .body
        .iter()
        .map(|s| Work::Statement(s, module))
        .collect();
    let mut names = vec![];
    let mut writes = vec![];
    while let Some(work) = pending.pop() {
        match work {
            Work::Statement(s, scope) => match s {
                Statement::ExportDeclaration(e) => {
                    if let Declaration::FunctionDeclaration(f) = &e.declaration {
                        pending.push(Work::Function(f, scope));
                    }
                }
                Statement::FunctionDeclaration(f) => pending.push(Work::Function(f, scope)),
                Statement::ClassDeclaration(class) => {
                    for member in &class.body.body {
                        match member {
                            ClassElement::MethodDefinition(m) => {
                                pending.push(Work::Function(&m.value, scope))
                            }
                            ClassElement::PropertyDefinition(p) => {
                                if let Some(e) = &p.value {
                                    pending.push(Work::Expr(e, scope));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Statement::BlockStatement(b) => {
                    let scope = Scope {
                        span: b.span,
                        ..scope
                    };
                    pending.extend(b.body.iter().map(|s| Work::Statement(s, scope)));
                }
                Statement::VariableDeclaration(d) => variables(d, scope, &mut names, &mut pending),
                Statement::ExpressionStatement(e) => pending.push(Work::Expr(&e.expression, scope)),
                Statement::ThrowStatement(t) => pending.push(Work::Expr(&t.argument, scope)),
                Statement::TryStatement(t) => {
                    let block = Scope {
                        span: t.block.span,
                        ..scope
                    };
                    pending.extend(t.block.body.iter().map(|s| Work::Statement(s, block)));
                    if let Some(c) = &t.handler {
                        let block = Scope {
                            span: c.body.span,
                            ..scope
                        };
                        if let Some(p) = &c.param
                            && let BindingPattern::BindingIdentifier(n) = &p.pattern
                        {
                            names.push(Name {
                                name: n.name.to_string(),
                                scope: block,
                                start: p.span.start,
                                mutable: false,
                            });
                        }
                        pending.extend(c.body.body.iter().map(|s| Work::Statement(s, block)));
                    }
                    if let Some(f) = &t.finalizer {
                        let block = Scope {
                            span: f.span,
                            ..scope
                        };
                        pending.extend(f.body.iter().map(|s| Work::Statement(s, block)));
                    }
                }
                Statement::ReturnStatement(r) => {
                    if let Some(e) = &r.argument {
                        pending.push(Work::Expr(e, scope));
                    }
                }
                Statement::IfStatement(i) => {
                    pending.push(Work::Expr(&i.test, scope));
                    pending.push(Work::Statement(&i.consequent, scope));
                    if let Some(b) = &i.alternate {
                        pending.push(Work::Statement(b, scope));
                    }
                }
                Statement::WhileStatement(w) => {
                    pending.push(Work::Expr(&w.test, scope));
                    pending.push(Work::Statement(&w.body, scope));
                }
                Statement::ForStatement(f) => {
                    let scope = Scope {
                        span: f.span,
                        ..scope
                    };
                    if let Some(ForStatementInit::VariableDeclaration(d)) = &f.init {
                        variables(d, scope, &mut names, &mut pending);
                    }
                    for e in [&f.test, &f.update].into_iter().flatten() {
                        pending.push(Work::Expr(e, scope));
                    }
                    pending.push(Work::Statement(&f.body, scope));
                }
                Statement::ForOfStatement(f) => {
                    let scope = Scope {
                        span: f.span,
                        ..scope
                    };
                    if let ForStatementLeft::VariableDeclaration(d) = &f.left {
                        variables(d, scope, &mut names, &mut pending);
                    }
                    pending.push(Work::Expr(&f.right, scope));
                    pending.push(Work::Statement(&f.body, scope));
                }
                _ => {}
            },
            Work::Function(f, parent) => {
                if let Some(id) = &f.id {
                    names.push(Name {
                        name: id.name.to_string(),
                        scope: parent,
                        start: f.span.start,
                        mutable: false,
                    });
                }
                let scope = Scope {
                    span: f.span,
                    owner: f.span.start,
                };
                parameters(&f.params, scope, &mut names, &mut pending);
                if let Some(body) = &f.body {
                    let scope = Scope {
                        span: body.span,
                        ..scope
                    };
                    pending.extend(body.statements.iter().map(|s| Work::Statement(s, scope)));
                }
            }
            Work::Expr(e, scope) => match e {
                Expression::NewExpression(n) => {
                    pending.push(Work::Expr(&n.callee, scope));
                    pending.extend(
                        n.arguments
                            .iter()
                            .filter_map(|a| a.as_expression())
                            .map(|e| Work::Expr(e, scope)),
                    );
                }
                Expression::ArrowFunctionExpression(a) => {
                    let scope = Scope {
                        span: a.span,
                        owner: a.span.start,
                    };
                    parameters(&a.params, scope, &mut names, &mut pending);
                    if let Some(e) = a.body.as_expression() {
                        pending.push(Work::Expr(e, scope));
                    } else if let ArrowFunctionBody::FunctionBody(b) = &a.body {
                        let scope = Scope {
                            span: b.span,
                            ..scope
                        };
                        pending.extend(b.statements.iter().map(|s| Work::Statement(s, scope)));
                    }
                }
                Expression::AssignmentExpression(a) => {
                    if let AssignmentTarget::AssignmentTargetIdentifier(n) = &a.left {
                        writes.push(Write {
                            name: n.name.to_string(),
                            offset: n.span.start,
                            owner: scope.owner,
                        });
                    }
                    match &a.left {
                        AssignmentTarget::StaticMemberExpression(m) => {
                            pending.push(Work::Expr(&m.object, scope))
                        }
                        AssignmentTarget::ComputedMemberExpression(m) => {
                            pending.push(Work::Expr(&m.object, scope));
                            pending.push(Work::Expr(&m.expression, scope));
                        }
                        _ => {}
                    }
                    pending.push(Work::Expr(&a.right, scope));
                }
                Expression::UpdateExpression(u) => {
                    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(n) = &u.argument {
                        writes.push(Write {
                            name: n.name.to_string(),
                            offset: n.span.start,
                            owner: scope.owner,
                        });
                    }
                }
                Expression::ArrayExpression(a) => pending.extend(
                    a.elements
                        .iter()
                        .filter_map(|e| e.as_expression())
                        .map(|e| Work::Expr(e, scope)),
                ),
                Expression::ObjectExpression(o) => {
                    for entry in &o.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = entry {
                            pending.push(Work::Expr(&p.value, scope));
                        }
                    }
                }
                Expression::BinaryExpression(b) => {
                    pending.push(Work::Expr(&b.left, scope));
                    pending.push(Work::Expr(&b.right, scope));
                }
                Expression::LogicalExpression(b) => {
                    pending.push(Work::Expr(&b.left, scope));
                    pending.push(Work::Expr(&b.right, scope));
                }
                Expression::ConditionalExpression(c) => {
                    for e in [&c.test, &c.consequent, &c.alternate] {
                        pending.push(Work::Expr(e, scope));
                    }
                }
                Expression::ParenthesizedExpression(p) => {
                    pending.push(Work::Expr(&p.expression, scope))
                }
                Expression::UnaryExpression(u) => pending.push(Work::Expr(&u.argument, scope)),
                Expression::StaticMemberExpression(m) => pending.push(Work::Expr(&m.object, scope)),
                Expression::ComputedMemberExpression(m) => {
                    pending.push(Work::Expr(&m.object, scope));
                    pending.push(Work::Expr(&m.expression, scope));
                }
                Expression::CallExpression(c) => call(c, scope, &mut pending),
                Expression::ChainExpression(c) => match &c.expression {
                    ChainElement::StaticMemberExpression(m) => {
                        pending.push(Work::Expr(&m.object, scope))
                    }
                    ChainElement::CallExpression(c) => call(c, scope, &mut pending),
                    _ => {}
                },
                Expression::TemplateLiteral(t) => {
                    pending.extend(t.expressions.iter().map(|e| Work::Expr(e, scope)))
                }
                _ => {}
            },
        }
    }
    // Exit, entry, and write events share one source-order sweep.
    let mut events = vec![];
    for (id, n) in names.iter().enumerate() {
        events.push((
            n.scope.span.start,
            1,
            std::cmp::Reverse(n.scope.span.end),
            id,
        ));
        events.push((
            n.scope.span.end,
            0,
            std::cmp::Reverse(n.scope.span.start),
            id,
        ));
    }
    for (id, w) in writes.iter().enumerate() {
        events.push((w.offset, 2, std::cmp::Reverse(0), id));
    }
    events.sort_unstable();
    let mut scopes = HashMap::<&str, Vec<usize>>::new();
    let mut result = Writes {
        captured: Default::default(),
        assignments: vec![],
    };
    for (_, kind, _, id) in events {
        match kind {
            0 => {
                if let Some(stack) = scopes.get_mut(names[id].name.as_str()) {
                    stack.pop();
                }
            }
            1 => scopes.entry(names[id].name.as_str()).or_default().push(id),
            _ => {
                let w = &writes[id];
                if let Some(id) = scopes.get(w.name.as_str()).and_then(|s| s.last()) {
                    let n = &names[*id];
                    if n.mutable {
                        result.assignments.push((n.start, w.offset));
                        if n.scope.owner != w.owner {
                            result.captured.insert(n.start);
                        }
                    }
                }
            }
        }
    }
    result.assignments.sort_unstable_by_key(|a| a.1);
    result
}
fn variables<'a>(
    d: &'a VariableDeclaration<'a>,
    scope: Scope,
    names: &mut Vec<Name>,
    pending: &mut Vec<Work<'a>>,
) {
    for v in &d.declarations {
        if let BindingPattern::BindingIdentifier(n) = &v.id {
            names.push(Name {
                name: n.name.to_string(),
                scope,
                start: v.span.start,
                mutable: d.kind == VariableDeclarationKind::Let,
            });
        }
        if let Some(e) = &v.init {
            pending.push(Work::Expr(e, scope));
        }
    }
}
fn parameters<'a>(
    p: &'a FormalParameters<'a>,
    scope: Scope,
    names: &mut Vec<Name>,
    pending: &mut Vec<Work<'a>>,
) {
    for p in &p.items {
        if let BindingPattern::BindingIdentifier(n) = &p.pattern {
            names.push(Name {
                name: n.name.to_string(),
                scope,
                start: p.span.start,
                mutable: true,
            });
        }
        if let Some(e) = &p.initializer {
            pending.push(Work::Expr(e, scope));
        }
    }
}
fn call<'a>(c: &'a CallExpression<'a>, scope: Scope, pending: &mut Vec<Work<'a>>) {
    pending.push(Work::Expr(&c.callee, scope));
    pending.extend(
        c.arguments
            .iter()
            .filter_map(|a| a.as_expression())
            .map(|e| Work::Expr(e, scope)),
    );
}
