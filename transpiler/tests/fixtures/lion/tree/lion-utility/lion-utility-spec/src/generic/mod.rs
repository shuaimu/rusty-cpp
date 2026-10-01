// Generic (per-method/return-type) utility verification template, aligned with
// the paper (§5.1) and the LION-RE design draft. Standalone: depends only on
// the framework + reactor view types. The composing layer's existing monotype
// utility model is unaffected; this module is the reusable target a concrete
// utility instance (Sleep / Notify / Mutex / …) verifies against.

pub mod types;
pub mod events;
pub mod log;
pub mod module_spec;
pub mod invariants;
pub mod contract;
pub mod extension;
