#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
pub fn adapted(v: Vec<u8>) {}

pub fn consumer() {
    fn adapted(v: Vec<u8>) {
        assert!(v.is_empty());
    }
    adapted(Vec::new());
}
