#[cfg_attr(any(), cpp_abi_alias(std_vector))]
pub type r#class = Vec<f64>;

pub struct Picker;
impl Picker {
    #[cfg_attr(any(), cpp_abi(param(values, const_ref(r#class))))]
    pub fn choose(values: &[f64]) -> u32 {
        values.len() as u32
    }
}

pub struct class_;
