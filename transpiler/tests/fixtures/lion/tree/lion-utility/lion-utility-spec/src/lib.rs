// Plain-build lint allowances: imports/variables flagged unused here are used
// by ghost/proof code that plain `cargo build` (no Verus cfg) cannot see.
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]

// Shared verification template for Lion utilities.
//
// This crate is the single source of truth for the utility-layer verification
// vocabulary: the contract/invariant framework (Listing 2-4), the reactor-side
// view types, and the generic (M,R) per-utility template. Both lion-liveness
// (the composition proof) and the concrete utility crates (lion-utility) depend
// on it, so the invariants a utility discharges are literally the ones the
// composition consumes — no second model, no drift.

pub mod view_types;
pub mod framework;
pub mod generic;
