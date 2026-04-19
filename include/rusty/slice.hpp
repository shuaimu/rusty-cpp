#ifndef RUSTY_SLICE_HPP
#define RUSTY_SLICE_HPP

#include <array>
#include <algorithm>
#include <cstddef>
#include <functional>
#include <iterator>
#include <limits>
#include <optional>
#include <span>
#include <tuple>
#include <type_traits>
#include <utility>
#include <vector>

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

private:
    pointer cur_;
    pointer end_;
};

} // namespace slice_iter

namespace detail {
template<typename T>
inline constexpr bool dependent_false_v = false;

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
auto option_like_take_value(Opt& opt) {
    if constexpr (slice_has_unwrap<Opt>::value) {
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

    explicit next_iter_range(NextIter iter) : iter_(std::forward<NextIter>(iter)) {}

    class iterator {
        using item_type = next_item_t<NextIter>;
        using iter_type = std::remove_reference_t<NextIter>;

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
            current_.emplace(option_like_take_value(next_item));
            at_end_ = false;
        }

        iter_type* iter_;
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
        using entry_type = std::tuple<size_t, item_type>;
        using next_result = rusty::Option<entry_type>;
        auto item = iter_.next();
        if (!option_like_has_value(item)) {
            return next_result(rusty::None);
        }
        return next_result(entry_type(
            index_++,
            option_like_take_value(item))
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
        using entry_type = std::tuple<size_t, item_type>;
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
            option_like_take_value(item))
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
        using left_item = next_item_t<std::remove_reference_t<LeftIter>>;
        using item_type = std::decay_t<left_item>;
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
} // namespace detail

template<typename Range>
decltype(auto) iter(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).iter(); }) {
        return std::forward<Range>(range).iter();
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
    } else if constexpr (requires { std::begin(std::forward<Range>(range)); std::end(std::forward<Range>(range)); }) {
        return std::forward<Range>(range);
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

template<typename Range>
decltype(auto) iter_mut(Range&& range) {
    if constexpr (requires { std::forward<Range>(range).iter_mut(); }) {
        return std::forward<Range>(range).iter_mut();
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

template<typename Range>
size_t count(Range&& range) {
    size_t n = 0;
    for (auto&& _item : for_in(std::forward<Range>(range))) {
        (void)_item;
        ++n;
    }
    return n;
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
auto try_fold(Range&& range, Acc init, Func&& func) {
    using range_iter = decltype(for_in(std::forward<Range>(range)));
    using item_ref = decltype(*std::begin(std::declval<range_iter&>()));
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
