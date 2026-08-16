#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::{
    helper, ChannelBase, ChannelError, ChannelFrame, ChannelProxy, ChannelTuple, ChannelUnit,
};
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::helper2;

pub mod external {
    #[repr(i32)]
    pub enum ChannelError {
        Foreign = 11,
    }

    #[repr(C)]
    pub struct ChannelFrame {
        pub foreign: i32,
    }
}

use self::external::ChannelFrame as OtherLeaf;

pub type ImportedFrame = ChannelFrame;

pub struct LocalChannel {
    pub value: i32,
}

pub struct AssociatedOwner;

impl AssociatedOwner {
    pub const ChannelFrame: i32 = 17;

    #[allow(non_snake_case)]
    pub fn ChannelTuple() -> i32 {
        19
    }
}

#[repr(i32)]
pub enum AssociatedKind {
    ChannelError = 23,
}

use self::AssociatedOwner as AssociatedAlias;

pub fn associated_const_same_tail() -> i32 {
    self::AssociatedOwner::ChannelFrame
}

pub fn associated_alias_const_same_tail() -> i32 {
    self::AssociatedAlias::ChannelFrame
}

pub fn associated_method_same_tail() -> i32 {
    self::AssociatedOwner::ChannelTuple()
}

pub fn associated_variant_same_tail() -> i32 {
    self::AssociatedKind::ChannelError as i32
}

#[cpp_inherit]
impl ChannelBase for LocalChannel {
    fn code(&self) -> i32 {
        self.value
    }
}

pub fn inspect(
    frame: &ChannelFrame,
    foreign: &external::ChannelFrame,
    renamed: &OtherLeaf,
) -> i32 {
    helper(frame.value) + helper2(0) + foreign.foreign + renamed.foreign
}

pub fn make_frame(value: i32) -> ChannelFrame {
    ChannelFrame { value }
}

pub fn destructure_frame(frame: ChannelFrame) -> i32 {
    let ChannelFrame { value } = frame;
    value
}

pub fn tuple_value(value: i32) -> i32 {
    let ChannelTuple(inner) = ChannelTuple(value);
    inner
}

pub fn unit_value() -> i32 {
    let _value = ChannelUnit;
    1
}

pub fn enum_value() -> ChannelError {
    ChannelError::None
}

pub fn external_enum_value() -> external::ChannelError {
    external::ChannelError::Foreign
}

pub fn make_external(value: i32) -> external::ChannelFrame {
    external::ChannelFrame { foreign: value }
}

pub fn inspect_self(value: &self::external::ChannelFrame) -> i32 {
    value.foreign
}

pub fn inspect_crate(value: &crate::consumer::external::ChannelFrame) -> i32 {
    value.foreign
}

pub fn external_enum_crate() -> crate::consumer::external::ChannelError {
    crate::consumer::external::ChannelError::Foreign
}

pub mod nested {
    pub fn inspect_super(value: &super::external::ChannelFrame) -> i32 {
        value.foreign
    }

    pub fn inspect_crate(value: &crate::consumer::external::ChannelFrame) -> i32 {
        value.foreign
    }
}

pub mod lexical_matrix {
    use super::{ChannelError, ChannelFrame, ChannelTuple, ChannelUnit};

    pub fn inspect_qualified(value: &super::ChannelFrame) -> i32 {
        value.value
    }

    pub fn inspect_imported(value: &ChannelFrame) -> i32 {
        value.value
    }

    pub fn construct_qualified(value: i32) -> i32 {
        super::ChannelFrame { value }.value
    }

    pub fn construct_imported(value: i32) -> i32 {
        let frame = ChannelFrame { value };
        let ChannelFrame { value } = frame;
        value
    }

    pub fn enum_qualified() -> i32 {
        super::ChannelError::None as i32
    }

    pub fn enum_imported() -> i32 {
        ChannelError::None as i32
    }

    pub fn tuple_qualified(value: i32) -> i32 {
        let super::ChannelTuple(inner) = super::ChannelTuple(value);
        inner
    }

    pub fn tuple_imported(value: i32) -> i32 {
        let ChannelTuple(inner) = ChannelTuple(value);
        inner
    }

    pub fn unit_qualified() -> i32 {
        let _value = super::ChannelUnit;
        2
    }

    pub fn unit_imported() -> i32 {
        let _value = ChannelUnit;
        3
    }

    pub fn inspect_generic<ChannelFrame>(_: &ChannelFrame) {}

    pub mod sibling_shadow {
        #[repr(C)]
        pub struct ChannelFrame {
            pub sibling: i32,
        }

        pub fn inspect_sibling(value: &ChannelFrame) -> i32 {
            value.sibling
        }
    }
}

pub fn clear_option(mut value: Option<i32>) -> Option<i32> {
    value = None;
    value
}

pub fn accept_nested(
    _frame: Option<Box<ChannelFrame>>,
    _proxy: Option<ChannelProxy>,
) -> ChannelError {
    ChannelError::None
}
