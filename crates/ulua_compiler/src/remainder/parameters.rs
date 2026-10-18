//! Whole-module uses determine which declarations have only direct calls.
use super::*;

pub(super) struct Parameters(pub HashMap<usize, Vec<Option<usize>>>);

#[derive(Default)]
struct Uses {
    reads: HashMap<usize, usize>,
    calls: HashMap<usize, usize>,
}
impl Visitor for Uses {
    fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
        match &e.kind {
            E::Read(id) => *self.reads.entry(*id).or_default() += 1,
            E::Call(callee, _) => {
                if let E::Read(id) = callee.kind {
                    *self.calls.entry(id).or_default() += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
impl Parameters {
    pub(super) fn collect(module: &mut Module) -> Result<Self, Diagnostic> {
        let mut uses = Uses::default();
        visit_module(module, &mut uses)?;
        let mut parameters = HashMap::new();
        for f in &module.functions {
            // An uncalled parameter stays ineligible even when its body assigns it.
            if f.declaration
                && !f.exported
                && !f.asynchronous
                && uses.calls.contains_key(&f.binding)
                && uses.calls.get(&f.binding) == uses.reads.get(&f.binding)
            {
                parameters.insert(
                    f.binding,
                    f.parameters
                        .iter()
                        .map(|(id, default)| default.is_none().then_some(*id))
                        .collect(),
                );
            }
        }
        Ok(Self(parameters))
    }
}
impl Build<'_> {
    pub(super) fn parameter_call(&mut self, e: &Expr) -> Result<(), Diagnostic> {
        if let E::Call(callee, args) = &e.kind
            && let E::Read(id) = callee.kind
            && let Some(parameters) = self.parameters.0.get(&id)
        {
            // The checker validates omitted defaults and argument counts.
            // Visit supplied arguments only, so omitted defaults add no cost per call.
            for (parameter, argument) in parameters.iter().zip(args) {
                if let Some(id) = parameter {
                    self.write_value(*id, argument)?;
                }
            }
        }
        Ok(())
    }
}
