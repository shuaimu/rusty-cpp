extern crate self as this_crate;

pub fn call(bytes: Vec<u8>) -> Vec<u8> {
    this_crate::api::adapted(bytes)
}
