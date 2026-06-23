// Parity test: rusty::fmt output must be byte-identical to Rust's `format!`.
//
// Reads the checked-in golden fixture (tests/fmt_parity/golden.txt, produced by
// tests/fmt_parity/gen_golden.rs against the real Rust toolchain) and, for each
// case id, reproduces the output through rusty::fmt and compares hex-encoded
// bytes. Rust is the oracle; any divergence fails the test.
//
// NOTE: only the rusty::fmt LIBRARY is no-std; this TEST may use std freely.
#include "../include/rusty/fmt_rt.hpp"

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <functional>
#include <map>
#include <sstream>
#include <string>
#include <string_view>

using rusty::fmt::rt::Alignment;
using rusty::fmt::rt::Buffer;
using rusty::fmt::rt::Formatter;
using rusty::fmt::rt::FormatSpec;

namespace {

std::string to_hex(std::string_view bytes) {
    static const char* digits = "0123456789abcdef";
    std::string out;
    out.reserve(bytes.size() * 2);
    for (unsigned char c : bytes) {
        out.push_back(digits[c >> 4]);
        out.push_back(digits[c & 0xF]);
    }
    return out;
}

// Render a string through `Formatter::pad` with the given spec and return the
// raw bytes (as a std::string copy so it outlives the buffer).
std::string padded(std::string_view value, const FormatSpec& spec) {
    Buffer buf;
    Formatter f(buf, spec);
    f.pad(value);
    auto v = buf.view();
    return std::string(v.data(), v.size());
}

FormatSpec width(std::size_t w, Alignment a = Alignment::Unknown, char fill = ' ') {
    FormatSpec s;
    s.has_width = true;
    s.width = w;
    s.align = a;
    s.fill = fill;
    return s;
}

FormatSpec precision(std::size_t p) {
    FormatSpec s;
    s.has_precision = true;
    s.precision = p;
    return s;
}

FormatSpec width_prec(std::size_t w, std::size_t p, Alignment a) {
    FormatSpec s;
    s.has_width = true;
    s.width = w;
    s.has_precision = true;
    s.precision = p;
    s.align = a;
    return s;
}

// The C++ reproduction of each golden case, keyed by id.
std::map<std::string, std::function<std::string()>> reproductions() {
    std::map<std::string, std::function<std::string()>> r;
    r["plain_hi"] = [] { return padded("hi", {}); };
    r["plain_empty"] = [] { return padded("", {}); };
    r["plain_unicode"] = [] { return padded("h\xc3\xa9llo", {}); };  // "héllo" UTF-8
    r["w8_hi"] = [] { return padded("hi", width(8)); };
    r["w8_right_hi"] = [] { return padded("hi", width(8, Alignment::Right)); };
    r["w8_left_hi"] = [] { return padded("hi", width(8, Alignment::Left)); };
    r["w8_center_hi"] = [] { return padded("hi", width(8, Alignment::Center)); };
    r["w7_center_hi"] = [] { return padded("hi", width(7, Alignment::Center)); };
    r["fill_star_right"] = [] { return padded("hi", width(6, Alignment::Right, '*')); };
    r["fill_star_left"] = [] { return padded("hi", width(6, Alignment::Left, '*')); };
    r["fill_star_center"] = [] { return padded("hi", width(7, Alignment::Center, '*')); };
    r["prec3_hello"] = [] { return padded("hello", precision(3)); };
    r["prec0_hello"] = [] { return padded("hello", precision(0)); };
    r["prec10_hello"] = [] { return padded("hello", precision(10)); };
    r["w6_prec3_right"] = [] { return padded("hello", width_prec(6, 3, Alignment::Right)); };
    r["w8_under_len"] = [] { return padded("hi", width(1)); };
    r["w6_left_default"] = [] { return padded("hi", width(6)); };
    return r;
}

}  // namespace

int main(int argc, char** argv) {
    const char* golden_path =
        argc > 1 ? argv[1] : "tests/fmt_parity/golden.txt";
    std::ifstream in(golden_path);
    if (!in) {
        std::fprintf(stderr, "FAIL: cannot open golden fixture '%s'\n", golden_path);
        return 2;
    }

    std::map<std::string, std::string> golden;  // id -> hex
    std::string line;
    while (std::getline(in, line)) {
        if (line.empty()) {
            continue;
        }
        auto bar = line.find('|');
        if (bar == std::string::npos) {
            continue;
        }
        golden[line.substr(0, bar)] = line.substr(bar + 1);
    }

    auto repros = reproductions();
    int failures = 0;
    int checked = 0;

    for (auto& [id, hex_expected] : golden) {
        auto it = repros.find(id);
        if (it == repros.end()) {
            std::fprintf(stderr, "FAIL: golden case '%s' has no C++ reproduction\n", id.c_str());
            ++failures;
            continue;
        }
        std::string actual_hex = to_hex(it->second());
        ++checked;
        if (actual_hex != hex_expected) {
            std::fprintf(stderr,
                         "FAIL %-18s expected=%s actual=%s\n",
                         id.c_str(), hex_expected.c_str(), actual_hex.c_str());
            ++failures;
        }
    }

    // Also flag C++ cases that aren't in the golden set (drift the other way).
    for (auto& [id, _] : repros) {
        if (golden.find(id) == golden.end()) {
            std::fprintf(stderr, "FAIL: C++ case '%s' missing from golden fixture\n", id.c_str());
            ++failures;
        }
    }

    std::printf("rusty::fmt parity: %d cases checked, %d failures\n", checked, failures);
    return failures == 0 ? 0 : 1;
}
