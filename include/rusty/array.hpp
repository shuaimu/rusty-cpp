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
template<typename T>
class Box;

template<typename Container>
auto as_slice(Container&& container);
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

// Mirror Rust slice equality semantics across borrowed spans and owned
// rusty::Vec payloads used by transpiled assertion scaffolding.
template<typename L, std::size_t LExtent, typename R>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(std::span<L, LExtent> lhs, const rusty::Vec<R>& rhs) {
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

template<typename L, typename R, std::size_t RExtent>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(const rusty::Vec<L>& lhs, std::span<R, RExtent> rhs) {
    return rhs == lhs;
}

template<typename L, typename R, std::size_t N>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(const rusty::Vec<L>& lhs, const std::array<R, N>& rhs) {
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

template<typename L, std::size_t N, typename R>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(const std::array<L, N>& lhs, const rusty::Vec<R>& rhs) {
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

    ArrayRepeatResult& operator=(const rusty::Vec<T>& rhs) {
        values_.assign(rhs.begin(), rhs.end());
        return *this;
    }

    ArrayRepeatResult& operator=(rusty::Vec<T>&& rhs) {
        values_.clear();
        values_.reserve(rhs.size());
        for (auto& item : rhs) {
            values_.push_back(std::move(item));
        }
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

    template<typename U = T>
    operator rusty::Vec<U>() const {
        rusty::Vec<U> out = rusty::Vec<U>::with_capacity(values_.size());
        for (const auto& item : values_) {
            out.push(static_cast<U>(item));
        }
        return out;
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
    return std::min(static_cast<C>(std::forward<A>(a)), static_cast<C>(std::forward<B>(b)));
}

template<typename A, typename B>
constexpr auto max(A&& a, B&& b) {
    using C = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    return std::max(static_cast<C>(std::forward<A>(a)), static_cast<C>(std::forward<B>(b)));
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

template<typename T>
Box<std::span<T>> into_boxed_slice(Vec<T> values) {
    const auto len = values.len();
    T* storage =
        (len == 0) ? nullptr : static_cast<T*>(::operator new(sizeof(T) * len));
    for (size_t i = 0; i < len; ++i) {
        new (storage + i) T(std::move(values[i]));
    }
    return Box<std::span<T>>::new_(std::span<T>(storage, len));
}

template<typename T>
Box<std::span<T>> into_boxed_slice(ArrayRepeatResult<T> values) {
    return into_boxed_slice(static_cast<std::vector<T>>(values));
}

/// Collect any iterable range into rusty::Vec<T>.
/// Used by transpiled Rust range `.collect()` calls.
template<typename Range>
auto collect_range(Range&& range_like) {
    if constexpr (requires(Range&& r) {
        std::begin(r);
        std::end(r);
    }) {
        using Elem = std::decay_t<decltype(*std::begin(range_like))>;
        Vec<Elem> out = Vec<Elem>::new_();
        for (auto&& item : range_like) {
            detail::push_collect_item(out, std::forward<decltype(item)>(item));
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
        Vec<Elem> out = Vec<Elem>::new_();
        while (true) {
            auto item = iter.next();
            if (!detail::option_has_value(item)) {
                break;
            }
            decltype(auto) value = detail::option_take_value(item);
            detail::push_collect_item(out, std::forward<decltype(value)>(value));
        }
        return out;
    } else if constexpr (requires(Range&& r) { std::forward<Range>(r).into_iter(); }) {
        return collect_range(std::forward<Range>(range_like).into_iter());
    } else {
        static_assert(
            detail::collect_range_dependent_false_v<Range>,
            "rusty::collect_range requires a range, into_iter(), or Option-like next()");
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
            if constexpr (!std::is_same_v<Pointee, char>) {
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
            if constexpr (!std::is_same_v<Pointee, char>) {
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

template<typename Left, typename Right>
auto zip(Left&& left, Right&& right) {
    using LeftElem = std::decay_t<decltype(*std::begin(left))>;
    using RightElem = std::decay_t<decltype(*std::begin(right))>;
    std::vector<std::tuple<LeftElem, RightElem>> out;
    auto left_it = std::begin(left);
    auto left_end = std::end(left);
    auto right_it = std::begin(right);
    auto right_end = std::end(right);
    for (; left_it != left_end && right_it != right_end; ++left_it, ++right_it) {
        out.emplace_back(*left_it, *right_it);
    }
    return out;
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
    using R = std::common_type_t<T, U>;
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
    using R = std::common_type_t<T, U>;
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
    using R = std::common_type_t<T, U>;
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
    } else if constexpr (requires(StoredRange& r) { r.next(); }) {
        auto iter = std::forward<Range>(range);
        auto mapper = std::forward<Func>(func);
        using NextResult = std::decay_t<decltype(iter.next())>;
        static_assert(
            detail::has_is_some<NextResult>::value || detail::has_has_value<NextResult>::value,
            "rusty::filter_map iterator fallback requires next() to return Option-like value"
        );
        using Item = detail::option_value_t<NextResult>;
        using MapResult = std::decay_t<decltype(mapper(std::declval<Item>()))>;
        static_assert(
            detail::has_is_some<MapResult>::value || detail::has_has_value<MapResult>::value,
            "rusty::filter_map mapper must return Option-like value"
        );
        using Value = detail::option_value_t<MapResult>;
        rusty::Vec<Value> out;
        while (true) {
            auto item = iter.next();
            if (!detail::option_has_value(item)) {
                break;
            }
            auto mapped = mapper(detail::option_take_value(item));
            if (detail::option_has_value(mapped)) {
                decltype(auto) value = detail::option_take_value(mapped);
                detail::push_collect_item(out, std::forward<decltype(value)>(value));
            }
        }
        return out;
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
    return detail::owned_container_slice<std::remove_cv_t<std::remove_reference_t<Container>>>{
        std::forward<Container>(container)};
}

template<typename Container>
auto slice_full(Container& container) {
    using Base = std::remove_cv_t<std::remove_reference_t<Container>>;
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
auto slice_full(const Container& container) {
    using Base = std::remove_cv_t<std::remove_reference_t<Container>>;
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
auto slice_full(rusty::Box<T>& container) {
    return slice_full(*container);
}

template<typename T>
auto slice_full(const rusty::Box<T>& container) {
    return slice_full(*container);
}

// Explicit helper surface for Rust-style `.as_slice()` lowering.
// Keeps const-view semantics even for mutable lvalue receivers and supports
// temporary receivers through forwarding-reference binding.
template<typename Container>
auto as_slice(Container&& container) {
    if constexpr (std::is_rvalue_reference_v<Container&&>) {
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
auto as_mut_slice(Container&& container) {
    return slice_full(std::forward<Container>(container));
}

// Normalize arbitrary slice-like containers into a byte view.
// For non-u8 element containers this keeps a thread-local converted buffer.
template<typename Container>
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

// Collect a slice-like container into rusty::Vec by value-cloning elements.
// Used by transpiled Rust `.to_vec()` lowering for slice/array/ArrayVec shapes.
template<typename Container>
auto to_vec(const Container& container) {
    auto span = slice_full(container);
    using Elem = std::remove_cv_t<std::remove_reference_t<decltype(*span.data())>>;
    Vec<Elem> out = Vec<Elem>::new_();
    for (const auto& item : span) {
        if constexpr (requires(const Elem& e) { e.clone(); }) {
            out.push(item.clone());
        } else {
            out.push(item);
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

template<typename Container, typename Mid>
requires (!std::is_same_v<std::remove_cvref_t<Container>, std::string_view>)
auto split_at(Container& container, Mid mid) {
    return split_at(slice_full(container), std::forward<Mid>(mid));
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

template<typename Container, typename End>
auto slice_to(Container& container, End end) {
    auto span = slice_full(container);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(span, 0, end_index);
    return span.first(end_index);
}

template<typename Container, typename End>
auto slice_to_inclusive(Container& container, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice_to(container, end_index + 1);
}

template<typename Start>
auto slice_from(std::string_view container, Start start) {
    const size_t start_index = detail::checked_index(start);
    detail::validate_slice_bounds(container, start_index, container.size());
    return container.substr(start_index);
}

template<typename Container, typename Start>
requires (!std::is_same_v<std::remove_cvref_t<Container>, std::string_view>)
auto slice_from(Container& container, Start start) {
    auto span = slice_full(container);
    const size_t start_index = detail::checked_index(start);
    detail::validate_slice_bounds(span, start_index, span.size());
    return span.subspan(start_index);
}

template<typename Container, typename Start, typename End>
auto slice(Container& container, Start start, End end) {
    auto span = slice_full(container);
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(span, start_index, end_index);
    if constexpr (std::is_same_v<std::remove_cvref_t<Container>, std::string_view>) {
        return container.substr(start_index, end_index - start_index);
    } else {
        return span.subspan(start_index, end_index - start_index);
    }
}

template<typename Start, typename End>
auto slice(std::string_view container, Start start, End end) {
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(container, start_index, end_index);
    return container.substr(start_index, end_index - start_index);
}

template<typename Container, typename Start, typename End>
auto slice_inclusive(Container& container, Start start, End end) {
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
    if (start_index > end_index || end_index > container.size()) {
        return Opt(None);
    }
    return Opt(container.substr(start_index, end_index - start_index));
}

template<typename T>
auto get(std::string_view container, const range_inclusive<T>& idx) {
    const size_t start_index = detail::checked_index(idx.start);
    const size_t end_index = detail::checked_index(idx.end_value());
    using Opt = Option<std::string_view>;
    if (start_index > end_index || end_index >= container.size()) {
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

} // namespace rusty

#endif // RUSTY_ARRAY_HPP
