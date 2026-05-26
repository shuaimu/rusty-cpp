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

    std::printf("right before as_slice: len=%zu capacity=%zu\n", v.len(), v.capacity());
    auto slice2 = v.as_slice();
    std::printf("post-realloc slice (size=%zu): ", slice2.size());
    for (size_t i = 0; i < slice2.size(); ++i) std::printf("%d ", slice2[i]);
    std::printf("\n");
    CHECK(slice2[0] == 10, "slice2[0] still 10 after reallocs");
    CHECK(slice2[1] == 20, "slice2[1] still 20 after reallocs");
    CHECK(slice2[21] == 19 * 100, "last element correct");

    // Clear
    v.clear();
    CHECK(v.len() == 0, "len after clear");
    CHECK(v.is_empty(), "is_empty after clear");

    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
