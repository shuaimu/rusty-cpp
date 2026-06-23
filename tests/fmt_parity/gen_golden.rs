// Parity golden generator for the self-contained rusty::fmt runtime.
//
// Emits, for each named case, the EXACT byte output of Rust's own `format!`,
// hex-encoded so any fill char / non-printable is captured unambiguously:
//
//     <case_id>|<hex-of-format!-output>
//
// The C++ parity test (tests/rusty_fmt_parity_test.cpp) reproduces each case
// through `rusty::fmt` and asserts byte-identical output. Rust is the oracle.
//
// Regenerate the checked-in fixture with:
//     rustc -O tests/fmt_parity/gen_golden.rs -o /tmp/gen_golden \
//         && /tmp/gen_golden > tests/fmt_parity/golden.txt
//
// As later phases land (ints, floats, debug builders), add cases here AND the
// matching reproduction in the C++ test. Once the runtime format-string parser
// exists, the C++ side can parse the same format string and the sync becomes
// automatic.

use std::fmt::Write as _;

fn hex(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        write!(out, "{:02x}", b).unwrap();
    }
    out
}

fn main() {
    // Phase 0: string Display through `Formatter::pad` (width / precision /
    // fill / alignment). Each entry is (case_id, format! output).
    let cases: Vec<(&str, String)> = vec![
        ("plain_hi", format!("{}", "hi")),
        ("plain_empty", format!("{}", "")),
        ("plain_unicode", format!("{}", "héllo")),
        ("w8_hi", format!("{:8}", "hi")),
        ("w8_right_hi", format!("{:>8}", "hi")),
        ("w8_left_hi", format!("{:<8}", "hi")),
        ("w8_center_hi", format!("{:^8}", "hi")),
        ("w7_center_hi", format!("{:^7}", "hi")),
        ("fill_star_right", format!("{:*>6}", "hi")),
        ("fill_star_left", format!("{:*<6}", "hi")),
        ("fill_star_center", format!("{:*^7}", "hi")),
        ("prec3_hello", format!("{:.3}", "hello")),
        ("prec0_hello", format!("{:.0}", "hello")),
        ("prec10_hello", format!("{:.10}", "hello")),
        ("w6_prec3_right", format!("{:>6.3}", "hello")),
        ("w8_under_len", format!("{:1}", "hi")),
        ("w6_left_default", format!("{:6}", "hi")),
        // Phase 1: integers — decimal Display/Debug (sign + magnitude).
        ("int_42", format!("{}", 42)),
        ("int_neg5", format!("{}", -5i32)),
        ("int_zero", format!("{}", 0)),
        ("int_plus", format!("{:+}", 42)),
        ("int_plus_neg", format!("{:+}", -5i32)),
        ("int_width6", format!("{:6}", 42)),
        ("int_width6_left", format!("{:<6}", 42)),
        ("int_zeropad", format!("{:06}", 42)),
        ("int_zeropad_neg", format!("{:06}", -42i32)),
        ("int_prec4", format!("{:.4}", 42)),
        ("int_width8_prec4", format!("{:8.4}", 42)),
        ("int_u64max", format!("{}", u64::MAX)),
        ("int_i64min", format!("{}", i64::MIN)),
        ("int_fill_star", format!("{:*>6}", 42)),
        // Phase 1: integers — radix (raw bit pattern, unsigned).
        ("hex_255", format!("{:x}", 255)),
        ("hex_upper_255", format!("{:X}", 255)),
        ("hex_alt_255", format!("{:#x}", 255)),
        ("hex_neg5_i32", format!("{:x}", -5i32)),
        ("oct_8", format!("{:o}", 8)),
        ("oct_alt_8", format!("{:#o}", 8)),
        ("bin_5", format!("{:b}", 5)),
        ("bin_alt_5", format!("{:#b}", 5)),
        ("hex_zeropad_alt", format!("{:#06x}", 255)),
        ("hex_width8", format!("{:8x}", 255)),
        ("hex_prec4", format!("{:.4x}", 255)),
        // Phase 1: bool.
        ("bool_true", format!("{}", true)),
        ("bool_false_dbg", format!("{:?}", false)),
        ("bool_width8", format!("{:8}", true)),
        // Phase 1: str Debug (escaping).
        ("str_dbg_plain", format!("{:?}", "hi")),
        ("str_dbg_escape", format!("{:?}", "a\nb\"c\\d")),
        ("str_dbg_tab", format!("{:?}", "\t")),
        ("str_dbg_unicode", format!("{:?}", "café")),
        // Phase 1: char Display + Debug.
        ("char_disp", format!("{}", 'A')),
        ("char_disp_unicode", format!("{}", 'é')),
        ("char_disp_width", format!("{:3}", 'x')),
        ("char_dbg", format!("{:?}", 'A')),
        ("char_dbg_newline", format!("{:?}", '\n')),
        ("char_dbg_quote", format!("{:?}", '\'')),
        ("char_dbg_unicode", format!("{:?}", 'é')),
    ];

    for (id, out) in &cases {
        println!("{}|{}", id, hex(out));
    }
}
