// Inline Rust DSL example in a .cpp file.
// V1 rule: this stays local to this translation unit unless declarations are
// surfaced manually in headers or module interfaces.
// In inline mode, includes are author-managed.

#include <cstdint>
#include <rusty/rusty.hpp>

#if RUSTYCPP_RUST
fn add(a: i32, b: i32) -> i32 {
    a + b
}
#endif
/*RUSTYCPP:GEN-BEGIN id=cmake_example.local.add version=1 rust_sha256=49395200c39034710abc77ad3d14a97a8d3be18a92994f8f03eeddc2021aa172*/
int32_t add(int32_t a, int32_t b);

int32_t add(int32_t a, int32_t b) {
    return rusty::detail::deref_if_pointer_like(a) + rusty::detail::deref_if_pointer_like(b);
}
/*RUSTYCPP:GEN-END id=cmake_example.local.add*/
