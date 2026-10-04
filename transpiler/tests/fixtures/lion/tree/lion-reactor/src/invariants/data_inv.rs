use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::spec::predicates::*;

verus! {

// timer_awaiting_wake moved to the shared crate (identical definition was
// duplicated on the liveness side); re-exported to keep the data_inv:: path.
#[cfg(verus_keep_ghost)]
pub use lion_reactor_spec::invariants::wake_on_expired::timer_awaiting_wake;

pub open spec fn timer_heap_entries_valid(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  log: Log,
) -> bool {
  forall |d: InstantView, rid: ResourceIdView, log_idx: int|
    #![auto] timers_view.contains((d, rid, log_idx)) ==> {
      timer_awaiting_wake(log, log_idx) &&
      get_register_timer_rid(log[log_idx]) == rid &&
      get_register_timer_deadline(log[log_idx]) == d
    }
}

pub open spec fn active_timers_in_heap(
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  log: Log,
) -> bool {
  forall |log_idx: int| #![auto] timer_awaiting_wake(log, log_idx) ==> {
    let rid = get_register_timer_rid(log[log_idx]);
    by_rid_view.contains_key(rid) &&
    by_rid_view[rid].2 == log_idx
  }
}

pub open spec fn timer_wakers_match(
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  log: Log,
) -> bool {
  forall |rid: ResourceIdView| #![auto]
    timer_wakers_view.contains_key(rid) ==> {
      by_rid_view.contains_key(rid) && {
        let log_idx = by_rid_view[rid].2;
        0 <= log_idx < log.len() &&
        is_succ_register_timer_at(log, log_idx) &&
        get_register_timer_rid(log[log_idx]) == rid &&
        timer_wakers_view[rid] == get_register_timer_waker(log[log_idx])
      }
    }
}

pub open spec fn timer_heap_has_wakers(
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
) -> bool {
  forall |rid: ResourceIdView| #![auto]
    by_rid_view.contains_key(rid) ==> timer_wakers_view.contains_key(rid)
}

pub open spec fn read_wakers_valid(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
) -> bool {
  forall |rid: ResourceIdView| #![auto]
    read_wakers_view.contains_key(rid) ==>
    io_currently_active(log, rid) &&
    exists |sw_idx: int| 0 <= sw_idx < log.len() &&
      is_succ_set_waker_at(log, sw_idx) &&
      get_set_waker_rid(log[sw_idx]) == rid &&
      get_set_waker_interest(log[sw_idx]).0 &&
      get_set_waker_waker(log[sw_idx]) == read_wakers_view[rid] &&
      io_api_active_at_set_waker(log, rid, sw_idx) &&
      forall |k: int| sw_idx < k < log.len() ==> !(
        is_succ_set_waker_at(log, k) &&
        get_set_waker_rid(log[k]) == rid &&
        get_set_waker_interest(log[k]).0
      )
}

pub open spec fn write_wakers_valid(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
) -> bool {
  forall |rid: ResourceIdView| #![auto]
    write_wakers_view.contains_key(rid) ==>
    io_currently_active(log, rid) &&
    exists |sw_idx: int| 0 <= sw_idx < log.len() &&
      is_succ_set_waker_at(log, sw_idx) &&
      get_set_waker_rid(log[sw_idx]) == rid &&
      get_set_waker_interest(log[sw_idx]).1 &&
      get_set_waker_waker(log[sw_idx]) == write_wakers_view[rid] &&
      io_api_active_at_set_waker(log, rid, sw_idx) &&
      forall |k: int| sw_idx < k < log.len() ==> !(
        is_succ_set_waker_at(log, k) &&
        get_set_waker_rid(log[k]) == rid &&
        get_set_waker_interest(log[k]).1
      )
}

pub open spec fn io_currently_active(log: Log, rid: ResourceIdView) -> bool {
  exists |reg_idx: int| 0 <= reg_idx < log.len() &&
    io_api_registered_at(log, reg_idx) &&
    get_io_api_register_rid(log[reg_idx]) == rid &&
    io_api_active_at(log, reg_idx, log.len() as int)
}

pub open spec fn has_active_readable_set_waker(log: Log, rid: ResourceIdView) -> bool {
  exists |reg_idx: int| 0 <= reg_idx < log.len() &&
    io_api_registered_at(log, reg_idx) &&
    get_io_api_register_rid(log[reg_idx]) == rid &&
    io_api_active_at(log, reg_idx, log.len() as int) &&
    exists |sw_idx: int| reg_idx < sw_idx < log.len() &&
      is_succ_set_waker_at(log, sw_idx) &&
      get_set_waker_rid(log[sw_idx]) == rid &&
      get_set_waker_interest(log[sw_idx]).0
}

pub open spec fn has_active_writable_set_waker(log: Log, rid: ResourceIdView) -> bool {
  exists |reg_idx: int| 0 <= reg_idx < log.len() &&
    io_api_registered_at(log, reg_idx) &&
    get_io_api_register_rid(log[reg_idx]) == rid &&
    io_api_active_at(log, reg_idx, log.len() as int) &&
    exists |sw_idx: int| reg_idx < sw_idx < log.len() &&
      is_succ_set_waker_at(log, sw_idx) &&
      get_set_waker_rid(log[sw_idx]) == rid &&
      get_set_waker_interest(log[sw_idx]).1
}

pub open spec fn read_wakers_complete(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
) -> bool {
  forall |rid: ResourceIdView| #![auto]
    has_active_readable_set_waker(log, rid) ==>
    read_wakers_view.contains_key(rid)
}

pub open spec fn write_wakers_complete(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
) -> bool {
  forall |rid: ResourceIdView| #![auto]
    has_active_writable_set_waker(log, rid) ==>
    write_wakers_view.contains_key(rid)
}

pub open spec fn data_inv(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  read_wakers_view: Map<ResourceIdView, WakerView>,
  write_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
) -> bool {
  timer_heap_entries_valid(timers_view, log) &&
  active_timers_in_heap(by_rid_view, log) &&
  timer_wakers_match(timer_wakers_view, by_rid_view, log) &&
  timer_heap_has_wakers(timer_wakers_view, by_rid_view) &&
  read_wakers_valid(read_wakers_view, log) &&
  write_wakers_valid(write_wakers_view, log) &&
  read_wakers_complete(read_wakers_view, log) &&
  write_wakers_complete(write_wakers_view, log)
}

}
