#ifndef RUSTY_DYN_ADAPTER_HPP
#define RUSTY_DYN_ADAPTER_HPP

#include <type_traits>

// A transpiled trait interface class `I` (the C++ lowering of `dyn Trait`)
// names its generic owning adapter `I::rusty_dyn_adapter<U>`: a class deriving
// from `I` that owns a `U` and forwards every pure virtual to `U`'s member of
// the same name. rusty::Box and rusty::Arc use it for the unsize coercion
// `Box<U>` -> `Box<dyn Trait>` when `U` implements a trait declared by another
// crate, whose own transpilation could not see `U` and so wrote no
// `<Trait>Adapter<U>` specialization for it.

namespace rusty {
namespace detail {

// SFINAE-friendly: no `type` when `I` is not such an interface.
template<typename I, typename U, typename = void>
struct dyn_adapter_for {};

template<typename I, typename U>
struct dyn_adapter_for<I, U, std::void_t<typename I::template rusty_dyn_adapter<U>>> {
    using type = typename I::template rusty_dyn_adapter<U>;
};

} // namespace detail
} // namespace rusty

#endif // RUSTY_DYN_ADAPTER_HPP
