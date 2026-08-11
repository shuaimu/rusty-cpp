use macro_dep::pretend_assert as r#assert;

#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    assert!(false);
    bytes
}
