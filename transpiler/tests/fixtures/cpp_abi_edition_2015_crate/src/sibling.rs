use outer as namespace;

pub fn call(bytes: Vec<u8>) -> Vec<u8> {
    namespace::api::adapted(bytes)
}
