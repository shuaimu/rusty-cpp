pub struct Owner;
impl Owner {
    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
    pub fn adapted(v: Vec<u8>) {}
}

pub fn consumer() {
    struct Owner;
    impl Owner {
        fn adapted(_: Vec<u8>) {}
    }
    Owner::adapted(Vec::new());
}
