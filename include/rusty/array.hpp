#ifndef RUSTY_ARRAY_HPP
#define RUSTY_ARRAY_HPP

#include <array>
#include <vector>
#include <cstddef>
#include <cstdint>
#include <algorithm>
#include <iterator>
#include <type_traits>
#include <utility>
#include <optional>
#include <cstring>
#include <limits>
#include <span>
#include <stdexcept>
#include <tuple>
#include <variant>
#include <functional>
#include <new>
#include <rusty/vec.hpp>
#include <rusty/box.hpp>
#include <rusty/maybe_uninit.hpp>

namespace rusty {
template<typename Container>
decltype(auto) as_slice(Container&& container);
}

// GCC/libstdc++ C++23 does not provide span equality operators.
// Keep a narrow value-comparison overload so transpiled Rust slice assertions compile.
template<typename L, std::size_t LExtent, typename R, std::size_t RExtent>
constexpr bool operator==(std::span<L, LExtent> lhs, std::span<R, RExtent> rhs) {
    if (lhs.size() != rhs.size()) {
        return false;
    }
    if constexpr (requires(const L& l, const R& r) { l == r; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(l == r); });
    } else if constexpr (requires(const L& l, const R& r) { r == l; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(r == l); });
    } else {
        return std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>;
    }
}

template<typename L, std::size_t LExtent, typename R, std::size_t N>
constexpr bool operator==(std::span<L, LExtent> lhs, const std::array<R, N>& rhs) {
    return lhs == std::span<const R, N>(rhs);
}

template<typename L, std::size_t N, typename R, std::size_t RExtent>
constexpr bool operator==(const std::array<L, N>& lhs, std::span<R, RExtent> rhs) {
    return rhs == lhs;
}

// Vec/slice assertion scaffolding often compares owned buffers (`std::vector`)
// with borrowed slice views (`std::span`). Mirror Rust slice equality semantics
// for these mixed container/view shapes.
template<typename L, typename Alloc, typename R, std::size_t RExtent>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(const std::vector<L, Alloc>& lhs, std::span<R, RExtent> rhs) {
    if (lhs.size() != rhs.size()) {
        return false;
    }
    if constexpr (requires(const L& l, const R& r) { l == r; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(l == r); });
    } else if constexpr (requires(const L& l, const R& r) { r == l; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(r == l); });
    } else {
        return true;
    }
}

template<typename L, std::size_t LExtent, typename R, typename Alloc>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(std::span<L, LExtent> lhs, const std::vector<R, Alloc>& rhs) {
    return rhs == lhs;
}

// Mixed-element std::array equality for transpiled assertion scaffolding.
// Keep this narrow: only for different element types and only when one-sided
// element equality is well-formed.
template<typename L, std::size_t N, typename R>
requires (
    !std::is_same_v<std::remove_cv_t<L>, std::remove_cv_t<R>> &&
    (requires(const L& l, const R& r) { l == r; } ||
     requires(const L& l, const R& r) { r == l; }))
constexpr bool operator==(const std::array<L, N>& lhs, const std::array<R, N>& rhs) {
    if constexpr (requires(const L& l, const R& r) { l == r; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(l == r); });
    } else {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(r == l); });
    }
}

template<typename T, typename = void>
struct has_member_as_slice : std::false_type {};

template<typename T>
struct has_member_as_slice<T, std::void_t<decltype(std::declval<const T&>().as_slice())>>
    : std::true_type {};

// Mixed equality between std::array and container-like types that expose
// `.as_slice()` (for example transpiled SmallVec and rusty::Vec surfaces).
template<typename L, typename R, std::size_t N>
requires has_member_as_slice<L>::value
constexpr bool operator==(const L& lhs, const std::array<R, N>& rhs) {
    const auto lhs_slice = rusty::as_slice(lhs);
    if (lhs_slice.size() != rhs.size()) {
        return false;
    }
    using LElem = std::remove_cv_t<std::remove_reference_t<decltype(*lhs_slice.begin())>>;
    if constexpr (requires(const LElem& l, const R& r) { l == r; }) {
        return std::equal(
            lhs_slice.begin(),
            lhs_slice.end(),
            rhs.begin(),
            [](const LElem& l, const R& r) { return static_cast<bool>(l == r); });
    } else if constexpr (requires(const LElem& l, const R& r) { r == l; }) {
        return std::equal(
            lhs_slice.begin(),
            lhs_slice.end(),
            rhs.begin(),
            [](const LElem& l, const R& r) { return static_cast<bool>(r == l); });
    } else if constexpr (std::is_empty_v<LElem> && std::is_empty_v<std::remove_cv_t<R>>) {
        return true;
    } else {
        return false;
    }
}

template<typename L, typename R, std::size_t RExtent>
requires has_member_as_slice<L>::value
constexpr bool operator==(const L& lhs, std::span<R, RExtent> rhs) {
    return rusty::as_slice(lhs) == rhs;
}

template<typename L, std::size_t LExtent, typename R>
requires has_member_as_slice<R>::value
constexpr bool operator==(std::span<L, LExtent> lhs, const R& rhs) {
    return rhs == lhs;
}

template<typename L, std::size_t N, typename R>
requires has_member_as_slice<R>::value
constexpr bool operator==(const std::array<L, N>& lhs, const R& rhs) {
    return rhs == lhs;
}

template<typename L, typename R>
requires (has_member_as_slice<L>::value && has_member_as_slice<R>::value)
constexpr bool operator==(const L& lhs, const R& rhs) {
    return rusty::as_slice(lhs) == rusty::as_slice(rhs);
}

// std::array vs std::vector — bitflags' transpiled assertions wrap a
// fixed-size literal as `std::array{…}` then compare it against the
// `std::vector` returned by `rusty::collect_range`. Without these
// overloads clang reports "invalid operands to binary expression".
template<typename L, std::size_t N, typename R, typename Alloc>
requires (
    requires(const L& l, const R& r) { static_cast<bool>(l == r); } ||
    requires(const L& l, const R& r) { static_cast<bool>(r == l); })
constexpr bool operator==(const std::array<L, N>& lhs,
                          const std::vector<R, Alloc>& rhs) {
    if (lhs.size() != rhs.size()) return false;
    if constexpr (requires(const L& l, const R& r) { static_cast<bool>(l == r); }) {
        return std::equal(lhs.begin(), lhs.end(), rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(l == r); });
    } else {
        return std::equal(lhs.begin(), lhs.end(), rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(r == l); });
    }
}

template<typename L, typename Alloc, typename R, std::size_t N>
requires (
    requires(const L& l, const R& r) { static_cast<bool>(l == r); } ||
    requires(const L& l, const R& r) { static_cast<bool>(r == l); })
constexpr bool operator==(const std::vector<L, Alloc>& lhs,
                          const std::array<R, N>& rhs) {
    return rhs == lhs;
}

// rusty::Vec (has_member_as_slice) vs std::vector — after the
// `into_vec(std::array<T,N>)` overload landed (vec_port.vec.cppm),
// bitflags' `vec![…]` macro lowers to a rusty::Vec rather than a
// std::array, so the previous std::array-vs-std::vector overloads
// don't fire any more. Compare element-by-element via .data()/.size().
template<typename L, typename R, typename Alloc>
requires (
    has_member_as_slice<L>::value &&
    (requires(const decltype(*std::declval<L>().as_slice().begin())& l, const R& r) {
        static_cast<bool>(l == r);
     } ||
     requires(const R& r, const decltype(*std::declval<L>().as_slice().begin())& l) {
        static_cast<bool>(r == l);
     }))
constexpr bool operator==(const L& lhs, const std::vector<R, Alloc>& rhs) {
    const auto slice = rusty::as_slice(lhs);
    if (slice.size() != rhs.size()) return false;
    using LElem = std::remove_cvref_t<decltype(*slice.begin())>;
    if constexpr (requires(const LElem& l, const R& r) { static_cast<bool>(l == r); }) {
        return std::equal(slice.begin(), slice.end(), rhs.begin(),
            [](const LElem& l, const R& r) { return static_cast<bool>(l == r); });
    } else {
        return std::equal(slice.begin(), slice.end(), rhs.begin(),
            [](const LElem& l, const R& r) { return static_cast<bool>(r == l); });
    }
}

template<typename L, typename Alloc, typename R>
requires (
    has_member_as_slice<R>::value &&
    (requires(const L& l, const decltype(*std::declval<R>().as_slice().begin())& r) {
        static_cast<bool>(l == r);
     } ||
     requires(const decltype(*std::declval<R>().as_slice().begin())& r, const L& l) {
        static_cast<bool>(r == l);
     }))
constexpr bool operator==(const std::vector<L, Alloc>& lhs, const R& rhs) {
    return rhs == lhs;
}

namespace rusty {

namespace detail {
template<typename T>
inline constexpr bool collect_range_dependent_false_v = false;

template<typename Index>
size_t checked_index(Index idx) {
    if constexpr (std::is_signed_v<Index>) {
        if (idx < 0) {
            throw std::out_of_range("slice index cannot be negative");
        }
    }
    return static_cast<size_t>(idx);
}

// Rust str::get returns None unless BOTH range ends land on UTF-8 char
// boundaries (a position is a boundary iff it's at either end or the byte
// there is not a continuation byte).
inline bool str_is_char_boundary(std::string_view s, size_t pos) {
    if (pos == 0 || pos == s.size()) {
        return true;
    }
    if (pos > s.size()) {
        return false;
    }
    return (static_cast<unsigned char>(s[pos]) & 0xC0) != 0x80;
}

template<typename SpanLike>
void validate_slice_bounds(const SpanLike& span, size_t start, size_t end) {
    if (start > end || end > span.size()) {
        throw std::out_of_range("slice range out of bounds");
    }
}

template<typename T, typename = void>
struct has_is_some : std::false_type {};

template<typename T>
struct has_is_some<T, std::void_t<decltype(std::declval<const T&>().is_some())>> : std::true_type {};

template<typename T, typename = void>
struct has_unwrap : std::false_type {};

template<typename T>
struct has_unwrap<T, std::void_t<decltype(std::declval<T&>().unwrap())>> : std::true_type {};

template<typename T, typename = void>
struct has_has_value : std::false_type {};

template<typename T>
struct has_has_value<T, std::void_t<decltype(std::declval<const T&>().has_value())>> : std::true_type {};

template<typename T, typename = void>
struct has_reset : std::false_type {};

template<typename T>
struct has_reset<T, std::void_t<decltype(std::declval<T&>().reset())>> : std::true_type {};

template<typename Opt>
bool option_has_value(const Opt& opt) {
    if constexpr (has_is_some<Opt>::value) {
        return opt.is_some();
    } else if constexpr (has_has_value<Opt>::value) {
        return opt.has_value();
    } else {
        return static_cast<bool>(opt);
    }
}

template<typename Opt>
decltype(auto) option_take_value(Opt& opt) {
    if constexpr (has_unwrap<Opt>::value) {
        return opt.unwrap();
    } else if constexpr (has_has_value<Opt>::value && has_reset<Opt>::value) {
        auto value = std::move(*opt);
        opt.reset();
        return value;
    } else {
        return std::move(*opt);
    }
}

template<typename Opt>
using option_value_t = std::decay_t<decltype(option_take_value(std::declval<Opt&>()))>;

template<typename VecLike, typename Item>
void push_collect_item(VecLike& out, Item&& item) {
    using ItemRef = Item&&;
    using ItemValue = std::remove_cvref_t<ItemRef>;
    if constexpr (requires(VecLike& v, Item&& i) { v.push(std::forward<Item>(i)); }) {
        out.push(std::forward<Item>(item));
    } else if constexpr (requires(const ItemValue& value) { value.clone(); }) {
        out.push(item.clone());
    } else if constexpr (!std::is_const_v<std::remove_reference_t<ItemRef>>
                         && std::is_move_constructible_v<ItemValue>) {
        out.push(std::move(item));
    } else {
        static_assert(
            std::is_copy_constructible_v<ItemValue>,
            "cannot collect/filter_map non-copy, non-cloneable elements");
    }
}

template<typename Range>
using range_storage_t =
    std::conditional_t<std::is_lvalue_reference_v<Range>, Range, std::decay_t<Range>>;

template<typename T>
struct maybe_uninit_payload {
    using type = void;
};

template<typename U>
struct maybe_uninit_payload<rusty::MaybeUninit<U>> {
    using type = U;
};

template<typename Container>
using container_item_t = typename std::remove_reference_t<Container>::Item;

template<typename Container, typename = void>
struct has_container_item : std::false_type {};

template<typename Container>
struct has_container_item<Container, std::void_t<container_item_t<Container>>> : std::true_type {};

template<typename Container>
using container_value_type_t = typename std::remove_reference_t<Container>::value_type;

template<typename Container, typename = void>
struct has_container_value_type : std::false_type {};

template<typename Container>
struct has_container_value_type<Container, std::void_t<container_value_type_t<Container>>>
    : std::true_type {};

template<
    typename Container,
    bool HasItem = has_container_item<Container>::value,
    bool HasValueType = has_container_value_type<Container>::value>
struct associated_item_impl;

template<typename Container, bool HasValueType>
struct associated_item_impl<Container, true, HasValueType> {
    using type = std::remove_reference_t<container_item_t<Container>>;
};

template<typename Container>
struct associated_item_impl<Container, false, true> {
    using type = std::remove_reference_t<container_value_type_t<Container>>;
};

template<typename Container>
using associated_item_t = typename associated_item_impl<Container>::type;

// First template type argument of a class template, else void.
template<typename T>
struct first_type_arg {
    using type = void;
};
template<template<typename...> class Tmpl, typename First, typename... Rest>
struct first_type_arg<Tmpl<First, Rest...>> {
    using type = First;
};

// Item type for a container the transpiler can't introspect — a user-defined
// wrapper (e.g. smallvec's `SmallVec<A>`) that exposes neither an `Item` alias nor
// a `value_type`. Such a wrapper holds the elements of its first type argument, so
// its item is that argument's item: recurse via `associated_item_t<First>`
// (`SmallVec<std::array<u32,2>>` → `associated_item_t<std::array<u32,2>>` → `u32`).
// When there is no type argument to recurse into, map to the container itself — a
// COMPLETE type that simply fails the downstream `is_same_v` comparisons. (`void`
// would ill-form `std::remove_reference_t`/reference uses.) Recursion terminates at
// the first arg that has an Item/value_type, or at a non-template leaf.
template<typename Container, typename First = typename first_type_arg<Container>::type>
struct unknown_container_item {
    using type = associated_item_t<First>;
};
template<typename Container>
struct unknown_container_item<Container, void> {
    using type = Container;
};

// `<C, false, false>`: neither an `Item` alias nor a `value_type`. Avoids the
// undefined-primary hard error ("implicit instantiation of undefined template")
// for such a container.
template<typename Container>
struct associated_item_impl<Container, false, false> {
    using type = typename unknown_container_item<Container>::type;
};

template<typename T>
constexpr size_t type_level_size() {
    using Raw = std::remove_cv_t<std::remove_reference_t<T>>;
    if constexpr (requires { std::tuple_size<Raw>::value; }) {
        return std::tuple_size_v<Raw>;
    } else if constexpr (requires { Raw::size(); }) {
        return static_cast<size_t>(Raw::size());
    } else {
        static_assert(
            collect_range_dependent_false_v<Raw>,
            "type_level_size requires tuple_size or static size() surface");
        return 0;
    }
}

template<typename Ptr>
using raw_ptr_t = std::remove_cv_t<std::remove_reference_t<Ptr>>;

template<typename Container, typename Ptr>
decltype(auto) adapt_as_ptr_result(const Container&, Ptr ptr) {
    using PtrRaw = raw_ptr_t<Ptr>;
    if constexpr (!std::is_pointer_v<PtrRaw>) {
        return ptr;
    } else {
        using Pointee = std::remove_pointer_t<PtrRaw>;
        using MaybeUninitT = std::remove_cv_t<Pointee>;
        using Payload = typename maybe_uninit_payload<MaybeUninitT>::type;
        constexpr bool has_associated_item =
            has_container_item<Container>::value || has_container_value_type<Container>::value;
        if constexpr (!std::is_void_v<Payload> && has_associated_item) {
            using Assoc = std::remove_reference_t<associated_item_t<Container>>;
            using AssocRaw = std::remove_cv_t<Assoc>;
            if constexpr (std::is_same_v<AssocRaw, MaybeUninitT>) {
                return ptr;
            } else {
                using AssocPayload = typename maybe_uninit_payload<std::remove_cv_t<Assoc>>::type;
                using AssocValue = std::conditional_t<
                    std::is_void_v<AssocPayload>,
                    std::remove_cv_t<Assoc>,
                    std::remove_cv_t<AssocPayload>>;
                if constexpr (std::is_same_v<AssocValue, std::remove_cv_t<Payload>>) {
                    using AdaptedPointee = std::add_const_t<AssocValue>;
                    using AdaptedPtr = std::add_pointer_t<AdaptedPointee>;
                    return reinterpret_cast<AdaptedPtr>(ptr);
                } else {
                    return ptr;
                }
            }
        } else {
            return ptr;
        }
    }
}

template<typename Container, typename Ptr>
decltype(auto) adapt_as_mut_ptr_result(Container&, Ptr ptr) {
    using PtrRaw = raw_ptr_t<Ptr>;
    if constexpr (!std::is_pointer_v<PtrRaw>) {
        return ptr;
    } else {
        using Pointee = std::remove_pointer_t<PtrRaw>;
        using MaybeUninitT = std::remove_cv_t<Pointee>;
        using Payload = typename maybe_uninit_payload<MaybeUninitT>::type;
        constexpr bool has_associated_item =
            has_container_item<Container>::value || has_container_value_type<Container>::value;
        if constexpr (!std::is_void_v<Payload> && has_associated_item) {
            using Assoc = std::remove_reference_t<associated_item_t<Container>>;
            using AssocRaw = std::remove_cv_t<Assoc>;
            if constexpr (std::is_same_v<AssocRaw, MaybeUninitT>) {
                return ptr;
            } else {
                using AssocPayload = typename maybe_uninit_payload<std::remove_cv_t<Assoc>>::type;
                using AssocValue = std::conditional_t<
                    std::is_void_v<AssocPayload>,
                    std::remove_cv_t<Assoc>,
                    std::remove_cv_t<AssocPayload>>;
                if constexpr (std::is_same_v<AssocValue, std::remove_cv_t<Payload>>) {
                    using AdaptedPointee = AssocValue;
                    using AdaptedPtr = std::add_pointer_t<AdaptedPointee>;
                    return reinterpret_cast<AdaptedPtr>(ptr);
                } else {
                    return ptr;
                }
            }
        } else {
            return ptr;
        }
    }
}
} // namespace detail

template<typename T>
class ArrayRepeatResult {
public:
    using value_type = T;

    ArrayRepeatResult(T value, size_t count) : values_(count, std::move(value)) {}

    ArrayRepeatResult& operator=(const std::vector<T>& rhs) {
        values_ = rhs;
        return *this;
    }

    ArrayRepeatResult& operator=(std::vector<T>&& rhs) {
        values_ = std::move(rhs);
        return *this;
    }

    // Accept any iterable (notably `rusty::Vec<T>` which the transpiler
    // can hand us when `vec![…; N]` lowers to `array_repeat(…)` but a
    // later assignment in the same scope produces a real `rusty::Vec`.
    // Surfaced by itertools' `test_checked_binomial` where
    // `let mut row = vec![Some(0); LIMIT+1]; … row = (1..=LIMIT).map(…)
    // .collect::<Vec<_>>();` becomes
    // `auto row = rusty::array_repeat(…); … row = rusty::Vec<…>::from_iter(…)`.
    //
    // Templated on the input container so we duck-type on `std::begin` /
    // `std::end` without naming `rusty::Vec` (which is module-only and
    // can't be referenced from a header — see the `rusty/vec.hpp`
    // comment). SFINAE-guards: skip when `Iterable` is one of the
    // already-overloaded `std::vector` types to avoid ambiguity.
    template<typename Iterable,
             typename = std::void_t<
                 decltype(std::begin(std::declval<Iterable&>())),
                 decltype(std::end(std::declval<Iterable&>()))
             >,
             typename = std::enable_if_t<
                 !std::is_same_v<std::remove_cvref_t<Iterable>, std::vector<T>>
             >>
    ArrayRepeatResult& operator=(Iterable&& rhs) {
        values_.assign(std::begin(rhs), std::end(rhs));
        return *this;
    }

    std::span<const T> as_slice() const noexcept {
        return std::span<const T>(values_.data(), values_.size());
    }

    std::span<T> as_mut_slice() noexcept {
        return std::span<T>(values_.data(), values_.size());
    }

    operator std::span<const T>() const noexcept {
        return as_slice();
    }

    operator std::span<T>() noexcept {
        return as_mut_slice();
    }

    auto begin() noexcept { return values_.begin(); }
    auto begin() const noexcept { return values_.begin(); }
    auto end() noexcept { return values_.end(); }
    auto end() const noexcept { return values_.end(); }

    T* data() noexcept { return values_.data(); }
    const T* data() const noexcept { return values_.data(); }

    T& operator[](size_t idx) noexcept { return values_[idx]; }
    const T& operator[](size_t idx) const noexcept { return values_[idx]; }

    T& at(size_t idx) { return values_.at(idx); }
    const T& at(size_t idx) const { return values_.at(idx); }

    size_t size() const noexcept { return values_.size(); }
    bool empty() const noexcept { return values_.empty(); }

    std::vector<T> Bytes() const { return values_; }
    std::vector<T> BorrowedBytes() const { return values_; }

    template<typename U = T>
    operator std::vector<U>() const {
        if constexpr (std::is_same_v<U, T>) {
            return values_;
        } else {
            std::vector<U> out;
            out.reserve(values_.size());
            for (const auto& item : values_) {
                out.push_back(static_cast<U>(item));
            }
            return out;
        }
    }

    template<typename U, size_t N>
    operator std::array<U, N>() const {
        std::array<U, N> out{};
        if constexpr (N > 0) {
            const U seed = values_.empty() ? U{} : static_cast<U>(values_.front());
            out.fill(seed);
        }
        return out;
    }

private:
    std::vector<T> values_;
};

/// Create a repeat sequence filled with `count` copies of `value`.
/// Equivalent to Rust's `[value; count]` array repeat syntax.
template<typename T>
ArrayRepeatResult<std::remove_cv_t<T>> array_repeat(T value, size_t count) {
    using Value = std::remove_cv_t<T>;
    return ArrayRepeatResult<Value>(static_cast<Value>(value), count);
}

template<size_t N, typename F>
auto array_from_fn(F&& func) {
    using mapped_type =
        std::decay_t<decltype(std::invoke(std::declval<F&>(), static_cast<size_t>(0)))>;
    auto mapper = std::forward<F>(func);
    return [&]<size_t... I>(std::index_sequence<I...>) {
        return std::array<mapped_type, N>{
            std::invoke(mapper, static_cast<size_t>(I))...};
    }(std::make_index_sequence<N>{});
}

template<typename Range>
void rotate_left(Range&& range, size_t mid) {
    auto&& view = range;
    auto first = std::begin(view);
    auto last = std::end(view);
    const auto len = static_cast<size_t>(std::distance(first, last));
    if (len == 0) {
        return;
    }
    mid %= len;
    std::rotate(first, std::next(first, static_cast<std::ptrdiff_t>(mid)), last);
}

template<typename Range>
void rotate_right(Range&& range, size_t k) {
    auto&& view = range;
    auto first = std::begin(view);
    auto last = std::end(view);
    const auto len = static_cast<size_t>(std::distance(first, last));
    if (len == 0) {
        return;
    }
    k %= len;
    if (k == 0) {
        return;
    }
    const auto pivot = len - k;
    std::rotate(first, std::next(first, static_cast<std::ptrdiff_t>(pivot)), last);
}

template<typename A, typename B>
constexpr auto min(A&& a, B&& b) {
    using C = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    if constexpr (std::is_floating_point_v<C>) {
        // Float-typed calls only ever come from f64::/f32::min (floats are
        // not Ord, so cmp::min never instantiates with them). Rust's min
        // ignores a one-sided NaN (IEEE minNum); std::min would return the
        // NaN whenever it is the FIRST argument.
        return std::fmin(static_cast<C>(std::forward<A>(a)), static_cast<C>(std::forward<B>(b)));
    } else {
        return std::min(static_cast<C>(std::forward<A>(a)), static_cast<C>(std::forward<B>(b)));
    }
}

template<typename A, typename B>
constexpr auto max(A&& a, B&& b) {
    using C = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    if constexpr (std::is_floating_point_v<C>) {
        // See min above — Rust f64::max is IEEE maxNum (NaN-ignoring).
        return std::fmax(static_cast<C>(std::forward<A>(a)), static_cast<C>(std::forward<B>(b)));
    } else {
        return std::max(static_cast<C>(std::forward<A>(a)), static_cast<C>(std::forward<B>(b)));
    }
}

template<typename T, typename Alloc>
Box<std::span<T>> into_boxed_slice(std::vector<T, Alloc> values) {
    const auto len = values.size();
    T* storage =
        (len == 0) ? nullptr : static_cast<T*>(::operator new(sizeof(T) * len));
    for (size_t i = 0; i < len; ++i) {
        new (storage + i) T(std::move(values[i]));
    }
    return Box<std::span<T>>::new_(std::span<T>(storage, len));
}

// Generic overload for any contiguous owning container with
// `.data()` + `.size()` and move-constructible elements — notably
// `rusty::Vec<T>` (which can't be named directly here because it's
// module-only). Used by serde_bytes' `bytes::Bytes` lowering which
// hands a `rusty::Vec<u8>` to `into_boxed_slice`. The `std::vector`
// overload above is strictly more specific for std::vector callers,
// so this generic form only catches the rusty::Vec / std::array
// shapes.
template<
    typename Container,
    typename = std::void_t<
        decltype(std::declval<Container&>().data()),
        decltype(std::declval<Container&>().size())
    >,
    typename = std::enable_if_t<
        !std::is_same_v<
            std::remove_cvref_t<Container>,
            std::vector<std::remove_cvref_t<decltype(*std::declval<Container&>().data())>>
        >
    >
>
auto into_boxed_slice(Container values) {
    using Elem = std::remove_cvref_t<decltype(*values.data())>;
    const auto len = values.size();
    Elem* storage =
        (len == 0) ? nullptr : static_cast<Elem*>(::operator new(sizeof(Elem) * len));
    auto* src = values.data();
    for (size_t i = 0; i < len; ++i) {
        new (storage + i) Elem(std::move(src[i]));
    }
    return Box<std::span<Elem>>::new_(std::span<Elem>(storage, len));
}

template<typename T>
Box<std::span<T>> into_boxed_slice(ArrayRepeatResult<T> values) {
    return into_boxed_slice(static_cast<std::vector<T>>(values));
}

namespace detail {
template<typename VecLike, typename Item>
void push_back_collect_item(VecLike& out, Item&& item) {
    using ItemRef = Item&&;
    using ItemValue = std::remove_cvref_t<ItemRef>;
    if constexpr (requires(VecLike& v, Item&& i) { v.push_back(std::forward<Item>(i)); }) {
        out.push_back(std::forward<Item>(item));
    } else if constexpr (requires(const ItemValue& value) { value.clone(); }) {
        out.push_back(item.clone());
    } else if constexpr (!std::is_const_v<std::remove_reference_t<ItemRef>>
                         && std::is_move_constructible_v<ItemValue>) {
        out.push_back(std::move(item));
    } else {
        static_assert(
            std::is_copy_constructible_v<ItemValue>,
            "cannot collect/filter_map non-copy, non-cloneable elements");
    }
}
}  // namespace detail

/// Collect any iterable range into std::vector<T>.
/// Used by transpiled Rust range `.collect()` calls.
template<typename Range>
auto collect_range(Range&& range_like) {
    if constexpr (requires(Range&& r) {
        std::begin(r);
        std::end(r);
    }) {
        using Elem = std::decay_t<decltype(*std::begin(range_like))>;
        std::vector<Elem> out;
        for (auto&& item : range_like) {
            detail::push_back_collect_item(out, std::forward<decltype(item)>(item));
        }
        return out;
    } else if constexpr (requires(Range&& r) { std::forward<Range>(r).next(); }) {
        // Consume option-like iterators through a forwarding reference instead of
        // by-value local materialization. This avoids creating an extra moved-from
        // iterator owner whose destructor can race ownership-forget bookkeeping.
        auto&& iter = range_like;
        using NextResult = decltype(iter.next());
        static_assert(
            requires(NextResult& next_item) {
                detail::option_has_value(next_item);
                detail::option_take_value(next_item);
            },
            "rusty::collect_range requires next() to return an Option/optional-like value");
        using Elem = std::decay_t<decltype(detail::option_take_value(
            std::declval<NextResult&>()))>;
        std::vector<Elem> out;
        while (true) {
            auto item = iter.next();
            if (!detail::option_has_value(item)) {
                break;
            }
            decltype(auto) value = detail::option_take_value(item);
            detail::push_back_collect_item(out, std::forward<decltype(value)>(value));
        }
        return out;
    } else if constexpr (requires(Range&& r) { std::forward<Range>(r).into_iter(); }) {
        return collect_range(std::forward<Range>(range_like).into_iter());
    } else if constexpr (requires(Range&& r) { r.iter(); }) {
        // Const-only-iterable containers expose a borrowing `.iter()` but no
        // const-callable `.into_iter()` (e.g. a `const Mapping&`, whose by-value
        // IntoIterator impl lowers to a non-const `into_iter()`). Rust's
        // `Vec::from_iter(&container)` selects the by-ref IntoIterator impl in
        // exactly this case; mirror that by collecting through `.iter()`.
        return collect_range(range_like.iter());
    } else {
        static_assert(
            detail::collect_range_dependent_false_v<Range>,
            "rusty::collect_range requires a range, into_iter(), iter(), or Option-like next()");
    }
}

template<typename T>
decltype(auto) as_ptr(const T& value) {
    if constexpr (requires { value.as_ptr(); }) {
        return detail::adapt_as_ptr_result(value, value.as_ptr());
    } else if constexpr (requires { const_cast<std::remove_cvref_t<T>&>(value).as_ptr(); }) {
        using RawPtr =
            decltype(const_cast<std::remove_cvref_t<T>&>(value).as_ptr());
        if constexpr (std::is_pointer_v<std::remove_reference_t<RawPtr>>
                      && std::is_const_v<
                          std::remove_pointer_t<std::remove_reference_t<RawPtr>>>) {
            return detail::adapt_as_ptr_result(
                value,
                const_cast<std::remove_cvref_t<T>&>(value).as_ptr());
        } else {
            return &value;
        }
    } else if constexpr (requires { value.data(); }) {
        return detail::adapt_as_ptr_result(value, value.data());
    } else if constexpr (requires { value.begin(); }) {
        if constexpr (std::is_pointer_v<std::remove_reference_t<decltype(value.begin())>>) {
            return detail::adapt_as_ptr_result(value, value.begin());
        } else {
            return &value;
        }
    } else {
        return &value;
    }
}

template<typename T>
decltype(auto) as_mut_ptr(T& value) {
    if constexpr (requires { value.as_mut_ptr(); }) {
        return detail::adapt_as_mut_ptr_result(value, value.as_mut_ptr());
    } else if constexpr (requires { value.data(); }) {
        return detail::adapt_as_mut_ptr_result(value, value.data());
    } else if constexpr (requires { value.begin(); }) {
        if constexpr (std::is_pointer_v<std::remove_reference_t<decltype(value.begin())>>) {
            return detail::adapt_as_mut_ptr_result(value, value.begin());
        } else {
            return &value;
        }
    } else {
        return &value;
    }
}

// Borrow helper for tuple-assertion style reference scaffolding.
// Prefer pointer-wrapper `.as_ptr()` surfaces when they expose element pointers,
// but keep string-like objects addressable as whole values.
template<typename T>
decltype(auto) as_ref_ptr(const T& value) {
    if constexpr (requires { value.as_ptr(); }) {
        using RawPtr = decltype(value.as_ptr());
        if constexpr (std::is_pointer_v<std::remove_reference_t<RawPtr>>) {
            using Pointee =
                std::remove_cv_t<std::remove_pointer_t<std::remove_reference_t<RawPtr>>>;
            // String-likes (char data, or u8 data with a c_str() — String/str
            // as_ptr() is *const u8 for Rust parity) keep object identity.
            constexpr bool string_like_data = std::is_same_v<Pointee, char>
                || (std::is_same_v<Pointee, uint8_t> && requires { value.c_str(); });
            if constexpr (!string_like_data) {
                return detail::adapt_as_ptr_result(value, value.as_ptr());
            } else {
                return &value;
            }
        } else {
            return &value;
        }
    } else if constexpr (requires { const_cast<std::remove_cvref_t<T>&>(value).as_ptr(); }) {
        using RawPtr =
            decltype(const_cast<std::remove_cvref_t<T>&>(value).as_ptr());
        if constexpr (std::is_pointer_v<std::remove_reference_t<RawPtr>>
                      && std::is_const_v<
                          std::remove_pointer_t<std::remove_reference_t<RawPtr>>>) {
            using Pointee =
                std::remove_cv_t<std::remove_pointer_t<std::remove_reference_t<RawPtr>>>;
            constexpr bool string_like_data2 = std::is_same_v<Pointee, char>
                || (std::is_same_v<Pointee, uint8_t> && requires { value.c_str(); });
            if constexpr (!string_like_data2) {
                return detail::adapt_as_ptr_result(
                    value,
                    const_cast<std::remove_cvref_t<T>&>(value).as_ptr());
            } else {
                return &value;
            }
        } else {
            return &value;
        }
    } else {
        return &value;
    }
}

// Lazy zip view — Rust's `a.zip(b)` semantics. Eager vector
// materialization hangs on unbounded sides (indexmap's
// `(start..).zip(end..)` shift loops zip two `range_from`s and are
// bounded only by the OUTER zip with the entries vector).
template<typename Left, typename Right>
struct zip_view {
    Left left;
    Right right;

    using left_iter = decltype(std::begin(std::declval<Left&>()));
    using left_sent = decltype(std::end(std::declval<Left&>()));
    using right_iter = decltype(std::begin(std::declval<Right&>()));
    using right_sent = decltype(std::end(std::declval<Right&>()));
    using LeftElem = std::decay_t<decltype(*std::begin(std::declval<Left&>()))>;
    using RightElem = std::decay_t<decltype(*std::begin(std::declval<Right&>()))>;
    using value_type = std::tuple<LeftElem, RightElem>;

    struct sentinel {};
    struct iterator {
        left_iter lit;
        left_sent lend;
        right_iter rit;
        right_sent rend;
        value_type operator*() const { return value_type(*lit, *rit); }
        iterator& operator++() {
            ++lit;
            ++rit;
            return *this;
        }
        bool operator==(sentinel) const { return !(lit != lend) || !(rit != rend); }
        bool operator!=(sentinel s) const { return !(*this == s); }
    };
    iterator begin() {
        return iterator{std::begin(left), std::end(left), std::begin(right), std::end(right)};
    }
    sentinel end() { return sentinel{}; }
};

template<typename Left, typename Right>
auto zip(Left&& left, Right&& right) {
    return zip_view<std::decay_t<Left>, std::decay_t<Right>>{
        std::forward<Left>(left), std::forward<Right>(right)};
}

/// Unified length helper for transpiled `.len()` calls.
/// Supports rusty types (`.len()`), STL/span (`.size()`), and native arrays.
inline size_t len(const char* cstr) {
    return cstr ? std::strlen(cstr) : 0;
}

inline size_t len(char* cstr) {
    return cstr ? std::strlen(cstr) : 0;
}

template<typename Container>
size_t len(const Container& container);

template<typename T>
size_t len(const rusty::Box<T>& container) {
    return rusty::len(*container);
}

template<typename Container>
size_t len(const Container& container) {
    if constexpr (requires { container.len(); }) {
        return static_cast<size_t>(container.len());
    } else if constexpr (requires { container.size(); }) {
        return static_cast<size_t>(container.size());
    } else if constexpr (requires { container.as_str(); }) {
        return rusty::len(container.as_str());
    } else if constexpr (requires { std::size(container); }) {
        return static_cast<size_t>(std::size(container));
    } else if constexpr (requires { container.size_hint(); }) {
        auto hint = container.size_hint();
        if constexpr (requires { std::get<1>(hint); }) {
            auto upper = std::get<1>(hint);
            if constexpr (requires {
                              detail::option_has_value(upper);
                              detail::option_take_value(upper);
                          }) {
                if (detail::option_has_value(upper)) {
                    return static_cast<size_t>(detail::option_take_value(upper));
                }
            } else if constexpr (std::is_integral_v<std::remove_cvref_t<decltype(upper)>>) {
                return static_cast<size_t>(upper);
            }
        }
        if constexpr (requires { std::get<0>(hint); }) {
            return static_cast<size_t>(std::get<0>(hint));
        } else {
            static_assert(
                detail::collect_range_dependent_false_v<Container>,
                "rusty::len requires tuple-like size_hint() bounds");
        }
    } else if constexpr (requires {
                             const_cast<std::remove_cvref_t<Container>&>(container).size_hint();
                         }) {
        auto hint = const_cast<std::remove_cvref_t<Container>&>(container).size_hint();
        if constexpr (requires { std::get<1>(hint); }) {
            auto upper = std::get<1>(hint);
            if constexpr (requires {
                              detail::option_has_value(upper);
                              detail::option_take_value(upper);
                          }) {
                if (detail::option_has_value(upper)) {
                    return static_cast<size_t>(detail::option_take_value(upper));
                }
            } else if constexpr (std::is_integral_v<std::remove_cvref_t<decltype(upper)>>) {
                return static_cast<size_t>(upper);
            }
        }
        if constexpr (requires { std::get<0>(hint); }) {
            return static_cast<size_t>(std::get<0>(hint));
        } else {
            static_assert(
                detail::collect_range_dependent_false_v<Container>,
                "rusty::len requires tuple-like size_hint() bounds");
        }
    } else if constexpr (requires { container.into_iter(); }) {
        return rusty::len(container.into_iter());
    } else if constexpr (requires {
                             const_cast<std::remove_cvref_t<Container>&>(container).into_iter();
                         }) {
        return rusty::len(const_cast<std::remove_cvref_t<Container>&>(container).into_iter());
    } else {
        static_assert(
            detail::collect_range_dependent_false_v<Container>,
            "rusty::len requires len(), size(), as_str(), std::size-compatible range, "
            "size_hint(), or into_iter()");
    }
}

template<typename T, typename U>
constexpr auto saturating_add(T lhs, U rhs) {
    /* Rust's rhs is always Self — compute in the RECEIVER type. The old
       common_type promotion made narrow saturations silently wrap
       (0u8.saturating_sub(1) came out 255 with an int literal rhs). */
    using R = T;
    static_assert(std::is_integral_v<R>, "rusty::saturating_add requires integral operands");
    const R a = static_cast<R>(lhs);
    const R b = static_cast<R>(rhs);
    if constexpr (std::is_unsigned_v<R>) {
        const R max = std::numeric_limits<R>::max();
        if (a > max - b) {
            return max;
        }
        return static_cast<R>(a + b);
    } else {
        const R max = std::numeric_limits<R>::max();
        const R min = std::numeric_limits<R>::min();
        if (b > 0 && a > max - b) {
            return max;
        }
        if (b < 0 && a < min - b) {
            return min;
        }
        return static_cast<R>(a + b);
    }
}

template<typename T, typename U>
constexpr auto saturating_sub(T lhs, U rhs) {
    /* Rust's rhs is always Self — compute in the RECEIVER type. The old
       common_type promotion made narrow saturations silently wrap
       (0u8.saturating_sub(1) came out 255 with an int literal rhs). */
    using R = T;
    static_assert(std::is_integral_v<R>, "rusty::saturating_sub requires integral operands");
    const R a = static_cast<R>(lhs);
    const R b = static_cast<R>(rhs);
    if constexpr (std::is_unsigned_v<R>) {
        if (a < b) {
            return static_cast<R>(0);
        }
        return static_cast<R>(a - b);
    } else {
        const R max = std::numeric_limits<R>::max();
        const R min = std::numeric_limits<R>::min();
        if (b > 0 && a < min + b) {
            return min;
        }
        if (b < 0 && a > max + b) {
            return max;
        }
        return static_cast<R>(a - b);
    }
}

template<typename T, typename U>
constexpr auto saturating_mul(T lhs, U rhs) {
    /* Rust's rhs is always Self — compute in the RECEIVER type. The old
       common_type promotion made narrow saturations silently wrap
       (0u8.saturating_sub(1) came out 255 with an int literal rhs). */
    using R = T;
    static_assert(std::is_integral_v<R>, "rusty::saturating_mul requires integral operands");
    const R a = static_cast<R>(lhs);
    const R b = static_cast<R>(rhs);
    if constexpr (std::is_unsigned_v<R>) {
        const R max = std::numeric_limits<R>::max();
        if (a != 0 && b > max / a) {
            return max;
        }
        return static_cast<R>(a * b);
    } else {
        if (a == 0 || b == 0) {
            return static_cast<R>(0);
        }
        const auto wide_a = static_cast<__int128>(a);
        const auto wide_b = static_cast<__int128>(b);
        const auto wide = wide_a * wide_b;
        const auto max = static_cast<__int128>(std::numeric_limits<R>::max());
        const auto min = static_cast<__int128>(std::numeric_limits<R>::min());
        if (wide > max) {
            return std::numeric_limits<R>::max();
        }
        if (wide < min) {
            return std::numeric_limits<R>::min();
        }
        return static_cast<R>(wide);
    }
}

/// Lazy filter_map view for transpiled iterator chains (`iter().filter_map(...)`).
/// The mapping closure is only evaluated while iterating the view.
template<typename Range, typename Func>
class filter_map_view {
    using base_iterator = decltype(std::begin(std::declval<Range&>()));
    using option_type = std::decay_t<decltype(std::declval<Func&>()(*std::declval<base_iterator&>()))>;
    using value_type = detail::option_value_t<option_type>;

public:
    template<typename R, typename F>
    filter_map_view(R&& range, F&& func)
        : range_(std::forward<R>(range)), func_(std::forward<F>(func)) {}

    class iterator {
    public:
        iterator(base_iterator current, base_iterator end, Func* func, bool at_end = false)
            : current_(current), end_(end), func_(func), at_end_(at_end) {
            if (!at_end_) {
                advance();
            }
        }

        const value_type& operator*() const { return *cached_; }

        iterator& operator++() {
            advance();
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return current_ != other.current_ || at_end_ != other.at_end_;
        }

    private:
        void advance() {
            cached_.reset();
            while (current_ != end_) {
                auto mapped = (*func_)(*current_);
                ++current_;
                if (detail::option_has_value(mapped)) {
                    cached_.emplace(detail::option_take_value(mapped));
                    at_end_ = false;
                    return;
                }
            }
            at_end_ = true;
        }

        base_iterator current_;
        base_iterator end_;
        Func* func_;
        std::optional<value_type> cached_;
        bool at_end_;
    };

    iterator begin() { return iterator(std::begin(range_), std::end(range_), &func_); }
    iterator end() { return iterator(std::end(range_), std::end(range_), &func_, true); }
    iterator begin() const { return iterator(std::begin(range_), std::end(range_), &func_); }
    iterator end() const { return iterator(std::end(range_), std::end(range_), &func_, true); }

private:
    Range range_;
    mutable Func func_;
};

template<typename Range, typename Func>
auto filter_map(Range&& range, Func&& func) {
    using StoredRange = detail::range_storage_t<Range&&>;
    using StoredFunc = std::decay_t<Func>;
    if constexpr (requires(StoredRange& r) { std::begin(r); std::end(r); }) {
        return filter_map_view<StoredRange, StoredFunc>(
            std::forward<Range>(range),
            std::forward<Func>(func)
        );
    } else if constexpr (requires(Range&& r) { std::forward<Range>(r).into_iter(); }) {
        return filter_map(std::forward<Range>(range).into_iter(), std::forward<Func>(func));
    } else {
        static_assert(
            detail::collect_range_dependent_false_v<Range>,
            "rusty::filter_map requires a range, into_iter(), or iterator-like next()"
        );
    }
}

/// Slice helpers used by transpiled Rust range-index expressions.
/// Examples:
/// - `x[..]` -> `slice_full(x)`
/// - `x[..n]` -> `slice_to(x, n)`
/// - `x[a..b]` -> `slice(x, a, b)`
namespace detail {

#ifndef RUSTY_DETAIL_STD_ARRAY_LIKE_TRAIT_DEFINED
#define RUSTY_DETAIL_STD_ARRAY_LIKE_TRAIT_DEFINED
template<typename T>
struct is_std_array_like : std::false_type {};

template<typename T, std::size_t N>
struct is_std_array_like<std::array<T, N>> : std::true_type {};

template<typename T>
inline constexpr bool is_std_array_like_v = is_std_array_like<T>::value;
#endif

// Preserve backing storage when `slice_full` is invoked with an rvalue
// `std::array{...}` temporary. Returning a plain `std::span` here would
// dangle immediately after the full expression.
template<typename T, std::size_t N>
struct owned_array_slice {
    std::array<T, N> storage;

    constexpr T* data() noexcept {
        return storage.data();
    }

    constexpr const T* data() const noexcept {
        return storage.data();
    }

    constexpr std::size_t size() const noexcept {
        return N;
    }

    constexpr T* begin() noexcept {
        return storage.data();
    }

    constexpr const T* begin() const noexcept {
        return storage.data();
    }

    constexpr T* end() noexcept {
        return storage.data() + N;
    }

    constexpr const T* end() const noexcept {
        return storage.data() + N;
    }

    constexpr std::span<T, N> as_mut_slice() noexcept {
        return std::span<T, N>(storage);
    }

    constexpr std::span<const T, N> as_slice() const noexcept {
        return std::span<const T, N>(storage);
    }

    constexpr operator std::span<T, N>() noexcept {
        return as_mut_slice();
    }

    constexpr operator std::span<const T, N>() const noexcept {
        return as_slice();
    }
};

// Preserve backing storage when `slice_full` is invoked with an rvalue
// non-array container (for example `rusty::Vec` or transpiled `SmallVec`).
// Returning a plain `std::span` from a temporary container would dangle
// immediately after the full expression.
template<typename Container>
struct owned_container_slice {
    using storage_type = std::remove_cv_t<std::remove_reference_t<Container>>;
    using elem_type = std::remove_reference_t<
        std::remove_reference_t<decltype(*rusty::as_mut_ptr(std::declval<storage_type&>()))>>;

    storage_type storage;

    elem_type* data() noexcept {
        return rusty::as_mut_ptr(storage);
    }

    const elem_type* data() const noexcept {
        return rusty::as_ptr(storage);
    }

    std::size_t size() const noexcept {
        return rusty::len(storage);
    }

    elem_type* begin() noexcept {
        return data();
    }

    const elem_type* begin() const noexcept {
        return data();
    }

    elem_type* end() noexcept {
        return data() + size();
    }

    const elem_type* end() const noexcept {
        return data() + size();
    }

    std::span<elem_type> as_mut_slice() noexcept {
        return std::span<elem_type>(data(), size());
    }

    std::span<const elem_type> as_slice() const noexcept {
        return std::span<const elem_type>(data(), size());
    }

    operator std::span<elem_type>() noexcept {
        return as_mut_slice();
    }

    operator std::span<const elem_type>() const noexcept {
        return as_slice();
    }
};

// Range-subscript routing (prefer a receiver's own emitted Index<Range…>
// subscript in the slice helpers). Default OFF: emitted deduced-I subscripts
// (`template<I> operator[](I)` from generic SliceIndex impls, e.g. smallvec)
// hard-error inside their bodies when probed with range types — a marker
// typedef emitted next to concrete range subscripts is needed before this
// can be on by default (see indexmap unlock notes).
#ifndef RUSTY_RANGE_SUBSCRIPT_ROUTING
#define RUSTY_RANGE_SUBSCRIPT_ROUTING 1
#endif

// A receiver whose storage is directly pointer-addressable (std containers,
// rusty::Vec/SmallVec ports). For these the span path is authoritative and
// range-subscript probing is SKIPPED: their emitted `operator[](I)`/
// `index_mut(I)` templates deduce I and hard-error on range args inside the
// body (outside the requires' immediate context).
// Emitted by the transpiler on structs whose Rust Index/IndexMut impls
// produced CONCRETE range-typed subscripts (`operator[](rusty::range…)`,
// `index_mut(rusty::range…)`). The helpers below probe range subscripts only
// behind this marker: a member-type check never instantiates method bodies,
// while probing `c[range…]` against a generic SliceIndex `operator[](I)`
// (deduced return) hard-errors inside the body — outside the requires'
// immediate context.
template<typename C>
inline constexpr bool has_range_index_marker_v =
    requires { typename std::remove_cvref_t<C>::__rusty_has_range_index; };

// Lazy probes: `if constexpr (marker && requires { c[r]; })` does NOT
// protect — clang computes the requires-expression's satisfaction while
// analyzing the whole condition, even when the left operand is false,
// instantiating deduced-return `operator[](I)` bodies (smallvec) and
// hard-erroring. A constrained partial specialization only evaluates its
// requires when the marker matched.
template<typename C, typename R, typename = void>
struct range_subscript_probe : std::false_type {};
template<typename C, typename R>
struct range_subscript_probe<C, R, std::enable_if_t<has_range_index_marker_v<C>>>
    : std::bool_constant<requires(C& c, R r) { c[r]; }> {};

template<typename C, typename R, typename = void>
struct range_index_mut_probe : std::false_type {};
template<typename C, typename R>
struct range_index_mut_probe<C, R, std::enable_if_t<has_range_index_marker_v<C>>>
    : std::bool_constant<requires(C& c, R r) { c.index_mut(r); }> {};

template<typename C>
inline constexpr bool raw_span_source_v =
    requires(C& c) { c.data(); } || requires(C& c) { c.as_ptr(); }
    // A member as_slice()/as_mut_slice() yielding a pointer-addressable view
    // (SmallVec → std::span). Crate types whose as_slice returns their OWN
    // slice type (indexmap's &Slice, no .data()) stay probeable.
    || requires(C& c) { c.as_slice().data(); }
    || requires(C& c) { c.as_mut_slice().data(); };

} // namespace detail

// Bound + range types — defined ahead of the slice helpers, which
// reference them in requires-clauses to prefer a receiver's own
// `Index<Range…>` subscript over span construction.
/// Iterable range [start, end) — equivalent to Rust's `start..end`.
template<typename T>
struct Bound_Unbounded {};

template<typename T>
struct Bound_Included {
    T _0;
};

template<typename T>
struct Bound_Excluded {
    T _0;
};

template<typename T>
using Bound = std::variant<Bound_Unbounded<T>, Bound_Included<T>, Bound_Excluded<T>>;

// Value-position factories for Rust's `Bound::…` variant constructors —
// `Bound` is an alias template over std::variant, so `Bound::Excluded(x)`
// cannot be spelled directly. `bound_unbounded` is a tag VALUE convertible
// to any Bound<T>, letting the consumer (tuple-bounds subscripts) pick T.
template<typename T>
Bound<std::decay_t<T>> bound_excluded(T&& value) {
    return Bound<std::decay_t<T>>(Bound_Excluded<std::decay_t<T>>{std::forward<T>(value)});
}
template<typename T>
Bound<std::decay_t<T>> bound_included(T&& value) {
    return Bound<std::decay_t<T>>(Bound_Included<std::decay_t<T>>{std::forward<T>(value)});
}
struct bound_unbounded_t {
    template<typename T>
    operator Bound<T>() const { return Bound<T>(Bound_Unbounded<T>{}); }
};
inline constexpr bound_unbounded_t bound_unbounded{};

template<typename T>
class range {
public:
    template<typename>
    friend class range;

    T start;

    constexpr range(T start_value, T end_value)
        : start(std::move(start_value)), end_(std::move(end_value)) {}

    template<typename U>
    constexpr range(const range<U>& other)
    requires std::is_convertible_v<U, T>
        : start(static_cast<T>(other.start)), end_(static_cast<T>(other.end_)) {}

    range into_iter() {
        return std::move(*this);
    }

    struct iterator {
        T current;
        T end;
        bool done;
        T operator*() const { return current; }
        iterator& operator++() {
            if (!done) {
                ++current;
                done = (current == end);
            }
            return *this;
        }
        bool operator!=(const iterator& other) const {
            (void)other;
            return !done;
        }
    };

    iterator begin() const { return {start, end_, start >= end_}; }
    iterator end() const { return {end_, end_, true}; }
    T& end_value() { return end_; }
    const T& end_value() const { return end_; }

    Bound<T> start_bound() const { return Bound<T>(Bound_Included<T>{start}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Excluded<T>{end_}); }

    bool contains(const T& value) const {
        return value >= start && value < end_;
    }

    /// Rust-style iterator protocol helper used by transpiled `.next()` calls.
    rusty::Option<T> next() {
        if (start >= end_) {
            return rusty::None;
        }
        T current = start;
        ++start;
        return rusty::Option<T>(current);
    }

    rusty::Option<T> next_back() {
        if (start >= end_) {
            return rusty::None;
        }
        --end_;
        return rusty::Option<T>(end_);
    }

    /// Rust-style iterator protocol helper used by transpiled `.count()` calls.
    size_t count() const {
        if (start >= end_) {
            return 0;
        }
        T current = start;
        size_t n = 0;
        while (current != end_) {
            ++current;
            ++n;
        }
        return n;
    }

    /// Rust-style iterator protocol helper used by transpiled `.size_hint()` calls.
    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        const size_t remaining = count();
        return std::make_tuple(remaining, rusty::Option<size_t>(remaining));
    }

    /// Rust-style `Range::step_by(n)` — yields start, start+n, start+2n, …
    /// strictly less than end. Iterable via range-for (begin/end) and the
    /// `.next()` protocol.
    struct StepBy {
        T current;
        T end_;
        T step;
        bool done;
        struct iterator {
            T current;
            T end;
            T step;
            bool done;
            T operator*() const { return current; }
            iterator& operator++() {
                T next = static_cast<T>(current + step);
                if (next >= end || next < current) {  // reached end (or wrapped)
                    done = true;
                }
                current = next;
                return *this;
            }
            bool operator!=(const iterator& other) const {
                (void)other;
                return !done;
            }
        };
        iterator begin() const { return {current, end_, step, current >= end_}; }
        iterator end() const { return {end_, end_, step, true}; }
        rusty::Option<T> next() {
            if (done || current >= end_) {
                done = true;
                return rusty::None;
            }
            T value = current;
            T n = static_cast<T>(current + step);
            if (n >= end_ || n < current) {
                done = true;
            }
            current = n;
            return rusty::Option<T>(value);
        }
    };
    StepBy step_by(T step) const { return StepBy{start, end_, step, start >= end_}; }

private:
    T end_;
};

template<typename A, typename B>
range(A, B) -> range<std::common_type_t<A, B>>;

/// Inclusive range [start, end] — equivalent to Rust's `start..=end`.
template<typename T>
class range_inclusive {
public:
    template<typename>
    friend class range_inclusive;

    T start;

    constexpr range_inclusive(T start_value, T end_value)
        : start(std::move(start_value)), end_(std::move(end_value)) {}

    template<typename U>
    constexpr range_inclusive(const range_inclusive<U>& other)
    requires std::is_convertible_v<U, T>
        : start(static_cast<T>(other.start)),
          end_(static_cast<T>(other.end_)),
          done_(other.done_) {}

    range_inclusive into_iter() {
        return std::move(*this);
    }

    struct iterator {
        T current;
        T end;
        bool done;
        T operator*() const { return current; }
        iterator& operator++() { if (current == end) done = true; else ++current; return *this; }
        bool operator!=(const iterator& other) const {
            (void)other;
            return !done;
        }
    };

    iterator begin() const { return {start, end_, start > end_}; }
    iterator end() const { return {end_, end_, true}; }
    T& end_value() { return end_; }
    const T& end_value() const { return end_; }

    Bound<T> start_bound() const { return Bound<T>(Bound_Included<T>{start}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Included<T>{end_}); }

    bool contains(const T& value) const {
        return value >= start && value <= end_;
    }

    /// Rust-style iterator protocol helper used by transpiled `.next()` calls.
    rusty::Option<T> next() {
        if (done_ || start > end_) {
            return rusty::None;
        }
        T current = start;
        if (start == end_) {
            done_ = true;
        } else {
            ++start;
        }
        return rusty::Option<T>(current);
    }

    rusty::Option<T> next_back() {
        if (done_ || start > end_) {
            return rusty::None;
        }
        T current = end_;
        if (start == end_) {
            done_ = true;
        } else {
            --end_;
        }
        return rusty::Option<T>(current);
    }

    /// Rust-style iterator protocol helper used by transpiled `.count()` calls.
    size_t count() const {
        if (done_ || start > end_) {
            return 0;
        }
        T current = start;
        size_t n = 0;
        while (true) {
            ++n;
            if (current == end_) {
                break;
            }
            ++current;
        }
        return n;
    }

    /// Rust-style iterator protocol helper used by transpiled `.size_hint()` calls.
    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        const size_t remaining = count();
        return std::make_tuple(remaining, rusty::Option<size_t>(remaining));
    }

private:
    T end_;
    bool done_ = false;
};

template<typename A, typename B>
range_inclusive(A, B) -> range_inclusive<std::common_type_t<A, B>>;

template<typename T>
auto get(std::string_view container, const range<T>& idx) {
    const size_t start_index = detail::checked_index(idx.start);
    const size_t end_index = detail::checked_index(idx.end_value());
    using Opt = Option<std::string_view>;
    if (start_index > end_index || end_index > container.size()
        || !detail::str_is_char_boundary(container, start_index)
        || !detail::str_is_char_boundary(container, end_index)) {
        return Opt(None);
    }
    return Opt(container.substr(start_index, end_index - start_index));
}

template<typename T>
auto get(std::string_view container, const range_inclusive<T>& idx) {
    const size_t start_index = detail::checked_index(idx.start);
    const size_t end_index = detail::checked_index(idx.end_value());
    using Opt = Option<std::string_view>;
    if (start_index > end_index || end_index >= container.size()
        || !detail::str_is_char_boundary(container, start_index)
        || !detail::str_is_char_boundary(container, end_index + 1)) {
        return Opt(None);
    }
    return Opt(container.substr(start_index, end_index - start_index + 1));
}

/// Open range from start — equivalent to Rust's `start..`.
template<typename T>
struct range_from {
    T start;

    range_from into_iter() {
        return std::move(*this);
    }

    Bound<T> start_bound() const { return Bound<T>(Bound_Included<T>{start}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }

    bool contains(const T& value) const {
        return value >= start;
    }

    /// Rust-style iterator protocol helper used by transpiled `.next()` calls.
    rusty::Option<T> next() {
        T current = start;
        ++start;
        return rusty::Option<T>(current);
    }

    /// C++ iteration surface: an unbounded counting iterator. Consumers
    /// (rusty::zip, range-for) terminate via the OTHER zipped side or an
    /// explicit break — matching Rust's lazy `start..` semantics.
    struct unbounded_iterator {
        T cur;
        T operator*() const { return cur; }
        unbounded_iterator& operator++() {
            ++cur;
            return *this;
        }
        bool operator==(std::unreachable_sentinel_t) const { return false; }
        bool operator!=(std::unreachable_sentinel_t) const { return true; }
    };
    unbounded_iterator begin() const { return unbounded_iterator{start}; }
    std::unreachable_sentinel_t end() const { return {}; }

    /// Rust-style iterator protocol helper used by transpiled `.count()` calls.
    /// `start..` is unbounded, so this mirrors an effectively-infinite count.
    size_t count() const {
        return std::numeric_limits<size_t>::max();
    }

    /// Rust-style iterator protocol helper used by transpiled `.size_hint()` calls.
    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        return std::make_tuple(
            std::numeric_limits<size_t>::max(),
            rusty::Option<size_t>(rusty::None)
        );
    }
};

/// Range to end — equivalent to Rust's `..end`.
template<typename T>
struct range_to {
    T end;

    Bound<T> start_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Excluded<T>{end}); }

    bool contains(const T& value) const {
        return value < end;
    }
};

/// Full range — equivalent to Rust's `..`.
struct range_full {
    template<typename T = size_t>
    Bound<T> start_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
    template<typename T = size_t>
    Bound<T> end_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }

    template<typename T>
    bool contains(const T&) const {
        return true;
    }
};

/// Range to inclusive — equivalent to Rust's `..=end`.
template<typename T>
struct range_to_inclusive {
    T end;

    Bound<T> start_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Included<T>{end}); }

    bool contains(const T& value) const {
        return value <= end;
    }
};

namespace detail {
// Rust RangeBounds shapes: the range structs and (Bound, Bound) pairs.
// Emitted Q-keyed lookup methods (`Q: Equivalent<K>`) constrain with
// `!range_bounds_like<Q>` so a range subscript never lands on the greedy
// key template — mirroring Rust, where range tuples don't implement
// Equivalent and overload selection happens through distinct impls.
template<typename T>
struct is_bound_like : std::false_type {};
template<typename T>
struct is_bound_like<
    std::variant<Bound_Unbounded<T>, Bound_Included<T>, Bound_Excluded<T>>>
    : std::true_type {};
template<>
struct is_bound_like<bound_unbounded_t> : std::true_type {};
template<typename T>
inline constexpr bool is_bound_like_v = is_bound_like<std::remove_cvref_t<T>>::value;

template<typename T>
struct is_range_bounds_like : std::false_type {};
template<typename A, typename B>
struct is_range_bounds_like<std::tuple<A, B>>
    : std::bool_constant<is_bound_like_v<A> && is_bound_like_v<B>> {};
template<typename A, typename B>
struct is_range_bounds_like<std::pair<A, B>>
    : std::bool_constant<is_bound_like_v<A> && is_bound_like_v<B>> {};
template<typename T>
struct is_range_bounds_like<range<T>> : std::true_type {};
template<typename T>
struct is_range_bounds_like<range_inclusive<T>> : std::true_type {};
template<typename T>
struct is_range_bounds_like<range_from<T>> : std::true_type {};
template<typename T>
struct is_range_bounds_like<range_to<T>> : std::true_type {};
template<typename T>
struct is_range_bounds_like<range_to_inclusive<T>> : std::true_type {};
template<>
struct is_range_bounds_like<range_full> : std::true_type {};
template<typename T>
inline constexpr bool is_range_bounds_like_v =
    is_range_bounds_like<std::remove_cvref_t<T>>::value;

template<typename T>
concept range_bounds_like = is_range_bounds_like_v<T>;

// Just the `(Bound, Bound)` pair shapes — the RangeBounds form with no
// dedicated range struct, so subscripts decode the pair at runtime.
template<typename T>
struct is_bound_pair_like : std::false_type {};
template<typename A, typename B>
struct is_bound_pair_like<std::tuple<A, B>>
    : std::bool_constant<is_bound_like_v<A> && is_bound_like_v<B>> {};
template<typename A, typename B>
struct is_bound_pair_like<std::pair<A, B>>
    : std::bool_constant<is_bound_like_v<A> && is_bound_like_v<B>> {};
template<typename T>
inline constexpr bool is_bound_pair_like_v =
    is_bound_pair_like<std::remove_cvref_t<T>>::value;

// Decode one slot of a `(Bound, Bound)` pair to a concrete element index.
// Start: Unbounded→0, Included(i)→i, Excluded(i)→i+1. A slot is either the
// Bound<T> variant or the type-erased `bound_unbounded_t` factory tag.
template<typename Bnd>
size_t bound_pair_start(const Bnd& b) {
    if constexpr (std::is_same_v<std::remove_cvref_t<Bnd>, bound_unbounded_t>) {
        return 0;
    } else {
        switch (b.index()) {
            case 1: return static_cast<size_t>(std::get<1>(b)._0);
            case 2: return static_cast<size_t>(std::get<2>(b)._0) + 1;
            default: return 0;
        }
    }
}
// End, given the container length: Unbounded→len, Included(i)→i+1, Excluded(i)→i.
template<typename Bnd>
size_t bound_pair_end(const Bnd& b, size_t len) {
    if constexpr (std::is_same_v<std::remove_cvref_t<Bnd>, bound_unbounded_t>) {
        return len;
    } else {
        switch (b.index()) {
            case 1: return static_cast<size_t>(std::get<1>(b)._0) + 1;
            case 2: return static_cast<size_t>(std::get<2>(b)._0);
            default: return len;
        }
    }
}

// The C++ mirror of Rust's `Q: Equivalent<K>` bound on keyed lookups:
// range shapes never implement Equivalent, so constraining the greedy
// Q-key template keeps range subscripts on the dedicated range impls.
template<typename Q>
concept equivalence_key_like = !is_range_bounds_like_v<Q>;
} // namespace detail


template<typename T, std::size_t N>
auto slice_full(std::array<T, N>&& container) {
    return detail::owned_array_slice<T, N>{std::move(container)};
}

template<typename Container>
requires (
    !std::is_lvalue_reference_v<Container&&> &&
    !detail::is_std_array_like_v<std::remove_cv_t<std::remove_reference_t<Container>>>)
auto slice_full(Container&& container) {
    if constexpr (std::is_const_v<std::remove_reference_t<Container>>) {
        // A CONST rvalue (std::move of a `ref`-pattern binding) cannot be
        // moved into owned storage, and copying an owning container may be
        // impossible (move-only elements) or a hidden deep clone. Emissions
        // only produce this shape for argument temporaries, which outlive
        // the full expression — a view is safe.
        return rusty::as_slice(container);
    } else {
        return detail::owned_container_slice<
            std::remove_cv_t<std::remove_reference_t<Container>>>{
            std::forward<Container>(container)};
    }
}

template<typename Container>
decltype(auto) slice_full(Container& container) {
    using Base = std::remove_cv_t<std::remove_reference_t<Container>>;
    // A receiver with its own full-range subscript (a transpiled
    // `Index<RangeFull>`/`IndexMut<RangeFull>` impl, e.g. indexmap) defines
    // Rust's `&c[..]` — route through it so reference identity is preserved.
    // Transpiled `IndexMut` lowers to an `index_mut` METHOD (operator[]
    // cannot overload on receiver constness alone), so mutable receivers
    // try that first.
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<Container, range_full>::value) {
        return container.index_mut(range_full{});
    } else if constexpr (detail::range_subscript_probe<Container, range_full>::value) {
        return container[range_full{}];
    } else
#endif
    if constexpr (requires { container.as_mut_slice(); }) {
        using Slice = std::remove_cv_t<std::remove_reference_t<decltype(container.as_mut_slice())>>;
        if constexpr (std::is_same_v<Slice, Base>) {
            using Elem = std::remove_reference_t<decltype(*rusty::as_mut_ptr(container))>;
            return std::span<Elem>(rusty::as_mut_ptr(container), rusty::len(container));
        } else {
            return container.as_mut_slice();
        }
    } else if constexpr (requires { container.as_mut(); }) {
        using Slice = std::remove_cv_t<std::remove_reference_t<decltype(container.as_mut())>>;
        if constexpr (std::is_same_v<Slice, Base>) {
            using Elem = std::remove_reference_t<decltype(*rusty::as_mut_ptr(container))>;
            return std::span<Elem>(rusty::as_mut_ptr(container), rusty::len(container));
        } else {
            return container.as_mut();
        }
    } else if constexpr (requires { container.as_slice(); }) {
        using Slice = std::remove_cv_t<std::remove_reference_t<decltype(container.as_slice())>>;
        if constexpr (std::is_same_v<Slice, Base>) {
            using Elem = std::remove_reference_t<decltype(*rusty::as_mut_ptr(container))>;
            return std::span<Elem>(rusty::as_mut_ptr(container), rusty::len(container));
        } else {
            return container.as_slice();
        }
    } else {
        using Elem = std::remove_reference_t<decltype(*rusty::as_mut_ptr(container))>;
        return std::span<Elem>(rusty::as_mut_ptr(container), rusty::len(container));
    }
}

template<typename Container>
decltype(auto) slice_full(const Container& container) {
    using Base = std::remove_cv_t<std::remove_reference_t<Container>>;
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_subscript_probe<const Base, range_full>::value) {
        return container[range_full{}];
    } else
#endif
    if constexpr (requires { container.as_slice(); }) {
        using Slice = std::remove_cv_t<std::remove_reference_t<decltype(container.as_slice())>>;
        if constexpr (std::is_same_v<Slice, Base>) {
            using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(container))>;
            return std::span<const Elem>(rusty::as_ptr(container), rusty::len(container));
        } else {
            return container.as_slice();
        }
    } else if constexpr (requires { container.as_ref(); }) {
        using Slice = std::remove_cv_t<std::remove_reference_t<decltype(container.as_ref())>>;
        if constexpr (std::is_same_v<Slice, Base>) {
            using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(container))>;
            return std::span<const Elem>(rusty::as_ptr(container), rusty::len(container));
        } else {
            return container.as_ref();
        }
    } else {
        using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(container))>;
        return std::span<const Elem>(rusty::as_ptr(container), rusty::len(container));
    }
}

template<typename T>
decltype(auto) slice_full(rusty::Box<T>& container) {
    return slice_full(*container);
}

template<typename T>
decltype(auto) slice_full(const rusty::Box<T>& container) {
    return slice_full(*container);
}

// Explicit helper surface for Rust-style `.as_slice()` lowering.
// Keeps const-view semantics even for mutable lvalue receivers and supports
// temporary receivers through forwarding-reference binding.
template<typename Container>
decltype(auto) as_slice(Container&& container) {
    // Rust's `Iter::as_slice` — the iterator's REMAINING range. slice_iter
    // iterators expose raw_cur/raw_end exactly for this view; the generic
    // fallthrough would wrap the iterator object itself.
    if constexpr (requires {
                      container.raw_cur();
                      container.raw_end();
                  }) {
        return std::span(
            container.raw_cur(),
            static_cast<size_t>(container.raw_end() - container.raw_cur()));
    } else if constexpr (std::is_rvalue_reference_v<Container&&>) {
        return slice_full(std::forward<Container>(container));
    } else {
        using Base = std::remove_reference_t<Container>;
        return slice_full(static_cast<const Base&>(container));
    }
}

// Explicit helper surface for Rust-style `.as_mut_slice()` lowering.
// Preserves mutable-view behavior for containers exposing mutable slices and
// falls back to mutable span construction where applicable.
template<typename Container>
decltype(auto) as_mut_slice(Container&& container) {
    return slice_full(std::forward<Container>(container));
}

// Normalize arbitrary slice-like containers into a byte view.
// For non-u8 element containers this keeps a thread-local converted buffer.
namespace detail {
template<typename Container>
concept as_u8_slice_compatible =
    std::is_pointer_v<std::remove_reference_t<Container>>
    || requires { std::variant_size<std::remove_cvref_t<Container>>::value; }
    || requires(Container&& container) { std::forward<Container>(container)._0; }
    || requires(Container&& container) { std::forward<Container>(container).as_slice(); }
    || requires(Container&& container) { std::forward<Container>(container).as_ref(); }
    || requires(Container&& container) { std::data(container); std::size(container); }
    || requires(Container&& container) {
        std::forward<Container>(container).len();
        std::forward<Container>(container).as_ptr();
    };
}

template<typename Container>
    requires detail::as_u8_slice_compatible<Container>
std::span<const uint8_t> as_u8_slice(Container&& container) {
    if constexpr (std::is_pointer_v<std::remove_reference_t<Container>>) {
        return as_u8_slice(*container);
    } else if constexpr (requires { std::variant_size<std::remove_cvref_t<Container>>::value; }) {
        return std::visit(
            [](auto&& value) -> std::span<const uint8_t> {
                return rusty::as_u8_slice(std::forward<decltype(value)>(value));
            },
            std::forward<Container>(container));
    } else if constexpr (requires { std::forward<Container>(container)._0; }) {
        return rusty::as_u8_slice(std::forward<Container>(container)._0);
    } else {
        auto slice = rusty::as_slice(std::forward<Container>(container));
        using Elem = std::remove_cv_t<std::remove_reference_t<decltype(*slice.data())>>;
        if constexpr (std::is_same_v<Elem, uint8_t>) {
            if constexpr (
                std::is_rvalue_reference_v<Container&&>
                && !std::is_pointer_v<std::remove_reference_t<Container>>) {
                thread_local std::vector<uint8_t> _rusty_u8_slice_tmp_owned;
                _rusty_u8_slice_tmp_owned.assign(slice.begin(), slice.end());
                return std::span<const uint8_t>(
                    _rusty_u8_slice_tmp_owned.data(), _rusty_u8_slice_tmp_owned.size());
            } else {
                return std::span<const uint8_t>(slice.data(), slice.size());
            }
        } else {
            thread_local std::vector<uint8_t> _rusty_u8_slice_tmp;
            _rusty_u8_slice_tmp.clear();
            _rusty_u8_slice_tmp.reserve(slice.size());
            for (const auto& item : slice) {
                _rusty_u8_slice_tmp.push_back(static_cast<uint8_t>(item));
            }
            return std::span<const uint8_t>(
                _rusty_u8_slice_tmp.data(), _rusty_u8_slice_tmp.size());
        }
    }
}

// Convert tuple-size/index-addressable array-like inputs to std::array<uint8_t, N>.
template<typename ArrayLike>
auto as_u8_array(ArrayLike&& value) {
    if constexpr (std::is_pointer_v<std::remove_reference_t<ArrayLike>>) {
        return as_u8_array(*value);
    } else {
        using Raw = std::remove_cv_t<std::remove_reference_t<ArrayLike>>;
        constexpr std::size_t N = std::tuple_size_v<Raw>;
        std::array<uint8_t, N> out{};
        for (std::size_t i = 0; i < N; ++i) {
            out[i] = static_cast<uint8_t>(value[i]);
        }
        return out;
    }
}

// Explicit helper surface for Rust-style `.get(index)` lowering on
// slice-like/Vec-like containers. Returns Option<&T> in const-view form.
template<typename Container, typename Index>
auto get(const Container& container, Index idx) {
    const auto span = slice_full(container);
    const size_t index = detail::checked_index(idx);
    using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(span))>;
    using Opt = Option<Elem&>;
    if constexpr (requires { container.get(index); }) {
        using GetResult = decltype(container.get(index));
        if constexpr (std::is_convertible_v<GetResult, Opt>) {
            return Opt(container.get(index));
        }
    }
    if (index < rusty::len(span)) {
        return Opt(*(rusty::as_ptr(span) + index));
    }
    return Opt(None);
}

// Rust `slice.get(range)` — the subslice when in bounds, None otherwise.
template<typename Container, typename T>
auto get(const Container& container, const range<T>& idx) {
    auto span = slice_full(container);
    using Sub = decltype(span.subspan(size_t{0}, size_t{0}));
    using Opt = Option<Sub>;
    const size_t start = static_cast<size_t>(idx.start);
    const size_t end = static_cast<size_t>(idx.end_value());
    if (start > end || end > span.size()) {
        return Opt(None);
    }
    return Opt(span.subspan(start, end - start));
}

template<typename T, std::size_t E, typename U>
auto get_mut(std::span<T, E> container, const range<U>& idx) {
    using Opt = Option<std::span<T>>;
    const size_t start = static_cast<size_t>(idx.start);
    const size_t end = static_cast<size_t>(idx.end_value());
    if (start > end || end > container.size()) {
        return Opt(None);
    }
    return Opt(container.subspan(start, end - start));
}

// Mutable counterpart for Rust-style `.get_mut(index)` lowering.
template<typename Container, typename Index>
auto get_mut(Container& container, Index idx) {
    auto span = slice_full(container);
    const size_t index = detail::checked_index(idx);
    using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(span))>;
    using Opt = Option<Elem&>;
    if constexpr (requires { container.get_mut(index); }) {
        using GetMutResult = decltype(container.get_mut(index));
        if constexpr (std::is_convertible_v<GetMutResult, Opt>) {
            return Opt(container.get_mut(index));
        }
    }
    if constexpr (requires { container.get(index); }) {
        using GetResult = decltype(container.get(index));
        if constexpr (std::is_convertible_v<GetResult, Opt>) {
            return Opt(container.get(index));
        }
    }
    if (index < rusty::len(span)) {
        return Opt(*(rusty::as_ptr(span) + index));
    }
    return Opt(None);
}

template<typename Container>
auto first(const Container& container) {
    return get(container, size_t{0});
}

template<typename Container>
auto first_mut(Container& container) {
    return get_mut(container, size_t{0});
}

template<typename Container>
auto last(const Container& container) {
    const auto span = slice_full(container);
    using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(span))>;
    using Opt = Option<Elem&>;
    if (rusty::len(span) == 0) {
        return Opt(None);
    }
    const size_t last_index = rusty::len(span) - 1;
    return Opt(*(rusty::as_ptr(span) + last_index));
}

template<typename Container>
auto last_mut(Container& container) {
    auto span = slice_full(container);
    using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(span))>;
    using Opt = Option<Elem&>;
    if (rusty::len(span) == 0) {
        return Opt(None);
    }
    const size_t last_index = rusty::len(span) - 1;
    return Opt(*(rusty::as_ptr(span) + last_index));
}

// std::span accessor counterparts — spans are borrowed views, so rvalue
// spans (`self.as_entries_mut()` materializes a prvalue) are safe to take
// by value; the generic `Container&` overloads reject rvalues. More
// specialized, so lvalue spans also prefer them.
template<typename T, std::size_t E, typename Index>
auto get_mut(std::span<T, E> container, Index idx) {
    const size_t index = detail::checked_index(idx);
    using Opt = Option<T&>;
    if (index < container.size()) {
        return Opt(container[index]);
    }
    return Opt(None);
}

template<typename T, std::size_t E>
auto first_mut(std::span<T, E> container) {
    return get_mut(container, size_t{0});
}

template<typename T, std::size_t E>
auto last_mut(std::span<T, E> container) {
    using Opt = Option<T&>;
    if (container.empty()) {
        return Opt(None);
    }
    return Opt(container[container.size() - 1]);
}

// Collect a slice-like container into std::vector by value-cloning elements.
// Used by transpiled Rust `.to_vec()` lowering for slice/array/ArrayVec shapes.
//
// Returns `std::vector<Elem>` because `rusty::Vec` is module-only (see the
// comment at the top of `rusty/vec.hpp`) — its name `rusty::Vec` is
// non-dependent in template-body lookup, so it can't be referenced from
// this header even via `if constexpr (requires { … })`. Two-phase lookup
// rejects `rusty::Vec` at template-definition time. Target-driven sites
// like `Content_ByteBuf{ to_vec(value) }` need a different solution —
// the transpiler should route those through `rusty::Vec<Elem>::from_iter`
// at the emit layer where `rusty::Vec` IS in scope (because the cppm has
// already `import rusty;`-ed). See `try_emit_to_vec_method_call` plus
// target-type plumbing.
template<typename Container>
auto to_vec(const Container& container) {
    auto span = slice_full(container);
    using Elem = std::remove_cv_t<std::remove_reference_t<decltype(*span.data())>>;
    std::vector<Elem> out;
    for (const auto& item : span) {
        if constexpr (requires(const Elem& e) { e.clone(); }) {
            out.push_back(item.clone());
        } else {
            out.push_back(item);
        }
    }
    return out;
}

namespace detail {
template<typename T>
struct is_std_array_type : std::false_type {};

template<typename Elem, std::size_t N>
struct is_std_array_type<std::array<Elem, N>> : std::true_type {};

template<typename T>
inline constexpr bool is_std_array_type_v =
    is_std_array_type<std::remove_cv_t<std::remove_reference_t<T>>>::value;

template<typename T>
struct std_array_meta;

template<typename Elem, std::size_t N>
struct std_array_meta<std::array<Elem, N>> {
    using elem_type = Elem;
    static constexpr std::size_t len = N;
};
} // namespace detail

// Generic array TryFrom helper used by transpiled `try_into()` lowering:
// - `std::span<T> -> Result<std::array<T, N>, ()>`
// - `std::span<T> -> Result<const std::array<T, N>&, ()>` (borrowed view)
template<typename Target, typename Source>
requires(detail::is_std_array_type_v<std::remove_reference_t<Target>>)
auto try_from(Source&& source) {
    using RawTarget = std::remove_cv_t<std::remove_reference_t<Target>>;
    using Meta = detail::std_array_meta<RawTarget>;
    using Elem = typename Meta::elem_type;
    constexpr std::size_t N = Meta::len;
    using ResultTy = rusty::Result<Target, std::tuple<>>;

    auto span = rusty::slice_full(source);
    if (rusty::len(span) != N) {
        return ResultTy::Err(std::tuple<>{});
    }

    using SpanElem = std::remove_cv_t<std::remove_reference_t<decltype(*span.data())>>;

    if constexpr (std::is_reference_v<Target>) {
        if constexpr (!std::is_lvalue_reference_v<Source&&>) {
            return ResultTy::Err(std::tuple<>{});
        }
        if constexpr (!std::is_same_v<SpanElem, Elem>) {
            return ResultTy::Err(std::tuple<>{});
        } else if constexpr (std::is_const_v<std::remove_reference_t<Target>>) {
            if constexpr (N == 0) {
                static const RawTarget empty{};
                return ResultTy::Ok(empty);
            } else {
                const auto* ptr = reinterpret_cast<const RawTarget*>(span.data());
                return ResultTy::Ok(*ptr);
            }
        } else {
            if constexpr (std::is_const_v<std::remove_reference_t<decltype(*span.data())>>) {
                return ResultTy::Err(std::tuple<>{});
            } else if constexpr (N == 0) {
                static RawTarget empty{};
                return ResultTy::Ok(empty);
            } else {
                auto* ptr = reinterpret_cast<RawTarget*>(span.data());
                return ResultTy::Ok(*ptr);
            }
        }
    } else {
        RawTarget out{};
        for (std::size_t i = 0; i < N; ++i) {
            if constexpr (std::is_same_v<SpanElem, Elem> || std::is_convertible_v<SpanElem, Elem>) {
                out[i] = static_cast<Elem>(span[i]);
            } else {
                return ResultTy::Err(std::tuple<>{});
            }
        }
        return ResultTy::Ok(std::move(out));
    }
}

// Split a slice-like view/container into `(prefix, suffix)` at `mid`.
// Mirrors Rust `[T]::split_at` semantics with bounds checking.
template<typename Elem, std::size_t Extent, typename Mid>
auto split_at(std::span<Elem, Extent> span, Mid mid) {
    const size_t mid_index = detail::checked_index(mid);
    detail::validate_slice_bounds(span, 0, mid_index);
    return std::make_tuple(span.first(mid_index), span.subspan(mid_index));
}

// Rust `[T]::split_at_mut` — same split, mutable halves. The span element
// type already carries mutability; the separate name exists for the
// container overload and call-site parity.
template<typename Elem, std::size_t Extent, typename Mid>
auto split_at_mut(std::span<Elem, Extent> span, Mid mid) {
    const size_t mid_index = detail::checked_index(mid);
    detail::validate_slice_bounds(span, 0, mid_index);
    return std::make_tuple(span.first(mid_index), span.subspan(mid_index));
}

template<typename Container, typename Mid>
requires (!std::is_same_v<std::remove_cvref_t<Container>, std::string_view>)
auto split_at_mut(Container& container, Mid mid) {
    // A container spelling its own split_at_mut (port types) wins.
    if constexpr (requires { container.split_at_mut(static_cast<size_t>(mid)); }) {
        return container.split_at_mut(static_cast<size_t>(mid));
    } else {
        return split_at_mut(slice_full(container), std::forward<Mid>(mid));
    }
}

// Split a span into `(first, rest)` where `first` is a borrowed element.
// Mirrors Rust `[T]::split_first`.
template<typename Elem, std::size_t Extent>
auto split_first(std::span<Elem, Extent> span) {
    using Tail = std::span<Elem, std::dynamic_extent>;
    using Pair = std::tuple<Elem&, Tail>;
    using Opt = Option<Pair>;
    if (span.empty()) {
        return Opt(None);
    }
    return Opt(Pair{span[0], Tail(span.subspan(1))});
}

template<typename Container>
auto split_first(Container& container) {
    return split_first(slice_full(container));
}

template<typename Elem, std::size_t Extent = std::dynamic_extent>
class ChunksExact {
public:
    using Span = std::span<Elem, std::dynamic_extent>;

    class iterator {
    public:
        using value_type = Span;
        using difference_type = std::ptrdiff_t;

        iterator(Span span, size_t chunk_size, size_t index)
            : span_(span), chunk_size_(chunk_size), index_(index) {}

        value_type operator*() const {
            return span_.subspan(index_ * chunk_size_, chunk_size_);
        }

        iterator& operator++() {
            ++index_;
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return index_ != other.index_;
        }

    private:
        Span span_;
        size_t chunk_size_;
        size_t index_;
    };

    ChunksExact(std::span<Elem, Extent> span, size_t chunk_size)
        : span_(span.data(), span.size()), chunk_size_(chunk_size == 0 ? 1 : chunk_size) {}

    iterator begin() const { return iterator(span_, chunk_size_, 0); }
    iterator end() const { return iterator(span_, chunk_size_, span_.size() / chunk_size_); }

    // Rust `ChunksExact::remainder()` — the trailing elements that don't
    // fill a whole chunk. Independent of iteration progress, like Rust.
    Span remainder() const {
        const size_t full = (span_.size() / chunk_size_) * chunk_size_;
        return span_.subspan(full);
    }

private:
    Span span_;
    size_t chunk_size_;
};

template<typename Elem, std::size_t Extent, typename Size>
auto chunks_exact(std::span<Elem, Extent> span, Size chunk_size) {
    return ChunksExact<Elem, Extent>(span, static_cast<size_t>(chunk_size));
}

template<typename Container, typename Size>
auto chunks_exact(Container& container, Size chunk_size) {
    return chunks_exact(slice_full(container), chunk_size);
}

// Rust `slice::windows(n)` — overlapping subslices of length n; empty when
// the slice is shorter than n. Same begin/end-range shape as ChunksExact.
template<typename Elem, std::size_t Extent = std::dynamic_extent>
class Windows {
public:
    using Span = std::span<Elem, std::dynamic_extent>;

    class iterator {
    public:
        using value_type = Span;
        using difference_type = std::ptrdiff_t;

        iterator(Span span, size_t window, size_t index)
            : span_(span), window_(window), index_(index) {}

        value_type operator*() const {
            return span_.subspan(index_, window_);
        }

        iterator& operator++() {
            ++index_;
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return index_ != other.index_;
        }

    private:
        Span span_;
        size_t window_;
        size_t index_;
    };

    Windows(std::span<Elem, Extent> span, size_t window)
        : span_(span.data(), span.size()), window_(window == 0 ? 1 : window) {}

    iterator begin() const { return iterator(span_, window_, 0); }
    iterator end() const {
        const size_t n =
            span_.size() >= window_ ? span_.size() - window_ + 1 : 0;
        return iterator(span_, window_, n);
    }

private:
    Span span_;
    size_t window_;
};

template<typename Elem, std::size_t Extent, typename Size>
auto windows(std::span<Elem, Extent> span, Size window) {
    return Windows<Elem, Extent>(span, static_cast<size_t>(window));
}

template<typename Container, typename Size>
auto windows(Container& container, Size window) {
    return windows(slice_full(container), window);
}

// Rust `slice::chunks(n)` — non-overlapping subslices of length n; the
// final chunk may be shorter (unlike chunks_exact, which drops it).
template<typename Elem, std::size_t Extent = std::dynamic_extent>
class Chunks {
public:
    using Span = std::span<Elem, std::dynamic_extent>;

    class iterator {
    public:
        using value_type = Span;
        using difference_type = std::ptrdiff_t;

        iterator(Span span, size_t chunk_size, size_t index)
            : span_(span), chunk_size_(chunk_size), index_(index) {}

        value_type operator*() const {
            const size_t start = index_ * chunk_size_;
            const size_t len = std::min(chunk_size_, span_.size() - start);
            return span_.subspan(start, len);
        }

        iterator& operator++() {
            ++index_;
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return index_ != other.index_;
        }

    private:
        Span span_;
        size_t chunk_size_;
        size_t index_;
    };

    Chunks(std::span<Elem, Extent> span, size_t chunk_size)
        : span_(span.data(), span.size()),
          chunk_size_(chunk_size == 0 ? 1 : chunk_size) {}

    iterator begin() const { return iterator(span_, chunk_size_, 0); }
    iterator end() const {
        const size_t n = (span_.size() + chunk_size_ - 1) / chunk_size_;
        return iterator(span_, chunk_size_, n);
    }

private:
    Span span_;
    size_t chunk_size_;
};

template<typename Elem, std::size_t Extent, typename Size>
auto chunks(std::span<Elem, Extent> span, Size chunk_size) {
    return Chunks<Elem, Extent>(span, static_cast<size_t>(chunk_size));
}

template<typename Container, typename Size>
auto chunks(Container& container, Size chunk_size) {
    return chunks(slice_full(container), chunk_size);
}

namespace memchr_runtime {

inline Option<size_t> memchr(uint8_t needle, std::span<const uint8_t> haystack) {
    for (size_t i = 0; i < haystack.size(); ++i) {
        if (haystack[i] == needle) {
            return Option<size_t>(i);
        }
    }
    return Option<size_t>{None};
}

inline Option<size_t> memrchr(uint8_t needle, std::span<const uint8_t> haystack) {
    for (size_t i = haystack.size(); i > 0; --i) {
        if (haystack[i - 1] == needle) {
            return Option<size_t>(i - 1);
        }
    }
    return Option<size_t>{None};
}

inline Option<size_t> memchr2(uint8_t a, uint8_t b, std::span<const uint8_t> haystack) {
    for (size_t i = 0; i < haystack.size(); ++i) {
        if (haystack[i] == a || haystack[i] == b) {
            return Option<size_t>(i);
        }
    }
    return Option<size_t>{None};
}

class MemchrIter {
public:
    MemchrIter(uint8_t needle, std::span<const uint8_t> haystack)
        : needle_(needle), haystack_(haystack) {}

    size_t count() const {
        size_t total = 0;
        for (uint8_t byte : haystack_) {
            if (byte == needle_) {
                ++total;
            }
        }
        return total;
    }

private:
    uint8_t needle_;
    std::span<const uint8_t> haystack_;
};

inline MemchrIter memchr_iter(uint8_t needle, std::span<const uint8_t> haystack) {
    return MemchrIter(needle, haystack);
}

} // namespace memchr_runtime

template<typename Container, typename Mid>
requires (!std::is_same_v<std::remove_cvref_t<Container>, std::string_view>)
auto split_at(Container& container, Mid mid) {
    // A container spelling its own split_at (port map/set Slice types) wins
    // over the generic span split.
    if constexpr (requires { container.split_at(static_cast<size_t>(mid)); }) {
        return container.split_at(static_cast<size_t>(mid));
    } else {
        return split_at(slice_full(container), std::forward<Mid>(mid));
    }
}

// Clone elements from one slice into another.
// Mirrors Rust `[T]::clone_from_slice` semantics with a size check.
template<typename DstElem, std::size_t DstExtent, typename SrcElem, std::size_t SrcExtent>
void clone_from_slice(std::span<DstElem, DstExtent> dst, std::span<SrcElem, SrcExtent> src) {
    static_assert(!std::is_const_v<DstElem>, "clone_from_slice destination must be mutable");
    if (dst.size() != src.size()) {
        throw std::length_error("clone_from_slice length mismatch");
    }
    for (size_t i = 0; i < dst.size(); ++i) {
        if constexpr (requires(const SrcElem& value) { value.clone(); }) {
            dst[i] = src[i].clone();
        } else {
            dst[i] = src[i];
        }
    }
}

// str-typed slice overloads (declared ahead of the generics so their
// as_str() branches can call them; defined below).
template<typename Start>
std::string_view slice_from(std::string_view container, Start start);
template<typename End>
std::string_view slice_to(std::string_view container, End end);
template<typename End>
std::string_view slice_to_inclusive(std::string_view container, End end);
template<typename Start, typename End>
std::string_view slice(std::string_view container, Start start, End end);
template<typename Start, typename End>
std::string_view slice_inclusive(std::string_view container, Start start, End end);

template<typename Container, typename End>
decltype(auto) slice_to(Container& container, End end) {
    const size_t end_index = detail::checked_index(end);
    // A container with its own range-subscript (a transpiled `Index<RangeTo>`
    // impl, e.g. indexmap's Slice) defines Rust's `&c[..end]` — use it.
    // `index_mut` first: transpiled IndexMut lowers to that method.
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<Container, range_to<size_t>>::value) {
        return container.index_mut(range_to<size_t>{end_index});
    } else if constexpr (detail::range_subscript_probe<Container, range_to<size_t>>::value) {
        return container[range_to<size_t>{end_index}];
    } else
#endif
    if constexpr (requires {
                      { container.as_str() } -> std::convertible_to<std::string_view>;
                  }) {
        // Rust `&String[..b]` is &str — see slice above.
        return slice_to(std::string_view(container.as_str()), end_index);
    } else {
        auto span = slice_full(container);
        detail::validate_slice_bounds(span, 0, end_index);
        return span.first(end_index);
    }
}

template<typename Container, typename End>
decltype(auto) slice_to_inclusive(Container& container, End end) {
    const size_t end_index = detail::checked_index(end);
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<Container, range_to_inclusive<size_t>>::value) {
        return container.index_mut(range_to_inclusive<size_t>{end_index});
    } else if constexpr (detail::range_subscript_probe<Container, range_to_inclusive<size_t>>::value) {
        return container[range_to_inclusive<size_t>{end_index}];
    } else
#endif
    {
        return slice_to(container, end_index + 1);
    }
}

template<typename Start>
std::string_view slice_from(std::string_view container, Start start) {
    const size_t start_index = detail::checked_index(start);
    detail::validate_slice_bounds(container, start_index, container.size());
    return container.substr(start_index);
}

// Rust `&str[..end]` is &str — keep str-ness (substr), don't decay to a
// char span like the generic container path would.
template<typename End>
std::string_view slice_to(std::string_view container, End end) {
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(container, 0, end_index);
    return container.substr(0, end_index);
}

template<typename End>
std::string_view slice_to_inclusive(std::string_view container, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice_to(container, end_index + 1);
}

template<typename Start, typename End>
std::string_view slice_inclusive(std::string_view container, Start start, End end) {
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(container, start_index, end_index + 1);
    return container.substr(start_index, end_index + 1 - start_index);
}

template<typename Container, typename Start>
requires (!std::is_same_v<std::remove_cvref_t<Container>, std::string_view>)
decltype(auto) slice_from(Container& container, Start start) {
    const size_t start_index = detail::checked_index(start);
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<Container, range_from<size_t>>::value) {
        return container.index_mut(range_from<size_t>{start_index});
    } else if constexpr (detail::range_subscript_probe<Container, range_from<size_t>>::value) {
        return container[range_from<size_t>{start_index}];
    } else
#endif
    if constexpr (requires {
                      { container.as_str() } -> std::convertible_to<std::string_view>;
                  }) {
        // Rust `&String[a..]` is &str — see slice above.
        return slice_from(std::string_view(container.as_str()), start_index);
    } else {
        auto span = slice_full(container);
        detail::validate_slice_bounds(span, start_index, span.size());
        // str slices lower to std::string_view, which spells the tail
        // operation `substr`, not `subspan` (Vec/array slices use spans).
        if constexpr (std::is_same_v<std::remove_cvref_t<decltype(span)>,
                                     std::string_view>) {
            return span.substr(start_index);
        } else {
            return span.subspan(start_index);
        }
    }
}

template<typename Container, typename Start, typename End>
decltype(auto) slice(Container& container, Start start, End end) {
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<Container, range<size_t>>::value) {
        return container.index_mut(range<size_t>(start_index, end_index));
    } else if constexpr (detail::range_subscript_probe<Container, range<size_t>>::value) {
        return container[range<size_t>(start_index, end_index)];
    } else
#endif
    if constexpr (requires {
                      { container.as_str() } -> std::convertible_to<std::string_view>;
                  }) {
        // Rust `&String[a..b]` is &str — keep str-ness (substr over the
        // UTF-8 bytes) instead of decaying to a byte span.
        return slice(
            std::string_view(container.as_str()), start_index, end_index);
    } else {
        auto span = slice_full(container);
        detail::validate_slice_bounds(span, start_index, end_index);
        if constexpr (std::is_same_v<std::remove_cvref_t<Container>, std::string_view>) {
            return container.substr(start_index, end_index - start_index);
        } else {
            return span.subspan(start_index, end_index - start_index);
        }
    }
}

template<typename Start, typename End>
std::string_view slice(std::string_view container, Start start, End end) {
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(container, start_index, end_index);
    return container.substr(start_index, end_index - start_index);
}

template<typename Container, typename Start, typename End>
decltype(auto) slice_inclusive(Container& container, Start start, End end) {
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<Container, range_inclusive<size_t>>::value) {
        return container.index_mut(range_inclusive<size_t>(start_index, end_index));
    } else if constexpr (detail::range_subscript_probe<Container, range_inclusive<size_t>>::value) {
        return container[range_inclusive<size_t>(start_index, end_index)];
    } else
#endif
    {
        return slice(container, start_index, end_index + 1);
    }
}

// std::span is a borrowed VIEW — slicing an rvalue span is safe and common in
// emitted code (`self.as_entries()[range]` materializes a prvalue span). The
// generic overloads take `Container&`, which rejects rvalues; these by-value
// span overloads are more specialized, so lvalue spans also prefer them.
template<typename T, std::size_t E, typename End>
auto slice_to(std::span<T, E> container, End end) {
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(container, 0, end_index);
    return container.first(end_index);
}

template<typename T, std::size_t E, typename End>
auto slice_to_inclusive(std::span<T, E> container, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice_to(container, end_index + 1);
}

template<typename T, std::size_t E, typename Start>
auto slice_from(std::span<T, E> container, Start start) {
    const size_t start_index = detail::checked_index(start);
    detail::validate_slice_bounds(container, start_index, container.size());
    return container.subspan(start_index);
}

template<typename T, std::size_t E, typename Start, typename End>
auto slice(std::span<T, E> container, Start start, End end) {
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(container, start_index, end_index);
    return container.subspan(start_index, end_index - start_index);
}

template<typename T, std::size_t E, typename Start, typename End>
auto slice_inclusive(std::span<T, E> container, Start start, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice(container, start, end_index + 1);
}

template<typename T>
decltype(auto) field_end(T&& value) {
    if constexpr (requires { std::forward<T>(value).end_value(); }) {
        return (std::forward<T>(value).end_value());
    } else if constexpr (requires { std::forward<T>(value).end; }) {
        return (std::forward<T>(value).end);
    } else {
        return std::forward<T>(value).end();
    }
}

template<typename T>
decltype(auto) field_start(T&& value) {
    if constexpr (requires { std::forward<T>(value).start_value(); }) {
        return (std::forward<T>(value).start_value());
    } else if constexpr (requires { std::forward<T>(value).start; }) {
        return (std::forward<T>(value).start);
    } else {
        return std::forward<T>(value).start();
    }
}

// Runtime helper used by transpiled dynamic range indexing (`base[idx]` where
// `idx` is a range-shaped value). This mirrors Rust indexing semantics while
// supporting both string-backed and slice-backed bases.
template<typename Base, typename T>
auto index_with_range(Base&& base, const range<T>& idx) {
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        return slice(
            std::string_view(std::forward<Base>(base)),
            idx.start,
            idx.end_value()
        );
    } else {
        return slice(std::forward<Base>(base), idx.start, idx.end_value());
    }
}

template<typename Base, typename T>
auto index_with_range(Base&& base, const range_inclusive<T>& idx) {
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        auto view = std::string_view(std::forward<Base>(base));
        return slice_inclusive(view, idx.start, idx.end_value());
    } else {
        return slice_inclusive(std::forward<Base>(base), idx.start, idx.end_value());
    }
}

template<typename Base, typename T>
auto index_with_range(Base&& base, const range_from<T>& idx) {
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        return slice_from(std::string_view(std::forward<Base>(base)), idx.start);
    } else {
        return slice_from(std::forward<Base>(base), idx.start);
    }
}

template<typename Base, typename T>
auto index_with_range(Base&& base, const range_to<T>& idx) {
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        auto view = std::string_view(std::forward<Base>(base));
        return slice(view, static_cast<size_t>(0), idx.end);
    } else {
        return slice_to(std::forward<Base>(base), idx.end);
    }
}

template<typename Base, typename T>
auto index_with_range(Base&& base, const range_to_inclusive<T>& idx) {
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        auto view = std::string_view(std::forward<Base>(base));
        return slice_inclusive(view, static_cast<size_t>(0), idx.end);
    } else {
        return slice_to_inclusive(std::forward<Base>(base), idx.end);
    }
}

template<typename Base>
auto index_with_range(Base&& base, const range_full&) {
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        return std::string_view(std::forward<Base>(base));
    } else {
        return slice_full(std::forward<Base>(base));
    }
}

// Rust's `(Bound, Bound)` subscript — the RangeBounds tuple form. A receiver
// with its own transpiled `Index<(Bound<usize>, Bound<usize>)>` impl wins,
// matching Rust's impl selection; otherwise the pair decodes to concrete
// indices over the span view.
template<typename Base, typename B>
    requires detail::is_bound_pair_like_v<B>
decltype(auto) index_with_range(Base&& base, const B& bounds) {
#if RUSTY_RANGE_SUBSCRIPT_ROUTING
    if constexpr (detail::range_index_mut_probe<std::remove_reference_t<Base>, B>::value) {
        return base.index_mut(bounds);
    } else if constexpr (detail::range_subscript_probe<std::remove_reference_t<Base>, B>::value) {
        return base[bounds];
    } else
#endif
    if constexpr (std::is_convertible_v<Base&&, std::string_view>) {
        auto view = std::string_view(std::forward<Base>(base));
        const size_t start_index = detail::bound_pair_start(std::get<0>(bounds));
        const size_t end_index = detail::bound_pair_end(std::get<1>(bounds), view.size());
        detail::validate_slice_bounds(view, start_index, end_index);
        return view.substr(start_index, end_index - start_index);
    } else {
        std::span view{base};
        const size_t start_index = detail::bound_pair_start(std::get<0>(bounds));
        const size_t end_index = detail::bound_pair_end(std::get<1>(bounds), view.size());
        detail::validate_slice_bounds(view, start_index, end_index);
        return view.subspan(start_index, end_index - start_index);
    }
}

} // namespace rusty

#endif // RUSTY_ARRAY_HPP
