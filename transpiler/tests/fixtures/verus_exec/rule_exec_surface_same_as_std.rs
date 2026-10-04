// @rule exec-surface
// @compile no: rusty::String has no is_ascii(); std's str::is_ascii is missing from the C++ runtime for plain Rust too, so this is a std-surface gap, not a Verus one.
// @expect-lowered s.is_ascii()
// @expect-cpp return s.is_ascii();
// T3: StringExecFnsIsAscii::is_ascii has the same behaviour as std's
// str::is_ascii, so the call is left exactly as written.
use vstd::prelude::*;

verus! {

pub fn ascii(s: &String) -> bool {
    s.is_ascii()
}

} // verus!
