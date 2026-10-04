#![feature(prelude_import)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod types {
    #[allow(unused_imports)]
    pub use lion_utility_spec::view_types::*;
}
pub mod events {
    use vstd::prelude::*;
    use crate::types::*;
    pub enum InboundCall {
        RegisterIoResource {
            source: SourceView,
            interest: InterestView,
            result: Option<IoResultView<ResourceIdView>>,
        },
        DeregisterIoResource {
            resource_id: ResourceIdView,
            result: Option<IoResultView<()>>,
        },
        SetWaker {
            resource_id: ResourceIdView,
            interest: InterestView,
            waker: WakerView,
            result: Option<IoResultView<()>>,
        },
        RegisterTimer {
            deadline: InstantView,
            waker: WakerView,
            result: Option<IoResultView<ResourceIdView>>,
        },
        DeregisterTimer { resource_id: ResourceIdView, result: bool },
        Park { timeout: Option<DurationView>, result: Option<IoResultView<()>> },
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for InboundCall {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for InboundCall {
        #[inline]
        fn eq(&self, other: &InboundCall) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        InboundCall::RegisterIoResource {
                            source: __self_0,
                            interest: __self_1,
                            result: __self_2,
                        },
                        InboundCall::RegisterIoResource {
                            source: __arg1_0,
                            interest: __arg1_1,
                            result: __arg1_2,
                        },
                    ) => {
                        __self_0 == __arg1_0 && __self_1 == __arg1_1
                            && __self_2 == __arg1_2
                    }
                    (
                        InboundCall::DeregisterIoResource {
                            resource_id: __self_0,
                            result: __self_1,
                        },
                        InboundCall::DeregisterIoResource {
                            resource_id: __arg1_0,
                            result: __arg1_1,
                        },
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    (
                        InboundCall::SetWaker {
                            resource_id: __self_0,
                            interest: __self_1,
                            waker: __self_2,
                            result: __self_3,
                        },
                        InboundCall::SetWaker {
                            resource_id: __arg1_0,
                            interest: __arg1_1,
                            waker: __arg1_2,
                            result: __arg1_3,
                        },
                    ) => {
                        __self_0 == __arg1_0 && __self_1 == __arg1_1
                            && __self_2 == __arg1_2 && __self_3 == __arg1_3
                    }
                    (
                        InboundCall::RegisterTimer {
                            deadline: __self_0,
                            waker: __self_1,
                            result: __self_2,
                        },
                        InboundCall::RegisterTimer {
                            deadline: __arg1_0,
                            waker: __arg1_1,
                            result: __arg1_2,
                        },
                    ) => {
                        __self_0 == __arg1_0 && __self_1 == __arg1_1
                            && __self_2 == __arg1_2
                    }
                    (
                        InboundCall::DeregisterTimer {
                            resource_id: __self_0,
                            result: __self_1,
                        },
                        InboundCall::DeregisterTimer {
                            resource_id: __arg1_0,
                            result: __arg1_1,
                        },
                    ) => __self_1 == __arg1_1 && __self_0 == __arg1_0,
                    (
                        InboundCall::Park { timeout: __self_0, result: __self_1 },
                        InboundCall::Park { timeout: __arg1_0, result: __arg1_1 },
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for InboundCall {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<SourceView>;
            let _: ::core::cmp::AssertParamIsEq<InterestView>;
            let _: ::core::cmp::AssertParamIsEq<Option<IoResultView<ResourceIdView>>>;
            let _: ::core::cmp::AssertParamIsEq<ResourceIdView>;
            let _: ::core::cmp::AssertParamIsEq<Option<IoResultView<()>>>;
            let _: ::core::cmp::AssertParamIsEq<WakerView>;
            let _: ::core::cmp::AssertParamIsEq<Option<IoResultView<()>>>;
            let _: ::core::cmp::AssertParamIsEq<InstantView>;
            let _: ::core::cmp::AssertParamIsEq<Option<IoResultView<ResourceIdView>>>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
            let _: ::core::cmp::AssertParamIsEq<Option<DurationView>>;
            let _: ::core::cmp::AssertParamIsEq<Option<IoResultView<()>>>;
        }
    }
    pub enum OutboundCall {
        RegisterIoResource {
            source: SourceView,
            resource_id: ResourceIdView,
            interest: InterestView,
            result: IoResultView<()>,
        },
        DeregisterIoResource {
            source: SourceView,
            resource_id: ResourceIdView,
            result: IoResultView<()>,
        },
        PollEvents { timeout: Option<DurationView>, result: IoResultView<nat> },
        IoEventReady { event: IoEventView },
        GetCurrentTime { timestamp: InstantView },
        WakeTask { waker: WakerView, source_rid: ResourceIdView },
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for OutboundCall {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for OutboundCall {
        #[inline]
        fn eq(&self, other: &OutboundCall) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        OutboundCall::RegisterIoResource {
                            source: __self_0,
                            resource_id: __self_1,
                            interest: __self_2,
                            result: __self_3,
                        },
                        OutboundCall::RegisterIoResource {
                            source: __arg1_0,
                            resource_id: __arg1_1,
                            interest: __arg1_2,
                            result: __arg1_3,
                        },
                    ) => {
                        __self_0 == __arg1_0 && __self_1 == __arg1_1
                            && __self_2 == __arg1_2 && __self_3 == __arg1_3
                    }
                    (
                        OutboundCall::DeregisterIoResource {
                            source: __self_0,
                            resource_id: __self_1,
                            result: __self_2,
                        },
                        OutboundCall::DeregisterIoResource {
                            source: __arg1_0,
                            resource_id: __arg1_1,
                            result: __arg1_2,
                        },
                    ) => {
                        __self_0 == __arg1_0 && __self_1 == __arg1_1
                            && __self_2 == __arg1_2
                    }
                    (
                        OutboundCall::PollEvents { timeout: __self_0, result: __self_1 },
                        OutboundCall::PollEvents { timeout: __arg1_0, result: __arg1_1 },
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    (
                        OutboundCall::IoEventReady { event: __self_0 },
                        OutboundCall::IoEventReady { event: __arg1_0 },
                    ) => __self_0 == __arg1_0,
                    (
                        OutboundCall::GetCurrentTime { timestamp: __self_0 },
                        OutboundCall::GetCurrentTime { timestamp: __arg1_0 },
                    ) => __self_0 == __arg1_0,
                    (
                        OutboundCall::WakeTask { waker: __self_0, source_rid: __self_1 },
                        OutboundCall::WakeTask { waker: __arg1_0, source_rid: __arg1_1 },
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for OutboundCall {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<SourceView>;
            let _: ::core::cmp::AssertParamIsEq<ResourceIdView>;
            let _: ::core::cmp::AssertParamIsEq<InterestView>;
            let _: ::core::cmp::AssertParamIsEq<IoResultView<()>>;
            let _: ::core::cmp::AssertParamIsEq<IoResultView<()>>;
            let _: ::core::cmp::AssertParamIsEq<Option<DurationView>>;
            let _: ::core::cmp::AssertParamIsEq<IoResultView<nat>>;
            let _: ::core::cmp::AssertParamIsEq<IoEventView>;
            let _: ::core::cmp::AssertParamIsEq<InstantView>;
            let _: ::core::cmp::AssertParamIsEq<WakerView>;
        }
    }
    pub enum ReactorEvent {
        Inbound(InboundCall),
        Outbound(OutboundCall),
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for ReactorEvent {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ReactorEvent {
        #[inline]
        fn eq(&self, other: &ReactorEvent) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        ReactorEvent::Inbound(__self_0),
                        ReactorEvent::Inbound(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    (
                        ReactorEvent::Outbound(__self_0),
                        ReactorEvent::Outbound(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for ReactorEvent {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<InboundCall>;
            let _: ::core::cmp::AssertParamIsEq<OutboundCall>;
        }
    }
}
pub mod log {
    use vstd::prelude::*;
    use crate::events::*;
    #[allow(unused_imports)]
    use crate::types::*;
    pub type Log = Seq<ReactorEvent>;
}
pub mod invariants {
    pub mod timer_deadline_future {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod park_has_timestamp {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod park_poll_once {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod io_ready_in_park {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod timer_waker_validity {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod io_waker_validity {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod timer_reg_uniqueness {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod io_reg_uniqueness {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod timer_io_disjoint {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod register_io_in_cycle {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod deregister_io_in_cycle {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod inbound_register_io_result {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod inbound_deregister_io_result {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod wake_has_registration {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod set_waker_active_io {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod wake_on_expired {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::local_liveness::*;
    }
    pub mod wake_on_io_ready {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::local_liveness::*;
    }
}
pub mod contracts {
    pub mod bounded_timer_wakeup {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::async_contract::*;
    }
    pub mod bounded_io_wakeup {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        #[allow(unused_imports)]
        use crate::types::*;
        use lion_framework_spec::async_contract::*;
    }
}
pub mod bridge {
    use vstd::prelude::*;
    use crate::log::*;
    use crate::events::*;
    #[allow(unused_imports)]
    use crate::types::*;
    use crate::invariants::inbound_register_io_result::*;
    use crate::invariants::inbound_deregister_io_result::*;
}
