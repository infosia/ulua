//! Embedded Ulua runtime and the version 1 C ABI.
#![warn(missing_docs)]

mod abi;
mod capabilities;
mod declarations;
mod host_names;
mod native;
mod objects;
#[cfg(test)]
mod p3_tests;
#[cfg(test)]
mod p41_tests;
#[cfg(test)]
mod tests;
mod types;

pub use abi::*;
pub use capabilities::{
    Capability, ulua_capability_free, ulua_capability_new, ulua_capability_revoke,
};
pub use declarations::ulua_declarations;
pub use objects::{Context, Instance, Module};
pub use types::*;
