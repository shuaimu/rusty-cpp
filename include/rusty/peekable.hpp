// rusty::iter_adapters::Peekable — minimal port of core::iter::Peekable for
// transpiled iterators (anything with an Option-returning `.next()`).
//
// Modeling note: Rust's Peekable caches `Option<Option<Item>>`. Here that is
// `has_peeked_` + a cached `Option<Item>`, and `peek()` hands back a
// `const Option<Item>&` (rather than Rust's `Option<&Item>`) so call sites
// use the ordinary Option API (`is_some`, `unwrap`, `map_or`, …).
#ifndef RUSTY_PEEKABLE_HPP
#define RUSTY_PEEKABLE_HPP

#include <type_traits>
#include <utility>

#include "rusty/option.hpp"

namespace rusty {
namespace iter_adapters {

template <typename I>
struct Peekable {
    using OptItem = std::remove_cvref_t<decltype(std::declval<I&>().next())>;

    I iter{};
    OptItem peeked_{rusty::None};
    bool has_peeked_ = false;

    Peekable() = default;
    explicit Peekable(I it) : iter(std::move(it)) {}

    OptItem next() {
        if (has_peeked_) {
            has_peeked_ = false;
            return std::move(peeked_);
        }
        return iter.next();
    }

    const OptItem& peek() {
        if (!has_peeked_) {
            peeked_ = iter.next();
            has_peeked_ = true;
        }
        return peeked_;
    }
};

template <typename I>
Peekable<std::remove_cvref_t<I>> peekable(I&& it) {
    return Peekable<std::remove_cvref_t<I>>(std::forward<I>(it));
}

} // namespace iter_adapters
} // namespace rusty

#endif // RUSTY_PEEKABLE_HPP
