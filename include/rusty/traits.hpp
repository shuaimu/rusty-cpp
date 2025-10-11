#pragma once

#include <type_traits>
#include <memory>

namespace rusty {

// Forward declarations for rusty types
template<typename T> class Arc;
template<typename T> class Rc;
template<typename T> class Box;
template<typename T> class Cell;
template<typename T> class RefCell;
template<typename T> class Mutex;
template<typename T> class Atomic;

// Forward declare is_sync for circular dependency with is_send
template<typename T, typename = void>
struct is_sync;

// ============================================================================
// Send Trait - Can transfer ownership across thread boundaries
// ============================================================================

// Default: types are NOT Send
template<typename T, typename = void>
struct is_send : std::false_type {};

// Primitives are Send
template<typename T>
struct is_send<T, std::enable_if_t<std::is_arithmetic_v<T>>> : std::true_type {};

// RUST RULE 1: const T& is Send if T is Sync
// This mimics Rust's: &T is Send if T is Sync
template<typename T>
struct is_send<const T&> : is_sync<T> {};

// RUST RULE 2: T& (mutable ref) is Send if T is Send
// This mimics Rust's: &mut T is Send if T is Send
template<typename T>
struct is_send<T&> : is_send<T> {};

// Rvalue references follow same rule as mutable references
template<typename T>
struct is_send<T&&> : is_send<T> {};

// Arc<T> is Send if T is Send + Sync
template<typename T>
struct is_send<Arc<T>> : std::bool_constant<
    is_send<T>::value && is_sync<T>::value
> {};

// Rc<T> is NEVER Send (non-atomic refcount)
template<typename T>
struct is_send<Rc<T>> : std::false_type {};

// Box<T> is Send if T is Send
template<typename T>
struct is_send<Box<T>> : is_send<T> {};

// std::unique_ptr<T> is Send if T is Send
template<typename T>
struct is_send<std::unique_ptr<T>> : is_send<T> {};

// Mutex<T> is Send if T is Send
template<typename T>
struct is_send<Mutex<T>> : is_send<T> {};

// Cell<T> is Send if T is Send (but not Sync)
template<typename T>
struct is_send<Cell<T>> : is_send<T> {};

// RefCell<T> is Send if T is Send (but not Sync)
template<typename T>
struct is_send<RefCell<T>> : is_send<T> {};

// Raw pointers are not Send by default (unsafe)
template<typename T>
struct is_send<T*> : std::false_type {};

template<typename T>
struct is_send<const T*> : std::false_type {};

// ============================================================================
// Sync Trait - Can safely share &T across threads
// ============================================================================

// Default: types are NOT Sync
template<typename T, typename>
struct is_sync : std::false_type {};

// Primitives are Sync
template<typename T>
struct is_sync<T, std::enable_if_t<std::is_arithmetic_v<T>>> : std::true_type {};

// RUST RULE 3: const T& is Sync if T is Sync
// This mimics Rust's: &T is Sync if T is Sync
template<typename T>
struct is_sync<const T&> : is_sync<T> {};

// RUST RULE 4: T& (mutable ref) is NEVER Sync
// This mimics Rust's: &mut T is never Sync
template<typename T>
struct is_sync<T&> : std::false_type {};

// Rvalue references are never Sync
template<typename T>
struct is_sync<T&&> : std::false_type {};

// Arc<T> is Sync if T is Send + Sync
template<typename T>
struct is_sync<Arc<T>> : std::bool_constant<
    is_send<T>::value && is_sync<T>::value
> {};

// Rc<T> is NEVER Sync
template<typename T>
struct is_sync<Rc<T>> : std::false_type {};

// Box<T> is Sync if T is Sync
template<typename T>
struct is_sync<Box<T>> : is_sync<T> {};

// std::unique_ptr<T> is Sync if T is Sync
template<typename T>
struct is_sync<std::unique_ptr<T>> : is_sync<T> {};

// Mutex<T> is Sync if T is Send (allows &Mutex<T> to be shared)
template<typename T>
struct is_sync<Mutex<T>> : is_send<T> {};

// Cell<T> is NEVER Sync (unsynchronized interior mutability)
template<typename T>
struct is_sync<Cell<T>> : std::false_type {};

// RefCell<T> is NEVER Sync (unsynchronized interior mutability)
template<typename T>
struct is_sync<RefCell<T>> : std::false_type {};

// Atomic<T> is Sync
template<typename T>
struct is_sync<Atomic<T>> : std::true_type {};

// Raw pointers are not Sync
template<typename T>
struct is_sync<T*> : std::false_type {};

template<typename T>
struct is_sync<const T*> : std::false_type {};

// ============================================================================
// Helper constexpr variables (C++17 compatible)
// ============================================================================

template<typename T>
inline constexpr bool Send = is_send<T>::value;

template<typename T>
inline constexpr bool Sync = is_sync<T>::value;

template<typename T>
inline constexpr bool ThreadSafe = Send<T> && Sync<T>;

} // namespace rusty
