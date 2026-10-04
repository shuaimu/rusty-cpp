#![feature(prelude_import)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod helpers {
    use vstd::prelude::*;
    pub fn wrapping_sub_u64(a: u64, b: u64) -> u64 {
        a.wrapping_sub(b)
    }
}
pub mod vec_map {
    use std::collections::HashMap;
    use vstd::prelude::*;
    const _: () = ();
    pub struct VecMap<V: View> {
        pub inner: HashMap<u64, V>,
    }
    impl<V: View> VecMap<V> {}
    impl<V: View> View for VecMap<V> {
        type V = Map<u64, V>;
    }
    impl<V: View + Copy> VecMap<V> {
        pub fn new() -> Self {
            VecMap { inner: HashMap::new() }
        }
        #[inline(always)]
        pub fn insert(&mut self, key: u64, value: V) {
            let _ = self.inner.insert(key, value);
        }
        #[inline(always)]
        pub fn get(&self, key: &u64) -> Option<&V> {
            self.inner.get(key)
        }
        #[inline(always)]
        pub fn remove(&mut self, key: &u64) -> Option<V> {
            self.inner.remove(key)
        }
        #[inline(always)]
        pub fn contains_key(&self, key: &u64) -> bool {
            self.inner.contains_key(key)
        }
        #[inline]
        pub fn is_empty(&self) -> bool {
            let r = self.inner.is_empty();
            {}
            r
        }
    }
}
pub mod wheel {
    use vstd::prelude::*;
    use crate::helpers::*;
    use crate::vec_map::VecMap;
    const _: () = ();
    pub const WHEEL_BITS: u32 = 8;
    pub const WHEEL_SIZE: usize = 256;
    pub const NUM_LEVELS: usize = 4;
    pub struct WheelPos {
        pub level: u8,
        pub slot: u8,
        pub idx: u64,
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for WheelPos {}
    #[automatically_derived]
    impl ::core::clone::Clone for WheelPos {
        #[inline]
        fn clone(&self) -> WheelPos {
            let _: ::core::clone::AssertParamIsClone<u8>;
            let _: ::core::clone::AssertParamIsClone<u64>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for WheelPos {}
    impl View for WheelPos {
        type V = WheelPos;
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
    }
    impl TimerWheel {
        pub fn new() -> Self {
            let mut levels: Vec<Vec<Vec<u64>>> = Vec::with_capacity(NUM_LEVELS);
            let mut li: usize = 0;
            while li < NUM_LEVELS {
                let mut level: Vec<Vec<u64>> = Vec::with_capacity(WHEEL_SIZE);
                let mut si: usize = 0;
                while si < WHEEL_SIZE {
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
            {}
            {}
            result
        }
        pub fn insert(&mut self, rid: u64, deadline: u64) {
            if let Some(old_deadline) = self.deadlines.get(&rid) {
                self.invalidate_min(*old_deadline);
            }
            self.wheel_remove_inner(rid);
            {}
            self.deadlines.insert(rid, deadline);
            let (level, slot) = Self::level_slot(deadline, self.elapsed);
            let idx = self.levels[level][slot].len() as u64;
            {}
            self.slot_push(level, slot, rid);
            self.positions
                .insert(
                    rid,
                    WheelPos {
                        level: level as u8,
                        slot: slot as u8,
                        idx,
                    },
                );
            {}
            match self.cached_min {
                Some(m) if deadline < m => self.cached_min = Some(deadline),
                _ => {}
            }
            {}
        }
        pub fn remove(&mut self, rid: u64) {
            if self.deadlines.contains_key(&rid) {
                let deadline = *self.deadlines.get(&rid).unwrap();
                self.invalidate_min(deadline);
                self.wheel_remove_inner(rid);
                {}
                self.deadlines.remove(&rid);
            }
            {}
        }
        pub fn try_pop_expired(&mut self, now: u64) -> Option<u64> {
            while self.pending.len() > 0 {
                let rid = self.pending.pop().unwrap();
                {}
                if self.deadlines.contains_key(&rid) {
                    let deadline = *self.deadlines.get(&rid).unwrap();
                    if deadline <= now {
                        self.invalidate_min(deadline);
                        self.deadlines.remove(&rid);
                        self.positions.remove(&rid);
                        {}
                        return Some(rid);
                    }
                }
                {}
            }
            {}
            if self.elapsed < now {
                self.advance_to(now);
            } else {
                {}
            }
            while self.pending.len() > 0 {
                let rid = self.pending.pop().unwrap();
                {}
                if self.deadlines.contains_key(&rid) {
                    let deadline = *self.deadlines.get(&rid).unwrap();
                    if deadline <= now {
                        self.invalidate_min(deadline);
                        self.deadlines.remove(&rid);
                        self.positions.remove(&rid);
                        {}
                        return Some(rid);
                    }
                }
                {}
            }
            {}
            None
        }
        pub fn next_deadline(&self) -> Option<u64> {
            if self.is_empty() {
                return None;
            }
            if let Some(d) = self.cached_min {
                {}
                return Some(d);
            }
            self.scan_wheel_min()
        }
        pub fn is_empty(&self) -> bool {
            let r = self.deadlines.is_empty();
            {}
            r
        }
        #[inline]
        pub fn get_deadline(&self, rid: u64) -> Option<u64> {
            self.deadlines.get(&rid).copied()
        }
        fn level_slot(deadline: u64, elapsed: u64) -> (usize, usize) {
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
            {}
            (level, slot_masked as usize)
        }
        fn advance_to(&mut self, now: u64) {
            let elapsed = self.elapsed;
            {}
            let mut done = false;
            let mut level: usize = 0;
            while level < NUM_LEVELS && !done {
                let shift: u32 = (level as u32) * WHEEL_BITS;
                {}
                let old_pos: u64 = elapsed >> shift;
                let new_pos: u64 = now >> shift;
                if old_pos == new_pos {
                    done = true;
                }
                if !done {
                    {}
                    let diff: u64 = new_pos - old_pos;
                    let slots_to_drain: usize = if diff < WHEEL_SIZE as u64 {
                        diff as usize
                    } else {
                        WHEEL_SIZE
                    };
                    let old_pos_masked: u64 = old_pos & (WHEEL_SIZE as u64 - 1);
                    {}
                    let old_slot: usize = old_pos_masked as usize;
                    {}
                    let mut i: usize = 1;
                    while i <= slots_to_drain {
                        let slot: usize = (old_slot + i) % WHEEL_SIZE;
                        {};
                        let slot_vec: Vec<u64> = self.slot_drain(level, slot);
                        {}
                        let mut j: usize = 0;
                        while j < slot_vec.len() {
                            let rid = slot_vec[j];
                            if self.deadlines.contains_key(&rid) {
                                let deadline = *self.deadlines.get(&rid).unwrap();
                                if deadline <= now {
                                    self.positions.remove(&rid);
                                    {}
                                    self.pending.push(rid);
                                    {}
                                } else {
                                    self.wheel_insert_inner(rid, deadline, now);
                                    {}
                                }
                            }
                            j = j + 1;
                        }
                        {}
                        i = i + 1;
                    }
                    {}
                    level = level + 1;
                }
            }
            let level_shift_val: u32 = if level < NUM_LEVELS {
                (level as u32) * WHEEL_BITS
            } else {
                0
            };
            {}
            self.elapsed = now;
            {}
        }
        #[inline]
        fn slot_push(&mut self, level: usize, slot: usize, rid: u64) {
            self.levels[level][slot].push(rid);
            self.level_counts[level] += 1;
        }
        #[inline]
        fn slot_swap_remove(&mut self, level: usize, slot: usize, idx: usize) -> u64 {
            self.level_counts[level] -= 1;
            self.levels[level][slot].swap_remove(idx)
        }
        #[inline]
        fn slot_pop(&mut self, level: usize, slot: usize) {
            self.level_counts[level] -= 1;
            self.levels[level][slot].pop();
        }
        #[inline]
        fn slot_drain(&mut self, level: usize, slot: usize) -> Vec<u64> {
            self.level_counts[level] -= self.levels[level][slot].len() as u64;
            std::mem::take(&mut self.levels[level][slot])
        }
        fn wheel_remove_inner(&mut self, rid: u64) {
            if let Some(pos) = self.positions.remove(&rid) {
                let level = pos.level as usize;
                let slot = pos.slot as usize;
                let idx = pos.idx as usize;
                {}
                if level < NUM_LEVELS && slot < WHEEL_SIZE {
                    {}
                    let slot_len = self.levels[level][slot].len();
                    if slot_len > 0 {
                        let last = slot_len - 1;
                        if idx <= last {
                            if idx < last {
                                let swapped_rid = self.levels[level][slot][last];
                                self.slot_swap_remove(level, slot, idx);
                                {}
                                self.positions
                                    .insert(
                                        swapped_rid,
                                        WheelPos {
                                            level: pos.level,
                                            slot: pos.slot,
                                            idx: pos.idx,
                                        },
                                    );
                            } else {
                                {}
                                self.slot_pop(level, slot);
                                {}
                            }
                        }
                    }
                    {}
                }
            }
            {}
        }
        fn wheel_insert_inner(
            &mut self,
            rid: u64,
            deadline: u64,
            elapsed_for_slot: u64,
        ) {
            let (level, slot) = Self::level_slot(deadline, elapsed_for_slot);
            let idx = self.levels[level][slot].len() as u64;
            self.slot_push(level, slot, rid);
            self.positions
                .insert(
                    rid,
                    WheelPos {
                        level: level as u8,
                        slot: slot as u8,
                        idx,
                    },
                );
            {}
        }
        fn scan_level_min(&self, level: usize, bound: Option<u64>) -> Option<u64> {
            let shift: u32 = (level as u32) * WHEEL_BITS;
            let e = self.elapsed;
            let cur: u64 = (e >> shift) & 255;
            if self.level_counts[level] == 0 {
                {}
                return None;
            }
            {}
            let mut o: u64 = 1;
            while o <= 256 {
                let slot: usize = (((cur + o) % 256) as usize);
                {}
                let slot_vec = &self.levels[level][slot];
                if slot_vec.len() > 0 {
                    if let Some(b) = bound {
                        {}
                        if (b >> shift) < (e >> shift) + o {
                            {}
                            return None;
                        }
                    }
                    if level == 0 {
                        let rid0 = slot_vec[0];
                        {}
                        if let Some(d0) = self.deadlines.get(&rid0) {
                            let d0 = *d0;
                            {}
                            return Some(d0);
                        }
                        {}
                    }
                    let mut best: u64 = 0;
                    let mut have: bool = false;
                    let mut idx: usize = 0;
                    while idx < slot_vec.len() {
                        let rid = slot_vec[idx];
                        {}
                        if let Some(d) = self.deadlines.get(&rid) {
                            let d = *d;
                            if !have || d < best {
                                best = d;
                                have = true;
                                {}
                            }
                        }
                        idx = idx + 1;
                    }
                    if have {
                        {}
                        return Some(best);
                    }
                    {}
                }
                {}
                o = o + 1;
            }
            {}
            None
        }
        fn merge_min(best: Option<u64>, d: u64) -> Option<u64> {
            match best {
                None => Some(d),
                Some(b) => if d < b { Some(d) } else { Some(b) }
            }
        }
        fn scan_wheel_min(&self) -> Option<u64> {
            let mut pbest: Option<u64> = None;
            let mut p: usize = 0;
            while p < self.pending.len() {
                let rid = self.pending[p];
                if let Some(d) = self.deadlines.get(&rid) {
                    let d = *d;
                    {}
                    pbest = Self::merge_min(pbest, d);
                }
                p = p + 1;
            }
            if let Some(pb) = pbest {
                {}
                return Some(pb);
            }
            let c0 = self.scan_level_min(0, None);
            let c1 = self.scan_level_min(1, c0);
            let c2 = self.scan_level_min(2, c0);
            let c3 = self.scan_level3_min();
            let mut best: Option<u64> = None;
            if let Some(d) = c0 {
                best = Self::merge_min(best, d);
            }
            if let Some(d) = c1 {
                best = Self::merge_min(best, d);
            }
            if let Some(d) = c2 {
                best = Self::merge_min(best, d);
            }
            if let Some(d) = c3 {
                best = Self::merge_min(best, d);
            }
            {}
            {}
            best
        }
        fn scan_level3_min(&self) -> Option<u64> {
            if self.level_counts[3] == 0 {
                {}
                return None;
            }
            let mut best: Option<u64> = None;
            let mut slot: usize = 0;
            while slot < WHEEL_SIZE {
                let slot_vec = &self.levels[3][slot];
                let mut idx: usize = 0;
                while idx < slot_vec.len() {
                    let rid = slot_vec[idx];
                    {}
                    if let Some(d) = self.deadlines.get(&rid) {
                        let d = *d;
                        best = Self::merge_min(best, d);
                        {}
                    }
                    idx = idx + 1;
                }
                slot = slot + 1;
            }
            {}
            best
        }
        fn invalidate_min(&mut self, deadline: u64) {
            if let Some(m) = self.cached_min {
                if deadline <= m {
                    self.cached_min = None;
                }
            }
        }
    }
}
pub use wheel::TimerWheel;
