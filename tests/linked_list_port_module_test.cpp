// Smoke test for linked_list_port — exercises the transpiled rustc
// LinkedList directly (no std::list bridge). API matches the Rust
// surface: `new_()`, `len()`, `is_empty()`, `push_back`, `push_front`,
// `pop_front`. Uses explicit return-code checks (not assert) because
// the test compile uses -DNDEBUG which would no-op asserts.
import linked_list_port;

#include <cstdio>
#include <cstdlib>

#define CHECK(expr, fmt, ...) do { \
    if (!(expr)) { \
        std::fprintf(stderr, "FAIL " #expr " — " fmt "\n", ##__VA_ARGS__); \
        return 1; \
    } \
} while (0)

int main() {
    auto ll = linked_list_port::LinkedList<int>::new_();
    CHECK(ll.is_empty(), "new_() not empty");
    CHECK(ll.len() == 0, "new_().len() = %zu, expected 0", ll.len());

    ll.push_back(1);
    ll.push_back(2);
    ll.push_front(0);
    CHECK(!ll.is_empty(), "after 3 pushes still empty");
    CHECK(ll.len() == 3, "len after 3 pushes = %zu, expected 3", ll.len());

    auto first = ll.pop_front();
    CHECK(first.is_some(), "pop_front from non-empty returned None");
    int v = first.unwrap();
    CHECK(v == 0, "pop_front = %d, expected 0 (most-recent push_front)", v);
    CHECK(ll.len() == 2, "len after pop_front = %zu, expected 2", ll.len());

    std::printf("linked_list_port smoke OK: push/pop/len round-trip\n");
    return 0;
}
