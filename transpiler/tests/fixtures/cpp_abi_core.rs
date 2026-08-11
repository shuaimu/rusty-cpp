#[cfg_attr(any(), cpp_abi(
    param(bytes, std_string_bytes),
    returns(std_string_bytes)
))]
pub fn roundtrip(bytes: Vec<u8>) -> Vec<u8> {
    assert!(bytes.len() < 1024);
    bytes
}

pub struct Codec;

impl Codec {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn encode(value: u8) -> Vec<u8> {
        let mut result = Vec::with_capacity(3);
        result.push(value);
        result.push(0);
        result.push(255);
        result
    }
}

#[cfg_attr(any(), cpp_abi_alias(std_vector))]
pub type Weights = Vec<f64>;

pub struct Picker;

impl Picker {
    #[cfg_attr(any(), cpp_abi(param(weights, const_ref(Weights))))]
    pub fn choose(weights: &[f64]) -> u32 {
        weights.len() as u32
    }
}

pub mod r#private {
    #[cfg_attr(any(), cpp_abi(
        param(bytes, std_string_bytes),
        returns(std_string_bytes)
    ))]
    pub fn r#class(bytes: Vec<u8>) -> Vec<u8> {
        bytes
    }

    #[cfg_attr(any(), cpp_abi_alias(std_vector))]
    pub type r#static = Vec<f64>;

    pub struct r#struct;

    impl r#struct {
        #[cfg_attr(any(), cpp_abi(param(values, const_ref(r#static))))]
        pub fn pause(values: &[f64]) -> u32 {
            values.len() as u32
        }
    }
}
