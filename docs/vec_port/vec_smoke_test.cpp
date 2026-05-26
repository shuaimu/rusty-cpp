// vec_port Phase E smoke test — exercise more Vec operations.

import vec_port;

#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

int main() {
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    std::printf("constructed Vec<int>; size hint: %zu, len_field: %zu\n",
                sizeof(v), v.len_field);

    // Try to call .len() if exposed (Rust style)
    if constexpr (requires { v.len(); }) {
        std::printf("len() = %zu\n", v.len());
    } else {
        std::printf("len() not available; using len_field = %zu\n", v.len_field);
    }

    // Try push if available
    if constexpr (requires { v.push(42); }) {
        v.push(42);
        v.push(7);
        std::printf("after push: len_field = %zu\n", v.len_field);
    } else {
        std::printf("push() not in Vec API\n");
    }

    return 0;
}
