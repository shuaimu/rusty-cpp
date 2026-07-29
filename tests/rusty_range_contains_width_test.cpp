// Regression pin: range `contains` must not NARROW the queried item.
//
// Rust's RangeBounds::contains compares item and bounds with no
// conversion (`T: PartialOrd<U>`), and integer-literal inference makes a
// range's bound type match the item: `(-64..=63).contains(&x)` with
// `x: i64` is a range of i64 — truncation is unrepresentable.
//
// The transpiler emits literal-bound ranges with C++'s default literal
// type (`range_inclusive<int>` for `-64..=63`), so a `const T&`
// parameter silently narrowed a wider argument and answered WRONG
// instead of failing: 17179869183 truncates to -1, which really is
// inside [-64, 63]. srpc's varint `val_size` therefore picked a 1-byte
// encoding for 5-byte values, and the translated wire codec produced
// bytes the Rust original never would (caught by the three-way golden
// corpus, not by any compile-time check).
#include "../include/rusty/rusty.hpp"
#include "../include/rusty/array.hpp"

#include <cstdint>
#include <cstdio>
#include <string>

namespace {

int failures = 0;

void check(bool ok, const char* what) {
    if (!ok) {
        std::fprintf(stderr, "FAIL %s\n", what);
        ++failures;
    }
}

// The exact shape srpc's SparseInt val_size has: int-literal bounds,
// i64 item.
size_t val_size(int64_t val) {
    if (rusty::range_inclusive(-64, 63).contains(val)) return 1;
    if (rusty::range_inclusive(-8192, 8191).contains(val)) return 2;
    if (rusty::range_inclusive(-1048576, 1048575).contains(val)) return 3;
    if (rusty::range_inclusive(-134217728, 134217727).contains(val)) return 4;
    if (rusty::range_inclusive(-17179869184LL, 17179869183LL).contains(val)) return 5;
    return 9;
}

}  // namespace

int main() {
    // 1. The original failure: values outside an int-bounded range whose
    //    low 32 bits land inside it.
    check(!rusty::range_inclusive(-64, 63).contains(int64_t{17179869183}),
          "17179869183 (low bits -1) must be OUTSIDE [-64, 63]");
    check(!rusty::range_inclusive(-64, 63).contains(int64_t{-17179869184}),
          "-17179869184 (low bits 0) must be OUTSIDE [-64, 63]");
    check(val_size(17179869183LL) == 5, "val_size(2^34-1) == 5");
    check(val_size(-17179869184LL) == 5, "val_size(-2^34) == 5");
    check(val_size(0) == 1, "val_size(0) == 1");
    check(val_size(-64) == 1, "val_size(-64) == 1");
    check(val_size(64) == 2, "val_size(64) == 2");

    // 2. Same hazard on the exclusive/open range forms.
    check(!rusty::range(-64, 64).contains(int64_t{4294967296}),
          "range: 2^32 (low bits 0) must be OUTSIDE [-64, 64)");
    check(!rusty::range_to(64).contains(int64_t{4294967296}),
          "range_to: 2^32 must be OUTSIDE ..64");
    check(!rusty::range_to_inclusive(63).contains(int64_t{4294967296}),
          "range_to_inclusive: 2^32 must be OUTSIDE ..=63");
    check(rusty::range_from(0).contains(int64_t{4294967296}),
          "range_from: 2^32 must be INSIDE 0..");
    check(!rusty::range_from(0).contains(int64_t{-4294967296}),
          "range_from: -2^32 must be OUTSIDE 0..");

    // 3. Signed/unsigned mixes must not wrap either.
    check(!rusty::range_inclusive(0, 100).contains(int64_t{-1}),
          "-1 must be OUTSIDE [0, 100]");
    check(!rusty::range_inclusive(int64_t{0}, int64_t{100}).contains(uint64_t{18446744073709551615ull}),
          "u64::MAX must be OUTSIDE [0, 100]");
    check(rusty::range_inclusive(0u, 100u).contains(int32_t{50}),
          "50 must be INSIDE [0u, 100u]");
    check(!rusty::range_inclusive(0u, 100u).contains(int32_t{-1}),
          "-1 must be OUTSIDE [0u, 100u]");

    // 4. Same-type and non-integer paths keep working.
    check(rusty::range_inclusive(-64, 63).contains(0), "same-type int item");
    check(rusty::range_inclusive(0.0, 1.0).contains(0.5), "double range");
    check(!rusty::range_inclusive(0.0, 1.0).contains(1.5), "double range outside");
    check(rusty::range_inclusive(0.0, 2.0).contains(1), "int item vs double bounds");

    std::printf("range contains width-safety: %s (%d failures)\n",
                failures ? "FAIL" : "OK", failures);
    return failures == 0 ? 0 : 1;
}
