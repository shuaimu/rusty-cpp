// Parity golden generator for the hand-written rusty iterator adapters.
//
// Rust's own iterators are the ORACLE. Each named case computes a value via an
// iterator-adapter chain and emits one line `<case_id>|<value>`, where <value>
// is a deterministic, format-independent rendering (comma-joined items for a
// sequence, or the plain number for a scalar) — so the comparison tests the
// ITERATOR behavior, not the formatting layer.
//
// The C++ parity test (tests/rusty_iter_parity_test.cpp) reproduces each case
// through rusty's iterators (`rusty::map`/`filter`/`chain`/...) and asserts the
// same value. A case present on only one side fails loudly, so drift is caught.
//
// Regenerate the checked-in fixture (needs a Rust toolchain):
//     rustc -O tests/iter_parity/gen_golden.rs -o /tmp/gen_iter_golden \
//         && /tmp/gen_iter_golden > tests/iter_parity/golden.txt

fn seq(v: Vec<i64>) -> String {
    v.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn main() {
    let mut out: Vec<(&str, String)> = Vec::new();
    out.push(("map", seq((1..=5).map(|x| x * 2).collect())));
    out.push(("filter", seq((1..=10).filter(|x| x % 2 == 0).collect())));
    out.push(("chain", seq((1..=3).chain(4..=6).collect())));
    out.push(("take", seq((1..=100).take(4).collect())));
    out.push(("skip", seq((1..=10).skip(7).collect())));
    out.push(("rev", seq((1..=5).rev().collect())));
    out.push(("step_by", seq((0..=10).step_by(3).collect())));
    out.push((
        "filter_map",
        seq((1..=6)
            .filter_map(|x| if x % 2 == 0 { Some(x * 10) } else { None })
            .collect()),
    ));
    out.push((
        "map_filter_chain",
        seq((1..=4)
            .map(|x| x * x)
            .chain((1..=3).filter(|x| x % 2 == 1))
            .collect()),
    ));
    out.push(("fold", (1..=5).fold(0i64, |a, x| a + x).to_string()));
    out.push(("count", ((1..=10).filter(|x| x % 3 == 0).count() as i64).to_string()));
    out.push(("sum", (1..=10).sum::<i64>().to_string()));
    for (id, val) in out {
        println!("{}|{}", id, val);
    }
}
