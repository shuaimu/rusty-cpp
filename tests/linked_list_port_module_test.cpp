// Smoke test for linked_list_port — exercises the transpiled rustc
// LinkedList. Phase B+C: push/pop/peek API surface.
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
    // ── Phase B: push / pop / len round-trip ────────────────────────
    auto ll = linked_list_port::LinkedList<int>::new_();
    CHECK(ll.is_empty(), "new_() not empty");
    CHECK(ll.len() == 0, "new_().len() = %zu", ll.len());

    ll.push_back(1);
    ll.push_back(2);
    ll.push_front(0);
    CHECK(ll.len() == 3, "len after 3 pushes = %zu, expected 3", ll.len());

    // ── Phase C: front() / back() peek (without mutating) ───────────
    {
        auto f = ll.front();
        auto b = ll.back();
        CHECK(f.is_some() && f.unwrap() == 0, "front() = %d", f.is_some() ? f.unwrap() : -1);
        CHECK(b.is_some() && b.unwrap() == 2, "back() = %d", b.is_some() ? b.unwrap() : -1);
        CHECK(ll.len() == 3, "peek changed len: %zu", ll.len());
    }

    // ── Phase B: pop ────────────────────────────────────────────────
    auto first = ll.pop_front();
    CHECK(first.is_some(), "pop_front from non-empty returned None");
    CHECK(first.unwrap() == 0, "pop_front = %d, expected 0", first.unwrap());
    CHECK(ll.len() == 2, "len after pop_front = %zu, expected 2", ll.len());

    std::printf("linked_list_port smoke OK: Phase B (push/pop/len) + "
                "Phase C (front/back peek)\n");
    return 0;
}
