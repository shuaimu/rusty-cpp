// Plain-build lint allowances: imports/variables flagged unused here are used
// by ghost/proof code that plain `cargo build` (no Verus cfg) cannot see.
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]

// The shared executor event-log vocabulary (event enums, log predicates,
// fifo-queue model, invariant templates, bounded async contracts), consumed by
// lion-executor (impl-side invariant proofs) and lion-liveness (composed proof).
pub mod types;
pub mod events;
pub mod log;
pub mod fifo_queue;
pub mod injection_schedule;
pub mod invariants;
pub mod contracts;
