//! p2 integration tests share one executable.
// Each former target keeps its helper scopes and test namespace.
#![allow(clippy::duplicate_mod)]
mod d17_equality;
mod d18_inference;
mod p21_functions;
mod p22_arrays;
mod p23_methods;
mod p24_strings;
mod p25_alias_links;
mod p25_coalesce_edges;
mod p25_edges;
mod p25_field_limits;
mod p25_field_updates;
mod p25_growth;
mod p25_nested_narrowing;
mod p25_store_order;
mod p25_test_positions;
mod p25_type_limits;
mod p25_unreachable;
mod p25_values;
mod p26_classes;
mod p26_cycles;
mod p26_growth;
mod p26_limits;
mod p26_random;
mod p26_rejects;
mod p27_exceptions;
mod p27_growth;
mod p27_jumps;
mod p27_narrowing;
mod p27_random;
mod p27_stops;
mod p27_unreachable;
mod p27_upvalues;
mod protected_loop_completions;
mod protected_loop_growth;
