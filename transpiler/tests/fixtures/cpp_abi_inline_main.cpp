#include <cstdint>
#include <string>
#include <vector>

import cpp_abi_inline;

namespace {

std::uint32_t byte_evaluations = 0;
std::uint32_t weight_evaluations = 0;

std::string make_binary_bytes() {
    ++byte_evaluations;
    return std::string({
        static_cast<char>(0x00),
        static_cast<char>(0x80),
        static_cast<char>(0xff),
    });
}

std::vector<double> make_weights() {
    ++weight_evaluations;
    return {1.0, 2.0, 3.0};
}

bool is_binary_payload(const std::string& value) {
    return value.size() == 3 &&
        static_cast<unsigned char>(value[0]) == 0x00 &&
        static_cast<unsigned char>(value[1]) == 0x80 &&
        static_cast<unsigned char>(value[2]) == 0xff;
}

} // namespace

int main() {
    byte_evaluations = 0;
    const auto direct = cpp_abi_inline::echo_bytes(make_binary_bytes());
    const auto indirect = cpp_abi_inline::InlineCodec::via_earlier(
        make_binary_bytes());
    if (byte_evaluations != 2 || !is_binary_payload(direct) ||
        !is_binary_payload(indirect) ||
        !cpp_abi_inline::echo_bytes(std::string{}).empty()) {
        return 1;
    }

    weight_evaluations = 0;
    if (cpp_abi_inline::InlineCodec::count_weights(make_weights()) != 3 ||
        weight_evaluations != 1) {
        return 2;
    }
    const std::vector<double> empty;
    if (cpp_abi_inline::InlineCodec::count_weights(empty) != 0) {
        return 3;
    }
    return 0;
}
