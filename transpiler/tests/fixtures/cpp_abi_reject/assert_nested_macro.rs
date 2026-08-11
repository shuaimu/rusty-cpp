macro_rules! id_bool {
    ($expression:expr) => {
        $expression
    };
}

#[cfg_attr(any(), cpp_abi(param(value, std_string_bytes)))]
pub fn adapted(value: Vec<u8>) {
    let _ = value;
}

pub fn checked() {
    assert!(id_bool!(false));
}
