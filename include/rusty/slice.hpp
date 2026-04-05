#ifndef RUSTY_SLICE_HPP
#define RUSTY_SLICE_HPP

#include <cstddef>
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
    } else if constexpr (requires { *std::forward<Range>(range); }) {
        return iter(*std::forward<Range>(range));
    } else {
        static_assert(
            detail::dependent_false_v<Range>,
            "rusty::iter requires iter(), data()/size(), or dereferenceable receiver"
        );
    }
}

} // namespace rusty

#endif // RUSTY_SLICE_HPP
