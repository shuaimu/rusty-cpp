// Shared invariant templates: the per-obligation trigger/validity (and, for
// liveness obligations, trigger/response/timely) spec fns plus the
// ActionSafety/LocalLiveness records consumed by lion-liveness's aggregator.
// The aggregators themselves stay in the consuming crates: lion-liveness keeps
// its combinator-form reactor_inv; lion-reactor keeps its inlined
// reactor_inv/reactor_ext_inv (spelled with these shared predicates).
pub mod timer_deadline_future;
pub mod park_has_timestamp;
pub mod park_poll_once;
pub mod io_ready_in_park;
pub mod timer_waker_validity;
pub mod io_waker_validity;
pub mod timer_reg_uniqueness;
pub mod io_reg_uniqueness;
pub mod timer_io_disjoint;
pub mod register_io_in_cycle;
pub mod deregister_io_in_cycle;
pub mod inbound_register_io_result;
pub mod inbound_deregister_io_result;
pub mod wake_has_registration;
pub mod set_waker_active_io;
pub mod wake_on_expired;
pub mod wake_on_io_ready;
