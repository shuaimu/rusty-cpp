use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::local_liveness::*;

verus! {

// L2a/L2b (liveness) / R17a/R17b (impl): WAKE_ON_IO_READY
//
// When an I/O event becomes ready with readable/writable flag, and there is a
// valid SetWaker of that direction for the resource, a WakeTask must occur
// before the next Park::End or PollEvents.
//
// IO ANCHOR DUALITY (F-K): the liveness-side definitions below use the SYSCALL
// anchor (has_valid_set_waker_*_syscall, io_syscall_active_at_set_waker,
// io_syscall_deregistered_at); lion-reactor's inlined R17a/R17b use the API
// anchor (has_valid_set_waker_*_api in log) and the choose-based
// find_last_set_waker_for_rid_readable/writable (also in log). The liveness
// side uses the constructive _rec searches below.

pub open spec fn find_last_set_waker_for_rid_readable_rec(l: Log, rid: ResourceIdView, before: int) -> int
  decreases before
{
  if before <= 0 {
    -1
  } else if is_succ_set_waker_at(l, before - 1) &&
    get_set_waker_rid(l[before - 1]) == rid &&
    get_set_waker_interest(l[before - 1]).0
  {
    before - 1
  } else {
    find_last_set_waker_for_rid_readable_rec(l, rid, before - 1)
  }
}

pub open spec fn find_last_set_waker_for_rid_writable_rec(l: Log, rid: ResourceIdView, before: int) -> int
  decreases before
{
  if before <= 0 {
    -1
  } else if is_succ_set_waker_at(l, before - 1) &&
    get_set_waker_rid(l[before - 1]) == rid &&
    get_set_waker_interest(l[before - 1]).1
  {
    before - 1
  } else {
    find_last_set_waker_for_rid_writable_rec(l, rid, before - 1)
  }
}

// Path-compat re-exports: these historically lived in this module on the
// liveness side.
#[cfg(verus_keep_ghost)]
pub use crate::log::find_io_syscall_register_for_rid;
#[cfg(verus_keep_ghost)]
pub use crate::log::find_io_syscall_register_for_rid_valid;
#[cfg(verus_keep_ghost)]
pub use crate::log::find_io_syscall_register_for_rid_ge;
#[cfg(verus_keep_ghost)]
pub use crate::log::io_syscall_active_at_set_waker;

pub open spec fn has_valid_set_waker_readable_syscall(l: Log, io_ready_idx: int) -> bool
  recommends is_io_event_ready_at(l, io_ready_idx)
{
  let event = get_io_event(l[io_ready_idx]);
  let rid = event.resource_id;
  event.readable &&
  exists |sw_idx: int| 0 <= sw_idx < io_ready_idx &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_interest(l[sw_idx]).0 &&
    io_syscall_active_at_set_waker(l, rid, sw_idx) &&
    forall |k: int| sw_idx < k < io_ready_idx ==> !(
      io_syscall_deregistered_at(l, k) && get_io_syscall_deregister_rid(l[k]) == rid
    )
}

pub open spec fn has_valid_set_waker_writable_syscall(l: Log, io_ready_idx: int) -> bool
  recommends is_io_event_ready_at(l, io_ready_idx)
{
  let event = get_io_event(l[io_ready_idx]);
  let rid = event.resource_id;
  event.writable &&
  exists |sw_idx: int| 0 <= sw_idx < io_ready_idx &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_interest(l[sw_idx]).1 &&
    io_syscall_active_at_set_waker(l, rid, sw_idx) &&
    forall |k: int| sw_idx < k < io_ready_idx ==> !(
      io_syscall_deregistered_at(l, k) && get_io_syscall_deregister_rid(l[k]) == rid
    )
}

// L2a: readable variant
pub open spec fn trigger_fn_readable(l: Log, i: int) -> bool {
  is_io_event_ready_at(l, i) &&
  has_valid_set_waker_readable_syscall(l, i)
}

pub open spec fn response_fn_readable(l: Log, trigger_idx: int, j: int) -> bool {
  let event = get_io_event(l[trigger_idx]);
  let rid = event.resource_id;
  let set_waker_idx = find_last_set_waker_for_rid_readable_rec(l, rid, trigger_idx);
  let waker = get_set_waker_waker(l[set_waker_idx]);
  is_wake_task_at(l, j) &&
  get_wake_task_source_rid(l[j]) == rid &&
  get_wake_task_waker(l[j]) == waker
}

pub open spec fn timely_fn_readable(l: Log, trigger_idx: int, response_idx: int) -> bool {
  response_idx > trigger_idx &&
  !exists |k: int| #![trigger l[k]]
    trigger_idx < k < response_idx &&
    (is_park_end_at(l, k) || is_poll_events_at(l, k))
}

pub open spec fn wake_on_io_ready_readable() -> LocalLiveness<Log> {
  LocalLiveness {
    acceptance: |l: Log, i: int| trigger_fn_readable(l, i),
    fulfillment: |l: Log, i: int, j: int| response_fn_readable(l, i, j),
    timely: |l: Log, i: int, j: int| timely_fn_readable(l, i, j),
  }
}

// L2b: writable variant
pub open spec fn trigger_fn_writable(l: Log, i: int) -> bool {
  is_io_event_ready_at(l, i) &&
  has_valid_set_waker_writable_syscall(l, i)
}

pub open spec fn response_fn_writable(l: Log, trigger_idx: int, j: int) -> bool {
  let event = get_io_event(l[trigger_idx]);
  let rid = event.resource_id;
  let set_waker_idx = find_last_set_waker_for_rid_writable_rec(l, rid, trigger_idx);
  let waker = get_set_waker_waker(l[set_waker_idx]);
  is_wake_task_at(l, j) &&
  get_wake_task_source_rid(l[j]) == rid &&
  get_wake_task_waker(l[j]) == waker
}

pub open spec fn timely_fn_writable(l: Log, trigger_idx: int, response_idx: int) -> bool {
  response_idx > trigger_idx &&
  !exists |k: int| #![trigger l[k]]
    trigger_idx < k < response_idx &&
    (is_park_end_at(l, k) || is_poll_events_at(l, k))
}

pub open spec fn wake_on_io_ready_writable() -> LocalLiveness<Log> {
  LocalLiveness {
    acceptance: |l: Log, i: int| trigger_fn_writable(l, i),
    fulfillment: |l: Log, i: int, j: int| response_fn_writable(l, i, j),
    timely: |l: Log, i: int, j: int| timely_fn_writable(l, i, j),
  }
}

}
