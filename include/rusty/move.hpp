#pragma once

/// @file move.hpp
/// @brief Rust-like move semantics for C++
///
/// This header provides `rusty::move` which differs from `std::move` in how
/// references are handled:
///
/// - For values: Same as std::move - transfers ownership
/// - For mutable references (T&): Invalidates the reference variable itself
///   (not the underlying object), matching Rust's &mut T move semantics
/// - For const references (const T&): Compile error - use = to copy
///
/// This allows RustyCpp to track reference "moves" with Rust-like semantics,
/// where moving a mutable reference invalidates that reference variable.

#include <type_traits>
#include <utility>

namespace rusty {

/// @brief Move a value or mutable reference with Rust-like semantics
///
/// Unlike std::move, rusty::move treats references as first-class types:
/// - Moving a mutable reference invalidates that reference (not the underlying object)
/// - Moving a const reference is a compile error (use = to copy)
///
/// @tparam T The type being moved (deduced via forwarding reference)
/// @param t The value or reference to move
/// @return An rvalue reference enabling move semantics
///
/// @par Example
/// @code
/// int x = 42;
/// int& r1 = x;
/// int& r2 = rusty::move(r1);  // r1 is now invalid, r2 is valid
/// // use(r1);  // ERROR: use after move (detected by RustyCpp)
/// use(r2);     // OK
///
/// Box<int> b1 = Box<int>::make(10);
/// Box<int> b2 = rusty::move(b1);  // b1 is now invalid
/// // use(b1);  // ERROR: use after move
/// use(b2);     // OK
///
/// const int& cr = x;
/// // const int& cr2 = rusty::move(cr);  // COMPILE ERROR!
/// const int& cr2 = cr;  // Use = for const refs
/// @endcode
/// Move a value with Rust-like tracking semantics
///
/// This behaves identically to std::move at runtime - it casts the argument
/// to an rvalue reference to enable move semantics. The difference is purely
/// in how RustyCpp tracks it:
///
/// - For values: The value is marked as moved (same as std::move)
/// - For references (T&): The REFERENCE ITSELF is marked as moved/invalidated,
///   not just the underlying object. This matches Rust's semantics where
///   &mut T is not Copy.
///
/// For const references: Use = to copy (compile error if you try rusty::move)
template<typename T>
constexpr std::remove_reference_t<T>&& move(T&& t) noexcept {
    using BaseT = std::remove_reference_t<T>;

    // Disallow moving const lvalue references - just use = to copy them
    // This matches Rust where &T is Copy but &mut T is not
    static_assert(
        !(std::is_lvalue_reference_v<T> && std::is_const_v<BaseT>),
        "Cannot rusty::move a const reference (const T&). "
        "Const references are like Rust's &T which is Copy. "
        "Use assignment (=) to copy const references instead."
    );

    // Return an rvalue reference to enable move semantics (same as std::move)
    // The RustyCpp checker will track what was moved based on the argument type
    return static_cast<BaseT&&>(t);
}

/// @brief Explicitly copy a value (for clarity when move is the default)
///
/// In Rust-style code, moves are common. Use `rusty::copy` to make
/// explicit that you want a copy, not a move.
///
/// @note Only works for types that are Copy (trivially copyable or have copy ctor)
template<typename T>
constexpr T copy(const T& t) noexcept(std::is_nothrow_copy_constructible_v<T>) {
    static_assert(
        std::is_copy_constructible_v<T>,
        "rusty::copy requires a copyable type"
    );
    return t;
}

} // namespace rusty
