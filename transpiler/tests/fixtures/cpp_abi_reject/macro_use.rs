#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

macro_rules! call_ident {
    ($callee:ident, $arg:expr) => {
        $callee($arg)
    };
}

pub fn invoke(bytes: Vec<u8>) -> Vec<u8> {
    call_ident!(adapted, bytes)
}
