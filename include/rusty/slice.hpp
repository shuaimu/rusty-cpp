#ifndef RUSTY_SLICE_HPP
#define RUSTY_SLICE_HPP

#include <array>
#include <algorithm>
#include <cstddef>
#include <functional>
#include <iterator>
#include <limits>
#include <memory>
#include <optional>
#include <span>
#include <tuple>
#include <type_traits>
#include <utility>
#include <vector>

#include "rusty/option.hpp"
#include "rusty/dispatch.hpp"
#include "rusty/peekable.hpp"

namespace rusty {

// Raw-pointer slice constructors used by expanded std/core::slice paths.
template<typename T>
auto from_raw_parts(const T* ptr, size_t len) {
    return std::span<const T>(ptr, len);
}

template<typename T>
auto from_raw_parts_mut(T* ptr, size_t len) {
    return std::span<T>(ptr, len);
}

template<typename Range, typename Value>
void fill(Range&& range, Value&& value) {
    if constexpr (std::is_pointer_v<std::remove_reference_t<Range>>) {
        auto&& view = *std::forward<Range>(range);
        std::fill(std::begin(view), std::end(view), std::forward<Value>(value));
    } else {
        auto&& view = std::forward<Range>(range);
        std::fill(std::begin(view), std::end(view), std::forward<Value>(value));
    }
}

namespace slice_iter {

template<typename T>
class Iter {
public:
    using elem_type = std::remove_const_t<T>;
    using pointer =
        std::conditional_t<std::is_const_v<T>, const elem_type*, elem_type*>;

    Iter() : cur_(nullptr), end_(nullptr) {}
    Iter(pointer begin, pointer end) : cur_(begin), end_(end) {}

    template<std::size_t Extent>
    explicit Iter(std::span<T, Extent> span)
        : cur_(span.data()), end_(span.data() + span.size()) {}

    // Iterator const-conversion (Iter<T> -> Iter<const T>), the same
    // qualification conversion std iterators allow: a Default-constructed
    // wrapper (`Self { iter: [].iter() }`) builds from a mutable-element
    // source while the field spells the const iterator.
    template<typename U>
        requires(!std::is_same_v<U, T>
                 && std::is_convertible_v<U (*)[], T (*)[]>)
    Iter(const Iter<U>& other)
        : cur_(other.raw_cur()), end_(other.raw_end()) {}

    pointer raw_cur() const { return cur_; }
    pointer raw_end() const { return end_; }

    Iter into_iter() const { return *this; }
    Iter& by_ref() { return *this; }
    const Iter& by_ref() const { return *this; }

    // Rust `Iterator::peekable()` — consume into the caching wrapper. Also
    // satisfies the transpiler's type-position spelling
    // `decltype(std::declval<Iter>().peekable())`.
    rusty::iter_adapters::Peekable<Iter> peekable() const {
        return rusty::iter_adapters::Peekable<Iter>(*this);
    }

    rusty::Option<pointer> next() {
        if (cur_ == end_) {
            return rusty::None;
        }
        pointer current = cur_;
        ++cur_;
        return rusty::Option<pointer>(current);
    }

    rusty::Option<pointer> next_back() {
        if (cur_ == end_) {
            return rusty::None;
        }
        --end_;
        return rusty::Option<pointer>(end_);
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        const size_t remaining = static_cast<size_t>(end_ - cur_);
        return std::make_tuple(remaining, rusty::Option<size_t>(remaining));
    }

    size_t count() {
        const size_t remaining = static_cast<size_t>(end_ - cur_);
        cur_ = end_;
        return remaining;
    }

    template<typename Pred>
    rusty::Option<size_t> position(Pred&& pred) {
        size_t idx = 0;
        while (cur_ != end_) {
            pointer current = cur_;
            ++cur_;
            if (std::forward<Pred>(pred)(*current)) {
                return rusty::Option<size_t>(idx);
            }
            ++idx;
        }
        return rusty::None;
    }

    // rposition: index (from the front) of the LAST element matching pred.
    // Consumes the iterator with a forward scan keeping the last match.
    template<typename Pred>
    rusty::Option<size_t> rposition(Pred&& pred) {
        rusty::Option<size_t> found = rusty::None;
        size_t idx = 0;
        while (cur_ != end_) {
            pointer current = cur_;
            ++cur_;
            if (pred(*current)) {
                found = rusty::Option<size_t>(idx);
            }
            ++idx;
        }
        return found;
    }

    class ClonedIter {
    public:
        using value_type = std::remove_const_t<T>;

        explicit ClonedIter(Iter iter) : iter_(std::move(iter)) {}

        ClonedIter into_iter() const { return *this; }

        rusty::Option<value_type> next() {
            auto next_ptr = iter_.next();
            if (next_ptr.is_none()) {
                return rusty::None;
            }
            return rusty::Option<value_type>(clone_value(*next_ptr.unwrap()));
        }

        rusty::Option<value_type> next_back() {
            auto next_ptr = iter_.next_back();
            if (next_ptr.is_none()) {
                return rusty::None;
            }
            return rusty::Option<value_type>(clone_value(*next_ptr.unwrap()));
        }

        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return iter_.size_hint();
        }

    private:
        template<typename U>
        static U clone_value(const U& value) {
            if constexpr (requires { value.clone(); }) {
                return value.clone();
            } else {
                static_assert(
                    std::is_copy_constructible_v<U>,
                    "rusty::slice_iter::Iter::cloned requires copy-constructible or clone() values"
                );
                return U(value);
            }
        }

        Iter iter_;
    };

    ClonedIter cloned() const { return ClonedIter(*this); }
    ClonedIter copied() const { return ClonedIter(*this); }

private:
    pointer cur_;
    pointer end_;
};

} // namespace slice_iter

namespace detail {

// Rebind a template's arguments onto another template: for a Rust
// `impl<K, V> From<Src<K, V>> for Owner<K, V>`, the emitted
// `Owner::from(arg)` can name its owner as
// `rebind_from<Owner, remove_cvref_t<decltype(arg)>>::type` when the
// owner's template args are not recoverable at transpile time — the C++
// compiler extracts them from the argument's concrete type (indexmap's
// `Entry::Occupied(e) => IndexedEntry::from(e)`). The transpiler only
// emits this when the impl's parameter shares the owner's params in the
// same order.
template<template<class...> class Owner, class Src>
struct rebind_from;

template<template<class...> class Owner, template<class...> class Src, class... As>
struct rebind_from<Owner, Src<As...>> {
    using type = Owner<As...>;
};

template<template<class...> class Owner, class Arg>
using rebind_from_t =
    typename rebind_from<Owner, std::remove_cvref_t<Arg>>::type;

template<typename T>
inline constexpr bool dependent_false_v = false;

template<typename T>
inline constexpr bool is_std_array_v = false;
template<typename T, std::size_t N>
inline constexpr bool is_std_array_v<std::array<T, N>> = true;

#ifndef RUSTY_DETAIL_STD_ARRAY_LIKE_TRAIT_DEFINED
#define RUSTY_DETAIL_STD_ARRAY_LIKE_TRAIT_DEFINED
template<typename T>
struct is_std_array_like : std::false_type {};

template<typename T, std::size_t N>
struct is_std_array_like<std::array<T, N>> : std::true_type {};

template<typename T>
inline constexpr bool is_std_array_like_v = is_std_array_like<T>::value;
#endif

template<typename T, typename = void>
struct slice_has_is_some : std::false_type {};

template<typename T>
struct slice_has_is_some<T, std::void_t<decltype(std::declval<const T&>().is_some())>>
    : std::true_type {};

template<typename T, typename = void>
struct slice_has_unwrap : std::false_type {};

template<typename T>
struct slice_has_unwrap<T, std::void_t<decltype(std::declval<T&>().unwrap())>>
    : std::true_type {};

template<typename T, typename = void>
struct slice_has_has_value : std::false_type {};

template<typename T>
struct slice_has_has_value<T, std::void_t<decltype(std::declval<const T&>().has_value())>>
    : std::true_type {};

template<typename T, typename = void>
struct slice_has_reset : std::false_type {};

template<typename T>
struct slice_has_reset<T, std::void_t<decltype(std::declval<T&>().reset())>>
    : std::true_type {};

template<typename Opt>
bool option_like_has_value(const Opt& opt) {
    if constexpr (slice_has_is_some<Opt>::value) {
        return opt.is_some();
    } else if constexpr (slice_has_has_value<Opt>::value) {
        return opt.has_value();
    } else {
        return static_cast<bool>(opt);
    }
}

template<typename Opt>
decltype(auto) option_like_take_value(Opt& opt) {
    if constexpr (slice_has_unwrap<Opt>::value) {
        // decltype(auto): a reference-payload Option (Option<const T&>,
        // btree_port's map::Values items) yields the reference itself —
        // the payload may be move-only (rusty::String), so materializing
        // a value here would be a deleted copy. Callers that need a value
        // (copied()/cloned()) construct one explicitly. Value-payload
        // Options deduce T by value exactly as before.
        return opt.unwrap();
    } else if constexpr (slice_has_has_value<Opt>::value && slice_has_reset<Opt>::value) {
        auto value = std::move(*opt);
        opt.reset();
        return value;
    } else {
        return std::move(*opt);
    }
}

template<typename NextResult, typename = void>
struct is_option_like_next_result : std::false_type {};

template<typename NextResult>
struct is_option_like_next_result<
    NextResult,
    std::void_t<
        decltype(option_like_has_value(std::declval<const NextResult&>())),
        decltype(option_like_take_value(std::declval<NextResult&>()))>> : std::true_type {};

template<typename NextResult>
inline constexpr bool is_option_like_next_result_v =
    is_option_like_next_result<NextResult>::value;

template<typename Iter, typename = void>
struct has_option_like_next : std::false_type {};

template<typename Iter>
struct has_option_like_next<Iter, std::void_t<decltype(std::declval<Iter&>().next())>>
    : std::bool_constant<
          is_option_like_next_result_v<decltype(std::declval<Iter&>().next())>> {};

template<typename Iter>
inline constexpr bool has_option_like_next_v = has_option_like_next<Iter>::value;

template<typename Iter>
using next_result_t = decltype(std::declval<Iter&>().next());

template<typename Iter>
using next_item_t =
    std::decay_t<decltype(option_like_take_value(std::declval<next_result_t<Iter>&>()))>;

template<typename T>
constexpr decltype(auto) deref_if_pointer(T&& value) {
    using deref_value_type = std::remove_reference_t<T>;
    if constexpr (std::is_pointer_v<deref_value_type>
                  && std::is_same_v<
                         std::remove_cv_t<std::remove_pointer_t<deref_value_type>>,
                         char>) {
        // Same rule as deref_if_pointer_like below: a plain-char pointer is
        // a STR carrier (Rust `&str`), never a reference-to-char — peeling
        // it read the FIRST CHARACTER (`t.0` on ("bye", 'z') gave 'b').
        return std::forward<T>(value);
    } else if constexpr (std::is_pointer_v<deref_value_type>) {
        return *std::forward<T>(value);
    } else {
        return std::forward<T>(value);
    }
}

template<typename T>
constexpr decltype(auto) deref_if_pointer_like(T&& value) {
    using value_type = std::remove_reference_t<T>;
    if constexpr (std::is_pointer_v<value_type>
                  && std::is_same_v<
                         std::remove_cv_t<std::remove_pointer_t<value_type>>, char>) {
        // A plain-char pointer is a STR carrier (Rust `&str` lowers to
        // const char*), never a reference-to-char: Rust has no plain-char
        // type (i8 = signed char, u8 = unsigned char, char = char32_t).
        // Peeling it read the FIRST CHARACTER — `Pair("a", "b") ==
        // (String, String)` compared 'a' against a String. Content
        // semantics live with the pointer itself.
        return std::forward<T>(value);
    } else if constexpr (std::is_pointer_v<value_type>) {
        return *std::forward<T>(value);
    } else if constexpr (requires {
                             typename std::remove_cv_t<
                                 value_type>::rusty_pointer_identity_semantics;
                         }) {
        // Types marked with `rusty_pointer_identity_semantics` (rusty::ptr::
        // NonNull) compare/hash by pointer VALUE in Rust — never through the
        // pointee. Unboxing them here would read the pointee, which is wrong
        // for value comparison and UB for tagged-pointer reprs (semver's
        // Identifier stores inline string bytes in the pointer itself).
        return std::forward<T>(value);
    } else if constexpr (requires(value_type& v) {
                             v.get();
                             *v;
                         }) {
        if constexpr (std::is_pointer_v<decltype(std::declval<value_type&>().get())>) {
            return *std::forward<T>(value);
        } else {
            return std::forward<T>(value);
        }
    } else if constexpr (!std::is_array_v<value_type> && requires(value_type& v) { *v; }) {
        // NB: arrays are EXCLUDED — a C-array (e.g. a string literal
        // `const char[N]`) satisfies `*v` via array-to-pointer decay, and
        // dereferencing it would wrongly yield a single element (`const char`),
        // breaking e.g. a `std::string_view` parameter. Real pointers are
        // already handled by the `is_pointer_v` branch above; only genuine
        // deref-wrapper *types* (ArrayVec/ArrayString/smart pointers) should
        // unbox here.
        using deref_type = std::remove_cvref_t<decltype(*std::declval<value_type&>())>;
        // Deref-like value types (for example ArrayVec/ArrayString) should compare/hash
        // on their target view instead of recursing through self-comparisons.
        if constexpr (!std::is_same_v<deref_type, value_type>) {
            if constexpr (std::is_pointer_v<deref_type>) {
                // A wrapper whose Deref target is a raw POINTER (a
                // ScopeGuard aliasing its place by address): the value is
                // two hops away.
                return *(*std::forward<T>(value));
            } else {
                return *std::forward<T>(value);
            }
        } else {
            return std::forward<T>(value);
        }
    } else {
        return std::forward<T>(value);
    }
}

template<typename T, typename = void>
struct variant_underlying_type {
    using type = std::remove_cvref_t<T>;
};

template<typename T>
struct variant_underlying_type<
    T,
    std::void_t<typename std::remove_cvref_t<T>::variant>> {
    using type = typename std::remove_cvref_t<T>::variant;
};

template<typename T>
using variant_underlying_type_t = typename variant_underlying_type<T>::type;

template<typename NextIter>
class next_iter_range {
public:
    static_assert(
        has_option_like_next_v<NextIter>,
        "rusty::for_in requires next() to return an Option/optional-like value"
    );

    explicit next_iter_range(NextIter iter) : iter_(std::forward<NextIter>(iter)) {}

    class iterator {
        using item_type = next_item_t<NextIter>;
        using iter_type = std::remove_reference_t<NextIter>;
        // A reference-payload item (ValuesMut/IterMut yield `Option<T&>`)
        // must not decay into the slot: `for v in map.values_mut() { *v *= 2 }`
        // would mutate a copy. Carry the referent's address instead —
        // `deref_if_pointer` in operator* restores the lvalue.
        using taken_type =
            decltype(option_like_take_value(std::declval<next_result_t<iter_type>&>()));
        static constexpr bool item_is_lvalue_ref = std::is_lvalue_reference_v<taken_type>;
        using stored_type = std::conditional_t<
            item_is_lvalue_ref,
            std::remove_reference_t<taken_type>*,
            item_type>;

    public:
        iterator() : iter_(nullptr), at_end_(true) {}

        explicit iterator(iter_type* iter, bool at_end = false)
            : iter_(iter), at_end_(at_end) {
            if (!at_end_) {
                advance();
            }
        }

        decltype(auto) operator*() {
            return deref_if_pointer(*current_);
        }

        decltype(auto) operator*() const {
            return deref_if_pointer(*current_);
        }

        iterator& operator++() {
            advance();
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return at_end_ != other.at_end_;
        }

    private:
        void advance() {
            current_.reset();
            auto next_item = iter_->next();
            if (!option_like_has_value(next_item)) {
                at_end_ = true;
                return;
            }
            if constexpr (item_is_lvalue_ref) {
                current_.emplace(std::addressof(option_like_take_value(next_item)));
            } else {
                current_.emplace(option_like_take_value(next_item));
            }
            at_end_ = false;
        }

        iter_type* iter_;
        std::optional<stored_type> current_;
        bool at_end_;
    };

    iterator begin() { return iterator(&iter_); }
    iterator end() { return iterator(&iter_, true); }

private:
    NextIter iter_;
};

template<typename Iter, typename Func>
class map_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::map requires next() to return an Option/optional-like value"
    );

    map_next_iter(Iter iter, Func func)
        : iter_(std::move(iter)), func_(std::move(func)) {}

    map_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = next_item_t<Iter>;
        using mapped_type = std::decay_t<decltype(
            std::invoke(std::declval<Func&>(), deref_if_pointer(std::declval<item_type>())))>;
        using next_result = rusty::Option<mapped_type>;

        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(std::invoke(
            func_,
            deref_if_pointer(option_like_take_value(item))));
    }

private:
    Iter iter_;
    Func func_;
};

template<typename Iter>
class enumerate_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::enumerate requires next() to return an Option/optional-like value"
    );

    explicit enumerate_next_iter(Iter iter) : iter_(std::move(iter)), index_(0) {}

    enumerate_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = next_item_t<Iter>;
        using entry_item_type = std::conditional_t<
            std::is_pointer_v<item_type>,
            decltype(deref_if_pointer(std::declval<item_type>())),
            item_type>;
        using entry_type = std::tuple<size_t, entry_item_type>;
        using next_result = rusty::Option<entry_type>;
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(entry_type(
            index_++,
            deref_if_pointer(option_like_take_value(item)))
        );
    }

    auto next_back() {
        static_assert(
            requires(Iter& iter) { iter.next_back(); },
            "rusty::enumerate::next_back requires next_back() on the inner iterator"
        );
        static_assert(
            requires(const Iter& iter) { iter.size_hint(); },
            "rusty::enumerate::next_back requires size_hint() on the inner iterator"
        );

        using item_type = next_item_t<Iter>;
        using entry_item_type = std::conditional_t<
            std::is_pointer_v<item_type>,
            decltype(deref_if_pointer(std::declval<item_type>())),
            item_type>;
        using entry_type = std::tuple<size_t, entry_item_type>;
        using next_result = rusty::Option<entry_type>;

        const auto hint = iter_.size_hint();
        size_t remaining = std::get<0>(hint);
        auto item = iter_.next_back();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        if (remaining == 0) {
            remaining = 1;
        }
        return next_result(entry_type(
            index_ + (remaining - 1),
            deref_if_pointer(option_like_take_value(item)))
        );
    }

private:
    Iter iter_;
    size_t index_;
};

template<typename Iter>
class rev_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::rev requires next() to return an Option/optional-like value"
    );
    static_assert(
        requires(Iter& iter) { iter.next_back(); },
        "rusty::rev requires next_back() on next-like iterators"
    );

    explicit rev_next_iter(Iter iter) : iter_(std::move(iter)) {}

    rev_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = next_item_t<Iter>;
        using next_result = rusty::Option<item_type>;

        auto item = iter_.next_back();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

private:
    Iter iter_;
};

// Rust `Iterator::take_while(pred)` / `::skip_while(pred)` over any
// option-like-next iterator. take_while ends the iteration permanently at
// the first non-matching item; skip_while drops the initial matching run
// and then yields everything.
template<typename Iter, typename Pred>
class take_while_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::take_while requires next() to return an Option/optional-like value"
    );

    take_while_next_iter(Iter iter, Pred pred)
        : iter_(std::move(iter)), pred_(std::move(pred)) {}

    take_while_next_iter into_iter() {
        return std::move(*this);
    }

    rusty::Option<next_item_t<Iter>> next() {
        using next_result = rusty::Option<next_item_t<Iter>>;
        if (done_) {
            return next_result(rusty::None);
        }
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            done_ = true;
            return next_result(rusty::None);
        }
        auto value = option_like_take_value(item);
        if (!pred_(value)) {
            done_ = true;
            return next_result(rusty::None);
        }
        return next_result(std::move(value));
    }

private:
    Iter iter_;
    Pred pred_;
    bool done_ = false;
};

template<typename Iter, typename Pred>
auto make_take_while_next_iter(Iter&& iter, Pred&& pred) {
    return take_while_next_iter<std::remove_reference_t<Iter>,
                                std::remove_reference_t<Pred>>(
        std::forward<Iter>(iter), std::forward<Pred>(pred));
}

template<typename Iter, typename Pred>
class skip_while_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::skip_while requires next() to return an Option/optional-like value"
    );

    skip_while_next_iter(Iter iter, Pred pred)
        : iter_(std::move(iter)), pred_(std::move(pred)) {}

    skip_while_next_iter into_iter() {
        return std::move(*this);
    }

    rusty::Option<next_item_t<Iter>> next() {
        using next_result = rusty::Option<next_item_t<Iter>>;
        while (true) {
            auto item = iter_.next();
            if (!option_like_has_value(item)) {
                return next_result(rusty::None);
            }
            auto value = option_like_take_value(item);
            if (skipping_ && pred_(value)) {
                continue;
            }
            skipping_ = false;
            return next_result(std::move(value));
        }
    }

private:
    Iter iter_;
    Pred pred_;
    bool skipping_ = true;
};

template<typename Iter, typename Pred>
auto make_skip_while_next_iter(Iter&& iter, Pred&& pred) {
    return skip_while_next_iter<std::remove_reference_t<Iter>,
                                std::remove_reference_t<Pred>>(
        std::forward<Iter>(iter), std::forward<Pred>(pred));
}

// Rust `Iterator::inspect(f)` — passthrough adapter running f(&item) for
// its side effect on every yielded item.
template<typename Iter, typename Func>
class inspect_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::inspect requires next() to return an Option/optional-like value"
    );

    inspect_next_iter(Iter iter, Func func)
        : iter_(std::move(iter)), func_(std::move(func)) {}

    inspect_next_iter into_iter() {
        return std::move(*this);
    }

    rusty::Option<next_item_t<Iter>> next() {
        using next_result = rusty::Option<next_item_t<Iter>>;
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto value = option_like_take_value(item);
        func_(value);
        return next_result(std::move(value));
    }

private:
    Iter iter_;
    Func func_;
};

template<typename Iter, typename Func>
auto make_inspect_next_iter(Iter&& iter, Func&& func) {
    return inspect_next_iter<std::remove_reference_t<Iter>,
                             std::remove_reference_t<Func>>(
        std::forward<Iter>(iter), std::forward<Func>(func));
}

// Rust `Iterator::fuse()` — once next() yields None, keep yielding None.
template<typename Iter>
class fuse_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::fuse requires next() to return an Option/optional-like value"
    );

    explicit fuse_next_iter(Iter iter) : iter_(std::move(iter)) {}

    fuse_next_iter into_iter() {
        return std::move(*this);
    }

    rusty::Option<next_item_t<Iter>> next() {
        using next_result = rusty::Option<next_item_t<Iter>>;
        if (done_) {
            return next_result(rusty::None);
        }
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            done_ = true;
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

private:
    Iter iter_;
    bool done_ = false;
};

template<typename Iter>
auto make_fuse_next_iter(Iter&& iter) {
    return fuse_next_iter<std::remove_reference_t<Iter>>(
        std::forward<Iter>(iter));
}

// Rust `Iterator::copied()` / `::cloned()` over ANY option-like-next
// iterator whose items are references — materializes each item by value.
// Runtime iterator types (slice_iter::Iter etc.) carry their own member
// adapters; these wrappers give the same surface to TRANSPILED iterator
// types (btree_port's set::Iter / map::Keys / set::Union, ...), which have
// `next()` but no adapter members (their Rust adapters are Iterator
// default methods, not inherent ones).
template<typename Iter>
class copied_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::copied requires next() to return an Option/optional-like value"
    );

    explicit copied_next_iter(Iter iter) : iter_(std::move(iter)) {}

    copied_next_iter into_iter() {
        return std::move(*this);
    }

    // DECLARED return type: option-like-next probes name this signature
    // without instantiating the body. A deduced return makes every
    // requires-probe instantiate through the wrapped iterator's next(),
    // which clang resolves order-sensitively across deferred POIs in
    // module units (indexmap set-ops from_iter).
    rusty::Option<next_item_t<Iter>> next() {
        using next_result = rusty::Option<next_item_t<Iter>>;

        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

private:
    Iter iter_;
};

template<typename Iter>
auto make_copied_next_iter(Iter&& iter) {
    return copied_next_iter<std::remove_reference_t<Iter>>(
        std::forward<Iter>(iter));
}

template<typename Iter>
class cloned_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::cloned requires next() to return an Option/optional-like value"
    );

    explicit cloned_next_iter(Iter iter) : iter_(std::move(iter)) {}

    cloned_next_iter into_iter() {
        return std::move(*this);
    }

    // DECLARED return type: option-like-next probes name this signature
    // without instantiating the body (see copied_next_iter::next).
    rusty::Option<next_item_t<Iter>> next() {
        using item_type = next_item_t<Iter>;
        using next_result = rusty::Option<item_type>;

        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        // Mirror slice_iter::Iter::cloned(): honor a user clone() when the
        // item type has one, fall back to copy construction otherwise.
        decltype(auto) value = option_like_take_value(item);
        if constexpr (requires { value.clone(); }) {
            return next_result(value.clone());
        } else {
            static_assert(
                std::is_copy_constructible_v<item_type>,
                "rusty::cloned requires copy-constructible or clone() items"
            );
            return next_result(item_type(value));
        }
    }

private:
    Iter iter_;
};

template<typename Iter>
auto make_cloned_next_iter(Iter&& iter) {
    return cloned_next_iter<std::remove_reference_t<Iter>>(
        std::forward<Iter>(iter));
}

// Rust `Iterator::cycle()` over ANY option-like-next iterator: repeats the
// base sequence by restarting from a saved copy of the start state. An
// empty base stays exhausted (Rust: cycle() of an empty iterator keeps
// returning None rather than spinning).
template<typename Iter>
class cycle_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::cycle requires next() to return an Option/optional-like value"
    );
    static_assert(
        std::is_copy_constructible_v<Iter>,
        "rusty::cycle requires a copyable base iterator (to restart it)"
    );

    explicit cycle_next_iter(Iter iter) : start_(iter), iter_(std::move(iter)) {}

    cycle_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = next_item_t<Iter>;
        using next_result = rusty::Option<item_type>;

        if (exhausted_) {
            return next_result(rusty::None);
        }
        auto item = iter_.next();
        if (option_like_has_value(item)) {
            return next_result(option_like_take_value(item));
        }
        iter_ = start_;
        auto again = iter_.next();
        if (!option_like_has_value(again)) {
            exhausted_ = true;
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(again));
    }

private:
    Iter start_;
    Iter iter_;
    bool exhausted_ = false;
};

template<typename Iter>
auto make_cycle_next_iter(Iter&& iter) {
    return cycle_next_iter<std::remove_reference_t<Iter>>(
        std::forward<Iter>(iter));
}

// Adapts a member-begin()/end() VIEW (zip_view: non-const begin, sentinel
// end) to the option-like next() protocol so the adapter/collect pipeline
// composes over it. The view is heap-pinned: its iterators point INTO the
// stored view (zip_view holds its sides by value), so moving the adapter
// must not relocate the view.
template<typename View>
class view_next_iter {
public:
    explicit view_next_iter(View view)
        : view_(std::make_shared<View>(std::move(view))),
          it_(view_->begin()),
          end_(view_->end()) {}

    view_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = std::decay_t<decltype(*it_)>;
        using next_result = rusty::Option<item_type>;

        if (!(it_ != end_)) {
            return next_result(rusty::None);
        }
        item_type value = *it_;
        ++it_;
        return next_result(std::move(value));
    }

private:
    std::shared_ptr<View> view_;
    decltype(std::declval<View&>().begin()) it_;
    decltype(std::declval<View&>().end()) end_;
};

template<typename View>
auto make_view_next_iter(View&& view) {
    return view_next_iter<std::remove_cvref_t<View>>(
        std::forward<View>(view));
}

// Reverses a FORWARD-ONLY next() iterator by materializing it. Rust's
// `.rev()` needs DoubleEndedIterator, and sources like zip-over-views are
// double-ended in Rust while the C++ adapters are forward-only — replaying
// a drained buffer backward preserves the observable sequence at a cost
// bounded by its length.
template<typename Iter>
class materialized_rev_next_iter {
public:
    static_assert(
        has_option_like_next_v<Iter>,
        "rusty::rev requires next() to return an Option/optional-like value"
    );
    static_assert(
        !std::is_reference_v<next_item_t<Iter>>,
        "materialized rev supports by-value items only"
    );

    explicit materialized_rev_next_iter(Iter iter) {
        auto source = std::move(iter);
        while (true) {
            auto item = source.next();
            if (!option_like_has_value(item)) {
                break;
            }
            items_.push_back(option_like_take_value(item));
        }
    }

    materialized_rev_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = next_item_t<Iter>;
        using next_result = rusty::Option<item_type>;

        if (items_.empty()) {
            return next_result(rusty::None);
        }
        item_type value = std::move(items_.back());
        items_.pop_back();
        return next_result(std::move(value));
    }

private:
    std::vector<next_item_t<Iter>> items_;
};

template<typename Iter>
auto make_materialized_rev_next_iter(Iter&& iter) {
    return materialized_rev_next_iter<std::remove_reference_t<Iter>>(
        std::forward<Iter>(iter));
}

template<typename Iter>
class take_next_iter;

template<typename Iter>
auto make_take_next_iter(Iter&& iter, size_t remaining);

template<typename T>
class repeat_next_iter {
public:
    using value_type = std::decay_t<T>;

    explicit repeat_next_iter(value_type value) : value_(std::move(value)) {}

    repeat_next_iter into_iter() const {
        return *this;
    }

    rusty::Option<value_type> next() {
        return rusty::Option<value_type>(clone_value(value_));
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        return std::make_tuple(
            std::numeric_limits<size_t>::max(),
            rusty::Option<size_t>(rusty::None));
    }

private:
    static value_type clone_value(const value_type& value) {
        if constexpr (requires { value.clone(); }) {
            return value.clone();
        } else {
            static_assert(
                std::is_copy_constructible_v<value_type>,
                "rusty::repeat requires copy-constructible or clone() values");
            return value;
        }
    }

    value_type value_;
};

template<typename F>
class repeat_with_next_iter {
public:
    using func_type = std::decay_t<F>;
    using value_type = std::invoke_result_t<func_type&>;

    explicit repeat_with_next_iter(func_type func) : func_(std::move(func)) {}

    repeat_with_next_iter into_iter() {
        return std::move(*this);
    }

    rusty::Option<value_type> next() {
        return rusty::Option<value_type>(func_());
    }

    auto take(size_t remaining) & {
        return make_take_next_iter(*this, remaining);
    }

    auto take(size_t remaining) && {
        return make_take_next_iter(std::move(*this), remaining);
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        return std::make_tuple(
            std::numeric_limits<size_t>::max(),
            rusty::Option<size_t>(rusty::None));
    }

private:
    func_type func_;
};

template<typename Iter>
class take_next_iter {
public:
    static_assert(
        has_option_like_next_v<std::remove_reference_t<Iter>>,
        "rusty::take requires next() to return an Option/optional-like value"
    );

    take_next_iter(Iter iter, size_t remaining)
        : iter_(std::forward<Iter>(iter)), remaining_(remaining) {}

    take_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using iter_type = std::remove_reference_t<Iter>;
        using item_type = next_item_t<iter_type>;
        using next_result = rusty::Option<item_type>;
        if (remaining_ == 0) {
            return next_result(rusty::None);
        }
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            remaining_ = 0;
            return next_result(rusty::None);
        }
        --remaining_;
        return next_result(option_like_take_value(item));
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        if (remaining_ == 0) {
            return std::make_tuple(0, rusty::Option<size_t>(0));
        }

        if constexpr (requires { iter_.size_hint(); }) {
            auto hint = iter_.size_hint();
            const size_t lower_bound = std::min(hint_lower_bound(hint), remaining_);
            auto upper_bound = hint_upper_bound(hint);
            if (upper_bound.is_some()) {
                const size_t bounded_upper =
                    std::min(static_cast<size_t>(upper_bound.unwrap()), remaining_);
                return std::make_tuple(lower_bound, rusty::Option<size_t>(bounded_upper));
            }
            return std::make_tuple(lower_bound, rusty::Option<size_t>(remaining_));
        }

        if constexpr (requires { iter_.count(); }) {
            const size_t bounded_count =
                std::min(static_cast<size_t>(iter_.count()), remaining_);
            return std::make_tuple(bounded_count, rusty::Option<size_t>(bounded_count));
        }

        return std::make_tuple(0, rusty::Option<size_t>(remaining_));
    }

private:
    template<typename Hint>
    static size_t hint_lower_bound(const Hint& hint) {
        if constexpr (requires { std::get<0>(hint); }) {
            return static_cast<size_t>(std::get<0>(hint));
        } else if constexpr (requires { hint._0; }) {
            return static_cast<size_t>(hint._0);
        } else {
            static_assert(
                dependent_false_v<Hint>,
                "rusty::take::size_hint requires lower-bound tuple-like access");
            return 0;
        }
    }

    template<typename Upper>
    static rusty::Option<size_t> normalize_upper_bound(const Upper& upper) {
        if constexpr (requires { upper.is_some(); upper.unwrap(); }) {
            if (upper.is_some()) {
                return rusty::Option<size_t>(static_cast<size_t>(upper.unwrap()));
            }
        } else if constexpr (requires { upper.has_value(); upper.value(); }) {
            if (upper.has_value()) {
                return rusty::Option<size_t>(static_cast<size_t>(upper.value()));
            }
        } else if constexpr (requires { static_cast<bool>(upper); *upper; }) {
            if (static_cast<bool>(upper)) {
                return rusty::Option<size_t>(static_cast<size_t>(*upper));
            }
        }
        return rusty::Option<size_t>(rusty::None);
    }

    template<typename Hint>
    static rusty::Option<size_t> hint_upper_bound(const Hint& hint) {
        if constexpr (requires { std::get<1>(hint); }) {
            return normalize_upper_bound(std::get<1>(hint));
        } else if constexpr (requires { hint._1; }) {
            return normalize_upper_bound(hint._1);
        } else {
            static_assert(
                dependent_false_v<Hint>,
                "rusty::take::size_hint requires upper-bound tuple-like access");
            return rusty::Option<size_t>(rusty::None);
        }
    }

    Iter iter_;
    size_t remaining_;
};

template<typename Iter>
class skip_next_iter {
public:
    static_assert(
        has_option_like_next_v<std::remove_reference_t<Iter>>,
        "rusty::skip requires next() to return an Option/optional-like value"
    );

    skip_next_iter(Iter iter, size_t remaining)
        : iter_(std::forward<Iter>(iter)), remaining_(remaining) {}

    // `rusty::clone(skip_iter)` must produce an independently-owning
    // copy of the underlying iterator — Rust's `Skip::clone` recurses
    // into the wrapped iterator's `Clone`. The implicit copy constructor
    // does a shallow member-wise copy which, for iterators owning heap
    // storage (e.g. SmallVec::into_iter holding a `SmallVec<A>` by
    // value), aliases the same buffer pointer and triggers a double-free
    // when both copies are destroyed. Surfaced by smallvec
    // `test_into_iter_clone_partially_consumed_iterator`. Restrict to
    // iterators that themselves expose a `.clone()` member so trivially
    // copyable wrappers stay copy-elided through the implicit ctor.
    skip_next_iter clone() const
        requires requires(const std::remove_reference_t<Iter>& it) {
            { it.clone() } -> std::same_as<std::remove_reference_t<Iter>>;
        }
    {
        return skip_next_iter(iter_.clone(), remaining_);
    }

    skip_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using iter_type = std::remove_reference_t<Iter>;
        using item_type = next_item_t<iter_type>;
        using next_result = rusty::Option<item_type>;

        while (remaining_ > 0) {
            auto skipped = iter_.next();
            if (!option_like_has_value(skipped)) {
                remaining_ = 0;
                return next_result(rusty::None);
            }
            --remaining_;
        }
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

private:
    Iter iter_;
    size_t remaining_;
};

template<typename LeftIter, typename RightIter>
class chain_next_iter {
public:
    static_assert(
        has_option_like_next_v<std::remove_reference_t<LeftIter>>,
        "rusty::chain requires left iterator next() to return Option/optional-like value");
    static_assert(
        has_option_like_next_v<std::remove_reference_t<RightIter>>,
        "rusty::chain requires right iterator next() to return Option/optional-like value");

    chain_next_iter(LeftIter left, RightIter right)
        : left_(std::forward<LeftIter>(left)),
          right_(std::forward<RightIter>(right)),
          left_done_(false) {}

    chain_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        // Probe the UNDECAYED payload (next_item_t decays by definition):
        // lvalue-reference items stay references — Rust's Chain over
        // ref-yielding iterators yields refs, and rusty::Option<T&> is
        // pointer-backed so returning it is safe. Decaying them mismatched
        // emitted adapter wrappers whose declared Item is `const T&`.
        using left_item_raw = decltype(option_like_take_value(
            std::declval<next_result_t<std::remove_reference_t<LeftIter>>&>()));
        using item_type = std::conditional_t<
            std::is_lvalue_reference_v<left_item_raw>,
            left_item_raw,
            std::decay_t<left_item_raw>>;
        using next_result = rusty::Option<item_type>;

        if (!left_done_) {
            auto item = left_.next();
            if (option_like_has_value(item)) {
                return next_result(option_like_take_value(item));
            }
            left_done_ = true;
        }

        auto item = right_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

    // DoubleEnded support (Rust's Chain is DoubleEndedIterator when both
    // halves are): drain the RIGHT half from the back first, then the
    // left. Only instantiated when both inners have next_back —
    // chain().enumerate().rev() hard-errored without it.
    auto next_back()
        requires requires {
            std::declval<std::remove_reference_t<LeftIter>&>().next_back();
            std::declval<std::remove_reference_t<RightIter>&>().next_back();
        }
    {
        using left_item_raw = decltype(option_like_take_value(
            std::declval<next_result_t<std::remove_reference_t<LeftIter>>&>()));
        using item_type = std::conditional_t<
            std::is_lvalue_reference_v<left_item_raw>,
            left_item_raw,
            std::decay_t<left_item_raw>>;
        using next_result = rusty::Option<item_type>;

        if (!right_done_back_) {
            auto item = right_.next_back();
            if (option_like_has_value(item)) {
                return next_result(option_like_take_value(item));
            }
            right_done_back_ = true;
        }
        auto item = left_.next_back();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        if constexpr (requires { left_.size_hint(); right_.size_hint(); }) {
            auto left_hint = left_.size_hint();
            auto right_hint = right_.size_hint();
            const auto lower =
                static_cast<size_t>(std::get<0>(left_hint))
                + static_cast<size_t>(std::get<0>(right_hint));
            auto upper = rusty::Option<size_t>(rusty::None);
            auto left_upper = std::get<1>(left_hint);
            auto right_upper = std::get<1>(right_hint);
            if (left_upper.is_some() && right_upper.is_some()) {
                upper = rusty::Option<size_t>(
                    static_cast<size_t>(left_upper.unwrap())
                    + static_cast<size_t>(right_upper.unwrap()));
            }
            return std::make_tuple(lower, upper);
        }
        return std::make_tuple(0, rusty::Option<size_t>(rusty::None));
    }

private:
    LeftIter left_;
    RightIter right_;
    bool left_done_;
    bool right_done_back_ = false;
};

template<typename Range>
class range_begin_end_next_iter {
public:
    using range_type = std::remove_reference_t<Range>;
    using iter_type = decltype(std::begin(std::declval<range_type&>()));
    using sentinel_type = decltype(std::end(std::declval<range_type&>()));
    using item_type = std::decay_t<decltype(*std::declval<iter_type&>())>;
    using next_result = rusty::Option<item_type>;

    explicit range_begin_end_next_iter(Range range)
        : range_(std::forward<Range>(range)),
          current_(std::begin(range_)),
          end_(std::end(range_)) {}

    range_begin_end_next_iter into_iter() {
        return std::move(*this);
    }

    next_result next() {
        if (current_ == end_) {
            return next_result(rusty::None);
        }
        auto value = item_type(*current_);
        ++current_;
        return next_result(std::move(value));
    }

private:
    Range range_;
    iter_type current_;
    sentinel_type end_;
};

struct filter_size_hint {
    size_t _0;
    rusty::Option<size_t> _1;
};

template<typename Iter, typename Pred>
class filter_next_iter {
public:
    static_assert(
        has_option_like_next_v<std::remove_reference_t<Iter>>,
        "rusty::filter requires next() to return an Option/optional-like value"
    );

    filter_next_iter(Iter iter, Pred pred)
        : iter_(std::forward<Iter>(iter)), pred_(std::move(pred)) {}

    filter_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using iter_type = std::remove_reference_t<Iter>;
        using item_type = next_item_t<iter_type>;
        using next_result = rusty::Option<item_type>;

        while (true) {
            auto item = iter_.next();
            if (!option_like_has_value(item)) {
                return next_result(rusty::None);
            }
            item_type candidate = option_like_take_value(item);
            if (std::invoke(pred_, deref_if_pointer(candidate))) {
                return next_result(std::move(candidate));
            }
        }
    }

    filter_size_hint size_hint() const {
        if constexpr (requires(const std::remove_reference_t<Iter>& iter) { iter.count(); }) {
            return filter_size_hint{
                0,
                rusty::Option<size_t>(static_cast<size_t>(iter_.count()))
            };
        } else {
            return filter_size_hint{0, rusty::None};
        }
    }

private:
    Iter iter_;
    Pred pred_;
};

template<typename Iter, typename State, typename Func>
class scan_next_iter {
public:
    static_assert(
        has_option_like_next_v<std::remove_reference_t<Iter>>,
        "rusty::scan requires next() to return an Option/optional-like value"
    );

    scan_next_iter(Iter iter, State state, Func func)
        : iter_(std::forward<Iter>(iter)),
          state_(std::move(state)),
          func_(std::move(func)),
          done_(false) {}

    scan_next_iter into_iter() {
        return std::move(*this);
    }

    auto next() {
        using item_type = next_item_t<std::remove_reference_t<Iter>>;
        using scan_result = std::decay_t<decltype(std::invoke(
            std::declval<Func&>(),
            std::declval<State&>(),
            deref_if_pointer(std::declval<item_type>())))>;
        using scan_item_type =
            std::decay_t<decltype(option_like_take_value(std::declval<scan_result&>()))>;
        using next_result = rusty::Option<scan_item_type>;
        static_assert(
            is_option_like_next_result_v<scan_result>,
            "rusty::scan closure must return an Option/optional-like value"
        );

        if (done_) {
            return next_result(rusty::None);
        }

        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }

        auto scanned = std::invoke(
            func_,
            state_,
            deref_if_pointer(option_like_take_value(item)));
        if (!option_like_has_value(scanned)) {
            done_ = true;
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(scanned));
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        if (done_) {
            return std::make_tuple(0, rusty::Option<size_t>(0));
        }

        if constexpr (requires { iter_.size_hint(); }) {
            auto hint = iter_.size_hint();
            auto upper = std::get<1>(hint);

            if constexpr (requires { upper.is_some(); upper.unwrap(); }) {
                if (upper.is_some()) {
                    return std::make_tuple(
                        0,
                        rusty::Option<size_t>(static_cast<size_t>(upper.unwrap())));
                }
            } else if constexpr (requires { upper.has_value(); upper.value(); }) {
                if (upper.has_value()) {
                    return std::make_tuple(
                        0,
                        rusty::Option<size_t>(static_cast<size_t>(upper.value())));
                }
            }
            return std::make_tuple(0, rusty::Option<size_t>(rusty::None));
        }

        if constexpr (requires { iter_.count(); }) {
            return std::make_tuple(
                0,
                rusty::Option<size_t>(static_cast<size_t>(iter_.count())));
        }

        return std::make_tuple(0, rusty::Option<size_t>(rusty::None));
    }

private:
    Iter iter_;
    State state_;
    Func func_;
    bool done_;
};

template<typename NextIter>
auto make_next_iter_range(NextIter&& iter) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<NextIter>, NextIter, std::decay_t<NextIter>>;
    return next_iter_range<stored_iter>(std::forward<NextIter>(iter));
}

template<typename Iter, typename Func>
auto make_map_next_iter(Iter&& iter, Func&& func) {
    using stored_iter = std::decay_t<Iter>;
    using stored_func = std::decay_t<Func>;
    return map_next_iter<stored_iter, stored_func>(
        std::forward<Iter>(iter),
        std::forward<Func>(func));
}

template<typename Iter>
auto make_enumerate_next_iter(Iter&& iter) {
    using stored_iter = std::decay_t<Iter>;
    return enumerate_next_iter<stored_iter>(std::forward<Iter>(iter));
}

template<typename Iter>
auto make_rev_next_iter(Iter&& iter) {
    using stored_iter = std::decay_t<Iter>;
    return rev_next_iter<stored_iter>(std::forward<Iter>(iter));
}

template<typename Iter>
auto make_take_next_iter(Iter&& iter, size_t remaining) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<Iter>, Iter, std::decay_t<Iter>>;
    return take_next_iter<stored_iter>(std::forward<Iter>(iter), remaining);
}

// Lazy `Iterator::step_by(n)`: yields the first item, then every n-th after it.
template<typename Iter>
class step_by_next_iter {
public:
    static_assert(
        has_option_like_next_v<std::remove_reference_t<Iter>>,
        "rusty::step_by requires next() to return an Option/optional-like value");

    step_by_next_iter(Iter iter, size_t step)
        : iter_(std::forward<Iter>(iter)), step_(step == 0 ? 1 : step) {}

    step_by_next_iter into_iter() { return std::move(*this); }

    auto next() {
        using iter_type = std::remove_reference_t<Iter>;
        using item_type = next_item_t<iter_type>;
        using next_result = rusty::Option<item_type>;
        if (!first_) {
            for (size_t i = 1; i < step_; ++i) {
                auto skip = iter_.next();
                if (!option_like_has_value(skip)) {
                    return next_result(rusty::None);
                }
            }
        }
        first_ = false;
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(option_like_take_value(item));
    }

private:
    Iter iter_;
    size_t step_;
    bool first_ = true;
};

template<typename Iter>
auto make_step_by_next_iter(Iter&& iter, size_t step) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<Iter>, Iter, std::decay_t<Iter>>;
    return step_by_next_iter<stored_iter>(std::forward<Iter>(iter), step);
}

template<typename Iter>
auto make_skip_next_iter(Iter&& iter, size_t remaining) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<Iter>, Iter, std::decay_t<Iter>>;
    return skip_next_iter<stored_iter>(std::forward<Iter>(iter), remaining);
}

template<typename Iter, typename Pred>
auto make_filter_next_iter(Iter&& iter, Pred&& pred) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<Iter>, Iter, std::decay_t<Iter>>;
    using stored_pred = std::decay_t<Pred>;
    return filter_next_iter<stored_iter, stored_pred>(
        std::forward<Iter>(iter),
        std::forward<Pred>(pred));
}

template<typename Range>
auto make_begin_end_next_iter(Range&& range) {
    using stored_range =
        std::conditional_t<std::is_lvalue_reference_v<Range>, Range, std::decay_t<Range>>;
    return range_begin_end_next_iter<stored_range>(std::forward<Range>(range));
}

template<typename LeftIter, typename RightIter>
auto make_chain_next_iter(LeftIter&& left, RightIter&& right) {
    using stored_left =
        std::conditional_t<std::is_lvalue_reference_v<LeftIter>, LeftIter, std::decay_t<LeftIter>>;
    using stored_right = std::conditional_t<
        std::is_lvalue_reference_v<RightIter>,
        RightIter,
        std::decay_t<RightIter>>;
    return chain_next_iter<stored_left, stored_right>(
        std::forward<LeftIter>(left),
        std::forward<RightIter>(right));
}

template<typename Iter, typename State, typename Func>
auto make_scan_next_iter(Iter&& iter, State&& state, Func&& func) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<Iter>, Iter, std::decay_t<Iter>>;
    using stored_state = std::decay_t<State>;
    using stored_func = std::decay_t<Func>;
    return scan_next_iter<stored_iter, stored_state, stored_func>(
        std::forward<Iter>(iter),
        std::forward<State>(state),
        std::forward<Func>(func));
}

template<typename T>
auto make_repeat_next_iter(T&& value) {
    using stored_value = std::decay_t<T>;
    return repeat_next_iter<stored_value>(std::forward<T>(value));
}

template<typename F>
auto make_repeat_with_next_iter(F&& func) {
    using stored_func = std::decay_t<F>;
    return repeat_with_next_iter<stored_func>(std::forward<F>(func));
}

template<typename Range>
decltype(auto) preserve_for_in_range(Range&& range) {
    if constexpr (std::is_lvalue_reference_v<Range>) {
        return (range);
    } else {
        return std::decay_t<Range>(std::forward<Range>(range));
    }
}

// `make_entry_probe(map, key)` mimics Rust's `map.entry(key)` API for the
// HashMap/BTreeMap `if let Entry::Vacant(entry) = used.entry(v) { … }`
// pattern that itertools' `unique_impl` and `kmerge` emit.
//
// In Rust the `Entry` is an enum with `Vacant(VacantEntry)` and
// `Occupied(OccupiedEntry)` variants. We model it with a single probe
// type that exposes `is_vacant()` / `is_occupied()` and corresponding
// `vacant_entry()` / `occupied_entry()` accessors. The transpiler emits:
//
//   if (auto&& probe = make_entry_probe(used, std::move(v)); probe.is_vacant()) {
//       auto&& entry = probe.vacant_entry();
//       auto elt = rusty::clone(entry.key());
//       entry.insert(rusty::Unit{});
//       …
//   }
//
// Backing storage: a `try_emplace` on the map. If the key wasn't present
// the map gets a default-constructed value at that slot; the `VacantEntry`
// then provides `key()` (reference to the stored key) and `insert(value)`
// (overwrites that slot's value). Move-only / non-default-constructible
// values are supported via `insert`.
template<typename Map>
class vacant_entry_probe {
public:
    using map_type = Map;
    using iterator = typename Map::iterator;
    using key_type = typename Map::key_type;
    using mapped_type = typename Map::mapped_type;

    vacant_entry_probe(Map& m, iterator it) : map_(&m), it_(it) {}

    const key_type& key() const { return it_->first; }

    template<typename V>
    void insert(V&& value) {
        it_->second = std::forward<V>(value);
    }

private:
    Map* map_;
    iterator it_;
};

template<typename Map>
class entry_probe {
public:
    using map_type = Map;
    using iterator = typename Map::iterator;

    entry_probe(Map& m, iterator it, bool vacant)
        : map_(&m), it_(it), vacant_(vacant) {}

    bool is_vacant() const { return vacant_; }
    bool is_occupied() const { return !vacant_; }

    vacant_entry_probe<Map> vacant_entry() const {
        return vacant_entry_probe<Map>(*map_, it_);
    }

    // Occupied accessor — currently a no-op for the itertools call sites
    // (they only branch on `is_vacant`). Returns a reference for parity
    // with the Vacant form.
    auto& occupied_entry() const { return *it_; }

private:
    Map* map_;
    iterator it_;
    bool vacant_;
};

template<typename Map, typename Key>
entry_probe<Map> make_entry_probe(Map& map, Key&& key) {
    using mapped_type = typename Map::mapped_type;
    auto result = map.try_emplace(std::forward<Key>(key), mapped_type{});
    return entry_probe<Map>(map, result.first, result.second);
}
} // namespace detail

template<typename Range>
decltype(auto) iter(Range&& range) {
    // Walk the receiver's deref chain looking for `.iter()`. Replaces the
    // old hand-rolled cooperation between a direct `.iter()` arm and a
    // separate `*r` recursion arm — `rusty::deref_call` now handles both
    // in one step. See rusty-std-book §6.11 for the universal-dispatcher
    // design. Subsequent arms remain to adapt receivers that have no
    // `.iter()` anywhere in their chain (raw `data()`/`size()`, Rust-shape
    // `begin()` returning a pointer, STL `std::begin`/`std::end`).
    if constexpr (requires {
        rusty::deref_call(std::forward<Range>(range),
            [](auto&& __r) -> decltype(__r.iter()) { return __r.iter(); });
    }) {
        return rusty::deref_call(std::forward<Range>(range),
            [](auto&& __r) -> decltype(__r.iter()) { return __r.iter(); });
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::preserve_for_in_range(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).data(); std::forward<Range>(range).size(); }) {
        auto&& view = std::forward<Range>(range);
        using view_type = std::remove_reference_t<decltype(view)>;
        using elem_ptr = decltype(view.data());
        using raw_elem_type = std::remove_pointer_t<elem_ptr>;
        using elem_type =
            std::conditional_t<std::is_const_v<view_type>, std::add_const_t<raw_elem_type>, raw_elem_type>;
        auto* data = view.data();
        return slice_iter::Iter<elem_type>(data, data + view.size());
    } else if constexpr (
        requires {
            std::forward<Range>(range).begin();
            std::forward<Range>(range).size();
            requires std::is_pointer_v<
                decltype(std::forward<Range>(range).begin())>;
        }
    ) {
        // Rust-style containers (notably vec_port::Vec) expose
        // `begin()` returning a `T*` plus `size()`, but not `data()`.
        // Treat them as contiguous ranges so callers get a
        // `slice_iter::Iter` rather than the container itself —
        // matches the `data()`/`size()` arm above.
        auto&& view = std::forward<Range>(range);
        auto* data = view.begin();
        using elem_type = std::remove_pointer_t<decltype(data)>;
        return slice_iter::Iter<elem_type>(data, data + view.size());
    } else if constexpr (requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }) {
        return std::forward<Range>(range);
    } else if constexpr (requires {
        std::forward<Range>(range).begin();
        std::forward<Range>(range).end();
    }) {
        // Member-begin/end view whose begin() is NON-const (zip_view), passed
        // by value or mutable ref — std::begin can't see it. Adapt to the
        // next() protocol so map/take/from_iter compose over it.
        return detail::make_view_next_iter(std::forward<Range>(range));
    } else if constexpr (
        std::is_copy_constructible_v<std::remove_cvref_t<Range>>
        && requires(std::remove_cvref_t<Range>& v) { v.begin(); v.end(); }
    ) {
        // Same view behind const: iterate a copy (Rust iteration consumes
        // the iterator by value).
        return detail::make_view_next_iter(std::remove_cvref_t<Range>(range));
    } else if constexpr (requires { *std::forward<Range>(range); }) {
        return iter(*std::forward<Range>(range));
    } else {
        static_assert(
            detail::dependent_false_v<Range>,
            "rusty::iter requires iter(), option-like next(), data()/size(), or dereferenceable receiver"
        );
    }
}

template<typename T>
auto repeat(T&& value) {
    return detail::make_repeat_next_iter(std::forward<T>(value));
}

template<typename F>
auto repeat_with(F&& func) {
    return detail::make_repeat_with_next_iter(std::forward<F>(func));
}

template<typename T>
struct empty_iter {
    using Item = T;

    empty_iter into_iter() const { return *this; }

    rusty::Option<T> next() { return rusty::None; }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        return std::make_tuple(0u, rusty::Option<size_t>(0u));
    }
};

template<typename T>
empty_iter<T> empty() {
    return empty_iter<T>{};
}

template<typename T>
struct once_iter {
    using Item = T;

    explicit once_iter(T value) : value_(std::move(value)) {}

    once_iter into_iter() const { return *this; }

    rusty::Option<T> next() {
        if (!value_.is_some()) {
            return rusty::None;
        }
        return rusty::Option<T>(value_.take().unwrap());
    }

    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        const auto remaining = value_.is_some() ? static_cast<size_t>(1) : static_cast<size_t>(0);
        return std::make_tuple(remaining, rusty::Option<size_t>(remaining));
    }

private:
    rusty::Option<T> value_;
};

template<typename T>
once_iter<std::decay_t<T>> once(T&& value) {
    return once_iter<std::decay_t<T>>(std::forward<T>(value));
}

// Rust `iter::successors(first, f)` — yields `first`, then repeatedly
// f(&prev) until it returns None.
template<typename T, typename F>
struct successors_iter {
    using Item = T;

    rusty::Option<T> current_;
    F func_;

    successors_iter into_iter() { return std::move(*this); }

    rusty::Option<T> next() {
        if (!current_.is_some()) {
            return rusty::None;
        }
        T value = current_.take().unwrap();
        current_ = func_(value);
        return rusty::Option<T>(std::move(value));
    }
};

template<typename T, typename F>
auto successors(rusty::Option<T> first, F&& func) {
    return successors_iter<T, std::remove_reference_t<F>>{
        std::move(first), std::forward<F>(func)};
}

template<typename Range>
decltype(auto) iter_mut(Range&& range) {
    // Mirror the `rusty::iter` change: walk the deref chain for `.iter_mut()`
    // via `rusty::deref_call`. The remaining arms (as_mut_slice, deref_mut,
    // data+size, std::begin/end, *r recursion) handle receivers that have
    // no `.iter_mut()` in their chain — those still walk through `*r` for
    // wrapped-container cases. See rusty-std-book §6.11.
    //
    // A next-protocol receiver IS the iterator: Rust `for x in &mut it`
    // consumes `it` by reference (`impl Iterator for &mut I`) — the items
    // keep their own mutability, there is nothing to "mut". Pass through.
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return std::forward<Range>(range);
    } else if constexpr (requires {
        rusty::deref_call(std::forward<Range>(range),
            [](auto&& __r) -> decltype(__r.iter_mut()) { return __r.iter_mut(); });
    }) {
        return rusty::deref_call(std::forward<Range>(range),
            [](auto&& __r) -> decltype(__r.iter_mut()) { return __r.iter_mut(); });
    } else if constexpr (requires { std::forward<Range>(range).as_mut_slice(); }) {
        return iter_mut(std::forward<Range>(range).as_mut_slice());
    } else if constexpr (requires { std::forward<Range>(range).deref_mut(); }) {
        return iter_mut(std::forward<Range>(range).deref_mut());
    } else if constexpr (requires { std::forward<Range>(range).data(); std::forward<Range>(range).size(); }) {
        auto&& view = std::forward<Range>(range);
        using elem_ptr = decltype(view.data());
        using elem_type = std::remove_pointer_t<elem_ptr>;
        static_assert(
            !std::is_const_v<elem_type>,
            "rusty::iter_mut requires mutable element access"
        );
        auto* data = view.data();
        return slice_iter::Iter<elem_type>(data, data + view.size());
    } else if constexpr (requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }) {
        using iter_ref = decltype(*std::begin(std::forward<Range>(range)));
        static_assert(
            !std::is_const_v<std::remove_reference_t<iter_ref>>,
            "rusty::iter_mut requires mutable iterator items"
        );
        return std::forward<Range>(range);
    } else if constexpr (requires { *std::forward<Range>(range); }) {
        return iter_mut(*std::forward<Range>(range));
    } else {
        static_assert(
            detail::dependent_false_v<Range>,
            "rusty::iter_mut requires iter_mut(), mutable data()/size(), or dereferenceable receiver"
        );
    }
}

template<typename Range>
decltype(auto) for_in(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_next_iter_range(std::forward<Range>(range));
    } else if constexpr (
        std::is_const_v<std::remove_reference_t<Range>>
        && detail::has_option_like_next_v<std::remove_cvref_t<Range>>
        && std::is_copy_constructible_v<std::remove_cvref_t<Range>>) {
        // A CONST binding of a next-protocol adapter (emitted
        // `const auto iter = …` later consumed by iteration): Rust iteration
        // takes the iterator by value, so iterate a copy — next() can't
        // advance through const.
        return detail::make_next_iter_range(std::remove_cvref_t<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::for_in requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return for_in(std::forward<Range>(range).into_iter());
    } else if constexpr (
        requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }
    ) {
        return detail::preserve_for_in_range(std::forward<Range>(range));
    } else if constexpr (
        requires { std::forward<Range>(range).iter(); }
        || requires { std::forward<Range>(range).data(); std::forward<Range>(range).size(); }
        || requires { *std::forward<Range>(range); }
    ) {
        return for_in(iter(std::forward<Range>(range)));
    } else {
        return detail::preserve_for_in_range(std::forward<Range>(range));
    }
}

template<typename Range, typename Func>
decltype(auto) map(Range&& range, Func&& func) {
    if constexpr (
        requires {
            std::forward<Range>(range).is_some();
            std::forward<Range>(range).map(std::forward<Func>(func));
        })
    {
        return std::forward<Range>(range).map(std::forward<Func>(func));
    } else if constexpr (detail::is_std_array_v<std::remove_cvref_t<Range>>) {
        // Rust ARRAY::map ([T; N] -> [U; N]) is not Iterator::map — produce
        // a std::array of the mapped elements (moving each source element).
        using Arr = std::remove_cvref_t<Range>;
        using Elem = decltype(func(std::declval<typename Arr::value_type>()));
        return [&]<std::size_t... Is>(std::index_sequence<Is...>) {
            return std::array<Elem, std::tuple_size_v<Arr>>{
                func(std::move(std::get<Is>(range)))...};
        }(std::make_index_sequence<std::tuple_size_v<Arr>>{});
    } else
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_map_next_iter(
            std::forward<Range>(range),
            std::forward<Func>(func));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::map requires next() to return an Option/optional-like value"
        );
    } else if constexpr (
        detail::is_std_array_like_v<std::remove_cv_t<std::remove_reference_t<Range>>>)
    {
        auto&& range_ref = std::forward<Range>(range);
        auto mapper = std::forward<Func>(func);
        using range_type = std::remove_cv_t<std::remove_reference_t<Range>>;
        constexpr std::size_t N = std::tuple_size_v<range_type>;
        using item_ref = decltype(std::declval<range_type&>()[0]);
        using mapped_type = std::decay_t<decltype(std::invoke(
            mapper,
            detail::deref_if_pointer(std::declval<item_ref>())))>;
        return [&]<std::size_t... I>(std::index_sequence<I...>) {
            return std::array<mapped_type, N>{
                std::invoke(mapper, detail::deref_if_pointer(range_ref[I]))...};
        }(std::make_index_sequence<N>{});
    } else if constexpr (
        requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }
    ) {
        auto&& range_ref = std::forward<Range>(range);
        auto mapper = std::forward<Func>(func);
        using item_ref = decltype(*std::begin(range_ref));
        using mapped_type = std::decay_t<decltype(std::invoke(
            mapper,
            detail::deref_if_pointer(std::declval<item_ref>())))>;
        std::vector<mapped_type> out;
        for (auto&& item : for_in(range_ref)) {
            out.push_back(std::invoke(
                mapper,
                detail::deref_if_pointer(std::forward<decltype(item)>(item))));
        }
        return out;
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return map(
            std::forward<Range>(range).into_iter(),
            std::forward<Func>(func));
    } else {
        return detail::make_map_next_iter(
            iter(std::forward<Range>(range)),
            std::forward<Func>(func));
    }
}

template<typename Range, typename Pred>
decltype(auto) filter(Range&& range, Pred&& pred) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_filter_next_iter(
            std::forward<Range>(range),
            std::forward<Pred>(pred));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::filter requires next() to return an Option/optional-like value"
        );
    } else if constexpr (
        requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }
    ) {
        return detail::make_filter_next_iter(
            detail::make_begin_end_next_iter(std::forward<Range>(range)),
            std::forward<Pred>(pred));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return filter(
            std::forward<Range>(range).into_iter(),
            std::forward<Pred>(pred));
    } else {
        return filter(
            iter(std::forward<Range>(range)),
            std::forward<Pred>(pred));
    }
}

template<typename Range, typename Pred>
decltype(auto) find(Range&& range, Pred&& pred) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto iter = std::forward<Range>(range);
        using iter_type = std::remove_reference_t<decltype(iter)>;
        using next_result = detail::next_result_t<iter_type>;
        using item_type = detail::next_item_t<iter_type>;
        while (true) {
            auto item = iter.next();
            if (!detail::option_like_has_value(item)) {
                return next_result(None);
            }
            item_type candidate = detail::option_like_take_value(item);
            if (std::invoke(
                    pred,
                    detail::deref_if_pointer_like(std::as_const(candidate)))) {
                return next_result(std::move(candidate));
            }
        }
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return find(
            std::forward<Range>(range).into_iter(),
            std::forward<Pred>(pred));
    } else {
        return find(
            iter(std::forward<Range>(range)),
            std::forward<Pred>(pred));
    }
}

template<typename Range>
decltype(auto) enumerate(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_enumerate_next_iter(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::enumerate requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return enumerate(std::forward<Range>(range).into_iter());
    } else {
        return enumerate(iter(std::forward<Range>(range)));
    }
}

template<typename Range>
decltype(auto) rev(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        using iter_type = std::remove_reference_t<Range>;
        if constexpr (requires(iter_type& iter) { iter.next_back(); }) {
            return detail::make_rev_next_iter(std::forward<Range>(range));
        } else {
            // Forward-only adapter chain (zip over views): double-ended in
            // Rust when its sources are, but the C++ adapters aren't —
            // materialize and replay backward.
            return detail::make_materialized_rev_next_iter(std::forward<Range>(range));
        }
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::rev requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return rev(std::forward<Range>(range).into_iter());
    } else {
        return rev(iter(std::forward<Range>(range)));
    }
}

// Rust `Iterator::copied()` / `::cloned()`. The member spelling wins when
// the receiver already provides one (runtime iterator types like
// slice_iter::Iter keep their exact adapter types); the generic wrapper
// serves transpiled iterator types that only expose option-like next().
template<typename Range>
decltype(auto) copied(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).copied(); }) {
        return std::forward<Range>(range).copied();
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_copied_next_iter(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::copied requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return copied(std::forward<Range>(range).into_iter());
    } else {
        return copied(iter(std::forward<Range>(range)));
    }
}

template<typename Range>
decltype(auto) cloned(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).cloned(); }) {
        return std::forward<Range>(range).cloned();
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_cloned_next_iter(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::cloned requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return cloned(std::forward<Range>(range).into_iter());
    } else {
        return cloned(iter(std::forward<Range>(range)));
    }
}

// Rust `Iterator::cycle()`. The member spelling wins when the receiver
// provides one; the generic wrapper serves iterators that only expose
// option-like next().
template<typename Range>
decltype(auto) cycle(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).cycle(); }) {
        return std::forward<Range>(range).cycle();
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_cycle_next_iter(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::cycle requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return cycle(std::forward<Range>(range).into_iter());
    } else {
        return cycle(iter(std::forward<Range>(range)));
    }
}

template<typename Range>
decltype(auto) take(Range&& range, size_t remaining) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_take_next_iter(
            std::forward<Range>(range),
            remaining);
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::take requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return take(
            std::forward<Range>(range).into_iter(),
            remaining);
    } else {
        return take(
            iter(std::forward<Range>(range)),
            remaining);
    }
}

template<typename Range>
decltype(auto) skip(Range&& range, size_t remaining) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_skip_next_iter(
            std::forward<Range>(range),
            remaining);
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::skip requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return skip(
            std::forward<Range>(range).into_iter(),
            remaining);
    } else {
        return skip(
            iter(std::forward<Range>(range)),
            remaining);
    }
}

// `Iterator::flat_map(f)` — map each item to an iterable, flatten lazily: hold
// the current inner iterator, and advance the outer one whenever it drains.
template<typename Iter, typename Func>
class flat_map_next_iter {
    using OuterItem = detail::next_item_t<std::remove_reference_t<Iter>>;
    using InnerIterable = std::invoke_result_t<Func&, OuterItem>;
    using InnerIter = std::remove_cvref_t<decltype(iter(std::declval<InnerIterable>()))>;
    using ItemType = detail::next_item_t<std::remove_reference_t<InnerIter>>;

public:
    flat_map_next_iter(Iter it, Func func)
        : iter_(std::forward<Iter>(it)), func_(std::forward<Func>(func)) {}

    flat_map_next_iter into_iter() { return std::move(*this); }

    auto next() {
        using next_result = rusty::Option<ItemType>;
        for (;;) {
            if (inner_.has_value()) {
                auto item = inner_->next();
                if (detail::option_like_has_value(item)) {
                    return next_result(detail::option_like_take_value(item));
                }
                inner_.reset();
            }
            auto outer = iter_.next();
            if (!detail::option_like_has_value(outer)) {
                return next_result(rusty::None);
            }
            inner_.emplace(iter(func_(detail::option_like_take_value(outer))));
        }
    }

private:
    Iter iter_;
    Func func_;
    std::optional<InnerIter> inner_;
};

template<typename Iter, typename Func>
auto make_flat_map_next_iter(Iter&& it, Func&& func) {
    using stored_iter =
        std::conditional_t<std::is_lvalue_reference_v<Iter>, Iter, std::decay_t<Iter>>;
    return flat_map_next_iter<stored_iter, std::decay_t<Func>>(
        std::forward<Iter>(it), std::forward<Func>(func));
}

template<typename Range, typename Func>
decltype(auto) flat_map(Range&& range, Func&& func) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return make_flat_map_next_iter(std::forward<Range>(range), std::forward<Func>(func));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return flat_map(std::forward<Range>(range).into_iter(), std::forward<Func>(func));
    } else {
        return flat_map(iter(std::forward<Range>(range)), std::forward<Func>(func));
    }
}

// `Iterator::step_by(n)` — yields the first item then every n-th. (The range
// types don't carry their own `.step_by`, so the transpiler lowers it here.)
template<typename Range>
decltype(auto) step_by(Range&& range, size_t step) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_step_by_next_iter(std::forward<Range>(range), step);
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return step_by(std::forward<Range>(range).into_iter(), step);
    } else {
        return step_by(iter(std::forward<Range>(range)), step);
    }
}

template<typename Left, typename Right>
decltype(auto) chain(Left&& left, Right&& right) {
    if constexpr (
        detail::has_option_like_next_v<std::remove_reference_t<Left>>
        && detail::has_option_like_next_v<std::remove_reference_t<Right>>)
    {
        return detail::make_chain_next_iter(
            std::forward<Left>(left),
            std::forward<Right>(right));
    } else {
        return chain(
            iter(std::forward<Left>(left)),
            iter(std::forward<Right>(right)));
    }
}

// Named type for the chain adapter (Rust `core::iter::Chain<A, B>`). The chain
// LOGIC already exists as a factory (`chain()` / `detail::make_chain_next_iter`),
// but a factory has no spellable type — so a `Chain<A, B>` used as a STRUCT
// FIELD or RETURN type (e.g. hashbrown's `iter: Chain<Difference, Difference>`)
// had nothing to map to. Expose the name as exactly what `chain(a, b)` returns,
// so a field declared `Chain<A, B>` matches the value the constructor produces.
template<typename Left, typename Right>
using Chain = decltype(chain(std::declval<Left>(), std::declval<Right>()));

template<typename Range, typename State, typename Func>
decltype(auto) scan(Range&& range, State&& state, Func&& func) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_scan_next_iter(
            std::forward<Range>(range),
            std::forward<State>(state),
            std::forward<Func>(func));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::scan requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return scan(
            std::forward<Range>(range).into_iter(),
            std::forward<State>(state),
            std::forward<Func>(func));
    } else {
        return scan(
            iter(std::forward<Range>(range)),
            std::forward<State>(state),
            std::forward<Func>(func));
    }
}

template<typename Range, typename Func>
void for_each(Range&& range, Func&& func) {
    for (auto&& item : for_in(std::forward<Range>(range))) {
        std::invoke(func, std::forward<decltype(item)>(item));
    }
}

template<typename Range, typename Pred>
bool all(Range&& range, Pred&& pred) {
    for (auto&& item : for_in(std::forward<Range>(range))) {
        if (!std::invoke(pred, std::forward<decltype(item)>(item))) {
            return false;
        }
    }
    return true;
}

template<typename Range, typename Pred>
bool any(Range&& range, Pred&& pred) {
    for (auto&& item : for_in(std::forward<Range>(range))) {
        if (std::invoke(pred, std::forward<decltype(item)>(item))) {
            return true;
        }
    }
    return false;
}

template<typename Range>
size_t count(Range&& range) {
    size_t n = 0;
    for (auto&& _item : for_in(std::forward<Range>(range))) {
        (void)_item;
        ++n;
    }
    return n;
}

// `Iterator::sum()` — the additive identity (0) plus every item. Works over any
// rusty iterator/range via `for_in`; the transpiler lowers `.sum()` to this.
template<typename Range>
auto sum(Range&& range) {
    auto range_for = for_in(std::forward<Range>(range));
    using Item = std::remove_cvref_t<decltype(detail::deref_if_pointer_like(
        *std::begin(range_for)))>;
    Item acc{};
    for (auto&& item : range_for) {
        acc += detail::deref_if_pointer_like(item);
    }
    return acc;
}

namespace detail {
// One-element append for unzip targets: the rusty ports expose `push`
// (Vec, String); std containers expose `push_back`.
template<typename C, typename V>
void unzip_append(C& c, V&& v) {
    if constexpr (requires { c.push(std::forward<V>(v)); }) {
        c.push(std::forward<V>(v));
    } else {
        c.push_back(std::forward<V>(v));
    }
}
} // namespace detail

// `Iterator::unzip()` — split an iterator of pairs into two collections.
// The target types come from the surrounding let annotation; the
// transpiler lowers `.unzip()` to `rusty::unzip_collect<A, B>(it)`.
// Owned (non-pointer) items are moved out fieldwise — Rust's unzip
// consumes Item=(A, B) pairs, so nothing else aliases them.
template<typename A, typename B, typename Range>
std::tuple<A, B> unzip_collect(Range&& range) {
    A first{};
    B second{};
    auto range_for = for_in(std::forward<Range>(range));
    for (auto&& item : range_for) {
        auto&& pair = detail::deref_if_pointer_like(item);
        if constexpr (!std::is_pointer_v<std::remove_cvref_t<decltype(item)>>
                      && std::is_rvalue_reference_v<decltype(item)>) {
            detail::unzip_append(first, std::move(std::get<0>(pair)));
            detail::unzip_append(second, std::move(std::get<1>(pair)));
        } else {
            detail::unzip_append(first, std::get<0>(pair));
            detail::unzip_append(second, std::get<1>(pair));
        }
    }
    return {std::move(first), std::move(second)};
}

// `Iterator::product()` — the multiplicative identity (1) times every item.
// The transpiler lowers `.product()` to this (parallel to sum()).
template<typename Range>
auto product(Range&& range) {
    auto range_for = for_in(std::forward<Range>(range));
    using Item = std::remove_cvref_t<decltype(detail::deref_if_pointer_like(
        *std::begin(range_for)))>;
    Item acc = static_cast<Item>(1);
    for (auto&& item : range_for) {
        acc *= detail::deref_if_pointer_like(item);
    }
    return acc;
}

// Iterator terminals returning the item Option. The `iter_` prefix keeps
// them clear of the same-named container helpers (rusty::last(container),
// rusty::max(a, b)). Items are yielded exactly as next() produces them
// (pointers for slice iterators) so downstream .copied()/.unwrap() chains
// keep Rust's Option<&T> shape.

// Rust `Iterator::last()` — the final item.
template<typename Range>
auto iter_last(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto value = detail::option_like_take_value(item);
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            value = detail::option_like_take_value(nxt);
        }
        return next_result(std::move(value));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_last(std::forward<Range>(range).into_iter());
    } else {
        return iter_last(iter(std::forward<Range>(range)));
    }
}

// Rust `Iterator::nth(n)` — skip n items, then yield the next.
template<typename Range>
auto iter_nth(Range&& range, size_t n) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        for (size_t i = 0; i < n; ++i) {
            auto skipped = it.next();
            if (!detail::option_like_has_value(skipped)) {
                return next_result(rusty::None);
            }
        }
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(detail::option_like_take_value(item));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_nth(std::forward<Range>(range).into_iter(), n);
    } else {
        return iter_nth(iter(std::forward<Range>(range)), n);
    }
}

// Rust `Iterator::max_by(cmp)` — last item ranked Greater; ties keep the
// later item (Rust replaces unless the incumbent compares Greater). The
// comparator returns the module-emitted `rusty::cmp::Ordering`, which this
// HEADER cannot name — resolve its `Greater` enumerator dependently from
// the comparator's own return type at instantiation.
template<typename Range, typename Cmp>
auto iter_max_by(Range&& range, Cmp&& cmp) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto best = detail::option_like_take_value(item);
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            auto value = detail::option_like_take_value(nxt);
            auto ord = cmp(best, value);
            using Ord = std::remove_cvref_t<decltype(ord)>;
            if (ord != Ord::Greater) {
                best = std::move(value);
            }
        }
        return next_result(std::move(best));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_max_by(std::forward<Range>(range).into_iter(),
                           std::forward<Cmp>(cmp));
    } else {
        return iter_max_by(iter(std::forward<Range>(range)),
                           std::forward<Cmp>(cmp));
    }
}

// Rust `Iterator::min_by(cmp)` — first item ranked Less; ties keep the
// earlier item (Rust replaces only when the incumbent compares Greater).
template<typename Range, typename Cmp>
auto iter_min_by(Range&& range, Cmp&& cmp) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto best = detail::option_like_take_value(item);
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            auto value = detail::option_like_take_value(nxt);
            auto ord = cmp(best, value);
            using Ord = std::remove_cvref_t<decltype(ord)>;
            if (ord == Ord::Greater) {
                best = std::move(value);
            }
        }
        return next_result(std::move(best));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_min_by(std::forward<Range>(range).into_iter(),
                           std::forward<Cmp>(cmp));
    } else {
        return iter_min_by(iter(std::forward<Range>(range)),
                           std::forward<Cmp>(cmp));
    }
}

// Rust `Iterator::max()` / `::min()` — Ord-based via `<` on the deref'd
// values; max keeps the LAST maximum, min the FIRST minimum (matching
// Rust's tie-breaking).
template<typename Range>
auto iter_max(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto best = detail::option_like_take_value(item);
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            auto value = detail::option_like_take_value(nxt);
            if (!(detail::deref_if_pointer_like(value)
                  < detail::deref_if_pointer_like(best))) {
                best = std::move(value);
            }
        }
        return next_result(std::move(best));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_max(std::forward<Range>(range).into_iter());
    } else {
        return iter_max(iter(std::forward<Range>(range)));
    }
}

template<typename Range>
auto iter_min(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto best = detail::option_like_take_value(item);
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            auto value = detail::option_like_take_value(nxt);
            if (detail::deref_if_pointer_like(value)
                < detail::deref_if_pointer_like(best)) {
                best = std::move(value);
            }
        }
        return next_result(std::move(best));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_min(std::forward<Range>(range).into_iter());
    } else {
        return iter_min(iter(std::forward<Range>(range)));
    }
}

// Rust `Iterator::max_by_key(f)` / `::min_by_key(f)` — rank by a derived
// key; same tie-breaking as max/min.
template<typename Range, typename KeyFn>
auto iter_max_by_key(Range&& range, KeyFn&& key_fn) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto best = detail::option_like_take_value(item);
        // Slice iterators yield POINTERS; the emitted key closures are
        // written against the item value (`|s| s.len()`), so feed them
        // the pointee (identity for value items).
        auto best_key = key_fn(detail::deref_if_pointer_like(best));
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            auto value = detail::option_like_take_value(nxt);
            auto key = key_fn(detail::deref_if_pointer_like(value));
            if (!(key < best_key)) {
                best = std::move(value);
                best_key = std::move(key);
            }
        }
        return next_result(std::move(best));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_max_by_key(std::forward<Range>(range).into_iter(),
                               std::forward<KeyFn>(key_fn));
    } else {
        return iter_max_by_key(iter(std::forward<Range>(range)),
                               std::forward<KeyFn>(key_fn));
    }
}

template<typename Range, typename KeyFn>
auto iter_min_by_key(Range&& range, KeyFn&& key_fn) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        auto best = detail::option_like_take_value(item);
        // See iter_max_by_key: key closures receive the pointee.
        auto best_key = key_fn(detail::deref_if_pointer_like(best));
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            auto value = detail::option_like_take_value(nxt);
            auto key = key_fn(detail::deref_if_pointer_like(value));
            if (key < best_key) {
                best = std::move(value);
                best_key = std::move(key);
            }
        }
        return next_result(std::move(best));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_min_by_key(std::forward<Range>(range).into_iter(),
                               std::forward<KeyFn>(key_fn));
    } else {
        return iter_min_by_key(iter(std::forward<Range>(range)),
                               std::forward<KeyFn>(key_fn));
    }
}

// Rust `Iterator::reduce(f)` — fold seeded by the first item; None on an
// empty iterator. The closure must return the item type (as in Rust).
template<typename Range, typename Func>
auto iter_reduce(Range&& range, Func&& func) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        using item_type = detail::next_item_t<decltype(it)>;
        using next_result = rusty::Option<item_type>;
        auto item = it.next();
        if (!detail::option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        item_type acc = detail::option_like_take_value(item);
        while (true) {
            auto nxt = it.next();
            if (!detail::option_like_has_value(nxt)) {
                break;
            }
            acc = func(std::move(acc), detail::option_like_take_value(nxt));
        }
        return next_result(std::move(acc));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_reduce(std::forward<Range>(range).into_iter(),
                           std::forward<Func>(func));
    } else {
        return iter_reduce(iter(std::forward<Range>(range)),
                           std::forward<Func>(func));
    }
}

// Rust `Iterator::take_while(pred)` / `::skip_while(pred)` — lazy adapters
// (see detail::take_while_next_iter / skip_while_next_iter).
template<typename Range, typename Pred>
decltype(auto) take_while(Range&& range, Pred&& pred) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_take_while_next_iter(
            std::forward<Range>(range), std::forward<Pred>(pred));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return take_while(std::forward<Range>(range).into_iter(),
                          std::forward<Pred>(pred));
    } else {
        return take_while(iter(std::forward<Range>(range)),
                          std::forward<Pred>(pred));
    }
}

template<typename Range, typename Pred>
decltype(auto) skip_while(Range&& range, Pred&& pred) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_skip_while_next_iter(
            std::forward<Range>(range), std::forward<Pred>(pred));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return skip_while(std::forward<Range>(range).into_iter(),
                          std::forward<Pred>(pred));
    } else {
        return skip_while(iter(std::forward<Range>(range)),
                          std::forward<Pred>(pred));
    }
}

// Rust `Iterator::peekable()` — the member spelling wins (slice_iter::Iter
// carries one); any other option-like-next iterator wraps in the generic
// iter_adapters::Peekable.
template<typename Range>
decltype(auto) peekable(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).peekable(); }) {
        return std::forward<Range>(range).peekable();
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return iter_adapters::Peekable<std::remove_reference_t<Range>>(
            std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return peekable(std::forward<Range>(range).into_iter());
    } else {
        return peekable(iter(std::forward<Range>(range)));
    }
}

// Rust `Iterator::inspect(f)` / `::fuse()` — lazy adapters (see
// detail::inspect_next_iter / fuse_next_iter). Member spelling wins.
template<typename Range, typename Func>
decltype(auto) inspect(Range&& range, Func&& func) {
    if constexpr (requires { std::forward<Range>(range).inspect(std::forward<Func>(func)); }) {
        return std::forward<Range>(range).inspect(std::forward<Func>(func));
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_inspect_next_iter(
            std::forward<Range>(range), std::forward<Func>(func));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return inspect(std::forward<Range>(range).into_iter(),
                       std::forward<Func>(func));
    } else {
        return inspect(iter(std::forward<Range>(range)),
                       std::forward<Func>(func));
    }
}

template<typename Range>
decltype(auto) fuse(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).fuse(); }) {
        return std::forward<Range>(range).fuse();
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_fuse_next_iter(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return fuse(std::forward<Range>(range).into_iter());
    } else {
        return fuse(iter(std::forward<Range>(range)));
    }
}

// Rust `Iterator::position(pred)` — index of the first matching item. The
// member spelling wins (slice_iter::Iter carries an optimized one); other
// option-like-next iterators (transpiled Chars etc.) use the generic loop.
template<typename Range, typename Pred>
auto iter_position(Range&& range, Pred&& pred) -> rusty::Option<size_t> {
    if constexpr (requires {
                      { std::forward<Range>(range).position(std::forward<Pred>(pred)) };
                  }) {
        return std::forward<Range>(range).position(std::forward<Pred>(pred));
    } else if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        auto it = std::forward<Range>(range);
        size_t index = 0;
        while (true) {
            auto item = it.next();
            if (!detail::option_like_has_value(item)) {
                return rusty::Option<size_t>(rusty::None);
            }
            auto value = detail::option_like_take_value(item);
            if (pred(value)) {
                return rusty::Option<size_t>(index);
            }
            ++index;
        }
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return iter_position(std::forward<Range>(range).into_iter(),
                             std::forward<Pred>(pred));
    } else {
        return iter_position(iter(std::forward<Range>(range)),
                             std::forward<Pred>(pred));
    }
}

template<typename Range, typename Acc, typename Func>
auto fold(Range&& range, Acc init, Func&& func) {
    auto acc = std::move(init);
    for (auto&& item : for_in(std::forward<Range>(range))) {
        acc = std::invoke(
            func,
            std::move(acc),
            std::forward<decltype(item)>(item));
    }
    return acc;
}

template<typename Range, typename Acc, typename Func>
auto try_fold(Range&& range, Acc&& init, Func&& func) {
    using range_iter = decltype(for_in(std::forward<Range>(range)));
    using item_ref = decltype(*std::begin(std::declval<range_iter&>()));
    if constexpr (std::is_lvalue_reference_v<Acc>) {
        using acc_ref = Acc;
        using acc_value = std::remove_reference_t<Acc>;
        using step_type = std::remove_cvref_t<std::invoke_result_t<Func&, acc_ref, item_ref>>;

        auto* acc = std::addressof(init);
        for (auto&& item : for_in(std::forward<Range>(range))) {
            auto step = std::invoke(
                func,
                static_cast<acc_ref>(*acc),
                std::forward<decltype(item)>(item));
            if constexpr (requires(const step_type& s) {
                              s.is_ok();
                              s.is_err();
                          }) {
                if (step.is_err()) {
                    return step;
                }
                auto&& next = step.unwrap();
                acc = std::addressof(next);
            } else if constexpr (requires(const step_type& s) {
                                     s.is_some();
                                     s.is_none();
                                 }) {
                if (step.is_none()) {
                    return step;
                }
                auto&& next = step.unwrap();
                acc = std::addressof(next);
            } else {
                auto&& next = step;
                acc = std::addressof(next);
            }
        }

        if constexpr (requires(acc_value& value) { step_type::Ok(value); }) {
            return step_type::Ok(*acc);
        } else if constexpr (requires(acc_value& value) { step_type(value); }) {
            return step_type(*acc);
        } else {
            return *acc;
        }
    } else {
    using acc_type = std::remove_cvref_t<Acc>;
    using step_type = std::remove_cvref_t<std::invoke_result_t<Func&, acc_type, item_ref>>;

    auto acc = static_cast<acc_type>(std::move(init));
    for (auto&& item : for_in(std::forward<Range>(range))) {
        auto step = std::invoke(
            func,
            std::move(acc),
            std::forward<decltype(item)>(item));
        if constexpr (requires(const step_type& s) {
                          s.is_ok();
                          s.is_err();
                      }) {
            if (step.is_err()) {
                return step;
            }
            acc = step.unwrap();
        } else if constexpr (requires(const step_type& s) {
                                 s.is_some();
                                 s.is_none();
                             }) {
            if (step.is_none()) {
                return step;
            }
            acc = step.unwrap();
        } else {
            acc = std::move(step);
        }
    }

    if constexpr (requires(acc_type value) { step_type::Ok(value); }) {
        return step_type::Ok(std::move(acc));
    } else if constexpr (requires(acc_type value) { step_type(value); }) {
        return step_type(std::move(acc));
    } else {
        return acc;
    }
    }
}

// `Iterator::try_for_each`: apply `func` (returning a Try type — Result/Option
// with `()` output) to each item, short-circuiting on the first Err/None.
// Mirrors `try_fold` above but carries no accumulator; on full success it
// returns the Try's unit success value (`Ok(())` / `Some(())`). `rusty::Unit`
// is `std::tuple<>`, so `std::make_tuple()` is that `()` payload.
template<typename Range, typename Func>
auto try_for_each(Range&& range, Func&& func) {
    using range_iter = decltype(for_in(std::forward<Range>(range)));
    using item_ref = decltype(*std::begin(std::declval<range_iter&>()));
    using step_type = std::remove_cvref_t<std::invoke_result_t<Func&, item_ref>>;
    for (auto&& item : for_in(std::forward<Range>(range))) {
        auto step = std::invoke(func, std::forward<decltype(item)>(item));
        if constexpr (requires(const step_type& s) {
                          s.is_ok();
                          s.is_err();
                      }) {
            if (step.is_err()) {
                return step;
            }
        } else if constexpr (requires(const step_type& s) {
                                 s.is_some();
                                 s.is_none();
                             }) {
            if (step.is_none()) {
                return step;
            }
        }
    }
    if constexpr (requires { step_type::Ok(std::make_tuple()); }) {
        return step_type::Ok(std::make_tuple());
    } else if constexpr (requires { step_type::Some(std::make_tuple()); }) {
        return step_type::Some(std::make_tuple());
    } else {
        return step_type{};
    }
}

namespace ops {

template<typename Lhs, typename Rhs>
constexpr auto add(Lhs&& lhs, Rhs&& rhs) {
    return detail::deref_if_pointer(std::forward<Lhs>(lhs))
           + detail::deref_if_pointer(std::forward<Rhs>(rhs));
}

struct add_fn_t {
    template<typename Lhs, typename Rhs>
    constexpr auto operator()(Lhs&& lhs, Rhs&& rhs) const {
        return add(std::forward<Lhs>(lhs), std::forward<Rhs>(rhs));
    }
};

inline constexpr add_fn_t add_fn{};

} // namespace ops

// ---- rusty::slice_ext namespace — minimal helpers for vec_port.
// Cannot live in `rusty::slice` because that's a free function in
// array.hpp; patcher rewrites `slice::range` → `rusty::slice_ext::range`. ----
namespace slice_ext {

// Mirrors `core::slice::range<R: RangeBounds<usize>>(range, ..len)` —
// converts a generic range expression + bounds into a concrete
// (start, end) pair. Returns std::pair<size_t, size_t>.
template<typename R, typename Bounds>
inline std::pair<std::size_t, std::size_t> range(R r, Bounds bounds) noexcept {
    std::size_t start = 0;
    std::size_t end = 0;
    if constexpr (requires { bounds.end; }) {
        end = bounds.end;
    }
    if constexpr (requires { r.start; }) {
        start = r.start;
    }
    if constexpr (requires { r.end; }) {
        end = r.end;
    } else if constexpr (requires { r.end_value(); }) {
        // `rusty::range<T>` keeps its end as a private `end_` field
        // exposed via `end_value()`. Without this branch the caller
        // sees `bounds.end` (= len), so a partial range like
        // `range(0, 2)` becomes a full-range drain.
        end = r.end_value();
    }
    return {start, end};
}

} // namespace slice_ext

// Rust-protocol sides for `rusty::zip`: a zipped side exposing option-like
// `next()` but no begin()/end() (indexmap's Keys/Values/Iter) is adapted
// through `detail::make_next_iter_range` and re-dispatched. After
// normalization neither side satisfies the constraint, so resolution falls
// to the zip_view overload (array.hpp) — no recursion. Sides that already
// have begin()/end() pass through untouched.
namespace detail {
template<typename T>
inline constexpr bool zip_needs_next_adapter_v =
    has_option_like_next_v<std::remove_reference_t<T>>
    && !requires(T& t) { std::begin(t); std::end(t); };

template<typename T>
decltype(auto) zip_normalize_side(T&& side) {
    if constexpr (zip_needs_next_adapter_v<T>) {
        return make_next_iter_range(std::forward<T>(side));
    } else {
        return std::forward<T>(side);
    }
}
} // namespace detail

template<typename A, typename B>
requires (detail::zip_needs_next_adapter_v<A> || detail::zip_needs_next_adapter_v<B>)
auto zip(A&& a, B&& b) {
    return zip(
        detail::zip_normalize_side(std::forward<A>(a)),
        detail::zip_normalize_side(std::forward<B>(b)));
}

// ---- rusty::iter_ext namespace — `iter_ext::zip(a, b)` helper.
// Cannot live in `rusty::iter` because that's already a free function;
// patcher maps `iter::zip` → `rusty::iter_ext::zip` to disambiguate. ----
namespace iter_ext {

template<typename A, typename B>
inline auto zip(A&& a, B&& b) {
    // Minimal stub: returns a tuple of references. Real implementation
    // would be a lazy zipped iterator. vec_port uses this in code paths
    // that may not be exercised at module-compile time.
    return std::make_pair(std::forward<A>(a), std::forward<B>(b));
}

} // namespace iter_ext

} // namespace rusty

#endif // RUSTY_SLICE_HPP
