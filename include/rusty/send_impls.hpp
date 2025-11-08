#pragma once

#include "send_trait.hpp"

// Send implementations for rusty types
// Mark which rusty types are thread-safe to send

namespace rusty {

// Forward declarations
template<typename T> class Box;
template<typename T> class Arc;
template<typename T> class Rc;  // NOT Send!
template<typename T> class Vec;
template<typename T> class Option;
template<typename T, typename E> class Result;

// Box<T> is Send if T is Send
template<typename T>
struct is_explicitly_send<Box<T>> : is_send<T> {};

// Arc<T> is Send if T is Send (uses atomic reference counting)
template<typename T>
struct is_explicitly_send<Arc<T>> : is_send<T> {};

// Rc<T> is NEVER Send (non-atomic reference counting)
// No specialization needed - defaults to false

// Vec<T> is Send if T is Send
template<typename T>
struct is_explicitly_send<Vec<T>> : is_send<T> {};

// Option<T> is Send if T is Send
template<typename T>
struct is_explicitly_send<Option<T>> : is_send<T> {};

// Result<T, E> is Send if both T and E are Send
template<typename T, typename E>
struct is_explicitly_send<Result<T, E>> : std::bool_constant<
    is_send<T>::value && is_send<E>::value
> {};

} // namespace rusty
