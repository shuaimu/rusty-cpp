// Tests for rusty range helpers used by transpiled iterator lowering
#include "../include/rusty/array.hpp"

#include <cassert>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <type_traits>

void test_range_next_and_count() {
    printf("test_range_next_and_count: ");
    {
        auto r = rusty::range(0, 3);

        auto n0 = r.next();
        assert(n0.has_value());
        assert(*n0 == 0);
        assert(r.count() == 2);

        auto n1 = r.next();
        assert(n1.has_value());
        assert(*n1 == 1);
        assert(r.count() == 1);

        auto n2 = r.next();
        assert(n2.has_value());
        assert(*n2 == 2);
        assert(r.count() == 0);

        auto n3 = r.next();
        assert(!n3.has_value());
        assert(r.count() == 0);
    }
    printf("PASS\n");
}

void test_range_from_next_and_count_shape() {
    printf("test_range_from_next_and_count_shape: ");
    {
        auto r = rusty::range_from(5);

        auto n0 = r.next();
        assert(n0.has_value());
        assert(*n0 == 5);

        auto n1 = r.next();
        assert(n1.has_value());
        assert(*n1 == 6);

        assert(r.count() == std::numeric_limits<size_t>::max());
    }
    printf("PASS\n");
}

void test_slice_helpers_basic_shapes() {
    printf("test_slice_helpers_basic_shapes: ");
    std::vector<uint8_t> data{10, 11, 12, 13, 14};

    auto full = rusty::slice_full(data);
    static_assert(std::is_same_v<decltype(full), std::span<uint8_t>>);
    assert(full.size() == 5);
    assert(full[0] == 10);

    auto to = rusty::slice_to(data, 3);
    assert(to.size() == 3);
    assert(to[2] == 12);

    auto from = rusty::slice_from(data, 2);
    assert(from.size() == 3);
    assert(from[0] == 12);

    auto mid = rusty::slice(data, 1, 4);
    assert(mid.size() == 3);
    assert(mid[0] == 11);
    assert(mid[2] == 13);

    auto to_inclusive = rusty::slice_to_inclusive(data, 2);
    assert(to_inclusive.size() == 3);
    assert(to_inclusive[2] == 12);

    auto mid_inclusive = rusty::slice_inclusive(data, 1, 3);
    assert(mid_inclusive.size() == 3);
    assert(mid_inclusive[0] == 11);
    assert(mid_inclusive[2] == 13);

    const std::vector<uint8_t>& cdata = data;
    auto cfull = rusty::slice_full(cdata);
    static_assert(std::is_same_v<decltype(cfull), std::span<const uint8_t>>);
    assert(cfull.size() == 5);
    assert(cfull[4] == 14);

    printf("PASS\n");
}

void test_len_helper_shapes() {
    printf("test_len_helper_shapes: ");
    std::vector<uint8_t> data{1, 2, 3, 4};
    assert(rusty::len(data) == 4);

    auto full = rusty::slice_full(data);
    assert(rusty::len(full) == 4);

    int native[3] = {1, 2, 3};
    assert(rusty::len(native) == 3);

    struct HasLenMethod {
        size_t len() const { return 7; }
    } has_len;
    assert(rusty::len(has_len) == 7);

    printf("PASS\n");
}

void test_span_equality_helper_shape() {
    printf("test_span_equality_helper_shape: ");
    std::vector<uint8_t> data{1, 2, 3, 4};

    auto lhs = rusty::slice_full(data);
    auto rhs = rusty::slice_to(data, 4);
    auto rhs_short = rusty::slice_to(data, 3);

    assert(lhs == rhs);
    assert(!(lhs == rhs_short));

    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty range helpers ===\n");

    test_range_next_and_count();
    test_range_from_next_and_count_shape();
    test_slice_helpers_basic_shapes();
    test_len_helper_shapes();
    test_span_equality_helper_shape();

    printf("\nAll rusty range tests passed!\n");
    return 0;
}
