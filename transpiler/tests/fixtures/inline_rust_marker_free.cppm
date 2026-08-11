export module inline_rust_marker_free;

export namespace inline_rust_marker_free {

#if RUSTYCPP_RUST
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
#endif
/*RUSTYCPP:GEN-BEGIN id=inline_rust_marker_free.add version=1 rust_sha256=54cd830f0b802e508232035fe7b0a2c0e3b89ce86eb9328032c154e12653cd2f*/
int32_t add(int32_t a, int32_t b);

int32_t add(int32_t a, int32_t b) {
    return rusty::detail::deref_if_pointer_like(a) + rusty::detail::deref_if_pointer_like(b);
}
/*RUSTYCPP:GEN-END id=inline_rust_marker_free.add*/

} // export namespace inline_rust_marker_free
