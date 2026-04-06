// Tests for rusty range helpers used by transpiled iterator lowering
#include "../include/rusty/array.hpp"
#include "../include/rusty/io.hpp"
#include "../include/rusty/slice.hpp"

#include <cassert>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <optional>
#include <type_traits>
#include <utility>

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

void test_range_bounds_helpers_shape() {
    printf("test_range_bounds_helpers_shape: ");
    {
        auto closed = rusty::range<size_t>(2, 5);
        auto closed_start = closed.start_bound();
        auto closed_end = closed.end_bound();
        assert(std::holds_alternative<rusty::Bound_Included<size_t>>(closed_start));
        assert(std::get<rusty::Bound_Included<size_t>>(closed_start)._0 == 2);
        assert(std::holds_alternative<rusty::Bound_Excluded<size_t>>(closed_end));
        assert(std::get<rusty::Bound_Excluded<size_t>>(closed_end)._0 == 5);

        auto inclusive = rusty::range_inclusive<size_t>(7, 9);
        auto inclusive_end = inclusive.end_bound();
        assert(std::holds_alternative<rusty::Bound_Included<size_t>>(inclusive_end));
        assert(std::get<rusty::Bound_Included<size_t>>(inclusive_end)._0 == 9);

        auto from = rusty::range_from<size_t>{10};
        auto from_end = from.end_bound();
        assert(std::holds_alternative<rusty::Bound_Unbounded<size_t>>(from_end));

        auto to = rusty::range_to<size_t>{4};
        auto to_start = to.start_bound();
        assert(std::holds_alternative<rusty::Bound_Unbounded<size_t>>(to_start));

        auto full = rusty::range_full{};
        auto full_start = full.start_bound<>();
        auto full_end = full.end_bound<>();
        assert(std::holds_alternative<rusty::Bound_Unbounded<size_t>>(full_start));
        assert(std::holds_alternative<rusty::Bound_Unbounded<size_t>>(full_end));
    }
    printf("PASS\n");
}

void test_saturating_math_helpers_shape() {
    printf("test_saturating_math_helpers_shape: ");
    {
        assert(rusty::saturating_add<size_t>(4, 3) == 7);
        assert(rusty::saturating_add<size_t>(std::numeric_limits<size_t>::max(), 1)
               == std::numeric_limits<size_t>::max());
        assert(rusty::saturating_sub<size_t>(3, 5) == 0);

        assert(rusty::saturating_add<int32_t>(20, 22) == 42);
        assert(rusty::saturating_add<int32_t>(std::numeric_limits<int32_t>::max(), 1)
               == std::numeric_limits<int32_t>::max());
        assert(rusty::saturating_sub<int32_t>(std::numeric_limits<int32_t>::min(), 1)
               == std::numeric_limits<int32_t>::min());
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

struct SliceOnlyContainer {
    std::array<int, 4> backing{1, 2, 3, 4};

    std::span<int> as_mut_slice() { return std::span<int>(backing); }
    std::span<const int> as_slice() const { return std::span<const int>(backing); }
    size_t len() const { return backing.size(); }
};

void test_slice_full_prefers_as_slice_helpers_shape() {
    printf("test_slice_full_prefers_as_slice_helpers_shape: ");
    SliceOnlyContainer container{};

    auto mut_span = rusty::slice_full(container);
    static_assert(std::is_same_v<decltype(mut_span), std::span<int>>);
    mut_span[1] = 42;
    assert(container.backing[1] == 42);

    const SliceOnlyContainer& const_container = container;
    auto const_span = rusty::slice_full(const_container);
    static_assert(std::is_same_v<decltype(const_span), std::span<const int>>);
    assert(const_span[1] == 42);

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

void test_slice_iter_helpers_shape() {
    printf("test_slice_iter_helpers_shape: ");
    std::vector<int> data{1, 2, 3};

    auto iter = rusty::iter(std::span<const int>(data));
    auto first = iter.next();
    assert(first.is_some());
    assert(*first.unwrap() == 1);

    auto back = iter.next_back();
    assert(back.is_some());
    assert(*back.unwrap() == 3);

    auto hint = iter.size_hint();
    assert(std::get<0>(hint) == 1);
    auto upper = std::move(std::get<1>(hint));
    assert(upper.is_some());
    assert(upper.unwrap() == 1);

    auto cloned = rusty::iter(std::span<const int>(data)).cloned();
    auto c0 = cloned.next();
    assert(c0.is_some());
    assert(c0.unwrap() == 1);

    auto mut_iter = rusty::iter(std::span<int>(data));
    auto mut_first = mut_iter.next();
    assert(mut_first.is_some());
    *mut_first.unwrap() = 9;
    assert(data[0] == 9);

    printf("PASS\n");
}

void test_cursor_new_helper_shape() {
    printf("test_cursor_new_helper_shape: ");
    auto cursor = rusty::io::cursor_new(std::vector<uint8_t>{7, 8, 9});
    uint8_t out[2] = {0, 0};
    auto read_res = cursor.read(std::span<uint8_t>(out, 2));
    assert(read_res.is_ok());
    assert(read_res.unwrap() == 2);
    assert(out[0] == 7);
    assert(out[1] == 8);
    printf("PASS\n");
}

void test_filter_map_lazy_shape() {
    printf("test_filter_map_lazy_shape: ");
    std::array<int, 4> values{1, 2, 3, 4};
    int calls = 0;
    auto view = rusty::filter_map(values, [&](int value) -> std::optional<int> {
        ++calls;
        if (value % 2 == 0) {
            return value * 10;
        }
        return std::nullopt;
    });

    assert(calls == 0);
    std::vector<int> out;
    for (int value : view) {
        out.push_back(value);
    }
    assert(calls == 4);
    assert(out.size() == 2);
    assert(out[0] == 20);
    assert(out[1] == 40);
    printf("PASS\n");
}

void test_filter_map_span_shape() {
    printf("test_filter_map_span_shape: ");
    const std::array<int, 3> values{3, 4, 5};
    std::span<const int> span(values);

    auto view = rusty::filter_map(span, [](int value) -> std::optional<int> {
        if (value > 3) {
            return value;
        }
        return std::nullopt;
    });

    std::vector<int> out;
    for (int value : view) {
        out.push_back(value);
    }
    assert(out.size() == 2);
    assert(out[0] == 4);
    assert(out[1] == 5);
    printf("PASS\n");
}

struct OptionalCounterIter {
    int cur = 0;

    std::optional<int> next() {
        if (cur >= 4) {
            return std::nullopt;
        }
        return cur++;
    }
};

struct RustyOptionCounterIter {
    int cur = 1;

    rusty::Option<int> next() {
        if (cur > 3) {
            return rusty::None;
        }
        return rusty::Option<int>(cur++);
    }
};

void test_for_in_map_fold_optional_next_shape() {
    printf("test_for_in_map_fold_optional_next_shape: ");
    {
        std::vector<int> seen;
        for (auto&& value : rusty::for_in(OptionalCounterIter{})) {
            seen.push_back(value);
        }
        assert((seen == std::vector<int>{0, 1, 2, 3}));
    }
    {
        auto mapped = rusty::map(OptionalCounterIter{}, [](int value) { return value + 10; });
        int sum = rusty::fold(std::move(mapped), 0, rusty::ops::add_fn);
        assert(sum == 46);
    }
    printf("PASS\n");
}

void test_for_in_map_fold_rusty_option_next_shape() {
    printf("test_for_in_map_fold_rusty_option_next_shape: ");
    auto mapped = rusty::map(RustyOptionCounterIter{}, [](int value) { return value * 2; });
    int sum = rusty::fold(std::move(mapped), 0, rusty::ops::add_fn);
    assert(sum == 12);
    printf("PASS\n");
}

void test_take_iterator_adapter_shape() {
    printf("test_take_iterator_adapter_shape: ");
    {
        auto range = rusty::range(0, 10);
        std::vector<int> seen;
        for (auto&& value : rusty::for_in(rusty::take(range, 5))) {
            seen.push_back(value);
        }
        assert((seen == std::vector<int>{0, 1, 2, 3, 4}));
        auto next = range.next();
        assert(next.has_value());
        assert(*next == 5);
    }
    {
        auto mapped = rusty::map(rusty::take(OptionalCounterIter{}, 2), [](int value) {
            return value + 1;
        });
        int sum = rusty::fold(std::move(mapped), 0, rusty::ops::add_fn);
        assert(sum == 3);
    }
    printf("PASS\n");
}

void test_maybe_uninit_reference_pointer_shape() {
    printf("test_maybe_uninit_reference_pointer_shape: ");
    using RefSlot = rusty::MaybeUninit<const int&>;
    static_assert(std::is_same_v<decltype(std::declval<RefSlot&>().as_mut_ptr()), const int*>);
    static_assert(
        std::is_same_v<decltype(std::declval<const RefSlot&>().as_ptr()), const int*>);
    printf("PASS\n");
}

void test_io_print_shim_shape() {
    printf("test_io_print_shim_shape: ");
    rusty::io::_print();
    rusty::io::_print(123, "abc");
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty range helpers ===\n");

    test_range_next_and_count();
    test_range_from_next_and_count_shape();
    test_range_bounds_helpers_shape();
    test_saturating_math_helpers_shape();
    test_slice_helpers_basic_shapes();
    test_slice_full_prefers_as_slice_helpers_shape();
    test_len_helper_shapes();
    test_span_equality_helper_shape();
    test_slice_iter_helpers_shape();
    test_cursor_new_helper_shape();
    test_filter_map_lazy_shape();
    test_filter_map_span_shape();
    test_for_in_map_fold_optional_next_shape();
    test_for_in_map_fold_rusty_option_next_shape();
    test_take_iterator_adapter_shape();
    test_maybe_uninit_reference_pointer_shape();
    test_io_print_shim_shape();

    printf("\nAll rusty range tests passed!\n");
    return 0;
}
