use vstd::prelude::*;
use crate::helpers::*;
use crate::vec_map::VecMap;

verus! {

broadcast use vstd::std_specs::hash::group_hash_axioms;

pub const WHEEL_BITS: u32 = 8;
pub const WHEEL_SIZE: usize = 256;
pub const NUM_LEVELS: usize = 4;

#[derive(Clone, Copy)]
pub struct WheelPos {
  pub level: u8,
  pub slot: u8,
  pub idx: u64,
}

impl View for WheelPos {
  type V = WheelPos;
  #[verifier::external_body]
  open spec fn view(&self) -> WheelPos { *self }
}

pub open spec fn slot_in_drained_range(s: int, old_slot: int, num_drained: int) -> bool {
  if num_drained > WHEEL_SIZE as int {
    true
  } else {
    let offset = (s - old_slot + WHEEL_SIZE as int) % (WHEEL_SIZE as int);
    1 <= offset && offset < num_drained
  }
}

pub struct TimerWheel {
  pub levels: Vec<Vec<Vec<u64>>>,
  pub elapsed: u64,
  pub pending: Vec<u64>,
  pub cached_min: Option<u64>,
  pub deadlines: VecMap<u64>,
  pub positions: VecMap<WheelPos>,
  pub level_counts: Vec<u64>,
}

impl View for TimerWheel {
  type V = Map<nat, int>;

  open spec fn view(&self) -> Map<nat, int> {
    Map::new(
      |k: nat| k <= u64::MAX as nat && self.deadlines@.contains_key(k as u64),
      |k: nat| self.deadlines@[k as u64] as int,
    )
  }
}

impl TimerWheel {

  // ---- spec functions ----

  pub open spec fn spec_level_slot(deadline: u64, elapsed: u64) -> (int, int) {
    let delta = (deadline - elapsed) as u64;
    if delta < 256 {
      (0int, ((deadline >> 0u32) & 0xff) as int)
    } else if delta < 256 * 256 {
      (1int, ((deadline >> 8u32) & 0xff) as int)
    } else if delta < 256 * 256 * 256 {
      (2int, ((deadline >> 16u32) & 0xff) as int)
    } else {
      (3int, ((deadline >> 24u32) & 0xff) as int)
    }
  }

  // Sum of the first `upto` slot lengths of one level.
  pub closed spec fn slots_len_sum(slots: Seq<Vec<u64>>, upto: int) -> int
    decreases upto
  {
    if upto <= 0 { 0 } else { Self::slots_len_sum(slots, upto - 1) + slots[upto - 1]@.len() }
  }

  pub proof fn slots_len_sum_zero(slots: Seq<Vec<u64>>, upto: int)
    requires forall |s: int| 0 <= s < upto ==> (#[trigger] slots[s])@.len() == 0,
    ensures Self::slots_len_sum(slots, upto) == 0,
    decreases upto
  {
    reveal(TimerWheel::slots_len_sum);
    if upto > 0 { Self::slots_len_sum_zero(slots, upto - 1); }
  }

  // Two slot rows equal except at index `s`: sums differ by the length delta.
  pub proof fn slots_len_sum_update(s1: Seq<Vec<u64>>, s2: Seq<Vec<u64>>, s: int, upto: int)
    requires
      0 <= s,
      forall |j: int| 0 <= j < upto && j != s ==> (#[trigger] s2[j])@ == (#[trigger] s1[j])@,
    ensures Self::slots_len_sum(s2, upto) ==
      Self::slots_len_sum(s1, upto) +
      (if s < upto { s2[s]@.len() - s1[s]@.len() } else { 0int }),
    decreases upto
  {
    reveal(TimerWheel::slots_len_sum);
    if upto > 0 { Self::slots_len_sum_update(s1, s2, s, upto - 1); }
  }

  // Pointwise-equal slot rows have equal sums.
  pub proof fn slots_len_sum_congruence(s1: Seq<Vec<u64>>, s2: Seq<Vec<u64>>, upto: int)
    requires forall |j: int| 0 <= j < upto ==> (#[trigger] s2[j])@ == (#[trigger] s1[j])@,
    ensures Self::slots_len_sum(s2, upto) == Self::slots_len_sum(s1, upto),
    decreases upto
  {
    reveal(TimerWheel::slots_len_sum);
    if upto > 0 { Self::slots_len_sum_congruence(s1, s2, upto - 1); }
  }

  // counts_wf transfers between states with equal counters and pointwise-equal
  // slot rows (the other fields may differ).
  pub proof fn level_counts_wf_transfer(w1: TimerWheel, w2: TimerWheel)
    requires
      w1.level_counts_wf(),
      w2.level_counts@ == w1.level_counts@,
      forall |l: int, sl: int| 0 <= l < NUM_LEVELS as int && 0 <= sl < WHEEL_SIZE as int ==>
        (#[trigger] w2.levels@[l]@[sl])@ == (#[trigger] w1.levels@[l]@[sl])@,
    ensures w2.level_counts_wf(),
  {
    assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
      (#[trigger] w2.level_counts@[l]) as int ==
      Self::slots_len_sum(w2.levels@[l]@, WHEEL_SIZE as int)
    by {
      assert forall |j: int| 0 <= j < WHEEL_SIZE as int implies
        (#[trigger] w2.levels@[l]@[j])@ == (#[trigger] w1.levels@[l]@[j])@ by {};
      Self::slots_len_sum_congruence(w1.levels@[l]@, w2.levels@[l]@, WHEEL_SIZE as int);
    };
  }

  // The sum bounds every member slot's length from above.
  pub proof fn slots_len_sum_lower(slots: Seq<Vec<u64>>, s: int, upto: int)
    requires
      0 <= s < upto,
      forall |j: int| 0 <= j < upto ==> (#[trigger] slots[j])@.len() >= 0,
    ensures Self::slots_len_sum(slots, upto) >= slots[s]@.len(),
    decreases upto
  {
    reveal(TimerWheel::slots_len_sum);
    if s < upto - 1 {
      Self::slots_len_sum_lower(slots, s, upto - 1);
      Self::slots_len_sum_nonneg(slots, upto - 1);
    } else {
      Self::slots_len_sum_nonneg(slots, upto - 1);
    }
  }

  pub proof fn slots_len_sum_nonneg(slots: Seq<Vec<u64>>, upto: int)
    ensures Self::slots_len_sum(slots, upto) >= 0,
    decreases upto
  {
    reveal(TimerWheel::slots_len_sum);
    if upto > 0 { Self::slots_len_sum_nonneg(slots, upto - 1); }
  }

  // The exec per-level counters mirror true slot occupancy (maintained by the
  // four trusted slot leaves; what lets the min-scan skip empty levels in O(1)).
  pub open spec fn level_counts_wf(&self) -> bool {
    &&& self.level_counts@.len() == NUM_LEVELS
    &&& forall |l: int| 0 <= l < NUM_LEVELS as int ==>
          (#[trigger] self.level_counts@[l]) as int ==
          Self::slots_len_sum(self.levels@[l]@, WHEEL_SIZE as int)
  }

  pub open spec fn structural_wf(&self) -> bool {
    &&& self.levels@.len() == NUM_LEVELS
    &&& forall |l: int| 0 <= l < NUM_LEVELS as int ==>
            (#[trigger] self.levels@[l])@.len() == WHEEL_SIZE as int
  }

  #[verifier::opaque]
  pub open spec fn cached_min_valid(&self) -> bool {
    match self.cached_min {
      Some(m) =>
        (exists |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == m) &&
        (forall |k: u64| #[trigger] self.deadlines@.contains_key(k) ==> m <= self.deadlines@[k]),
      None => true,
    }
  }

  pub open spec fn dl_pos_consistent(&self) -> bool {
    &&& forall |rid: u64| #[trigger] self.deadlines@.contains_key(rid) ==>
          self.positions@.contains_key(rid) || self.pending@.contains(rid)
    &&& forall |rid: u64| #[trigger] self.positions@.contains_key(rid) ==>
          self.deadlines@.contains_key(rid)
  }

  pub open spec fn pos_levels_consistent(&self) -> bool {
    &&& forall |rid: u64| self.positions@.contains_key(rid) &&
          !self.pending@.contains(rid) ==> {
            let pos = #[trigger] self.positions@[rid];
            &&& (pos.level as int) < NUM_LEVELS
            &&& (pos.slot as int) < WHEEL_SIZE
            &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
            &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid
          }
    &&& forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len() ==>
          self.positions@.contains_key(self.levels@[l]@[s]@[i])
    &&& forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len()
          && self.positions@.contains_key(self.levels@[l]@[s]@[i])
          && !self.pending@.contains(self.levels@[l]@[s]@[i])
          ==> {
            let rid = self.levels@[l]@[s]@[i];
            let pos = self.positions@[rid];
            pos.level as int == l && pos.slot as int == s && pos.idx as int == i
          }
  }

  pub open spec fn position_ahead(&self) -> bool {
    forall |rid: u64| self.positions@.contains_key(rid) &&
      !self.pending@.contains(rid) ==> {
        let pos = self.positions@[rid];
        let shift = (pos.level as u32) * WHEEL_BITS;
        #[trigger] self.deadlines@[rid] >> shift > self.elapsed >> shift
      }
  }

  pub open spec fn pending_expired(&self) -> bool {
    forall |rid: u64| self.pending@.contains(rid) &&
      #[trigger] self.deadlines@.contains_key(rid) ==>
      self.deadlines@[rid] <= self.elapsed
  }

  pub open spec fn pending_positions_disjoint(&self) -> bool {
    forall |rid: u64| self.pending@.contains(rid) ==>
      !self.positions@.contains_key(rid)
  }

  pub open spec fn slot_matches_deadline(&self) -> bool {
    forall |rid: u64| self.positions@.contains_key(rid) &&
      !self.pending@.contains(rid) ==> {
        let pos = #[trigger] self.positions@[rid];
        let shift = (pos.level as u32) * WHEEL_BITS;
        pos.slot as int == ((self.deadlines@[rid] >> shift) & 0xff) as int
      }
  }

  // Delta band: a wheel-resident timer at level L in {0,1,2} is less than
  // one full rotation of that level away (deadline - elapsed < 256^(L+1)).
  // Insertion places by delta magnitude, advancing elapsed only shrinks
  // deltas, and cascades re-place by the shrunk delta — so the band is
  // preserved. Level 3 carries NO band (deltas beyond 2^32 legally sit there
  // and re-cascade), which is why the min-scan treats level 3 exhaustively
  // while levels 0-2 admit the ring-order early exit.
  pub open spec fn level_band(&self) -> bool {
    forall |rid: u64| self.positions@.contains_key(rid) &&
      !self.pending@.contains(rid) &&
      self.deadlines@.contains_key(rid) &&
      (#[trigger] self.positions@[rid]).level < 3 ==> {
        let pos = self.positions@[rid];
        let width = ((pos.level as u32) + 1) * WHEEL_BITS;
        (self.deadlines@[rid] as int - self.elapsed as int) < ((1u64 << width) as int)
      }
  }

  pub open spec fn wf(&self) -> bool {
    &&& self.structural_wf()
    &&& self.cached_min_valid()
    &&& self.deadlines.count_wf()
    &&& self.level_counts_wf()
  }

  pub open spec fn full_wf(&self) -> bool {
    &&& self.wf()
    &&& self.dl_pos_consistent()
    &&& self.pos_levels_consistent()
    &&& self.position_ahead()
    &&& self.pending_expired()
    &&& self.pending_positions_disjoint()
    &&& self.slot_matches_deadline()
    &&& self.level_band()
  }

  // ---- public methods ----

  pub exec fn new() -> (result: Self)
    ensures
      result@ == Map::<nat, int>::empty(),
      result.wf(),
      result.full_wf(),
      result.pending@.len() == 0,
      result.elapsed == 0,
  {
    let mut levels: Vec<Vec<Vec<u64>>> = Vec::with_capacity(NUM_LEVELS);
    let mut li: usize = 0;
    while li < NUM_LEVELS
      invariant
        li <= NUM_LEVELS,
        levels@.len() == li as int,
        forall |l: int| 0 <= l < li as int ==>
          (#[trigger] levels@[l])@.len() == WHEEL_SIZE as int,
        forall |l: int, s: int| #![trigger levels@[l]@[s]]
          0 <= l < li as int && 0 <= s < WHEEL_SIZE as int ==>
          levels@[l]@[s]@.len() == 0,
      decreases NUM_LEVELS - li,
    {
      let mut level: Vec<Vec<u64>> = Vec::with_capacity(WHEEL_SIZE);
      let mut si: usize = 0;
      while si < WHEEL_SIZE
        invariant
          si <= WHEEL_SIZE,
          level@.len() == si as int,
          forall |s: int| 0 <= s < si as int ==> (#[trigger] level@[s])@.len() == 0,
        decreases WHEEL_SIZE - si,
      {
        level.push(Vec::new());
        si = si + 1;
      }
      levels.push(level);
      li = li + 1;
    }
    let mut level_counts: Vec<u64> = Vec::new();
    level_counts.push(0);
    level_counts.push(0);
    level_counts.push(0);
    level_counts.push(0);
    let result = TimerWheel {
      levels,
      elapsed: 0,
      pending: Vec::new(),
      cached_min: None,
      deadlines: VecMap::new(),
      positions: VecMap::new(),
      level_counts,
    };
    proof {
      assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
        (#[trigger] result.level_counts@[l]) as int ==
        Self::slots_len_sum(result.levels@[l]@, WHEEL_SIZE as int)
      by {
        Self::slots_len_sum_zero(result.levels@[l]@, WHEEL_SIZE as int);
      };
      assert(result.level_counts_wf());
    }
    proof {
      reveal(TimerWheel::cached_min_valid);
      assert(result.deadlines@ == Map::<u64, u64>::empty());
      assert forall |k: nat| !result@.contains_key(k) by {
        if k <= u64::MAX as nat {
          assert(!result.deadlines@.contains_key(k as u64));
        }
      };
      assert(result@ =~= Map::<nat, int>::empty());
      assert(result.positions@ == Map::<u64, WheelPos>::empty());
      assert(result.pending@.len() == 0);
    }
    result
  }

  pub exec fn insert(&mut self, rid: u64, deadline: u64)
    requires
      old(self).wf(),
      old(self).full_wf(),
      old(self).pending@.len() == 0,
      deadline > old(self).elapsed,
    ensures
      self@ == old(self)@.insert(rid as nat, deadline as int),
      self.wf(),
      self.full_wf(),
      self.pending@.len() == 0,
      self.elapsed == old(self).elapsed,
  {
    let ghost old_dl = self.deadlines@;
    // Track cached_min witness for proof
    let ghost cached_min_before_invalidate = self.cached_min;
    let ghost cached_min_witness: u64 = match self.cached_min {
      Some(m) => choose |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == m,
      None => 0u64,
    };
    if let Some(old_deadline) = self.deadlines.get(&rid) {
      self.invalidate_min(*old_deadline);
    }
    self.wheel_remove_inner(rid);
    let ghost post_wr_state = *self;
    let ghost post_wr_positions = self.positions@;
    let ghost post_wr_levels = self.levels@;
    proof {
      assert forall |rid2: u64| post_wr_positions.contains_key(rid2) implies ({
        let pos2 = #[trigger] post_wr_positions[rid2];
        &&& (pos2.level as int) < NUM_LEVELS
        &&& (pos2.slot as int) < WHEEL_SIZE
        &&& (pos2.idx as int) < post_wr_levels[pos2.level as int]@[pos2.slot as int]@.len()
        &&& post_wr_levels[pos2.level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2
      })
      by {
        assert(self.levels@ =~= post_wr_levels);
      };
    }
    self.deadlines.insert(rid, deadline);
    let (level, slot) = Self::level_slot(deadline, self.elapsed);
    let ghost pre_push_state = *self;
    let ghost mid_slot_vec = self.levels@[level as int]@[slot as int]@;
    let idx = self.levels[level][slot].len() as u64;
    let ghost idx_val = mid_slot_vec.len();
    proof { assert(idx as int == idx_val as int); }
    self.slot_push(level, slot, rid);
    self.positions.insert(rid, WheelPos { level: level as u8, slot: slot as u8, idx });
    proof {
      assert(self.levels@[level as int]@[slot as int]@ =~= mid_slot_vec.push(rid));
      assert(idx as int == idx_val as int);
      assert(mid_slot_vec.push(rid)[idx_val as int] == rid);
      assert(self.levels@[level as int]@[slot as int]@[idx as int]
        == mid_slot_vec.push(rid)[idx as int]);
      assert(self.levels@[level as int]@[slot as int]@[idx as int] == rid);
      assert forall |l: int, s: int|
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && (l != level as int || s != slot as int)
        implies self.levels@[l]@[s]@ =~= post_wr_levels[l]@[s]@
      by {};
    }
    let ghost cached_min_before_update = self.cached_min;
    match self.cached_min {
      Some(m) if deadline < m => self.cached_min = Some(deadline),
      _ => {}
    }
    proof {
      assert(self.deadlines@ == old_dl.insert(rid, deadline));
      assert forall |k: nat| self@.contains_key(k) <==>
        old(self)@.insert(rid as nat, deadline as int).contains_key(k)
      by {
        if k <= u64::MAX as nat {
          if k == rid as nat {
          } else {
            assert(self.deadlines@.contains_key(k as u64) == old_dl.contains_key(k as u64));
          }
        }
      };
      assert forall |k: nat| self@.contains_key(k) implies
        self@[k] == old(self)@.insert(rid as nat, deadline as int)[k]
      by {
        if k == rid as nat {
          assert(self.deadlines@[rid] == deadline);
        } else {
          assert(self.deadlines@[k as u64] == old_dl[k as u64]);
        }
      };
      assert(self@ =~= old(self)@.insert(rid as nat, deadline as int));

      assert(self.levels@.len() == NUM_LEVELS as int);
      assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
        (#[trigger] self.levels@[l])@.len() == WHEEL_SIZE as int
      by {
      };

      // cached_min_valid proof
      reveal(TimerWheel::cached_min_valid);
      assert(self.deadlines@.contains_key(rid) && self.deadlines@[rid] == deadline);
      match self.cached_min {
        Some(m) => {
          if m == deadline {
          } else {
            // cached_min was Some(m) before the match and wasn't changed (deadline >= m)
            // m came from cached_min_before_update == cached_min_before_invalidate or None
            // If cached_min_before_invalidate was Some(m), witness was cached_min_witness
            // cached_min_witness != rid because if it were, invalidate_min would have set None
            assert(cached_min_before_update == Some(m));
            assert(cached_min_before_invalidate.is_some());
            assert(old_dl.contains_key(cached_min_witness) && old_dl[cached_min_witness] == m);
            if cached_min_witness == rid {
              // old_dl[rid] == m, and we called invalidate_min(old_dl[rid]) = invalidate_min(m)
              // invalidate_min(m) with cached_min == Some(m): m <= m, so cached_min → None
              // But cached_min_before_update == Some(m), contradiction
              assert(false);
            }
            // cached_min_witness != rid, so after deadlines.insert(rid, deadline):
            assert(self.deadlines@.contains_key(cached_min_witness));
            assert(self.deadlines@[cached_min_witness] == m);
          }
        },
        None => {},
      }

      // dl_pos_consistent
      assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
        self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
      by {
        if rid2 == rid {
          assert(self.positions@.contains_key(rid));
        } else {
          assert(old_dl.contains_key(rid2));
          assert(old(self).positions@.contains_key(rid2));
          assert(post_wr_positions.contains_key(rid2));
          assert(self.positions@.contains_key(rid2));
        }
      };
      assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
        self.deadlines@.contains_key(rid2)
      by {
        if rid2 == rid {
        } else {
          assert(post_wr_positions.contains_key(rid2));
          assert(old(self).positions@.contains_key(rid2));
          assert(old_dl.contains_key(rid2));
          assert(self.deadlines@.contains_key(rid2));
        }
      };

      // pos_levels_consistent Part 1 - prove for rid
      {
        let pos_rid = self.positions@[rid];
        assert(pos_rid == WheelPos { level: level as u8, slot: slot as u8, idx });
        assert((pos_rid.level as int) < NUM_LEVELS);
        assert((pos_rid.slot as int) < WHEEL_SIZE);
        assert((pos_rid.idx as int) < self.levels@[pos_rid.level as int]@[pos_rid.slot as int]@.len());
        assert(self.levels@[pos_rid.level as int]@[pos_rid.slot as int]@[pos_rid.idx as int] == rid);
      }

      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = #[trigger] self.positions@[rid2];
          &&& (pos2.level as int) < NUM_LEVELS
          &&& (pos2.slot as int) < WHEEL_SIZE
          &&& (pos2.idx as int) < self.levels@[pos2.level as int]@[pos2.slot as int]@.len()
          &&& self.levels@[pos2.level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2
        })
      by {
        if rid2 == rid {
        } else {
          assert(post_wr_positions.contains_key(rid2));
          assert(self.positions@[rid2] == post_wr_positions[rid2]);
          let pos2 = post_wr_positions[rid2];
          assert((pos2.level as int) < NUM_LEVELS);
          assert((pos2.slot as int) < WHEEL_SIZE);
          assert((pos2.idx as int) < post_wr_levels[pos2.level as int]@[pos2.slot as int]@.len());
          assert(post_wr_levels[pos2.level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2);
          if pos2.level as int == level as int && pos2.slot as int == slot as int {
            assert(mid_slot_vec =~= post_wr_levels[level as int]@[slot as int]@);
            assert(mid_slot_vec[pos2.idx as int] == rid2);
            assert((pos2.idx as int) < mid_slot_vec.len() as int);
            assert(self.levels@[level as int]@[slot as int]@[pos2.idx as int] == mid_slot_vec[pos2.idx as int]);
          } else {
            assert(self.levels@[pos2.level as int]@[pos2.slot as int]@ =~= post_wr_levels[pos2.level as int]@[pos2.slot as int]@);
          }
        }
      };
      // pos_levels_consistent Part 2
      assert forall |l: int, s: int, i: int|
        #![trigger self.levels@[l]@[s]@[i]]
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && 0 <= i < self.levels@[l]@[s]@.len() implies
        self.positions@.contains_key(self.levels@[l]@[s]@[i])
      by {
        if l == level as int && s == slot as int {
          if i == mid_slot_vec.len() as int {
            assert(self.levels@[l]@[s]@[i] == rid);
          } else {
            assert(self.levels@[l]@[s]@[i] == mid_slot_vec[i]);
            assert(post_wr_levels[l]@[s]@[i] == mid_slot_vec[i]);
            assert(post_wr_positions.contains_key(mid_slot_vec[i]));
            assert(mid_slot_vec[i] != rid);
          }
        } else {
          assert(self.levels@[l]@[s]@[i] == post_wr_levels[l]@[s]@[i]);
          assert(post_wr_positions.contains_key(post_wr_levels[l]@[s]@[i]));
          assert(post_wr_levels[l]@[s]@[i] != rid);
        }
      };
      // pos_levels_consistent Part 3
      assert forall |l: int, s: int, i: int|
        #![trigger self.levels@[l]@[s]@[i]]
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && 0 <= i < self.levels@[l]@[s]@.len()
        && self.positions@.contains_key(self.levels@[l]@[s]@[i])
        && !self.pending@.contains(self.levels@[l]@[s]@[i])
        implies ({
          let rid2 = self.levels@[l]@[s]@[i];
          let pos2 = self.positions@[rid2];
          pos2.level as int == l && pos2.slot as int == s && pos2.idx as int == i
        })
      by {
        let rid2 = self.levels@[l]@[s]@[i];
        if rid2 == rid {
          assert(self.positions@[rid] == WheelPos { level: level as u8, slot: slot as u8, idx });
          assert(l == level as int && s == slot as int && i == idx as int);
        } else {
          assert(self.positions@[rid2] == post_wr_positions[rid2]);
          if l == level as int && s == slot as int {
            assert(rid2 == mid_slot_vec[i]);
          }
        }
      };

      // position_ahead
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = self.positions@[rid2];
          let shift = (pos2.level as u32) * WHEEL_BITS;
          #[trigger] self.deadlines@[rid2] >> shift > self.elapsed >> shift
        })
      by {
        if rid2 == rid {
          let d = deadline;
          let e = self.elapsed;
          if level == 0 {
            assert(d >> 0u32 > e >> 0u32) by (bit_vector)
              requires d > e;
          } else if level == 1 {
            assert(deadline - self.elapsed >= 256) by {
              assert(level as int == Self::spec_level_slot(deadline, self.elapsed).0);
            };
            assert(d >> 8u32 > e >> 8u32) by (bit_vector)
              requires d >= e, d - e >= 256u64;
          } else if level == 2 {
            assert(deadline - self.elapsed >= 65536) by {
              assert(level as int == Self::spec_level_slot(deadline, self.elapsed).0);
            };
            assert(d >> 16u32 > e >> 16u32) by (bit_vector)
              requires d >= e, d - e >= 65536u64;
          } else {
            assert(deadline - self.elapsed >= 16777216) by {
              assert(level as int == Self::spec_level_slot(deadline, self.elapsed).0);
            };
            assert(d >> 24u32 > e >> 24u32) by (bit_vector)
              requires d >= e, d - e >= 16777216u64;
          }
        } else {
          assert(post_wr_positions.contains_key(rid2));
          assert(self.positions@[rid2] == post_wr_positions[rid2]);
        }
      };
      // pending_expired: vacuously true (pending is empty)

      // slot_matches_deadline
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = #[trigger] self.positions@[rid2];
          let shift = (pos2.level as u32) * WHEEL_BITS;
          pos2.slot as int == ((self.deadlines@[rid2] >> shift) & 0xff) as int
        })
      by {
        if rid2 == rid {
          let pos2 = self.positions@[rid];
          assert(pos2 == WheelPos { level: level as u8, slot: slot as u8, idx });
          assert(pos2.slot as int == slot as int);
          assert(slot as int == Self::spec_level_slot(deadline, self.elapsed).1);
          let shift = (pos2.level as u32) * WHEEL_BITS;
          assert(self.deadlines@[rid] == deadline);
          assert(pos2.level as int == Self::spec_level_slot(deadline, self.elapsed).0);
          if level == 0 {
            assert(((deadline >> 0u32) & 0xff) as int == Self::spec_level_slot(deadline, self.elapsed).1);
          } else if level == 1 {
            assert(((deadline >> 8u32) & 0xff) as int == Self::spec_level_slot(deadline, self.elapsed).1);
          } else if level == 2 {
            assert(((deadline >> 16u32) & 0xff) as int == Self::spec_level_slot(deadline, self.elapsed).1);
          } else {
            assert(((deadline >> 24u32) & 0xff) as int == Self::spec_level_slot(deadline, self.elapsed).1);
          }
        } else {
          assert(post_wr_positions.contains_key(rid2));
          assert(self.positions@[rid2] == post_wr_positions[rid2]);
          assert(old(self).deadlines@.contains_key(rid2));
          assert(self.deadlines@[rid2] == old(self).deadlines@[rid2]);
          assert(old(self).slot_matches_deadline());
        }
      };

      // level_band
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) &&
        self.deadlines@.contains_key(rid2) &&
        (#[trigger] self.positions@[rid2]).level < 3 implies ({
          let pos2 = self.positions@[rid2];
          let width = ((pos2.level as u32) + 1) * WHEEL_BITS;
          (self.deadlines@[rid2] as int - self.elapsed as int) < ((1u64 << width) as int)
        })
      by {
        if rid2 == rid {
          assert(self.deadlines@[rid2] == deadline);
          assert(self.positions@[rid2] == WheelPos { level: level as u8, slot: slot as u8, idx });
          assert(level as int == Self::spec_level_slot(deadline, self.elapsed).0);
          if level == 0 {
            assert(deadline - self.elapsed < 256);
            assert((1u64 << 8u32) == 256u64) by (bit_vector);
          } else if level == 1 {
            assert(deadline - self.elapsed < 65536);
            assert((1u64 << 16u32) == 65536u64) by (bit_vector);
          } else if level == 2 {
            assert(deadline - self.elapsed < 16777216);
            assert((1u64 << 24u32) == 16777216u64) by (bit_vector);
          }
        } else {
          assert(post_wr_state.level_band());
          assert(post_wr_state.positions@.contains_key(rid2));
          assert(post_wr_state.positions@[rid2] == self.positions@[rid2]);
          assert(post_wr_state.deadlines@.contains_key(rid2));
          assert(post_wr_state.deadlines@[rid2] == self.deadlines@[rid2]);
          assert(post_wr_state.elapsed == self.elapsed);
        }
      };

      // level_counts_wf: exactly one slot gained one element since pre_push
      Self::level_counts_wf_transfer(post_wr_state, pre_push_state);
      assert(pre_push_state.level_counts_wf());
      assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
        (#[trigger] self.level_counts@[l]) as int ==
        Self::slots_len_sum(self.levels@[l]@, WHEEL_SIZE as int)
      by {
        if l == level as int {
          Self::slots_len_sum_update(pre_push_state.levels@[l]@, self.levels@[l]@, slot as int, WHEEL_SIZE as int);
        } else {
          assert(self.levels@[l] == pre_push_state.levels@[l]);
          Self::slots_len_sum_congruence(pre_push_state.levels@[l]@, self.levels@[l]@, WHEEL_SIZE as int);
        }
      };
      assert(self.level_counts_wf());
    }
  }

  pub exec fn remove(&mut self, rid: u64)
    requires
      old(self).wf(),
      old(self).full_wf(),
      old(self).pending@.len() == 0,
    ensures
      self@ == old(self)@.remove(rid as nat),
      self.wf(),
      self.full_wf(),
      self.pending@.len() == 0,
      self.elapsed == old(self).elapsed,
  {
    let ghost old_dl = self.deadlines@;
    let ghost old_cached_min = self.cached_min;
    let ghost cached_min_witness: u64 = match self.cached_min {
      Some(m) => choose |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == m,
      None => 0u64,
    };
    let ghost post_wr_positions = self.positions@;
    let ghost did_remove = self.deadlines@.contains_key(rid);
    if self.deadlines.contains_key(&rid) {
      let deadline = *self.deadlines.get(&rid).unwrap();
      self.invalidate_min(deadline);
      self.wheel_remove_inner(rid);
      proof { post_wr_positions = self.positions@; }
      self.deadlines.remove(&rid);
    }
    proof {
      assert forall |k: nat| self@.contains_key(k) <==>
        old(self)@.remove(rid as nat).contains_key(k)
      by {
        if k <= u64::MAX as nat {
          if k == rid as nat {
          } else {
            assert(self.deadlines@.contains_key(k as u64) == old_dl.contains_key(k as u64));
          }
        }
      };
      assert forall |k: nat| self@.contains_key(k) implies
        self@[k] == old(self)@.remove(rid as nat)[k]
      by {
        assert(k != rid as nat);
        assert(self.deadlines@[k as u64] == old_dl[k as u64]);
      };
      assert(self@ =~= old(self)@.remove(rid as nat));

      // cached_min_valid proof
      reveal(TimerWheel::cached_min_valid);
      match self.cached_min {
        Some(m) => {
          assert(old_cached_min == Some(m));
          assert(old_dl.contains_key(cached_min_witness) && old_dl[cached_min_witness] == m);
          if old_dl.contains_key(rid) {
            assert(cached_min_witness != rid);
          }
          assert(self.deadlines@.contains_key(cached_min_witness));
          assert(self.deadlines@[cached_min_witness] == m);
        },
        None => {},
      }

      // dl_pos_consistent
      assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
        self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
      by {
        assert(rid2 != rid);
        assert(old_dl.contains_key(rid2));
        assert(old(self).positions@.contains_key(rid2));
        if did_remove {
          assert(post_wr_positions.contains_key(rid2));
        }
        assert(self.positions@.contains_key(rid2));
      };
      assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
        self.deadlines@.contains_key(rid2)
      by {
        if did_remove {
          assert(post_wr_positions.contains_key(rid2));
          assert(old(self).positions@.contains_key(rid2));
          assert(rid2 != rid);
        }
        assert(old_dl.contains_key(rid2));
        assert(self.deadlines@.contains_key(rid2));
      };

      // pos_levels_consistent: only depends on positions and levels, unchanged by deadlines.remove
      if did_remove {
        // After wheel_remove_inner: self.pos_levels_consistent() holds
        // deadlines.remove doesn't change positions or levels
      }
      // In both cases (did_remove or not), pos_levels_consistent is maintained
      assert(self.pos_levels_consistent());

      // position_ahead: deadlines[rid2] >> shift > elapsed >> shift for rid2 in positions
      // wheel_remove_inner removed rid from positions, so rid2 != rid for all rid2 in positions
      // deadlines.remove(&rid) only changed deadlines for key rid, not for rid2
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = self.positions@[rid2];
          let shift = (pos2.level as u32) * WHEEL_BITS;
          #[trigger] self.deadlines@[rid2] >> shift > self.elapsed >> shift
        })
      by {
        if did_remove {
          assert(rid2 != rid);
          assert(self.deadlines@[rid2] == old_dl[rid2]);
        }
      };
      // pending_expired: vacuously true (pending is empty)

      // slot_matches_deadline
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = #[trigger] self.positions@[rid2];
          let shift = (pos2.level as u32) * WHEEL_BITS;
          pos2.slot as int == ((self.deadlines@[rid2] >> shift) & 0xff) as int
        })
      by {
        if did_remove {
          assert(rid2 != rid);
          assert(self.deadlines@[rid2] == old_dl[rid2]);
        }
        assert(old(self).slot_matches_deadline());
      };
    }
  }

  pub exec fn try_pop_expired(&mut self, now: u64)
    -> (result: Option<u64>)
    requires
      old(self).wf(),
      old(self).full_wf(),
      now >= old(self).elapsed,
    ensures
      self.wf(),
      self.full_wf(),
      result.is_some() ==> {
        let rid = result.unwrap();
        old(self)@.contains_key(rid as nat) &&
        old(self)@[rid as nat] <= now as int &&
        self@ == old(self)@.remove(rid as nat)
      },
      result.is_none() ==> {
        &&& self@ == old(self)@
        &&& forall |r: nat| #![auto] old(self)@.contains_key(r) ==> old(self)@[r] > now as int
      },
      self.elapsed <= now,
      result.is_none() ==> self.pending@.len() == 0,
  {
    let ghost old_dl = self.deadlines@;
    let ghost old_elapsed = self.elapsed;

    // Phase 1: drain old pending
    while self.pending.len() > 0
      invariant
        self.structural_wf(),
        self.cached_min_valid(),
        self.level_counts == old(self).level_counts,
        self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
        self.deadlines@ == old_dl,
        old_dl == old(self).deadlines@,
        self.elapsed == old_elapsed,
        old_elapsed == old(self).elapsed,
        now >= self.elapsed,
        self.positions@ == old(self).positions@,
        self.levels@ == old(self).levels@,
        self.pending@.len() <= old(self).pending@.len(),
        forall |i: int| 0 <= i < self.pending@.len() as int ==>
          self.pending@[i] == old(self).pending@[i],
        forall |rid2: u64| self.deadlines@.contains_key(rid2) &&
          !self.positions@.contains_key(rid2) ==>
          self.pending@.contains(rid2),
        old(self).full_wf(),
        self.level_band(),
      decreases self.pending@.len(),
    {
      let ghost pre_pop_pending = self.pending@;
      let rid = self.pending.pop().unwrap();
      let ghost pre_dl_iter = self.deadlines@;
      let ghost pre_cm_iter = self.cached_min;
      proof {
        reveal(TimerWheel::cached_min_valid);
        assert(forall |k: u64| pre_cm_iter is Some && #[trigger] pre_dl_iter.contains_key(k)
          ==> pre_cm_iter->Some_0 <= pre_dl_iter[k]);
      }
      if self.deadlines.contains_key(&rid) {
        let deadline = *self.deadlines.get(&rid).unwrap();
        if deadline <= now {
          let ghost cm_witness: u64 = match self.cached_min {
            Some(m) => choose |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == m,
            None => 0u64,
          };
          self.invalidate_min(deadline);
          self.deadlines.remove(&rid);
          self.positions.remove(&rid);
          proof {
            reveal(TimerWheel::cached_min_valid);
            assert forall |k: nat| self@.contains_key(k) <==>
              old(self)@.remove(rid as nat).contains_key(k)
            by {
              if k <= u64::MAX as nat {
                if k == rid as nat {
                } else {
                  assert(self.deadlines@.contains_key(k as u64)
                    == old_dl.contains_key(k as u64));
                }
              }
            };
            assert forall |k: nat| self@.contains_key(k) implies
              self@[k] == old(self)@.remove(rid as nat)[k]
            by {
              assert(k != rid as nat);
              assert(self.deadlines@[k as u64] == old_dl[k as u64]);
            };
            assert(self@ =~= old(self)@.remove(rid as nat));

            if self.cached_min.is_some() {
              let m = self.cached_min.unwrap();
              assert(cm_witness != rid);
              assert(self.deadlines@.contains_key(cm_witness) && self.deadlines@[cm_witness] == m);
            }

            // dl_pos_consistent
            assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
              self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
            by {
              assert(rid2 != rid);
              assert(old_dl.contains_key(rid2));
              if !self.positions@.contains_key(rid2) {
                assert(!old(self).positions@.contains_key(rid2));
                assert(pre_pop_pending.contains(rid2));
                if rid2 == rid { assert(false); }
                let j = choose |j: int| 0 <= j < pre_pop_pending.len() && pre_pop_pending[j] == rid2;
                if j == pre_pop_pending.len() as int - 1 {
                  assert(pre_pop_pending[j] == rid);
                  assert(false);
                }
                assert(self.pending@[j] == pre_pop_pending[j]);
              }
            };
            assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
              self.deadlines@.contains_key(rid2)
            by {
              assert(old(self).positions@.contains_key(rid2));
              assert(old_dl.contains_key(rid2));
              assert(rid2 != rid);
            };

            // pos_levels_consistent: positions = old(self).positions (rid wasn't in it),
            // levels = old(self).levels, from old(self).pending_positions_disjoint rid2 not in pending
            assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) implies ({
                let pos = #[trigger] self.positions@[rid2];
                &&& (pos.level as int) < NUM_LEVELS
                &&& (pos.slot as int) < WHEEL_SIZE
                &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
                &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid2
              })
            by {
              assert(old(self).positions@.contains_key(rid2));
              assert(!old(self).pending@.contains(rid2));
              assert(old(self).pos_levels_consistent());
            };
            assert forall |l: int, s: int, i: int|
              #![trigger self.levels@[l]@[s]@[i]]
              0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
              && 0 <= i < self.levels@[l]@[s]@.len() implies
              self.positions@.contains_key(self.levels@[l]@[s]@[i])
            by {
              assert(old(self).pos_levels_consistent());
            };
            assert forall |l: int, s: int, i: int|
              #![trigger self.levels@[l]@[s]@[i]]
              0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
              && 0 <= i < self.levels@[l]@[s]@.len()
              && self.positions@.contains_key(self.levels@[l]@[s]@[i])
              && !self.pending@.contains(self.levels@[l]@[s]@[i])
              implies ({
                let r = self.levels@[l]@[s]@[i];
                let pos = self.positions@[r];
                pos.level as int == l && pos.slot as int == s && pos.idx as int == i
              })
            by {
              let r = self.levels@[l]@[s]@[i];
              assert(!old(self).pending@.contains(r));
              assert(old(self).pos_levels_consistent());
            };

            // position_ahead
            assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) implies ({
                let pos = self.positions@[rid2];
                let shift = (pos.level as u32) * WHEEL_BITS;
                #[trigger] self.deadlines@[rid2] >> shift > self.elapsed >> shift
              })
            by {
              assert(!old(self).pending@.contains(rid2));
              assert(rid2 != rid);
              assert(self.deadlines@[rid2] == old_dl[rid2]);
              assert(old(self).position_ahead());
            };

            // pending_expired
            assert forall |rid2: u64| self.pending@.contains(rid2) &&
              #[trigger] self.deadlines@.contains_key(rid2) implies
              self.deadlines@[rid2] <= self.elapsed
            by {
              assert(rid2 != rid);
              assert(pre_pop_pending.contains(rid2));
              let j = choose |j: int| 0 <= j < self.pending@.len() && self.pending@[j] == rid2;
              assert(self.pending@[j] == old(self).pending@[j]);
              assert(old(self).pending@.contains(rid2));
              assert(old_dl.contains_key(rid2));
              assert(self.deadlines@[rid2] == old_dl[rid2]);
              assert(old(self).pending_expired());
            };

            // pending_positions_disjoint
            assert forall |rid2: u64| self.pending@.contains(rid2) implies
              !self.positions@.contains_key(rid2)
            by {
              let j = choose |j: int| 0 <= j < self.pending@.len() && self.pending@[j] == rid2;
              assert(self.pending@[j] == old(self).pending@[j]);
              assert(old(self).pending@.contains(rid2));
              assert(old(self).pending_positions_disjoint());
            };

            // slot_matches_deadline
            assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) implies ({
                let pos2 = #[trigger] self.positions@[rid2];
                let shift = (pos2.level as u32) * WHEEL_BITS;
                pos2.slot as int == ((self.deadlines@[rid2] >> shift) & 0xff) as int
              })
            by {
              assert(rid2 != rid);
              assert(self.deadlines@[rid2] == old_dl[rid2]);
              assert(!old(self).pending@.contains(rid2));
              assert(old(self).slot_matches_deadline());
            };
          }
          return Some(rid);
        }
      }
      proof {
        // rid was in old(self).pending (prefix invariant)
        let idx = pre_pop_pending.len() - 1;
        assert(rid == pre_pop_pending[idx as int]);
        assert(pre_pop_pending[idx as int] == old(self).pending@[idx as int]);
        assert(old(self).pending@.contains(rid));
        // If rid is in deadlines, pending_expired gives deadline <= elapsed <= now
        // But we didn't return, so deadline > now. Contradiction.
        // Therefore !deadlines.contains_key(rid)
        if self.deadlines@.contains_key(rid) {
          assert(old(self).full_wf());
          assert(old(self).deadlines@.contains_key(rid));
          assert(old(self).deadlines@[rid] <= old(self).elapsed);
        }

        assert forall |rid2: u64| self.deadlines@.contains_key(rid2) &&
          !self.positions@.contains_key(rid2) implies
          self.pending@.contains(rid2)
        by {
          assert(pre_pop_pending.contains(rid2));
          if rid2 == rid {
            assert(self.deadlines@.contains_key(rid));
            assert(self.deadlines@[rid] <= self.elapsed);
            assert(false);
          }
          let j = choose |j: int| 0 <= j < pre_pop_pending.len() && pre_pop_pending[j] == rid2;
          if j == pre_pop_pending.len() - 1 {
            assert(pre_pop_pending[j] == rid);
            assert(rid2 == rid);
            assert(false);
          }
          assert(self.pending@[j] == pre_pop_pending[j]);
        };

        assert forall |i: int| 0 <= i < self.pending@.len() as int implies
          self.pending@[i] == old(self).pending@[i]
        by {
          assert(self.pending@[i] == pre_pop_pending[i]);
        };
      }
    }

    // After Phase 1: self.pending@.len() == 0
    // positions, levels, deadlines, cached_min, elapsed all unchanged from old(self)
    proof {
      // dl_pos_consistent: forward direction
      assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
        self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
      by {
        // From loop invariant: if !positions, then in pending. But pending empty.
        // So must be in positions.
      };
      // dl_pos_consistent: reverse direction (positions ⊆ deadlines)
      assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
        self.deadlines@.contains_key(rid2)
      by {
        assert(old(self).positions@.contains_key(rid2));
        assert(old(self).dl_pos_consistent());
        assert(old(self).deadlines@.contains_key(rid2));
      };

      // For pos_levels_consistent and position_ahead:
      // pending is now empty, positions/levels/deadlines/elapsed unchanged from old(self).
      // From old(self).pending_positions_disjoint(): entries in old pending have no positions.
      // So positions.contains_key(rid) ==> !old(self).pending.contains(rid).
      // Therefore old conditions "positions.contains_key(rid) && !pending.contains(rid)"
      // are equivalent to just "positions.contains_key(rid)".

      // pos_levels_consistent part 1
      assert forall |rid: u64| self.positions@.contains_key(rid) &&
        !self.pending@.contains(rid) implies ({
          let pos = #[trigger] self.positions@[rid];
          &&& (pos.level as int) < NUM_LEVELS
          &&& (pos.slot as int) < WHEEL_SIZE
          &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
          &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid
        })
      by {
        assert(old(self).positions@.contains_key(rid));
        assert(old(self).pending_positions_disjoint());
        assert(!old(self).pending@.contains(rid));
        assert(old(self).pos_levels_consistent());
      };
      // pos_levels_consistent part 2
      assert forall |l: int, s: int, i: int|
        #![trigger self.levels@[l]@[s]@[i]]
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && 0 <= i < self.levels@[l]@[s]@.len() implies
        self.positions@.contains_key(self.levels@[l]@[s]@[i])
      by {
        assert(old(self).pos_levels_consistent());
        assert(old(self).levels@[l]@[s]@[i] == self.levels@[l]@[s]@[i]);
      };
      // pos_levels_consistent part 3
      assert forall |l: int, s: int, i: int|
        #![trigger self.levels@[l]@[s]@[i]]
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && 0 <= i < self.levels@[l]@[s]@.len()
        && self.positions@.contains_key(self.levels@[l]@[s]@[i])
        && !self.pending@.contains(self.levels@[l]@[s]@[i])
        implies ({
          let rid = self.levels@[l]@[s]@[i];
          let pos = self.positions@[rid];
          pos.level as int == l && pos.slot as int == s && pos.idx as int == i
        })
      by {
        let rid = self.levels@[l]@[s]@[i];
        assert(old(self).positions@.contains_key(rid));
        assert(old(self).pending_positions_disjoint());
        assert(!old(self).pending@.contains(rid));
        assert(old(self).pos_levels_consistent());
      };

      // position_ahead
      assert forall |rid: u64| self.positions@.contains_key(rid) &&
        !self.pending@.contains(rid) implies ({
          let pos = self.positions@[rid];
          let shift = (pos.level as u32) * WHEEL_BITS;
          #[trigger] self.deadlines@[rid] >> shift > self.elapsed >> shift
        })
      by {
        assert(old(self).positions@.contains_key(rid));
        assert(old(self).pending_positions_disjoint());
        assert(!old(self).pending@.contains(rid));
        assert(old(self).position_ahead());
      };

      // pending_expired: vacuous (pending empty)
      // pending_positions_disjoint: vacuous (pending empty)

      // slot_matches_deadline
      assert forall |rid: u64| self.positions@.contains_key(rid) &&
        !self.pending@.contains(rid) implies ({
          let pos = #[trigger] self.positions@[rid];
          let shift = (pos.level as u32) * WHEEL_BITS;
          pos.slot as int == ((self.deadlines@[rid] >> shift) & 0xff) as int
        })
      by {
        assert(old(self).positions@.contains_key(rid));
        assert(!old(self).pending@.contains(rid));
        assert(old(self).slot_matches_deadline());
      };
    }

    // Phase 2: advance and establish coverage
    if self.elapsed < now {
      self.advance_to(now);
    } else {
      proof {
        Self::level_counts_wf_transfer(*old(self), *self);
        assert forall |rid: u64| self.deadlines@.contains_key(rid) && self.deadlines@[rid] <= now implies
          self.pending@.contains(rid)
        by {
          assert(self.positions@.contains_key(rid));
          assert(!self.pending@.contains(rid));
          let pos = self.positions@[rid];
          assert((pos.level as int) < NUM_LEVELS);
          let d = self.deadlines@[rid];
          let e = self.elapsed;
          if pos.level == 0u8 {
            assert(d >> 0u32 > e >> 0u32);
            assert(d >> 0u32 <= e >> 0u32) by (bit_vector) requires d <= e;
          } else if pos.level == 1u8 {
            assert(d >> 8u32 > e >> 8u32);
            assert(d >> 8u32 <= e >> 8u32) by (bit_vector) requires d <= e;
          } else if pos.level == 2u8 {
            assert(d >> 16u32 > e >> 16u32);
            assert(d >> 16u32 <= e >> 16u32) by (bit_vector) requires d <= e;
          } else {
            assert(d >> 24u32 > e >> 24u32);
            assert(d >> 24u32 <= e >> 24u32) by (bit_vector) requires d <= e;
          }
        };
      }
    }

    // Phase 3: drain new pending
    while self.pending.len() > 0
      invariant
        self.structural_wf(),
        self.cached_min_valid(),
        self.deadlines.count_wf(),
        self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
        self.deadlines@ == old_dl,
        old_dl == old(self).deadlines@,
        now >= self.elapsed,
        forall |rid: u64| self.deadlines@.contains_key(rid) && self.deadlines@[rid] <= now ==>
          self.pending@.contains(rid),
        self.dl_pos_consistent(),
        self.pos_levels_consistent(),
        self.position_ahead(),
        self.pending_expired(),
        self.pending_positions_disjoint(),
        self.slot_matches_deadline(),
        self.level_band(),
        self.level_counts_wf(),
      decreases self.pending@.len(),
    {
      let ghost pre_pop_pending = self.pending@;
      let rid = self.pending.pop().unwrap();
      let ghost pre_dl_iter = self.deadlines@;
      let ghost pre_cm_iter = self.cached_min;
      proof {
        reveal(TimerWheel::cached_min_valid);
        assert(forall |k: u64| pre_cm_iter is Some && #[trigger] pre_dl_iter.contains_key(k)
          ==> pre_cm_iter->Some_0 <= pre_dl_iter[k]);
      }

      if self.deadlines.contains_key(&rid) {
        let deadline = *self.deadlines.get(&rid).unwrap();
        if deadline <= now {
          let ghost cm_witness: u64 = match self.cached_min {
            Some(m) => choose |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == m,
            None => 0u64,
          };
          self.invalidate_min(deadline);
          self.deadlines.remove(&rid);
          self.positions.remove(&rid);
          proof {
            reveal(TimerWheel::cached_min_valid);
            assert forall |k: nat| self@.contains_key(k) <==>
              old(self)@.remove(rid as nat).contains_key(k)
            by {
              if k <= u64::MAX as nat {
                if k == rid as nat {
                } else {
                  assert(self.deadlines@.contains_key(k as u64)
                    == old_dl.contains_key(k as u64));
                }
              }
            };
            assert forall |k: nat| self@.contains_key(k) implies
              self@[k] == old(self)@.remove(rid as nat)[k]
            by {
              assert(k != rid as nat);
              assert(self.deadlines@[k as u64] == old_dl[k as u64]);
            };
            assert(self@ =~= old(self)@.remove(rid as nat));

            if self.cached_min.is_some() {
              let m = self.cached_min.unwrap();
              assert(cm_witness != rid);
              assert(self.deadlines@.contains_key(cm_witness) && self.deadlines@[cm_witness] == m);
              assert(pre_cm_iter == Some(m));
              assert forall |k: u64| self.deadlines@.contains_key(k) implies m <= self.deadlines@[k] by {
                assert(k != rid);
                assert(pre_dl_iter.contains_key(k) && pre_dl_iter[k] == self.deadlines@[k]);
              };
            }

            // rid was in pending (popped), so not in positions (from pending_positions_disjoint)
            // positions.remove(&rid) was no-op

            // dl_pos_consistent
            assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
              self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
            by {
              assert(rid2 != rid);
              // From old dl_pos_consistent (before pop): positions or pre_pop_pending
              if !self.positions@.contains_key(rid2) {
                assert(pre_pop_pending.contains(rid2));
                let j = choose |j: int| 0 <= j < pre_pop_pending.len() && pre_pop_pending[j] == rid2;
                if j == pre_pop_pending.len() as int - 1 {
                  assert(pre_pop_pending[j] == rid);
                  assert(false);
                }
                assert(self.pending@[j] == pre_pop_pending[j]);
              }
            };
            assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
              self.deadlines@.contains_key(rid2)
            by {
              assert(rid2 != rid);
            };

            // pos_levels_consistent: positions/levels unchanged (modulo no-op remove)
            // position_ahead: same
            // pending_expired, pending_positions_disjoint: rid removed from pending, remaining unchanged

            // slot_matches_deadline: rid removed from both deadlines and positions, others unchanged
            assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) implies ({
                let pos2 = #[trigger] self.positions@[rid2];
                let shift = (pos2.level as u32) * WHEEL_BITS;
                pos2.slot as int == ((self.deadlines@[rid2] >> shift) & 0xff) as int
              })
            by {
              assert(rid2 != rid);
            };
          }
          return Some(rid);
        }
      }

      proof {
        assert forall |rid2: u64| self.deadlines@.contains_key(rid2) && self.deadlines@[rid2] <= now
          implies self.pending@.contains(rid2)
        by {
          assert(pre_pop_pending.contains(rid2));
          if rid2 == rid {
            assert(false);
          }
          let j = choose |j: int| 0 <= j < pre_pop_pending.len() && pre_pop_pending[j] == rid2;
          if j == pre_pop_pending.len() - 1 {
            assert(pre_pop_pending[j] == rid);
            assert(rid2 == rid);
            assert(false);
          }
          assert(self.pending@[j] == pre_pop_pending[j]);
        };
      }
    }

    // After Phase 3: pending is empty
    proof {
      assert forall |k: nat| self@.contains_key(k) <==>
        old(self)@.contains_key(k)
      by {
        if k <= u64::MAX as nat {
          assert(self.deadlines@.contains_key(k as u64)
            == old_dl.contains_key(k as u64));
        }
      };
      assert forall |k: nat| self@.contains_key(k) implies
        self@[k] == old(self)@[k]
      by {
        assert(self.deadlines@[k as u64] == old_dl[k as u64]);
      };
      assert(self@ =~= old(self)@);

      assert forall |r: nat| #![auto] old(self)@.contains_key(r) implies
        old(self)@[r] > now as int
      by {
        if r <= u64::MAX as nat {
          let rid = r as u64;
          assert(self.deadlines@.contains_key(rid));
          if self.deadlines@[rid] <= now {
            assert(self.pending@.contains(rid));
            assert(self.pending@.len() == 0);
            assert(false);
          }
        }
      };

      // full_wf from loop invariants (pending is empty)
    }
    None
  }

  // Returns the earliest (minimum) pending deadline, or None if empty. The
  // minimum guarantee rests on the verified cached_min_valid invariant (cached_min,
  // when Some, is the minimum) for the fast path, and scan_wheel_min for the
  // fallback.
  pub exec fn next_deadline(&self) -> (result: Option<u64>)
    requires self.full_wf(),
    ensures
      result.is_some() ==> {
        let d = result.unwrap();
        (exists |r: nat| #![auto] self@.contains_key(r) && self@[r] == d as int) &&
        (forall |r: nat| #![auto] self@.contains_key(r) ==> d as int <= self@[r])
      },
      result.is_none() ==> self@ == Map::<nat, int>::empty(),
  {
    if self.is_empty() {
      return None;
    }
    if let Some(d) = self.cached_min {
      proof {
        reveal(TimerWheel::cached_min_valid);
        let wk = choose |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == d;
        assert(self@.contains_key(wk as nat) && self@[wk as nat] == d as int);
        assert forall |r: nat| self@.contains_key(r) implies d as int <= self@[r] by {
          assert(r <= u64::MAX as nat);
          assert(self.deadlines@.contains_key(r as u64));
          assert(d <= self.deadlines@[r as u64]);
        };
      }
      return Some(d);
    }
    self.scan_wheel_min()
  }

  pub exec fn is_empty(&self) -> (result: bool)
    requires self.deadlines.count_wf(),
    ensures result == (self@ == Map::<nat, int>::empty()),
  {
    let r = self.deadlines.is_empty();
    proof {
      if r {
        assert(self.deadlines@ == Map::<u64, u64>::empty());
        assert forall |k: nat| !self@.contains_key(k) by {
          if k <= u64::MAX as nat {
            assert(!self.deadlines@.contains_key(k as u64));
          }
        };
        assert(self@ =~= Map::<nat, int>::empty());
      } else {
        assert(exists |k: u64| self.deadlines@.contains_key(k)) by {
          if forall |k: u64| !self.deadlines@.contains_key(k) {
            assert(self.deadlines@ =~= Map::<u64, u64>::empty());
          }
        };
        let ghost k = choose |k: u64| self.deadlines@.contains_key(k);
        assert(k as nat <= u64::MAX as nat);
        assert(self@.contains_key(k as nat));
      }
    }
    r
  }

  #[verifier::external_body]
  #[inline]
  pub exec fn get_deadline(&self, rid: u64) -> (result: Option<u64>) {
    self.deadlines.get(&rid).copied()
  }

  // ---- private helpers ----

  fn level_slot(deadline: u64, elapsed: u64) -> (result: (usize, usize))
    requires deadline >= elapsed,
    ensures
      result.0 < NUM_LEVELS,
      result.1 < WHEEL_SIZE,
      result.0 as int == Self::spec_level_slot(deadline, elapsed).0,
      result.1 as int == Self::spec_level_slot(deadline, elapsed).1,
  {
    let delta = wrapping_sub_u64(deadline, elapsed);
    let level: usize;
    let shift: u32;
    if delta < 256 {
      level = 0;
      shift = 0;
    } else if delta < 65536 {
      level = 1;
      shift = 8;
    } else if delta < 16777216 {
      level = 2;
      shift = 16;
    } else {
      level = 3;
      shift = 24;
    }
    let slot_masked: u64 = (deadline >> shift) & 255u64;
    proof {
      assert(slot_masked < 256u64) by (bit_vector)
        requires slot_masked == (deadline >> shift) & 255u64;
      assert(delta == (deadline - elapsed) as u64);
      assert(256u64 * 256u64 == 65536u64) by (bit_vector);
      assert(256u64 * 256u64 * 256u64 == 16777216u64) by (bit_vector);
    }
    (level, slot_masked as usize)
  }

  #[verifier::rlimit(50)]
  fn advance_to(&mut self, now: u64)
    requires
      old(self).wf(),
      old(self).full_wf(),
      old(self).pending@.len() == 0,
      old(self).elapsed <= now,
    ensures
      self.wf(),
      self.full_wf(),
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.cached_min == old(self).cached_min,
      self.elapsed == now,
      forall |rid: u64| self.deadlines@.contains_key(rid) && self.deadlines@[rid] <= now ==>
        self.pending@.contains(rid),
  {
    let elapsed = self.elapsed;
    let ghost cm_witness: u64 = match self.cached_min {
      Some(m) => choose |k: u64| self.deadlines@.contains_key(k) && self.deadlines@[k] == m,
      None => 0u64,
    };
    proof {
      // band vs `now` at entry: from the old band vs `elapsed` plus
      // elapsed <= now (deltas only shrink)
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) &&
        self.deadlines@.contains_key(rid2) &&
        (#[trigger] self.positions@[rid2]).level < 3 implies ({
          let width = ((self.positions@[rid2].level as u32) + 1) * WHEEL_BITS;
          (self.deadlines@[rid2] as int - now as int) < ((1u64 << width) as int)
        })
      by {
        assert(old(self).level_band());
      };
    }
    let mut done = false;
    let mut level: usize = 0;
    while level < NUM_LEVELS && !done
      invariant
        level <= NUM_LEVELS,
        self.structural_wf(),
        self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
        self.cached_min == old(self).cached_min,
        elapsed <= now,
        self.elapsed == elapsed,
        now >= self.elapsed,
        forall |rid: u64| self.pending@.contains(rid) &&
          #[trigger] self.deadlines@.contains_key(rid) ==> self.deadlines@[rid] <= now,
        forall |rid: u64| self.pending@.contains(rid) ==>
          !self.positions@.contains_key(rid),
        forall |rid: u64| #[trigger] self.deadlines@.contains_key(rid) ==>
          self.positions@.contains_key(rid) || self.pending@.contains(rid),
        forall |rid: u64| #[trigger] self.positions@.contains_key(rid) ==>
          self.deadlines@.contains_key(rid),
        self.position_ahead(),
        self.slot_matches_deadline(),
        self.pos_levels_consistent(),
        forall |rid: u64| self.positions@.contains_key(rid) &&
          !self.pending@.contains(rid) &&
          (self.positions@[rid].level as int) < level as int ==> {
            let pos = self.positions@[rid];
            let shift2 = (pos.level as u32) * WHEEL_BITS;
            self.deadlines@[rid] >> shift2 > now >> shift2
          },
        forall |rid2: u64| self.positions@.contains_key(rid2) &&
          !self.pending@.contains(rid2) &&
          self.deadlines@.contains_key(rid2) &&
          (#[trigger] self.positions@[rid2]).level < 3 ==> {
            let width = ((self.positions@[rid2].level as u32) + 1) * WHEEL_BITS;
            (self.deadlines@[rid2] as int - now as int) < ((1u64 << width) as int)
          },
        self.level_counts_wf(),
        done ==> level < NUM_LEVELS,
        done ==> elapsed >> ((level as u32) * WHEEL_BITS) == now >> ((level as u32) * WHEEL_BITS),
      decreases NUM_LEVELS - level + (if done { 0usize } else { 1 }),
    {
      let shift: u32 = (level as u32) * WHEEL_BITS;
      assert(shift <= 24) by {
        assert(level < NUM_LEVELS);
      }
      let old_pos: u64 = elapsed >> shift;
      let new_pos: u64 = now >> shift;
      if old_pos == new_pos {
        done = true;
      }
      if !done {
      proof {
        assert(old_pos <= new_pos) by (bit_vector)
          requires old_pos == elapsed >> shift, new_pos == now >> shift, elapsed <= now;
      }
      let diff: u64 = new_pos - old_pos;
      let slots_to_drain: usize = if diff < WHEEL_SIZE as u64 { diff as usize } else { WHEEL_SIZE };
      let old_pos_masked: u64 = old_pos & (WHEEL_SIZE as u64 - 1);
      proof {
        assert(old_pos_masked < 256u64) by (bit_vector)
          requires old_pos_masked == old_pos & 255u64;
      }
      let old_slot: usize = old_pos_masked as usize;
      proof {
        assert forall |rid: u64| self.positions@.contains_key(rid) &&
          !self.pending@.contains(rid) &&
          self.positions@[rid].level as int == level as int &&
          self.deadlines@[rid] <= now implies
          self.deadlines@[rid] >> shift >= old_pos + 1u64
        by {
          assert(self.deadlines@[rid] >> shift > old_pos);
        };
      }
      let mut i: usize = 1;
      while i <= slots_to_drain
        invariant
          1 <= i,
          i <= slots_to_drain + 1,
          slots_to_drain <= WHEEL_SIZE,
          old_slot < WHEEL_SIZE,
          level < NUM_LEVELS,
          self.structural_wf(),
          self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
          self.cached_min == old(self).cached_min,
          self.elapsed == elapsed,
          elapsed <= now,
          forall |rid: u64| self.pending@.contains(rid) &&
            #[trigger] self.deadlines@.contains_key(rid) ==> self.deadlines@[rid] <= now,
          forall |rid: u64| self.pending@.contains(rid) ==>
            !self.positions@.contains_key(rid),
          forall |rid: u64| #[trigger] self.deadlines@.contains_key(rid) ==>
            self.positions@.contains_key(rid) || self.pending@.contains(rid),
          forall |rid: u64| #[trigger] self.positions@.contains_key(rid) ==>
            self.deadlines@.contains_key(rid),
          self.position_ahead(),
          self.slot_matches_deadline(),
          self.pos_levels_consistent(),
          forall |rid: u64| self.positions@.contains_key(rid) &&
            !self.pending@.contains(rid) &&
            (self.positions@[rid].level as int) < level as int ==> {
              let pos = self.positions@[rid];
              let shift2 = (pos.level as u32) * WHEEL_BITS;
              self.deadlines@[rid] >> shift2 > now >> shift2
            },
          old_pos == elapsed >> shift,
          shift == (level as u32) * WHEEL_BITS,
          shift <= 24u32,
          old_pos_masked == old_pos & 255u64,
          old_slot as int == old_pos_masked as int,
          forall |rid: u64| self.positions@.contains_key(rid) &&
            !self.pending@.contains(rid) &&
            self.positions@[rid].level as int == level as int &&
            self.deadlines@[rid] <= now ==>
            self.deadlines@[rid] >> shift >= old_pos + i as u64,
          forall |rid: u64| self.positions@.contains_key(rid) &&
            !self.pending@.contains(rid) &&
            self.positions@[rid].level as int == level as int &&
            slot_in_drained_range(self.positions@[rid].slot as int, old_slot as int, i as int) ==>
            #[trigger] self.deadlines@[rid] >> shift > now >> shift,
          forall |rid2: u64| self.positions@.contains_key(rid2) &&
            !self.pending@.contains(rid2) &&
            self.deadlines@.contains_key(rid2) &&
            (#[trigger] self.positions@[rid2]).level < 3 ==> {
              let width = ((self.positions@[rid2].level as u32) + 1) * WHEEL_BITS;
              (self.deadlines@[rid2] as int - now as int) < ((1u64 << width) as int)
            },
          self.level_counts_wf(),
        decreases slots_to_drain - i + 1,
      {
        let slot: usize = (old_slot + i) % WHEEL_SIZE;
        assert(slot < WHEEL_SIZE);
        let ghost pre_drain_counts_state = *self;
        let slot_vec: Vec<u64> = self.slot_drain(level, slot);
        let ghost pre_drain_levels = self.levels@;
        let ghost pre_drain_positions = self.positions@;
        let ghost pre_drain_pending = self.pending@;
        proof {
          // level_counts_wf across the drain
          assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
            (#[trigger] self.level_counts@[l]) as int ==
            Self::slots_len_sum(self.levels@[l]@, WHEEL_SIZE as int)
          by {
            if l == level as int {
              Self::slots_len_sum_update(pre_drain_counts_state.levels@[l]@, self.levels@[l]@, slot as int, WHEEL_SIZE as int);
            } else {
              Self::slots_len_sum_congruence(pre_drain_counts_state.levels@[l]@, self.levels@[l]@, WHEEL_SIZE as int);
            }
          };
          assert(self.level_counts_wf());
          // Establish j-loop invariants at j=0 after emptying slot (level, slot)
          // levels[level][slot] is now Vec::new(), everything else unchanged
          // positions and pending unchanged

          // Part A (weakened): for rid2 NOT in slot_vec, position correctly points to levels
          assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
            !self.pending@.contains(rid2) &&
            !(exists |k: int| 0 <= k < slot_vec@.len() && slot_vec@[k] == rid2)
            implies ({
              let pos = #[trigger] self.positions@[rid2];
              &&& (pos.level as int) < NUM_LEVELS
              &&& (pos.slot as int) < WHEEL_SIZE
              &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
              &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid2
            })
          by {
            let pos = self.positions@[rid2];
            if pos.level as int == level as int && pos.slot as int == slot as int {
              // rid2's position points to the drained slot, but rid2 is NOT in slot_vec
              // From pre-drain pos_levels_consistent part 1: levels[level][slot][idx] == rid2
              // But slot_vec == old levels[level][slot], so rid2 IS in slot_vec
              // Contradiction
              assert(false);
            } else {
              // Different slot, levels unchanged
            }
          };

          // Part B: all levels entries in positions
          assert forall |l2: int, s2: int, i2: int|
            #![trigger self.levels@[l2]@[s2]@[i2]]
            0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int
            && 0 <= i2 < self.levels@[l2]@[s2]@.len() implies
            self.positions@.contains_key(self.levels@[l2]@[s2]@[i2])
          by {
            if l2 == level as int && s2 == slot as int {
              // Drained slot is empty (Vec::new()), no entries
              assert(self.levels@[l2]@[s2]@.len() == 0);
              assert(false);
            } else {
              // Other slots unchanged
            }
          };

          // Part C: levels → pos match
          assert forall |l2: int, s2: int, i2: int|
            #![trigger self.levels@[l2]@[s2]@[i2]]
            0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int
            && 0 <= i2 < self.levels@[l2]@[s2]@.len()
            && self.positions@.contains_key(self.levels@[l2]@[s2]@[i2])
            && !self.pending@.contains(self.levels@[l2]@[s2]@[i2])
            implies ({
              let rid2 = self.levels@[l2]@[s2]@[i2];
              let pos2 = self.positions@[rid2];
              pos2.level as int == l2 && pos2.slot as int == s2 && pos2.idx as int == i2
            })
          by {
            if l2 == level as int && s2 == slot as int {
              assert(self.levels@[l2]@[s2]@.len() == 0);
              assert(false);
            } else {
            }
          };

          // slot_vec uniqueness: from pre-drain pos_levels_consistent Part C
          assert forall |k1: int, k2: int|
            #![trigger slot_vec@[k1], slot_vec@[k2]]
            0 <= k1 < slot_vec@.len() && 0 <= k2 < slot_vec@.len() && k1 != k2
            implies slot_vec@[k1] != slot_vec@[k2]
          by {
            // slot_vec == old levels[level][slot]
            // From pos_levels_consistent Part C: if levels[l][s][i1] == levels[l][s][i2],
            // then positions[levels[l][s][i1]] = (l, s, i1) and (l, s, i2), so i1 == i2
          };

          // slot_vec entries not in levels after emptying
          assert forall |k: int, l2: int, s2: int, i2: int|
            #![trigger slot_vec@[k], self.levels@[l2]@[s2]@[i2]]
            0 <= k < slot_vec@.len() &&
            0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int &&
            0 <= i2 < self.levels@[l2]@[s2]@.len()
            implies self.levels@[l2]@[s2]@[i2] != slot_vec@[k]
          by {
            if l2 == level as int && s2 == slot as int {
              // Drained slot is empty
              assert(self.levels@[l2]@[s2]@.len() == 0);
              assert(false);
            } else {
              // Other slots unchanged from pre-drain
              // slot_vec[k] was at (level, slot) in pre-drain pos_levels_consistent
              // If levels[l2][s2][i2] == slot_vec[k], then from pre-drain Part C:
              // positions[slot_vec[k]] = (l2, s2, i2)
              // But positions[slot_vec[k]] = (level, slot, k) from pre-drain Part C applied to slot_vec
              // So (l2, s2) = (level, slot), contradiction
              let r = slot_vec@[k];
              if self.levels@[l2]@[s2]@[i2] == r {
                // r was at (level, slot, k) in pre-drain; also at (l2, s2, i2)
                // Both are in positions, from pre-drain Part C they must be the same location
                assert(false);
              }
            }
          };

          assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
            !self.pending@.contains(rid2) &&
            self.positions@[rid2].level as int == level as int &&
            self.deadlines@[rid2] <= now &&
            !(exists |k: int| 0 <= k < slot_vec@.len() && slot_vec@[k] == rid2) implies
            self.deadlines@[rid2] >> shift >= old_pos + i as u64 + 1
          by {
            let d_shifted = self.deadlines@[rid2] >> shift;
            assert(d_shifted >= old_pos + i as u64);
            if d_shifted == old_pos + i as u64 {
              let pos2 = self.positions@[rid2];
              assert(pos2.level as int == level as int);
              assert(pos2.slot as int == ((self.deadlines@[rid2] >> ((pos2.level as u32) * WHEEL_BITS)) & 0xff) as int);
              assert((pos2.idx as int) < self.levels@[level as int]@[pos2.slot as int]@.len());
              assert(self.levels@[level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2);
              assert(d_shifted & 0xFFu64 == d_shifted % 256u64) by (bit_vector);
              assert(d_shifted as int == old_pos + i);
              assert(d_shifted % 256u64 == (old_pos + i) % 256);
              assert(old_pos & 0xFFu64 == old_pos % 256u64) by (bit_vector);
              assert(old_pos_masked == old_pos & 255u64);
              assert(old_pos & 255u64 == old_pos & 0xFFu64) by (bit_vector);
              assert(old_pos_masked == old_pos & 0xFFu64);
              assert(old_pos_masked as int == old_pos % 256);
              assert((old_pos + i) % 256 == (old_pos % 256 + i) % 256) by (nonlinear_arith)
                requires old_pos >= 0, i >= 0;
              assert(old_slot as int == old_pos_masked as int);
              assert(slot as int == (old_slot as int + i as int) % (WHEEL_SIZE as int));
              assert((old_pos_masked as int + i as int) % 256 == slot as int) by (nonlinear_arith)
                requires old_pos_masked as int == old_pos % 256,
                  old_slot as int == old_pos_masked as int,
                  slot as int == (old_slot as int + i as int) % (WHEEL_SIZE as int),
                  0 <= old_pos_masked < 256,
                  0 <= i, i <= 256,
                  WHEEL_SIZE == 256;
              assert(d_shifted == self.deadlines@[rid2] >> shift);
              assert(pos2.level as int == level as int);
              assert((pos2.level as u32) * WHEEL_BITS == (level as u32) * WHEEL_BITS);
              assert(shift == (level as u32) * WHEEL_BITS);
              assert(self.deadlines@[rid2] >> ((pos2.level as u32) * WHEEL_BITS) == d_shifted);
              assert(pos2.slot as int == (d_shifted & 0xFFu64) as int);
              assert(d_shifted & 0xFFu64 == d_shifted % 256u64);
              assert((d_shifted % 256u64) as int == (old_pos + i) % 256);
              assert(pos2.slot as int == (old_pos + i) % 256);
              assert((old_pos + i) % 256 == slot as int);
              assert(pos2.slot as int == slot as int);
              assert(self.levels@[level as int]@[slot as int]@.len() == 0);
              assert(false);
            }
          };
          // drained-range: i-loop drained range invariant preserved
          assert(self.positions@ =~= pre_drain_positions);
          assert(self.pending@ =~= pre_drain_pending);
          assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
            !self.pending@.contains(rid2) &&
            self.positions@[rid2].level as int == level as int &&
            slot_in_drained_range(self.positions@[rid2].slot as int, old_slot as int, i as int) implies
            #[trigger] self.deadlines@[rid2] >> shift > now >> shift
          by {
            assert(pre_drain_positions.contains_key(rid2));
            assert(self.positions@[rid2] == pre_drain_positions[rid2]);
            assert(!pre_drain_pending.contains(rid2));
          };
          // current-slot: slot emptied, no entry can be at (level, slot) unless in slot_vec
          assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
            !self.pending@.contains(rid2) &&
            self.positions@[rid2].level as int == level as int &&
            self.positions@[rid2].slot as int == slot as int &&
            !(exists |k: int| 0 as int <= k < slot_vec@.len() && slot_vec@[k] == rid2) implies
            self.deadlines@[rid2] >> shift > now >> shift
          by {
            let pos = self.positions@[rid2];
            assert(pos.level as int == level as int);
            assert(pos.slot as int == slot as int);
            assert((pos.idx as int) < self.levels@[level as int]@[slot as int]@.len());
            assert(self.levels@[level as int]@[slot as int]@.len() == 0);
            assert(false);
          };
        }
        let mut j: usize = 0;
        while j < slot_vec.len()
          invariant
            j <= slot_vec@.len(),
            self.structural_wf(),
            self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
            self.cached_min == old(self).cached_min,
            level < NUM_LEVELS,
            self.elapsed == elapsed,
            elapsed <= now,
            forall |rid: u64| self.pending@.contains(rid) &&
              #[trigger] self.deadlines@.contains_key(rid) ==> self.deadlines@[rid] <= now,
            forall |rid: u64| self.pending@.contains(rid) ==>
              !self.positions@.contains_key(rid),
            forall |rid: u64| #[trigger] self.deadlines@.contains_key(rid) ==>
              self.positions@.contains_key(rid) || self.pending@.contains(rid),
            forall |rid: u64| #[trigger] self.positions@.contains_key(rid) ==>
              self.deadlines@.contains_key(rid),
            self.position_ahead(),
            self.slot_matches_deadline(),
            slot < WHEEL_SIZE,
            forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) &&
              !(exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2)
              ==> {
                let pos = #[trigger] self.positions@[rid2];
                &&& (pos.level as int) < NUM_LEVELS
                &&& (pos.slot as int) < WHEEL_SIZE
                &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
                &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid2
              },
            forall |l: int, s: int, i: int|
              #![trigger self.levels@[l]@[s]@[i]]
              0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
              && 0 <= i < self.levels@[l]@[s]@.len() ==>
              self.positions@.contains_key(self.levels@[l]@[s]@[i]),
            forall |l: int, s: int, i: int|
              #![trigger self.levels@[l]@[s]@[i]]
              0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
              && 0 <= i < self.levels@[l]@[s]@.len()
              && self.positions@.contains_key(self.levels@[l]@[s]@[i])
              && !self.pending@.contains(self.levels@[l]@[s]@[i])
              ==> {
                let rid2 = self.levels@[l]@[s]@[i];
                let pos2 = self.positions@[rid2];
                pos2.level as int == l && pos2.slot as int == s && pos2.idx as int == i
              },
            forall |k: int, l2: int, s2: int, i2: int|
              #![trigger slot_vec@[k], self.levels@[l2]@[s2]@[i2]]
              j as int <= k < slot_vec@.len() &&
              0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int &&
              0 <= i2 < self.levels@[l2]@[s2]@.len()
              ==> self.levels@[l2]@[s2]@[i2] != slot_vec@[k],
            forall |k1: int, k2: int|
              #![trigger slot_vec@[k1], slot_vec@[k2]]
              0 <= k1 < slot_vec@.len() && 0 <= k2 < slot_vec@.len() && k1 != k2
              ==> slot_vec@[k1] != slot_vec@[k2],
            forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) &&
              (self.positions@[rid2].level as int) < level as int ==> {
                let pos2 = self.positions@[rid2];
                let shift2 = (pos2.level as u32) * WHEEL_BITS;
                self.deadlines@[rid2] >> shift2 > now >> shift2
              },
            shift <= 24,
            shift == (level as u32) * WHEEL_BITS,
            old_pos == elapsed >> shift,
            old_pos_masked == old_pos & 255u64,
            old_slot as int == old_pos_masked as int,
            slot as int == (old_slot as int + i as int) % (WHEEL_SIZE as int),
            forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) &&
              self.positions@[rid2].level as int == level as int &&
              self.deadlines@[rid2] <= now &&
              !(exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2) ==>
              self.deadlines@[rid2] >> shift >= old_pos + i as u64 + 1,
            forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) &&
              self.positions@[rid2].level as int == level as int &&
              slot_in_drained_range(self.positions@[rid2].slot as int, old_slot as int, i as int) ==>
              #[trigger] self.deadlines@[rid2] >> shift > now >> shift,
            forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) &&
              self.positions@[rid2].level as int == level as int &&
              self.positions@[rid2].slot as int == slot as int &&
              !(exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2) ==>
              self.deadlines@[rid2] >> shift > now >> shift,
            forall |rid2: u64| self.positions@.contains_key(rid2) &&
              !self.pending@.contains(rid2) &&
              self.deadlines@.contains_key(rid2) &&
              (#[trigger] self.positions@[rid2]).level < 3 ==> {
                let width = ((self.positions@[rid2].level as u32) + 1) * WHEEL_BITS;
                (self.deadlines@[rid2] as int - now as int) < ((1u64 << width) as int)
              },
            self.level_counts_wf(),
          decreases slot_vec@.len() - j,
        {
          let rid = slot_vec[j];
          let ghost pre_pending = self.pending@;
          let ghost pre_positions = self.positions@;
          let ghost pre_levels = self.levels@;
          if self.deadlines.contains_key(&rid) {
            let deadline = *self.deadlines.get(&rid).unwrap();
            if deadline <= now {
              self.positions.remove(&rid);
              let ghost pending_after_remove = self.pending@;
              proof { assert(pending_after_remove =~= pre_pending); }
              self.pending.push(rid);
              proof {
                assert(self.pending@ =~= pending_after_remove.push(rid));
                assert(self.pending@[pending_after_remove.len() as int] == rid);
                // pending_expired: new entry has deadline <= now, others unchanged
                assert forall |rid2: u64| self.pending@.contains(rid2) &&
                  #[trigger] self.deadlines@.contains_key(rid2) implies self.deadlines@[rid2] <= now
                by {
                  if rid2 == rid {
                  } else {
                    let j = choose |j: int| 0 <= j < self.pending@.len() && self.pending@[j] == rid2;
                    if j < pre_pending.len() as int {
                      assert(pre_pending[j] == rid2);
                      assert(pre_pending.contains(rid2));
                    } else {
                      assert(j == pending_after_remove.len() as int);
                      assert(rid2 == rid);
                    }
                  }
                };
                // pending_positions_disjoint: rid removed from positions before push
                assert forall |rid2: u64| self.pending@.contains(rid2) implies
                  !self.positions@.contains_key(rid2)
                by {
                  if rid2 == rid {
                  } else {
                    let j = choose |j: int| 0 <= j < self.pending@.len() && self.pending@[j] == rid2;
                    if j < pre_pending.len() as int {
                      assert(pre_pending[j] == rid2);
                      assert(pre_pending.contains(rid2));
                      assert(!pre_positions.contains_key(rid2));
                    } else {
                      assert(rid2 == rid);
                    }
                  }
                };
                // dl_pos_consistent forward: rid now in pending
                assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
                  self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
                by {
                  if rid2 == rid {
                    assert(self.pending@[pending_after_remove.len() as int] == rid);
                  } else {
                    if pre_positions.contains_key(rid2) {
                      assert(self.positions@.contains_key(rid2));
                    } else {
                      assert(pre_pending.contains(rid2));
                      let j = choose |j: int| 0 <= j < pre_pending.len() && pre_pending[j] == rid2;
                      assert(self.pending@[j] == rid2);
                    }
                  }
                };
                // dl_pos_consistent reverse: rid removed from positions
                assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
                  self.deadlines@.contains_key(rid2)
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                };
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) implies ({
                    let pos2 = self.positions@[rid2];
                    let shift2 = (pos2.level as u32) * WHEEL_BITS;
                    #[trigger] self.deadlines@[rid2] >> shift2 > self.elapsed >> shift2
                  })
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                  assert(!pre_pending.contains(rid2));
                };
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) implies ({
                    let pos2 = #[trigger] self.positions@[rid2];
                    let shift2 = (pos2.level as u32) * WHEEL_BITS;
                    pos2.slot as int == ((self.deadlines@[rid2] >> shift2) & 0xff) as int
                  })
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                  assert(!pre_pending.contains(rid2));
                };
                // weakened pos_levels_consistent Part A: rid removed from positions
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  !(exists |k: int| (j + 1) as int <= k < slot_vec@.len() && slot_vec@[k] == rid2)
                  implies ({
                    let pos = #[trigger] self.positions@[rid2];
                    &&& (pos.level as int) < NUM_LEVELS
                    &&& (pos.slot as int) < WHEEL_SIZE
                    &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
                    &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid2
                  })
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                  assert(!pre_pending.contains(rid2));
                  assert(self.positions@[rid2] == pre_positions[rid2]);
                  // rid2 is NOT in slot_vec[j+1..], and rid2 != rid = slot_vec[j]
                  // So rid2 is NOT in slot_vec[j..], so pre-iteration Part A applies
                  if exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2 {
                    let k = choose |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2;
                    if k == j as int {
                      assert(slot_vec@[k] == rid);
                      assert(rid2 == rid);
                      assert(false);
                    } else {
                      assert((j + 1) as int <= k);
                      assert(exists |k2: int| (j + 1) as int <= k2 < slot_vec@.len() && slot_vec@[k2] == rid2);
                      assert(false);
                    }
                  }
                };
                // Part B: levels unchanged, rid not in levels (from "not in levels" invariant)
                assert forall |l2: int, s2: int, i2: int|
                  #![trigger self.levels@[l2]@[s2]@[i2]]
                  0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int
                  && 0 <= i2 < self.levels@[l2]@[s2]@.len() implies
                  self.positions@.contains_key(self.levels@[l2]@[s2]@[i2])
                by {
                  let r = self.levels@[l2]@[s2]@[i2];
                  // rid = slot_vec[j], and slot_vec[j..] entries are not in levels
                  assert(r != slot_vec@[j as int]);
                  assert(r != rid);
                  assert(pre_positions.contains_key(r));
                };
                // Part C: levels → pos match, levels unchanged
                assert forall |l2: int, s2: int, i2: int|
                  #![trigger self.levels@[l2]@[s2]@[i2]]
                  0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int
                  && 0 <= i2 < self.levels@[l2]@[s2]@.len()
                  && self.positions@.contains_key(self.levels@[l2]@[s2]@[i2])
                  && !self.pending@.contains(self.levels@[l2]@[s2]@[i2])
                  implies ({
                    let rid2 = self.levels@[l2]@[s2]@[i2];
                    let pos2 = self.positions@[rid2];
                    pos2.level as int == l2 && pos2.slot as int == s2 && pos2.idx as int == i2
                  })
                by {
                  let r = self.levels@[l2]@[s2]@[i2];
                  assert(r != slot_vec@[j as int]);
                  assert(r != rid);
                  assert(pre_positions.contains_key(r));
                  assert(self.positions@[r] == pre_positions[r]);
                };
                // slot_vec not in levels: unchanged (levels unchanged, pending for j+1..)
                assert forall |k: int, l2: int, s2: int, i2: int|
                  #![trigger slot_vec@[k], self.levels@[l2]@[s2]@[i2]]
                  (j + 1) as int <= k < slot_vec@.len() &&
                  0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int &&
                  0 <= i2 < self.levels@[l2]@[s2]@.len()
                  implies self.levels@[l2]@[s2]@[i2] != slot_vec@[k]
                by {
                  // k >= j+1 >= j, so from pre-iteration invariant, levels[l2][s2][i2] != slot_vec[k]
                };
                // slot_vec uniqueness: unchanged
                // strong position_ahead for levels < current
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  (self.positions@[rid2].level as int) < level as int implies {
                    let pos2 = self.positions@[rid2];
                    let shift2 = (pos2.level as u32) * WHEEL_BITS;
                    self.deadlines@[rid2] >> shift2 > now >> shift2
                  }
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                  assert(self.positions@[rid2] == pre_positions[rid2]);
                  assert(!pre_pending.contains(rid2));
                };
                // drained-range invariant: rid removed from positions, others unchanged
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  self.positions@[rid2].level as int == level as int &&
                  slot_in_drained_range(self.positions@[rid2].slot as int, old_slot as int, i as int) implies
                  #[trigger] self.deadlines@[rid2] >> shift > now >> shift
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                  assert(self.positions@[rid2] == pre_positions[rid2]);
                  assert(!pre_pending.contains(rid2));
                };
                // current-slot invariant: rid removed, others unchanged
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  self.positions@[rid2].level as int == level as int &&
                  self.positions@[rid2].slot as int == slot as int &&
                  !(exists |k: int| (j + 1) as int <= k < slot_vec@.len() && slot_vec@[k] == rid2) implies
                  self.deadlines@[rid2] >> shift > now >> shift
                by {
                  assert(rid2 != rid);
                  assert(pre_positions.contains_key(rid2));
                  assert(self.positions@[rid2] == pre_positions[rid2]);
                  assert(!pre_pending.contains(rid2));
                  if exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2 {
                    let k = choose |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2;
                    if k == j as int {
                      assert(slot_vec@[k] == rid);
                      assert(rid2 == rid);
                      assert(false);
                    } else {
                      assert((j + 1) as int <= k);
                      assert(false);
                    }
                  }
                };
              }
            } else {
              let ghost pre_wii_state = *self;
              self.wheel_insert_inner(rid, deadline, now);
              proof {
                // wheel_insert_inner only modifies positions and levels, not pending or deadlines
                // pending unchanged, positions may have rid updated
                // pending_expired: pending unchanged, deadlines unchanged
                // pending_positions_disjoint: pending unchanged. Need to show no pending entry
                //   got added to positions. wheel_insert_inner inserts `rid` into positions.
                //   If rid was in pending, that would break it. But rid is from slot_vec,
                //   and from dl_pos_consistent + pending_positions_disjoint before this step,
                //   rid was in positions (from dl_pos_consistent forward), and pending entries
                //   are not in positions. So rid was not in pending.
                assert(!pre_pending.contains(rid));
                assert forall |rid2: u64| self.pending@.contains(rid2) implies
                  !self.positions@.contains_key(rid2)
                by {
                  assert(pre_pending.contains(rid2));
                  assert(!pre_positions.contains_key(rid2));
                  assert(rid2 != rid);
                };
                // dl_pos_consistent forward: rid is in positions (from wheel_insert_inner)
                assert forall |rid2: u64| #[trigger] self.deadlines@.contains_key(rid2) implies
                  self.positions@.contains_key(rid2) || self.pending@.contains(rid2)
                by {
                  if rid2 == rid {
                    assert(self.positions@.contains_key(rid));
                  } else {
                    if pre_positions.contains_key(rid2) {
                    } else {
                      assert(pre_pending.contains(rid2));
                    }
                  }
                };
                // dl_pos_consistent reverse: wheel_insert_inner inserts rid (which is in deadlines)
                assert forall |rid2: u64| #[trigger] self.positions@.contains_key(rid2) implies
                  self.deadlines@.contains_key(rid2)
                by {
                  if rid2 == rid {
                  } else {
                  }
                };
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) implies ({
                    let pos2 = self.positions@[rid2];
                    let shift2 = (pos2.level as u32) * WHEEL_BITS;
                    #[trigger] self.deadlines@[rid2] >> shift2 > self.elapsed >> shift2
                  })
                by {
                  if rid2 == rid {
                    let pos2 = self.positions@[rid];
                    assert(pos2.level as int == Self::spec_level_slot(deadline as u64, now as u64).0);
                    assert(deadline > now);
                    assert(deadline > elapsed);
                    if pos2.level == 0u8 {
                      assert(deadline as u64 >> 0u32 > elapsed as u64 >> 0u32) by (bit_vector)
                        requires deadline as u64 > elapsed as u64;
                    } else if pos2.level == 1u8 {
                      assert((deadline - now) as u64 >= 256);
                      assert(deadline >= elapsed + 256);
                      assert(deadline as u64 >> 8u32 > elapsed as u64 >> 8u32) by (bit_vector)
                        requires deadline as u64 >= elapsed as u64 + 256u64;
                    } else if pos2.level == 2u8 {
                      assert((deadline - now) as u64 >= 256 * 256);
                      assert(deadline >= elapsed + 65536);
                      assert(deadline as u64 >> 16u32 > elapsed as u64 >> 16u32) by (bit_vector)
                        requires deadline as u64 >= elapsed as u64 + 65536u64;
                    } else {
                      assert(pos2.level == 3u8);
                      assert((deadline - now) as u64 >= 256 * 256 * 256);
                      assert(deadline >= elapsed + 16777216);
                      assert(deadline as u64 >> 24u32 > elapsed as u64 >> 24u32) by (bit_vector)
                        requires deadline as u64 >= elapsed as u64 + 16777216u64;
                    }
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                  }
                };
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) implies ({
                    let pos2 = #[trigger] self.positions@[rid2];
                    let shift2 = (pos2.level as u32) * WHEEL_BITS;
                    pos2.slot as int == ((self.deadlines@[rid2] >> shift2) & 0xff) as int
                  })
                by {
                  if rid2 == rid {
                    let pos2 = self.positions@[rid];
                    let ghost lev = Self::spec_level_slot(deadline as u64, now as u64).0;
                    assert(pos2.level as int == lev);
                    assert(pos2.slot as int == Self::spec_level_slot(deadline as u64, now as u64).1);
                    if lev == 0 {
                      assert((pos2.level as u32) * WHEEL_BITS == 0u32);
                    } else if lev == 1 {
                      assert((pos2.level as u32) * WHEEL_BITS == 8u32);
                    } else if lev == 2 {
                      assert((pos2.level as u32) * WHEEL_BITS == 16u32);
                    } else {
                      assert((pos2.level as u32) * WHEEL_BITS == 24u32);
                    }
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                  }
                };
                let ghost tl = Self::spec_level_slot(deadline as u64, now as u64).0;
                let ghost ts = Self::spec_level_slot(deadline as u64, now as u64).1;
                // level_counts_wf across the cascade insert
                assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
                  (#[trigger] self.level_counts@[l]) as int ==
                  Self::slots_len_sum(self.levels@[l]@, WHEEL_SIZE as int)
                by {
                  if l == tl {
                    Self::slots_len_sum_update(pre_wii_state.levels@[l]@, self.levels@[l]@, ts, WHEEL_SIZE as int);
                  } else {
                    Self::slots_len_sum_congruence(pre_wii_state.levels@[l]@, self.levels@[l]@, WHEEL_SIZE as int);
                  }
                };
                assert(self.level_counts_wf());
                // weakened pos_levels_consistent Part A: rid re-inserted with correct position
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  !(exists |k: int| (j + 1) as int <= k < slot_vec@.len() && slot_vec@[k] == rid2)
                  implies ({
                    let pos = #[trigger] self.positions@[rid2];
                    &&& (pos.level as int) < NUM_LEVELS
                    &&& (pos.slot as int) < WHEEL_SIZE
                    &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
                    &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid2
                  })
                by {
                  if rid2 == rid {
                    assert(self.positions@[rid].level as int == tl);
                    assert(self.positions@[rid].slot as int == ts);
                    assert(self.positions@[rid].idx as int == pre_levels[tl]@[ts]@.len());
                    assert(self.levels@[tl]@[ts]@ =~= pre_levels[tl]@[ts]@.push(rid));
                    assert(self.levels@[tl]@[ts]@[self.positions@[rid].idx as int] == rid);
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                    // rid2 != rid = slot_vec[j], and rid2 not in slot_vec[j+1..], so not in slot_vec[j..]
                    if exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2 {
                      let k = choose |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2;
                      if k == j as int {
                        assert(slot_vec@[k] == rid);
                        assert(false);
                      } else {
                        assert((j + 1) as int <= k);
                        assert(false);
                      }
                    }
                    let pos = pre_positions[rid2];
                    assert((pos.level as int) < NUM_LEVELS);
                    assert((pos.slot as int) < WHEEL_SIZE);
                    if pos.level as int == tl && pos.slot as int == ts {
                      assert(pre_levels[tl]@[ts]@[pos.idx as int] == rid2);
                      assert(self.levels@[tl]@[ts]@ =~= pre_levels[tl]@[ts]@.push(rid));
                      assert(self.levels@[tl]@[ts]@[pos.idx as int] == pre_levels[tl]@[ts]@[pos.idx as int]);
                    } else {
                      assert(self.levels@[pos.level as int]@[pos.slot as int]@ =~= pre_levels[pos.level as int]@[pos.slot as int]@);
                    }
                  }
                };
                // Part B: levels entries all in positions
                assert forall |l2: int, s2: int, i2: int|
                  #![trigger self.levels@[l2]@[s2]@[i2]]
                  0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int
                  && 0 <= i2 < self.levels@[l2]@[s2]@.len() implies
                  self.positions@.contains_key(self.levels@[l2]@[s2]@[i2])
                by {
                  if l2 == tl && s2 == ts {
                    assert(self.levels@[tl]@[ts]@ =~= pre_levels[tl]@[ts]@.push(rid));
                    if i2 < pre_levels[tl]@[ts]@.len() as int {
                      assert(self.levels@[tl]@[ts]@[i2] == pre_levels[tl]@[ts]@[i2]);
                      assert(pre_positions.contains_key(pre_levels[tl]@[ts]@[i2]));
                      let r = pre_levels[tl]@[ts]@[i2];
                      if r == rid {
                        assert(self.positions@.contains_key(rid));
                      } else {
                        assert(pre_positions.contains_key(r));
                        assert(self.positions@.contains_key(r));
                      }
                    } else {
                      assert(i2 == pre_levels[tl]@[ts]@.len() as int);
                      assert(self.levels@[tl]@[ts]@[i2] == rid);
                      assert(self.positions@.contains_key(rid));
                    }
                  } else {
                    assert(self.levels@[l2]@[s2]@ =~= pre_levels[l2]@[s2]@);
                    assert(self.levels@[l2]@[s2]@[i2] == pre_levels[l2]@[s2]@[i2]);
                    let r = pre_levels[l2]@[s2]@[i2];
                    if r == rid {
                      assert(self.positions@.contains_key(rid));
                    } else {
                      assert(pre_positions.contains_key(r));
                      assert(self.positions@.contains_key(r));
                    }
                  }
                };
                // Part C: levels → pos match
                assert forall |l2: int, s2: int, i2: int|
                  #![trigger self.levels@[l2]@[s2]@[i2]]
                  0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int
                  && 0 <= i2 < self.levels@[l2]@[s2]@.len()
                  && self.positions@.contains_key(self.levels@[l2]@[s2]@[i2])
                  && !self.pending@.contains(self.levels@[l2]@[s2]@[i2])
                  implies ({
                    let rid2 = self.levels@[l2]@[s2]@[i2];
                    let pos2 = self.positions@[rid2];
                    pos2.level as int == l2 && pos2.slot as int == s2 && pos2.idx as int == i2
                  })
                by {
                  if l2 == tl && s2 == ts {
                    assert(self.levels@[tl]@[ts]@ =~= pre_levels[tl]@[ts]@.push(rid));
                    if i2 < pre_levels[tl]@[ts]@.len() as int {
                      let r = pre_levels[tl]@[ts]@[i2];
                      assert(self.levels@[tl]@[ts]@[i2] == r);
                      if r == rid {
                        assert(self.positions@[rid].level as int == tl);
                        assert(self.positions@[rid].slot as int == ts);
                      } else {
                        assert(self.positions@[r] == pre_positions[r]);
                      }
                    } else {
                      assert(self.levels@[tl]@[ts]@[i2] == rid);
                      assert(self.positions@[rid].level as int == tl);
                      assert(self.positions@[rid].slot as int == ts);
                    }
                  } else {
                    let r = self.levels@[l2]@[s2]@[i2];
                    assert(r == pre_levels[l2]@[s2]@[i2]);
                    assert(r != rid);
                    assert(self.positions@[r] == pre_positions[r]);
                  }
                };
                // slot_vec not in levels after wheel_insert_inner
                assert forall |k: int, l2: int, s2: int, i2: int|
                  #![trigger slot_vec@[k], self.levels@[l2]@[s2]@[i2]]
                  (j + 1) as int <= k < slot_vec@.len() &&
                  0 <= l2 < NUM_LEVELS as int && 0 <= s2 < WHEEL_SIZE as int &&
                  0 <= i2 < self.levels@[l2]@[s2]@.len()
                  implies self.levels@[l2]@[s2]@[i2] != slot_vec@[k]
                by {
                  if l2 == tl && s2 == ts {
                    assert(self.levels@[tl]@[ts]@ =~= pre_levels[tl]@[ts]@.push(rid));
                    if i2 < pre_levels[tl]@[ts]@.len() as int {
                      // Old entry, from pre-iteration invariant (k >= j+1 > j)
                    } else {
                      // New entry is rid = slot_vec[j], and k >= j+1, unique entries
                      assert(self.levels@[tl]@[ts]@[i2] == rid);
                      assert(rid == slot_vec@[j as int]);
                      assert(slot_vec@[k] != slot_vec@[j as int]);
                    }
                  } else {
                    assert(self.levels@[l2]@[s2]@ =~= pre_levels[l2]@[s2]@);
                  }
                };
                // strong position_ahead for levels < current
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  (self.positions@[rid2].level as int) < level as int implies {
                    let pos2 = self.positions@[rid2];
                    let shift2 = (pos2.level as u32) * WHEEL_BITS;
                    self.deadlines@[rid2] >> shift2 > now >> shift2
                  }
                by {
                  if rid2 == rid {
                    let pos2 = self.positions@[rid];
                    let lev = Self::spec_level_slot(deadline as u64, now as u64).0;
                    assert(pos2.level as int == lev);
                    assert(deadline > now);
                    if lev == 0 {
                      assert(deadline as u64 >> 0u32 > now as u64 >> 0u32) by (bit_vector)
                        requires deadline as u64 > now as u64;
                    } else if lev == 1 {
                      assert((deadline - now) as u64 >= 256);
                      assert(deadline as u64 >> 8u32 > now as u64 >> 8u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 256u64;
                    } else if lev == 2 {
                      assert((deadline - now) as u64 >= 256 * 256);
                      assert(deadline as u64 >> 16u32 > now as u64 >> 16u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 65536u64;
                    } else {
                      assert(lev == 3);
                      assert(lev < level as int);
                      assert(level >= 4);
                      assert(level < NUM_LEVELS);
                      assert(false);
                    }
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                  }
                };
                // drained-range invariant: re-inserted entry
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  self.positions@[rid2].level as int == level as int &&
                  slot_in_drained_range(self.positions@[rid2].slot as int, old_slot as int, i as int) implies
                  #[trigger] self.deadlines@[rid2] >> shift > now >> shift
                by {
                  if rid2 == rid {
                    let pos2 = self.positions@[rid];
                    let lev = Self::spec_level_slot(deadline as u64, now as u64).0;
                    assert(pos2.level as int == lev);
                    assert(pos2.level as int == level as int);
                    assert(lev == level as int);
                    assert(deadline > now);
                    assert(256u64 * 256u64 == 65536u64) by (bit_vector);
                    assert(256u64 * 256u64 * 256u64 == 16777216u64) by (bit_vector);
                    if level == 0 {
                      assert(deadline as u64 >> 0u32 > now as u64 >> 0u32) by (bit_vector)
                        requires deadline as u64 > now as u64;
                    } else if level == 1 {
                      assert((deadline - now) as u64 >= 256);
                      assert(deadline as u64 >> 8u32 > now as u64 >> 8u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 256u64;
                    } else if level == 2 {
                      assert((deadline - now) as u64 >= 65536);
                      assert(deadline as u64 >> 16u32 > now as u64 >> 16u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 65536u64;
                    } else {
                      assert((deadline - now) as u64 >= 16777216);
                      assert(deadline as u64 >> 24u32 > now as u64 >> 24u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 16777216u64;
                    }
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                  }
                };
                // current-slot invariant: re-inserted entry
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  self.positions@[rid2].level as int == level as int &&
                  self.positions@[rid2].slot as int == slot as int &&
                  !(exists |k: int| (j + 1) as int <= k < slot_vec@.len() && slot_vec@[k] == rid2) implies
                  self.deadlines@[rid2] >> shift > now >> shift
                by {
                  if rid2 == rid {
                    let pos2 = self.positions@[rid];
                    let lev = Self::spec_level_slot(deadline as u64, now as u64).0;
                    assert(pos2.level as int == lev);
                    assert(pos2.level as int == level as int);
                    assert(lev == level as int);
                    assert(deadline > now);
                    assert(256u64 * 256u64 == 65536u64) by (bit_vector);
                    assert(256u64 * 256u64 * 256u64 == 16777216u64) by (bit_vector);
                    if level == 0 {
                      assert(deadline as u64 >> 0u32 > now as u64 >> 0u32) by (bit_vector)
                        requires deadline as u64 > now as u64;
                    } else if level == 1 {
                      assert((deadline - now) as u64 >= 256);
                      assert(deadline as u64 >> 8u32 > now as u64 >> 8u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 256u64;
                    } else if level == 2 {
                      assert((deadline - now) as u64 >= 65536);
                      assert(deadline as u64 >> 16u32 > now as u64 >> 16u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 65536u64;
                    } else {
                      assert((deadline - now) as u64 >= 16777216);
                      assert(deadline as u64 >> 24u32 > now as u64 >> 24u32) by (bit_vector)
                        requires deadline as u64 >= now as u64 + 16777216u64;
                    }
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                    if exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2 {
                      let k = choose |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2;
                      if k == j as int {
                        assert(slot_vec@[k] == rid);
                        assert(rid2 == rid);
                        assert(false);
                      } else {
                        assert((j + 1) as int <= k);
                        assert(false);
                      }
                    }
                  }
                };
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  self.positions@[rid2].level as int == level as int &&
                  self.deadlines@[rid2] <= now &&
                  !(exists |k: int| (j + 1) as int <= k < slot_vec@.len() && slot_vec@[k] == rid2) implies
                  self.deadlines@[rid2] >> shift >= old_pos + i as u64 + 1
                by {
                  if rid2 == rid {
                    assert(deadline > now);
                    assert(self.deadlines@[rid2] == deadline as u64);
                    assert(false);
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                    assert(!pre_pending.contains(rid2));
                    if exists |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2 {
                      let k = choose |k: int| j as int <= k < slot_vec@.len() && slot_vec@[k] == rid2;
                      if k == j as int {
                        assert(slot_vec@[k] == rid);
                        assert(rid2 == rid);
                        assert(false);
                      } else {
                        assert((j + 1) as int <= k);
                        assert(false);
                      }
                    }
                  }
                };
                // band-vs-now: re-cascaded rid gets its band from the
                // spec_level_slot branch at `now`; untouched rids keep theirs
                assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
                  !self.pending@.contains(rid2) &&
                  self.deadlines@.contains_key(rid2) &&
                  (#[trigger] self.positions@[rid2]).level < 3 implies ({
                    let width = ((self.positions@[rid2].level as u32) + 1) * WHEEL_BITS;
                    (self.deadlines@[rid2] as int - now as int) < ((1u64 << width) as int)
                  })
                by {
                  if rid2 == rid {
                    let lev = Self::spec_level_slot(deadline as u64, now as u64).0;
                    assert(self.positions@[rid].level as int == lev);
                    assert(self.deadlines@[rid2] == deadline as u64);
                    assert(deadline > now);
                    if lev == 0 {
                      assert(deadline - now < 256);
                      assert((1u64 << 8u32) == 256u64) by (bit_vector);
                    } else if lev == 1 {
                      assert(deadline - now < 65536);
                      assert((1u64 << 16u32) == 65536u64) by (bit_vector);
                    } else if lev == 2 {
                      assert(deadline - now < 16777216);
                      assert((1u64 << 24u32) == 16777216u64) by (bit_vector);
                    }
                  } else {
                    assert(pre_positions.contains_key(rid2));
                    assert(self.positions@[rid2] == pre_positions[rid2]);
                  }
                };
              }
            }
          }
          j = j + 1;
        }
        proof {
          // Expand drained range from i to i+1
          // After j-loop: entries in previously-drained slots (slot_in_drained_range(s, old_slot, i))
          //   have d >> shift > now >> shift (from drained range j-loop invariant)
          // And entries at current slot (s == slot == (old_slot + i) % 256)
          //   not in slot_vec[slot_vec.len()..] = not in anything = all such entries
          //   have d >> shift > now >> shift (from current-slot j-loop invariant)
          // Combined: slot_in_drained_range(s, old_slot, i + 1) ==> d >> shift > now >> shift
          assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
            !self.pending@.contains(rid2) &&
            self.positions@[rid2].level as int == level as int &&
            slot_in_drained_range(self.positions@[rid2].slot as int, old_slot as int, (i + 1) as int) implies
            #[trigger] self.deadlines@[rid2] >> shift > now >> shift
          by {
            let s = self.positions@[rid2].slot as int;
            if slot_in_drained_range(s, old_slot as int, i as int) {
              // previously drained
            } else {
              // must be the current slot — manually expand slot_in_drained_range
              let offset_new = (s - old_slot as int + WHEEL_SIZE as int) % (WHEEL_SIZE as int);
              if (i + 1) as int > WHEEL_SIZE as int {
                // i == 256, i+1 == 257 > 256: slot_in_drained_range(s, old_slot, 257) = true (if branch)
                // !slot_in_drained_range(s, old_slot, 256): 256 <= 256 so else branch:
                //   !(1 <= offset_new && offset_new < 256), i.e. offset_new == 0
                assert(i as int == WHEEL_SIZE as int);
                assert(!(1 <= offset_new && offset_new < WHEEL_SIZE as int));
                assert(0 <= offset_new < WHEEL_SIZE as int) by (nonlinear_arith)
                  requires offset_new == (s - old_slot as int + 256) % 256,
                    0 <= s < 256, 0 <= old_slot < 256;
                assert(offset_new == 0);
                assert(s == old_slot as int) by (nonlinear_arith)
                  requires offset_new == 0,
                    offset_new == (s - old_slot as int + 256) % 256,
                    0 <= s < 256, 0 <= old_slot < 256;
                assert(slot as int == (old_slot as int + i as int) % (WHEEL_SIZE as int));
                assert(slot as int == old_slot as int) by (nonlinear_arith)
                  requires slot as int == (old_slot as int + 256) % 256,
                    0 <= old_slot < 256;
              } else {
                // i+1 <= 256: both in else branch
                assert(1 <= offset_new && offset_new < (i + 1) as int);
                assert(offset_new < 1 || offset_new >= i as int);
                assert(offset_new == i as int);
                assert(s == (old_slot as int + i as int) % (WHEEL_SIZE as int)) by (nonlinear_arith)
                  requires
                    offset_new == i as int,
                    offset_new == (s - old_slot as int + WHEEL_SIZE as int) % (WHEEL_SIZE as int),
                    0 <= s < WHEEL_SIZE as int,
                    0 <= old_slot < WHEEL_SIZE as int,
                    WHEEL_SIZE == 256;
              }
              assert(s == slot as int);
              assert(!(exists |k: int| slot_vec@.len() as int <= k < slot_vec@.len() && slot_vec@[k] == rid2));
            }
          };
        }
        i = i + 1;
      }
      proof {
        assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
          !self.pending@.contains(rid2) &&
          self.positions@[rid2].level as int == level as int implies {
            let pos2 = self.positions@[rid2];
            let shift2 = (pos2.level as u32) * WHEEL_BITS;
            self.deadlines@[rid2] >> shift2 > now >> shift2
          }
        by {
          let d = self.deadlines@[rid2];
          let pos2 = self.positions@[rid2];
          let shift2 = (pos2.level as u32) * WHEEL_BITS;
          assert(pos2.level as int == level as int);
          assert(shift2 == shift);
          if diff >= WHEEL_SIZE as u64 {
            assert(slots_to_drain == WHEEL_SIZE);
            assert(i as int == 257);
            assert(slot_in_drained_range(pos2.slot as int, old_slot as int, i as int));
          } else {
            assert(d >> shift > old_pos);
            if d >> shift > new_pos {
            } else {
              assert(d >> shift <= new_pos);
              assert(d >> shift > old_pos);
              let ds = d >> shift;
              let gap = ds - old_pos;
              assert(1 <= gap);
              assert(gap <= diff);
              assert(gap < WHEEL_SIZE as int);
              assert(pos2.slot as int == (ds & 0xFFu64) as int);
              assert(old_slot as int == old_pos_masked as int);
              assert(old_pos_masked as int == (old_pos & 0xFFu64) as int);
              let s = pos2.slot as int;
              let o = old_slot as int;
              let offset_val = (s - o + WHEEL_SIZE as int) % (WHEEL_SIZE as int);
              assert(ds & 0xFFu64 == ds % 256u64) by (bit_vector);
              assert(old_pos & 0xFFu64 == old_pos % 256u64) by (bit_vector);
              assert(s == (ds % 256u64) as int);
              assert(o == (old_pos % 256u64) as int);
              assert(ds == old_pos + gap);
              assert(offset_val == gap) by (nonlinear_arith)
                requires
                  s == ((old_pos + gap) % 256) as int,
                  o == (old_pos % 256) as int,
                  offset_val == (s - o + 256) % 256,
                  0 <= old_pos,
                  1 <= gap,
                  gap < 256,
              ;
              assert(gap < i as int);
              assert(1 <= offset_val && offset_val < i as int);
              assert(slot_in_drained_range(s, old_slot as int, i as int));
            }
          }
        };
      }
      level = level + 1;
      } // end if !done
    }
    let level_shift_val: u32 = if level < NUM_LEVELS { (level as u32) * WHEEL_BITS } else { 0 };
    let ghost pre_elapsed_deadlines = self.deadlines@;
    let ghost pre_elapsed_positions = self.positions@;
    let ghost pre_elapsed_pending = self.pending@;
    let ghost pre_elapsed_levels = self.levels@;
    let ghost pre_elapsed_cached_min = self.cached_min;
    proof {
      assert forall |rid: u64| self.pending@.contains(rid) implies
        !self.positions@.contains_key(rid)
      by {};
      assert forall |rid: u64| #[trigger] self.deadlines@.contains_key(rid) implies
        self.positions@.contains_key(rid) || self.pending@.contains(rid)
      by {};
      assert forall |rid: u64| #[trigger] self.positions@.contains_key(rid) implies
        self.deadlines@.contains_key(rid)
      by {};
      assert forall |rid: u64| self.pending@.contains(rid) &&
        #[trigger] self.deadlines@.contains_key(rid) implies self.deadlines@[rid] <= now
      by {};
    }
    self.elapsed = now;
    proof {
      reveal(TimerWheel::cached_min_valid);
      // structural_wf: levels unchanged
      assert(self.levels@ =~= pre_elapsed_levels);
      assert(self.structural_wf());
      // cached_min_valid
      match self.cached_min {
        Some(m) => {
          assert(self.deadlines@.contains_key(cm_witness));
          assert(self.deadlines@[cm_witness] == m);
        },
        None => {},
      }
      assert(self.cached_min_valid());
      assert(self.wf());
      // dl_pos_consistent: deadlines, positions, pending unchanged
      assert(self.deadlines@ =~= pre_elapsed_deadlines);
      assert(self.positions@ =~= pre_elapsed_positions);
      assert(self.pending@ =~= pre_elapsed_pending);
      assert forall |rid: u64| #[trigger] self.deadlines@.contains_key(rid) implies
        self.positions@.contains_key(rid) || self.pending@.contains(rid)
      by {
        assert(pre_elapsed_deadlines.contains_key(rid));
        assert(pre_elapsed_positions.contains_key(rid) || pre_elapsed_pending.contains(rid));
      };
      assert forall |rid: u64| #[trigger] self.positions@.contains_key(rid) implies
        self.deadlines@.contains_key(rid)
      by {
        assert(pre_elapsed_positions.contains_key(rid));
        assert(pre_elapsed_deadlines.contains_key(rid));
      };
      assert(self.dl_pos_consistent());
      // pending_positions_disjoint
      assert forall |rid: u64| self.pending@.contains(rid) implies
        !self.positions@.contains_key(rid)
      by {
        assert(pre_elapsed_pending.contains(rid));
        assert(!pre_elapsed_positions.contains_key(rid));
      };
      assert(self.pending_positions_disjoint());
      // pending_expired: deadlines[rid] <= now == self.elapsed
      assert forall |rid: u64| self.pending@.contains(rid) &&
        #[trigger] self.deadlines@.contains_key(rid) implies self.deadlines@[rid] <= self.elapsed
      by {
        assert(pre_elapsed_pending.contains(rid));
        assert(pre_elapsed_deadlines.contains_key(rid));
        assert(pre_elapsed_deadlines[rid] <= now);
        assert(self.deadlines@[rid] == pre_elapsed_deadlines[rid]);
      };
      assert(self.pending_expired());
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos = #[trigger] self.positions@[rid2];
          &&& (pos.level as int) < NUM_LEVELS
          &&& (pos.slot as int) < WHEEL_SIZE
          &&& (pos.idx as int) < self.levels@[pos.level as int]@[pos.slot as int]@.len()
          &&& self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid2
        })
      by {
        assert(pre_elapsed_positions.contains_key(rid2));
        assert(pre_elapsed_pending.contains(rid2) == self.pending@.contains(rid2));
      };
      assert forall |l: int, s: int, i2: int|
        #![trigger self.levels@[l]@[s]@[i2]]
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && 0 <= i2 < self.levels@[l]@[s]@.len() implies
        self.positions@.contains_key(self.levels@[l]@[s]@[i2])
      by {
        assert(pre_elapsed_levels[l]@[s]@[i2] == self.levels@[l]@[s]@[i2]);
      };
      assert forall |l: int, s: int, i2: int|
        #![trigger self.levels@[l]@[s]@[i2]]
        0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && 0 <= i2 < self.levels@[l]@[s]@.len()
        && self.positions@.contains_key(self.levels@[l]@[s]@[i2])
        && !self.pending@.contains(self.levels@[l]@[s]@[i2])
        implies ({
          let rid2 = self.levels@[l]@[s]@[i2];
          let pos2 = self.positions@[rid2];
          pos2.level as int == l && pos2.slot as int == s && pos2.idx as int == i2
        })
      by {
        let rid2 = self.levels@[l]@[s]@[i2];
        assert(pre_elapsed_levels[l]@[s]@[i2] == rid2);
        assert(pre_elapsed_positions.contains_key(rid2) == self.positions@.contains_key(rid2));
        assert(pre_elapsed_pending.contains(rid2) == self.pending@.contains(rid2));
      };
      assert(self.pos_levels_consistent());
      // level_band: the loops carry the band stated against `now`; with
      // elapsed == now it is the invariant verbatim
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) &&
        self.deadlines@.contains_key(rid2) &&
        (#[trigger] self.positions@[rid2]).level < 3 implies ({
          let width = ((self.positions@[rid2].level as u32) + 1) * WHEEL_BITS;
          (self.deadlines@[rid2] as int - self.elapsed as int) < ((1u64 << width) as int)
        })
      by {
        assert(pre_elapsed_positions.contains_key(rid2));
        assert(self.positions@[rid2] == pre_elapsed_positions[rid2]);
        assert(self.deadlines@[rid2] == pre_elapsed_deadlines[rid2]);
        assert(pre_elapsed_pending.contains(rid2) == self.pending@.contains(rid2));
      };
      assert(self.level_band());
      // position_ahead: deadline >> shift > self.elapsed >> shift where self.elapsed == now
      // For entries at levels < level (processed): strong position_ahead gives D > now >> shift
      // For entries at levels >= level (unprocessed or break): elapsed >> shift == now >> shift at level_stop
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = self.positions@[rid2];
          let shift2 = (pos2.level as u32) * WHEEL_BITS;
          #[trigger] self.deadlines@[rid2] >> shift2 > self.elapsed >> shift2
        })
      by {
        assert(pre_elapsed_positions.contains_key(rid2));
        assert(self.positions@[rid2] == pre_elapsed_positions[rid2]);
        assert(pre_elapsed_pending.contains(rid2) == self.pending@.contains(rid2));
        assert(self.deadlines@[rid2] == pre_elapsed_deadlines[rid2]);
        let pos2 = self.positions@[rid2];
        let shift2 = (pos2.level as u32) * WHEEL_BITS;
        if (pos2.level as int) < level as int {
          // From strong position_ahead for processed levels
          assert(pre_elapsed_deadlines[rid2] >> shift2 > now >> shift2);
          assert(self.elapsed == now);
        } else {
          assert(pre_elapsed_deadlines[rid2] >> shift2 > elapsed >> shift2);
          if level < NUM_LEVELS {
            assert(done);
            assert(level_shift_val <= 24u32) by {
              assert(level < NUM_LEVELS);
            }
            assert(elapsed >> level_shift_val == now >> level_shift_val);
            if pos2.level as int == level as int {
              assert(shift2 == level_shift_val as int);
            } else if pos2.level as int == level as int + 1 {
              assert(shift2 == level_shift_val as int + WHEEL_BITS as int);
              assert(elapsed >> (level_shift_val + WHEEL_BITS) == now >> (level_shift_val + WHEEL_BITS)) by (bit_vector)
                requires elapsed >> level_shift_val == now >> level_shift_val,
                  level_shift_val <= 24u32;
            } else if pos2.level as int == level as int + 2 {
              assert(shift2 == level_shift_val as int + 2 * WHEEL_BITS as int);
              assert(elapsed >> (level_shift_val + 16u32) == now >> (level_shift_val + 16u32)) by (bit_vector)
                requires elapsed >> level_shift_val == now >> level_shift_val,
                  level_shift_val <= 24u32;
            } else {
              assert(shift2 == level_shift_val as int + 3 * WHEEL_BITS as int);
              assert(elapsed >> (level_shift_val + 24u32) == now >> (level_shift_val + 24u32)) by (bit_vector)
                requires elapsed >> level_shift_val == now >> level_shift_val,
                  level_shift_val <= 24u32;
            }
          } else {
            assert(false);
          }
        }
      };
      assert(self.position_ahead());
      // coverage postcondition: if d <= now, entry must be in pending
      assert forall |rid: u64| self.deadlines@.contains_key(rid) && self.deadlines@[rid] <= now implies
        self.pending@.contains(rid)
      by {
        if !self.pending@.contains(rid) {
          assert(self.positions@.contains_key(rid));
          let pos = self.positions@[rid];
          let shift2 = (pos.level as u32) * WHEEL_BITS;
          assert(self.deadlines@[rid] >> shift2 > self.elapsed >> shift2);
          assert(self.elapsed == now);
          let d = self.deadlines@[rid] as u64;
          let n = now as u64;
          assert(d >> shift2 > n >> shift2);
          if shift2 == 0u32 {
            assert(d > n) by (bit_vector) requires d >> 0u32 > n >> 0u32;
          } else if shift2 == 8u32 {
            assert(d > n) by (bit_vector) requires d >> 8u32 > n >> 8u32;
          } else if shift2 == 16u32 {
            assert(d > n) by (bit_vector) requires d >> 16u32 > n >> 16u32;
          } else {
            assert(shift2 == 24u32);
            assert(d > n) by (bit_vector) requires d >> 24u32 > n >> 24u32;
          }
          assert(false);
        }
      };
      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = #[trigger] self.positions@[rid2];
          let shift2 = (pos2.level as u32) * WHEEL_BITS;
          pos2.slot as int == ((self.deadlines@[rid2] >> shift2) & 0xff) as int
        })
      by {
        assert(pre_elapsed_positions.contains_key(rid2));
        assert(self.positions@[rid2] == pre_elapsed_positions[rid2]);
        assert(pre_elapsed_pending.contains(rid2) == self.pending@.contains(rid2));
        assert(self.deadlines@[rid2] == pre_elapsed_deadlines[rid2]);
      };
      assert(self.slot_matches_deadline());
    }
  }

  #[inline]
  #[verifier::external_body]
  fn slot_push(&mut self, level: usize, slot: usize, rid: u64)
    requires
      old(self).structural_wf(),
      level < NUM_LEVELS,
      slot < WHEEL_SIZE,
    ensures
      self.structural_wf(),
      self.levels@[level as int]@[slot as int]@ == old(self).levels@[level as int]@[slot as int]@.push(rid),
      forall |l: int, s: int| 0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && (l != level as int || s != slot as int)
        ==> #[trigger] self.levels@[l]@[s]@ == old(self).levels@[l]@[s]@,
      forall |l: int| 0 <= l < NUM_LEVELS as int && l != level as int
        ==> self.levels@[l] == old(self).levels@[l],
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.positions@ == old(self).positions@,
      self.pending@ == old(self).pending@,
      self.elapsed == old(self).elapsed,
      self.cached_min == old(self).cached_min,
      self.level_counts@.len() == old(self).level_counts@.len(),
      self.level_counts@[level as int] as int == old(self).level_counts@[level as int] as int + 1,
      forall |l: int| 0 <= l < old(self).level_counts@.len() && l != level as int ==>
        #[trigger] self.level_counts@[l] == old(self).level_counts@[l],
  {
    self.levels[level][slot].push(rid);
    self.level_counts[level] += 1;
  }

  #[inline]
  #[verifier::external_body]
  fn slot_swap_remove(&mut self, level: usize, slot: usize, idx: usize) -> (result: u64)
    requires
      old(self).structural_wf(),
      level < NUM_LEVELS,
      slot < WHEEL_SIZE,
      idx < old(self).levels@[level as int]@[slot as int]@.len(),
      old(self).levels@[level as int]@[slot as int]@.len() > 0,
    ensures
      self.structural_wf(),
      result == old(self).levels@[level as int]@[slot as int]@[idx as int],
      ({
        let old_sv = old(self).levels@[level as int]@[slot as int]@;
        let last = old_sv.len() - 1;
        if idx < last {
          let new_sv = self.levels@[level as int]@[slot as int]@;
          new_sv.len() == old_sv.len() - 1
          && (forall |i: int| 0 <= i < new_sv.len() && i != idx as int ==> new_sv[i] == old_sv[i])
          && new_sv[idx as int] == old_sv[last as int]
        } else {
          self.levels@[level as int]@[slot as int]@.len() == old_sv.len() - 1
          && (forall |i: int| 0 <= i < self.levels@[level as int]@[slot as int]@.len()
              ==> self.levels@[level as int]@[slot as int]@[i] == old_sv[i])
        }
      }),
      forall |l: int, s: int| 0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && (l != level as int || s != slot as int)
        ==> #[trigger] self.levels@[l]@[s]@ == old(self).levels@[l]@[s]@,
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.positions@ == old(self).positions@,
      self.pending@ == old(self).pending@,
      self.elapsed == old(self).elapsed,
      self.cached_min == old(self).cached_min,
      self.level_counts@.len() == old(self).level_counts@.len(),
      self.level_counts@[level as int] as int == old(self).level_counts@[level as int] as int - 1,
      forall |l: int| 0 <= l < old(self).level_counts@.len() && l != level as int ==>
        #[trigger] self.level_counts@[l] == old(self).level_counts@[l],
  {
    self.level_counts[level] -= 1;
    self.levels[level][slot].swap_remove(idx)
  }

  #[inline]
  #[verifier::external_body]
  fn slot_pop(&mut self, level: usize, slot: usize)
    requires
      old(self).structural_wf(),
      level < NUM_LEVELS,
      slot < WHEEL_SIZE,
      old(self).levels@[level as int]@[slot as int]@.len() > 0,
    ensures
      self.structural_wf(),
      self.levels@[level as int]@[slot as int]@ == old(self).levels@[level as int]@[slot as int]@.drop_last(),
      forall |l: int, s: int| 0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && (l != level as int || s != slot as int)
        ==> #[trigger] self.levels@[l]@[s]@ == old(self).levels@[l]@[s]@,
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.positions@ == old(self).positions@,
      self.pending@ == old(self).pending@,
      self.elapsed == old(self).elapsed,
      self.cached_min == old(self).cached_min,
      self.level_counts@.len() == old(self).level_counts@.len(),
      self.level_counts@[level as int] as int == old(self).level_counts@[level as int] as int - 1,
      forall |l: int| 0 <= l < old(self).level_counts@.len() && l != level as int ==>
        #[trigger] self.level_counts@[l] == old(self).level_counts@[l],
  {
    self.level_counts[level] -= 1;
    self.levels[level][slot].pop();
  }

  #[inline]
  #[verifier::external_body]
  fn slot_drain(&mut self, level: usize, slot: usize) -> (result: Vec<u64>)
    requires
      old(self).structural_wf(),
      level < NUM_LEVELS,
      slot < WHEEL_SIZE,
    ensures
      self.structural_wf(),
      result@ == old(self).levels@[level as int]@[slot as int]@,
      self.levels@[level as int]@[slot as int]@.len() == 0,
      forall |l: int, s: int| 0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
        && (l != level as int || s != slot as int)
        ==> #[trigger] self.levels@[l]@[s]@ == old(self).levels@[l]@[s]@,
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.positions@ == old(self).positions@,
      self.pending@ == old(self).pending@,
      self.elapsed == old(self).elapsed,
      self.cached_min == old(self).cached_min,
      self.level_counts@.len() == old(self).level_counts@.len(),
      self.level_counts@[level as int] as int == old(self).level_counts@[level as int] as int - old(self).levels@[level as int]@[slot as int]@.len(),
      forall |l: int| 0 <= l < old(self).level_counts@.len() && l != level as int ==>
        #[trigger] self.level_counts@[l] == old(self).level_counts@[l],
  {
    self.level_counts[level] -= self.levels[level][slot].len() as u64;
    std::mem::take(&mut self.levels[level][slot])
  }

  fn wheel_remove_inner(&mut self, rid: u64)
    requires
      old(self).structural_wf(),
      old(self).pos_levels_consistent(),
      old(self).position_ahead(),
      old(self).slot_matches_deadline(),
      old(self).level_band(),
      old(self).level_counts_wf(),
      old(self).pending@.len() == 0,
    ensures
      self.structural_wf(),
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.elapsed == old(self).elapsed,
      self.pending@ == old(self).pending@,
      self.cached_min == old(self).cached_min,
      self.level_counts_wf(),
      !self.positions@.contains_key(rid),
      self.pos_levels_consistent(),
      self.position_ahead(),
      self.slot_matches_deadline(),
      self.level_band(),
      forall |r: u64| r != rid && old(self).positions@.contains_key(r) ==>
        self.positions@.contains_key(r),
      forall |r: u64| self.positions@.contains_key(r) ==>
        old(self).positions@.contains_key(r) && r != rid,
  {
    let ghost old_positions = self.positions@;
    let ghost old_levels = self.levels@;
    let ghost mut did_mutate = false;
    let ghost mut did_swap = false;
    let ghost mut swapped_rid_ghost: u64 = 0;
    let ghost mut removed_level: int = 0;
    let ghost mut removed_slot: int = 0;
    let ghost mut removed_idx: int = 0;
    let ghost mut had_pos = false;
    let ghost mut old_slot_vec: Seq<u64> = Seq::empty();
    let ghost mut new_slot_vec: Seq<u64> = Seq::empty();

    if let Some(pos) = self.positions.remove(&rid) {
      let level = pos.level as usize;
      let slot = pos.slot as usize;
      let idx = pos.idx as usize;

      proof {
        had_pos = true;
        removed_level = level as int;
        removed_slot = slot as int;
        removed_idx = idx as int;
        assert(old_positions.contains_key(rid));
        assert(old_positions[rid] == pos);
        assert((pos.level as int) < NUM_LEVELS);
        assert((pos.slot as int) < WHEEL_SIZE);
        assert((pos.idx as int) < old_levels[pos.level as int]@[pos.slot as int]@.len());
        assert(old_levels[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
      }

      if level < NUM_LEVELS && slot < WHEEL_SIZE {
        proof { old_slot_vec = self.levels@[level as int]@[slot as int]@; }

        let slot_len = self.levels[level][slot].len();
        if slot_len > 0 {
          let last = slot_len - 1;
          if idx <= last {
            if idx < last {
              let swapped_rid = self.levels[level][slot][last];
              self.slot_swap_remove(level, slot, idx);
              proof {
                did_mutate = true;
                did_swap = true;
                swapped_rid_ghost = swapped_rid;
                assert(old_slot_vec[idx as int] == rid);
                assert(swapped_rid == old_slot_vec[last as int]);
                assert(old_positions.contains_key(swapped_rid));
                let sp = old_positions[swapped_rid];
                assert(sp.level as int == level as int && sp.slot as int == slot as int && sp.idx as int == last as int);
                assert(swapped_rid != rid);
              }
              self.positions.insert(swapped_rid, WheelPos { level: pos.level, slot: pos.slot, idx: pos.idx });
            } else {
              proof { assert(removed_idx == old_slot_vec.len() as int - 1); }
              self.slot_pop(level, slot);
              proof { did_mutate = true; }
            }
          }
        }
        proof { new_slot_vec = self.levels@[level as int]@[slot as int]@; }
      }
    }

    proof {
      // level_counts_wf: at most one slot of one level lost one element
      assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
        (#[trigger] self.level_counts@[l]) as int ==
        Self::slots_len_sum(self.levels@[l]@, WHEEL_SIZE as int)
      by {
        if did_mutate && l == removed_level {
          Self::slots_len_sum_update(old_levels[l]@, self.levels@[l]@, removed_slot, WHEEL_SIZE as int);
        } else {
          Self::slots_len_sum_congruence(old_levels[l]@, self.levels@[l]@, WHEEL_SIZE as int);
        }
      };
      assert(self.level_counts_wf());

      // structural_wf
      assert(self.levels@.len() == NUM_LEVELS as int);
      assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
        (#[trigger] self.levels@[l])@.len() == WHEEL_SIZE as int
      by {};

      if !had_pos {
        // rid was not in old positions — nothing changed
        assert(self.positions@ == old_positions.remove(rid));
        assert(self.levels@ == old_levels);
      }

      // !positions.contains_key(rid)
      if had_pos && did_swap {
        assert(self.positions@ == old_positions.remove(rid).insert(swapped_rid_ghost,
          WheelPos { level: removed_level as u8, slot: removed_slot as u8, idx: removed_idx as u64 }));
        assert(swapped_rid_ghost != rid);
        assert(!self.positions@.contains_key(rid));
      }

      if !had_pos {
        assert(!old_positions.contains_key(rid));
        assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
          !self.pending@.contains(rid2) implies ({
            let pos2 = #[trigger] self.positions@[rid2];
            &&& (pos2.level as int) < NUM_LEVELS
            &&& (pos2.slot as int) < WHEEL_SIZE
            &&& (pos2.idx as int) < self.levels@[pos2.level as int]@[pos2.slot as int]@.len()
            &&& self.levels@[pos2.level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2
          })
        by {
          assert(old_positions.contains_key(rid2));
          assert(self.positions@[rid2] == old_positions[rid2]);
        };
        assert forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len() implies
          self.positions@.contains_key(self.levels@[l]@[s]@[i])
        by {
          assert(old_positions.contains_key(old_levels[l]@[s]@[i]));
          assert(old_levels[l]@[s]@[i] != rid);
        };
        assert forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len()
          && self.positions@.contains_key(self.levels@[l]@[s]@[i])
          && !self.pending@.contains(self.levels@[l]@[s]@[i])
          implies ({
            let rid2 = self.levels@[l]@[s]@[i];
            let pos2 = self.positions@[rid2];
            pos2.level as int == l && pos2.slot as int == s && pos2.idx as int == i
          })
        by {
          let rid2 = self.levels@[l]@[s]@[i];
          assert(rid2 != rid);
          assert(self.positions@[rid2] == old_positions[rid2]);
        };
      } else if !did_swap {
        assert(self.positions@ =~= old_positions.remove(rid));
        assert(removed_idx == old_slot_vec.len() as int - 1);
        assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
          !self.pending@.contains(rid2) implies ({
            let pos2 = #[trigger] self.positions@[rid2];
            &&& (pos2.level as int) < NUM_LEVELS
            &&& (pos2.slot as int) < WHEEL_SIZE
            &&& (pos2.idx as int) < self.levels@[pos2.level as int]@[pos2.slot as int]@.len()
            &&& self.levels@[pos2.level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2
          })
        by {
          assert(rid2 != rid);
          assert(old_positions.contains_key(rid2));
          assert(self.positions@[rid2] == old_positions[rid2]);
          let pos2 = old_positions[rid2];
          if pos2.level as int == removed_level && pos2.slot as int == removed_slot {
            assert(pos2.idx as int != removed_idx);
          }
        };
        assert forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len() implies
          self.positions@.contains_key(self.levels@[l]@[s]@[i])
        by {
          if l == removed_level && s == removed_slot {
            assert(new_slot_vec[i] == old_slot_vec[i]);
            assert(old_levels[removed_level]@[removed_slot]@[i] == old_slot_vec[i]);
            assert(old_positions.contains_key(old_slot_vec[i]));
            if old_slot_vec[i] == rid {
              assert(old_positions[rid].idx as int == removed_idx);
              assert(old_positions[old_slot_vec[i]].idx as int == i);
              assert(false);
            }
          } else {
            assert(old_positions.contains_key(old_levels[l]@[s]@[i]));
            assert(old_levels[l]@[s]@[i] != rid);
          }
        };
        assert forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len()
          && self.positions@.contains_key(self.levels@[l]@[s]@[i])
          && !self.pending@.contains(self.levels@[l]@[s]@[i])
          implies ({
            let rid2 = self.levels@[l]@[s]@[i];
            let pos2 = self.positions@[rid2];
            pos2.level as int == l && pos2.slot as int == s && pos2.idx as int == i
          })
        by {
          let rid2 = self.levels@[l]@[s]@[i];
          assert(rid2 != rid);
          assert(self.positions@[rid2] == old_positions[rid2]);
          if l == removed_level && s == removed_slot {
            assert(rid2 == old_slot_vec[i]);
          }
        };
      } else {
        assert(new_slot_vec[removed_idx as int] == swapped_rid_ghost);
        assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
          !self.pending@.contains(rid2) implies ({
            let pos2 = #[trigger] self.positions@[rid2];
            &&& (pos2.level as int) < NUM_LEVELS
            &&& (pos2.slot as int) < WHEEL_SIZE
            &&& (pos2.idx as int) < self.levels@[pos2.level as int]@[pos2.slot as int]@.len()
            &&& self.levels@[pos2.level as int]@[pos2.slot as int]@[pos2.idx as int] == rid2
          })
        by {
          assert(rid2 != rid);
          if rid2 == swapped_rid_ghost {
            assert(self.positions@[rid2] == WheelPos {
              level: removed_level as u8, slot: removed_slot as u8, idx: removed_idx as u64
            });
          } else {
            assert(old_positions.contains_key(rid2));
            assert(self.positions@[rid2] == old_positions[rid2]);
            let pos2 = old_positions[rid2];
            if pos2.level as int == removed_level && pos2.slot as int == removed_slot {
              assert(pos2.idx as int != removed_idx);
              assert(pos2.idx as int != old_slot_vec.len() as int - 1);
            }
          }
        };
        assert forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len() implies
          self.positions@.contains_key(self.levels@[l]@[s]@[i])
        by {
          if l == removed_level && s == removed_slot {
            if i == removed_idx as int {
              assert(new_slot_vec[i] == swapped_rid_ghost);
            } else {
              assert(new_slot_vec[i] == old_slot_vec[i]);
              assert(old_levels[removed_level]@[removed_slot]@[i] == old_slot_vec[i]);
              assert(old_positions.contains_key(old_slot_vec[i]));
              if old_slot_vec[i] == rid {
                assert(old_positions[rid].idx as int == removed_idx);
                assert(old_positions[old_slot_vec[i]].idx as int == i);
                assert(false);
              }
            }
          } else {
            assert(old_positions.contains_key(old_levels[l]@[s]@[i]));
            assert(old_levels[l]@[s]@[i] != rid);
          }
        };
        assert forall |l: int, s: int, i: int|
          #![trigger self.levels@[l]@[s]@[i]]
          0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int
          && 0 <= i < self.levels@[l]@[s]@.len()
          && self.positions@.contains_key(self.levels@[l]@[s]@[i])
          && !self.pending@.contains(self.levels@[l]@[s]@[i])
          implies ({
            let rid2 = self.levels@[l]@[s]@[i];
            let pos2 = self.positions@[rid2];
            pos2.level as int == l && pos2.slot as int == s && pos2.idx as int == i
          })
        by {
          let rid2 = self.levels@[l]@[s]@[i];
          if l == removed_level && s == removed_slot {
            if i == removed_idx as int {
              assert(rid2 == swapped_rid_ghost);
              assert(self.positions@[rid2] == WheelPos {
                level: removed_level as u8, slot: removed_slot as u8, idx: removed_idx as u64
              });
            } else {
              assert(rid2 == old_slot_vec[i]);
              assert(rid2 != rid);
              if rid2 == swapped_rid_ghost {
                assert(old_positions[swapped_rid_ghost].idx as int == old_slot_vec.len() as int - 1);
                assert(old_positions[old_slot_vec[i]].idx as int == i);
                assert(i == old_slot_vec.len() as int - 1);
                assert(i < new_slot_vec.len() as int);
                assert(new_slot_vec.len() as int == old_slot_vec.len() as int - 1);
                assert(false);
              }
              assert(self.positions@[rid2] == old_positions[rid2]);
            }
          } else {
            assert(rid2 == old_levels[l]@[s]@[i]);
            assert(rid2 != rid);
            if rid2 == swapped_rid_ghost {
              assert(old_positions[swapped_rid_ghost].level as int == removed_level);
              assert(old_positions[swapped_rid_ghost].slot as int == removed_slot);
              assert(old_positions[old_levels[l]@[s]@[i]].level as int == l);
              assert(old_positions[old_levels[l]@[s]@[i]].slot as int == s);
              assert(false);
            }
            assert(self.positions@[rid2] == old_positions[rid2]);
          }
        };
      }

      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = self.positions@[rid2];
          let shift = (pos2.level as u32) * WHEEL_BITS;
          #[trigger] self.deadlines@[rid2] >> shift > self.elapsed >> shift
        })
      by {
        assert(old_positions.contains_key(rid2));
        assert(rid2 != rid);
        let old_pos2 = old_positions[rid2];
        assert(!old(self).pending@.contains(rid2));
        assert(old(self).position_ahead());
        assert(old(self).deadlines@[rid2] >> ((old_pos2.level as u32) * WHEEL_BITS) > old(self).elapsed >> ((old_pos2.level as u32) * WHEEL_BITS));
        if had_pos && did_swap && rid2 == swapped_rid_ghost {
          assert(self.positions@[rid2].level as int == removed_level);
          assert(old_pos2.level as int == removed_level);
        } else {
          assert(self.positions@[rid2] == old_positions[rid2]);
        }
      };

      assert forall |rid2: u64| self.positions@.contains_key(rid2) &&
        !self.pending@.contains(rid2) implies ({
          let pos2 = #[trigger] self.positions@[rid2];
          let shift = (pos2.level as u32) * WHEEL_BITS;
          pos2.slot as int == ((self.deadlines@[rid2] >> shift) & 0xff) as int
        })
      by {
        assert(old_positions.contains_key(rid2));
        assert(rid2 != rid);
        assert(!old(self).pending@.contains(rid2));
        assert(old(self).slot_matches_deadline());
        let old_pos2 = old_positions[rid2];
        if had_pos && did_swap && rid2 == swapped_rid_ghost {
          assert(self.positions@[rid2].level as int == removed_level);
          assert(self.positions@[rid2].slot as int == removed_slot as int);
          assert(old_pos2.level as int == removed_level);
          assert(old_pos2.slot as int == removed_slot as int);
        } else {
          assert(self.positions@[rid2] == old_positions[rid2]);
        }
      };
    }
  }

  fn wheel_insert_inner(&mut self, rid: u64, deadline: u64, elapsed_for_slot: u64)
    requires
      old(self).structural_wf(),
      deadline >= elapsed_for_slot,
    ensures
      self.structural_wf(),
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.elapsed == old(self).elapsed,
      self.cached_min == old(self).cached_min,
      self.pending@ == old(self).pending@,
      self.positions@.contains_key(rid),
      self.level_counts@.len() == old(self).level_counts@.len(),
      self.level_counts@[Self::spec_level_slot(deadline, elapsed_for_slot).0] as int ==
        old(self).level_counts@[Self::spec_level_slot(deadline, elapsed_for_slot).0] as int + 1,
      forall |l: int| 0 <= l < old(self).level_counts@.len() &&
        l != Self::spec_level_slot(deadline, elapsed_for_slot).0 ==>
        #[trigger] self.level_counts@[l] == old(self).level_counts@[l],
      self.positions@[rid].level as int == Self::spec_level_slot(deadline, elapsed_for_slot).0,
      self.positions@[rid].slot as int == Self::spec_level_slot(deadline, elapsed_for_slot).1,
      forall |rid2: u64| rid2 != rid && old(self).positions@.contains_key(rid2) ==>
        self.positions@.contains_key(rid2),
      forall |rid2: u64| rid2 != rid && old(self).positions@.contains_key(rid2) ==>
        self.positions@[rid2] == old(self).positions@[rid2],
      forall |rid2: u64| self.positions@.contains_key(rid2) ==>
        rid2 == rid || old(self).positions@.contains_key(rid2),
      ({
        let tl = Self::spec_level_slot(deadline, elapsed_for_slot).0;
        let ts = Self::spec_level_slot(deadline, elapsed_for_slot).1;
        &&& self.positions@[rid].idx as int == old(self).levels@[tl]@[ts]@.len()
        &&& self.levels@[tl]@[ts]@ =~= old(self).levels@[tl]@[ts]@.push(rid)
        &&& forall |l: int, s: int| 0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int &&
              !(l == tl && s == ts) ==> self.levels@[l]@[s]@ =~= old(self).levels@[l]@[s]@
      }),
  {
    let (level, slot) = Self::level_slot(deadline, elapsed_for_slot);
    let ghost tl = Self::spec_level_slot(deadline, elapsed_for_slot).0;
    let ghost ts = Self::spec_level_slot(deadline, elapsed_for_slot).1;
    let ghost old_levels = self.levels@;
    let idx = self.levels[level][slot].len() as u64;
    self.slot_push(level, slot, rid);
    self.positions.insert(rid, WheelPos { level: level as u8, slot: slot as u8, idx });
    proof {
      assert(self.levels@.len() == NUM_LEVELS as int);
      assert forall |l: int| 0 <= l < NUM_LEVELS as int implies
        (#[trigger] self.levels@[l])@.len() == WHEEL_SIZE as int
      by {
        if l == level as int {
        } else if l < level as int {
          assert(self.levels@[l] == old(self).levels@[l]);
        } else {
          assert(self.levels@[l] == old(self).levels@[l]);
        }
      };
      assert forall |l: int, s: int| 0 <= l < NUM_LEVELS as int && 0 <= s < WHEEL_SIZE as int &&
        !(l == tl && s == ts) implies self.levels@[l]@[s]@ =~= old(self).levels@[l]@[s]@
      by {
        if l == level as int {
        } else {
          assert(self.levels@[l] == old(self).levels@[l]);
        }
      };
    }
  }

  // Merge a found deadline into the running minimum.
  // Ring position: a timer whose shifted deadline sits `o` ticks ahead of
  // the shifted elapsed lands in the slot `o` ring steps after the current one.
  proof fn lemma_ring_slot(d: u64, e: u64, shift: u32, o: u64)
    requires
      shift <= 16,
      1 <= o, o <= 256,
      d >> shift >= e >> shift,
      (d >> shift) - (e >> shift) == o,
    ensures ((d >> shift) & 0xff) == (((e >> shift) & 255) + o) % 256,
  {
    assert(((d >> shift) & 0xff) == (((e >> shift) & 255) + o) % 256) by (bit_vector)
      requires shift <= 16, 1 <= o, o <= 256, d >> shift >= e >> shift,
        (d >> shift) - (e >> shift) == o;
  }

  // Inverse ring position: within one ring rotation, the masked slot
  // determines the ring diff uniquely.
  proof fn lemma_ring_slot_inv(d: u64, e: u64, shift: u32, o: u64)
    requires
      shift <= 16,
      1 <= o, o <= 256,
      d >> shift >= e >> shift,
      1 <= (d >> shift) - (e >> shift), (d >> shift) - (e >> shift) <= 256,
      ((d >> shift) & 0xff) == (((e >> shift) & 255) + o) % 256,
    ensures (d >> shift) - (e >> shift) == o,
  {
    assert((d >> shift) - (e >> shift) == o) by (bit_vector)
      requires shift <= 16, 1 <= o, o <= 256, d >> shift >= e >> shift,
        1 <= (d >> shift) - (e >> shift), (d >> shift) - (e >> shift) <= 256,
        ((d >> shift) & 0xff) == (((e >> shift) & 255) + o) % 256;
  }

  // Shift granularity: a strictly larger shifted value means a strictly
  // larger value.
  proof fn lemma_diff_order(d2: u64, db: u64, shift: u32)
    requires shift <= 24, (d2 >> shift) > (db >> shift),
    ensures d2 > db,
  {
    assert(d2 > db) by (bit_vector)
      requires shift <= 24, (d2 >> shift) > (db >> shift);
  }

  // Every banded-level timer has ring diff in [1, 256].
  proof fn lemma_at_level_diff_range(&self, rid: u64, level: usize)
    requires
      self.full_wf(),
      level < 3,
      self.at_level(rid, level as int),
    ensures
      self.deadlines@.contains_key(rid),
      1 <= (self.deadlines@[rid] >> ((level as u32) * WHEEL_BITS)) as int
        - (self.elapsed >> ((level as u32) * WHEEL_BITS)) as int,
      (self.deadlines@[rid] >> ((level as u32) * WHEEL_BITS)) as int
        - (self.elapsed >> ((level as u32) * WHEEL_BITS)) as int <= 256,
  {
    assert(self.deadlines@.contains_key(rid));
    let d = self.deadlines@[rid];
    let e = self.elapsed;
    assert(d >> ((level as u32) * WHEEL_BITS) > e >> ((level as u32) * WHEEL_BITS));
    assert((d as int - e as int) < ((1u64 << ((level as u32 + 1) * WHEEL_BITS)) as int));
    if level == 0 {
      Self::lemma_diff_order(d, e, 0);
      assert(d - e < (1u64 << 8u32));
      assert((d >> 0u32) - (e >> 0u32) <= 256) by (bit_vector)
        requires d > e, d - e < (1u64 << 8u32);
    } else if level == 1 {
      Self::lemma_diff_order(d, e, 8);
      assert(d - e < (1u64 << 16u32));
      assert((d >> 8u32) - (e >> 8u32) <= 256) by (bit_vector)
        requires d >= e, d - e < (1u64 << 16u32);
    } else {
      Self::lemma_diff_order(d, e, 16);
      assert(d - e < (1u64 << 24u32));
      assert((d >> 16u32) - (e >> 16u32) <= 256) by (bit_vector)
        requires d >= e, d - e < (1u64 << 24u32);
    }
  }

  // A wheel-resident (non-pending) timer registered at `level`.
  pub open spec fn at_level(&self, rid: u64, level: int) -> bool {
    &&& self.positions@.contains_key(rid)
    &&& !self.pending@.contains(rid)
    &&& self.positions@[rid].level as int == level
  }

  // Ring-order scan for one banded level (0..=2): visit slots at offsets
  // 1..=256 from the current position; the FIRST occupied slot holds exactly
  // the timers with the smallest ring diff, and (by level_band +
  // position_ahead + slot_matches_deadline) ring order equals deadline
  // order within the level, so its minimum is the level minimum.
  // `bound`: prefilter — when Some(b), the caller already holds a candidate
  // b and only needs this level's minimum if it could be smaller. The scan
  // may then return None as soon as the first occupied slot's ring position
  // proves every member exceeds b (shifted-domain comparison, no overflow),
  // skipping the member scan entirely.
  fn scan_level_min(&self, level: usize, bound: Option<u64>) -> (result: Option<u64>)
    requires self.full_wf(), level < 3, bound is Some ==> 1 <= level,
    ensures
      result is Some ==> {
        &&& exists |rid: u64| #![auto] self.at_level(rid, level as int) &&
              self.deadlines@.contains_key(rid) && self.deadlines@[rid] == result->0
        &&& forall |rid: u64| #![auto] self.at_level(rid, level as int) &&
              self.deadlines@.contains_key(rid) ==> result->0 <= self.deadlines@[rid]
      },
      result is None ==> forall |rid: u64| #![auto] self.at_level(rid, level as int) ==>
        bound is Some && self.deadlines@[rid] > bound->0,
  {
    let shift: u32 = (level as u32) * WHEEL_BITS;
    let e = self.elapsed;
    let cur: u64 = (e >> shift) & 255;

    // O(1) empty-level skip: the verified per-level counter mirrors true
    // occupancy, so zero means no timer lives anywhere in this level.
    if self.level_counts[level] == 0 {
      proof {
        assert forall |rid: u64| #![auto] !self.at_level(rid, level as int) by {
          if self.at_level(rid, level as int) {
            let pos = self.positions@[rid];
            assert(self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
            Self::slots_len_sum_lower(self.levels@[level as int]@, pos.slot as int, WHEEL_SIZE as int);
          }
        };
      }
      return None;
    }

    proof {
      assert(cur < 256) by (bit_vector) requires cur == (e >> shift) & 255;
      assert forall |rid: u64| #![auto] self.at_level(rid, level as int) implies
        (self.deadlines@[rid] >> shift) as int - (e >> shift) as int >= 1
      by {
        self.lemma_at_level_diff_range(rid, level);
      };
    }

    let mut o: u64 = 1;
    while o <= 256
      invariant
        1 <= o <= 257,
        self.full_wf(),
        level < 3,
        bound is Some ==> 1 <= level,
        shift == (level as u32) * WHEEL_BITS,
        e == self.elapsed,
        cur == (e >> shift) & 255,
        cur < 256,
        forall |rid: u64| #![auto] self.at_level(rid, level as int) ==>
          (self.deadlines@[rid] >> shift) as int - (e >> shift) as int >= o as int,
      decreases 257 - o,
    {
      let slot: usize = (((cur + o) % 256) as usize);
      proof { assert(cur + o < 512 && (cur + o) % 256 < 256) by (bit_vector) requires cur < 256, o <= 256; }
      let slot_vec = &self.levels[level][slot];
      if slot_vec.len() > 0 {
        // Prefilter: if the caller's candidate already beats this slot's
        // shifted-domain lower bound, every member of this level exceeds it
        // (members at this or any later offset have diff >= o), so the whole
        // member scan is unnecessary. Shifted comparison — no overflow.
        if let Some(b) = bound {
          proof {
            assert(1 <= level && level < 3);
            if level == 1 {
              assert(shift == 1u32 * WHEEL_BITS);
            } else {
              assert(level == 2);
              assert(shift == 2u32 * WHEEL_BITS);
            }
            assert(shift >= 8);
            assert((e >> shift) + o <= u64::MAX) by (bit_vector)
              requires shift >= 8, o <= 256;
          }
          if (b >> shift) < (e >> shift) + o {
            proof {
              assert forall |rid: u64| #![auto] self.at_level(rid, level as int) implies
                bound is Some && self.deadlines@[rid] > bound->0
              by {
                self.lemma_at_level_diff_range(rid, level);
                let d = self.deadlines@[rid];
                assert((d >> shift) as int - (e >> shift) as int >= o as int);
                assert((d >> shift) as int >= (e >> shift) as int + o as int);
                assert((d >> shift) > (b >> shift));
                Self::lemma_diff_order(d, b, shift);
              };
            }
            return None;
          }
        }
        // Level-0 shortcut: at shift 0 every member of this slot has diff
        // EXACTLY o, i.e. deadline == elapsed + o — all members share one
        // deadline, so the first member is the slot minimum (and, with the
        // gap-loop invariant, the level minimum). No member scan needed.
        if level == 0 {
          let rid0 = slot_vec[0];
          proof {
            assert(self.levels@[0int]@[slot as int]@[0int] == rid0);
            assert(self.positions@.contains_key(rid0));
            assert(!self.pending@.contains(rid0));
            assert(self.deadlines@.contains_key(rid0));
            assert(self.at_level(rid0, 0int));
          }
          if let Some(d0) = self.deadlines.get(&rid0) {
            let d0 = *d0;
            proof {
              assert(shift == 0u32);
              let pos0 = self.positions@[rid0];
              assert(pos0.level as int == 0 && pos0.slot as int == slot as int);
              assert(pos0.slot as int == ((d0 >> 0u32) & 0xff) as int);
              self.lemma_at_level_diff_range(rid0, 0);
              assert(cur == (e >> 0u32) & 255);
              assert(((d0 >> 0u32) & 0xff) as int == ((((e >> 0u32) & 255) + o) % 256) as int) by (bit_vector)
                requires ((d0 >> 0u32) & 0xff) as int == slot as int,
                  slot as int == ((cur + o) % 256) as int,
                  cur == (e >> 0u32) & 255;
              Self::lemma_ring_slot_inv(d0, e, 0u32, o);
              let diff0 = (d0 >> 0u32) as int - (e >> 0u32) as int;
              assert(diff0 == o as int);
              assert forall |rid: u64| #![auto] self.at_level(rid, 0int) &&
                self.deadlines@.contains_key(rid) implies d0 <= self.deadlines@[rid]
              by {
                self.lemma_at_level_diff_range(rid, 0);
                let d2 = self.deadlines@[rid];
                let diff2 = (d2 >> 0u32) as int - (e >> 0u32) as int;
                assert(diff2 >= o as int);
                assert((d0 >> 0u32) == d0 && (d2 >> 0u32) == d2 && (e >> 0u32) == e) by (bit_vector);
                if diff2 == o as int {
                  assert(d2 == d0);
                } else {
                  assert(d2 as int - e as int > d0 as int - e as int);
                }
              };
              assert(self.at_level(rid0, 0int) &&
                self.deadlines@.contains_key(rid0) && self.deadlines@[rid0] == d0);
            }
            return Some(d0);
          }
          proof {
            // occupied slot but no deadline entry: impossible (dl_pos_consistent)
            assert(self.deadlines@.contains_key(rid0));
            assert(false);
          }
        }
        // slot occupied: its members are exactly the diff == o timers
        let mut best: u64 = 0;
        let mut have: bool = false;
        let mut idx: usize = 0;
        while idx < slot_vec.len()
          invariant
            idx <= slot_vec@.len(),
            self.full_wf(),
            level < 3,
            slot < WHEEL_SIZE,
            shift == (level as u32) * WHEEL_BITS,
            e == self.elapsed,
            slot_vec@ == self.levels@[level as int]@[slot as int]@,
            have ==> exists |k: int| #![auto] 0 <= k < idx as int &&
              self.deadlines@.contains_key(slot_vec@[k]) &&
              self.deadlines@[slot_vec@[k]] == best,
            forall |k: int| #![trigger slot_vec@[k]] 0 <= k < idx as int &&
              self.deadlines@.contains_key(slot_vec@[k]) ==>
              have && best <= self.deadlines@[slot_vec@[k]],
          decreases slot_vec@.len() - idx,
        {
          let rid = slot_vec[idx];
          proof {
            assert(self.levels@[level as int]@[slot as int]@[idx as int] == rid);
            assert(self.positions@.contains_key(rid));
            assert(!self.pending@.contains(rid));
            assert(self.positions@[rid].level as int == level as int);
            assert(self.deadlines@.contains_key(rid));
          }
          if let Some(d) = self.deadlines.get(&rid) {
            let d = *d;
            if !have || d < best {
              best = d;
              have = true;
              proof {
                assert(self.deadlines@[rid] == best);
                assert(0 <= idx as int && (idx as int) < idx as int + 1 &&
                  self.deadlines@.contains_key(slot_vec@[idx as int]) &&
                  self.deadlines@[slot_vec@[idx as int]] == best);
              }
            }
          }
          idx = idx + 1;
        }
        if have {
          proof {
            // the winning entry lives in the scanned slot, so its ring diff
            // is exactly o; every other at-level timer has diff >= o, and a
            // strictly larger diff means a strictly larger deadline
            let kw = choose |k: int| #![auto] 0 <= k < slot_vec@.len() &&
              self.deadlines@.contains_key(slot_vec@[k]) &&
              self.deadlines@[slot_vec@[k]] == best;
            let bw = slot_vec@[kw];
            assert(self.levels@[level as int]@[slot as int]@[kw] == bw);
            assert(self.positions@.contains_key(bw));
            assert(!self.pending@.contains(bw));
            assert(self.positions@[bw].level as int == level as int);
            assert(self.at_level(bw, level as int));
            let db = self.deadlines@[bw];
            assert(db == best);
            self.lemma_at_level_diff_range(bw, level);
            let diffb = (db >> shift) as int - (e >> shift) as int;
            assert(1 <= diffb && diffb <= 256);
            assert(self.positions@[bw].slot as int == ((db >> shift) & 0xff) as int);
            assert(self.positions@[bw].slot as int == slot as int);
            assert((db >> shift) as int >= (e >> shift) as int);
            assert(((db >> shift) & 0xff) as int == ((((e >> shift) & 255) + o) % 256) as int) by {
              assert(slot as int == (((cur + o) % 256)) as int);
              assert(cur == (e >> shift) & 255);
            }
            Self::lemma_ring_slot_inv(db, e, shift, o);
            assert(diffb == o as int);
            assert forall |rid: u64| #![auto] self.at_level(rid, level as int) &&
              self.deadlines@.contains_key(rid) implies best <= self.deadlines@[rid]
            by {
              self.lemma_at_level_diff_range(rid, level);
              let d2 = self.deadlines@[rid];
              let diff2 = (d2 >> shift) as int - (e >> shift) as int;
              assert(diff2 >= o as int);
              assert(self.positions@[rid].slot as int == ((d2 >> shift) & 0xff) as int);
              if diff2 == o as int {
                // same diff => same slot => rid was scanned in this slot
                assert((d2 >> shift) as int == (db >> shift) as int);
                assert(self.positions@[rid].slot as int == slot as int);
                let pos = self.positions@[rid];
                assert(self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
                assert(0 <= pos.idx as int && (pos.idx as int) < slot_vec@.len());
                assert(slot_vec@[pos.idx as int] == rid);
              } else {
                assert(diff2 > diffb);
                assert((d2 >> shift) > (db >> shift));
                Self::lemma_diff_order(d2, db, shift);
              }
            };
            assert(self.at_level(bw, level as int) &&
              self.deadlines@.contains_key(bw) && self.deadlines@[bw] == best);
          }
          return Some(best);
        }
        proof {
          // occupied slot but no deadline entries: impossible under
          // dl_pos_consistent (positions keys have deadlines)
          let rid0 = slot_vec@[0];
          assert(self.positions@.contains_key(rid0));
          assert(self.deadlines@.contains_key(rid0));
          assert(slot_vec@.len() > 0);
          assert(false);
        }
      }
      proof {
        // slot at offset o empty: no at-level rid has diff == o
        assert forall |rid: u64| #![auto] self.at_level(rid, level as int) implies
          (self.deadlines@[rid] >> shift) as int - (e >> shift) as int >= o as int + 1
        by {
          self.lemma_at_level_diff_range(rid, level);
          let d = self.deadlines@[rid];
          let diff = (d >> shift) as int - (e >> shift) as int;
          assert(diff >= o as int);
          if diff == o as int {
            Self::lemma_ring_slot(d, e, shift, o);
            assert(self.positions@[rid].slot as int == ((d >> shift) & 0xff) as int);
            assert(self.positions@[rid].slot as int == slot as int);
            let pos = self.positions@[rid];
            assert(self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
            assert(self.levels@[level as int]@[slot as int]@.len() > 0);
            assert(false);
          }
        };
      }
      o = o + 1;
    }
    proof {
      // all 256 offsets empty: level is empty (any at-level rid would have
      // diff in [1,256] >= 257 — contradiction)
      assert forall |rid: u64| #![auto] !self.at_level(rid, level as int) by {
        if self.at_level(rid, level as int) {
          self.lemma_at_level_diff_range(rid, level);
          let d = self.deadlines@[rid];
          assert((d >> shift) as int - (e >> shift) as int <= 256);
          assert((d >> shift) as int - (e >> shift) as int >= 257);
        }
      };
    }
    None
  }

  fn merge_min(best: Option<u64>, d: u64) -> (result: Option<u64>)
    ensures
      result is Some,
      result->0 <= d,
      best is Some ==> result->0 <= best->0,
      result->0 == d || (best is Some && result->0 == best->0),
  {
    match best {
      None => Some(d),
      Some(b) => if d < b { Some(d) } else { Some(b) },
    }
  }

  // VERIFIED fast fallback (only reached when cached_min is None):
  //   1. pending first — pending deadlines are <= elapsed while every
  //      wheel-resident deadline is > elapsed, so a pending entry dominates;
  //   2. levels 0..2 — ring-order early-exit per level (sound by level_band);
  //   3. level 3 — exhaustive (no band there);
  // and the minimum of the <= 4 candidates is the global minimum.
  fn scan_wheel_min(&self) -> (result: Option<u64>)
    requires self.full_wf(),
    ensures
      result.is_some() ==> {
        let d = result.unwrap();
        (exists |r: nat| #![auto] self@.contains_key(r) && self@[r] == d as int) &&
        (forall |r: nat| #![auto] self@.contains_key(r) ==> d as int <= self@[r])
      },
      result.is_none() ==> self@ == Map::<nat, int>::empty(),
  {
    let mut pbest: Option<u64> = None;
    let mut p: usize = 0;
    while p < self.pending.len()
      invariant
        p <= self.pending@.len(),
        pbest is Some ==> exists |r: nat| #![auto] self@.contains_key(r) && self@[r] == pbest->0 as int,
        pbest is Some ==> exists |rid: u64| #![auto] self.pending@.contains(rid) &&
          self.deadlines@.contains_key(rid) && self.deadlines@[rid] == pbest->0,
        forall |j: int| #![trigger self.pending@[j]] 0 <= j < p as int &&
          self.deadlines@.contains_key(self.pending@[j]) ==>
          pbest is Some && pbest->0 <= self.deadlines@[self.pending@[j]],
      decreases self.pending@.len() - p,
    {
      let rid = self.pending[p];
      if let Some(d) = self.deadlines.get(&rid) {
        let d = *d;
        proof {
          assert(self@.contains_key(rid as nat));
          assert(self@[rid as nat] == d as int);
          assert(self.pending@.contains(rid));
        }
        pbest = Self::merge_min(pbest, d);
      }
      p = p + 1;
    }

    if let Some(pb) = pbest {
      proof {
        // pb <= elapsed (pending_expired); every wheel-resident deadline is
        // > elapsed (position_ahead + shift granularity) — pb dominates all
        let pw = choose |rid: u64| #![auto] self.pending@.contains(rid) &&
          self.deadlines@.contains_key(rid) && self.deadlines@[rid] == pb;
        assert(pb <= self.elapsed);
        assert forall |r: nat| self@.contains_key(r) implies pb as int <= self@[r] by {
          let rid = r as u64;
          assert(self.deadlines@.contains_key(rid));
          if self.pending@.contains(rid) {
            let j = choose |j: int| 0 <= j < self.pending@.len() && self.pending@[j] == rid;
            assert(self.pending@[j] == rid);
          } else {
            assert(self.positions@.contains_key(rid));
            let pos = self.positions@[rid];
            let shift = (pos.level as u32) * WHEEL_BITS;
            let d = self.deadlines@[rid];
            assert(d >> shift > self.elapsed >> shift);
            assert(shift <= 24) by { assert((pos.level as int) < NUM_LEVELS); }
            Self::lemma_diff_order(d, self.elapsed, shift as u32);
            assert(d > self.elapsed);
          }
        };
      }
      return Some(pb);
    }

    // no pending entry carries a deadline: every map key is wheel-resident
    let c0 = self.scan_level_min(0, None);
    // Levels 1-2 are prefiltered against the level-0 candidate: when c0
    // already undercuts the slot's shifted lower bound the member scan is
    // skipped (None then means "every member exceeds c0", not "empty").
    let c1 = self.scan_level_min(1, c0);
    let c2 = self.scan_level_min(2, c0);
    let c3 = self.scan_level3_min();

    let mut best: Option<u64> = None;
    if let Some(d) = c0 { best = Self::merge_min(best, d); }
    if let Some(d) = c1 { best = Self::merge_min(best, d); }
    if let Some(d) = c2 { best = Self::merge_min(best, d); }
    if let Some(d) = c3 { best = Self::merge_min(best, d); }
    proof {
      assert(c0 is Some ==> best is Some && best->0 <= c0->0);
    }

    proof {
      assert forall |r: nat| self@.contains_key(r) implies ({
        &&& best is Some
        &&& best->0 as int <= self@[r]
      }) by {
        let rid = r as u64;
        assert(self.deadlines@.contains_key(rid));
        if self.pending@.contains(rid) {
          let j = choose |j: int| 0 <= j < self.pending@.len() && self.pending@[j] == rid;
          assert(self.pending@[j] == rid);
          assert(false);
        }
        assert(self.positions@.contains_key(rid));
        let lev = self.positions@[rid].level as int;
        assert(0 <= lev < NUM_LEVELS as int);
        assert(self.at_level(rid, lev));
        if lev == 0 {
          assert(c0 is Some && c0->0 <= self.deadlines@[rid]);
        } else if lev == 1 {
          if c1 is Some {
            assert(c1->0 <= self.deadlines@[rid]);
          } else {
            // prefiltered: every level-1 member exceeds the level-0 candidate
            assert(c0 is Some && self.deadlines@[rid] > c0->0);
            assert(best is Some && best->0 <= c0->0);
          }
        } else if lev == 2 {
          if c2 is Some {
            assert(c2->0 <= self.deadlines@[rid]);
          } else {
            assert(c0 is Some && self.deadlines@[rid] > c0->0);
            assert(best is Some && best->0 <= c0->0);
          }
        } else {
          assert(lev == 3);
          assert(c3 is Some && c3->0 <= self.deadlines@[rid]);
        }
      };
      match best {
        Some(b) => {
          assert(exists |r: nat| #![auto] self@.contains_key(r) && self@[r] == b as int) by {
            if c0 is Some && b == c0->0 {
              let rid = choose |rid: u64| #![auto] self.at_level(rid, 0) &&
                self.deadlines@.contains_key(rid) && self.deadlines@[rid] == c0->0;
              assert(self@.contains_key(rid as nat));
            } else if c1 is Some && b == c1->0 {
              let rid = choose |rid: u64| #![auto] self.at_level(rid, 1) &&
                self.deadlines@.contains_key(rid) && self.deadlines@[rid] == c1->0;
              assert(self@.contains_key(rid as nat));
            } else if c2 is Some && b == c2->0 {
              let rid = choose |rid: u64| #![auto] self.at_level(rid, 2) &&
                self.deadlines@.contains_key(rid) && self.deadlines@[rid] == c2->0;
              assert(self@.contains_key(rid as nat));
            } else {
              assert(c3 is Some && b == c3->0);
              let rid = choose |rid: u64| #![auto] self.at_level(rid, 3) &&
                self.deadlines@.contains_key(rid) && self.deadlines@[rid] == c3->0;
              assert(self@.contains_key(rid as nat));
            }
          }
        },
        None => {
          assert forall |r: nat| !self@.contains_key(r) by {
            if self@.contains_key(r) {
              assert(best is Some);
            }
          };
          assert(self@ =~= Map::<nat, int>::empty());
        },
      }
    }
    best
  }

  // Exhaustive minimum over level 3 (which carries no delta band).
  fn scan_level3_min(&self) -> (result: Option<u64>)
    requires self.full_wf(),
    ensures
      result is Some ==> {
        &&& exists |rid: u64| #![auto] self.at_level(rid, 3) &&
              self.deadlines@.contains_key(rid) && self.deadlines@[rid] == result->0
        &&& forall |rid: u64| #![auto] self.at_level(rid, 3) &&
              self.deadlines@.contains_key(rid) ==> result->0 <= self.deadlines@[rid]
      },
      result is None ==> forall |rid: u64| #![auto] !self.at_level(rid, 3),
  {
    if self.level_counts[3] == 0 {
      proof {
        assert forall |rid: u64| #![auto] !self.at_level(rid, 3) by {
          if self.at_level(rid, 3) {
            let pos = self.positions@[rid];
            assert(self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
            Self::slots_len_sum_lower(self.levels@[3]@, pos.slot as int, WHEEL_SIZE as int);
          }
        };
      }
      return None;
    }
    let mut best: Option<u64> = None;
    let mut slot: usize = 0;
    while slot < WHEEL_SIZE
      invariant
        slot <= WHEEL_SIZE,
        self.full_wf(),
        best is Some ==> exists |rid: u64| #![auto] self.at_level(rid, 3) &&
          self.deadlines@.contains_key(rid) && self.deadlines@[rid] == best->0,
        forall |s: int, i: int| #![trigger self.levels@[3]@[s]@[i]]
          0 <= s < slot as int &&
          0 <= i < self.levels@[3]@[s]@.len() &&
          self.deadlines@.contains_key(self.levels@[3]@[s]@[i]) ==>
          best is Some && best->0 <= self.deadlines@[self.levels@[3]@[s]@[i]],
      decreases WHEEL_SIZE - slot,
    {
      let slot_vec = &self.levels[3][slot];
      let mut idx: usize = 0;
      while idx < slot_vec.len()
        invariant
          idx <= slot_vec@.len(),
          self.full_wf(),
          slot < WHEEL_SIZE,
          slot_vec@ == self.levels@[3]@[slot as int]@,
          best is Some ==> exists |rid: u64| #![auto] self.at_level(rid, 3) &&
            self.deadlines@.contains_key(rid) && self.deadlines@[rid] == best->0,
          forall |s: int, i: int| #![trigger self.levels@[3]@[s]@[i]]
            0 <= s < slot as int &&
            0 <= i < self.levels@[3]@[s]@.len() &&
            self.deadlines@.contains_key(self.levels@[3]@[s]@[i]) ==>
            best is Some && best->0 <= self.deadlines@[self.levels@[3]@[s]@[i]],
          forall |i: int| #![trigger self.levels@[3]@[slot as int]@[i]]
            0 <= i < idx as int &&
            self.deadlines@.contains_key(self.levels@[3]@[slot as int]@[i]) ==>
            best is Some && best->0 <= self.deadlines@[self.levels@[3]@[slot as int]@[i]],
        decreases slot_vec@.len() - idx,
      {
        let rid = slot_vec[idx];
        proof {
          assert(self.positions@.contains_key(rid));
          assert(!self.pending@.contains(rid));
          assert(self.positions@[rid].level as int == 3);
        }
        if let Some(d) = self.deadlines.get(&rid) {
          let d = *d;
          best = Self::merge_min(best, d);
          proof {
            if best->0 == d {
              assert(self.at_level(rid, 3) &&
                self.deadlines@.contains_key(rid) && self.deadlines@[rid] == best->0);
            }
          }
        }
        idx = idx + 1;
      }
      slot = slot + 1;
    }
    proof {
      match best {
        Some(b) => {
          assert forall |rid: u64| self.at_level(rid, 3) &&
            self.deadlines@.contains_key(rid) implies b <= self.deadlines@[rid]
          by {
            let pos = self.positions@[rid];
            assert(self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
          };
        },
        None => {
          assert forall |rid: u64| !self.at_level(rid, 3) by {
            if self.at_level(rid, 3) {
              assert(self.deadlines@.contains_key(rid));
              let pos = self.positions@[rid];
              assert(self.levels@[pos.level as int]@[pos.slot as int]@[pos.idx as int] == rid);
            }
          };
        },
      }
    }
    best
  }


  fn invalidate_min(&mut self, deadline: u64)
    ensures
      self.deadlines == old(self).deadlines,
      self.deadlines@ == old(self).deadlines@,
      self.level_counts == old(self).level_counts,
      self.levels == old(self).levels,
      self.elapsed == old(self).elapsed,
      self.pending@ == old(self).pending@,
      self.positions@ == old(self).positions@,
      self.cached_min.is_some() ==> self.cached_min == old(self).cached_min,
      self.cached_min.is_some() ==> deadline > self.cached_min.unwrap(),
  {
    if let Some(m) = self.cached_min {
      if deadline <= m {
        self.cached_min = None;
      }
    }
  }
}

}
