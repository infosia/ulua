//! runtime integration tests share one executable.
// Each former target keeps its helper scopes and test namespace.
#![allow(clippy::duplicate_mod)]
mod analyzer_status;
mod examples;
mod optional_main;
mod p55_interrupts;
mod p55_strict;
mod p6_builtin_aliases;
mod p6_index_edges;
mod s4_bytecode;
mod s4_review;
mod s5_native;
mod s8_runtime;
mod sanitize;

fn measurement_lock() -> std::sync::MutexGuard<'static, ()> {
    static MEASUREMENT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    MEASUREMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}
