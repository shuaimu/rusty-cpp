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
    ];

    for (id, out) in &cases {
        println!("{}|{}", id, hex(out));
    }
}
