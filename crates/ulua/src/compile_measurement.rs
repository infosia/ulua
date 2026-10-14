//! Optional observations of the real ABI pipeline for timing witnesses.
use std::cell::Cell;

/// A boundary of the compiler worker. The observer borrows the emitted source.
#[non_exhaustive]
pub enum CompileEvent<'a> {
    /// The worker starts the Rust compiler.
    Start,
    /// The Rust compiler finishes. Native compilation starts after this observation.
    Rust(&'a str),
    /// The Rust compiler rejects the input. Native compilation does not run.
    Rejected,
    /// Native compilation finishes. The ABI still must copy the bytecode.
    Native,
}

/// A synchronous observer. It must not panic or retain borrowed source data.
pub type Observer = fn(CompileEvent<'_>);

thread_local! {
    static OBSERVER: Cell<Option<Observer>> = const { Cell::new(None) };
}

/// Replace the observer for ABI calls on this thread. Return the prior observer.
/// The ABI copies this function pointer to its worker without a heap allocation.
pub fn replace_observer(observer: Option<Observer>) -> Option<Observer> {
    OBSERVER.with(|current| current.replace(observer))
}

pub(crate) fn observer() -> Option<Observer> {
    OBSERVER.with(Cell::get)
}

pub(crate) fn notify(observer: Option<Observer>, event: CompileEvent<'_>) {
    if let Some(observer) = observer {
        observer(event);
    }
}
