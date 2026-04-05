#ifndef RUSTY_MEM_HPP
#define RUSTY_MEM_HPP

namespace rusty {
namespace mem {

// Rust std::mem::forget consumes a value and intentionally leaks/drop-skips it.
// Transpiled code uses this as a semantic marker; no-op keeps the value consumed.
template<typename T>
inline void forget([[maybe_unused]] T&& value) noexcept {}

} // namespace mem
} // namespace rusty

#endif // RUSTY_MEM_HPP
