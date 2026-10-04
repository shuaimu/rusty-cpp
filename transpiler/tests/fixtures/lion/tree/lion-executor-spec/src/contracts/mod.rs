// STATUS (audit): the five AsyncContract *objects* built in these
// modules (`bounded_*_poll(..)`) are paper-facing packaging of each contract's
// trigger/response/assumption triple; no proof attests them as records. The
// liveness derivations consume the component spec fns (trigger_fn /
// response_fn / assumption_fn) directly. The assumption_fn (tid_unique) is
// likewise carried as a documentation conjunct in lion-liveness's env core.
pub mod bounded_injection_poll;
pub mod bounded_reactor_wake_poll;
pub mod bounded_task_wake_poll;
pub mod bounded_deferred_poll;
pub mod bounded_drain_poll;
