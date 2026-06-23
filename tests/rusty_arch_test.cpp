// Tests for the rusty::arch facade over Rust's std::arch/core::arch surface.
#include "../include/rusty/arch.hpp"

#include <array>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <type_traits>

namespace {

void test_x86_feature_detection_shape() {
    std::printf("test_x86_feature_detection_shape: ");
    assert(!rusty::arch::is_x86_feature_detected("definitely-not-a-real-feature"));
#if defined(__x86_64__) || defined(_M_X64)
    assert(rusty::arch::x86_target_feature_enabled("sse2"));
    assert(rusty::arch::is_x86_feature_detected("sse2"));
#endif
    std::printf("PASS\n");
}

void test_x86_sse2_import_surface() {
    std::printf("test_x86_sse2_import_surface: ");
#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || defined(_M_IX86)
    using rusty::arch::x86_64::__m128i;
    using rusty::arch::x86_64::_mm_loadu_si128;
    using rusty::arch::x86_64::_mm_movemask_epi8;
    using rusty::arch::x86_64::_mm_or_si128;
    using rusty::arch::x86_64::_mm_setzero_si128;

    static_assert(std::is_same_v<rusty::arch::x86_64::CpuidResult,
                                 rusty::arch::x86::CpuidResult>);

    alignas(16) std::array<std::uint8_t, 16> bytes{};
    bytes[0] = 0x80;
    bytes[15] = 0x80;

    const auto loaded =
        _mm_loadu_si128(reinterpret_cast<const __m128i*>(bytes.data()));
    const auto combined = _mm_or_si128(loaded, _mm_setzero_si128());
    assert(_mm_movemask_epi8(combined) == 0x8001);

    const auto cpuid = rusty::arch::x86_64::__cpuid(0);
    assert(cpuid.eax != 0 || cpuid.ebx != 0 || cpuid.ecx != 0 || cpuid.edx != 0);
#endif
    std::printf("PASS\n");
}

void test_x86_namespace_alias_shape() {
    std::printf("test_x86_namespace_alias_shape: ");
#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || defined(_M_IX86)
    namespace arch = rusty::arch;
    namespace x86_64 = arch::x86_64;
    using namespace x86_64;

    alignas(16) std::array<std::uint8_t, 16> bytes{};
    const auto zero = _mm_loadu_si128(reinterpret_cast<const __m128i*>(bytes.data()));
    assert(_mm_movemask_epi8(zero) == 0);
#endif
    std::printf("PASS\n");
}

} // namespace

int main() {
    test_x86_feature_detection_shape();
    test_x86_sse2_import_surface();
    test_x86_namespace_alias_shape();
    return 0;
}
