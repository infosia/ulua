//! Statements and return coverage.
use super::*;
use crate::ir::{Expr, ExprKind, Statement as S};
impl Checker<'_> {
    pub(super) fn variables(
        &mut self,
        d: &VariableDeclaration<'_>,
        depth: u32,
    ) -> Result<Vec<S>, Diagnostic> {
        let mut out = vec![];
        for v in &d.declarations {
            let BindingPattern::BindingIdentifier(n) = &v.id else {
                return Err(self.unsupported(v.span));
            };
            let id = self
                .scopes
                .last()
                .and_then(|s| s.get(n.name.as_str()))
                .copied()
                .ok_or_else(|| self.error(v.span, "C11", "Missing declaration"))?;
            let depth = self.descend(depth, v.span)?;
            let value = self.expr(
                v.init
                    .as_ref()
                    .ok_or_else(|| self.error(v.span, "C8", "Missing initializer"))?,
                depth,
                false,
            )?;
            if matches!(value.ty, Type::Function(..)) {
                return Err(self.unsupported(v.span));
            }
            if self.bindings[id].ty.is_some() {
                self.flow(&value.ty, self.bindings[id].ty.as_ref(), v.span)?;
            } else {
                self.bindings[id].ty = Some(value.ty.clone());
            }
            self.equality.bindings[id] = if !self.bindings[id].mutable {
                self.expression_fact(&value)?
            } else if value.ty == Type::Boolean {
                NARROWABLE
            } else {
                WIDE
            };
            out.push(S::Located(v.span.start, Box::new(S::Declare(id, value))));
        }
        Ok(out)
    }
    pub(super) fn block(
        &mut self,
        statements: &[Statement<'_>],
        depth: u32,
    ) -> Result<Vec<S>, Diagnostic> {
        self.scopes.push(HashMap::new());
        let out = self.block_contents(statements, depth)?;
        self.scopes.pop();
        Ok(out)
    }
    pub(super) fn block_contents(
        &mut self,
        statements: &[Statement<'_>],
        depth: u32,
    ) -> Result<Vec<S>, Diagnostic> {
        for s in statements {
            if let Statement::VariableDeclaration(d) = s {
                self.variable_names(d)?;
            }
        }
        let mut out = vec![];
        for s in statements {
            out.push(self.statement(s, depth)?);
        }
        Ok(out)
    }
    fn test(&mut self, e: &Expression<'_>, depth: u32) -> Result<Expr, Diagnostic> {
        let test = self.expr(e, depth, false)?;
        if test.ty != Type::Boolean {
            return Err(self.error(e.span(), "C3", "Condition must be boolean"));
        }
        Ok(test)
    }
    fn target(&mut self, n: &IdentifierReference<'_>) -> Result<usize, Diagnostic> {
        let id = self.lookup(n.name.as_str(), n.span)?;
        if !self.bindings[id].mutable {
            return Err(self.error(n.span, "C11", "Target is not mutable"));
        }
        if self.bindings[id].ty == Some(Type::Boolean) {
            self.equality.bindings[id] = NARROWABLE;
        }
        Ok(id)
    }
    fn effect(&mut self, e: &Expression<'_>, depth: u32, update: bool) -> Result<S, Diagnostic> {
        let depth = self.descend(depth, e.span())?;
        match e {
            Expression::AssignmentExpression(a) => {
                let AssignmentTarget::AssignmentTargetIdentifier(n) = &a.left else {
                    return Err(match &a.left {
                        AssignmentTarget::StaticMemberExpression(_)
                        | AssignmentTarget::ComputedMemberExpression(_) => {
                            self.unsupported(a.left.span())
                        }
                        _ => self.error(a.left.span(), "C11", "Invalid assignment target"),
                    });
                };
                let id = self.target(n)?;
                let value = self.expr(&a.right, depth, false)?;
                let op = match a.operator.as_str() {
                    "=" => None,
                    "+=" => Some("+"),
                    "-=" => Some("-"),
                    "*=" => Some("*"),
                    "/=" => Some("/"),
                    "%=" => Some("%"),
                    _ => return Err(self.error(a.span, "C4", "Assignment operator is rejected")),
                };
                if op == Some("+")
                    && value.ty == Type::String
                    && self.bindings[id].ty == Some(Type::String)
                {
                    return Err(self.unsupported(a.span));
                }
                if op.is_some()
                    && (value.ty != Type::Number || self.bindings[id].ty != Some(Type::Number))
                {
                    return Err(self.error(a.span, "C4", "Compound assignment requires numbers"));
                }
                self.flow(&value.ty, self.bindings[id].ty.as_ref(), a.span)?;
                Ok(S::Assign(id, op, value))
            }
            Expression::UpdateExpression(u) => {
                let SimpleAssignmentTarget::AssignmentTargetIdentifier(n) = &u.argument else {
                    return Err(match &u.argument {
                        SimpleAssignmentTarget::StaticMemberExpression(_)
                        | SimpleAssignmentTarget::ComputedMemberExpression(_) => {
                            self.unsupported(u.argument.span())
                        }
                        _ => self.error(u.argument.span(), "C11", "Invalid update target"),
                    });
                };
                let id = self.target(n)?;
                if self.bindings[id].ty != Some(Type::Number) {
                    return Err(self.error(u.span, "C4", "Update requires number"));
                }
                Ok(S::Assign(
                    id,
                    Some(if u.operator.as_str() == "++" {
                        "+"
                    } else {
                        "-"
                    }),
                    Expr {
                        start: u.span.start,
                        ty: Type::Number,
                        kind: ExprKind::Number(1.0),
                    },
                ))
            }
            Expression::CallExpression(_) if !update => Ok(S::Eval(self.expr(e, depth, true)?)),
            Expression::TaggedTemplateExpression(_) | Expression::SequenceExpression(_) => {
                Err(self.error(e.span(), "C8", "Expression has no lowering"))
            }
            Expression::NewExpression(_) | Expression::ChainExpression(_) => {
                Err(self.unsupported(e.span()))
            }
            _ => Err(self.error(e.span(), "C11", "Invalid statement expression")),
        }
    }
    fn statement(&mut self, s: &Statement<'_>, depth: u32) -> Result<S, Diagnostic> {
        let depth = self.descend(depth, s.span())?;
        let form = match s {
            Statement::VariableDeclaration(d) => S::Block(self.variables(d, depth)?),
            Statement::BlockStatement(b) => S::Block(self.block(&b.body, depth)?),
            Statement::ExpressionStatement(e) => self.effect(&e.expression, depth, false)?,
            Statement::IfStatement(i) => S::If(
                self.test(&i.test, depth)?,
                Box::new(self.statement(&i.consequent, depth)?),
                i.alternate
                    .as_ref()
                    .map(|s| self.statement(s, depth).map(Box::new))
                    .transpose()?,
            ),
            Statement::WhileStatement(w) => {
                let test = self.test(&w.test, depth)?;
                self.loops += 1;
                let body = self.statement(&w.body, depth)?;
                self.loops -= 1;
                S::While(test, Box::new(body))
            }
            Statement::ForStatement(f) => {
                self.scopes.push(HashMap::new());
                let init = if let Some(ForStatementInit::VariableDeclaration(d)) = &f.init {
                    if d.kind != VariableDeclarationKind::Let {
                        return Err(self.error(d.span, "C11", "For initializer requires let"));
                    }
                    self.variable_names(d)?;
                    self.variables(d, depth)?
                } else if f.init.is_none() {
                    vec![]
                } else {
                    return Err(self.error(f.span, "C11", "For initializer must be a declaration"));
                };
                let ids = init
                    .iter()
                    .filter_map(|s| {
                        if let S::Declare(id, _) = s.form() {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();
                let test = f.test.as_ref().map(|e| self.test(e, depth)).transpose()?;
                let update = f
                    .update
                    .as_ref()
                    .map(|e| self.effect(e, depth, true).map(Box::new))
                    .transpose()?;
                self.loops += 1;
                let body = self.statement(&f.body, depth)?;
                self.loops -= 1;
                self.scopes.pop();
                S::For(init, test, update, Box::new(body), ids)
            }
            Statement::BreakStatement(b) if b.label.is_none() && self.loops > 0 => S::Break,
            Statement::ContinueStatement(c) if c.label.is_none() && self.loops > 0 => S::Continue,
            Statement::ReturnStatement(r) => {
                let value = r
                    .argument
                    .as_ref()
                    .map(|e| self.expr(e, depth, false))
                    .transpose()?;
                let ty = value.as_ref().map_or(&Type::Void, |v| &v.ty);
                self.flow(ty, Some(&self.result), r.span)?;
                S::Return(value)
            }
            Statement::FunctionDeclaration(_) | Statement::ForOfStatement(_) => {
                return Err(self.unsupported(s.span()));
            }
            Statement::SwitchStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForInStatement(_)
            | Statement::LabeledStatement(_)
            | Statement::TSInterfaceDeclaration(_) => {
                return Err(self.error(s.span(), "C8", "Statement has no lowering"));
            }
            _ => return Err(self.error(s.span(), "C11", "Statement is outside the surface")),
        };
        Ok(S::Located(s.span().start, Box::new(form)))
    }
}
// Return coverage uses an explicit postorder stack. It adds no compiler recursion.
pub(super) fn returns(body: &[S]) -> bool {
    enum Work<'a> {
        Body(&'a [S]),
        Form(&'a S),
        Any(usize),
        Both,
    }
    let mut pending = vec![Work::Body(body)];
    let mut values = vec![];
    while let Some(work) = pending.pop() {
        match work {
            Work::Body(body) => {
                pending.push(Work::Any(body.len()));
                pending.extend(body.iter().map(Work::Form));
            }
            Work::Form(s) => match s.form() {
                S::Return(_) => values.push(true),
                S::Block(b) => pending.push(Work::Body(b)),
                S::If(_, a, Some(b)) => {
                    pending.push(Work::Both);
                    pending.push(Work::Form(a));
                    pending.push(Work::Form(b));
                }
                S::While(test, body) => values
                    .push(matches!(test.kind, ExprKind::Boolean(true)) && !ir::loop_break(body)),
                S::For(_, test, _, body, _) => values.push(
                    test.as_ref()
                        .is_none_or(|t| matches!(t.kind, ExprKind::Boolean(true)))
                        && !ir::loop_break(body),
                ),
                _ => values.push(false),
            },
            Work::Any(n) => {
                let start = values.len().saturating_sub(n);
                let complete = values.drain(start..).any(|v| v);
                values.push(complete);
            }
            Work::Both => {
                let a = values.pop().unwrap_or(false);
                let b = values.pop().unwrap_or(false);
                values.push(a && b);
            }
        }
    }
    values.pop().unwrap_or(false)
}
