#![cfg_attr(verus_keep_ghost, verus::trusted)]

use std::collections::VecDeque as StdVecDeque;
use vstd::prelude::*;
use vstd::seq::Seq;

verus! {

#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct VecDeque<T> {
  inner: StdVecDeque<T>,
}

impl<T: View> View for VecDeque<T> {
  type V = Seq<T::V>;

  #[verifier::external_body]
  spec fn view(&self) -> Seq<T::V> {
  unimplemented!()
  }
}

impl<T: View> VecDeque<T> {
  #[verifier::external_body]
  pub exec fn new() -> (result: Self)
    ensures result@ =~= Seq::<T::V>::empty(),
  {
  VecDeque { inner: StdVecDeque::new() }
  }

  #[verifier::external_body]
  pub exec fn push_back(&mut self, value: T)
    ensures self@ =~= old(self)@.push(value@),
  {
  self.inner.push_back(value)
  }

  #[verifier::external_body]
  pub exec fn push_front(&mut self, value: T)
  {
  self.inner.push_front(value)
  }

  #[verifier::external_body]
  pub exec fn pop_front(&mut self) -> (result: Option<T>)
    ensures
      old(self)@.len() > 0 ==> (
        result.is_some() &&
        result.unwrap()@ == old(self)@[0] &&
        self@ =~= old(self)@.subrange(1, old(self)@.len() as int)
      ),
      old(self)@.len() == 0 ==> (
        result.is_none() &&
        self@ =~= old(self)@
      ),
  {
  self.inner.pop_front()
  }

  #[verifier::external_body]
  pub exec fn pop_back(&mut self) -> (result: Option<T>)
  {
  self.inner.pop_back()
  }

  #[verifier::external_body]
  pub exec fn is_empty(&self) -> (result: bool)
    ensures result == (self@.len() == 0),
  {
  self.inner.is_empty()
  }

  #[verifier::external_body]
  pub exec fn len(&self) -> (result: usize)
    ensures result as int == self@.len(),
  {
  self.inner.len()
  }

  #[verifier::external_body]
  pub exec fn clear(&mut self)
    ensures self@ =~= Seq::<T::V>::empty(),
  {
  self.inner.clear()
  }
}

impl<T: View> Default for VecDeque<T> {
  #[verifier::external_body]
  fn default() -> (result: Self)
    ensures result@ =~= Seq::<T::V>::empty(),
  {
  VecDeque { inner: StdVecDeque::new() }
  }
}

impl<T> From<VecDeque<T>> for Vec<T> {
  #[verifier::external_body]
  fn from(deque: VecDeque<T>) -> (result: Self)
  {
  deque.inner.into()
  }
}

}
