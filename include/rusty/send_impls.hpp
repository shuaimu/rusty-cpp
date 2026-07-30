#pragma once

#include "send_trait.hpp"
#include <array>
#include <optional>
#include <tuple>
#include <utility>
#include <variant>

// Send implementations for rusty types
// Mark which rusty types are thread-safe to send

namespace rusty {

// Forward declarations
template<typename T, typename A> class Box;
template<typename T> class Arc;
// `rusty::Rc<T, A>` lives in module `rc_port`'s purview; cannot fwd-
// declare it across the GMF / named-module boundary (C++20). The
// `is_send<Rc<...>>` specialization is part of rc_port.
// VecLegacy retired — the rusty::Vec is_send specialization for the
// transpiled vec_port::Vec lives in vec_port itself (or its consumer
// module). No header-mode forward decl needed.
template<typename T> class Option;
template<typename T, typename E> class Result;

// Note: Most rusty types (Box, Arc, Rc, Mutex, Cell, RefCell) are already
// handled in traits.hpp. This file provides additional specializations
// for container types.

// is_send specialization for the retired VecLegacy class is gone with the
// class. rusty::Vec is now an alias of ::Vec<T,A> from vec_port.vec; if
// channels need is_send<rusty::Vec<T,A>>, declare it in a module unit
// that imports vec_port.vec rather than here.

// Option<T> is Send if T is Send
template<typename T>
struct is_send<Option<T>> : is_send<T> {};

// Result<T, E> is Send if both T and E are Send
template<typename T, typename E>
struct is_send<Result<T, E>> : std::bool_constant<
    is_send<T>::value && is_send<E>::value
> {};

// std::tuple<Ts...> is Send if all tuple elements are Send.
template<typename... Ts>
struct is_send<std::tuple<Ts...>> : std::bool_constant<(is_send<Ts>::value && ...)> {};

// The rest of the structural composites, on the same rule Rust applies
// to a tuple, an array and an Option: the composite is Send/Sync when
// every part is. `std::variant` is the load-bearing one — a transpiled
// data enum lowers to a variant of one struct per variant, so the
// enum's Send follows from its variants'.
template<typename... Ts>
struct is_send<std::variant<Ts...>> : std::bool_constant<(is_send<Ts>::value && ...)> {};

template<typename A, typename B>
struct is_send<std::pair<A, B>>
    : std::bool_constant<is_send<A>::value && is_send<B>::value> {};

template<typename T, std::size_t N>
struct is_send<std::array<T, N>> : is_send<T> {};

template<typename T>
struct is_send<std::optional<T>> : is_send<T> {};

// The Sync halves. Sync had no composite rules at all, which left an
// `Arc<EnumType>` un-shareable however its variants were marked.
template<typename... Ts>
struct is_sync<std::variant<Ts...>> : std::bool_constant<(is_sync<Ts>::value && ...)> {};

template<typename... Ts>
struct is_sync<std::tuple<Ts...>> : std::bool_constant<(is_sync<Ts>::value && ...)> {};

template<typename A, typename B>
struct is_sync<std::pair<A, B>>
    : std::bool_constant<is_sync<A>::value && is_sync<B>::value> {};

template<typename T, std::size_t N>
struct is_sync<std::array<T, N>> : is_sync<T> {};

template<typename T>
struct is_sync<std::optional<T>> : is_sync<T> {};

template<typename T>
struct is_sync<Option<T>> : is_sync<T> {};

} // namespace rusty
