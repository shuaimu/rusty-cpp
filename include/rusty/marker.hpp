#ifndef RUSTY_MARKER_HPP
#define RUSTY_MARKER_HPP

namespace rusty {

// Zero-sized marker used to carry type/lifetime information in transpiled code.
template<typename T>
struct PhantomData {
    constexpr PhantomData() noexcept = default;
};

} // namespace rusty

#endif // RUSTY_MARKER_HPP
