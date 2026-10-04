#![feature(prelude_import)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod spec {
    pub mod types {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::types::*;
        use vstd::prelude::*;
        pub enum ResourceSlotView {
            Timer { entry: (InstantView, nat, int), waker: WakerView },
            Io { read_waker: Option<WakerView>, write_waker: Option<WakerView> },
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for ResourceSlotView {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for ResourceSlotView {
            #[inline]
            fn eq(&self, other: &ResourceSlotView) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
                    && match (self, other) {
                        (
                            ResourceSlotView::Timer { entry: __self_0, waker: __self_1 },
                            ResourceSlotView::Timer { entry: __arg1_0, waker: __arg1_1 },
                        ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                        (
                            ResourceSlotView::Io {
                                read_waker: __self_0,
                                write_waker: __self_1,
                            },
                            ResourceSlotView::Io {
                                read_waker: __arg1_0,
                                write_waker: __arg1_1,
                            },
                        ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                        _ => unsafe { ::core::intrinsics::unreachable() }
                    }
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for ResourceSlotView {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<(InstantView, nat, int)>;
                let _: ::core::cmp::AssertParamIsEq<WakerView>;
                let _: ::core::cmp::AssertParamIsEq<Option<WakerView>>;
                let _: ::core::cmp::AssertParamIsEq<Option<WakerView>>;
            }
        }
        impl ResourceSlotView {}
    }
    pub mod log {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::events::*;
        #[allow(unused_imports)]
        pub use lion_reactor_spec::log::Log;
    }
    pub mod predicates {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::events::*;
        #[allow(unused_imports)]
        pub use lion_reactor_spec::log::*;
        use vstd::prelude::*;
        use super::types::*;
        use super::log::Log;
    }
}
pub mod types {
    mod interest {
        use vstd::prelude::*;
        use crate::spec::types::InterestView;
        pub struct Interest {
            pub readable: bool,
            pub writable: bool,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Interest {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Interest {}
        #[automatically_derived]
        impl ::core::clone::Clone for Interest {
            #[inline]
            fn clone(&self) -> Interest {
                let _: ::core::clone::AssertParamIsClone<bool>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Interest {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Interest {
            #[inline]
            fn eq(&self, other: &Interest) -> bool {
                self.readable == other.readable && self.writable == other.writable
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Interest {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<bool>;
            }
        }
        impl View for Interest {
            type V = InterestView;
        }
        impl Interest {
            pub const READABLE: Interest = Interest {
                readable: true,
                writable: false,
            };
            pub const WRITABLE: Interest = Interest {
                readable: false,
                writable: true,
            };
            pub const READABLE_WRITABLE: Interest = Interest {
                readable: true,
                writable: true,
            };
            pub fn is_readable(&self) -> bool {
                self.readable
            }
            pub fn is_writable(&self) -> bool {
                self.writable
            }
        }
    }
    mod interrupt_handle {
        use crate::os::OsInterrupt;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use vstd::prelude::*;
        pub struct InterruptHandleShared {
            os: Arc<dyn OsInterrupt>,
            notified: AtomicBool,
        }
        pub struct InterruptHandleInner(pub Arc<InterruptHandleShared>);
        #[automatically_derived]
        impl ::core::clone::Clone for InterruptHandleInner {
            #[inline]
            fn clone(&self) -> InterruptHandleInner {
                InterruptHandleInner(::core::clone::Clone::clone(&self.0))
            }
        }
        impl InterruptHandleInner {
            pub fn new(os: Arc<dyn OsInterrupt>) -> Self {
                InterruptHandleInner(
                    Arc::new(InterruptHandleShared {
                        os,
                        notified: AtomicBool::new(false),
                    }),
                )
            }
            pub fn wake(&self) {
                if !self.0.notified.swap(true, Ordering::AcqRel) {
                    self.0.os.signal().expect("failed to wake reactor");
                }
            }
            pub fn reset(&self) {
                self.0.notified.store(false, Ordering::Release);
            }
        }
        pub struct InterruptHandle {
            pub(crate) inner: InterruptHandleInner,
        }
        impl View for InterruptHandle {
            type V = int;
        }
        impl Clone for InterruptHandle {
            fn clone(&self) -> Self {
                InterruptHandle {
                    inner: self.inner.clone(),
                }
            }
        }
        impl InterruptHandle {
            pub fn wake(&self) {
                self.inner.wake()
            }
            pub fn reset(&self) {
                self.inner.reset()
            }
        }
    }
    mod io_event {
        use super::ResourceId;
        use vstd::prelude::*;
        use crate::spec::types::IoEventView;
        pub enum IoMode {
            Readable,
            Writable,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for IoMode {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for IoMode {}
        #[automatically_derived]
        impl ::core::clone::Clone for IoMode {
            #[inline]
            fn clone(&self) -> IoMode {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for IoMode {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for IoMode {
            #[inline]
            fn eq(&self, other: &IoMode) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for IoMode {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {}
        }
        impl IoMode {
            pub fn is_readable(&self) -> bool {
                match self {
                    IoMode::Readable => true,
                    IoMode::Writable => false,
                }
            }
            pub fn is_writable(&self) -> bool {
                match self {
                    IoMode::Readable => false,
                    IoMode::Writable => true,
                }
            }
        }
        pub struct IoEvent {
            pub resource_id: ResourceId,
            pub mode: IoMode,
            pub error: bool,
            pub read_closed: bool,
            pub write_closed: bool,
        }
        impl View for IoEvent {
            type V = IoEventView;
        }
        impl DeepView for IoEvent {
            type V = IoEventView;
        }
    }
    mod io_event_queue {
        use super::IoResult;
        use crate::os::OsEvent;
        use vstd::prelude::*;
        pub struct IoEventQueue {
            pub(crate) inner: Vec<OsEvent>,
        }
        impl View for IoEventQueue {
            type V = int;
        }
        impl IoEventQueue {
            pub fn with_capacity(capacity: usize) -> IoResult<Self> {
                IoResult::Ok(IoEventQueue {
                    inner: Vec::with_capacity(capacity),
                })
            }
        }
    }
    mod io_result {
        use vstd::prelude::*;
        use crate::spec::types::IoResultView;
        pub struct IoError {
            pub(crate) inner: std::io::Error,
        }
        impl View for IoError {
            type V = int;
        }
        pub enum IoResult<T> {
            Ok(T),
            Err(IoError),
        }
        impl<T: View> View for IoResult<T> {
            type V = IoResultView<T::V>;
        }
        impl<T: DeepView> DeepView for IoResult<T> {
            type V = IoResultView<T::V>;
        }
        impl IoError {
            pub fn resource_id_overflow() -> Self {
                IoError {
                    inner: std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "reactor resource ID overflow: maximum resource IDs exhausted",
                    ),
                }
            }
        }
        impl Clone for IoError {
            fn clone(&self) -> Self {
                IoError {
                    inner: std::io::Error::new(self.inner.kind(), self.inner.to_string()),
                }
            }
        }
        impl IoError {
            /// The underlying `std::io::Error`.
            pub fn into_io_error(self) -> std::io::Error {
                self.inner
            }
        }
        impl From<IoError> for std::io::Error {
            fn from(e: IoError) -> std::io::Error {
                e.inner
            }
        }
        impl std::fmt::Debug for IoError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.inner.fmt(f)
            }
        }
    }
    mod poll {
        use crate::os::OsBackend;
        use vstd::prelude::*;
        pub struct Poll {
            pub(crate) inner: Box<dyn OsBackend>,
        }
        impl View for Poll {
            type V = int;
        }
        impl Poll {
            pub fn new(backend: Box<dyn OsBackend>) -> Poll {
                Poll { inner: backend }
            }
        }
    }
    mod resource_id {
        use vstd::prelude::*;
        use crate::spec::types::ResourceIdView;
        pub struct ResourceId(pub u64);
        #[automatically_derived]
        impl ::core::marker::Copy for ResourceId {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for ResourceId {}
        #[automatically_derived]
        impl ::core::clone::Clone for ResourceId {
            #[inline]
            fn clone(&self) -> ResourceId {
                let _: ::core::clone::AssertParamIsClone<u64>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for ResourceId {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for ResourceId {
            #[inline]
            fn eq(&self, other: &ResourceId) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for ResourceId {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u64>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for ResourceId {
            #[inline]
            fn partial_cmp(
                &self,
                other: &ResourceId,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for ResourceId {
            #[inline]
            fn cmp(&self, other: &ResourceId) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for ResourceId {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for ResourceId {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "ResourceId",
                    &&self.0,
                )
            }
        }
        impl View for ResourceId {
            type V = ResourceIdView;
        }
    }
    mod source {
        use vstd::prelude::*;
        use crate::os::RawFd;
        use crate::spec::types::SourceView;
        pub struct Source {
            pub(crate) fd: RawFd,
        }
        impl View for Source {
            type V = SourceView;
        }
        impl Source {
            pub fn new(fd: RawFd) -> Self {
                Source { fd }
            }
            pub fn fd(&self) -> RawFd {
                self.fd
            }
        }
    }
    mod time {
        use vstd::prelude::*;
        use crate::spec::types::{InstantView, DurationView};
        pub struct Duration {
            pub inner: u64,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Duration {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "Duration",
                    "inner",
                    &&self.inner,
                )
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Duration {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Duration {}
        #[automatically_derived]
        impl ::core::clone::Clone for Duration {
            #[inline]
            fn clone(&self) -> Duration {
                let _: ::core::clone::AssertParamIsClone<u64>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Duration {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Duration {
            #[inline]
            fn eq(&self, other: &Duration) -> bool {
                self.inner == other.inner
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Duration {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u64>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Duration {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Duration,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Duration {
            #[inline]
            fn cmp(&self, other: &Duration) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Duration {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.inner, state)
            }
        }
        pub struct Instant {
            pub inner: u64,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Instant {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "Instant",
                    "inner",
                    &&self.inner,
                )
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Instant {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Instant {}
        #[automatically_derived]
        impl ::core::clone::Clone for Instant {
            #[inline]
            fn clone(&self) -> Instant {
                let _: ::core::clone::AssertParamIsClone<u64>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Instant {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Instant {
            #[inline]
            fn eq(&self, other: &Instant) -> bool {
                self.inner == other.inner
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Instant {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u64>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Instant {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Instant,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Instant {
            #[inline]
            fn cmp(&self, other: &Instant) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Instant {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.inner, state)
            }
        }
        impl View for Duration {
            type V = DurationView;
        }
        impl DeepView for Duration {
            type V = DurationView;
        }
        impl View for Instant {
            type V = InstantView;
        }
        impl DeepView for Instant {
            type V = InstantView;
        }
        impl Duration {
            pub fn from_millis(millis: u64) -> Self {
                Duration { inner: millis }
            }
            pub fn from_secs(secs: u64) -> Self {
                if secs > u64::MAX / 1000 {
                    Duration { inner: u64::MAX }
                } else {
                    Duration { inner: secs * 1000 }
                }
            }
            pub fn as_millis(&self) -> u64 {
                self.inner
            }
            pub fn from_std(std_duration: std::time::Duration) -> Self {
                Duration {
                    inner: std_duration.as_millis() as u64,
                }
            }
        }
        impl Instant {
            pub fn now() -> Self {
                use std::sync::OnceLock;
                static START: OnceLock<std::time::Instant> = OnceLock::new();
                let start = *START.get_or_init(std::time::Instant::now);
                Instant {
                    inner: start.elapsed().as_millis() as u64,
                }
            }
            pub fn elapsed(&self) -> Duration {
                let now = Self::now();
                Duration {
                    inner: now.inner.saturating_sub(self.inner),
                }
            }
        }
        impl std::ops::Add<Duration> for Instant {
            type Output = Instant;
            fn add(self, duration: Duration) -> Instant {
                Instant {
                    inner: self.inner.saturating_add(duration.inner),
                }
            }
        }
        impl std::ops::Sub<Duration> for Instant {
            type Output = Instant;
            fn sub(self, duration: Duration) -> Instant {
                Instant {
                    inner: self.inner.saturating_sub(duration.inner),
                }
            }
        }
        impl From<std::time::Duration> for Duration {
            fn from(d: std::time::Duration) -> Self {
                Duration {
                    inner: d.as_millis() as u64,
                }
            }
        }
        impl From<Duration> for std::time::Duration {
            fn from(d: Duration) -> Self {
                std::time::Duration::from_millis(d.inner)
            }
        }
        impl std::ops::Add<std::time::Duration> for Instant {
            type Output = Instant;
            fn add(self, duration: std::time::Duration) -> Instant {
                Instant {
                    inner: self.inner.saturating_add(duration.as_millis() as u64),
                }
            }
        }
        impl std::ops::Sub<std::time::Duration> for Instant {
            type Output = Instant;
            fn sub(self, duration: std::time::Duration) -> Instant {
                Instant {
                    inner: self.inner.saturating_sub(duration.as_millis() as u64),
                }
            }
        }
        impl std::ops::AddAssign<std::time::Duration> for Instant {
            fn add_assign(&mut self, duration: std::time::Duration) {
                self.inner = self.inner.saturating_add(duration.as_millis() as u64);
            }
        }
    }
    mod timer_entry {
        use crate::types::{Instant, ResourceId};
        use crate::spec::types::InstantView;
        use std::cmp::Ordering;
        use vstd::prelude::*;
        pub struct TimerEntry {
            pub deadline: Instant,
            pub resource_id: ResourceId,
            pub log_index: Ghost<int>,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for TimerEntry {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for TimerEntry {}
        #[automatically_derived]
        impl ::core::clone::Clone for TimerEntry {
            #[inline]
            fn clone(&self) -> TimerEntry {
                let _: ::core::clone::AssertParamIsClone<Instant>;
                let _: ::core::clone::AssertParamIsClone<ResourceId>;
                let _: ::core::clone::AssertParamIsClone<Ghost<int>>;
                *self
            }
        }
        impl View for TimerEntry {
            type V = (InstantView, nat, int);
        }
        impl PartialEq for TimerEntry {
            fn eq(&self, other: &Self) -> bool {
                self.deadline == other.deadline && self.resource_id == other.resource_id
            }
        }
        impl Eq for TimerEntry {}
        impl PartialOrd for TimerEntry {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for TimerEntry {
            fn cmp(&self, other: &Self) -> Ordering {
                match self.deadline.cmp(&other.deadline) {
                    Ordering::Equal => self.resource_id.cmp(&other.resource_id),
                    ord => ord,
                }
            }
        }
    }
    mod waker {
        use std::task::Waker as StdWaker;
        use vstd::prelude::*;
        use crate::spec::types::WakerView;
        pub struct Waker {
            pub(crate) inner: StdWaker,
        }
        impl View for Waker {
            type V = WakerView;
        }
        impl Clone for Waker {
            fn clone(&self) -> Self {
                Waker { inner: self.inner.clone() }
            }
        }
        impl Waker {
            pub fn from_std(waker: StdWaker) -> Self {
                Waker { inner: waker }
            }
        }
    }
    pub use interest::Interest;
    pub use interrupt_handle::{InterruptHandle, InterruptHandleInner};
    pub use io_event::{IoEvent, IoMode};
    pub use io_event_queue::IoEventQueue;
    pub use io_result::{IoError, IoResult};
    pub use poll::Poll;
    pub use resource_id::ResourceId;
    pub use source::Source;
    pub use time::{Duration, Instant};
    pub use timer_entry::TimerEntry;
    pub use waker::Waker;
}
pub mod collections {}
pub mod resource_slot {
    use crate::types::{TimerEntry, Waker};
    pub enum ResourceSlot {
        Timer { entry: TimerEntry, waker: Waker },
        Io { read_waker: Option<Waker>, write_waker: Option<Waker> },
    }
}
pub mod resource_slot_wrapper {
    use crate::resource_slot::ResourceSlot;
    use crate::spec::types::ResourceSlotView;
    use crate::types::{TimerEntry, Waker};
    use vstd::prelude::*;
    pub struct ResourceSlotWrapper {
        pub(crate) inner: ResourceSlot,
    }
    impl View for ResourceSlotWrapper {
        type V = ResourceSlotView;
    }
    impl ResourceSlotWrapper {
        pub fn new_timer(entry: TimerEntry, waker: Waker) -> Self {
            ResourceSlotWrapper {
                inner: ResourceSlot::Timer {
                    entry,
                    waker,
                },
            }
        }
        pub fn new_io() -> Self {
            ResourceSlotWrapper {
                inner: ResourceSlot::Io {
                    read_waker: None,
                    write_waker: None,
                },
            }
        }
        pub fn is_timer(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self.inner {
                ResourceSlot::Timer { .. } => true,
                _ => false,
            }
        }
        pub fn is_io(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self.inner {
                ResourceSlot::Io { .. } => true,
                _ => false,
            }
        }
        pub fn with_read_waker(self, waker: Waker) -> Self {
            match self.inner {
                ResourceSlot::Io { write_waker, .. } => {
                    ResourceSlotWrapper {
                        inner: ResourceSlot::Io {
                            read_waker: Some(waker),
                            write_waker,
                        },
                    }
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
        pub fn with_write_waker(self, waker: Waker) -> Self {
            match self.inner {
                ResourceSlot::Io { read_waker, .. } => {
                    ResourceSlotWrapper {
                        inner: ResourceSlot::Io {
                            read_waker,
                            write_waker: Some(waker),
                        },
                    }
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
        pub fn clone_timer_waker(&self) -> Waker {
            match &self.inner {
                ResourceSlot::Timer { waker, .. } => waker.clone(),
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
        pub fn clone_read_waker(&self) -> Option<Waker> {
            match &self.inner {
                ResourceSlot::Io { read_waker, .. } => {
                    read_waker.as_ref().map(|w| w.clone())
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
        pub fn clone_write_waker(&self) -> Option<Waker> {
            match &self.inner {
                ResourceSlot::Io { write_waker, .. } => {
                    write_waker.as_ref().map(|w| w.clone())
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
    }
}
pub mod resource_slab {
    use crate::resource_slot::ResourceSlot;
    use crate::resource_slot_wrapper::ResourceSlotWrapper;
    use crate::spec::types::{InstantView, ResourceIdView, ResourceSlotView, WakerView};
    use crate::types::{Instant, ResourceId, TimerEntry, Waker};
    use vstd::prelude::*;
    use vstd::set::Set;
    pub struct ResourceSlab {
        pub inner: lion_slab::Slab<ResourceSlotWrapper>,
    }
    impl View for ResourceSlab {
        type V = Map<nat, ResourceSlotView>;
    }
    impl ResourceSlab {
        pub fn new() -> Self {
            let inner = lion_slab::Slab::new();
            let result = ResourceSlab { inner };
            {}
            result
        }
        #[inline]
        pub fn p_insert_timer(&mut self, key: u64, entry: TimerEntry, waker: Waker) {
            let wrapper = ResourceSlotWrapper::new_timer(entry, waker);
            self.inner.insert(key, wrapper);
        }
        pub fn p_insert_io(&mut self, key: u64) {
            let wrapper = ResourceSlotWrapper::new_io();
            self.inner.insert(key, wrapper);
        }
        #[inline]
        pub fn p_remove(&mut self, key: u64) -> bool {
            let result = self.inner.remove(key);
            result.is_some()
        }
        pub fn p_set_read_waker(&mut self, key: u64, waker: Waker) {
            let old_wrapper = self.inner.remove(key).unwrap();
            let new_wrapper = old_wrapper.with_read_waker(waker);
            self.inner.insert(key, new_wrapper);
            {}
        }
        pub fn p_set_write_waker(&mut self, key: u64, waker: Waker) {
            let old_wrapper = self.inner.remove(key).unwrap();
            let new_wrapper = old_wrapper.with_write_waker(waker);
            self.inner.insert(key, new_wrapper);
            {}
        }
        pub fn p_get_timer_waker(&self, key: u64) -> Option<Waker> {
            let wrapper_opt = self.inner.get(key);
            match wrapper_opt {
                Some(wrapper) => {
                    if wrapper.is_timer() {
                        Some(wrapper.clone_timer_waker())
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
        pub fn p_get_read_waker(&self, key: u64) -> Option<Waker> {
            let wrapper_opt = self.inner.get(key);
            match wrapper_opt {
                Some(wrapper) => {
                    if wrapper.is_io() { wrapper.clone_read_waker() } else { None }
                }
                None => None,
            }
        }
        pub fn p_get_write_waker(&self, key: u64) -> Option<Waker> {
            let wrapper_opt = self.inner.get(key);
            match wrapper_opt {
                Some(wrapper) => {
                    if wrapper.is_io() { wrapper.clone_write_waker() } else { None }
                }
                None => None,
            }
        }
        pub fn v_set_read_waker(&mut self, key: u64, waker: Waker) {
            self.p_set_read_waker(key, waker);
            {}
        }
        pub fn v_set_write_waker(&mut self, key: u64, waker: Waker) {
            self.p_set_write_waker(key, waker);
            {}
        }
        #[inline]
        pub fn v_insert_timer_slot(
            &mut self,
            key: u64,
            entry: TimerEntry,
            waker: Waker,
        ) {
            self.p_insert_timer(key, entry, waker);
            {}
        }
        pub fn v_insert_io_slot(&mut self, key: u64) {
            self.p_insert_io(key);
            {}
        }
        #[inline]
        pub fn v_remove_timer_slot(&mut self, key: u64) {
            let _ = self.p_remove(key);
            {}
        }
        pub fn v_remove_io_slot(&mut self, key: u64) {
            let _ = self.p_remove(key);
            {}
        }
        #[inline]
        pub fn v_take_timer_waker(&self, key: u64) -> Option<Waker> {
            let result = self.p_get_timer_waker(key);
            {}
            result
        }
        pub fn v_get_read_waker(&self, key: u64) -> Option<Waker> {
            let result = self.p_get_read_waker(key);
            {}
            result
        }
        pub fn v_get_write_waker(&self, key: u64) -> Option<Waker> {
            let result = self.p_get_write_waker(key);
            {}
            result
        }
    }
    impl ResourceSlab {
        #[inline]
        fn get_slot(&self, key: u64) -> Option<&ResourceSlot> {
            self.inner.get(key).map(|w| &w.inner)
        }
        #[inline]
        fn get_slot_mut(&mut self, key: u64) -> Option<&mut ResourceSlot> {
            self.inner.get_mut(key).map(|w| &mut w.inner)
        }
        #[inline]
        pub fn contains(&self, key: u64) -> bool {
            self.inner.get(key).is_some()
        }
        pub fn get_timer_entry(&self, key: u64) -> Option<TimerEntry> {
            match self.get_slot(key) {
                Some(ResourceSlot::Timer { entry, .. }) => Some(*entry),
                _ => None,
            }
        }
        pub fn take_timer_waker(&mut self, key: u64) -> Option<Waker> {
            match self.get_slot_mut(key) {
                Some(ResourceSlot::Timer { waker, .. }) => {
                    let w = waker.clone();
                    Some(w)
                }
                _ => None,
            }
        }
        pub fn get_read_waker(&self, key: u64) -> Option<&Waker> {
            match self.get_slot(key) {
                Some(ResourceSlot::Io { read_waker, .. }) => read_waker.as_ref(),
                _ => None,
            }
        }
        pub fn get_write_waker(&self, key: u64) -> Option<&Waker> {
            match self.get_slot(key) {
                Some(ResourceSlot::Io { write_waker, .. }) => write_waker.as_ref(),
                _ => None,
            }
        }
        pub fn set_read_waker(&mut self, key: u64, waker: Waker) {
            if let Some(ResourceSlot::Io { read_waker, .. }) = self.get_slot_mut(key) {
                *read_waker = Some(waker);
            }
        }
        pub fn set_write_waker(&mut self, key: u64, waker: Waker) {
            if let Some(ResourceSlot::Io { write_waker, .. }) = self.get_slot_mut(key) {
                *write_waker = Some(waker);
            }
        }
        #[inline]
        pub fn replace_timer_waker(&mut self, key: u64, new_waker: Waker) -> bool {
            match self.inner.get_mut(key) {
                Some(
                    ResourceSlotWrapper { inner: ResourceSlot::Timer { waker, .. } },
                ) => {
                    *waker = new_waker;
                    true
                }
                _ => false,
            }
        }
        pub fn is_empty_timers(&self) -> bool {
            !self
                .inner
                .values()
                .any(|w| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match w.inner {
                        ResourceSlot::Timer { .. } => true,
                        _ => false,
                    }
                })
        }
    }
}
pub mod reactor {
    pub(crate) mod enter {
        use crate::reactor::{Reactor, ReactorGuard};
        use std::cell::Cell;
        const CURRENT_REACTOR: ::std::thread::LocalKey<Cell<Option<*mut Reactor>>> = {
            const __RUST_STD_INTERNAL_INIT: Cell<Option<*mut Reactor>> = {
                Cell::new(None)
            };
            unsafe {
                ::std::thread::LocalKey::new(const {
                    if ::std::mem::needs_drop::<Cell<Option<*mut Reactor>>>() {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::EagerStorage<
                                Cell<Option<*mut Reactor>>,
                            > = ::std::thread::local_impl::EagerStorage::new(
                                __RUST_STD_INTERNAL_INIT,
                            );
                            __RUST_STD_INTERNAL_VAL.get()
                        }
                    } else {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: Cell<Option<*mut Reactor>> = __RUST_STD_INTERNAL_INIT;
                            &__RUST_STD_INTERNAL_VAL
                        }
                    }
                })
            }
        };
        const CURRENT_EPOCH: ::std::thread::LocalKey<Cell<u64>> = {
            const __RUST_STD_INTERNAL_INIT: Cell<u64> = { Cell::new(0) };
            unsafe {
                ::std::thread::LocalKey::new(const {
                    if ::std::mem::needs_drop::<Cell<u64>>() {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::EagerStorage<
                                Cell<u64>,
                            > = ::std::thread::local_impl::EagerStorage::new(
                                __RUST_STD_INTERNAL_INIT,
                            );
                            __RUST_STD_INTERNAL_VAL.get()
                        }
                    } else {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: Cell<u64> = __RUST_STD_INTERNAL_INIT;
                            &__RUST_STD_INTERNAL_VAL
                        }
                    }
                })
            }
        };
        const NEXT_EPOCH: ::std::thread::LocalKey<Cell<u64>> = {
            const __RUST_STD_INTERNAL_INIT: Cell<u64> = { Cell::new(1) };
            unsafe {
                ::std::thread::LocalKey::new(const {
                    if ::std::mem::needs_drop::<Cell<u64>>() {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::EagerStorage<
                                Cell<u64>,
                            > = ::std::thread::local_impl::EagerStorage::new(
                                __RUST_STD_INTERNAL_INIT,
                            );
                            __RUST_STD_INTERNAL_VAL.get()
                        }
                    } else {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: Cell<u64> = __RUST_STD_INTERNAL_INIT;
                            &__RUST_STD_INTERNAL_VAL
                        }
                    }
                })
            }
        };
        impl Reactor {
            pub fn enter(&mut self) -> ReactorGuard {
                let ptr = self as *mut Reactor;
                let already_current = CURRENT_REACTOR.with(|r| r.get() == Some(ptr));
                if !already_current {
                    let epoch = NEXT_EPOCH
                        .with(|n| {
                            let e = n.get();
                            n.set(e + 1);
                            e
                        });
                    CURRENT_EPOCH.with(|c| c.set(epoch));
                }
                CURRENT_REACTOR
                    .with(|r| {
                        r.set(Some(ptr));
                    });
                ReactorGuard {
                    reactor: ptr as usize,
                }
            }
            /// Whether a reactor is entered on this thread.
            pub fn is_entered_on_current_thread() -> bool {
                CURRENT_REACTOR.try_with(|r| r.get().is_some()).unwrap_or(false)
            }
        }
        impl Drop for ReactorGuard {
            fn drop(&mut self) {
                let _ = CURRENT_REACTOR
                    .try_with(|r| {
                        if r.get().map(|p| p as usize) == Some(self.reactor) {
                            r.set(None);
                            let _ = CURRENT_EPOCH.try_with(|c| c.set(0));
                            crate::handle::clear_cached_now();
                            crate::readiness::clear_all();
                        }
                    });
            }
        }
        pub(crate) fn current_reactor_epoch() -> Option<u64> {
            let entered = CURRENT_REACTOR
                .try_with(|r| r.get().is_some())
                .unwrap_or(false);
            if !entered {
                return None;
            }
            CURRENT_EPOCH.try_with(|c| c.get()).ok().filter(|e| *e != 0)
        }
        pub(crate) fn with_current_reactor<F, R>(f: F) -> Option<R>
        where
            F: FnOnce(&mut Reactor) -> R,
        {
            CURRENT_REACTOR
                .with(|r| {
                    let ptr = r.get();
                    ptr.and_then(|ptr| unsafe { ptr.as_mut().map(f) })
                })
        }
    }
    pub mod ext {
        use crate::os::OsEvent;
        use crate::reactor::Reactor;
        use crate::spec::log::*;
        use crate::types::{
            Duration, Instant, Interest, IoEvent, IoMode, IoResult, IoError, ResourceId,
            Source, Waker,
        };
        use vstd::prelude::*;
        pub fn encode_token_raw(rid: u64) -> usize {
            rid as usize
        }
        pub fn decode_token_raw(raw: usize) -> u64 {
            raw as u64
        }
        fn collect_io_events(os_events: &[OsEvent]) -> Vec<IoEvent> {
            let mut events = Vec::with_capacity(64);
            for e in os_events {
                let resource_id = ResourceId(decode_token_raw(e.token));
                let error = e.error;
                let read_closed = e.read_closed;
                let write_closed = e.write_closed;
                if e.readable || read_closed || error {
                    events
                        .push(IoEvent {
                            resource_id,
                            mode: IoMode::Readable,
                            error,
                            read_closed,
                            write_closed,
                        });
                }
                if e.writable || write_closed || error {
                    events
                        .push(IoEvent {
                            resource_id,
                            mode: IoMode::Writable,
                            error,
                            read_closed,
                            write_closed,
                        });
                }
            }
            events
        }
        impl Reactor {
            pub fn park_begin_action(&mut self, timeout: Option<Duration>) {}
            pub fn park_end_action(
                &mut self,
                timeout: Option<Duration>,
                result: &IoResult<()>,
            ) {}
            pub fn register_io_begin_action(
                &mut self,
                source: &Source,
                interest: Interest,
            ) {}
            pub fn register_io_end_action(
                &mut self,
                source: &Source,
                interest: Interest,
                result: &IoResult<ResourceId>,
            ) {}
            pub fn deregister_io_begin_action(&mut self, resource_id: ResourceId) {}
            pub fn deregister_io_end_action(
                &mut self,
                resource_id: ResourceId,
                result: &IoResult<()>,
            ) {}
            pub fn set_waker_begin_action(
                &mut self,
                resource_id: ResourceId,
                interest: Interest,
                waker: &Waker,
            ) {}
            pub fn set_waker_end_action(
                &mut self,
                resource_id: ResourceId,
                interest: Interest,
                waker: &Waker,
            ) {}
            #[inline]
            pub fn register_timer_begin_action(
                &mut self,
                deadline: Instant,
                waker: &Waker,
            ) {}
            #[inline]
            pub fn register_timer_end_action(
                &mut self,
                deadline: Instant,
                waker: &Waker,
                result: &IoResult<ResourceId>,
            ) {}
            #[inline]
            pub fn deregister_timer_begin_action(&mut self, resource_id: ResourceId) {}
            fn log_get_current_time_action(&mut self, t: Instant) {}
        }
        impl Reactor {
            pub fn register_io_source_action(
                &mut self,
                source: &mut Source,
                resource_id: ResourceId,
                interest: Interest,
            ) -> IoResult<()> {
                let token = encode_token_raw(resource_id.0);
                match self.poll.inner.register(source.fd, token, interest) {
                    Ok(()) => IoResult::Ok(()),
                    Err(e) => IoResult::Err(IoError { inner: e }),
                }
            }
            pub fn deregister_io_source_action(
                &mut self,
                source: &mut Source,
                resource_id: ResourceId,
            ) -> IoResult<()> {
                match self.poll.inner.deregister(source.fd) {
                    Ok(()) => IoResult::Ok(()),
                    Err(e) => IoResult::Err(IoError { inner: e }),
                }
            }
            pub fn poll_events_action(
                &mut self,
                timeout: Option<Duration>,
            ) -> IoResult<Vec<IoEvent>> {
                let std_timeout = timeout
                    .map(|d| std::time::Duration::from_millis(d.as_millis()));
                self.events.inner.clear();
                match self.poll.inner.wait(&mut self.events.inner, std_timeout) {
                    Ok(()) => {
                        let lion_events: Vec<IoEvent> = collect_io_events(
                            &self.events.inner,
                        );
                        IoResult::Ok(lion_events)
                    }
                    Err(e) => IoResult::Err(IoError { inner: e }),
                }
            }
            pub fn io_event_ready_action(&mut self, event: &IoEvent) {}
            #[inline]
            fn publish_cached_now(now: Instant) {
                crate::handle::store_cached_now(now.inner);
            }
            pub fn get_current_time_action(&mut self) -> Instant {
                let raw = Instant::now();
                let now = if raw.inner >= self.wheel.elapsed {
                    raw
                } else {
                    Instant {
                        inner: self.wheel.elapsed,
                    }
                };
                Self::publish_cached_now(now);
                self.log_get_current_time_action(now);
                now
            }
            #[inline]
            pub fn wake_task_action(&mut self, waker: &Waker, source_rid: ResourceId) {
                waker.inner.wake_by_ref();
            }
        }
    }
    pub(crate) mod new {
        use crate::reactor::Reactor;
        use crate::resource_slab::ResourceSlab;
        use lion_timer_wheel::TimerWheel;
        use crate::os::OsBackend;
        use crate::types::{
            InterruptHandle, InterruptHandleInner, IoError, IoEventQueue, IoResult, Poll,
        };
        use crate::invariants::*;
        use crate::invariants::data_inv::*;
        use crate::spec::predicates::*;
        use vstd::prelude::*;
        fn os_handles(
            backend: Box<dyn OsBackend>,
        ) -> (Poll, IoEventQueue, InterruptHandle) {
            let interrupt_handle = InterruptHandle {
                inner: InterruptHandleInner::new(backend.interrupt()),
            };
            (
                Poll { inner: backend },
                IoEventQueue {
                    inner: Vec::with_capacity(1024),
                },
                interrupt_handle,
            )
        }
        impl Reactor {
            fn backend_setup(poll: Poll) -> (Poll, IoEventQueue, InterruptHandle) {
                os_handles(poll.inner)
            }
            pub fn with_poll(poll: Poll) -> IoResult<(Self, InterruptHandle)> {
                let (poll, events, interrupt_handle) = Self::backend_setup(poll);
                IoResult::Ok(Self::assemble(poll, events, interrupt_handle))
            }
            fn assemble(
                poll: Poll,
                events: IoEventQueue,
                interrupt_handle: InterruptHandle,
            ) -> (Self, InterruptHandle) {
                let wheel = TimerWheel::new();
                let resources = ResourceSlab::new();
                let reactor = Reactor {
                    next_resource_id: 1,
                    poll,
                    events,
                    wheel,
                    resources,
                    log: Ghost::assume_new_fallback(|| ::core::panicking::panic(
                        "internal error: entered unreachable code",
                    )),
                    free_rids: Vec::new(),
                    pending_deregister: None,
                };
                {}
                (reactor, interrupt_handle)
            }
        }
        impl Reactor {
            /// A reactor over an embedder's OS backend (see `crate::os` for the contract
            /// it must meet). The verified constructor is `with_poll`; this only wraps
            /// the backend for it.
            pub fn with_backend(
                backend: Box<dyn OsBackend>,
            ) -> IoResult<(Self, InterruptHandle)> {
                Self::with_poll(Poll::new(backend))
            }
        }
    }
    mod park {
        use crate::reactor::Reactor;
        use crate::readiness;
        use crate::types::{
            Duration, Instant, IoEvent, IoMode, IoResult, ResourceId, Waker,
        };
        use crate::invariants::*;
        use crate::invariants::data_inv::*;
        use crate::spec::predicates::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::proof::preservation::*;
        use crate::proof::safety_preservation::*;
        use crate::proof::liveness_preservation::*;
        use crate::proof::preservation_ext::*;
        use crate::proof::park_safety::*;
        use crate::invariants::park_has_timestamp::*;
        use crate::invariants::park_poll_once::*;
        use crate::invariants::io_ready_in_park::*;
        use crate::resource_slab::ResourceSlab;
        use vstd::prelude::*;
        fn mark_io_readable(rid: ResourceId) {
            readiness::mark_readable(rid);
        }
        fn mark_io_writable(rid: ResourceId) {
            readiness::mark_writable(rid);
        }
        impl Reactor {
            fn try_pop_expired_timer(
                &mut self,
                now: Instant,
            ) -> Option<(ResourceId, Ghost<int>, Ghost<InstantView>)> {
                {}
                match self.wheel.try_pop_expired(now.inner) {
                    None => {
                        {}
                        None
                    }
                    Some(rid_u64) => {
                        let resource_id = ResourceId(rid_u64);
                        {}
                        let log_idx: Ghost<int> = Ghost::assume_new_fallback(|| ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        ));
                        let deadline: Ghost<InstantView> = Ghost::assume_new_fallback(|| ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        ));
                        {}
                        Some((resource_id, log_idx, deadline))
                    }
                }
            }
            fn wake_expired_timers(&mut self, now: Instant) {
                {}
                loop {
                    match self.try_pop_expired_timer(now) {
                        None => {
                            return;
                        }
                        Some((rid, log_idx_ghost, deadline_ghost)) => {
                            {}
                            let waker_opt = self.resources.v_take_timer_waker(rid.0);
                            match waker_opt {
                                Some(waker) => {
                                    self.wake_task_action(&waker, rid);
                                    {}
                                    self.resources.v_remove_timer_slot(rid.0);
                                    {}
                                }
                                None => {}
                            }
                        }
                    }
                }
            }
            fn process_io_events(&mut self, io_events: &Vec<IoEvent>) {
                let num_events = io_events.len();
                let mut i: usize = 0;
                while i < num_events {
                    let event = &io_events[i];
                    self.io_event_ready_action(event);
                    let rid = event.resource_id;
                    {}
                    match event.mode {
                        IoMode::Readable => {
                            mark_io_readable(rid);
                            let waker_opt = self.resources.v_get_read_waker(rid.0);
                            match waker_opt {
                                Some(waker) => {
                                    self.wake_task_action(&waker, rid);
                                    {}
                                }
                                None => {}
                            }
                        }
                        IoMode::Writable => {
                            mark_io_writable(rid);
                            let waker_opt = self.resources.v_get_write_waker(rid.0);
                            match waker_opt {
                                Some(waker) => {
                                    self.wake_task_action(&waker, rid);
                                    {}
                                }
                                None => {}
                            }
                        }
                    }
                    i += 1;
                }
            }
            pub fn park(&mut self, timeout: Option<Duration>) -> IoResult<()> {
                {}
                self.park_begin_action(timeout);
                {}
                let now = self.get_current_time_action();
                {}
                let effective_timeout = match (timeout, self.next_deadline()) {
                    (Some(t), Some(deadline)) => {
                        if deadline.inner <= now.inner {
                            Some(Duration::from_millis(0))
                        } else {
                            let timer_ms = (deadline.inner - now.inner) as u64;
                            let timer_dur = Duration::from_millis(timer_ms);
                            if timer_dur.as_millis() < t.as_millis() {
                                Some(timer_dur)
                            } else {
                                Some(t)
                            }
                        }
                    }
                    (None, Some(deadline)) => {
                        if deadline.inner <= now.inner {
                            Some(Duration::from_millis(0))
                        } else {
                            let timer_ms = (deadline.inner - now.inner) as u64;
                            Some(Duration::from_millis(timer_ms))
                        }
                    }
                    (t, None) => t,
                };
                let poll_result = self.poll_events_action(effective_timeout);
                {}
                let io_events = match poll_result {
                    IoResult::Ok(events) => events,
                    IoResult::Err(e) => {
                        self.wake_expired_timers(now);
                        let result: IoResult<()> = IoResult::Err(e);
                        self.park_end_action(timeout, &result);
                        {}
                        return result;
                    }
                };
                self.wake_expired_timers(now);
                {}
                self.process_io_events(&io_events);
                let result = IoResult::Ok(());
                self.park_end_action(timeout, &result);
                {}
                result
            }
        }
    }
    mod register {
        use crate::reactor::Reactor;
        use crate::types::{Interest, IoResult, ResourceId, Source};
        use crate::invariants::*;
        use crate::invariants::data_inv::*;
        use crate::spec::log::*;
        use crate::spec::predicates::*;
        use crate::spec::types::IoResultView;
        use crate::proof::preservation::*;
        use crate::proof::safety_preservation::*;
        use crate::proof::preservation_ext::*;
        use crate::invariants::register_io_in_cycle::*;
        use crate::invariants::deregister_io_in_cycle::*;
        use crate::invariants::inbound_register_io_result::*;
        use crate::invariants::inbound_deregister_io_result::*;
        use vstd::prelude::*;
        impl Reactor {
            pub fn register_io_resource(
                &mut self,
                source: &mut Source,
                interest: Interest,
            ) -> IoResult<ResourceId> {
                self.register_io_begin_action(source, interest);
                {}
                let alloc_result = self.alloc_resource_id();
                let result = match alloc_result {
                    IoResult::Ok(resource_id) => {
                        let os_result = self
                            .register_io_source_action(source, resource_id, interest);
                        {}
                        match os_result {
                            IoResult::Ok(()) => {
                                {}
                                self.resources.v_insert_io_slot(resource_id.0);
                                {}
                                IoResult::Ok(resource_id)
                            }
                            IoResult::Err(e) => IoResult::Err(e),
                        }
                    }
                    IoResult::Err(e) => IoResult::Err(e),
                };
                self.register_io_end_action(source, interest, &result);
                {}
                result
            }
            pub fn deregister_io_resource(
                &mut self,
                resource_id: ResourceId,
                source: &mut Source,
            ) -> IoResult<()> {
                self.deregister_io_begin_action(resource_id);
                {}
                self.resources.v_remove_io_slot(resource_id.0);
                {}
                let result = self.deregister_io_source_action(source, resource_id);
                {}
                self.deregister_io_end_action(resource_id, &result);
                {}
                result
            }
        }
    }
    mod timer {
        use crate::reactor::Reactor;
        use crate::types::{Instant, IoResult, ResourceId, TimerEntry, Waker};
        use crate::spec::types::{InstantView, ResourceIdView, WakerView};
        use crate::invariants::*;
        use crate::invariants::data_inv::*;
        use crate::spec::log::*;
        use crate::spec::predicates::*;
        use crate::proof::preservation::*;
        use crate::proof::safety_preservation::*;
        use crate::proof::preservation_ext::*;
        use crate::proof::park_safety::*;
        use crate::resource_slab::ResourceSlab;
        use vstd::prelude::*;
        impl Reactor {
            #[inline]
            pub fn register_timer(
                &mut self,
                deadline: Instant,
                waker: Waker,
            ) -> IoResult<ResourceId> {
                self.register_timer_begin_action(deadline, &waker);
                {}
                let alloc_result = self.alloc_resource_id();
                let result = match alloc_result {
                    IoResult::Ok(resource_id) => {
                        let log_index: Ghost<int> = Ghost::assume_new_fallback(|| ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        ));
                        let entry = TimerEntry {
                            deadline,
                            resource_id,
                            log_index,
                        };
                        {}
                        self.resources
                            .v_insert_timer_slot(resource_id.0, entry, waker.clone());
                        self.wheel.insert(resource_id.0, deadline.inner);
                        {}
                        IoResult::Ok(resource_id)
                    }
                    IoResult::Err(e) => IoResult::Err(e),
                };
                self.register_timer_end_action(deadline, &waker, &result);
                {}
                result
            }
            #[inline]
            pub fn deregister_timer(&mut self, resource_id: ResourceId) {
                self.deregister_timer_begin_action(resource_id);
                self.wheel.remove(resource_id.0);
                {}
                self.resources.v_remove_timer_slot(resource_id.0);
                {}
            }
            pub fn next_deadline(&self) -> Option<Instant> {
                match self.wheel.next_deadline() {
                    Some(d) => Some(Instant { inner: d }),
                    None => None,
                }
            }
            #[inline]
            pub fn flush_pending_deregister(&mut self) {
                if let Some((rid, _)) = self.pending_deregister.take() {
                    self.wheel.remove(rid);
                    self.resources.v_remove_timer_slot(rid);
                }
            }
        }
    }
    mod waker {
        use crate::reactor::Reactor;
        use crate::types::{Interest, ResourceId, Waker};
        use crate::spec::types::ResourceIdView;
        use crate::invariants::*;
        use crate::invariants::data_inv::*;
        use crate::spec::log::*;
        use crate::spec::predicates::*;
        use crate::proof::preservation::*;
        use crate::proof::safety_preservation::*;
        use crate::proof::preservation_ext::*;
        use vstd::prelude::*;
        impl Reactor {
            pub fn set_waker(
                &mut self,
                resource_id: ResourceId,
                interest: Interest,
                waker: Waker,
            ) {
                self.set_waker_begin_action(resource_id, interest, &waker);
                {}
                if interest.readable {
                    self.resources.v_set_read_waker(resource_id.0, waker.clone());
                }
                if interest.writable {
                    self.resources.v_set_write_waker(resource_id.0, waker.clone());
                }
                {}
                self.set_waker_end_action(resource_id, interest, &waker);
                {}
            }
        }
    }
    use crate::resource_slab::ResourceSlab;
    use crate::spec::log::Log;
    use lion_timer_wheel::TimerWheel;
    use crate::types::{IoEventQueue, Poll, ResourceId, Waker};
    use vstd::prelude::*;
    pub struct Reactor {
        pub next_resource_id: u64,
        pub poll: Poll,
        pub events: IoEventQueue,
        pub wheel: TimerWheel,
        pub resources: ResourceSlab,
        pub log: Ghost<Log>,
        pub free_rids: Vec<u64>,
        pub pending_deregister: Option<(u64, u64)>,
    }
    pub struct ReactorGuard {
        pub(crate) reactor: usize,
    }
}
pub mod handle {
    use crate::reactor::Reactor;
    use crate::reactor::enter::with_current_reactor;
    use crate::readiness;
    use crate::types::{Instant, Interest, IoResult, ResourceId, Source, Waker};
    use std::cell::Cell;
    const CACHED_NOW: ::std::thread::LocalKey<Cell<Option<u64>>> = {
        const __RUST_STD_INTERNAL_INIT: Cell<Option<u64>> = { Cell::new(None) };
        unsafe {
            ::std::thread::LocalKey::new(const {
                if ::std::mem::needs_drop::<Cell<Option<u64>>>() {
                    |_| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::EagerStorage<
                            Cell<Option<u64>>,
                        > = ::std::thread::local_impl::EagerStorage::new(
                            __RUST_STD_INTERNAL_INIT,
                        );
                        __RUST_STD_INTERNAL_VAL.get()
                    }
                } else {
                    |_| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: Cell<Option<u64>> = __RUST_STD_INTERNAL_INIT;
                        &__RUST_STD_INTERNAL_VAL
                    }
                }
            })
        }
    };
    #[inline]
    pub fn store_cached_now(now_ms: u64) {
        CACHED_NOW.with(|c| c.set(Some(now_ms)));
    }
    pub(crate) fn clear_cached_now() {
        let _ = CACHED_NOW.try_with(|c| c.set(None));
    }
    pub struct ReactorHandle;
    #[automatically_derived]
    impl ::core::marker::Copy for ReactorHandle {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for ReactorHandle {}
    #[automatically_derived]
    impl ::core::clone::Clone for ReactorHandle {
        #[inline]
        fn clone(&self) -> ReactorHandle {
            *self
        }
    }
    impl ReactorHandle {
        #[inline]
        pub fn new() -> Self {
            ReactorHandle
        }
        /// The reactor's park-cycle clock observation (== `wheel.elapsed`), if the
        /// current thread's reactor has parked at least once. `None` before the
        /// first park (callers fall back to `Instant::now()`).
        #[inline]
        pub fn cached_now() -> Option<Instant> {
            CACHED_NOW.with(|c| c.get()).map(|ms| Instant { inner: ms })
        }
        pub fn register_io_resource(
            &self,
            source: &mut Source,
            interest: Interest,
        ) -> IoResult<ResourceId> {
            with_current_reactor(|reactor| {
                    reactor.register_io_resource(source, interest)
                })
                .expect("ReactorHandle used outside reactor context")
        }
        pub fn deregister_io_resource(
            &self,
            resource_id: ResourceId,
            source: &mut Source,
        ) -> IoResult<()> {
            let result = with_current_reactor(|reactor| {
                    reactor.deregister_io_resource(resource_id, source)
                })
                .expect("ReactorHandle used outside reactor context");
            readiness::remove_readiness(resource_id);
            result
        }
        pub fn set_waker(
            &self,
            resource_id: ResourceId,
            interest: Interest,
            waker: Waker,
        ) {
            with_current_reactor(|reactor| {
                    reactor.set_waker(resource_id, interest, waker)
                })
                .expect("ReactorHandle used outside reactor context")
        }
        #[inline]
        pub fn register_timer(
            &self,
            deadline: Instant,
            waker: Waker,
        ) -> IoResult<ResourceId> {
            with_current_reactor(|reactor| {
                    if let Some((rid, old_deadline)) = reactor.pending_deregister.take()
                    {
                        if old_deadline == deadline.inner
                            && reactor.resources.contains(rid)
                        {
                            reactor.resources.replace_timer_waker(rid, waker);
                            return IoResult::Ok(ResourceId(rid));
                        }
                        reactor.wheel.remove(rid);
                        reactor.resources.v_remove_timer_slot(rid);
                    }
                    reactor.register_timer(deadline, waker)
                })
                .expect("ReactorHandle used outside reactor context")
        }
        #[inline]
        pub fn deregister_timer(&self, resource_id: ResourceId) {
            with_current_reactor(|reactor| {
                    if let Some((old_rid, _)) = reactor.pending_deregister.take() {
                        reactor.wheel.remove(old_rid);
                        reactor.resources.v_remove_timer_slot(old_rid);
                    }
                    if let Some(deadline) = reactor.wheel.get_deadline(resource_id.0) {
                        reactor.pending_deregister = Some((resource_id.0, deadline));
                    } else {
                        reactor.deregister_timer(resource_id);
                    }
                })
                .expect("ReactorHandle used outside reactor context")
        }
    }
}
pub mod readiness {
    use crate::types::ResourceId;
    use std::cell::RefCell;
    use std::collections::HashMap;
    const READABLE: u8 = 0x01;
    const WRITABLE: u8 = 0x02;
    const IO_READINESS: ::std::thread::LocalKey<RefCell<HashMap<u64, u8>>> = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<HashMap<u64, u8>> {
            RefCell::new(HashMap::new())
        }
        unsafe {
            ::std::thread::LocalKey::new(const {
                if ::std::mem::needs_drop::<RefCell<HashMap<u64, u8>>>() {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            RefCell<HashMap<u64, u8>>,
                            (),
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(
                                __rust_std_internal_init,
                                __rust_std_internal_init_fn,
                            )
                    }
                } else {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            RefCell<HashMap<u64, u8>>,
                            !,
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(
                                __rust_std_internal_init,
                                __rust_std_internal_init_fn,
                            )
                    }
                }
            })
        }
    };
    pub fn init_readiness(resource_id: ResourceId) {
        IO_READINESS
            .with(|r| {
                r.borrow_mut().insert(resource_id.0, READABLE | WRITABLE);
            });
    }
    pub(crate) fn clear_all() {
        let _ = IO_READINESS
            .try_with(|r| {
                if let Ok(mut r) = r.try_borrow_mut() {
                    r.clear();
                }
            });
    }
    /// Number of live readiness entries on this thread (one per registered io
    /// resource that initialised its readiness). For tests of resource cleanup.
    pub fn live_entries() -> usize {
        IO_READINESS.with(|r| r.borrow().len())
    }
    pub fn remove_readiness(resource_id: ResourceId) {
        IO_READINESS
            .with(|r| {
                r.borrow_mut().remove(&resource_id.0);
            });
    }
    pub fn mark_readable(resource_id: ResourceId) {
        IO_READINESS
            .with(|r| {
                if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
                    *f |= READABLE;
                }
            });
    }
    pub fn mark_writable(resource_id: ResourceId) {
        IO_READINESS
            .with(|r| {
                if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
                    *f |= WRITABLE;
                }
            });
    }
    pub fn is_readable(resource_id: ResourceId) -> bool {
        IO_READINESS
            .with(|r| {
                r.borrow().get(&resource_id.0).is_some_and(|f| (f & READABLE) != 0)
            })
    }
    pub fn is_writable(resource_id: ResourceId) -> bool {
        IO_READINESS
            .with(|r| {
                r.borrow().get(&resource_id.0).is_some_and(|f| (f & WRITABLE) != 0)
            })
    }
    pub fn clear_readable(resource_id: ResourceId) {
        IO_READINESS
            .with(|r| {
                if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
                    *f &= !READABLE;
                }
            });
    }
    pub fn clear_writable(resource_id: ResourceId) {
        IO_READINESS
            .with(|r| {
                if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
                    *f &= !WRITABLE;
                }
            });
    }
}
pub mod framework {
    pub mod action_safety {
        #[allow(unused_imports)]
        pub use lion_framework_spec::action_safety::*;
    }
    pub mod local_liveness {
        #[allow(unused_imports)]
        pub use lion_framework_spec::local_liveness::*;
    }
}
pub mod invariants {
    pub mod timer_waker_validity {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::timer_waker_validity::*;
    }
    pub mod io_waker_validity {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::io_waker_validity::*;
    }
    pub mod timer_reg_uniqueness {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::timer_reg_uniqueness::*;
    }
    pub mod io_reg_uniqueness {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::io_reg_uniqueness::*;
    }
    pub mod timer_io_disjoint {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::timer_io_disjoint::*;
    }
    pub mod wake_has_registration {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::wake_has_registration::*;
    }
    pub mod wake_on_expired {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::wake_on_expired::*;
    }
    pub mod wake_on_io_ready {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::wake_on_io_ready::*;
    }
    pub mod data_inv {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::spec::predicates::*;
    }
    pub mod timer_deadline_future {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::timer_deadline_future::*;
    }
    pub mod park_has_timestamp {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::park_has_timestamp::*;
    }
    pub mod park_poll_once {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::park_poll_once::*;
    }
    pub mod io_ready_in_park {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::io_ready_in_park::*;
    }
    pub mod register_io_in_cycle {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::register_io_in_cycle::*;
    }
    pub mod deregister_io_in_cycle {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::deregister_io_in_cycle::*;
    }
    pub mod inbound_register_io_result {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::inbound_register_io_result::*;
    }
    pub mod inbound_deregister_io_result {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::inbound_deregister_io_result::*;
    }
    pub mod set_waker_active_io {
        #[allow(unused_imports)]
        pub use lion_reactor_spec::invariants::set_waker_active_io::*;
    }
    use vstd::prelude::*;
    use crate::spec::log::*;
    use crate::spec::predicates::*;
}
pub mod proof {
    pub mod preservation {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::spec::predicates::*;
        use crate::invariants::*;
        use crate::invariants::timer_waker_validity::*;
        use crate::invariants::io_waker_validity::*;
        use crate::invariants::data_inv::*;
    }
    pub mod safety_preservation {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::spec::predicates::*;
        use crate::invariants::*;
        use crate::invariants::timer_waker_validity::*;
        use crate::invariants::io_waker_validity::*;
        use crate::invariants::data_inv::*;
    }
    pub mod liveness_preservation {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::spec::predicates::*;
        use crate::invariants::*;
        use crate::invariants::data_inv::*;
    }
    pub mod preservation_ext {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::spec::predicates::*;
        use crate::invariants::*;
        use crate::invariants::register_io_in_cycle::*;
        use crate::invariants::deregister_io_in_cycle::*;
        use crate::invariants::inbound_register_io_result::*;
        use crate::invariants::inbound_deregister_io_result::*;
        use crate::invariants::timer_deadline_future::*;
        use crate::invariants::park_has_timestamp::*;
        use crate::invariants::park_poll_once::*;
        use crate::invariants::io_ready_in_park::*;
        use crate::invariants::set_waker_active_io::*;
    }
    pub mod park_safety {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::types::*;
        use crate::spec::predicates::*;
        use crate::invariants::*;
        use crate::invariants::timer_waker_validity::*;
        use crate::invariants::io_waker_validity;
        use crate::invariants::io_waker_validity::*;
        use crate::invariants::data_inv::*;
        use crate::invariants::park_poll_once::*;
        use crate::invariants::io_ready_in_park::*;
        use crate::proof::preservation::*;
        use crate::proof::safety_preservation::*;
        use crate::proof::preservation_ext::*;
    }
    pub mod io_anchor_bridge {
        use vstd::prelude::*;
    }
}
pub mod alloc_verified {
    use crate::reactor::Reactor;
    use crate::types::{IoError, IoResult, ResourceId};
    use crate::spec::predicates::*;
    use vstd::prelude::*;
    impl Reactor {
        pub fn alloc_resource_id(&mut self) -> IoResult<ResourceId> {
            if self.next_resource_id >= u64::MAX {
                IoResult::Err(IoError::resource_id_overflow())
            } else {
                let id = self.next_resource_id;
                self.next_resource_id = self.next_resource_id + 1;
                {}
                IoResult::Ok(ResourceId(id))
            }
        }
    }
}
pub mod os {
    //! The operating-system seam of the reactor.
    //!
    //! Everything the reactor needs from the OS goes through one object-safe trait,
    //! [`OsBackend`]: register, reregister and deregister a raw file descriptor
    //! with an interest, wait for readiness events with a timeout, and hand out a
    //! cross-thread [`OsInterrupt`] that cuts a wait short. The reactor owns its
    //! backend as a `Box<dyn OsBackend>` (one dynamic call per park, one per
    //! registration). [`MioBackend`] (feature `mio`, on by default) is Lion's own
    //! backend; an embedder with its own event loop plugs in its own backend with
    //! `Reactor::with_backend` (lion-executor: `RuntimeBuilder::os_backend`), and
    //! then needs no mio at all.
    //!
    //! # The contract a backend must meet
    //!
    //! The reactor keeps a readiness flag per io resource and direction (read,
    //! write). A flag is set when a wait reports an event for the resource and is
    //! cleared only by the resource's owner after an operation returned
    //! `WouldBlock` (see [`crate::async_fd`]). The reactor therefore needs
    //! **edge-triggered** reports, exactly epoll's `EPOLLET` as mio uses it:
    //!
    //! * **Registration.** `register(fd, token, interest)` starts reporting `fd`
    //!   under `token`, for readability if `interest.readable`, for writability if
    //!   `interest.writable` (an interest with neither is treated as readable).
    //!   Tokens are opaque to the backend and must be returned verbatim in events.
    //!   The reactor never registers token 0 (a backend may reserve it for its
    //!   interrupt) and never registers one fd twice without deregistering it.
    //!   An fd that is already ready when registered need not be reported: the
    //!   reactor starts every resource as ready in both directions. `reregister`
    //!   replaces the token and interest of a registered fd (the reactor itself
    //!   does not call it today). `deregister(fd)` stops all reports for `fd`,
    //!   including events not yet returned by a wait; it is called before the fd
    //!   is closed. Errors are returned as `io::Error` and fail the registration.
    //! * **Edge-triggered events.** After an event for (token, direction) has been
    //!   returned, the backend must report that direction again whenever the fd
    //!   becomes ready anew: new data arrives (even if unread data remains), buffer
    //!   space frees up, a connection completes, the peer closes, an error occurs.
    //!   It must never swallow such a transition. Reporting more often (spurious
    //!   events, or level-triggered reports) is safe but costs wakeups: a
    //!   level-triggered backend re-reports a ready fd on every wait, so a task that
    //!   leaves data unread keeps the loop spinning.
    //! * **Event flags** ([`OsEvent`]), with mio's meaning on Linux: `readable` =
    //!   `EPOLLIN|EPOLLPRI`; `writable` = `EPOLLOUT`; `error` = `EPOLLERR`;
    //!   `read_closed` = `EPOLLHUP`, or `EPOLLIN` with `EPOLLRDHUP`; `write_closed` =
    //!   `EPOLLHUP`, or `EPOLLOUT` with `EPOLLERR`, or `EPOLLERR` alone. The error
    //!   and hang-up conditions must be reported whatever the interest (epoll does
    //!   so), and a registration should ask for `EPOLLRDHUP` so that a peer's
    //!   half-close is visible. The reactor wakes a resource's reader on
    //!   `readable || read_closed || error` and its writer on
    //!   `writable || write_closed || error`, so an error-only event (a pipe whose
    //!   reader went away while the writer waits for space) wakes the writer.
    //! * **Wait.** `wait(events, timeout)` appends the ready events to `events`
    //!   (the reactor passes it empty, with capacity reserved; a backend should
    //!   not exceed `events.capacity()`, and events it does not return stay pending
    //!   for the next wait). `timeout` `None` blocks until an event or an
    //!   interrupt; `Some(d)` blocks at most about `d` (the reactor's clock is in
    //!   milliseconds); `Some(Duration::ZERO)` does not block. A wait cut short by a
    //!   signal (`EINTR`) returns `Ok` with the events it has, possibly none. Any
    //!   other error is returned; the reactor then treats the park as an empty one
    //!   (expired timers still fire).
    //! * **Interrupt.** [`OsBackend::interrupt`] returns the backend's
    //!   [`OsInterrupt`]; the reactor calls it once, when it is built, and shares it
    //!   with every thread that may need to wake the loop. `signal()` may be called
    //!   from any thread at any time. A signal made before or during a wait makes
    //!   that wait return promptly; a signal made while no wait is in progress makes
    //!   the next wait return promptly. Signals may coalesce into one early return,
    //!   and waits may return early without a signal. The backend consumes its own
    //!   notification inside `wait`, on the owner thread (for example by reading an
    //!   eventfd registered under a reserved token), and never reports it as an
    //!   event. Callers coalesce: the reactor's `InterruptHandle` signals only on
    //!   the first wake after its owner reset it, and the owner resets before it
    //!   drains its cross-thread queue and then waits, so no wake is lost as long as
    //!   a signal is never dropped between `signal()` and the next wait's return.
    //!
    //! All methods but `OsInterrupt::signal` run on the thread that owns the
    //! reactor.
    use crate::types::Interest;
    use std::io;
    use std::sync::Arc;
    use std::time::Duration;
    /// A raw file descriptor, as the OS knows it (`std::os::fd::RawFd` on Unix).
    pub type RawFd = i32;
    /// One readiness report for a registered fd. See the module documentation for
    /// the meaning of each flag.
    pub struct OsEvent {
        /// The token the fd was registered under.
        pub token: usize,
        pub readable: bool,
        pub writable: bool,
        pub error: bool,
        pub read_closed: bool,
        pub write_closed: bool,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for OsEvent {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for OsEvent {}
    #[automatically_derived]
    impl ::core::clone::Clone for OsEvent {
        #[inline]
        fn clone(&self) -> OsEvent {
            let _: ::core::clone::AssertParamIsClone<usize>;
            let _: ::core::clone::AssertParamIsClone<bool>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for OsEvent {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "token",
                "readable",
                "writable",
                "error",
                "read_closed",
                "write_closed",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.token,
                &self.readable,
                &self.writable,
                &self.error,
                &self.read_closed,
                &&self.write_closed,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "OsEvent",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for OsEvent {
        #[inline]
        fn default() -> OsEvent {
            OsEvent {
                token: ::core::default::Default::default(),
                readable: ::core::default::Default::default(),
                writable: ::core::default::Default::default(),
                error: ::core::default::Default::default(),
                read_closed: ::core::default::Default::default(),
                write_closed: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for OsEvent {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for OsEvent {
        #[inline]
        fn eq(&self, other: &OsEvent) -> bool {
            self.readable == other.readable && self.writable == other.writable
                && self.error == other.error && self.read_closed == other.read_closed
                && self.write_closed == other.write_closed && self.token == other.token
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for OsEvent {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<usize>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
        }
    }
    /// What the reactor needs from the operating system. Object safe: the reactor
    /// holds a `Box<dyn OsBackend>`. See the module documentation for the
    /// edge-triggered contract every implementation must meet.
    pub trait OsBackend: Send {
        /// Starts reporting `fd` under `token` for the directions in `interest`.
        fn register(
            &mut self,
            fd: RawFd,
            token: usize,
            interest: Interest,
        ) -> io::Result<()>;
        /// Replaces the token and interest of an fd registered earlier.
        fn reregister(
            &mut self,
            fd: RawFd,
            token: usize,
            interest: Interest,
        ) -> io::Result<()>;
        /// Stops all reports for `fd`, including pending ones.
        fn deregister(&mut self, fd: RawFd) -> io::Result<()>;
        /// Blocks for at most `timeout` (`None`: no bound) until at least one event
        /// is ready or the interrupt is signalled, and appends the ready events to
        /// `events`.
        fn wait(
            &mut self,
            events: &mut Vec<OsEvent>,
            timeout: Option<Duration>,
        ) -> io::Result<()>;
        /// The backend's cross-thread interrupt. Called once, when the reactor is
        /// built.
        fn interrupt(&self) -> Arc<dyn OsInterrupt>;
    }
    /// The cross-thread half of a backend: makes the owner's current or next
    /// [`OsBackend::wait`] return promptly. Callable from any thread.
    pub trait OsInterrupt: Send + Sync {
        fn signal(&self) -> io::Result<()>;
    }
}
pub mod async_fd {
    //! [`AsyncFd`]: a task waits for a raw fd to become readable or writable.
    //!
    //! This is the one place the reactor's edge-triggered readiness protocol is
    //! written down and implemented for general use. The state is a flag per io
    //! resource and direction (`crate::readiness`) plus, in the reactor, one waker
    //! per direction. The protocol keeps five invariants:
    //!
    //! 1. **Only a park sets a flag.** The reactor's park (`process_io_events`)
    //!    sets a direction's flag, on this thread, when the backend reports an edge
    //!    (or an error or hang-up) for it. Nothing sets a flag between two parks,
    //!    so a task poll is atomic with respect to readiness.
    //! 2. **Only an observed `WouldBlock` clears a flag.** [`AsyncFdReadyGuard::try_io`]
    //!    clears the direction's flag right after the operation returned
    //!    `WouldBlock`, in the same poll. By (1) no edge can have arrived between
    //!    that `WouldBlock` and the clear, so the clear consumes exactly the
    //!    readiness the `WouldBlock` disproved, and the next transition to ready is
    //!    a new edge (the backend contract) that sets the flag at the next park.
    //!    Clearing on any other evidence (a successful operation, a short read on a
    //!    datagram fd, "not connected yet" before the connect edge was consumed) can
    //!    consume the flag of the last edge that will ever come: the lost wakeup of
    //!    HANG_FIXING_STORY.md story 1, where the connect path cleared the writable
    //!    flag after the connection-complete edge and the first write then waited
    //!    forever. This type has no other way to clear a flag.
    //! 3. **Register the waker, then re-check, then return `Pending`.** A poll that
    //!    finds the flag clear stores `cx.waker()` in the reactor for the
    //!    direction, reads the flag again, and returns `Pending` only if it is
    //!    still clear. By (1) the re-read cannot differ on this single-threaded
    //!    reactor; it stays so that the protocol does not rest on that fact
    //!    (register-then-check is the order that survives a flag set concurrently).
    //! 4. **The caller's waker, not the task's.** The waker stored is
    //!    `cx.waker()`, so combinators that give a sub-future its own waker (select,
    //!    join, FuturesUnordered) are woken for the right sub-future, and a wait
    //!    costs no allocation. The reactor keeps a direction's waker until the next
    //!    registration for that direction and wakes it on every edge, so a stale
    //!    waker costs only a spurious wake. There is one waiter per direction: a
    //!    second task waiting on the same direction of the same `AsyncFd` replaces
    //!    the first one's waker (a reader task and a writer task can share one
    //!    `AsyncFd`, e.g. through an `Rc`).
    //! 5. **Ready from the start.** Registration sets both flags (the fd is
    //!    optimistically ready), so the first operation discovers the real state
    //!    and the backend need not report an fd that was already ready when it was
    //!    registered.
    //!
    //! A caller must act on a `Ready` result: if it gets a guard and returns
    //! `Pending` without calling `try_io` (or `poll_*_io`) until `WouldBlock`, no
    //! waker is registered and nothing will wake it.
    //!
    //! `AsyncFd` is `!Send`: the flags, the waker slots and the reactor are
    //! per-thread, and (1) holds only on the reactor's own thread.
    use crate::handle::ReactorHandle;
    use crate::os::RawFd;
    use crate::reactor::enter::current_reactor_epoch;
    use crate::readiness;
    use crate::types::{Interest, IoResult, ResourceId, Source, Waker};
    use std::future::poll_fn;
    use std::io;
    use std::marker::PhantomData;
    use std::task::{Context, Poll};
    /// A raw file descriptor registered with the current thread's reactor for
    /// read and write readiness.
    ///
    /// The fd must be in non-blocking mode (`AsyncFd` never touches it: it neither
    /// sets flags nor reads, writes or closes it) and must stay open until the
    /// `AsyncFd` is dropped; dropping it deregisters the fd.
    pub struct AsyncFd {
        fd: RawFd,
        rid: ResourceId,
        reactor: u64,
        _not_send: PhantomData<*const ()>,
    }
    enum Direction {
        Read,
        Write,
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Direction {}
    #[automatically_derived]
    impl ::core::clone::Clone for Direction {
        #[inline]
        fn clone(&self) -> Direction {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Direction {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Direction {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Direction::Read => "Read",
                    Direction::Write => "Write",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Direction {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Direction {
        #[inline]
        fn eq(&self, other: &Direction) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Direction {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {}
    }
    impl Direction {
        fn interest(self) -> Interest {
            match self {
                Direction::Read => Interest::READABLE,
                Direction::Write => Interest::WRITABLE,
            }
        }
        fn is_ready(self, rid: ResourceId) -> bool {
            match self {
                Direction::Read => readiness::is_readable(rid),
                Direction::Write => readiness::is_writable(rid),
            }
        }
        fn clear(self, rid: ResourceId) {
            match self {
                Direction::Read => readiness::clear_readable(rid),
                Direction::Write => readiness::clear_writable(rid),
            }
        }
    }
    /// Proof that the fd was ready in one direction at the last poll. Its only
    /// operation, [`try_io`](AsyncFdReadyGuard::try_io), runs one non-blocking
    /// operation and consumes the readiness if and only if that operation returned
    /// `WouldBlock`.
    #[must_use = "a ready guard that is dropped without try_io leaves the task with no waker registered"]
    pub struct AsyncFdReadyGuard<'a> {
        fd: &'a AsyncFd,
        dir: Direction,
    }
    impl<'a> AsyncFdReadyGuard<'a> {
        /// Runs `f` on the fd. If it fails with `WouldBlock`, the readiness for this
        /// direction is cleared and `WouldBlock` is returned: poll for readiness
        /// again to wait for the next edge. Any other outcome, success or error,
        /// leaves the readiness set.
        pub fn try_io<R>(self, f: impl FnOnce(RawFd) -> io::Result<R>) -> io::Result<R> {
            let result = f(self.fd.fd);
            if let Err(e) = &result {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.dir.clear(self.fd.rid);
                }
            }
            result
        }
        pub fn fd(&self) -> RawFd {
            self.fd.fd
        }
    }
    impl AsyncFd {
        /// Registers `fd` with the current thread's reactor for read and write
        /// readiness. Fails if no reactor is entered on this thread (no Lion runtime)
        /// or if the backend refuses the registration.
        pub fn new(fd: RawFd) -> io::Result<AsyncFd> {
            let reactor = current_reactor_epoch()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        "AsyncFd::new called on a thread with no Lion reactor",
                    )
                })?;
            let mut source = Source::new(fd);
            match ReactorHandle::new()
                .register_io_resource(&mut source, Interest::READABLE_WRITABLE)
            {
                IoResult::Ok(rid) => {
                    readiness::init_readiness(rid);
                    Ok(AsyncFd {
                        fd,
                        rid,
                        reactor,
                        _not_send: PhantomData,
                    })
                }
                IoResult::Err(e) => Err(e.into_io_error()),
            }
        }
        /// The registered fd.
        pub fn as_raw_fd(&self) -> RawFd {
            self.fd
        }
        /// Ready when the fd may be readable; otherwise registers `cx.waker()` for
        /// the next read edge and returns `Pending`. Fails if polled on a thread
        /// whose current reactor is not the one the fd was registered with.
        pub fn poll_read_ready(
            &self,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<AsyncFdReadyGuard<'_>>> {
            self.poll_ready(Direction::Read, cx)
        }
        /// As [`poll_read_ready`](AsyncFd::poll_read_ready), for writability.
        pub fn poll_write_ready(
            &self,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<AsyncFdReadyGuard<'_>>> {
            self.poll_ready(Direction::Write, cx)
        }
        /// Runs the read operation `f` when the fd is readable, until it does not
        /// return `WouldBlock`; returns `Pending` (with `cx.waker()` registered) when
        /// it would block.
        pub fn poll_read_io<R>(
            &self,
            cx: &mut Context<'_>,
            f: impl FnMut(RawFd) -> io::Result<R>,
        ) -> Poll<io::Result<R>> {
            self.poll_io(Direction::Read, cx, f)
        }
        /// As [`poll_read_io`](AsyncFd::poll_read_io), for a write operation.
        pub fn poll_write_io<R>(
            &self,
            cx: &mut Context<'_>,
            f: impl FnMut(RawFd) -> io::Result<R>,
        ) -> Poll<io::Result<R>> {
            self.poll_io(Direction::Write, cx, f)
        }
        /// Waits until the fd may be readable.
        pub async fn readable(&self) -> io::Result<AsyncFdReadyGuard<'_>> {
            poll_fn(|cx| self.poll_read_ready(cx)).await
        }
        /// Waits until the fd may be writable.
        pub async fn writable(&self) -> io::Result<AsyncFdReadyGuard<'_>> {
            poll_fn(|cx| self.poll_write_ready(cx)).await
        }
        /// Runs the read operation `f` once the fd is readable, retrying after each
        /// `WouldBlock`.
        pub async fn read_io<R>(
            &self,
            mut f: impl FnMut(RawFd) -> io::Result<R>,
        ) -> io::Result<R> {
            poll_fn(|cx| self.poll_read_io(cx, &mut f)).await
        }
        /// Runs the write operation `f` once the fd is writable, retrying after each
        /// `WouldBlock`.
        pub async fn write_io<R>(
            &self,
            mut f: impl FnMut(RawFd) -> io::Result<R>,
        ) -> io::Result<R> {
            poll_fn(|cx| self.poll_write_io(cx, &mut f)).await
        }
        fn check_reactor(&self) -> io::Result<()> {
            if current_reactor_epoch() == Some(self.reactor) {
                Ok(())
            } else {
                Err(
                    io::Error::new(
                        io::ErrorKind::Other,
                        "AsyncFd polled outside the Lion reactor it was registered with",
                    ),
                )
            }
        }
        fn poll_ready(
            &self,
            dir: Direction,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<AsyncFdReadyGuard<'_>>> {
            if let Err(e) = self.check_reactor() {
                return Poll::Ready(Err(e));
            }
            if dir.is_ready(self.rid) {
                return Poll::Ready(Ok(AsyncFdReadyGuard { fd: self, dir }));
            }
            ReactorHandle::new()
                .set_waker(
                    self.rid,
                    dir.interest(),
                    Waker::from_std(cx.waker().clone()),
                );
            if dir.is_ready(self.rid) {
                return Poll::Ready(Ok(AsyncFdReadyGuard { fd: self, dir }));
            }
            Poll::Pending
        }
        fn poll_io<R>(
            &self,
            dir: Direction,
            cx: &mut Context<'_>,
            mut f: impl FnMut(RawFd) -> io::Result<R>,
        ) -> Poll<io::Result<R>> {
            loop {
                let guard = match self.poll_ready(dir, cx) {
                    Poll::Ready(Ok(guard)) => guard,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                };
                match guard.try_io(&mut f) {
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                    result => return Poll::Ready(result),
                }
            }
        }
    }
    impl std::os::fd::AsRawFd for AsyncFd {
        fn as_raw_fd(&self) -> RawFd {
            self.fd
        }
    }
    impl std::fmt::Debug for AsyncFd {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AsyncFd")
                .field("fd", &self.fd)
                .field("rid", &self.rid.0)
                .finish()
        }
    }
    impl Drop for AsyncFd {
        fn drop(&mut self) {
            if current_reactor_epoch() == Some(self.reactor) {
                let mut source = Source::new(self.fd);
                let _ = ReactorHandle::new()
                    .deregister_io_resource(self.rid, &mut source);
            }
        }
    }
}
pub use async_fd::{AsyncFd, AsyncFdReadyGuard};
pub use handle::ReactorHandle;
pub use os::{OsBackend, OsEvent, OsInterrupt, RawFd};
pub use reactor::{Reactor, ReactorGuard};
pub use types::{
    Duration, Instant, Interest, InterruptHandle, IoError, IoResult, ResourceId, Source,
    Waker,
};
