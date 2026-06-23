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

}  // namespace

int main() {
    printf("=== rusty::fmt Phase 0 coverage ===\n");
    test_buffer_growth();
    test_write_str_and_char();
    test_pad_plain();
    test_pad_precision_truncates();
    test_pad_width_align();
    test_pad_custom_fill();
    test_pad_precision_then_width();
    printf("All rusty::fmt Phase 0 tests passed.\n");
    return 0;
}
