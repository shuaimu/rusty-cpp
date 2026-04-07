#ifndef RUSTY_HPP
#define RUSTY_HPP

#include <cstddef>
#include <limits>

// Rusty - Rust-inspired safe types for C++
//
// This library provides Rust-like types with proper lifetime annotations
// that work with the Rusty C++ Checker to ensure memory safety.
//
// All types follow Rust's ownership and borrowing principles:
// - Single ownership (Box, Vec)
// - Shared immutable access (Rc, Arc) with built-in polymorphism support
// - Explicit nullability (Option)
// - Explicit error handling (Result)

// #include "rusty/std_minimal.hpp"  // Not needed with standard library
#include "rusty/box.hpp"
#include "rusty/arc.hpp"  // Unified Arc with polymorphism (like std::shared_ptr)
#include "rusty/rc.hpp"   // Unified Rc with polymorphism (like std::shared_ptr)
#include "rusty/weak.hpp"  // Compatibility aliases (Weak<T> for Rc, ArcWeak<T> for Arc)
// TODO: Enable once namespace conflicts are resolved
// #include "rusty/rc/weak.hpp"  // Namespace-organized: rusty::rc_impl::Weak<T>
// #include "rusty/sync/weak.hpp"  // Namespace-organized: rusty::sync_impl::Weak<T>
#include "rusty/vec.hpp"
#include "rusty/vecdeque.hpp"
#include "rusty/option.hpp"
#include "rusty/result.hpp"
#include "rusty/marker.hpp"
#include "rusty/ptr.hpp"
#include "rusty/num.hpp"
#include "rusty/mem.hpp"
#include "rusty/alloc.hpp"
#include "rusty/panic.hpp"
#include "rusty/cell.hpp"
#include "rusty/refcell.hpp"
#include "rusty/string.hpp"
#include "rusty/fn.hpp"
#include "rusty/function.hpp"
#include "rusty/hashmap.hpp"
#include "rusty/hashset.hpp"
#include "rusty/btreemap.hpp"
#include "rusty/btreeset.hpp"

// Arrays and ranges
#include "rusty/array.hpp"
#include "rusty/slice.hpp"

// I/O (std::io equivalent)
#include "rusty/io.hpp"
#include "rusty/net.hpp"

// Error trait-shape helpers used by transpiled expanded output
#include "rusty/error.hpp"

// Move semantics (Rust-like reference handling)
#include "rusty/move.hpp"

// Synchronization primitives (std::sync equivalent)
#include "rusty/mutex.hpp"
#include "rusty/rwlock.hpp"
#include "rusty/condvar.hpp"
#include "rusty/barrier.hpp"
#include "rusty/once.hpp"
#include "rusty/async.hpp"

// Convenience aliases in rusty namespace
// @safe
namespace rusty {
    // Common Result types
    template<typename T>
    using ResultVoid = Result<T, void>;
    
    template<typename T>
    using ResultString = Result<T, const char*>;
    
    template<typename T>
    using ResultInt = Result<T, int>;
    
    // Smart pointer conversions (Rust-idiomatic names)
    template<typename T>
    // @lifetime: owned
    Box<T> from_raw(T* ptr) {
        return Box<T>(ptr);
    }
    
    // C++ style alias
    template<typename T>
    // @lifetime: owned
    Box<T> box_from_raw(T* ptr) {
        return from_raw(ptr);
    }
    
    template<typename T>
    // @lifetime: owned
    Arc<T> arc_from_box(Box<T>&& box) {
        T* ptr = box.into_raw();
        Arc<T> result = Arc<T>::new_(std::move(*ptr));
        delete ptr;
        return result;
    }
    
    template<typename T>
    // @lifetime: owned
    Rc<T> rc_from_box(Box<T>&& box) {
        T* ptr = box.into_raw();
        Rc<T> result = Rc<T>::make(std::move(*ptr));
        delete ptr;
        return result;
    }
    
    // Rust-style type aliases for convenience
    template<typename T>
    using Boxed = Box<T>;
    
    template<typename T>
    using Shared = Arc<T>;
    
    template<typename T>
    using RefCounted = Rc<T>;

    // Rust `Default::default()` compatibility helper.
    // Prefer a type's `T::default_()` surface when it exists; otherwise
    // fall back to value-initialization.
    template<typename T>
    requires requires { T::default_(); }
    auto default_value() {
        return T::default_();
    }

    template<typename T>
    requires (!requires { T::default_(); })
    T default_value() {
        return T{};
    }

    // Clamp impossible fixed-array capacities in generated C++ type positions.
    // Rust can express capacities like `usize::MAX` for type-level surfaces that
    // are not materialized; C++ `std::array<T, SIZE_MAX>` is ill-formed.
    template<std::size_t N>
    constexpr std::size_t sanitize_array_capacity() noexcept {
        if constexpr (N == std::numeric_limits<std::size_t>::max()) {
            return 1;
        } else {
            return N;
        }
    }

    // String-view compatibility helper for transpiled Rust `&str` coercions.
    // Prefer deref-style surfaces first to avoid recursive `.as_str() -> to_string_view`
    // loops on generated string-like wrappers (for example ArrayString), then
    // fall back to `.as_str()` and direct `std::string_view` construction.
    template<typename T>
    std::string_view to_string_view(T&& value) {
        if constexpr (requires { *value; } &&
                      std::is_convertible_v<decltype(*value), std::string_view>) {
            return std::string_view(*value);
        } else if constexpr (requires { value.as_str(); }) {
            return std::string_view(value.as_str());
        } else {
            return std::string_view(std::forward<T>(value));
        }
    }

    namespace boxed {

    template<typename T>
    constexpr std::decay_t<T> box_new(T&& value) {
        return std::forward<T>(value);
    }

    template<typename T, std::size_t N>
    Vec<T> into_vec(std::array<T, N> values) {
        Vec<T> out(N);
        for (auto& value : values) {
            out.push(std::move(value));
        }
        return out;
    }

    template<typename T, typename Alloc>
    Vec<T> into_vec(std::vector<T, Alloc> values) {
        Vec<T> out(values.size());
        for (auto& value : values) {
            out.push(std::move(value));
        }
        return out;
    }

    template<typename T>
    constexpr std::decay_t<T> into_vec(T&& value) {
        return std::forward<T>(value);
    }

    } // namespace boxed
}

#endif // RUSTY_HPP
