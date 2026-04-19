module;

#include <cstdint>

export module interop.host;

export namespace interop::host {

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

} // namespace interop::host
