//! One typed accessor isolates repeated closures from the versions of a mutable captured local.
use super::*;

pub(super) fn name(id: usize) -> String {
    format!("__ulua_access{id}")
}

pub(super) fn select(module: &ir::Module) -> Vec<bool> {
    let mut copies = std::collections::HashSet::new();
    for f in &module.functions {
        let mut work: Vec<_> = f.initializers.iter().chain(&f.body).collect();
        while let Some(s) = work.pop() {
            use ir::Statement as S;
            match s.form() {
                S::For(init, _, update, body, ids) => {
                    copies.extend(ids.iter().copied());
                    work.extend(init);
                    work.extend(update.iter().map(|s| s.as_ref()));
                    work.push(body);
                }
                S::Block(b) => work.extend(b),
                S::If(_, a, b) => {
                    work.push(a);
                    work.extend(b.iter().map(|s| s.as_ref()));
                }
                S::While(_, b) | S::ForOf(_, _, b) => work.push(b),
                S::Try(a, b, f) => {
                    work.push(a);
                    work.extend(b.iter().map(|(_, s)| s.as_ref()));
                    work.extend(f.iter().map(|s| s.as_ref()));
                }
                _ => {}
            }
        }
    }
    module
        .capture_counts
        .iter()
        .enumerate()
        .map(|(id, count)| *count > 1 && module.bindings[id].mutable && !copies.contains(&id))
        .collect()
}

pub(super) fn declaration(id: usize, location: &str, ty: &str) -> String {
    // C10 prevents a read before initialization. Each write supplies the checked declared type.
    // The helper returns the original native value; the binding has no wrapper or extra field.
    format!(
        "local function {}(write: boolean, value: {ty}): {ty}\nif write then {location} = value end\nreturn {location}\nend\n",
        name(id)
    )
}

impl Emitter<'_> {
    pub(super) fn uses_accessor(&self, id: usize) -> bool {
        self.context
            .map(|context| context.accessors.as_slice())
            .or(self.probe_accessors)
            .is_some_and(|accessors| accessors[id])
            && !self.copies.contains_key(&id)
    }
}
