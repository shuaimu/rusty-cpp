// string_port stub module — Phase B/C bridge.
//
// The full transpiled `string_port.cppm` (3606 LOC from
// library/alloc/src/string.rs) is blocked on cross-port deps
// (core::str Searcher/Pattern, alloc::borrow Cow/ToOwned,
// alloc::ascii::Char — none vendored). See docs/string_port/STATUS.md.
//
// Bridge re-exports the hand-written `rusty::String` from
// `rusty/string.hpp` under the `string_port` namespace.

module;

#include <rusty/string.hpp>

export module string_port;

namespace string_port {

export using String = ::rusty::String;

} // namespace string_port
