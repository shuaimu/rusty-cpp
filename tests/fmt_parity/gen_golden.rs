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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}
#[derive(Debug)]
struct Wrap(i32, &'static str);
#[derive(Debug)]
struct Empty;
#[derive(Debug)]
struct Outer {
    inner: Point,
    label: &'static str,
}

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
        // Phase 2: Debug builders.
        ("dbg_struct", format!("{:?}", Point { x: 1, y: 2 })),
        ("dbg_struct_pretty", format!("{:#?}", Point { x: 1, y: 2 })),
        ("dbg_struct_empty", format!("{:?}", Empty)),
        ("dbg_tuple", format!("{:?}", Wrap(1, "hi"))),
        ("dbg_tuple_pretty", format!("{:#?}", Wrap(1, "hi"))),
        ("dbg_list", format!("{:?}", [1, 2, 3])),
        ("dbg_list_pretty", format!("{:#?}", [1, 2, 3])),
        ("dbg_list_empty", format!("{:?}", Vec::<i32>::new())),
        ("dbg_set", format!("{:?}", BTreeSet::from([1, 2, 3]))),
        ("dbg_map", format!("{:?}", BTreeMap::from([(1, "a"), (2, "b")]))),
        ("dbg_map_pretty", format!("{:#?}", BTreeMap::from([(1, "a"), (2, "b")]))),
        ("dbg_nested", format!("{:?}", Outer { inner: Point { x: 1, y: 2 }, label: "hi" })),
        ("dbg_nested_pretty", format!("{:#?}", Outer { inner: Point { x: 1, y: 2 }, label: "hi" })),
        // Phase 3: f64 Display (always positional).
        ("f_disp_whole", format!("{}", 100.0f64)),
        ("f_disp_one", format!("{}", 1.0f64)),
        ("f_disp_frac", format!("{}", 12.34f64)),
        ("f_disp_half", format!("{}", 0.5f64)),
        ("f_disp_small", format!("{}", 0.00125f64)),
        ("f_disp_third", format!("{}", 0.3f64)),
        ("f_disp_pi", format!("{}", std::f64::consts::PI)),
        ("f_disp_neg", format!("{}", -3.14f64)),
        ("f_disp_negzero", format!("{}", -0.0f64)),
        ("f_disp_zero", format!("{}", 0.0f64)),
        ("f_disp_big", format!("{}", 1e21f64)),
        ("f_disp_e16", format!("{}", 1e16f64)),
        ("f_disp_tiny", format!("{}", 1e-5f64)),
        ("f_disp_123456789", format!("{}", 123456789.0f64)),
        // Phase 3: f64 Debug (positional with ".0", flips to scientific).
        ("f_dbg_whole", format!("{:?}", 100.0f64)),
        ("f_dbg_one", format!("{:?}", 1.0f64)),
        ("f_dbg_frac", format!("{:?}", 12.34f64)),
        ("f_dbg_half", format!("{:?}", 0.5f64)),
        ("f_dbg_negzero", format!("{:?}", -0.0f64)),
        ("f_dbg_zero", format!("{:?}", 0.0f64)),
        ("f_dbg_e15", format!("{:?}", 1e15f64)),
        ("f_dbg_e16", format!("{:?}", 1e16f64)),
        ("f_dbg_e20", format!("{:?}", 1e20f64)),
        ("f_dbg_e_minus4", format!("{:?}", 1e-4f64)),
        ("f_dbg_e_minus5", format!("{:?}", 1e-5f64)),
        ("f_dbg_small_frac", format!("{:?}", 0.000123f64)),
        ("f_dbg_big_mantissa", format!("{:?}", 12345678901234567.0f64)),
        // Phase 3: scientific {:e} / {:E}.
        ("f_exp_1234", format!("{:e}", 1234.5f64)),
        ("f_exp_whole", format!("{:e}", 100.0f64)),
        ("f_exp_small", format!("{:e}", 0.00125f64)),
        ("f_exp_zero", format!("{:e}", 0.0f64)),
        ("f_exp_neg", format!("{:e}", -3.14f64)),
        ("f_Exp_upper", format!("{:E}", 1234.5f64)),
        ("f_exp_pi", format!("{:e}", std::f64::consts::PI)),
        // Phase 3: width / fill / sign / zero-pad on floats.
        ("f_width8", format!("{:8}", 3.14f64)),
        ("f_width8_left", format!("{:<8}", 3.14f64)),
        ("f_zeropad", format!("{:08.2}", 3.14f64)),
        ("f_zeropad_neg", format!("{:08.2}", -3.14f64)),
        ("f_plus", format!("{:+}", 3.14f64)),
        ("f_plus_neg", format!("{:+}", -3.14f64)),
        ("f_fill_star", format!("{:*>8}", 3.14f64)),
        // Phase 3: non-finite.
        ("f_nan", format!("{}", f64::NAN)),
        ("f_nan_dbg", format!("{:?}", f64::NAN)),
        ("f_nan_plus", format!("{:+}", f64::NAN)),
        ("f_inf", format!("{}", f64::INFINITY)),
        ("f_neg_inf", format!("{}", f64::NEG_INFINITY)),
        ("f_inf_plus", format!("{:+}", f64::INFINITY)),
        ("f_inf_width8", format!("{:8}", f64::INFINITY)),
        ("f_inf_exp", format!("{:e}", f64::INFINITY)),
        // Phase 3: f32 (own shortest, fewer digits than the widened f64).
        ("f32_tenth", format!("{}", 0.1f32)),
        ("f32_dbg_tenth", format!("{:?}", 0.1f32)),
        ("f32_third", format!("{}", 0.3f32)),
        ("f32_pi", format!("{}", std::f32::consts::PI)),
        ("f32_big", format!("{}", 1e20f32)),
        // Phase 3b: fixed precision {:.N} (positional, round-half-to-even).
        ("fp_pi2", format!("{:.2}", std::f64::consts::PI)),
        ("fp_pi5", format!("{:.5}", std::f64::consts::PI)),
        ("fp_whole2", format!("{:.2}", 100.0f64)),
        ("fp_zero3", format!("{:.3}", 0.0f64)),
        ("fp_negzero2", format!("{:.2}", -0.0f64)),
        ("fp_neg2", format!("{:.2}", -3.14159f64)),
        ("fp_small5", format!("{:.5}", 0.00125f64)),
        ("fp_round0_half", format!("{:.0}", 0.5f64)),
        ("fp_round0_1p5", format!("{:.0}", 1.5f64)),
        ("fp_round0_2p5", format!("{:.0}", 2.5f64)),
        ("fp_round0_3p7", format!("{:.0}", 3.7f64)),
        ("fp_round2_125", format!("{:.2}", 0.125f64)),
        ("fp_round2_375", format!("{:.2}", 0.375f64)),
        ("fp_big2", format!("{:.2}", 12345.678f64)),
        ("fp_dbg2", format!("{:.2?}", 3.14159f64)),
        ("fp_dbg0_whole", format!("{:.0?}", 5.0f64)),
        ("fp_zeropad", format!("{:08.2}", 3.14159f64)),
        ("fp_zeropad_neg", format!("{:08.2}", -3.14159f64)),
        ("fp_width", format!("{:10.3}", 3.14159f64)),
        ("fp_f32_2", format!("{:.2}", 0.1f32)),
    ];

    // Phase 3: exponent sweep — pins the Debug positional/scientific switch
    // (scientific iff exp = decimal_exponent-1 is < -4 or >= 16) across the
    // whole range, and Display staying positional throughout.
    let mut cases = cases;
    for e in -7i32..23 {
        let v = 10f64.powi(e);
        cases.push((Box::leak(format!("f_sweep_dbg_{}", e + 7).into_boxed_str()), format!("{:?}", v)));
        cases.push((Box::leak(format!("f_sweep_disp_{}", e + 7).into_boxed_str()), format!("{}", v)));
    }

    for (id, out) in &cases {
        println!("{}|{}", id, hex(out));
    }
}
