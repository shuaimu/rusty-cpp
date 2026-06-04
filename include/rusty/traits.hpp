#pragma once

#include <concepts>
#include <type_traits>
#include <memory>

namespace rusty {

// Forward declarations for rusty types
template<typename T> class Arc;
// `rusty::Rc<T, A>` is the transpiled rustc Rc; it lives in module
// `rc_port`'s purview. C++20 modules forbid forward-declaring the same
// name in the global module and inside a named module, so no fwd-decl
// here. Its `is_send` / `is_sync` impls live alongside it.
template<typename T, typename A> class Box;
template<typename T> class Cell;
template<typename T> class RefCell;
template<typename T> class Mutex;
namespace sync::atomic::detail {
template<typename T> class Atomic;
} // namespace sync::atomic::detail
// Note: the transpiled `rusty::port::sync::Arc<T, A>` / `Weak<T, A>`
// live inside the `arc_port` module purview, so we cannot forward-
// declare them here (C++20 modules forbid declaring the same name in
// the global module and inside a named module). Their `is_send` /
// `is_sync` specializations are injected at the tail of arc_port.cppm
// by the patcher (see docs/arc_port/post_transpile_patch.py
// `patch_arc_traits_specializations`).

// Forward declare is_sync for circular dependency with is_send
template<typename T>
struct is_sync;

// ============================================================================
// Explicit Opt-in Send Trait System (for user-defined types)
// ============================================================================

// Explicit opt-in via specialization
// Usage: template<> struct rusty::is_explicitly_send<MyType> : std::true_type {};
template<typename T>
struct is_explicitly_send : std::false_type {};

// ============================================================================
// Send Trait - Can transfer ownership across thread boundaries
// ============================================================================

// Default: types are NOT Send
template<typename T>
struct is_send : std::false_type {};

// Primitives are Send
template<typename T>
    requires std::is_arithmetic_v<T>
struct is_send<T> : std::true_type {};

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

// Arc<T> is Send if T is Send + Sync (hand-written rusty::Arc<T>).
// Specializations for the transpiled `port::sync::Arc<T, A>` /
// `Weak<T, A>` are injected at the tail of arc_port.cppm by the
// patcher — see the note above the forward-decl block.
template<typename T>
struct is_send<Arc<T>> : std::bool_constant<
    is_send<T>::value && is_sync<T>::value
> {};

// Rc<T> is NEVER Send — specialization lives in rc_port.cppm alongside
// the type; cannot specialize across the module boundary from GMF.

// Box<T, A> is Send if T is Send
template<typename T, typename A>
struct is_send<Box<T, A>> : is_send<T> {};

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
template<typename T>
struct is_sync : std::false_type {};

// Primitives are Sync
template<typename T>
    requires std::is_arithmetic_v<T>
struct is_sync<T> : std::true_type {};

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

// Arc<T> is Sync if T is Send + Sync (hand-written rusty::Arc<T>).
// `port::sync::Arc<T, A>` / `Weak<T, A>` specializations are injected
// at the tail of arc_port.cppm by the patcher.
template<typename T>
struct is_sync<Arc<T>> : std::bool_constant<
    is_send<T>::value && is_sync<T>::value
> {};

// Rc<T> is NEVER Sync — specialization lives in rc_port.cppm alongside
// the type; cannot specialize across the module boundary from GMF.

// Box<T, A> is Sync if T is Sync
template<typename T, typename A>
struct is_sync<Box<T, A>> : is_sync<T> {};

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

// Atomic<T> is Sync (private detail template behind the public
// concrete aliases like `AtomicBool`/`AtomicU64`).
template<typename T>
struct is_sync<sync::atomic::detail::Atomic<T>> : std::true_type {};

// Raw pointers are not Sync
template<typename T>
struct is_sync<T*> : std::false_type {};

template<typename T>
struct is_sync<const T*> : std::false_type {};

// ============================================================================
// C++20 Concepts
// ============================================================================

template<typename T>
concept Send = is_send<T>::value;

template<typename T>
concept Sync = is_sync<T>::value;

template<typename T>
concept ThreadSafe = Send<T> && Sync<T>;

} // namespace rusty
