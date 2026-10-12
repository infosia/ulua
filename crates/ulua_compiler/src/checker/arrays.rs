//! Array iteration uses one receiver and a fresh constant binding per step.
use super::*;
use crate::ir::Statement as S;
impl Checker<'_> {
    pub(super) fn for_of(&mut self, f: &ForOfStatement<'_>, depth: u32) -> Result<S, Diagnostic> {
        let ForStatementLeft::VariableDeclaration(d) = &f.left else {
            return Err(self.error(f.span, "C11", "For-of requires a const declaration"));
        };
        if f.r#await
            || d.kind != VariableDeclarationKind::Const
            || d.declare
            || d.declarations.len() != 1
        {
            return Err(self.error(f.span, "C11", "Invalid for-of declaration"));
        }
        let v = &d.declarations[0];
        let BindingPattern::BindingIdentifier(n) = &v.id else {
            return Err(self.error(v.span, "C11", "For-of requires an identifier"));
        };
        if v.init.is_some() || v.definite || v.type_annotation.is_some() {
            return Err(self.error(v.span, "C11", "Invalid for-of binding"));
        }
        // The iteration binding also shadows outer names in the receiver's TDZ.
        self.scopes.push(HashMap::new());
        let id = self.bind(
            n.name.as_str(),
            ir::Binding {
                ty: None,
                owner: self.owner,
                start: v.span.start,
                end: f.right.span().end,
                mutable: false,
                declaration: false,
            },
            v.span,
        )?;
        let receiver = self.expr(&f.right, depth, false)?;
        let element = self
            .types
            .element(receiver.ty)
            .ok_or_else(|| self.error(f.span, "C6", "For-of requires an array"))?;
        self.bindings[id].ty = Some(element);
        self.equality.bindings[id] = self.types.element(element).is_some();
        let mut exit = self.equality.join();
        exit.edge(&self.equality, self.reachable);
        let update = self.equality.join();
        self.loops.push(equality::LoopFlow { exit, update });
        let body = self.statement(&f.body, depth)?;
        let flow = self
            .loops
            .pop()
            .ok_or_else(|| self.error(f.span, "C11", "Missing loop flow"))?;
        self.reachable = self.equality.finish(flow.exit);
        self.scopes.pop();
        // Entry makes captured storage local to each execution of the loop body.
        let body = S::Block(vec![S::Entry(vec![id]), body]);
        Ok(S::Located(
            f.span.start,
            Box::new(S::ForOf(id, receiver, Box::new(body))),
        ))
    }
}
