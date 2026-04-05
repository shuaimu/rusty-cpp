#ifndef RUSTY_MEM_HPP
#define RUSTY_MEM_HPP

#include <cstddef>
#include <type_traits>
#include <utility>

namespace rusty {
namespace mem {

template<typename T>
constexpr std::size_t size_of() noexcept {
    return sizeof(T);
}

template<typename T, typename U>
inline T replace(T& destination, U&& value) {
    T old = std::move(destination);
    destination = std::forward<U>(value);
    return old;
}

// Rust std::mem::drop consumes a value and destroys it at the end of this call.
template<typename T>
inline void drop(T&& value) noexcept {
    using Value = std::remove_reference_t<T>;
    [[maybe_unused]] Value consumed = std::forward<T>(value);
}

// Rust std::mem::forget consumes a value and intentionally leaks/drop-skips it.
// For drop-enabled transpiled structs, mark the value as forgotten so generated
// destructors can skip user Drop bodies on scope-exit/moved-from states.
template<typename T>
inline void forget(T&& value) noexcept {
    using Value = std::remove_reference_t<T>;
    if constexpr (requires(Value& v) { v.rusty_mark_forgotten(); }) {
        value.rusty_mark_forgotten();
    }
}

} // namespace mem
} // namespace rusty

#endif // RUSTY_MEM_HPP
