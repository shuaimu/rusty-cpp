// Tests for rusty range helpers used by transpiled iterator lowering
#include "../include/rusty/array.hpp"

#include <cassert>
#include <cstdio>
#include <limits>

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

int main() {
    printf("=== Testing rusty range helpers ===\n");

    test_range_next_and_count();
    test_range_from_next_and_count_shape();

    printf("\nAll rusty range tests passed!\n");
    return 0;
}
