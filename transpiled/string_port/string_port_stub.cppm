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

namespace rusty::port::string {

export using String = ::rusty::String;

} // namespace rusty::port::string

// User-facing alias: `rusty::string::*` mirrors Rust's `std::string::*`.
// End users don't observe the underlying `rusty::port::*` scaffolding.
export namespace rusty::string {
    using String = ::rusty::port::string::String;
}
