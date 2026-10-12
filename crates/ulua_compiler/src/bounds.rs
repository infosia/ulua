//! Compiler budgets. The opt-in `s3_measure` example reproduces the measurements.

/// Parameters per declaration; default parameters also require one local copy each.
pub const PARAMETER_BOUND: usize = 32;
/// Arguments per call. The local budget leaves room for the outgoing argument registers.
pub const ARGUMENT_BOUND: usize = 32;
/// Literal parts plus substitutions per template, including empty literal parts.
pub const TEMPLATE_PART_BOUND: usize = 1025;
/// Non-spill locals per function, including library locals and default-parameter copies.
pub const TEMPORARY_BOUND: usize = 136;
/// Locals per function before bindings spill; loop locals count by their live maximum.
/// The measured native ceiling is 200. Reserve 24 locals and 79 registers.
pub const LOCAL_BUDGET: usize = 176;
/// Distinct tested references per true or false set of a boolean binding initializer.
pub const ALIAS_TEST_BOUND: usize = 8;

/// Upvalues per generated function, including relayed captures and library locals.
/// The pinned compiler accepts 200; reserve eight upvalues below that ceiling.
pub const UPVALUE_BUDGET: usize = 192;
