// Parity test: rusty::fmt output must be byte-identical to Rust's `format!`.
//
// Reads the checked-in golden fixture (tests/fmt_parity/golden.txt, produced by
// tests/fmt_parity/gen_golden.rs against the real Rust toolchain) and, for each
// case id, reproduces the output through rusty::fmt and compares hex-encoded
// bytes. Rust is the oracle; any divergence fails the test.
//
// NOTE: only the rusty::fmt LIBRARY is no-std; this TEST may use std freely.
#include "../include/rusty/fmt_rt.hpp"

#include <cstdint>
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

namespace rt = rusty::fmt::rt;

// Render via an arbitrary formatting callback and return the raw bytes.
std::string render(const FormatSpec& spec, const std::function<void(Formatter&)>& fn) {
    Buffer buf;
    Formatter f(buf, spec);
    fn(f);
    auto v = buf.view();
    return std::string(v.data(), v.size());
}

// Render a string through `Formatter::pad` with the given spec.
std::string padded(std::string_view value, const FormatSpec& spec) {
    return render(spec, [&](Formatter& f) { f.pad(value); });
}

FormatSpec spec_flags(bool alt = false, bool plus = false, bool zero = false) {
    FormatSpec s;
    s.alternate = alt;
    s.sign_plus = plus;
    s.sign_aware_zero_pad = zero;
    return s;
}

FormatSpec spec_zero_width(std::size_t w) {
    FormatSpec s;
    s.has_width = true;
    s.width = w;
    s.sign_aware_zero_pad = true;
    s.fill = '0';
    return s;
}

FormatSpec spec_alt() {
    FormatSpec s;
    s.alternate = true;
    return s;
}

// A user type with a Debug `fmt` — exercises the nested-builder dispatch and the
// pretty-print indentation (debug_value routes to `value.fmt(f)`).
struct CppPoint {
    int x;
    int y;
    rusty::fmt::Result fmt(Formatter& f) const {
        return f.debug_struct("Point").field("x", x).field("y", y).finish();
    }
};

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

    // Phase 1: integers — decimal Display.
    r["int_42"] = [] { return render({}, [](Formatter& f) { rt::fmt_int(f, 42); }); };
    r["int_neg5"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int(f, static_cast<std::int32_t>(-5)); });
    };
    r["int_zero"] = [] { return render({}, [](Formatter& f) { rt::fmt_int(f, 0); }); };
    r["int_plus"] = [] {
        return render(spec_flags(false, true), [](Formatter& f) { rt::fmt_int(f, 42); });
    };
    r["int_plus_neg"] = [] {
        return render(spec_flags(false, true),
                      [](Formatter& f) { rt::fmt_int(f, static_cast<std::int32_t>(-5)); });
    };
    r["int_width6"] = [] {
        return render(width(6), [](Formatter& f) { rt::fmt_int(f, 42); });
    };
    r["int_width6_left"] = [] {
        return render(width(6, Alignment::Left), [](Formatter& f) { rt::fmt_int(f, 42); });
    };
    r["int_zeropad"] = [] {
        return render(spec_zero_width(6), [](Formatter& f) { rt::fmt_int(f, 42); });
    };
    r["int_zeropad_neg"] = [] {
        return render(spec_zero_width(6),
                      [](Formatter& f) { rt::fmt_int(f, static_cast<std::int32_t>(-42)); });
    };
    r["int_prec4"] = [] {
        return render(precision(4), [](Formatter& f) { rt::fmt_int(f, 42); });
    };
    r["int_width8_prec4"] = [] {
        return render(width_prec(8, 4, Alignment::Unknown),
                      [](Formatter& f) { rt::fmt_int(f, 42); });
    };
    r["int_u64max"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int(f, static_cast<std::uint64_t>(18446744073709551615ULL)); });
    };
    r["int_i64min"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int(f, static_cast<std::int64_t>(INT64_MIN)); });
    };
    r["int_fill_star"] = [] {
        return render(width(6, Alignment::Right, '*'), [](Formatter& f) { rt::fmt_int(f, 42); });
    };

    // Phase 1: integers — radix.
    r["hex_255"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::LowerHex); });
    };
    r["hex_upper_255"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::UpperHex); });
    };
    r["hex_alt_255"] = [] {
        return render(spec_flags(true),
                      [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::LowerHex); });
    };
    r["hex_neg5_i32"] = [] {
        return render({}, [](Formatter& f) {
            rt::fmt_int_radix(f, static_cast<std::int32_t>(-5), rt::Base::LowerHex);
        });
    };
    r["oct_8"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int_radix(f, 8u, rt::Base::Octal); });
    };
    r["oct_alt_8"] = [] {
        return render(spec_flags(true),
                      [](Formatter& f) { rt::fmt_int_radix(f, 8u, rt::Base::Octal); });
    };
    r["bin_5"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_int_radix(f, 5u, rt::Base::Binary); });
    };
    r["bin_alt_5"] = [] {
        return render(spec_flags(true),
                      [](Formatter& f) { rt::fmt_int_radix(f, 5u, rt::Base::Binary); });
    };
    r["hex_zeropad_alt"] = [] {
        FormatSpec s = spec_zero_width(6);
        s.alternate = true;
        return render(s, [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::LowerHex); });
    };
    r["hex_width8"] = [] {
        return render(width(8), [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::LowerHex); });
    };
    r["hex_prec4"] = [] {
        return render(precision(4),
                      [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::LowerHex); });
    };

    // Phase 1: bool.
    r["bool_true"] = [] { return render({}, [](Formatter& f) { rt::fmt_bool(f, true); }); };
    r["bool_false_dbg"] = [] { return render({}, [](Formatter& f) { rt::fmt_bool(f, false); }); };
    r["bool_width8"] = [] {
        return render(width(8), [](Formatter& f) { rt::fmt_bool(f, true); });
    };

    // Phase 1: str Debug.
    r["str_dbg_plain"] = [] { return render({}, [](Formatter& f) { rt::fmt_str_debug(f, "hi"); }); };
    r["str_dbg_escape"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_str_debug(f, "a\nb\"c\\d"); });
    };
    r["str_dbg_tab"] = [] { return render({}, [](Formatter& f) { rt::fmt_str_debug(f, "\t"); }); };
    r["str_dbg_unicode"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_str_debug(f, "caf\xc3\xa9"); });
    };

    // Phase 1: char Display + Debug.
    r["char_disp"] = [] { return render({}, [](Formatter& f) { rt::fmt_char_display(f, U'A'); }); };
    r["char_disp_unicode"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_char_display(f, U'é'); });
    };
    r["char_disp_width"] = [] {
        return render(width(3), [](Formatter& f) { rt::fmt_char_display(f, U'x'); });
    };
    r["char_dbg"] = [] { return render({}, [](Formatter& f) { rt::fmt_char_debug(f, U'A'); }); };
    r["char_dbg_newline"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_char_debug(f, U'\n'); });
    };
    r["char_dbg_quote"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_char_debug(f, U'\''); });
    };
    r["char_dbg_unicode"] = [] {
        return render({}, [](Formatter& f) { rt::fmt_char_debug(f, U'é'); });
    };

    // Phase 2: Debug builders.
    auto sv = [](const char* s) { return std::string_view(s); };
    r["dbg_struct"] = [] {
        return render({}, [](Formatter& f) {
            f.debug_struct("Point").field("x", 1).field("y", 2).finish();
        });
    };
    r["dbg_struct_pretty"] = [] {
        return render(spec_alt(), [](Formatter& f) {
            f.debug_struct("Point").field("x", 1).field("y", 2).finish();
        });
    };
    r["dbg_struct_empty"] = [] {
        return render({}, [](Formatter& f) { f.debug_struct("Empty").finish(); });
    };
    r["dbg_tuple"] = [sv] {
        return render({}, [sv](Formatter& f) {
            f.debug_tuple("Wrap").field(1).field(sv("hi")).finish();
        });
    };
    r["dbg_tuple_pretty"] = [sv] {
        return render(spec_alt(), [sv](Formatter& f) {
            f.debug_tuple("Wrap").field(1).field(sv("hi")).finish();
        });
    };
    r["dbg_list"] = [] {
        return render({}, [](Formatter& f) {
            f.debug_list().entry(1).entry(2).entry(3).finish();
        });
    };
    r["dbg_list_pretty"] = [] {
        return render(spec_alt(), [](Formatter& f) {
            f.debug_list().entry(1).entry(2).entry(3).finish();
        });
    };
    r["dbg_list_empty"] = [] {
        return render({}, [](Formatter& f) { f.debug_list().finish(); });
    };
    r["dbg_set"] = [] {
        return render({}, [](Formatter& f) {
            f.debug_set().entry(1).entry(2).entry(3).finish();
        });
    };
    r["dbg_map"] = [sv] {
        return render({}, [sv](Formatter& f) {
            f.debug_map().entry(1, sv("a")).entry(2, sv("b")).finish();
        });
    };
    r["dbg_map_pretty"] = [sv] {
        return render(spec_alt(), [sv](Formatter& f) {
            f.debug_map().entry(1, sv("a")).entry(2, sv("b")).finish();
        });
    };
    r["dbg_nested"] = [sv] {
        return render({}, [sv](Formatter& f) {
            f.debug_struct("Outer").field("inner", CppPoint{1, 2}).field("label", sv("hi")).finish();
        });
    };
    r["dbg_nested_pretty"] = [sv] {
        return render(spec_alt(), [sv](Formatter& f) {
            f.debug_struct("Outer").field("inner", CppPoint{1, 2}).field("label", sv("hi")).finish();
        });
    };
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
