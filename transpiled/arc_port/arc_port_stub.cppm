// arc_port stub module — Phase B/C bridge.
//
// The full transpiled `arc_port.cppm` (4936 LOC from
// library/alloc/src/sync.rs) has the same shape of transpiler-side
// blockers as rc_port (multi-template-arg Arc<T,A>, ::cast free-fn,
// Cluster A 'auto' regressions) plus atomics-specific issues
// (rusty::atomic ordering helpers, compare_exchange_weak overload
// mismatch). See docs/arc_port/STATUS.md.
//
// Bridge re-exports the hand-written `rusty::Arc<T>` from
// `rusty/arc.hpp` under the `arc_port` namespace, mapping the
// two-arg `Arc<T,A>` to the single-arg hand version (A ignored).

module;

#include <rusty/arc.hpp>
#include <rusty/alloc.hpp>

export module arc_port;

namespace arc_port {

export template<typename T, typename A = ::rusty::alloc::Global>
using Arc = ::rusty::Arc<T>;

export template<typename T, typename A = ::rusty::alloc::Global>
using Weak = ::rusty::sync::Weak<T>;

} // namespace arc_port
