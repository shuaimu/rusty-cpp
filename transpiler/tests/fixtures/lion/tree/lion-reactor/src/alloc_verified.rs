use crate::reactor::Reactor;
use crate::types::{IoError, IoResult, ResourceId};
use crate::spec::predicates::*;
use vstd::prelude::*;

verus! {

impl Reactor {
  // NOTE: the reactor does NOT recycle resource ids. `next_resource_id` is strictly
  // monotonic and the free-list reuse path is disabled ([RID-REUSE DISABLED] markers),
  // so each rid is handed out at most once — the freshness facts below rest on that.
  // Memory does not grow with the rids ever allocated: every rid-keyed container is a
  // map holding LIVE entries only (the ResourceSlab's `lion_slab::Slab` and the timer
  // wheel's `deadlines`/`positions` are std `HashMap`s; the trusted `readiness` table
  // drops a rid's entry on deregistration), so memory is proportional to the live
  // resources. The id space is u64 per reactor: reaching `u64::MAX` returns
  // `resource_id_overflow` below rather than wrapping.
  #[verifier::rlimit(50)]
  pub fn alloc_resource_id(&mut self) -> (result: IoResult<ResourceId>)
    requires
      alloc_inv(old(self).log@, old(self).next_resource_id as nat),
      slab_alloc_inv(old(self).resources@, old(self).next_resource_id as nat),
      free_rids_wf(old(self).free_rids@, old(self).log@, old(self).resources@, old(self).next_resource_id as nat),
      forall |j: int| #![trigger is_wake_task_at(old(self).log@, j)] 0 <= j < old(self).log@.len() && is_wake_task_at(old(self).log@, j) ==>
        get_wake_task_source_rid(old(self).log@[j]) < old(self).next_resource_id as nat,
    ensures
      self.log == old(self).log,
      self.wheel == old(self).wheel,
      self.resources == old(self).resources,
      self.next_resource_id >= old(self).next_resource_id,
      alloc_inv(self.log@, self.next_resource_id as nat),
      free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
      match result {
        IoResult::Ok(id) => {
          &&& id@ >= 1
          &&& id@ < self.next_resource_id as nat
          &&& !self.resources@.contains_key(id@)
          &&& no_prior_timer_registration(self.log@, id@, self.log@.len() as int)
          &&& no_prior_io_api_registration(self.log@, id@, self.log@.len() as int)
          &&& no_timer_with_rid_before(self.log@, id@, self.log@.len() as int)
          &&& no_io_api_with_rid_before(self.log@, id@, self.log@.len() as int)
          &&& forall |i: int| #![trigger self.free_rids@[i]] 0 <= i < self.free_rids@.len() ==> self.free_rids@[i] as nat != id@
        },
        IoResult::Err(_) => {
          old(self).next_resource_id == u64::MAX &&
          self.next_resource_id == old(self).next_resource_id &&
          self.free_rids@ == old(self).free_rids@
        }
      }
  {
    // [RID-REUSE DISABLED — restore when generational ids are added]
    // if let Some(rid) = self.free_rids.pop() {
    //   proof {
    //     free_rids_wf_pop(
    //       old(self).free_rids@,
    //       old(self).log@,
    //       old(self).resources@,
    //       old(self).next_resource_id as nat,
    //     );
    //     let ghost remaining = old(self).free_rids@.drop_last();
    //     assert(self.free_rids@ =~= remaining);
    //     assert forall |i: int| #![trigger self.free_rids@[i]]
    //       0 <= i < self.free_rids@.len() implies self.free_rids@[i] as nat != rid as nat
    //     by {
    //       assert(remaining[i] as nat != rid as nat);
    //     }
    //   }
    //   return IoResult::Ok(ResourceId(rid));
    // }
    if self.next_resource_id >= u64::MAX {
      IoResult::Err(IoError::resource_id_overflow())
    } else {
      let id = self.next_resource_id;
      self.next_resource_id = self.next_resource_id + 1;
      proof {
        let rid = id as nat;
        no_prior_timer_reg_vacuous(self.log@, rid, old(self).next_resource_id as nat);
        no_prior_io_api_reg_vacuous(self.log@, rid, old(self).next_resource_id as nat);
        assert(!self.resources@.contains_key(rid)) by {
          if self.resources@.contains_key(rid) {
            assert(rid < old(self).next_resource_id as nat);
          }
        }
        // [RID-REUSE DISABLED — restore when generational ids are added]
        // Reuse pop removed: free_rids is no longer known empty here. The free_rids_wf
        // ensures and the id-freshness clause both follow from the incoming free_rids_wf
        // under a monotonic next_rid increase (every free_rids[i] < old next_rid <= id).
        free_rids_wf_preserved_by_next_rid_increase(
          self.free_rids@,
          self.log@,
          self.resources@,
          old(self).next_resource_id as nat,
          self.next_resource_id as nat,
        );
      }
      IoResult::Ok(ResourceId(id))
    }
  }
}

}
