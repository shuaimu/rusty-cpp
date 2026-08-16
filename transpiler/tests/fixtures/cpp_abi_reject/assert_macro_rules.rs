macro_rules! r#assert {
    ($expression:expr) => {{
        let _ = $expression;
    }};
}

#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

pub fn checked() {
    r#assert!(false);
}
