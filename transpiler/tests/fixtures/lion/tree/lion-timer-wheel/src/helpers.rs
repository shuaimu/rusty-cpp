use vstd::prelude::*;

verus! {

#[verifier::external_body]
pub fn wrapping_sub_u64(a: u64, b: u64) -> (result: u64)
  ensures a >= b ==> result == a - b,
{
  a.wrapping_sub(b)
}

}
