// rusty/hash.hpp - std::hash-module runtime pieces for the transpiled std port
//
// Rust's `std::hash::SipHasher` (and the `SipHasher13` behind DefaultHasher)
// back std HashMap's RandomState. Rust documents DefaultHasher's OUTPUT as
// unspecified (only deterministic within a process), so this port supplies a
// seeded FNV-1a-style hasher with SipHasher's API shape rather than a
// bit-exact SipHash: same construction surface (`new_with_keys(k0, k1)`),
// same Hasher method family, honest determinism.

#ifndef RUSTY_HASH_HPP
#define RUSTY_HASH_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <functional>
#include <span>
#include <string_view>
#include <type_traits>

namespace rusty {
namespace detail {

// `#[derive(Hash)]` lowered to a std::hash specialization: every field's
// std::hash, combined in declaration order. A field type without std::hash
// contributes nothing (equal values still hash equal).
inline void hash_combine(std::size_t& seed, std::size_t value) noexcept {
    seed ^= value + 0x9e3779b97f4a7c15ULL + (seed << 6) + (seed >> 2);
}

template<typename... Fields>
std::size_t hash_fields(const Fields&... fields) {
    std::size_t seed = 0;
    ([&](const auto& field) {
        using Field = std::remove_cvref_t<decltype(field)>;
        if constexpr (requires { std::hash<Field>{}(field); }) {
            hash_combine(seed, std::hash<Field>{}(field));
        }
    }(fields), ...);
    return seed;
}

} // namespace detail

namespace hash {

class SipHasher {
    std::uint64_t state_;

    static constexpr std::uint64_t FNV_OFFSET = 0xcbf29ce484222325ull;
    static constexpr std::uint64_t FNV_PRIME = 0x100000001b3ull;

    constexpr void mix_byte(std::uint8_t b) noexcept {
        state_ = (state_ ^ b) * FNV_PRIME;
    }

public:
    constexpr SipHasher() noexcept : state_(FNV_OFFSET) {}

    static constexpr SipHasher new_with_keys(std::uint64_t k0, std::uint64_t k1) noexcept {
        SipHasher h;
        h.state_ = FNV_OFFSET ^ k0 ^ (k1 * FNV_PRIME);
        return h;
    }
    static constexpr SipHasher new_() noexcept { return new_with_keys(0, 0); }

    constexpr void write(const std::uint8_t* p, std::size_t n) noexcept {
        for (std::size_t i = 0; i < n; ++i) mix_byte(p[i]);
    }
    void write(std::span<const std::uint8_t> bytes) noexcept {
        write(bytes.data(), bytes.size());
    }
    void write_str(std::string_view s) noexcept {
        write(reinterpret_cast<const std::uint8_t*>(s.data()), s.size());
        mix_byte(0xff);  // Rust's write_str suffix delimiter
    }
    template<typename I>
    constexpr void write_int(I v) noexcept {
        for (std::size_t i = 0; i < sizeof(I); ++i) {
            mix_byte(static_cast<std::uint8_t>(static_cast<std::uint64_t>(v) >> (8 * i)));
        }
    }
    constexpr void write_u8(std::uint8_t v) noexcept { write_int(v); }
    constexpr void write_u16(std::uint16_t v) noexcept { write_int(v); }
    constexpr void write_u32(std::uint32_t v) noexcept { write_int(v); }
    constexpr void write_u64(std::uint64_t v) noexcept { write_int(v); }
    constexpr void write_usize(std::size_t v) noexcept { write_int(v); }
    constexpr void write_i8(std::int8_t v) noexcept { write_int(v); }
    constexpr void write_i16(std::int16_t v) noexcept { write_int(v); }
    constexpr void write_i32(std::int32_t v) noexcept { write_int(v); }
    constexpr void write_i64(std::int64_t v) noexcept { write_int(v); }
    constexpr void write_isize(std::ptrdiff_t v) noexcept { write_int(v); }

    // Rust `Hasher::write_length_prefix` default = write_usize. The
    // transpiled BTreeMap::hash prefixes with the length through this
    // (via the __mdisp_write_length_prefix dispatcher).
    constexpr void write_length_prefix(std::size_t len) noexcept { write_usize(len); }

    constexpr std::uint64_t finish() const noexcept { return state_; }
};

}  // namespace hash
}  // namespace rusty

#endif  // RUSTY_HASH_HPP
