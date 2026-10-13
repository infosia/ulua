//! runtime integration tests share one executable.
// Each former target keeps its helper scopes and test namespace.
#![allow(clippy::duplicate_mod)]
mod analyzer_status;
mod s4_bytecode;
mod s4_growth;
mod s4_review;
mod s5_native;
mod s8_runtime;
