#ifndef RUSTY_HPP
#define RUSTY_HPP

#include <cstddef>
#include <cmath>
#include <limits>
#include <span>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>
#include <variant>

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
#include "rusty/fmt.hpp"
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
#include "rusty/winnow_stream.hpp"

// I/O (std::io equivalent)
#include "rusty/io.hpp"
#include "rusty/net.hpp"
#include "rusty/process.hpp"

// Error trait-shape helpers used by transpiled expanded output
#include "rusty/error.hpp"

// Move semantics (Rust-like reference handling)
#include "rusty/move.hpp"

// Synchronization primitives (std::sync equivalent)
#include "rusty/sync/atomic.hpp"
#include "rusty/sync/mpsc.hpp"
#include "rusty/mutex.hpp"
#include "rusty/rwlock.hpp"
#include "rusty/condvar.hpp"
#include "rusty/barrier.hpp"
#include "rusty/once.hpp"
#include "rusty/thread.hpp"
#include "rusty/async.hpp"

// Convenience aliases in rusty namespace
// @safe
namespace rusty {
    using Unit = std::tuple<>;
    using StrView = std::string_view;
    template<typename T, std::size_t Extent = std::dynamic_extent>
    using Span = std::span<T, Extent>;

    template<typename T>
    constexpr T&& forward(std::remove_reference_t<T>& value) noexcept {
        return static_cast<T&&>(value);
    }

    template<typename T>
    constexpr T&& forward(std::remove_reference_t<T>&& value) noexcept {
        static_assert(
            !std::is_lvalue_reference_v<T>,
            "rusty::forward<T>(value) with lvalue-reference T requires an lvalue");
        return static_cast<T&&>(value);
    }

    template<typename T, typename U = T>
    constexpr T exchange(T& target, U&& replacement)
        noexcept(std::is_nothrow_move_constructible_v<T> && std::is_nothrow_assignable_v<T&, U&&>) {
        T old = rusty::move(target);
        target = rusty::forward<U>(replacement);
        return old;
    }

    template<typename T>
    constexpr void swap(T& lhs, T& rhs) noexcept(noexcept(std::swap(lhs, rhs))) {
        using std::swap;
        swap(lhs, rhs);
    }

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

    namespace sync {
        template<typename T>
        using Arc = ::rusty::Arc<T>;
    } // namespace sync

    // Rust `Default::default()` compatibility helper.
    // Prefer a type's `T::default_()` surface when it exists; otherwise
    // fall back to value-initialization.
    template<typename T>
    requires requires { T::default_(); }
    auto default_value() {
        return T::default_();
    }

    template<typename T>
    requires (!requires { T::default_(); } && requires { T::empty(); })
    auto default_value() {
        return T::empty();
    }

    template<typename T>
    requires (!requires { T::default_(); } && !requires { T::empty(); } && requires { T{}; })
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

    namespace detail {
        template<typename T>
        struct is_std_variant : std::false_type {};

        template<typename... Ts>
        struct is_std_variant<std::variant<Ts...>> : std::true_type {};

        template<typename T>
        inline constexpr bool is_std_variant_v =
            is_std_variant<std::remove_cv_t<std::remove_reference_t<T>>>::value;

        template<typename Variant>
        constexpr decltype(auto) as_variant_ref(Variant&& value) {
            using Raw = std::remove_reference_t<Variant>;
            using Bare = std::remove_cv_t<Raw>;
            if constexpr (requires { typename Bare::variant; }) {
                using Underlying = typename Bare::variant;
                if constexpr (std::is_const_v<Raw>) {
                    return static_cast<const Underlying&>(value);
                } else {
                    return static_cast<Underlying&>(value);
                }
            } else {
                return std::forward<Variant>(value);
            }
        }

        template<typename T, typename Variant, std::size_t Index = 0>
        constexpr bool variant_holds_impl(const Variant& value) {
            if constexpr (Index >= std::variant_size_v<Variant>) {
                return false;
            } else {
                if constexpr (std::is_same_v<T, std::variant_alternative_t<Index, Variant>>) {
                    if (value.index() == Index) {
                        return true;
                    }
                }
                return variant_holds_impl<T, Variant, Index + 1>(value);
            }
        }

        // `std::holds_alternative<T>` rejects variants with duplicate `T`.
        // This helper returns true if *any* matching alternative index is active.
        template<typename T, typename VariantLike>
        constexpr bool variant_holds(VariantLike&& value) {
            auto&& variant_ref = as_variant_ref(std::forward<VariantLike>(value));
            using Variant = std::remove_cv_t<std::remove_reference_t<decltype(variant_ref)>>;
            if constexpr (!is_std_variant_v<Variant>) {
                return false;
            } else {
                return variant_holds_impl<T, Variant>(variant_ref);
            }
        }

        template<typename T, typename Variant, std::size_t Index = 0>
        constexpr T* variant_get_if_impl(Variant& value) {
            if constexpr (Index >= std::variant_size_v<Variant>) {
                return nullptr;
            } else {
                if constexpr (std::is_same_v<T, std::variant_alternative_t<Index, Variant>>) {
                    if (value.index() == Index) {
                        return &std::get<Index>(value);
                    }
                }
                return variant_get_if_impl<T, Variant, Index + 1>(value);
            }
        }

        template<typename T, typename Variant, std::size_t Index = 0>
        constexpr const T* variant_get_if_impl(const Variant& value) {
            if constexpr (Index >= std::variant_size_v<Variant>) {
                return nullptr;
            } else {
                if constexpr (std::is_same_v<T, std::variant_alternative_t<Index, Variant>>) {
                    if (value.index() == Index) {
                        return &std::get<Index>(value);
                    }
                }
                return variant_get_if_impl<T, Variant, Index + 1>(value);
            }
        }

        template<typename T, typename VariantLike>
        constexpr decltype(auto) variant_get(VariantLike&& value) {
            auto&& variant_ref = as_variant_ref(std::forward<VariantLike>(value));
            using Variant = std::remove_cv_t<std::remove_reference_t<decltype(variant_ref)>>;
            static_assert(
                is_std_variant_v<Variant>,
                "variant_get requires a std::variant-like value");
            if constexpr (std::is_const_v<std::remove_reference_t<decltype(variant_ref)>>) {
                if (const T* ptr = variant_get_if_impl<T, Variant>(variant_ref)) {
                    return *ptr;
                }
            } else {
                if (T* ptr = variant_get_if_impl<T, Variant>(variant_ref)) {
                    return *ptr;
                }
            }
            throw std::bad_variant_access();
        }

        // Parse decimal literals into 128-bit integers without relying on host
        // integer literal width (which can reject valid Rust u128/i128 values).
        template<typename Int>
        constexpr Int parse_decimal_int_literal(const char* digits) {
            static_assert(
                std::is_same_v<Int, unsigned __int128> || std::is_same_v<Int, __int128>,
                "parse_decimal_int_literal supports 128-bit integer types only");
            unsigned __int128 value = 0;
            for (const char* p = digits; *p != '\0'; ++p) {
                value = (value * static_cast<unsigned __int128>(10))
                    + static_cast<unsigned __int128>(*p - '0');
            }
            if constexpr (std::is_same_v<Int, unsigned __int128>) {
                return value;
            } else {
                return static_cast<__int128>(value);
            }
        }
    } // namespace detail

    // String-view compatibility helper for transpiled Rust `&str` coercions.
    // Prefer deref-style surfaces first to avoid recursive `.as_str() -> to_string_view`
    // loops on generated string-like wrappers (for example ArrayString), then
    // fall back to `.as_str()` and direct `std::string_view` construction.
    template<typename T>
    std::string_view to_string_view(T&& value) {
        // Use a single requires expression so decltype(*value) is not
        // evaluated eagerly when *value is not a valid expression.
        if constexpr (requires { { *value } -> std::convertible_to<std::string_view>; }) {
            return std::string_view(*value);
        } else if constexpr (requires { value.as_str(); }) {
            return std::string_view(value.as_str());
        } else {
            return std::string_view(std::forward<T>(value));
        }
    }

    inline String to_owned(std::string_view value) {
        return String::from(value);
    }

    inline String to_owned(const char* value) {
        return String::from(value);
    }

    inline String to_owned(const str& value) {
        return String::from(value.as_str());
    }

    template<typename T, std::size_t Extent>
    Vec<std::remove_const_t<T>> to_owned(std::span<T, Extent> value) {
        using Elem = std::remove_const_t<T>;
        Vec<Elem> out(value.size());
        for (const auto& item : value) {
            out.push(static_cast<Elem>(item));
        }
        return out;
    }

    template<typename T>
    auto to_owned(const T& value) {
        if constexpr (requires { value.clone(); }) {
            return value.clone();
        } else {
            return T(value);
        }
    }

    template<typename Left, typename Right>
    auto join(Left&& left, Right&& right) {
        if constexpr (requires { std::forward<Left>(left).join(std::forward<Right>(right)); }) {
            return std::forward<Left>(left).join(std::forward<Right>(right));
        } else {
            auto&& range = left;
            std::string delimiter;
            if constexpr (std::is_convertible_v<Right, std::string_view>) {
                delimiter = std::string(std::string_view(std::forward<Right>(right)));
            } else if constexpr (requires { std::forward<Right>(right).as_str(); }) {
                delimiter = std::string(std::string_view(std::forward<Right>(right).as_str()));
            } else {
                delimiter = std::string(std::forward<Right>(right));
            }

            std::string out;
            bool first = true;
            for (const auto& item : range) {
                if (!first) {
                    out += delimiter;
                }
                first = false;
                if constexpr (std::is_convertible_v<decltype(item), std::string_view>) {
                    out += std::string_view(item);
                } else if constexpr (requires { item.as_str(); }) {
                    out += std::string_view(item.as_str());
                } else if constexpr (requires { item.as_ref(); }) {
                    out += std::string_view(item.as_ref());
                } else {
                    out += std::string(item);
                }
            }
            return out;
        }
    }

    template<typename T>
    Option<std::decay_t<T>> then_some(bool condition, T&& value) {
        if (condition) {
            return Option<std::decay_t<T>>(std::forward<T>(value));
        }
        return Option<std::decay_t<T>>(None);
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

    #if !defined(RUSTY_NO_STD_VECTOR_INTEROP)
    template<typename T, typename Alloc>
    Vec<T> into_vec(std::vector<T, Alloc> values) {
        Vec<T> out(values.size());
        for (auto& value : values) {
            out.push(std::move(value));
        }
        return out;
    }
    #endif

    template<typename T>
    constexpr std::decay_t<T> into_vec(T&& value) {
        return std::forward<T>(value);
    }

    } // namespace boxed
}

namespace ser::impls::rusty_ext {
    template<typename S, typename T, std::size_t Extent>
    requires requires(S serializer, std::span<T, Extent> self_) { serializer.collect_seq(self_); }
    auto serialize(std::span<T, Extent> self_, S serializer) {
        return serializer.collect_seq(self_);
    }

    template<typename S, typename T, std::size_t Extent>
    requires (
        !requires(S serializer, std::span<T, Extent> self_) { serializer.collect_seq(self_); }
        && (
            requires(S serializer, std::span<T, Extent> self_) {
                serializer.serialize_seq(static_cast<size_t>(self_.size()));
            }
            || requires(S serializer, std::span<T, Extent> self_) {
                serializer.serialize_seq(rusty::Option<size_t>(self_.size()));
            }
        )
    )
    auto serialize(std::span<T, Extent> self_, S serializer) {
        auto state_result = [&]() {
            if constexpr (requires { serializer.serialize_seq(rusty::Option<size_t>(self_.size())); }) {
                return serializer.serialize_seq(rusty::Option<size_t>(self_.size()));
            } else {
                return serializer.serialize_seq(static_cast<size_t>(self_.size()));
            }
        }();
        using state_type = std::remove_reference_t<decltype(
            rusty::detail::deref_if_pointer(std::declval<decltype(state_result.unwrap())>())
        )>;
        using return_type = decltype(std::declval<state_type&>().end());
        if (state_result.is_err()) {
            return return_type::Err(std::move(state_result.unwrap_err()));
        }
        auto&& state_ref = state_result.unwrap();
        auto&& state = rusty::detail::deref_if_pointer(state_ref);
        for (const auto& item : self_) {
            auto element_result = state.serialize_element(item);
            if (element_result.is_err()) {
                return return_type::Err(std::move(element_result.unwrap_err()));
            }
        }
        return state.end();
    }

    template<typename S, typename M>
    requires (
        requires(const M& m) { m.iter(); }
        && requires(const M& m) { rusty::len(m); }
        && (
            requires(S serializer, const M& m) {
                serializer.serialize_map(static_cast<size_t>(rusty::len(m)));
            }
            || requires(S serializer, const M& m) {
                serializer.serialize_map(rusty::Option<size_t>(rusty::len(m)));
            }
        )
    )
    auto serialize(const M& self_, S serializer) {
        auto state_result = [&]() {
            if constexpr (requires { serializer.serialize_map(rusty::Option<size_t>(rusty::len(self_))); }) {
                return serializer.serialize_map(rusty::Option<size_t>(rusty::len(self_)));
            } else {
                return serializer.serialize_map(static_cast<size_t>(rusty::len(self_)));
            }
        }();

        using state_type = std::remove_reference_t<decltype(
            rusty::detail::deref_if_pointer(std::declval<decltype(state_result.unwrap())>())
        )>;
        using return_type = decltype(std::declval<state_type&>().end());

        if (state_result.is_err()) {
            return return_type::Err(std::move(state_result.unwrap_err()));
        }

        auto&& state_ref = state_result.unwrap();
        auto&& state = rusty::detail::deref_if_pointer(state_ref);

        for (auto&& item : rusty::for_in(rusty::iter(self_))) {
            auto&& pair_item = rusty::detail::deref_if_pointer(std::forward<decltype(item)>(item));
            auto&& key = rusty::detail::deref_if_pointer(std::get<0>(pair_item));
            auto&& value = rusty::detail::deref_if_pointer(std::get<1>(pair_item));
            if constexpr (requires { state.serialize_entry(key, value); }) {
                auto entry_result = state.serialize_entry(key, value);
                if (entry_result.is_err()) {
                    return return_type::Err(std::move(entry_result.unwrap_err()));
                }
            } else if constexpr (requires { state.serialize_key(key); state.serialize_value(value); }) {
                auto key_result = state.serialize_key(key);
                if (key_result.is_err()) {
                    return return_type::Err(std::move(key_result.unwrap_err()));
                }
                auto value_result = state.serialize_value(value);
                if (value_result.is_err()) {
                    return return_type::Err(std::move(value_result.unwrap_err()));
                }
            } else {
                static_assert(
                    requires { state.serialize_entry(key, value); },
                    "serialize_map state must provide serialize_entry or serialize_key/serialize_value");
            }
        }

        return state.end();
    }
} // namespace ser::impls::rusty_ext

// std::formatter specialization for char32_t (Rust char → C++ char32_t)
// Required for std::format with char32_t arguments.
#include <format>
template<>
struct std::formatter<char32_t> : std::formatter<uint32_t> {
    auto format(char32_t c, std::format_context& ctx) const {
        return std::formatter<uint32_t>::format(static_cast<uint32_t>(c), ctx);
    }
};

#endif // RUSTY_HPP
