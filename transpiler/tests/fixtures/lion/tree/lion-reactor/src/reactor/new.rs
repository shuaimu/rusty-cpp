use crate::reactor::Reactor;
use crate::resource_slab::ResourceSlab;
use lion_timer_wheel::TimerWheel;
use crate::os::OsBackend;
use crate::types::{InterruptHandle, InterruptHandleInner, IoError, IoEventQueue, IoResult, Poll};
use crate::invariants::*;
use crate::invariants::data_inv::*;
use crate::spec::predicates::*;
use vstd::prelude::*;

// Builds the OS handles around a backend: the backend itself, the event buffer
// its waits fill (capacity 1024, as the mio event batch always had) and the
// cross-thread interrupt handle over the backend's interrupt.
fn os_handles(backend: Box<dyn OsBackend>) -> (Poll, IoEventQueue, InterruptHandle) {
  let interrupt_handle = InterruptHandle {
    inner: InterruptHandleInner::new(backend.interrupt()),
  };
  (
    Poll { inner: backend },
    IoEventQueue { inner: Vec::with_capacity(1024) },
    interrupt_handle,
  )
}

verus! {

impl Reactor {
  // The only trust in construction: creating the OS handles. No semantic
  // ensures — an opaque effect. Everything downstream of these handles is
  // assembled and PROVEN in `assemble` below.
  //
  // With the default `mio` feature: Lion's mio (epoll) backend.
  #[cfg(feature = "mio")]
  #[verifier::external_body]
  fn mio_setup() -> (result: IoResult<(Poll, IoEventQueue, InterruptHandle)>)
  {
    match crate::os::MioBackend::new() {
      Ok(backend) => IoResult::Ok(os_handles(Box::new(backend))),
      Err(e) => IoResult::Err(IoError { inner: e }),
    }
  }

  // An embedder's backend (see crate::os for the contract it must meet),
  // already wrapped in the opaque `Poll` (Verus does not support `dyn` types
  // in verified signatures; `with_backend` below does the wrapping).
  #[verifier::external_body]
  fn backend_setup(poll: Poll) -> (result: (Poll, IoEventQueue, InterruptHandle))
  {
    os_handles(poll.inner)
  }

  // A reactor over Lion's mio backend (default feature `mio`).
  #[cfg(feature = "mio")]
  pub fn new() -> (result: IoResult<(Self, InterruptHandle)>)
    ensures
      match result {
        IoResult::Ok((reactor, _)) => {
          reactor_wf(reactor.log@, reactor.next_resource_id as nat) &&
          timer_impl_inv(reactor.resources.timer_set_view(), reactor.resources.timer_map_view(), reactor.next_resource_id as nat) &&
          slab_alloc_inv(reactor.resources@, reactor.next_resource_id as nat) &&
          free_rids_wf(reactor.free_rids@, reactor.log@, reactor.resources@, reactor.next_resource_id as nat) &&
          data_inv(
            reactor.resources.timer_set_view(), reactor.resources.timer_map_view(),
            reactor.resources.timer_wakers_view(), reactor.resources.read_wakers_view(), reactor.resources.write_wakers_view(),
            reactor.log@,
          ) &&
          wheel_slab_consistent(reactor.wheel@, reactor.resources.timer_map_view()) &&
          reactor.wheel.full_wf() &&
          reactor.resources.slab_wf()
        },
        IoResult::Err(_) => true,
      }
  {
    let handles = match Self::mio_setup() {
      IoResult::Ok(t) => t,
      IoResult::Err(e) => return IoResult::Err(e),
    };
    let (poll, events, interrupt_handle) = handles;
    IoResult::Ok(Self::assemble(poll, events, interrupt_handle))
  }

  // A reactor over the OS backend in `poll` (`Poll::new(backend)`; or use
  // `with_backend`). Same guarantees as `new`.
  pub fn with_poll(poll: Poll) -> (result: IoResult<(Self, InterruptHandle)>)
    ensures
      match result {
        IoResult::Ok((reactor, _)) => {
          reactor_wf(reactor.log@, reactor.next_resource_id as nat) &&
          timer_impl_inv(reactor.resources.timer_set_view(), reactor.resources.timer_map_view(), reactor.next_resource_id as nat) &&
          slab_alloc_inv(reactor.resources@, reactor.next_resource_id as nat) &&
          free_rids_wf(reactor.free_rids@, reactor.log@, reactor.resources@, reactor.next_resource_id as nat) &&
          data_inv(
            reactor.resources.timer_set_view(), reactor.resources.timer_map_view(),
            reactor.resources.timer_wakers_view(), reactor.resources.read_wakers_view(), reactor.resources.write_wakers_view(),
            reactor.log@,
          ) &&
          wheel_slab_consistent(reactor.wheel@, reactor.resources.timer_map_view()) &&
          reactor.wheel.full_wf() &&
          reactor.resources.slab_wf()
        },
        IoResult::Err(_) => true,
      }
  {
    let (poll, events, interrupt_handle) = Self::backend_setup(poll);
    IoResult::Ok(Self::assemble(poll, events, interrupt_handle))
  }

  // VERIFIED base case: all 8 initialization invariants are PROVEN of the
  // freshly assembled reactor (empty ghost log, empty wheel/slab, next_rid = 1)
  // — formerly the whole constructor was external_body and the ensures were
  // assumed wholesale (the old "largest single trust point").
  fn assemble(poll: Poll, events: IoEventQueue, interrupt_handle: InterruptHandle) -> (result: (Self, InterruptHandle))
    ensures
      ({
        let reactor = result.0;
        reactor_wf(reactor.log@, reactor.next_resource_id as nat) &&
        timer_impl_inv(reactor.resources.timer_set_view(), reactor.resources.timer_map_view(), reactor.next_resource_id as nat) &&
        slab_alloc_inv(reactor.resources@, reactor.next_resource_id as nat) &&
        free_rids_wf(reactor.free_rids@, reactor.log@, reactor.resources@, reactor.next_resource_id as nat) &&
        data_inv(
          reactor.resources.timer_set_view(), reactor.resources.timer_map_view(),
          reactor.resources.timer_wakers_view(), reactor.resources.read_wakers_view(), reactor.resources.write_wakers_view(),
          reactor.log@,
        ) &&
        wheel_slab_consistent(reactor.wheel@, reactor.resources.timer_map_view()) &&
        reactor.wheel.full_wf() &&
        reactor.resources.slab_wf()
      }),
  {
    let wheel = TimerWheel::new();
    let resources = ResourceSlab::new();

    let reactor = Reactor {
      next_resource_id: 1,
      poll,
      events,
      wheel,
      resources,
      log: Ghost(Seq::empty()),
      free_rids: Vec::new(),
      pending_deregister: None,
    };

    proof {
      let l = reactor.log@;
      let nrid = reactor.next_resource_id as nat;
      assert(l.len() == 0);
      assert(reactor.free_rids@ =~= Seq::<u64>::empty());

      // reactor_wf: reactor_inv + reactor_ext_inv + alloc_inv — every clause is
      // an index-guarded forall over the (empty) log, hence vacuous; plus
      // next_rid = 1 >= 1.
      assert(reactor_wf(l, nrid));

      // timer_impl_inv over the empty timer set/map (ResourceSlab::new ensures).
      assert(reactor.resources.timer_set_view().finite());
      assert(timer_impl_inv(reactor.resources.timer_set_view(), reactor.resources.timer_map_view(), nrid));

      // slab_alloc_inv over the empty slab view.
      assert(reactor.resources@ =~= Map::<nat, crate::spec::types::ResourceSlotView>::empty());
      assert(slab_alloc_inv(reactor.resources@, nrid));

      // free_rids_wf is closed — discharged by its dedicated empty lemma.
      free_rids_wf_empty(l, reactor.resources@, nrid);

      // data_inv over empty views and the empty log.
      assert(data_inv(
        reactor.resources.timer_set_view(), reactor.resources.timer_map_view(),
        reactor.resources.timer_wakers_view(), reactor.resources.read_wakers_view(), reactor.resources.write_wakers_view(),
        l,
      ));

      // wheel_slab_consistent: both maps are empty (TimerWheel::new /
      // ResourceSlab::new ensures).
      assert(wheel_slab_consistent(reactor.wheel@, reactor.resources.timer_map_view()));
    }

    (reactor, interrupt_handle)
  }
}

}

impl Reactor {
  /// A reactor over an embedder's OS backend (see `crate::os` for the contract
  /// it must meet). The verified constructor is `with_poll`; this only wraps
  /// the backend for it.
  pub fn with_backend(backend: Box<dyn OsBackend>) -> IoResult<(Self, InterruptHandle)> {
    Self::with_poll(Poll::new(backend))
  }
}
