pub fn call(bytes: Vec<u8>) -> Vec<u8> {
    crate::api::adapted(bytes)
}

pub fn value() -> fn(Vec<u8>) -> Vec<u8> {
    crate::api::adapted
}
