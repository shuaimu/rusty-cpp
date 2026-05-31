// vec_deque_port stub module — Phase B/C bridge.
//
// The full transpiled `vec_deque_port.cppm` (5527 LOC, 10 .cppm files)
// is blocked on same shape as rc_port (single- vs two-arg VecDeque,
// std::Allocator vs rusty::alloc::Allocator mis-emit, cross-port
// imports). See docs/vec_deque_port/STATUS.md.
//
// Bridge re-exports hand-written `rusty::VecDeque<T>` under the
// `vec_deque_port` namespace.

module;

#include <rusty/vecdeque.hpp>
#include <rusty/alloc.hpp>

export module vec_deque_port;

namespace vec_deque_port {

export template<typename T, typename A = ::rusty::alloc::Global>
using VecDeque = ::rusty::VecDeque<T>;

} // namespace vec_deque_port
