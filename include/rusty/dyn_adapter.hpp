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
//
// The adapters themselves derive from `I` and so INHERIT the alias; without
// the `is_final` guard `Box<TrAdapter<X>>::new_(v)` would take the adapter
// for an interface and box it in `TrAdapter<TrAdapter<...>>` without end
// (§3.2.4 generic forwarders are `final`; an interface — abstract, with a
// protected constructor — never is).
template<typename I, typename U, typename = void>
struct dyn_adapter_for {};

template<typename I, typename U>
struct dyn_adapter_for<I, U,
    std::enable_if_t<!std::is_final_v<I>,
                     std::void_t<typename I::template rusty_dyn_adapter<U>>>> {
    using type = typename I::template rusty_dyn_adapter<U>;
};

} // namespace detail

// `&mut dyn Trait` in ARGUMENT position over a tier-2 value (book §3.2.10):
// the forwarder `TraitAdapterRefMut<U>(x)` is a prvalue, and a non-const
// `Trait&` parameter cannot bind one. This lends it an lvalue; the temporary
// lives to the end of the full-expression, which covers the call. A `let`
// binding takes a named forwarder local instead (its lifetime must outlive
// the statement).
// (`static_cast`: C++23 P2266 treats a returned rvalue-reference parameter
// as an xvalue, which an lvalue reference return type cannot bind.)
template<typename T>
constexpr T& dyn_lvalue(T&& t) noexcept { return static_cast<T&>(t); }

} // namespace rusty

#endif // RUSTY_DYN_ADAPTER_HPP
