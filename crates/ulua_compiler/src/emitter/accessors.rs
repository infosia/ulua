//! One typed accessor isolates repeated closures from the versions of a mutable captured local.
use super::*;

pub(super) fn name(id: usize) -> String {
    format!("__ulua_access{id}")
}

pub(super) fn select(module: &ir::Module) -> Vec<bool> {
    let mut counts = vec![0u8; module.bindings.len()];
    let mut copies = std::collections::HashSet::new();
    for f in &module.functions {
        let mut ids = std::collections::HashSet::new();
        let mut work: Vec<_> = f.initializers.iter().chain(&f.body).collect();
        for s in &work {
            ids.extend(ir::references(s));
        }
        for id in ids {
            if module.bindings[id].owner != f.binding + 1 {
                counts[id] = counts[id].saturating_add(1);
            }
        }
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
    counts
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
        self.context.is_some_and(|context| context.accessors[id]) && !self.copies.contains_key(&id)
    }
}
