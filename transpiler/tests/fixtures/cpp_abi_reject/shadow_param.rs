#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

fn alternate(_: Vec<u8>) -> Vec<u8> {
    vec![99]
}

pub fn invoke(adapted: fn(Vec<u8>) -> Vec<u8>) -> Vec<u8> {
    adapted(vec![1])
}

pub fn run() -> Vec<u8> {
    invoke(alternate)
}
