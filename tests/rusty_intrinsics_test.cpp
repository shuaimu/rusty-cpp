// Runtime tests for integer/range intrinsics added for hashbrown:
//   - rusty::num::NonZero<T>::trailing_zeros / leading_zeros / count_ones
//   - rusty::range<T>::step_by (Rust's Range::step_by)
#include "../include/rusty/rusty.hpp"

#include <cassert>
#include <cstdint>
#include <cstdio>

namespace {

void test_nonzero_bit_methods() {
    printf("test_nonzero_bit_methods: ");
    using rusty::num::NonZero;
    // 0x0140 = 0b0000'0001'0100'0000: lowest set bit is bit 6, two bits set.
    NonZero<std::uint16_t> nz(static_cast<std::uint16_t>(0x0140));
    assert(nz.trailing_zeros() == 6);
    assert(nz.count_ones() == 2);
    assert(nz.leading_zeros() == 7);  // bit 8 is the highest set bit (0-indexed)

    NonZero<std::uint16_t> one(static_cast<std::uint16_t>(1));
    assert(one.trailing_zeros() == 0);
    assert(one.leading_zeros() == 15);
    assert(one.count_ones() == 1);

    NonZero<std::uint32_t> high(static_cast<std::uint32_t>(0x8000'0000u));
    assert(high.trailing_zeros() == 31);
    assert(high.leading_zeros() == 0);
    assert(high.count_ones() == 1);
    printf("PASS\n");
}

void test_range_step_by_range_for() {
    printf("test_range_step_by_range_for: ");
    int sum = 0, n = 0;
    for (auto&& i : rusty::range<unsigned long>(0, 10).step_by(3)) {  // 0,3,6,9
        sum += static_cast<int>(i);
        ++n;
    }
    assert(n == 4 && sum == 0 + 3 + 6 + 9);

    // Step that overshoots: 0..5 step 10 -> just {0}.
    n = 0;
    for (auto&& i : rusty::range<unsigned long>(0, 5).step_by(10)) {
        (void)i;
        ++n;
    }
    assert(n == 1);

    // Empty range -> no iterations.
    n = 0;
    for (auto&& i : rusty::range<unsigned long>(5, 5).step_by(2)) {
        (void)i;
        ++n;
    }
    assert(n == 0);
    printf("PASS\n");
}

void test_range_step_by_next_protocol() {
    printf("test_range_step_by_next_protocol: ");
    auto sb = rusty::range<unsigned long>(0, 10).step_by(3);
    assert(sb.next().unwrap() == 0);
    assert(sb.next().unwrap() == 3);
    assert(sb.next().unwrap() == 6);
    assert(sb.next().unwrap() == 9);
    assert(sb.next().is_none());
    printf("PASS\n");
}

}  // namespace

int main() {
    printf("=== rusty intrinsics (NonZero bit-ops, range::step_by) ===\n");
    test_nonzero_bit_methods();
    test_range_step_by_range_for();
    test_range_step_by_next_protocol();
    printf("All rusty intrinsics tests passed.\n");
    return 0;
}
