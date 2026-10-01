// Invariant templates over the executor log (paper Listing 4): each executor
// property is an instance of ActionSafety<Log> or LocalLiveness<Log>.
// NOTE: the aggregator spec fns (structural/semantic on the impl side,
// safety/liveness on the lion-liveness side) stay LOCAL to each consumer —
// their groupings and memberships differ (poll_within_tick is impl-only).
pub mod park_drain_reactor_wake;
pub mod tick_polls_if_runnable;
pub mod poll_within_tick;
pub mod tick_has_park;
pub mod tick_has_pop_injection;
pub mod tick_has_drain_deferred;
pub mod tick_has_drain_task_wake;
pub mod fifo_task_selection;
pub mod valid_task_polling;
