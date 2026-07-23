// rusty/ascii.hpp — ASCII byte predicates + case folding helpers.
//
// Mirrors the `u8::is_ascii_X()` / `u8::to_ascii_X()` family from Rust's
// stdlib. All functions are pure, branch-only-on-byte, no allocations.
//
// Used by ascii_port (transpiled core::ascii::ascii_char) — the
// transpiler emits `to_u8(self_).to_ascii_X()` patterns that bind to
// Rust u8 methods; these free functions provide the equivalent on
// `uint8_t` in C++.

#pragma once

#include <cstdint>
#include <stdint.h>   // guarantee global ::u?int*_t under header-unit include-translation

namespace rusty {

// ─── Predicates (mirrors Rust's u8::is_ascii_X) ────────────────────

inline constexpr bool is_ascii(uint8_t byte) { return byte <= 0x7F; }

inline constexpr bool is_ascii_alphabetic(uint8_t byte) {
    return (byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z');
}

inline constexpr bool is_ascii_uppercase(uint8_t byte) {
    return byte >= 'A' && byte <= 'Z';
}

inline constexpr bool is_ascii_lowercase(uint8_t byte) {
    return byte >= 'a' && byte <= 'z';
}

// is_ascii_digit lives in string.hpp historically; redeclare guarded.
#ifndef RUSTY_HAS_IS_ASCII_DIGIT
#define RUSTY_HAS_IS_ASCII_DIGIT 1
inline constexpr bool is_ascii_digit(uint8_t byte) {
    return byte >= '0' && byte <= '9';
}
#endif

inline constexpr bool is_ascii_alphanumeric(uint8_t byte) {
    return is_ascii_alphabetic(byte) || is_ascii_digit(byte);
}

inline constexpr bool is_ascii_octdigit(uint8_t byte) {
    return byte >= '0' && byte <= '7';
}

inline constexpr bool is_ascii_hexdigit(uint8_t byte) {
    return is_ascii_digit(byte)
        || (byte >= 'a' && byte <= 'f')
        || (byte >= 'A' && byte <= 'F');
}

inline constexpr bool is_ascii_punctuation(uint8_t byte) {
    return (byte >= '!' && byte <= '/')
        || (byte >= ':' && byte <= '@')
        || (byte >= '[' && byte <= '`')
        || (byte >= '{' && byte <= '~');
}

inline constexpr bool is_ascii_graphic(uint8_t byte) {
    return byte >= '!' && byte <= '~';
}

inline constexpr bool is_ascii_whitespace(uint8_t byte) {
    // Rust's definition: HT (0x09), LF (0x0A), FF (0x0C), CR (0x0D), SP (0x20).
    // NOTE: Rust's set is intentionally narrower than C's isspace().
    return byte == ' ' || byte == '\t' || byte == '\n'
        || byte == '\x0c' || byte == '\r';
}

inline constexpr bool is_ascii_control(uint8_t byte) {
    return byte <= 0x1F || byte == 0x7F;
}

// ─── Case folding (mirrors Rust's u8::to_ascii_X) ──────────────────

inline constexpr uint8_t to_ascii_uppercase(uint8_t byte) {
    return is_ascii_lowercase(byte) ? static_cast<uint8_t>(byte - 0x20) : byte;
}

inline constexpr uint8_t to_ascii_lowercase(uint8_t byte) {
    return is_ascii_uppercase(byte) ? static_cast<uint8_t>(byte + 0x20) : byte;
}

inline constexpr bool eq_ignore_ascii_case(uint8_t a, uint8_t b) {
    return to_ascii_lowercase(a) == to_ascii_lowercase(b);
}

} // namespace rusty
