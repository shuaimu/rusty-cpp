module;

#include <cstdint>

export module interop.host;

// The named-module identity is `interop.host`, while its exported symbols
// intentionally live in a different namespace. This distinction is covered
// by the Rust -> C++ runtime call below.
export namespace interop::api {

std::int32_t apply_bias(std::int32_t value) {
    return value + 4;
}

class Counter {
public:
    explicit Counter(std::int32_t seed) : value_(seed) {}

    std::int32_t add(std::int32_t delta) {
        value_ += delta;
        return value_;
    }

    std::int32_t value() const {
        return value_;
    }

private:
    std::int32_t value_;
};

} // namespace interop::api
