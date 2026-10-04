#![feature(prelude_import)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod view_types {
    use vstd::prelude::*;
    pub type ResourceIdView = nat;
    pub type WakerView = int;
    pub type InstantView = int;
    pub type DurationView = int;
    pub type InterestView = (bool, bool);
    pub type SourceView = int;
    pub type TokenView = nat;
    pub struct IoEventView {
        pub resource_id: nat,
        pub readable: bool,
        pub writable: bool,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for IoEventView {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for IoEventView {
        #[inline]
        fn eq(&self, other: &IoEventView) -> bool {
            self.readable == other.readable && self.writable == other.writable
                && self.resource_id == other.resource_id
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for IoEventView {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<nat>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
        }
    }
    pub enum IoResultView<T> {
        Ok(T),
        Err(int),
    }
    #[automatically_derived]
    impl<T> ::core::marker::StructuralPartialEq for IoResultView<T> {}
    #[automatically_derived]
    impl<T: ::core::cmp::PartialEq> ::core::cmp::PartialEq for IoResultView<T> {
        #[inline]
        fn eq(&self, other: &IoResultView<T>) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (IoResultView::Ok(__self_0), IoResultView::Ok(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (IoResultView::Err(__self_0), IoResultView::Err(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl<T: ::core::cmp::Eq> ::core::cmp::Eq for IoResultView<T> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<T>;
            let _: ::core::cmp::AssertParamIsEq<int>;
        }
    }
    impl<T> IoResultView<T> {}
}
pub mod framework {
    pub mod module_spec {
        #[allow(unused_imports)]
        pub use lion_framework_spec::module_spec::*;
    }
    pub mod async_contract {
        #[allow(unused_imports)]
        pub use lion_framework_spec::async_contract::*;
    }
    pub mod local_liveness {
        #[allow(unused_imports)]
        pub use lion_framework_spec::local_liveness::*;
    }
    pub mod action_safety {
        #[allow(unused_imports)]
        pub use lion_framework_spec::action_safety::*;
    }
    #[allow(unused_imports)]
    pub use module_spec::*;
    #[allow(unused_imports)]
    pub use async_contract::*;
    #[allow(unused_imports)]
    pub use local_liveness::*;
    #[allow(unused_imports)]
    pub use action_safety::*;
}
pub mod generic {
    pub mod types {
        use vstd::prelude::*;
        pub use crate::view_types::{
            ResourceIdView, WakerView, InstantView, DurationView, InterestView,
            SourceView, IoResultView,
        };
        pub enum TickResult<R> {
            Pending,
            Finished(R),
            Ongoing(R),
        }
    }
    pub mod events {
        use vstd::prelude::*;
        use crate::generic::types::*;
        pub enum UtilityInbound<M, R> {
            Tick { waker: WakerView, method: M, result: Option<TickResult<R>> },
        }
        pub enum UtilityOutbound {
            PassWaker { waker: WakerView },
            WakeWaker { waker: WakerView },
            CancelWaker { waker: WakerView },
            RegisterTimer {
                deadline: InstantView,
                waker: WakerView,
                result: Option<ResourceIdView>,
            },
            DeregisterTimer { resource_id: ResourceIdView, result: bool },
            RegisterIoResource {
                source: SourceView,
                interest: InterestView,
                result: Option<IoResultView<ResourceIdView>>,
            },
            DeregisterIoResource {
                resource_id: ResourceIdView,
                result: IoResultView<()>,
            },
            SetIoWaker {
                resource_id: ResourceIdView,
                interest: InterestView,
                waker: WakerView,
                result: IoResultView<()>,
            },
            Defer,
        }
        pub enum UtilityEvent<M, R> {
            Inbound(UtilityInbound<M, R>),
            Outbound(UtilityOutbound),
        }
    }
    pub mod log {
        use vstd::prelude::*;
        use crate::generic::events::*;
        use crate::generic::types::{ResourceIdView, WakerView};
        pub type Log<M, R> = Seq<UtilityEvent<M, R>>;
    }
    pub mod module_spec {
        use vstd::prelude::*;
        use crate::generic::log::Log;
        use crate::framework::module_spec::ModuleSpec;
    }
    pub mod invariants {
        use vstd::prelude::*;
        use crate::generic::events::*;
        use crate::generic::log::*;
        use crate::framework::action_safety::*;
    }
    pub mod contract {
        use vstd::prelude::*;
        use crate::generic::types::*;
        use crate::generic::events::*;
        use crate::generic::log::*;
        use crate::framework::async_contract::*;
    }
    pub mod extension {
        use vstd::prelude::*;
        use crate::framework::action_safety::*;
        use crate::view_types::*;
        use crate::generic::events::*;
        use crate::generic::log::*;
        use crate::generic::invariants::*;
    }
}
