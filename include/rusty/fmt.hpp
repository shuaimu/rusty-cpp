#ifndef RUSTY_FMT_HPP
#define RUSTY_FMT_HPP

#include <tuple>
#include "rusty/result.hpp"

namespace rusty {
namespace fmt {

/// Formatting error type (infallible in practice for String writes).
struct Error {};

/// Result type for formatting operations.
using Result = rusty::Result<std::tuple<>, Error>;

} // namespace fmt
} // namespace rusty

#endif // RUSTY_FMT_HPP
