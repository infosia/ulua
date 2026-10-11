//! Each bit holds the pending state at one control-flow depth.
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Joins {
    flags: HashMap<usize, u64>,
    depth: u32,
}
impl Joins {
    pub(super) fn enter(&mut self) {
        self.depth += 1;
    }
    pub(super) fn leave(&mut self) {
        self.depth -= 1;
    }
    pub(super) fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }
    pub(super) fn clear(&mut self, id: usize) -> bool {
        let Some(flags) = self.flags.get_mut(&id) else {
            return false;
        };
        let pending = *flags & (1 << self.depth) != 0;
        // A read or overwrite also invalidates stale states from finished descendants.
        *flags &= (1 << self.depth) - 1;
        if *flags == 0 {
            self.flags.remove(&id);
        }
        pending
    }
    pub(super) fn assigned(&mut self, id: usize) {
        if self.depth != 0 {
            // The write resolves this path but requires a join in each enclosing scope.
            self.flags.insert(id, (1 << self.depth) - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_paths_preserve_outer_pending_states() {
        let mut joins = Joins::default();
        joins.enter();
        joins.assigned(7);
        assert!(!joins.clear(7));
        joins.enter();
        joins.assigned(7);
        joins.leave();
        assert!(joins.clear(7));
        assert!(!joins.clear(7));
        joins.leave();
        assert!(joins.clear(7));
        assert!(joins.is_empty());
        joins.enter();
        assert!(!joins.clear(7));
        joins.leave();
    }
}
