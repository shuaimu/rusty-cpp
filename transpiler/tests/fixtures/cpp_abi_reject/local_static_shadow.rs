#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
pub fn adapted(v: Vec<u8>) {}

fn alternate(_: Vec<u8>) {}

pub fn consumer() {
    static adapted: fn(Vec<u8>) = alternate;
    adapted(Vec::new());
}
