//! One byte budget for the module and its nested closures.
use super::*;
use std::cell::Cell;

pub(super) struct Output {
    bound: usize,
    bytes: Cell<usize>,
    crossing: Cell<Option<usize>>,
}
impl Output {
    pub(super) fn new(bound: usize) -> Self {
        Self {
            bound,
            bytes: Cell::new(0),
            crossing: Cell::new(None),
        }
    }
    pub(super) fn charge(&self, bytes: usize, offset: usize) {
        let total = self.bytes.get().saturating_add(bytes);
        self.bytes.set(total);
        if total > self.bound && self.crossing.get().is_none() {
            self.crossing.set(Some(offset));
        }
    }
    pub(super) fn exceeded(&self) -> bool {
        self.crossing.get().is_some()
    }
    pub(super) fn check(&self, module: &ir::Module) -> Result<(), Diagnostic> {
        if let Some(offset) = self.crossing.get() {
            return Err(Diagnostic::at(
                &module.source,
                offset,
                "limit:output",
                "Generated source exceeds the output bound",
            ));
        }
        Ok(())
    }
}
