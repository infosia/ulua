//! Transitive declaration reachability, partitioned by the binding's function owner.
use super::*;
use std::collections::BinaryHeap;

impl Checker<'_> {
    pub(super) fn tdz(&self) -> Result<(), Diagnostic> {
        let mut latest = vec![HashMap::<usize, u32>::new(); self.bindings.len() + 1];
        let mut callers = vec![vec![]; latest.len()];
        for function in &self.functions {
            let parent = self.bindings[function.binding].owner;
            if parent != 0 {
                // C10.3 attributes every created function's references to its creator.
                callers[function.binding + 1].push(parent);
            }
        }
        for &(owner, id, offset) in &self.references {
            if owner == 0 {
                continue;
            }
            let b = &self.bindings[id];
            if b.declaration || self.class_bindings.contains(&id) {
                callers[id + 1].push(owner);
            }
            let inside_class =
                self.class_bindings.contains(&id) && offset >= b.start && offset < b.end;
            if !b.declaration && b.owner != owner && !inside_class {
                let end = latest[owner].entry(b.owner).or_default();
                *end = (*end).max(b.end);
            }
        }
        let mut pending = BinaryHeap::new();
        for (function, owners) in latest.iter().enumerate() {
            for (&owner, &end) in owners {
                pending.push((end, function, owner));
            }
        }
        while let Some((end, function, owner)) = pending.pop() {
            if latest[function].get(&owner) != Some(&end) {
                continue;
            }
            for &caller in &callers[function] {
                // A declaration inside the caller is not an external dependency of that caller.
                if caller == owner {
                    continue;
                }
                let previous = latest[caller].entry(owner).or_default();
                if *previous < end {
                    *previous = end;
                    pending.push((end, caller, owner));
                }
            }
        }
        for &(_, id, offset) in &self.references {
            if (self.bindings[id].declaration || self.class_bindings.contains(&id))
                && latest[id + 1].values().any(|end| offset < *end)
            {
                return Err(Diagnostic::at(
                    self.source,
                    offset as usize,
                    "C10",
                    "Function reaches an undeclared binding",
                ));
            }
        }
        Ok(())
    }
}
