// rusty/collections.hpp — minimal stubs for rustc's collections
// crate types that aren't full collections themselves.
//
// Added for vec_port to satisfy `std::collections::TryReserveError`
// references in raw_vec.

#ifndef RUSTY_COLLECTIONS_HPP
#define RUSTY_COLLECTIONS_HPP

#include <cstddef>

namespace rusty {
namespace collections {

// Like core::collections::TryReserveError. Rust gives this richer
// internal structure (Kind enum + layout), but for the C++ port we
// just track the discriminant — anyone consuming this only needs to
// distinguish CapacityOverflow from AllocError(layout).
struct TryReserveError {
    enum class Kind {
        CapacityOverflow,
        AllocError,
    };
    Kind kind = Kind::CapacityOverflow;
    // For AllocError: the layout that failed (size + alignment).
    // Stored as raw fields rather than rusty::alloc::Layout to avoid
    // pulling in alloc.hpp here. Set only when kind == AllocError.
    std::size_t layout_size = 0;
    std::size_t layout_align = 0;

    constexpr TryReserveError() = default;
    constexpr /*implicit*/ TryReserveError(Kind k) noexcept : kind(k) {}
    constexpr TryReserveError(Kind k, std::size_t size, std::size_t align) noexcept
        : kind(k), layout_size(size), layout_align(align) {}

    constexpr Kind kind_of() const noexcept { return kind; }

    // From conversions for rusty::from_into.
    static constexpr TryReserveError from(Kind k) noexcept {
        return TryReserveError(k);
    }
    // Accept anything aggregate-shaped with .layout / .non_exhaustive
    // (i.e. rusty::alloc::AllocError) — extract size/align if present.
    template<typename AllocErr>
    static constexpr TryReserveError from(AllocErr e) noexcept {
        TryReserveError err{Kind::AllocError};
        if constexpr (requires { e.layout.size; e.layout.align; }) {
            err.layout_size = e.layout.size;
            err.layout_align = e.layout.align;
        }
        return err;
    }
};

// Enum re-exported as `TryReserveErrorKind::CapacityOverflow` /
// `TryReserveErrorKind::AllocError(...)` in rustc.
using TryReserveErrorKind = TryReserveError::Kind;

}  // namespace collections
}  // namespace rusty

#endif  // RUSTY_COLLECTIONS_HPP
