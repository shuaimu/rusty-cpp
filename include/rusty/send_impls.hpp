#pragma once

#include "send_trait.hpp"
#include <tuple>

// Send implementations for rusty types
// Mark which rusty types are thread-safe to send

namespace rusty {

// Forward declarations
template<typename T, typename A> class Box;
template<typename T> class Arc;
template<typename T> class Rc;  // NOT Send!
template<typename T, typename A> class VecLegacy;
template<typename T> class Option;
template<typename T, typename E> class Result;

// Note: Most rusty types (Box, Arc, Rc, Mutex, Cell, RefCell) are already
// handled in traits.hpp. This file provides additional specializations
// for container types.

// VecLegacy<T, A> is Send if T is Send
template<typename T, typename A>
struct is_send<VecLegacy<T, A>> : is_send<T> {};

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

} // namespace rusty
