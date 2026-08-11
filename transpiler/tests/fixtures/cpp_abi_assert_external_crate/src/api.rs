#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> bool {
    !bytes.is_empty()
}
