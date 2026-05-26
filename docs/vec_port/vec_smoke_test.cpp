// vec_port Phase E smoke test — exercise more Vec operations.

import vec_port;

#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg) do { \
    if (!(cond)) { std::printf("FAIL: %s (at line %d)\n", msg, __LINE__); std::exit(1); } \
} while (0)

int main() {
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    std::printf("constructed Vec<int>; size hint: %zu\n", sizeof(v));

    CHECK(v.len() == 0, "fresh Vec len == 0");
    CHECK(v.is_empty(), "fresh Vec is_empty");
    CHECK(v.capacity() == 0, "fresh Vec capacity == 0");

    // Push values - triggers growth path
    v.push(10);
    v.push(20);
    v.push(30);
    CHECK(v.len() == 3, "len after 3 pushes");
    CHECK(!v.is_empty(), "non-empty after pushes");
    CHECK(v.capacity() >= 3, "capacity grew");

    // Read back via as_slice
    auto slice = v.as_slice();
    CHECK(slice.size() == 3, "slice size matches len");
    CHECK(slice[0] == 10, "slice[0] == 10");
    CHECK(slice[1] == 20, "slice[1] == 20");
    CHECK(slice[2] == 30, "slice[2] == 30");
    std::printf("read-back via as_slice: [%d, %d, %d]\n", slice[0], slice[1], slice[2]);

    // Pop
    auto popped = v.pop();
    CHECK(popped.is_some(), "pop returns Some");
    CHECK(popped.unwrap() == 30, "popped value == 30");
    CHECK(v.len() == 2, "len after pop");

    // Push more to trigger reallocation
    for (int i = 0; i < 20; ++i) {
        v.push(i * 100);
    }
    CHECK(v.len() == 22, "len after batch push");
    CHECK(v.capacity() >= 22, "capacity grew further");
    std::printf("after 20 more pushes: len=%zu, capacity=%zu\n",
                v.len(), v.capacity());

    auto slice2 = v.as_slice();
    CHECK(slice2[0] == 10, "slice2[0] still 10 after reallocs");
    CHECK(slice2[1] == 20, "slice2[1] still 20 after reallocs");
    CHECK(slice2[21] == 19 * 100, "last element correct");

    // truncate
    v.truncate(5);
    CHECK(v.len() == 5, "len after truncate(5)");
    CHECK(v.as_slice()[4] == 200, "element 4 still 200 after truncate");

    // insert / remove
    v.insert(0, 999);
    CHECK(v.len() == 6, "len after insert");
    CHECK(v.as_slice()[0] == 999, "inserted at front");
    CHECK(v.as_slice()[1] == 10, "shifted right");

    auto removed = v.remove(0);
    CHECK(removed == 999, "remove returns inserted value");
    CHECK(v.len() == 5, "len after remove");
    CHECK(v.as_slice()[0] == 10, "shifted back left");

    // swap_remove
    v.push(7777);  // len=6, last is 7777
    auto sw = v.swap_remove(1);  // remove [1]=20, last (7777) moves to [1]
    CHECK(sw == 20, "swap_remove returns 20");
    CHECK(v.len() == 5, "len after swap_remove");
    CHECK(v.as_slice()[1] == 7777, "swap_remove swapped last in");

    // clear
    v.clear();
    CHECK(v.len() == 0, "len after clear");
    CHECK(v.is_empty(), "is_empty after clear");

    // reserve / extend_from_slice
    v.reserve(100);
    CHECK(v.capacity() >= 100, "reserve grew capacity");
    int seed[] = {1, 2, 3, 4, 5};
    v.extend_from_slice(std::span<const int>(seed, 5));
    CHECK(v.len() == 5, "extend_from_slice added 5");
    CHECK(v.as_slice()[0] == 1, "extended[0]");
    CHECK(v.as_slice()[4] == 5, "extended[4]");

    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
