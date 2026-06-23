#ifndef RUSTY_ARCH_HPP
#define RUSTY_ARCH_HPP

// Lightweight facade for Rust's `std::arch` / `core::arch`.
//
// Rust exposes architecture-specific intrinsic modules such as
// `std::arch::x86_64`. In C++, the compiler already provides those intrinsics
// through vendor headers. This header imports the matching names into
// `rusty::arch::<target>` and adds a few Rust-shaped helpers where C/C++ spell
// the surface differently.

#include <cstdint>
#include <string_view>

#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || defined(_M_IX86)
#define RUSTY_ARCH_X86_AVAILABLE 1
#include <immintrin.h>
#ifdef _xgetbv
#undef _xgetbv
#endif
#if defined(__GNUC__) || defined(__clang__)
#include <cpuid.h>
#ifdef __cpuid
#undef __cpuid
#endif
#ifdef __cpuid_count
#undef __cpuid_count
#endif
#endif
#else
#define RUSTY_ARCH_X86_AVAILABLE 0
#endif

namespace rusty {
namespace arch {

namespace detail {

inline bool x86_feature_detected(std::string_view feature) noexcept {
#if RUSTY_ARCH_X86_AVAILABLE && (defined(__GNUC__) || defined(__clang__))
    __builtin_cpu_init();

    if (feature == "sse") return __builtin_cpu_supports("sse");
    if (feature == "sse2") return __builtin_cpu_supports("sse2");
    if (feature == "sse3") return __builtin_cpu_supports("sse3");
    if (feature == "ssse3") return __builtin_cpu_supports("ssse3");
    if (feature == "sse4.1") return __builtin_cpu_supports("sse4.1");
    if (feature == "sse4.2") return __builtin_cpu_supports("sse4.2");
    if (feature == "avx") return __builtin_cpu_supports("avx");
    if (feature == "avx2") return __builtin_cpu_supports("avx2");
    if (feature == "fma") return __builtin_cpu_supports("fma");
    if (feature == "bmi1") return __builtin_cpu_supports("bmi");
    if (feature == "bmi2") return __builtin_cpu_supports("bmi2");
    if (feature == "lzcnt") return __builtin_cpu_supports("lzcnt");
    if (feature == "popcnt") return __builtin_cpu_supports("popcnt");
    if (feature == "aes") return __builtin_cpu_supports("aes");
    if (feature == "pclmulqdq") return __builtin_cpu_supports("pclmul");
    if (feature == "rdrand") return __builtin_cpu_supports("rdrnd");
    if (feature == "rdseed") return __builtin_cpu_supports("rdseed");
    if (feature == "sha") return __builtin_cpu_supports("sha");
    if (feature == "avx512f") return __builtin_cpu_supports("avx512f");
    if (feature == "avx512bw") return __builtin_cpu_supports("avx512bw");
    if (feature == "avx512cd") return __builtin_cpu_supports("avx512cd");
    if (feature == "avx512dq") return __builtin_cpu_supports("avx512dq");
    if (feature == "avx512vl") return __builtin_cpu_supports("avx512vl");
#else
    (void)feature;
#endif
    return false;
}

inline constexpr bool x86_target_feature_enabled(std::string_view feature) noexcept {
#if RUSTY_ARCH_X86_AVAILABLE
#if defined(__SSE__)
    if (feature == "sse") return true;
#endif
#if defined(__SSE2__) || defined(__x86_64__) || defined(_M_X64)
    if (feature == "sse2") return true;
#endif
#if defined(__SSE3__)
    if (feature == "sse3") return true;
#endif
#if defined(__SSSE3__)
    if (feature == "ssse3") return true;
#endif
#if defined(__SSE4_1__)
    if (feature == "sse4.1") return true;
#endif
#if defined(__SSE4_2__)
    if (feature == "sse4.2") return true;
#endif
#if defined(__AVX__)
    if (feature == "avx") return true;
#endif
#if defined(__AVX2__)
    if (feature == "avx2") return true;
#endif
#if defined(__FMA__)
    if (feature == "fma") return true;
#endif
#if defined(__BMI__)
    if (feature == "bmi1") return true;
#endif
#if defined(__BMI2__)
    if (feature == "bmi2") return true;
#endif
#if defined(__LZCNT__)
    if (feature == "lzcnt") return true;
#endif
#if defined(__POPCNT__)
    if (feature == "popcnt") return true;
#endif
#if defined(__AES__)
    if (feature == "aes") return true;
#endif
#if defined(__PCLMUL__)
    if (feature == "pclmulqdq") return true;
#endif
#if defined(__RDRND__)
    if (feature == "rdrand") return true;
#endif
#if defined(__RDSEED__)
    if (feature == "rdseed") return true;
#endif
#if defined(__SHA__)
    if (feature == "sha") return true;
#endif
#if defined(__AVX512F__)
    if (feature == "avx512f") return true;
#endif
#if defined(__AVX512BW__)
    if (feature == "avx512bw") return true;
#endif
#if defined(__AVX512CD__)
    if (feature == "avx512cd") return true;
#endif
#if defined(__AVX512DQ__)
    if (feature == "avx512dq") return true;
#endif
#if defined(__AVX512VL__)
    if (feature == "avx512vl") return true;
#endif
#else
    (void)feature;
#endif
    return false;
}

#if RUSTY_ARCH_X86_AVAILABLE
namespace x86_common {

using ::__m128;
using ::__m128d;
using ::__m128i;
using ::__m256;
using ::__m256d;
using ::__m256i;

#if defined(__AVX512F__) || defined(__clang__) || defined(__GNUC__)
using ::__m512;
using ::__m512d;
using ::__m512i;
#endif

using ::_mm_add_epi8;
using ::_mm_add_epi16;
using ::_mm_add_epi32;
using ::_mm_add_epi64;
using ::_mm_and_si128;
using ::_mm_andnot_si128;
using ::_mm_cmpeq_epi8;
using ::_mm_cmpeq_epi16;
using ::_mm_cmpeq_epi32;
using ::_mm_cmpgt_epi8;
using ::_mm_cmpgt_epi16;
using ::_mm_cmpgt_epi32;
using ::_mm_cvtsi128_si32;
using ::_mm_cvtsi32_si128;
using ::_mm_load_si128;
using ::_mm_loadu_si128;
using ::_mm_movemask_epi8;
using ::_mm_or_si128;
using ::_mm_pause;
using ::_mm_sad_epu8;
using ::_mm_set_epi8;
using ::_mm_set_epi16;
using ::_mm_set_epi32;
using ::_mm_set_epi64x;
using ::_mm_set1_epi8;
using ::_mm_set1_epi16;
using ::_mm_set1_epi32;
using ::_mm_set1_epi64x;
using ::_mm_setzero_si128;
using ::_mm_slli_epi16;
using ::_mm_slli_epi32;
using ::_mm_slli_epi64;
using ::_mm_srai_epi16;
using ::_mm_srai_epi32;
using ::_mm_srli_epi16;
using ::_mm_srli_epi32;
using ::_mm_srli_epi64;
using ::_mm_store_si128;
using ::_mm_storeu_si128;
using ::_mm_sub_epi8;
using ::_mm_sub_epi16;
using ::_mm_sub_epi32;
using ::_mm_sub_epi64;
using ::_mm_unpackhi_epi8;
using ::_mm_unpackhi_epi16;
using ::_mm_unpackhi_epi32;
using ::_mm_unpackhi_epi64;
using ::_mm_unpacklo_epi8;
using ::_mm_unpacklo_epi16;
using ::_mm_unpacklo_epi32;
using ::_mm_unpacklo_epi64;
using ::_mm_xor_si128;

using ::_mm_blendv_epi8;
using ::_mm_blendv_pd;
using ::_mm_blendv_ps;
using ::_mm_cmpeq_epi64;
using ::_mm_max_epi8;
using ::_mm_max_epi32;
using ::_mm_max_epu8;
using ::_mm_max_epu16;
using ::_mm_max_epu32;
using ::_mm_min_epi8;
using ::_mm_min_epi32;
using ::_mm_min_epu8;
using ::_mm_min_epu16;
using ::_mm_min_epu32;
using ::_mm_testz_si128;

using ::_mm256_add_epi8;
using ::_mm256_add_epi16;
using ::_mm256_add_epi32;
using ::_mm256_add_epi64;
using ::_mm256_and_si256;
using ::_mm256_cmpeq_epi8;
using ::_mm256_cmpeq_epi16;
using ::_mm256_cmpeq_epi32;
using ::_mm256_cmpgt_epi8;
using ::_mm256_cmpgt_epi16;
using ::_mm256_cmpgt_epi32;
using ::_mm256_load_si256;
using ::_mm256_loadu_si256;
using ::_mm256_movemask_epi8;
using ::_mm256_or_si256;
using ::_mm256_set1_epi8;
using ::_mm256_set1_epi16;
using ::_mm256_set1_epi32;
using ::_mm256_set1_epi64x;
using ::_mm256_setzero_si256;
using ::_mm256_store_si256;
using ::_mm256_storeu_si256;
using ::_mm256_sub_epi8;
using ::_mm256_sub_epi16;
using ::_mm256_sub_epi32;
using ::_mm256_sub_epi64;
using ::_mm256_xor_si256;

struct CpuidResult {
    std::uint32_t eax = 0;
    std::uint32_t ebx = 0;
    std::uint32_t ecx = 0;
    std::uint32_t edx = 0;
};

inline CpuidResult __cpuid_count(std::uint32_t leaf, std::uint32_t sub_leaf) noexcept {
    CpuidResult out{};
#if defined(__GNUC__) || defined(__clang__)
    unsigned int eax = 0;
    unsigned int ebx = 0;
    unsigned int ecx = 0;
    unsigned int edx = 0;
    if (::__get_cpuid_count(leaf, sub_leaf, &eax, &ebx, &ecx, &edx) != 0) {
        out.eax = static_cast<std::uint32_t>(eax);
        out.ebx = static_cast<std::uint32_t>(ebx);
        out.ecx = static_cast<std::uint32_t>(ecx);
        out.edx = static_cast<std::uint32_t>(edx);
    }
#elif defined(_MSC_VER)
    int regs[4] = {};
    ::__cpuidex(regs, static_cast<int>(leaf), static_cast<int>(sub_leaf));
    out.eax = static_cast<std::uint32_t>(regs[0]);
    out.ebx = static_cast<std::uint32_t>(regs[1]);
    out.ecx = static_cast<std::uint32_t>(regs[2]);
    out.edx = static_cast<std::uint32_t>(regs[3]);
#else
    (void)leaf;
    (void)sub_leaf;
#endif
    return out;
}

inline CpuidResult __cpuid(std::uint32_t leaf) noexcept {
    return __cpuid_count(leaf, 0);
}

inline unsigned long long _rdtsc() noexcept {
    return ::__rdtsc();
}

inline unsigned long long _xgetbv(unsigned int xcr) noexcept {
#if defined(_MSC_VER)
    return ::_xgetbv(xcr);
#elif defined(__clang__)
    return __builtin_ia32_xgetbv(static_cast<long long>(xcr));
#elif defined(__GNUC__)
    return __builtin_ia32_xgetbv(xcr);
#else
    (void)xcr;
    return 0;
#endif
}

} // namespace x86_common
#endif // RUSTY_ARCH_X86_AVAILABLE

} // namespace detail

inline bool is_x86_feature_detected(std::string_view feature) noexcept {
    return detail::x86_feature_detected(feature);
}

inline constexpr bool x86_target_feature_enabled(std::string_view feature) noexcept {
    return detail::x86_target_feature_enabled(feature);
}

namespace x86 {
#if RUSTY_ARCH_X86_AVAILABLE
using namespace detail::x86_common;
#endif
} // namespace x86

namespace x86_64 {
#if RUSTY_ARCH_X86_AVAILABLE
using namespace detail::x86_common;
#endif
} // namespace x86_64

} // namespace arch
} // namespace rusty

#undef RUSTY_ARCH_X86_AVAILABLE

#endif // RUSTY_ARCH_HPP
