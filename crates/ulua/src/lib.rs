//! Embedded Ulua runtime and the version 1 C ABI.
#![warn(missing_docs)]

mod abi;
mod capabilities;
#[cfg(feature = "timing")]
#[doc(hidden)]
pub mod compile_measurement;
mod declarations;
#[cfg(feature = "fault-injection")]
pub mod fault_injection;
mod host_names;
mod math;
#[cfg(test)]
mod math_tests;
mod native;
mod number;
mod objects;
#[cfg(test)]
mod p3_tests;
#[cfg(test)]
mod p41_tests;
#[cfg(test)]
mod p43_tests;
mod requests;
#[cfg(test)]
mod tests;
mod types;

pub use abi::*;
pub use capabilities::{
    Capability, ulua_capability_free, ulua_capability_new, ulua_capability_revoke,
};
pub use declarations::ulua_declarations;
pub use math::ulua_instance_seed_random;
pub use objects::{Context, Instance, Module};
pub use requests::*;
pub use types::*;
