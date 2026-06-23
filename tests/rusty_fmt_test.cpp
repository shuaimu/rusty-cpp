// Coverage tests for the self-contained rusty::fmt runtime (Phase 0).
//
// Phase 0 covers the sink (Buffer), the Formatter skeleton (write_str /
// write_char) and the string padding path (pad: precision + width + fill +
// align). Integer/float/debug-builder coverage is added as later phases land.
#include "../include/rusty/fmt_rt.hpp"

#include <cassert>
#include <cstdio>
#include <string_view>

using rusty::fmt::rt::Alignment;
using rusty::fmt::rt::Buffer;
using rusty::fmt::rt::Formatter;
using rusty::fmt::rt::FormatSpec;

namespace {

// Format `value` through `pad` with `spec` and return the bytes as a view into
// the (caller-owned) buffer.
std::string_view pad_with(Buffer& buf, std::string_view value, const FormatSpec& spec) {
    buf.clear();
    Formatter f(buf, spec);
    f.pad(value);
    return buf.view();
}

void test_buffer_growth() {
    printf("test_buffer_growth: ");
    Buffer buf;
    assert(buf.is_empty());
    // Push more than the initial capacity to force multiple grows.
    for (int i = 0; i < 1000; ++i) {
        buf.push_str("ab");
    }
    assert(buf.len() == 2000);
    auto v = buf.view();
    assert(v.size() == 2000);
    assert(v[0] == 'a' && v[1] == 'b' && v[1999] == 'b');
    buf.clear();
    assert(buf.is_empty());
    printf("PASS\n");
}

void test_write_str_and_char() {
    printf("test_write_str_and_char: ");
    Buffer buf;
    Formatter f(buf);
    f.write_str("he");
    f.write_char('l');
    f.write_char('l');
    f.write_str("o");
    assert(buf.view() == "hello");
    printf("PASS\n");
}

void test_pad_plain() {
    printf("test_pad_plain: ");
    Buffer buf;
    FormatSpec spec;  // no width, no precision
    assert(pad_with(buf, "hello", spec) == "hello");
    printf("PASS\n");
}

void test_pad_precision_truncates() {
    printf("test_pad_precision_truncates: ");
    Buffer buf;
    FormatSpec spec;
    spec.has_precision = true;
    spec.precision = 3;
    assert(pad_with(buf, "hello", spec) == "hel");
    // Precision larger than the string is a no-op.
    spec.precision = 10;
    assert(pad_with(buf, "hello", spec) == "hello");
    printf("PASS\n");
}

void test_pad_width_align() {
    printf("test_pad_width_align: ");
    Buffer buf;
    FormatSpec spec;
    spec.has_width = true;
    spec.width = 8;

    // Default (Unknown) for strings is left-aligned: "hi      "
    assert(pad_with(buf, "hi", spec) == "hi      ");

    spec.align = Alignment::Right;  // "      hi"
    assert(pad_with(buf, "hi", spec) == "      hi");

    spec.align = Alignment::Center;  // 6 pad split 3/3: "   hi   "
    assert(pad_with(buf, "hi", spec) == "   hi   ");

    // Odd padding centers with the extra on the right (Rust's behavior).
    spec.width = 7;  // 5 pad split 2/3
    assert(pad_with(buf, "hi", spec) == "  hi   ");

    // Width <= length is a no-op.
    spec.width = 1;
    spec.align = Alignment::Right;
    assert(pad_with(buf, "hi", spec) == "hi");
    printf("PASS\n");
}

void test_pad_custom_fill() {
    printf("test_pad_custom_fill: ");
    Buffer buf;
    FormatSpec spec;
    spec.has_width = true;
    spec.width = 6;
    spec.fill = '*';
    spec.align = Alignment::Right;
    assert(pad_with(buf, "hi", spec) == "****hi");
    spec.align = Alignment::Left;
    assert(pad_with(buf, "hi", spec) == "hi****");
    printf("PASS\n");
}

void test_pad_precision_then_width() {
    printf("test_pad_precision_then_width: ");
    Buffer buf;
    FormatSpec spec;
    spec.has_precision = true;
    spec.precision = 3;  // "hello" -> "hel"
    spec.has_width = true;
    spec.width = 6;      // then pad to 6
    spec.align = Alignment::Right;
    assert(pad_with(buf, "hello", spec) == "   hel");
    printf("PASS\n");
}

namespace rt = rusty::fmt::rt;

std::string_view render(Buffer& buf, const FormatSpec& spec,
                        void (*fn)(Formatter&)) {
    buf.clear();
    Formatter f(buf, spec);
    fn(f);
    return buf.view();
}

void test_int_decimal() {
    printf("test_int_decimal: ");
    Buffer buf;
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_int(f, 0); }) == "0");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_int(f, 42); }) == "42");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_int(f, -5); }) == "-5");
    // precision is ignored for integers.
    {
        FormatSpec s; s.has_precision = true; s.precision = 4;
        assert(render(buf, s, [](Formatter& f) { rt::fmt_int(f, 42); }) == "42");
    }
    // sign-aware zero pad keeps the sign outside the zeros.
    {
        FormatSpec s; s.has_width = true; s.width = 6;
        s.sign_aware_zero_pad = true; s.fill = '0';
        assert(render(buf, s, [](Formatter& f) { rt::fmt_int(f, -42); }) == "-00042");
    }
    printf("PASS\n");
}

void test_int_radix() {
    printf("test_int_radix: ");
    Buffer buf;
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::LowerHex); }) == "ff");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_int_radix(f, 255u, rt::Base::UpperHex); }) == "FF");
    // alternate adds the 0x/0o/0b prefix.
    {
        FormatSpec s; s.alternate = true;
        assert(render(buf, s, [](Formatter& f) { rt::fmt_int_radix(f, 5u, rt::Base::Binary); }) == "0b101");
    }
    // signed hex is the two's-complement bit pattern.
    assert(render(buf, {}, [](Formatter& f) {
        rt::fmt_int_radix(f, static_cast<int>(-5), rt::Base::LowerHex);
    }) == "fffffffb");
    printf("PASS\n");
}

void test_bool_char_strdebug() {
    printf("test_bool_char_strdebug: ");
    Buffer buf;
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_bool(f, true); }) == "true");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_str_debug(f, "a\"b"); }) == "\"a\\\"b\"");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_char_debug(f, U'\n'); }) == "'\\n'");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_char_display(f, U'A'); }) == "A");
    printf("PASS\n");
}

void test_float_basic() {
    printf("test_float_basic: ");
    Buffer buf;
    using rt::FloatStyle;
    // Display: always positional, whole numbers carry no ".0".
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 100.0, FloatStyle::Display); }) == "100");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 12.34, FloatStyle::Display); }) == "12.34");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 0.5, FloatStyle::Display); }) == "0.5");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, -0.0, FloatStyle::Display); }) == "-0");
    // Debug: positional whole numbers gain ".0", and flip to scientific at 1e16.
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 100.0, FloatStyle::Debug); }) == "100.0");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 1e16, FloatStyle::Debug); }) == "1e16");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 1e15, FloatStyle::Debug); }) == "1000000000000000.0");
    // Scientific.
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 1234.5, FloatStyle::LowerExp); }) == "1.2345e3");
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f64(f, 0.00125, FloatStyle::LowerExp); }) == "1.25e-3");
    // Non-finite: NaN never carries a sign, inf does.
    {
        FormatSpec s; s.sign_plus = true;
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, __builtin_nan(""), FloatStyle::Display); }) == "NaN");
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, __builtin_huge_val(), FloatStyle::Display); }) == "+inf");
    }
    // f32 has its own (shorter) shortest representation.
    assert(render(buf, {}, [](Formatter& f) { rt::fmt_f32(f, 0.1f, FloatStyle::Display); }) == "0.1");
    // Fixed precision {:.N}: round-half-to-even, with the point placed exactly.
    {
        FormatSpec s; s.has_precision = true; s.precision = 2;
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, 3.14159, FloatStyle::Display); }) == "3.14");
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, 100.0, FloatStyle::Display); }) == "100.00");
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, 0.125, FloatStyle::Display); }) == "0.12");
        s.precision = 0;
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, 2.5, FloatStyle::Display); }) == "2");
        assert(render(buf, s, [](Formatter& f) { rt::fmt_f64(f, 3.5, FloatStyle::Display); }) == "4");
    }
    printf("PASS\n");
}

}  // namespace

int main() {
    printf("=== rusty::fmt Phase 0+1 coverage ===\n");
    test_buffer_growth();
    test_write_str_and_char();
    test_pad_plain();
    test_pad_precision_truncates();
    test_pad_width_align();
    test_pad_custom_fill();
    test_pad_precision_then_width();
    test_int_decimal();
    test_int_radix();
    test_bool_char_strdebug();
    test_float_basic();
    printf("All rusty::fmt Phase 0+1+3 tests passed.\n");
    return 0;
}
