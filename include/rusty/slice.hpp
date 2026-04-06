#ifndef RUSTY_SLICE_HPP
#define RUSTY_SLICE_HPP

#include <cstddef>
#include <functional>
#include <iterator>
#include <optional>
#include <span>
#include <tuple>
#include <type_traits>
#include <utility>

#include "rusty/option.hpp"

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

    Iter into_iter() const { return *this; }

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
            return rusty::Option<value_type>(*next_ptr.unwrap());
        }

        rusty::Option<value_type> next_back() {
            auto next_ptr = iter_.next_back();
            if (next_ptr.is_none()) {
                return rusty::None;
            }
            return rusty::Option<value_type>(*next_ptr.unwrap());
        }

        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return iter_.size_hint();
        }

    private:
        Iter iter_;
    };

    ClonedIter cloned() const { return ClonedIter(*this); }

private:
    pointer cur_;
    pointer end_;
};

} // namespace slice_iter

namespace detail {
template<typename T>
inline constexpr bool dependent_false_v = false;

template<typename NextResult, typename = void>
struct is_option_like_next_result : std::false_type {};

template<typename NextResult>
struct is_option_like_next_result<
    NextResult,
    std::void_t<
        decltype(option_has_value(std::declval<const NextResult&>())),
        decltype(option_take_value(std::declval<NextResult&>()))>> : std::true_type {};

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
    std::decay_t<decltype(option_take_value(std::declval<next_result_t<Iter>&>()))>;

template<typename T>
constexpr decltype(auto) deref_if_pointer(T&& value) {
    if constexpr (std::is_pointer_v<std::remove_reference_t<T>>) {
        return *std::forward<T>(value);
    } else {
        return std::forward<T>(value);
    }
}

template<typename NextIter>
class next_iter_range {
public:
    static_assert(
        has_option_like_next_v<NextIter>,
        "rusty::for_in requires next() to return an Option/optional-like value"
    );

    explicit next_iter_range(NextIter iter) : iter_(std::move(iter)) {}

    class iterator {
        using item_type = next_item_t<NextIter>;

    public:
        iterator() : iter_(nullptr), at_end_(true) {}

        explicit iterator(NextIter* iter, bool at_end = false)
            : iter_(iter), at_end_(at_end) {
            if (!at_end_) {
                advance();
            }
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
            if (!option_has_value(next_item)) {
                at_end_ = true;
                return;
            }
            current_.emplace(option_take_value(next_item));
            at_end_ = false;
        }

        NextIter* iter_;
        std::optional<item_type> current_;
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

    auto next() {
        using item_type = next_item_t<Iter>;
        using mapped_type = std::decay_t<decltype(
            std::invoke(std::declval<Func&>(), deref_if_pointer(std::declval<item_type>())))>;

        auto item = iter_.next();
        if (!option_has_value(item)) {
            return std::optional<mapped_type>{};
        }
        return std::optional<mapped_type>(std::invoke(
            func_,
            deref_if_pointer(option_take_value(item))));
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

    auto next() {
        using item_type = next_item_t<Iter>;
        using entry_type = std::tuple<size_t, item_type>;
        auto item = iter_.next();
        if (!option_has_value(item)) {
            return std::optional<entry_type>{};
        }
        return std::optional<entry_type>(
            std::in_place,
            index_++,
            option_take_value(item)
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

    auto next() {
        return iter_.next_back();
    }

private:
    Iter iter_;
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

    auto next() {
        using next_result = decltype(iter_.next());
        if (remaining_ == 0) {
            return next_result{};
        }
        auto item = iter_.next();
        if (!option_has_value(item)) {
            return item;
        }
        --remaining_;
        return item;
    }

private:
    Iter iter_;
    size_t remaining_;
};

template<typename NextIter>
auto make_next_iter_range(NextIter&& iter) {
    using stored_iter = std::decay_t<NextIter>;
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
} // namespace detail

template<typename Range>
decltype(auto) iter(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).iter(); }) {
        return std::forward<Range>(range).iter();
    } else if constexpr (requires { std::forward<Range>(range).data(); std::forward<Range>(range).size(); }) {
        auto&& view = std::forward<Range>(range);
        using elem_ptr = decltype(view.data());
        using elem_type = std::remove_pointer_t<elem_ptr>;
        auto* data = view.data();
        return slice_iter::Iter<elem_type>(data, data + view.size());
    } else if constexpr (requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }) {
        return std::forward<Range>(range);
    } else if constexpr (requires { *std::forward<Range>(range); }) {
        return iter(*std::forward<Range>(range));
    } else {
        static_assert(
            detail::dependent_false_v<Range>,
            "rusty::iter requires iter(), data()/size(), or dereferenceable receiver"
        );
    }
}

template<typename Range>
decltype(auto) for_in(Range&& range) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_next_iter_range(std::forward<Range>(range));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::for_in requires next() to return an Option/optional-like value"
        );
    } else if constexpr (requires { std::forward<Range>(range).into_iter(); }) {
        return for_in(std::forward<Range>(range).into_iter());
    } else if constexpr (
        requires { std::forward<Range>(range).iter(); }
        || requires { std::forward<Range>(range).data(); std::forward<Range>(range).size(); }
        || requires { *std::forward<Range>(range); }
    ) {
        return for_in(iter(std::forward<Range>(range)));
    } else {
        return std::forward<Range>(range);
    }
}

template<typename Range, typename Func>
decltype(auto) map(Range&& range, Func&& func) {
    if constexpr (detail::has_option_like_next_v<std::remove_reference_t<Range>>) {
        return detail::make_map_next_iter(
            std::forward<Range>(range),
            std::forward<Func>(func));
    } else if constexpr (requires { std::forward<Range>(range).next(); }) {
        static_assert(
            detail::dependent_false_v<std::remove_reference_t<Range>>,
            "rusty::map requires next() to return an Option/optional-like value"
        );
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
        static_assert(
            requires(iter_type& iter) { iter.next_back(); },
            "rusty::rev requires next_back() on next-like iterators"
        );
        return detail::make_rev_next_iter(std::forward<Range>(range));
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

} // namespace rusty

#endif // RUSTY_SLICE_HPP
