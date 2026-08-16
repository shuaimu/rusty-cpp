#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
pub fn adapted(v: Vec<u8>) {}

pub fn consumer() {
    unsafe extern "Rust" {
        fn adapted(v: Vec<u8>);
    }
    unsafe {
        adapted(Vec::new());
    }
}
