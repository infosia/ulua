//! p4 integration tests share one executable.
// Each former target keeps its helper scopes and test namespace.
#![allow(clippy::duplicate_mod)]
mod p41_async;
mod p41_growth;
mod p41_random;
mod p41_review;
mod p41_stops;
mod p41_then;
mod p42_nullable;
mod p42_promises;

mod p43_order;
mod p52_findings;
