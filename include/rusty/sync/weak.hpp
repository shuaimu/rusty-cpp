#ifndef RUSTY_SYNC_WEAK_HPP
#define RUSTY_SYNC_WEAK_HPP

// rusty::sync::Weak — alias to the transpiled rustc Arc/Weak pair.
//
// The hand-written `rusty::sync::Weak<T>` class (paired with the
// hand-written `rusty::Arc<T>` in `arc.hpp`) was retired. The
// canonical Weak type is now the transpiled
// `rusty::port::sync::Weak<T, A>` from `transpiled/arc_port/`.
//
// API change (no compatibility shim — `we don't worry about
// compatibility yet`):
//   - The free function `rusty::downgrade(arc)` / `rusty::sync::downgrade(arc)`
//     is GONE. Use the Rust idiom: `Arc<T>::downgrade(arc)`.
//   - `rusty::sync::Weak<T>` is now a template alias to
//     `::rusty::port::sync::Weak<T, ::rusty::alloc::Global>`.
//
// Implementation note: importing `arc_port` here would pull in the
// full `<future>`-touching module graph and trigger libstdc++14 ADL
// issues in threading tests. Tests/consumers that need the Weak type
// should `import arc_port;` themselves.

#include "../alloc.hpp"

namespace rusty::port::sync {
template<typename T, typename A>
    requires (rusty::alloc::Allocator<A>)
struct Weak;  // forward decl; full type lives in module arc_port
} // namespace rusty::port::sync

namespace rusty::sync {
template<typename T, typename A = ::rusty::alloc::Global>
using Weak = ::rusty::port::sync::Weak<T, A>;
} // namespace rusty::sync

#endif // RUSTY_SYNC_WEAK_HPP
