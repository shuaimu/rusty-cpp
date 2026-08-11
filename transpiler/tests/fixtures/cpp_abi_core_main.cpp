#include <cassert>
#include <cstdint>
#include <string>
#include <type_traits>
#include <vector>

import cpp_abi_core;

static int string_evaluations = 0;
static int vector_evaluations = 0;

static std::string make_bytes() {
    ++string_evaluations;
    return std::string("A\0\x80\xff", 4);
}

static std::vector<double> make_weights() {
    ++vector_evaluations;
    return {1.25, 2.5, 5.0};
}

int main() {
    static_assert(std::is_same_v<Weights, std::vector<double>>);
    static_assert(std::is_same_v<decltype(&roundtrip), std::string (*)(std::string)>);
    static_assert(std::is_same_v<decltype(&Codec::encode), std::string (*)(uint8_t)>);
    static_assert(std::is_same_v<decltype(&Picker::choose), uint32_t (*)(const Weights&)>);
    static_assert(std::is_same_v<private_::static_, std::vector<double>>);
    static_assert(std::is_same_v<decltype(&private_::class_), std::string (*)(std::string)>);
    static_assert(std::is_same_v<decltype(&private_::struct_::pause),
                                 uint32_t (*)(const private_::static_&)>);

    const auto empty = roundtrip({});
    assert(empty.empty());

    const auto bytes = roundtrip(make_bytes());
    assert(string_evaluations == 1);
    assert(bytes.size() == 4);
    assert(static_cast<unsigned char>(bytes[0]) == 0x41);
    assert(static_cast<unsigned char>(bytes[1]) == 0x00);
    assert(static_cast<unsigned char>(bytes[2]) == 0x80);
    assert(static_cast<unsigned char>(bytes[3]) == 0xff);

    const auto encoded = Codec::encode(0x80);
    assert(encoded.size() == 3);
    assert(static_cast<unsigned char>(encoded[0]) == 0x80);
    assert(static_cast<unsigned char>(encoded[1]) == 0x00);
    assert(static_cast<unsigned char>(encoded[2]) == 0xff);

    assert(Picker::choose(make_weights()) == 3);
    assert(vector_evaluations == 1);

    const auto nested = private_::class_(std::string("\0\xff", 2));
    assert(nested.size() == 2);
    assert(static_cast<unsigned char>(nested[0]) == 0x00);
    assert(static_cast<unsigned char>(nested[1]) == 0xff);
    assert(private_::struct_::pause(private_::static_{2.0, 4.0}) == 2);
}
