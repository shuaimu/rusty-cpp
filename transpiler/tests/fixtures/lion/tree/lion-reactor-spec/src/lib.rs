// Plain-build lint allowances: imports/variables flagged unused here are used
// by ghost/proof code that plain `cargo build` (no Verus cfg) cannot see.
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]

// The shared reactor event-log vocabulary (event enums, log predicates, state
// tracking, invariant templates, bounded async contracts, and the io anchor
// bridge), consumed by lion-reactor (impl-side invariant proofs) and
// lion-liveness (composed proof).
//
// IO ANCHOR DUALITY (a deliberate duality): the two consumers anchor "an IO
// resource is registered" on DIFFERENT events, and both anchors are kept here
// side by side under explicit names:
//   - io_api_*     — Inbound anchor (API success: Inbound RegisterIoResource
//                    End with Ok(rid)); this is lion-reactor's convention.
//   - io_syscall_* — Outbound anchor (syscall success: Outbound
//                    RegisterIoResource with Ok(())); lion-liveness's
//                    convention.
// The two are connected per registration cycle by the proven bridge in
// `bridge` (derived from the R12/R13 inbound-result obligations).
pub mod types;
pub mod events;
pub mod log;
pub mod invariants;
pub mod contracts;
pub mod bridge;
