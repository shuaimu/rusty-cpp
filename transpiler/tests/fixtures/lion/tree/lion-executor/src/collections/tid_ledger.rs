use crate::types::TaskId;
use lion_executor_spec::types::TID;
use vstd::prelude::*;

verus! {

proof fn lemma_bit_set_self(w: u64, b: u64)
  requires b < 64,
  ensures ((w | (1u64 << b)) >> b) & 1u64 == 1u64,
{
  assert(((w | (1u64 << b)) >> b) & 1u64 == 1u64) by (bit_vector)
    requires b < 64;
}

proof fn lemma_bit_set_other(w: u64, b: u64, b2: u64)
  requires b < 64, b2 < 64, b != b2,
  ensures ((w | (1u64 << b)) >> b2) & 1u64 == (w >> b2) & 1u64,
{
  assert(((w | (1u64 << b)) >> b2) & 1u64 == (w >> b2) & 1u64) by (bit_vector)
    requires b < 64 && b2 < 64 && b != b2;
}

proof fn lemma_zero_bit(b: u64)
  requires b < 64,
  ensures (0u64 >> b) & 1u64 == 0u64,
{
  assert((0u64 >> b) & 1u64 == 0u64) by (bit_vector)
    requires b < 64;
}

// Bit-packed monotone TID set (1 bit per TID, dense from 0). TIDs are
// allocated by a monotone counter, so the words vector grows with the max
// TID ever marked and is never shrunk. Unlike the task slab and the TLS
// notified set (which hold live tasks only), this is by design a HISTORY set —
// "was ever injected" is exactly what the TID-freshness filter checks — so it
// is the one id-proportional structure left, at 1 bit per task ever spawned
// (125 KB per million tasks).
pub struct TidLedger {
  pub words: Vec<u64>,
}

impl TidLedger {
  pub open spec fn spec_has(&self, tid: TID) -> bool {
    &&& tid / 64 < self.words@.len()
    &&& (self.words@[(tid / 64) as int] >> ((tid % 64) as u64)) & 1u64 == 1u64
  }

  pub fn new() -> (result: Self)
    ensures forall |t: TID| !result.spec_has(t),
  {
    TidLedger { words: Vec::new() }
  }

  pub fn contains(&self, tid: TaskId) -> (result: bool)
    ensures result == self.spec_has(tid@),
  {
    let w64 = tid.0 / 64;
    proof {
      assert(tid@ / 64 == w64 as nat);
      assert(tid@ % 64 == (tid.0 % 64) as nat);
      assert(tid.0 % 64 < 64);
    }
    if w64 >= self.words.len() as u64 {
      return false;
    }
    let w = w64 as usize;
    let bit = (self.words[w] >> (tid.0 % 64)) & 1;
    bit == 1
  }

  pub fn mark(&mut self, tid: TaskId)
    ensures
      forall |t: TID| self.spec_has(t) <==> (old(self).spec_has(t) || t == tid@),
  {
    let w64 = tid.0 / 64;
    let b = tid.0 % 64;
    let ghost old_words = self.words@;
    proof {
      assert(tid@ / 64 == w64 as nat);
      assert(tid@ % 64 == (tid.0 % 64) as nat);
      assert(b < 64);
    }

    while (self.words.len() as u64) <= w64
      invariant
        self.words@.len() >= old_words.len(),
        forall |j: int| 0 <= j < old_words.len() ==> self.words@[j] == old_words[j],
        forall |j: int| old_words.len() <= j < self.words@.len() ==> self.words@[j] == 0u64,
      decreases w64 + 1 - self.words@.len(),
    {
      self.words.push(0);
    }

    let w = w64 as usize;
    proof {
      assert(self.words@.len() > w64);
      assert(w as int == w64 as int);
    }

    let ghost pre_set = self.words@;
    let word = self.words[w];
    let updated = word | (1u64 << b);
    self.words.set(w, updated);

    proof {
      assert(self.words@ =~= pre_set.update(w as int, updated));
      lemma_bit_set_self(word, b);
      assert forall |t: TID| self.spec_has(t) <==> (old(self).spec_has(t) || t == tid@) by {
        let tw = (t / 64) as int;
        let tb = (t % 64) as u64;
        assert(t % 64 < 64);
        if t == tid@ {
          assert(tw == w as int);
          assert(tb == b);
          assert(self.spec_has(t));
        } else if tw == w as int {
          assert(tb != b) by {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(t as int, 64);
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(tid@ as int, 64);
          }
          lemma_bit_set_other(word, b, tb);
          if t / 64 < old_words.len() {
            assert(pre_set[tw] == old_words[tw]);
          } else {
            assert(pre_set[tw] == 0u64);
            lemma_zero_bit(tb);
          }
        } else {
          if tw < pre_set.len() {
            assert(self.words@[tw] == pre_set[tw]);
            if t / 64 < old_words.len() {
            } else {
              assert(pre_set[tw] == 0u64);
              lemma_zero_bit(tb);
            }
          }
        }
      }
    }
  }
}

}
