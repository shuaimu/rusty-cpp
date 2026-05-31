// linked_list_port stub module — Phase B/C bridge.
//
// The full transpiled `linked_list_port.cppm` (2255 LOC from
// library/alloc/src/collections/linked_list.rs) is blocked on the
// transpiler-side Cluster A regression (13 'auto' template-arg sites).
// See docs/linked_list_port/STATUS.md.
//
// No hand-written `rusty::LinkedList<T>` exists in the library yet,
// so this bridge re-exports `std::list<T>` under a thin
// `linked_list_port::LinkedList` alias.

module;

#include <list>
#include <rusty/alloc.hpp>

export module linked_list_port;

namespace linked_list_port {

// `std::list<T>` provides the doubly-linked list surface; mapping to
// `LinkedList<T, A>` ignores the allocator parameter.
export template<typename T, typename A = ::rusty::alloc::Global>
using LinkedList = std::list<T>;

} // namespace linked_list_port
