#[repr(i32)]
#[cfg_attr(not(any()), derive(Clone, Copy))]
pub enum ChannelError {
    None = 0,
}

#[repr(C)]
pub struct ChannelFrame {
    pub value: i32,
}

#[repr(transparent)]
pub struct ChannelTuple(pub i32);

pub struct ChannelUnit;

pub trait ChannelBase {
    fn code(&self) -> i32;
}

pub type ChannelProxy = Box<dyn ChannelBase>;

pub fn helper(value: i32) -> i32 {
    value + 1
}

pub fn helper2(value: i32) -> i32 {
    value
}
