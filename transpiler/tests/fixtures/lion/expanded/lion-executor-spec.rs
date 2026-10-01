#![feature(prelude_import)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod types {
    use std::task::Poll as StdPoll;
    use vstd::prelude::*;
    pub type TID = nat;
    pub struct TaskView {
        pub id: TID,
    }
    pub enum PollResult<T> {
        Ready(T),
        Pending,
        Invalid,
    }
    impl<T> PollResult<T> {
        pub fn is_ready(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                PollResult::Ready(_) => true,
                _ => false,
            }
        }
        pub fn is_pending(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                PollResult::Pending => true,
                _ => false,
            }
        }
        pub fn is_invalid(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                PollResult::Invalid => true,
                _ => false,
            }
        }
    }
    impl<T: View> View for PollResult<T> {
        type V = PollResult<T::V>;
    }
    impl<T> From<StdPoll<T>> for PollResult<T> {
        fn from(poll: StdPoll<T>) -> Self {
            match poll {
                StdPoll::Ready(v) => PollResult::Ready(v),
                StdPoll::Pending => PollResult::Pending,
            }
        }
    }
    impl<T> From<PollResult<T>> for StdPoll<T> {
        fn from(poll: PollResult<T>) -> Self {
            match poll {
                PollResult::Ready(v) => StdPoll::Ready(v),
                PollResult::Pending => StdPoll::Pending,
                PollResult::Invalid => StdPoll::Pending,
            }
        }
    }
}
pub mod events {
    use vstd::prelude::*;
    use crate::types::{TID, TaskView, PollResult};
    pub enum DrainSource {
        ReactorWake,
        TaskWake,
        Deferred,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for DrainSource {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for DrainSource {
        #[inline]
        fn eq(&self, other: &DrainSource) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for DrainSource {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {}
    }
    pub enum InboundCall {
        Tick { result: Option<()> },
    }
    pub enum OutboundCall {
        PollTask { task_id: TID, task: Option<TaskView>, result: PollResult<()> },
        Drain { source: DrainSource, task_ids: Seq<TID> },
        Park,
        PopInjection { task: Option<TaskView> },
    }
    pub enum ExecutorEvent {
        Inbound(InboundCall),
        Outbound(OutboundCall),
    }
}
pub mod log {
    use vstd::prelude::*;
    use crate::types::*;
    use crate::events::*;
    pub type Log = Seq<ExecutorEvent>;
}
pub mod fifo_queue {
    use vstd::prelude::*;
    use crate::types::TID;
    use crate::log::*;
    use crate::events::*;
}
pub mod injection_schedule {
    use vstd::prelude::*;
    use crate::events::*;
    use crate::log::Log;
    use crate::types::*;
}
pub mod invariants {
    pub mod park_drain_reactor_wake {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::local_liveness::*;
    }
    pub mod tick_polls_if_runnable {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::local_liveness::*;
    }
    pub mod poll_within_tick {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod tick_has_park {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod tick_has_pop_injection {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod tick_has_drain_deferred {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod tick_has_drain_task_wake {
        use vstd::prelude::*;
        use crate::log::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod fifo_task_selection {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        use lion_framework_spec::action_safety::*;
    }
    pub mod valid_task_polling {
        use vstd::prelude::*;
        use crate::log::*;
        use crate::events::*;
        use crate::types::*;
        use lion_framework_spec::action_safety::*;
    }
}
pub mod contracts {
    pub mod bounded_injection_poll {
        use vstd::prelude::*;
        use crate::types::*;
        use crate::log::*;
        use crate::events::*;
        use lion_framework_spec::async_contract::*;
    }
    pub mod bounded_reactor_wake_poll {
        use vstd::prelude::*;
        use crate::types::*;
        use crate::log::*;
        use crate::events::*;
        use lion_framework_spec::async_contract::*;
    }
    pub mod bounded_task_wake_poll {
        use vstd::prelude::*;
        use crate::types::*;
        use crate::log::*;
        use crate::events::*;
        use lion_framework_spec::async_contract::*;
    }
    pub mod bounded_deferred_poll {
        use vstd::prelude::*;
        use crate::types::*;
        use crate::log::*;
        use crate::events::*;
        use lion_framework_spec::async_contract::*;
    }
    pub mod bounded_drain_poll {
        use vstd::prelude::*;
        use crate::types::*;
        use crate::log::*;
        use crate::events::*;
        use lion_framework_spec::async_contract::*;
    }
}
