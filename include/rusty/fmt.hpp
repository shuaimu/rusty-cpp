#ifndef RUSTY_FMT_HPP
#define RUSTY_FMT_HPP

#include <tuple>
#include "rusty/result.hpp"

namespace rusty {
namespace fmt {

/// Formatting error type (infallible in practice for String writes).
struct Error {};

struct Write {};

/// Result type for formatting operations.
using Result = rusty::Result<std::tuple<>, Error>;

struct Formatter;

// Lightweight typed-erased reference used for `&dyn fmt::Display` surfaces.
struct DisplayRef {
    using FmtFn = Result (*)(const void*, Formatter&);

    const void* ptr = nullptr;
    FmtFn fmt_fn = nullptr;

    DisplayRef() = default;

    template<typename T>
    DisplayRef(const T& value) : ptr(&value), fmt_fn(&DisplayRef::fmt_impl<T>) {}

    Result fmt(Formatter& formatter) const {
        if (ptr == nullptr || fmt_fn == nullptr) {
            return Result::Err(Error{});
        }
        return fmt_fn(ptr, formatter);
    }

private:
    template<typename T>
    static Result fmt_impl(const void* raw, Formatter& formatter) {
        const auto& value = *static_cast<const T*>(raw);
        if constexpr (requires { value.fmt(formatter); }) {
            return value.fmt(formatter);
        } else if constexpr (requires { rusty_fmt(value, formatter); }) {
            return rusty_fmt(value, formatter);
        } else {
            return Result::Err(Error{});
        }
    }
};

} // namespace fmt
} // namespace rusty

#endif // RUSTY_FMT_HPP
