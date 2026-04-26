#![feature(prelude_import)]
//! This crate is a Rust port of Google's high-performance [SwissTable] hash
//! map, adapted to make it a drop-in replacement for Rust's standard `HashMap`
//! and `HashSet` types.
//!
//! The original C++ version of [SwissTable] can be found [here], and this
//! [CppCon talk] gives an overview of how the algorithm works.
//!
//! [SwissTable]: https://abseil.io/blog/20180927-swisstables
//! [here]: https://github.com/abseil/abseil-cpp/blob/master/absl/container/internal/raw_hash_set.h
//! [CppCon talk]: https://www.youtube.com/watch?v=ncHmEUmJZf4
#![no_std]
#![allow(
    clippy::doc_markdown,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::option_if_let_else,
    clippy::redundant_else,
    clippy::manual_map,
    clippy::missing_safety_doc,
    clippy::missing_errors_doc
)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
extern crate core;
#[prelude_import]
use core::prelude::rust_2021::*;
extern crate alloc;
#[macro_use]
mod macros {}
mod control {
    mod bitmask {
        use super::group::{
            BitMaskWord, NonZeroBitMaskWord, BITMASK_ITER_MASK, BITMASK_MASK,
            BITMASK_STRIDE,
        };
        /// A bit mask which contains the result of a `Match` operation on a `Group` and
        /// allows iterating through them.
        ///
        /// The bit mask is arranged so that low-order bits represent lower memory
        /// addresses for group match results.
        ///
        /// For implementation reasons, the bits in the set may be sparsely packed with
        /// groups of 8 bits representing one element. If any of these bits are non-zero
        /// then this element is considered to true in the mask. If this is the
        /// case, `BITMASK_STRIDE` will be 8 to indicate a divide-by-8 should be
        /// performed on counts/indices to normalize this difference. `BITMASK_MASK` is
        /// similarly a mask of all the actually-used bits.
        ///
        /// To iterate over a bit mask, it must be converted to a form where only 1 bit
        /// is set per element. This is done by applying `BITMASK_ITER_MASK` on the
        /// mask bits.
        pub(crate) struct BitMask(pub(crate) BitMaskWord);
        #[automatically_derived]
        impl ::core::marker::Copy for BitMask {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for BitMask {}
        #[automatically_derived]
        impl ::core::clone::Clone for BitMask {
            #[inline]
            fn clone(&self) -> BitMask {
                let _: ::core::clone::AssertParamIsClone<BitMaskWord>;
                *self
            }
        }
        #[allow(clippy::use_self)]
        impl BitMask {
            /// Returns a new `BitMask` with all bits inverted.
            #[inline]
            #[must_use]
            #[allow(dead_code)]
            pub(crate) fn invert(self) -> Self {
                BitMask(self.0 ^ BITMASK_MASK)
            }
            /// Returns a new `BitMask` with the lowest bit removed.
            #[inline]
            #[must_use]
            fn remove_lowest_bit(self) -> Self {
                BitMask(self.0 & (self.0 - 1))
            }
            /// Returns whether the `BitMask` has at least one set bit.
            #[inline]
            pub(crate) fn any_bit_set(self) -> bool {
                self.0 != 0
            }
            /// Returns the first set bit in the `BitMask`, if there is one.
            #[inline]
            pub(crate) fn lowest_set_bit(self) -> Option<usize> {
                if let Some(nonzero) = NonZeroBitMaskWord::new(self.0) {
                    Some(Self::nonzero_trailing_zeros(nonzero))
                } else {
                    None
                }
            }
            /// Returns the number of trailing zeroes in the `BitMask`.
            #[inline]
            pub(crate) fn trailing_zeros(self) -> usize {
                if false && BITMASK_STRIDE % 8 == 0 {
                    self.0.swap_bytes().leading_zeros() as usize / BITMASK_STRIDE
                } else {
                    self.0.trailing_zeros() as usize / BITMASK_STRIDE
                }
            }
            /// Same as above but takes a `NonZeroBitMaskWord`.
            #[inline]
            fn nonzero_trailing_zeros(nonzero: NonZeroBitMaskWord) -> usize {
                if false && BITMASK_STRIDE % 8 == 0 {
                    let swapped = unsafe {
                        NonZeroBitMaskWord::new_unchecked(nonzero.get().swap_bytes())
                    };
                    swapped.leading_zeros() as usize / BITMASK_STRIDE
                } else {
                    nonzero.trailing_zeros() as usize / BITMASK_STRIDE
                }
            }
            /// Returns the number of leading zeroes in the `BitMask`.
            #[inline]
            pub(crate) fn leading_zeros(self) -> usize {
                self.0.leading_zeros() as usize / BITMASK_STRIDE
            }
        }
        impl IntoIterator for BitMask {
            type Item = usize;
            type IntoIter = BitMaskIter;
            #[inline]
            fn into_iter(self) -> BitMaskIter {
                BitMaskIter(BitMask(self.0 & BITMASK_ITER_MASK))
            }
        }
        /// Iterator over the contents of a `BitMask`, returning the indices of set
        /// bits.
        pub(crate) struct BitMaskIter(pub(crate) BitMask);
        #[automatically_derived]
        impl ::core::clone::Clone for BitMaskIter {
            #[inline]
            fn clone(&self) -> BitMaskIter {
                BitMaskIter(::core::clone::Clone::clone(&self.0))
            }
        }
        impl Iterator for BitMaskIter {
            type Item = usize;
            #[inline]
            fn next(&mut self) -> Option<usize> {
                let bit = self.0.lowest_set_bit()?;
                self.0 = self.0.remove_lowest_bit();
                Some(bit)
            }
        }
    }
    mod group {
        mod sse2 {
            use super::super::{BitMask, Tag};
            use core::mem;
            use core::num::NonZeroU16;
            use core::arch::x86_64 as x86;
            pub(crate) type BitMaskWord = u16;
            pub(crate) type NonZeroBitMaskWord = NonZeroU16;
            pub(crate) const BITMASK_STRIDE: usize = 1;
            pub(crate) const BITMASK_MASK: BitMaskWord = 0xffff;
            pub(crate) const BITMASK_ITER_MASK: BitMaskWord = !0;
            /// Abstraction over a group of control tags which can be scanned in
            /// parallel.
            ///
            /// This implementation uses a 128-bit SSE value.
            pub(crate) struct Group(x86::__m128i);
            #[automatically_derived]
            impl ::core::marker::Copy for Group {}
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Group {}
            #[automatically_derived]
            impl ::core::clone::Clone for Group {
                #[inline]
                fn clone(&self) -> Group {
                    let _: ::core::clone::AssertParamIsClone<x86::__m128i>;
                    *self
                }
            }
            #[allow(clippy::use_self)]
            impl Group {
                /// Number of bytes in the group.
                pub(crate) const WIDTH: usize = mem::size_of::<Self>();
                /// Returns a full group of empty tags, suitable for use as the initial
                /// value for an empty hash table.
                ///
                /// This is guaranteed to be aligned to the group size.
                #[inline]
                #[allow(clippy::items_after_statements)]
                pub(crate) const fn static_empty() -> &'static [Tag; Group::WIDTH] {
                    #[repr(C)]
                    struct AlignedTags {
                        _align: [Group; 0],
                        tags: [Tag; Group::WIDTH],
                    }
                    const ALIGNED_TAGS: AlignedTags = AlignedTags {
                        _align: [],
                        tags: [Tag::EMPTY; Group::WIDTH],
                    };
                    &ALIGNED_TAGS.tags
                }
                /// Loads a group of tags starting at the given address.
                #[inline]
                #[allow(clippy::cast_ptr_alignment)]
                pub(crate) unsafe fn load(ptr: *const Tag) -> Self {
                    Group(x86::_mm_loadu_si128(ptr.cast()))
                }
                /// Loads a group of tags starting at the given address, which must be
                /// aligned to `mem::align_of::<Group>()`.
                #[inline]
                #[allow(clippy::cast_ptr_alignment)]
                pub(crate) unsafe fn load_aligned(ptr: *const Tag) -> Self {
                    if true {
                        match (&ptr.align_offset(mem::align_of::<Self>()), &0) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                    }
                    Group(x86::_mm_load_si128(ptr.cast()))
                }
                /// Stores the group of tags to the given address, which must be
                /// aligned to `mem::align_of::<Group>()`.
                #[inline]
                #[allow(clippy::cast_ptr_alignment)]
                pub(crate) unsafe fn store_aligned(self, ptr: *mut Tag) {
                    if true {
                        match (&ptr.align_offset(mem::align_of::<Self>()), &0) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                    }
                    x86::_mm_store_si128(ptr.cast(), self.0);
                }
                /// Returns a `BitMask` indicating all tags in the group which have
                /// the given value.
                #[inline]
                pub(crate) fn match_tag(self, tag: Tag) -> BitMask {
                    #[allow(
                        clippy::cast_possible_wrap,
                        clippy::cast_sign_loss,
                        clippy::cast_possible_truncation
                    )]
                    unsafe {
                        let cmp = x86::_mm_cmpeq_epi8(
                            self.0,
                            x86::_mm_set1_epi8(tag.0 as i8),
                        );
                        BitMask(x86::_mm_movemask_epi8(cmp) as u16)
                    }
                }
                /// Returns a `BitMask` indicating all tags in the group which are
                /// `EMPTY`.
                #[inline]
                pub(crate) fn match_empty(self) -> BitMask {
                    self.match_tag(Tag::EMPTY)
                }
                /// Returns a `BitMask` indicating all tags in the group which are
                /// `EMPTY` or `DELETED`.
                #[inline]
                pub(crate) fn match_empty_or_deleted(self) -> BitMask {
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    unsafe { BitMask(x86::_mm_movemask_epi8(self.0) as u16) }
                }
                /// Returns a `BitMask` indicating all tags in the group which are full.
                #[inline]
                pub(crate) fn match_full(&self) -> BitMask {
                    self.match_empty_or_deleted().invert()
                }
                /// Performs the following transformation on all tags in the group:
                /// - `EMPTY => EMPTY`
                /// - `DELETED => EMPTY`
                /// - `FULL => DELETED`
                #[inline]
                pub(crate) fn convert_special_to_empty_and_full_to_deleted(
                    self,
                ) -> Self {
                    #[allow(clippy::cast_possible_wrap)]
                    unsafe {
                        let zero = x86::_mm_setzero_si128();
                        let special = x86::_mm_cmpgt_epi8(zero, self.0);
                        Group(
                            x86::_mm_or_si128(
                                special,
                                x86::_mm_set1_epi8(Tag::DELETED.0 as i8),
                            ),
                        )
                    }
                }
            }
        }
        use sse2 as imp;
        pub(crate) use self::imp::Group;
        pub(super) use self::imp::{
            BitMaskWord, NonZeroBitMaskWord, BITMASK_ITER_MASK, BITMASK_MASK,
            BITMASK_STRIDE,
        };
    }
    mod tag {
        use core::{fmt, mem};
        /// Single tag in a control group.
        #[repr(transparent)]
        pub(crate) struct Tag(pub(super) u8);
        #[automatically_derived]
        impl ::core::marker::Copy for Tag {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Tag {}
        #[automatically_derived]
        impl ::core::clone::Clone for Tag {
            #[inline]
            fn clone(&self) -> Tag {
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Tag {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Tag {
            #[inline]
            fn eq(&self, other: &Tag) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Tag {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u8>;
            }
        }
        impl Tag {
            /// Control tag value for an empty bucket.
            pub(crate) const EMPTY: Tag = Tag(0b1111_1111);
            /// Control tag value for a deleted bucket.
            pub(crate) const DELETED: Tag = Tag(0b1000_0000);
            /// Checks whether a control tag represents a full bucket (top bit is clear).
            #[inline]
            pub(crate) const fn is_full(self) -> bool {
                self.0 & 0x80 == 0
            }
            /// Checks whether a control tag represents a special value (top bit is set).
            #[inline]
            pub(crate) const fn is_special(self) -> bool {
                self.0 & 0x80 != 0
            }
            /// Checks whether a special control value is EMPTY (just check 1 bit).
            #[inline]
            pub(crate) const fn special_is_empty(self) -> bool {
                if true {
                    if !self.is_special() {
                        ::core::panicking::panic("assertion failed: self.is_special()")
                    }
                }
                self.0 & 0x01 != 0
            }
            /// Creates a control tag representing a full bucket with the given hash.
            #[inline]
            #[allow(clippy::cast_possible_truncation)]
            pub(crate) const fn full(hash: u64) -> Tag {
                const MIN_HASH_LEN: usize = if mem::size_of::<usize>()
                    < mem::size_of::<u64>()
                {
                    mem::size_of::<usize>()
                } else {
                    mem::size_of::<u64>()
                };
                let top7 = hash >> (MIN_HASH_LEN * 8 - 7);
                Tag((top7 & 0x7f) as u8)
            }
        }
        impl fmt::Debug for Tag {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.is_special() {
                    if self.special_is_empty() {
                        f.pad("EMPTY")
                    } else {
                        f.pad("DELETED")
                    }
                } else {
                    f.debug_tuple("full").field(&(self.0 & 0x7F)).finish()
                }
            }
        }
        /// Extension trait for slices of tags.
        pub(crate) trait TagSliceExt {
            /// Fills the control with the given tag.
            fn fill_tag(&mut self, tag: Tag);
            /// Clears out the control.
            #[inline]
            fn fill_empty(&mut self) {
                self.fill_tag(Tag::EMPTY)
            }
        }
        impl TagSliceExt for [Tag] {
            #[inline]
            fn fill_tag(&mut self, tag: Tag) {
                unsafe { self.as_mut_ptr().write_bytes(tag.0, self.len()) }
            }
        }
    }
    use self::bitmask::BitMask;
    pub(crate) use self::{bitmask::BitMaskIter, group::Group, tag::{Tag, TagSliceExt}};
}
mod hasher {
    /// Default hash builder for the `S` type parameter of
    /// [`HashMap`](crate::HashMap) and [`HashSet`](crate::HashSet).
    ///
    /// This only implements `BuildHasher` when the "default-hasher" crate feature
    /// is enabled; otherwise it just serves as a placeholder, and a custom `S` type
    /// must be used to have a fully functional `HashMap` or `HashSet`.
    pub struct DefaultHashBuilder {}
    #[automatically_derived]
    impl ::core::clone::Clone for DefaultHashBuilder {
        #[inline]
        fn clone(&self) -> DefaultHashBuilder {
            DefaultHashBuilder {}
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for DefaultHashBuilder {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "DefaultHashBuilder")
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for DefaultHashBuilder {
        #[inline]
        fn default() -> DefaultHashBuilder {
            DefaultHashBuilder {}
        }
    }
}
mod raw {
    use crate::alloc::alloc::{handle_alloc_error, Layout};
    use crate::control::{BitMaskIter, Group, Tag, TagSliceExt};
    use crate::scopeguard::{guard, ScopeGuard};
    use crate::util::{invalid_mut, likely, unlikely};
    use crate::TryReserveError;
    use core::array;
    use core::iter::FusedIterator;
    use core::marker::PhantomData;
    use core::mem;
    use core::ptr::NonNull;
    use core::slice;
    use core::{hint, ptr};
    mod alloc {
        pub(crate) use self::inner::{do_alloc, Allocator, Global};
        mod inner {
            use crate::alloc::alloc::{alloc, dealloc, Layout};
            use core::ptr::NonNull;
            #[allow(clippy::missing_safety_doc)]
            pub unsafe trait Allocator {
                fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, ()>;
                unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);
            }
            pub struct Global;
            #[automatically_derived]
            impl ::core::marker::Copy for Global {}
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Global {}
            #[automatically_derived]
            impl ::core::clone::Clone for Global {
                #[inline]
                fn clone(&self) -> Global {
                    *self
                }
            }
            unsafe impl Allocator for Global {
                #[inline]
                fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, ()> {
                    match unsafe { NonNull::new(alloc(layout)) } {
                        Some(data) => {
                            Ok(unsafe {
                                NonNull::new_unchecked(
                                    core::ptr::slice_from_raw_parts_mut(
                                        data.as_ptr(),
                                        layout.size(),
                                    ),
                                )
                            })
                        }
                        None => Err(()),
                    }
                }
                #[inline]
                unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                    dealloc(ptr.as_ptr(), layout);
                }
            }
            impl Default for Global {
                #[inline]
                fn default() -> Self {
                    Global
                }
            }
            pub(crate) fn do_alloc<A: Allocator>(
                alloc: &A,
                layout: Layout,
            ) -> Result<NonNull<[u8]>, ()> {
                alloc.allocate(layout)
            }
        }
    }
    pub(crate) use self::alloc::{do_alloc, Allocator, Global};
    #[inline]
    unsafe fn offset_from<T>(to: *const T, from: *const T) -> usize {
        to.offset_from(from) as usize
    }
    /// Whether memory allocation errors should return an error or abort.
    enum Fallibility {
        Fallible,
        Infallible,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Fallibility {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Fallibility {}
    #[automatically_derived]
    impl ::core::clone::Clone for Fallibility {
        #[inline]
        fn clone(&self) -> Fallibility {
            *self
        }
    }
    impl Fallibility {
        /// Error to return on capacity overflow.
        fn capacity_overflow(self) -> TryReserveError {
            match self {
                Fallibility::Fallible => TryReserveError::CapacityOverflow,
                Fallibility::Infallible => {
                    ::core::panicking::panic_fmt(
                        format_args!("Hash table capacity overflow"),
                    );
                }
            }
        }
        /// Error to return on allocation error.
        fn alloc_err(self, layout: Layout) -> TryReserveError {
            match self {
                Fallibility::Fallible => {
                    TryReserveError::AllocError {
                        layout,
                    }
                }
                Fallibility::Infallible => handle_alloc_error(layout),
            }
        }
    }
    trait SizedTypeProperties: Sized {
        const IS_ZERO_SIZED: bool = mem::size_of::<Self>() == 0;
        const NEEDS_DROP: bool = mem::needs_drop::<Self>();
    }
    impl<T> SizedTypeProperties for T {}
    /// Primary hash function, used to select the initial bucket to probe from.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn h1(hash: u64) -> usize {
        hash as usize
    }
    /// Probe sequence based on triangular numbers, which is guaranteed (since our
    /// table size is a power of two) to visit every group of elements exactly once.
    ///
    /// A triangular probe has us jump by 1 more group every time. So first we
    /// jump by 1 group (meaning we just continue our linear scan), then 2 groups
    /// (skipping over 1 group), then 3 groups (skipping over 2 groups), and so on.
    ///
    /// Proof that the probe will visit every group in the table:
    /// <https://fgiesen.wordpress.com/2015/02/22/triangular-numbers-mod-2n/>
    struct ProbeSeq {
        pos: usize,
        stride: usize,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ProbeSeq {
        #[inline]
        fn clone(&self) -> ProbeSeq {
            ProbeSeq {
                pos: ::core::clone::Clone::clone(&self.pos),
                stride: ::core::clone::Clone::clone(&self.stride),
            }
        }
    }
    impl ProbeSeq {
        #[inline]
        fn move_next(&mut self, bucket_mask: usize) {
            if true {
                if !(self.stride <= bucket_mask) {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("Went past end of probe sequence"),
                        );
                    }
                }
            }
            self.stride += Group::WIDTH;
            self.pos += self.stride;
            self.pos &= bucket_mask;
        }
    }
    /// Returns the number of buckets needed to hold the given number of items,
    /// taking the maximum load factor into account.
    ///
    /// Returns `None` if an overflow occurs.
    ///
    /// This ensures that `buckets * table_layout.size >= table_layout.ctrl_align`.
    #[inline]
    fn capacity_to_buckets(cap: usize, table_layout: TableLayout) -> Option<usize> {
        if true {
            match (&cap, &0) {
                (left_val, right_val) => {
                    if *left_val == *right_val {
                        let kind = ::core::panicking::AssertKind::Ne;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        }
        if cap < 15 {
            let min_cap = match (Group::WIDTH, table_layout.size) {
                (16, 0..=1) => 14,
                (16, 2..=3) => 7,
                (8, 0..=1) => 7,
                _ => 3,
            };
            let cap = min_cap.max(cap);
            let buckets = if cap < 4 { 4 } else if cap < 8 { 8 } else { 16 };
            ensure_bucket_bytes_at_least_ctrl_align(table_layout, buckets);
            return Some(buckets);
        }
        let adjusted_cap = cap.checked_mul(8)? / 7;
        let buckets = adjusted_cap.next_power_of_two();
        ensure_bucket_bytes_at_least_ctrl_align(table_layout, buckets);
        Some(buckets)
    }
    #[inline]
    fn ensure_bucket_bytes_at_least_ctrl_align(
        table_layout: TableLayout,
        buckets: usize,
    ) {
        if table_layout.size != 0 {
            let prod = table_layout.size.saturating_mul(buckets);
            if true {
                if !(prod >= table_layout.ctrl_align) {
                    ::core::panicking::panic(
                        "assertion failed: prod >= table_layout.ctrl_align",
                    )
                }
            }
        }
    }
    /// Returns the maximum effective capacity for the given bucket mask, taking
    /// the maximum load factor into account.
    #[inline]
    fn bucket_mask_to_capacity(bucket_mask: usize) -> usize {
        if bucket_mask < 8 { bucket_mask } else { ((bucket_mask + 1) / 8) * 7 }
    }
    /// Helper which allows the max calculation for `ctrl_align` to be statically computed for each `T`
    /// while keeping the rest of `calculate_layout_for` independent of `T`
    struct TableLayout {
        size: usize,
        ctrl_align: usize,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TableLayout {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TableLayout {}
    #[automatically_derived]
    impl ::core::clone::Clone for TableLayout {
        #[inline]
        fn clone(&self) -> TableLayout {
            let _: ::core::clone::AssertParamIsClone<usize>;
            *self
        }
    }
    impl TableLayout {
        #[inline]
        const fn new<T>() -> Self {
            let layout = Layout::new::<T>();
            Self {
                size: layout.size(),
                ctrl_align: if layout.align() > Group::WIDTH {
                    layout.align()
                } else {
                    Group::WIDTH
                },
            }
        }
        #[inline]
        fn calculate_layout_for(self, buckets: usize) -> Option<(Layout, usize)> {
            if true {
                if !buckets.is_power_of_two() {
                    ::core::panicking::panic(
                        "assertion failed: buckets.is_power_of_two()",
                    )
                }
            }
            let TableLayout { size, ctrl_align } = self;
            let ctrl_offset = size.checked_mul(buckets)?.checked_add(ctrl_align - 1)?
                & !(ctrl_align - 1);
            let len = ctrl_offset.checked_add(buckets + Group::WIDTH)?;
            if len > isize::MAX as usize - (ctrl_align - 1) {
                return None;
            }
            Some((
                unsafe { Layout::from_size_align_unchecked(len, ctrl_align) },
                ctrl_offset,
            ))
        }
    }
    /// A reference to a hash table bucket containing a `T`.
    ///
    /// This is usually just a pointer to the element itself. However if the element
    /// is a ZST, then we instead track the index of the element in the table so
    /// that `erase` works properly.
    pub struct Bucket<T> {
        ptr: NonNull<T>,
    }
    unsafe impl<T> Send for Bucket<T> {}
    impl<T> Clone for Bucket<T> {
        #[inline]
        fn clone(&self) -> Self {
            Self { ptr: self.ptr }
        }
    }
    impl<T> Bucket<T> {
        /// Creates a [`Bucket`] that contain pointer to the data.
        /// The pointer calculation is performed by calculating the
        /// offset from given `base` pointer (convenience for
        /// `base.as_ptr().sub(index)`).
        ///
        /// `index` is in units of `T`; e.g., an `index` of 3 represents a pointer
        /// offset of `3 * size_of::<T>()` bytes.
        ///
        /// If the `T` is a ZST, then we instead track the index of the element
        /// in the table so that `erase` works properly (return
        /// `NonNull::new_unchecked((index + 1) as *mut T)`)
        ///
        /// # Safety
        ///
        /// If `mem::size_of::<T>() != 0`, then the safety rules are directly derived
        /// from the safety rules for [`<*mut T>::sub`] method of `*mut T` and the safety
        /// rules of [`NonNull::new_unchecked`] function.
        ///
        /// Thus, in order to uphold the safety contracts for the [`<*mut T>::sub`] method
        /// and [`NonNull::new_unchecked`] function, as well as for the correct
        /// logic of the work of this crate, the following rules are necessary and
        /// sufficient:
        ///
        /// * the `base` pointer must not be `dangling` and must points to the
        ///   end of the first `value element` from the `data part` of the table, i.e.
        ///   must be the pointer that returned by [`RawTable::data_end`] or by
        ///   [`RawTableInner::data_end<T>`];
        ///
        /// * `index` must not be greater than `RawTableInner.bucket_mask`, i.e.
        ///   `index <= RawTableInner.bucket_mask` or, in other words, `(index + 1)`
        ///   must be no greater than the number returned by the function
        ///   [`RawTable::buckets`] or [`RawTableInner::buckets`].
        ///
        /// If `mem::size_of::<T>() == 0`, then the only requirement is that the
        /// `index` must not be greater than `RawTableInner.bucket_mask`, i.e.
        /// `index <= RawTableInner.bucket_mask` or, in other words, `(index + 1)`
        /// must be no greater than the number returned by the function
        /// [`RawTable::buckets`] or [`RawTableInner::buckets`].
        ///
        /// [`Bucket`]: crate::raw::Bucket
        /// [`<*mut T>::sub`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.sub-1
        /// [`NonNull::new_unchecked`]: https://doc.rust-lang.org/stable/std/ptr/struct.NonNull.html#method.new_unchecked
        /// [`RawTable::data_end`]: crate::raw::RawTable::data_end
        /// [`RawTableInner::data_end<T>`]: RawTableInner::data_end<T>
        /// [`RawTable::buckets`]: crate::raw::RawTable::buckets
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        #[inline]
        unsafe fn from_base_index(base: NonNull<T>, index: usize) -> Self {
            let ptr = if T::IS_ZERO_SIZED {
                invalid_mut(index + 1)
            } else {
                base.as_ptr().sub(index)
            };
            Self {
                ptr: NonNull::new_unchecked(ptr),
            }
        }
        /// Calculates the index of a [`Bucket`] as distance between two pointers
        /// (convenience for `base.as_ptr().offset_from(self.ptr.as_ptr()) as usize`).
        /// The returned value is in units of T: the distance in bytes divided by
        /// [`core::mem::size_of::<T>()`].
        ///
        /// If the `T` is a ZST, then we return the index of the element in
        /// the table so that `erase` works properly (return `self.ptr.as_ptr() as usize - 1`).
        ///
        /// This function is the inverse of [`from_base_index`].
        ///
        /// # Safety
        ///
        /// If `mem::size_of::<T>() != 0`, then the safety rules are directly derived
        /// from the safety rules for [`<*const T>::offset_from`] method of `*const T`.
        ///
        /// Thus, in order to uphold the safety contracts for [`<*const T>::offset_from`]
        /// method, as well as for the correct logic of the work of this crate, the
        /// following rules are necessary and sufficient:
        ///
        /// * `base` contained pointer must not be `dangling` and must point to the
        ///   end of the first `element` from the `data part` of the table, i.e.
        ///   must be a pointer that returns by [`RawTable::data_end`] or by
        ///   [`RawTableInner::data_end<T>`];
        ///
        /// * `self` also must not contain dangling pointer;
        ///
        /// * both `self` and `base` must be created from the same [`RawTable`]
        ///   (or [`RawTableInner`]).
        ///
        /// If `mem::size_of::<T>() == 0`, this function is always safe.
        ///
        /// [`Bucket`]: crate::raw::Bucket
        /// [`from_base_index`]: crate::raw::Bucket::from_base_index
        /// [`RawTable::data_end`]: crate::raw::RawTable::data_end
        /// [`RawTableInner::data_end<T>`]: RawTableInner::data_end<T>
        /// [`RawTable`]: crate::raw::RawTable
        /// [`RawTableInner`]: RawTableInner
        /// [`<*const T>::offset_from`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.offset_from
        #[inline]
        unsafe fn to_base_index(&self, base: NonNull<T>) -> usize {
            if T::IS_ZERO_SIZED {
                self.ptr.as_ptr() as usize - 1
            } else {
                offset_from(base.as_ptr(), self.ptr.as_ptr())
            }
        }
        /// Acquires the underlying raw pointer `*mut T` to `data`.
        ///
        /// # Note
        ///
        /// If `T` is not [`Copy`], do not use `*mut T` methods that can cause calling the
        /// destructor of `T` (for example the [`<*mut T>::drop_in_place`] method), because
        /// for properly dropping the data we also need to clear `data` control bytes. If we
        /// drop data, but do not clear `data control byte` it leads to double drop when
        /// [`RawTable`] goes out of scope.
        ///
        /// If you modify an already initialized `value`, so [`Hash`] and [`Eq`] on the new
        /// `T` value and its borrowed form *must* match those for the old `T` value, as the map
        /// will not re-evaluate where the new value should go, meaning the value may become
        /// "lost" if their location does not reflect their state.
        ///
        /// [`RawTable`]: crate::raw::RawTable
        /// [`<*mut T>::drop_in_place`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.drop_in_place
        /// [`Hash`]: https://doc.rust-lang.org/core/hash/trait.Hash.html
        /// [`Eq`]: https://doc.rust-lang.org/core/cmp/trait.Eq.html
        #[inline]
        pub fn as_ptr(&self) -> *mut T {
            if T::IS_ZERO_SIZED {
                invalid_mut(mem::align_of::<T>())
            } else {
                unsafe { self.ptr.as_ptr().sub(1) }
            }
        }
        /// Acquires the underlying non-null pointer `*mut T` to `data`.
        #[inline]
        fn as_non_null(&self) -> NonNull<T> {
            unsafe { NonNull::new_unchecked(self.as_ptr()) }
        }
        /// Create a new [`Bucket`] that is offset from the `self` by the given
        /// `offset`. The pointer calculation is performed by calculating the
        /// offset from `self` pointer (convenience for `self.ptr.as_ptr().sub(offset)`).
        /// This function is used for iterators.
        ///
        /// `offset` is in units of `T`; e.g., a `offset` of 3 represents a pointer
        /// offset of `3 * size_of::<T>()` bytes.
        ///
        /// # Safety
        ///
        /// If `mem::size_of::<T>() != 0`, then the safety rules are directly derived
        /// from the safety rules for [`<*mut T>::sub`] method of `*mut T` and safety
        /// rules of [`NonNull::new_unchecked`] function.
        ///
        /// Thus, in order to uphold the safety contracts for [`<*mut T>::sub`] method
        /// and [`NonNull::new_unchecked`] function, as well as for the correct
        /// logic of the work of this crate, the following rules are necessary and
        /// sufficient:
        ///
        /// * `self` contained pointer must not be `dangling`;
        ///
        /// * `self.to_base_index() + offset` must not be greater than `RawTableInner.bucket_mask`,
        ///   i.e. `(self.to_base_index() + offset) <= RawTableInner.bucket_mask` or, in other
        ///   words, `self.to_base_index() + offset + 1` must be no greater than the number returned
        ///   by the function [`RawTable::buckets`] or [`RawTableInner::buckets`].
        ///
        /// If `mem::size_of::<T>() == 0`, then the only requirement is that the
        /// `self.to_base_index() + offset` must not be greater than `RawTableInner.bucket_mask`,
        /// i.e. `(self.to_base_index() + offset) <= RawTableInner.bucket_mask` or, in other words,
        /// `self.to_base_index() + offset + 1` must be no greater than the number returned by the
        /// function [`RawTable::buckets`] or [`RawTableInner::buckets`].
        ///
        /// [`Bucket`]: crate::raw::Bucket
        /// [`<*mut T>::sub`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.sub-1
        /// [`NonNull::new_unchecked`]: https://doc.rust-lang.org/stable/std/ptr/struct.NonNull.html#method.new_unchecked
        /// [`RawTable::buckets`]: crate::raw::RawTable::buckets
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        #[inline]
        unsafe fn next_n(&self, offset: usize) -> Self {
            let ptr = if T::IS_ZERO_SIZED {
                invalid_mut(self.ptr.as_ptr() as usize + offset)
            } else {
                self.ptr.as_ptr().sub(offset)
            };
            Self {
                ptr: NonNull::new_unchecked(ptr),
            }
        }
        /// Executes the destructor (if any) of the pointed-to `data`.
        ///
        /// # Safety
        ///
        /// See [`ptr::drop_in_place`] for safety concerns.
        ///
        /// You should use [`RawTable::erase`] instead of this function,
        /// or be careful with calling this function directly, because for
        /// properly dropping the data we need also clear `data` control bytes.
        /// If we drop data, but do not erase `data control byte` it leads to
        /// double drop when [`RawTable`] goes out of scope.
        ///
        /// [`ptr::drop_in_place`]: https://doc.rust-lang.org/core/ptr/fn.drop_in_place.html
        /// [`RawTable`]: crate::raw::RawTable
        /// [`RawTable::erase`]: crate::raw::RawTable::erase
        pub(crate) unsafe fn drop(&self) {
            self.as_ptr().drop_in_place();
        }
        /// Reads the `value` from `self` without moving it. This leaves the
        /// memory in `self` unchanged.
        ///
        /// # Safety
        ///
        /// See [`ptr::read`] for safety concerns.
        ///
        /// You should use [`RawTable::remove`] instead of this function,
        /// or be careful with calling this function directly, because compiler
        /// calls its destructor when the read `value` goes out of scope. It
        /// can cause double dropping when [`RawTable`] goes out of scope,
        /// because of not erased `data control byte`.
        ///
        /// [`ptr::read`]: https://doc.rust-lang.org/core/ptr/fn.read.html
        /// [`RawTable`]: crate::raw::RawTable
        /// [`RawTable::remove`]: crate::raw::RawTable::remove
        #[inline]
        pub(crate) unsafe fn read(&self) -> T {
            self.as_ptr().read()
        }
        /// Overwrites a memory location with the given `value` without reading
        /// or dropping the old value (like [`ptr::write`] function).
        ///
        /// # Safety
        ///
        /// See [`ptr::write`] for safety concerns.
        ///
        /// # Note
        ///
        /// [`Hash`] and [`Eq`] on the new `T` value and its borrowed form *must* match
        /// those for the old `T` value, as the map will not re-evaluate where the new
        /// value should go, meaning the value may become "lost" if their location
        /// does not reflect their state.
        ///
        /// [`ptr::write`]: https://doc.rust-lang.org/core/ptr/fn.write.html
        /// [`Hash`]: https://doc.rust-lang.org/core/hash/trait.Hash.html
        /// [`Eq`]: https://doc.rust-lang.org/core/cmp/trait.Eq.html
        #[inline]
        pub(crate) unsafe fn write(&self, val: T) {
            self.as_ptr().write(val);
        }
        /// Returns a shared immutable reference to the `value`.
        ///
        /// # Safety
        ///
        /// See [`NonNull::as_ref`] for safety concerns.
        ///
        /// [`NonNull::as_ref`]: https://doc.rust-lang.org/core/ptr/struct.NonNull.html#method.as_ref
        #[inline]
        pub unsafe fn as_ref<'a>(&self) -> &'a T {
            &*self.as_ptr()
        }
        /// Returns a unique mutable reference to the `value`.
        ///
        /// # Safety
        ///
        /// See [`NonNull::as_mut`] for safety concerns.
        ///
        /// # Note
        ///
        /// [`Hash`] and [`Eq`] on the new `T` value and its borrowed form *must* match
        /// those for the old `T` value, as the map will not re-evaluate where the new
        /// value should go, meaning the value may become "lost" if their location
        /// does not reflect their state.
        ///
        /// [`NonNull::as_mut`]: https://doc.rust-lang.org/core/ptr/struct.NonNull.html#method.as_mut
        /// [`Hash`]: https://doc.rust-lang.org/core/hash/trait.Hash.html
        /// [`Eq`]: https://doc.rust-lang.org/core/cmp/trait.Eq.html
        #[inline]
        pub unsafe fn as_mut<'a>(&self) -> &'a mut T {
            &mut *self.as_ptr()
        }
    }
    /// A raw hash table with an unsafe API.
    pub struct RawTable<T, A: Allocator = Global> {
        table: RawTableInner,
        alloc: A,
        marker: PhantomData<T>,
    }
    /// Non-generic part of `RawTable` which allows functions to be instantiated only once regardless
    /// of how many different key-value types are used.
    struct RawTableInner {
        bucket_mask: usize,
        ctrl: NonNull<u8>,
        growth_left: usize,
        items: usize,
    }
    impl<T> RawTable<T, Global> {
        /// Creates a new empty hash table without allocating any memory.
        ///
        /// In effect this returns a table with exactly 1 bucket. However we can
        /// leave the data pointer dangling since that bucket is never written to
        /// due to our load factor forcing us to always have at least 1 free bucket.
        #[inline]
        pub const fn new() -> Self {
            Self {
                table: RawTableInner::NEW,
                alloc: Global,
                marker: PhantomData,
            }
        }
        /// Allocates a new hash table with at least enough capacity for inserting
        /// the given number of elements without reallocating.
        pub fn with_capacity(capacity: usize) -> Self {
            Self::with_capacity_in(capacity, Global)
        }
    }
    impl<T, A: Allocator> RawTable<T, A> {
        const TABLE_LAYOUT: TableLayout = TableLayout::new::<T>();
        /// Creates a new empty hash table without allocating any memory, using the
        /// given allocator.
        ///
        /// In effect this returns a table with exactly 1 bucket. However we can
        /// leave the data pointer dangling since that bucket is never written to
        /// due to our load factor forcing us to always have at least 1 free bucket.
        #[inline]
        pub const fn new_in(alloc: A) -> Self {
            Self {
                table: RawTableInner::NEW,
                alloc,
                marker: PhantomData,
            }
        }
        /// Allocates a new hash table with the given number of buckets.
        ///
        /// The control bytes are left uninitialized.
        unsafe fn new_uninitialized(
            alloc: A,
            buckets: usize,
            fallibility: Fallibility,
        ) -> Result<Self, TryReserveError> {
            if true {
                if !buckets.is_power_of_two() {
                    ::core::panicking::panic(
                        "assertion failed: buckets.is_power_of_two()",
                    )
                }
            }
            Ok(Self {
                table: RawTableInner::new_uninitialized(
                    &alloc,
                    Self::TABLE_LAYOUT,
                    buckets,
                    fallibility,
                )?,
                alloc,
                marker: PhantomData,
            })
        }
        /// Allocates a new hash table using the given allocator, with at least enough capacity for
        /// inserting the given number of elements without reallocating.
        pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
            Self {
                table: RawTableInner::with_capacity(
                    &alloc,
                    Self::TABLE_LAYOUT,
                    capacity,
                ),
                alloc,
                marker: PhantomData,
            }
        }
        /// Returns a reference to the underlying allocator.
        #[inline]
        pub fn allocator(&self) -> &A {
            &self.alloc
        }
        /// Returns pointer to one past last `data` element in the table as viewed from
        /// the start point of the allocation.
        ///
        /// The caller must ensure that the `RawTable` outlives the returned [`NonNull<T>`],
        /// otherwise using it may result in [`undefined behavior`].
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        pub fn data_end(&self) -> NonNull<T> {
            self.table.ctrl.cast()
        }
        /// Returns the total amount of memory allocated internally by the hash
        /// table, in bytes.
        ///
        /// The returned number is informational only. It is intended to be
        /// primarily used for memory profiling.
        #[inline]
        pub fn allocation_size(&self) -> usize {
            unsafe { self.table.allocation_size_or_zero(Self::TABLE_LAYOUT) }
        }
        /// Returns the index of a bucket from a `Bucket`.
        #[inline]
        pub unsafe fn bucket_index(&self, bucket: &Bucket<T>) -> usize {
            bucket.to_base_index(self.data_end())
        }
        /// Returns a pointer to an element in the table.
        ///
        /// The caller must ensure that the `RawTable` outlives the returned [`Bucket<T>`],
        /// otherwise using it may result in [`undefined behavior`].
        ///
        /// # Safety
        ///
        /// If `mem::size_of::<T>() != 0`, then the caller of this function must observe the
        /// following safety rules:
        ///
        /// * The table must already be allocated;
        ///
        /// * The `index` must not be greater than the number returned by the [`RawTable::buckets`]
        ///   function, i.e. `(index + 1) <= self.buckets()`.
        ///
        /// It is safe to call this function with index of zero (`index == 0`) on a table that has
        /// not been allocated, but using the returned [`Bucket`] results in [`undefined behavior`].
        ///
        /// If `mem::size_of::<T>() == 0`, then the only requirement is that the `index` must
        /// not be greater than the number returned by the [`RawTable::buckets`] function, i.e.
        /// `(index + 1) <= self.buckets()`.
        ///
        /// [`RawTable::buckets`]: RawTable::buckets
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        pub unsafe fn bucket(&self, index: usize) -> Bucket<T> {
            if true {
                match (&self.table.bucket_mask, &0) {
                    (left_val, right_val) => {
                        if *left_val == *right_val {
                            let kind = ::core::panicking::AssertKind::Ne;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            if true {
                if !(index < self.buckets()) {
                    ::core::panicking::panic("assertion failed: index < self.buckets()")
                }
            }
            Bucket::from_base_index(self.data_end(), index)
        }
        /// Erases an element from the table without dropping it.
        unsafe fn erase_no_drop(&mut self, item: &Bucket<T>) {
            let index = self.bucket_index(item);
            self.table.erase(index);
        }
        /// Erases an element from the table, dropping it in place.
        #[allow(clippy::needless_pass_by_value)]
        pub unsafe fn erase(&mut self, item: Bucket<T>) {
            self.erase_no_drop(&item);
            item.drop();
        }
        /// Removes an element from the table, returning it.
        ///
        /// This also returns an index to the newly free bucket.
        #[allow(clippy::needless_pass_by_value)]
        pub unsafe fn remove(&mut self, item: Bucket<T>) -> (T, usize) {
            self.erase_no_drop(&item);
            (item.read(), self.bucket_index(&item))
        }
        /// Removes an element from the table, returning it.
        ///
        /// This also returns an index to the newly free bucket
        /// and the former `Tag` for that bucket.
        #[allow(clippy::needless_pass_by_value)]
        pub(crate) unsafe fn remove_tagged(
            &mut self,
            item: Bucket<T>,
        ) -> (T, usize, Tag) {
            let index = self.bucket_index(&item);
            let tag = *self.table.ctrl(index);
            self.table.erase(index);
            (item.read(), index, tag)
        }
        /// Finds and removes an element from the table, returning it.
        pub fn remove_entry(
            &mut self,
            hash: u64,
            eq: impl FnMut(&T) -> bool,
        ) -> Option<T> {
            match self.find(hash, eq) {
                Some(bucket) => Some(unsafe { self.remove(bucket).0 }),
                None => None,
            }
        }
        /// Marks all table buckets as empty without dropping their contents.
        pub fn clear_no_drop(&mut self) {
            self.table.clear_no_drop();
        }
        /// Removes all elements from the table without freeing the backing memory.
        pub fn clear(&mut self) {
            if self.is_empty() {
                return;
            }
            let mut self_ = guard(self, |self_| self_.clear_no_drop());
            unsafe {
                self_.table.drop_elements::<T>();
            }
        }
        /// Shrinks the table to fit `max(self.len(), min_size)` elements.
        pub fn shrink_to(&mut self, min_size: usize, hasher: impl Fn(&T) -> u64) {
            let min_size = usize::max(self.table.items, min_size);
            if min_size == 0 {
                let mut old_inner = mem::replace(&mut self.table, RawTableInner::NEW);
                unsafe {
                    old_inner.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
                }
                return;
            }
            let min_buckets = match capacity_to_buckets(min_size, Self::TABLE_LAYOUT) {
                Some(buckets) => buckets,
                None => return,
            };
            if min_buckets < self.buckets() {
                if self.table.items == 0 {
                    let new_inner = RawTableInner::with_capacity(
                        &self.alloc,
                        Self::TABLE_LAYOUT,
                        min_size,
                    );
                    let mut old_inner = mem::replace(&mut self.table, new_inner);
                    unsafe {
                        old_inner
                            .drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
                    }
                } else {
                    unsafe {
                        if self
                            .resize(min_size, hasher, Fallibility::Infallible)
                            .is_err()
                        {
                            hint::unreachable_unchecked()
                        }
                    }
                }
            }
        }
        /// Ensures that at least `additional` items can be inserted into the table
        /// without reallocation.
        pub fn reserve(&mut self, additional: usize, hasher: impl Fn(&T) -> u64) {
            if unlikely(additional > self.table.growth_left) {
                unsafe {
                    if self
                        .reserve_rehash(additional, hasher, Fallibility::Infallible)
                        .is_err()
                    {
                        hint::unreachable_unchecked()
                    }
                }
            }
        }
        /// Tries to ensure that at least `additional` items can be inserted into
        /// the table without reallocation.
        pub fn try_reserve(
            &mut self,
            additional: usize,
            hasher: impl Fn(&T) -> u64,
        ) -> Result<(), TryReserveError> {
            if additional > self.table.growth_left {
                unsafe { self.reserve_rehash(additional, hasher, Fallibility::Fallible) }
            } else {
                Ok(())
            }
        }
        /// Out-of-line slow path for `reserve` and `try_reserve`.
        ///
        /// # Safety
        ///
        /// The [`RawTableInner`] must have properly initialized control bytes,
        /// otherwise calling this function results in [`undefined behavior`]
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[cold]
        #[inline(never)]
        unsafe fn reserve_rehash(
            &mut self,
            additional: usize,
            hasher: impl Fn(&T) -> u64,
            fallibility: Fallibility,
        ) -> Result<(), TryReserveError> {
            unsafe {
                self.table
                    .reserve_rehash_inner(
                        &self.alloc,
                        additional,
                        &|table, index| hasher(table.bucket::<T>(index).as_ref()),
                        fallibility,
                        Self::TABLE_LAYOUT,
                        if T::NEEDS_DROP {
                            Some(|ptr| ptr::drop_in_place(ptr as *mut T))
                        } else {
                            None
                        },
                    )
            }
        }
        /// Allocates a new table of a different size and moves the contents of the
        /// current table into it.
        ///
        /// # Safety
        ///
        /// The [`RawTableInner`] must have properly initialized control bytes,
        /// otherwise calling this function results in [`undefined behavior`]
        ///
        /// The caller of this function must ensure that `capacity >= self.table.items`
        /// otherwise:
        ///
        /// * If `self.table.items != 0`, calling of this function with `capacity`
        ///   equal to 0 (`capacity == 0`) results in [`undefined behavior`].
        ///
        /// * If `self.table.items > capacity_to_buckets(capacity, Self::TABLE_LAYOUT)`
        ///   calling this function are never return (will loop infinitely).
        ///
        /// See [`RawTableInner::find_insert_index`] for more information.
        ///
        /// [`RawTableInner::find_insert_index`]: RawTableInner::find_insert_index
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        unsafe fn resize(
            &mut self,
            capacity: usize,
            hasher: impl Fn(&T) -> u64,
            fallibility: Fallibility,
        ) -> Result<(), TryReserveError> {
            self.table
                .resize_inner(
                    &self.alloc,
                    capacity,
                    &|table, index| hasher(table.bucket::<T>(index).as_ref()),
                    fallibility,
                    Self::TABLE_LAYOUT,
                )
        }
        /// Inserts a new element into the table, and returns its raw bucket.
        ///
        /// This does not check if the given element already exists in the table.
        pub fn insert(
            &mut self,
            hash: u64,
            value: T,
            hasher: impl Fn(&T) -> u64,
        ) -> Bucket<T> {
            unsafe {
                let mut index = self.table.find_insert_index(hash);
                let old_ctrl = *self.table.ctrl(index);
                if unlikely(self.table.growth_left == 0 && old_ctrl.special_is_empty()) {
                    self.reserve(1, hasher);
                    index = self.table.find_insert_index(hash);
                }
                self.insert_at_index(hash, index, value)
            }
        }
        /// Inserts a new element into the table, and returns a mutable reference to it.
        ///
        /// This does not check if the given element already exists in the table.
        pub fn insert_entry(
            &mut self,
            hash: u64,
            value: T,
            hasher: impl Fn(&T) -> u64,
        ) -> &mut T {
            unsafe { self.insert(hash, value, hasher).as_mut() }
        }
        /// Temporary removes a bucket, applying the given function to the removed
        /// element and optionally put back the returned value in the same bucket.
        ///
        /// Returns `true` if the bucket still contains an element
        ///
        /// This does not check if the given bucket is actually occupied.
        pub unsafe fn replace_bucket_with<F>(&mut self, bucket: Bucket<T>, f: F) -> bool
        where
            F: FnOnce(T) -> Option<T>,
        {
            let index = self.bucket_index(&bucket);
            let old_ctrl = *self.table.ctrl(index);
            if true {
                if !self.is_bucket_full(index) {
                    ::core::panicking::panic(
                        "assertion failed: self.is_bucket_full(index)",
                    )
                }
            }
            let old_growth_left = self.table.growth_left;
            let item = self.remove(bucket).0;
            if let Some(new_item) = f(item) {
                self.table.growth_left = old_growth_left;
                self.table.set_ctrl(index, old_ctrl);
                self.table.items += 1;
                self.bucket(index).write(new_item);
                true
            } else {
                false
            }
        }
        /// Searches for an element in the table. If the element is not found,
        /// returns `Err` with the position of a slot where an element with the
        /// same hash could be inserted.
        ///
        /// This function may resize the table if additional space is required for
        /// inserting an element.
        #[inline]
        pub fn find_or_find_insert_index(
            &mut self,
            hash: u64,
            mut eq: impl FnMut(&T) -> bool,
            hasher: impl Fn(&T) -> u64,
        ) -> Result<Bucket<T>, usize> {
            self.reserve(1, hasher);
            unsafe {
                match self
                    .table
                    .find_or_find_insert_index_inner(
                        hash,
                        &mut |index| eq(self.bucket(index).as_ref()),
                    )
                {
                    Ok(index) => Ok(self.bucket(index)),
                    Err(index) => Err(index),
                }
            }
        }
        /// Inserts a new element into the table at the given index with the given hash,
        /// and returns its raw bucket.
        ///
        /// # Safety
        ///
        /// `index` must point to a slot previously returned by
        /// `find_or_find_insert_index`, and no mutation of the table must have
        /// occurred since that call.
        #[inline]
        pub unsafe fn insert_at_index(
            &mut self,
            hash: u64,
            index: usize,
            value: T,
        ) -> Bucket<T> {
            self.insert_tagged_at_index(Tag::full(hash), index, value)
        }
        /// Inserts a new element into the table at the given index with the given tag,
        /// and returns its raw bucket.
        ///
        /// # Safety
        ///
        /// `index` must point to a slot previously returned by
        /// `find_or_find_insert_index`, and no mutation of the table must have
        /// occurred since that call.
        #[inline]
        pub(crate) unsafe fn insert_tagged_at_index(
            &mut self,
            tag: Tag,
            index: usize,
            value: T,
        ) -> Bucket<T> {
            let old_ctrl = *self.table.ctrl(index);
            self.table.record_item_insert_at(index, old_ctrl, tag);
            let bucket = self.bucket(index);
            bucket.write(value);
            bucket
        }
        /// Searches for an element in the table.
        #[inline]
        pub fn find(
            &self,
            hash: u64,
            mut eq: impl FnMut(&T) -> bool,
        ) -> Option<Bucket<T>> {
            unsafe {
                let result = self
                    .table
                    .find_inner(hash, &mut |index| eq(self.bucket(index).as_ref()));
                match result {
                    Some(index) => Some(self.bucket(index)),
                    None => None,
                }
            }
        }
        /// Gets a reference to an element in the table.
        #[inline]
        pub fn get(&self, hash: u64, eq: impl FnMut(&T) -> bool) -> Option<&T> {
            match self.find(hash, eq) {
                Some(bucket) => Some(unsafe { bucket.as_ref() }),
                None => None,
            }
        }
        /// Gets a mutable reference to an element in the table.
        #[inline]
        pub fn get_mut(
            &mut self,
            hash: u64,
            eq: impl FnMut(&T) -> bool,
        ) -> Option<&mut T> {
            match self.find(hash, eq) {
                Some(bucket) => Some(unsafe { bucket.as_mut() }),
                None => None,
            }
        }
        /// Gets a reference to an element in the table at the given bucket index.
        #[inline]
        pub fn get_bucket(&self, index: usize) -> Option<&T> {
            unsafe {
                if index < self.buckets() && self.is_bucket_full(index) {
                    Some(self.bucket(index).as_ref())
                } else {
                    None
                }
            }
        }
        /// Gets a mutable reference to an element in the table at the given bucket index.
        #[inline]
        pub fn get_bucket_mut(&mut self, index: usize) -> Option<&mut T> {
            unsafe {
                if index < self.buckets() && self.is_bucket_full(index) {
                    Some(self.bucket(index).as_mut())
                } else {
                    None
                }
            }
        }
        /// Returns a pointer to an element in the table, but only after verifying that
        /// the index is in-bounds and the bucket is occupied.
        #[inline]
        pub fn checked_bucket(&self, index: usize) -> Option<Bucket<T>> {
            unsafe {
                if index < self.buckets() && self.is_bucket_full(index) {
                    Some(self.bucket(index))
                } else {
                    None
                }
            }
        }
        /// Attempts to get mutable references to `N` entries in the table at once.
        ///
        /// Returns an array of length `N` with the results of each query.
        ///
        /// At most one mutable reference will be returned to any entry. `None` will be returned if any
        /// of the hashes are duplicates. `None` will be returned if the hash is not found.
        ///
        /// The `eq` argument should be a closure such that `eq(i, k)` returns true if `k` is equal to
        /// the `i`th key to be looked up.
        pub fn get_disjoint_mut<const N: usize>(
            &mut self,
            hashes: [u64; N],
            eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<&'_ mut T>; N] {
            unsafe {
                let ptrs = self.get_disjoint_mut_pointers(hashes, eq);
                for (i, cur) in ptrs.iter().enumerate() {
                    if cur.is_some() && ptrs[..i].contains(cur) {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!("duplicate keys found"),
                            );
                        };
                    }
                }
                ptrs.map(|ptr| ptr.map(|mut ptr| ptr.as_mut()))
            }
        }
        pub unsafe fn get_disjoint_unchecked_mut<const N: usize>(
            &mut self,
            hashes: [u64; N],
            eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<&'_ mut T>; N] {
            let ptrs = self.get_disjoint_mut_pointers(hashes, eq);
            ptrs.map(|ptr| ptr.map(|mut ptr| ptr.as_mut()))
        }
        unsafe fn get_disjoint_mut_pointers<const N: usize>(
            &mut self,
            hashes: [u64; N],
            mut eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<NonNull<T>>; N] {
            array::from_fn(|i| {
                self.find(hashes[i], |k| eq(i, k)).map(|cur| cur.as_non_null())
            })
        }
        /// Returns the number of elements the map can hold without reallocating.
        ///
        /// This number is a lower bound; the table might be able to hold
        /// more, but is guaranteed to be able to hold at least this many.
        #[inline]
        pub fn capacity(&self) -> usize {
            self.table.items + self.table.growth_left
        }
        /// Returns the number of elements in the table.
        #[inline]
        pub fn len(&self) -> usize {
            self.table.items
        }
        /// Returns `true` if the table contains no elements.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
        /// Returns the number of buckets in the table.
        #[inline]
        pub fn buckets(&self) -> usize {
            self.table.bucket_mask + 1
        }
        /// Checks whether the bucket at `index` is full.
        ///
        /// # Safety
        ///
        /// The caller must ensure `index` is less than the number of buckets.
        #[inline]
        pub unsafe fn is_bucket_full(&self, index: usize) -> bool {
            self.table.is_bucket_full(index)
        }
        /// Returns an iterator over every element in the table. It is up to
        /// the caller to ensure that the `RawTable` outlives the `RawIter`.
        /// Because we cannot make the `next` method unsafe on the `RawIter`
        /// struct, we have to make the `iter` method unsafe.
        #[inline]
        pub unsafe fn iter(&self) -> RawIter<T> {
            self.table.iter()
        }
        /// Returns an iterator over occupied buckets that could match a given hash.
        ///
        /// `RawTable` only stores 7 bits of the hash value, so this iterator may
        /// return items that have a hash value different than the one provided. You
        /// should always validate the returned values before using them.
        ///
        /// It is up to the caller to ensure that the `RawTable` outlives the
        /// `RawIterHash`. Because we cannot make the `next` method unsafe on the
        /// `RawIterHash` struct, we have to make the `iter_hash` method unsafe.
        pub unsafe fn iter_hash(&self, hash: u64) -> RawIterHash<T> {
            RawIterHash::new(self, hash)
        }
        /// Returns an iterator over occupied bucket indices that could match a given hash.
        ///
        /// `RawTable` only stores 7 bits of the hash value, so this iterator may
        /// return items that have a hash value different than the one provided. You
        /// should always validate the returned values before using them.
        ///
        /// It is up to the caller to ensure that the `RawTable` outlives the
        /// `RawIterHashIndices`. Because we cannot make the `next` method unsafe on the
        /// `RawIterHashIndices` struct, we have to make the `iter_hash_buckets` method unsafe.
        pub(crate) unsafe fn iter_hash_buckets(&self, hash: u64) -> RawIterHashIndices {
            RawIterHashIndices::new(&self.table, hash)
        }
        /// Returns an iterator over full buckets indices in the table.
        ///
        /// See [`RawTableInner::full_buckets_indices`] for safety conditions.
        #[inline(always)]
        pub(crate) unsafe fn full_buckets_indices(&self) -> FullBucketsIndices {
            self.table.full_buckets_indices()
        }
        /// Returns an iterator which removes all elements from the table without
        /// freeing the memory.
        pub fn drain(&mut self) -> RawDrain<'_, T, A> {
            unsafe {
                let iter = self.iter();
                self.drain_iter_from(iter)
            }
        }
        /// Returns an iterator which removes all elements from the table without
        /// freeing the memory.
        ///
        /// Iteration starts at the provided iterator's current location.
        ///
        /// It is up to the caller to ensure that the iterator is valid for this
        /// `RawTable` and covers all items that remain in the table.
        pub unsafe fn drain_iter_from(
            &mut self,
            iter: RawIter<T>,
        ) -> RawDrain<'_, T, A> {
            if true {
                match (&iter.len(), &self.len()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            RawDrain {
                iter,
                table: mem::replace(&mut self.table, RawTableInner::NEW),
                orig_table: NonNull::from(&mut self.table),
                marker: PhantomData,
            }
        }
        /// Returns an iterator which consumes all elements from the table.
        ///
        /// Iteration starts at the provided iterator's current location.
        ///
        /// It is up to the caller to ensure that the iterator is valid for this
        /// `RawTable` and covers all items that remain in the table.
        pub unsafe fn into_iter_from(self, iter: RawIter<T>) -> RawIntoIter<T, A> {
            if true {
                match (&iter.len(), &self.len()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            let allocation = self.into_allocation();
            RawIntoIter {
                iter,
                allocation,
                marker: PhantomData,
            }
        }
        /// Converts the table into a raw allocation. The contents of the table
        /// should be dropped using a `RawIter` before freeing the allocation.
        pub(crate) fn into_allocation(self) -> Option<(NonNull<u8>, Layout, A)> {
            let alloc = if self.table.is_empty_singleton() {
                None
            } else {
                let (layout, ctrl_offset) = match Self::TABLE_LAYOUT
                    .calculate_layout_for(self.table.buckets())
                {
                    Some(lco) => lco,
                    None => unsafe { hint::unreachable_unchecked() }
                };
                Some((
                    unsafe {
                        NonNull::new_unchecked(
                            self.table.ctrl.as_ptr().sub(ctrl_offset).cast(),
                        )
                    },
                    layout,
                    unsafe { ptr::read(&self.alloc) },
                ))
            };
            mem::forget(self);
            alloc
        }
    }
    unsafe impl<T, A: Allocator> Send for RawTable<T, A>
    where
        T: Send,
        A: Send,
    {}
    unsafe impl<T, A: Allocator> Sync for RawTable<T, A>
    where
        T: Sync,
        A: Sync,
    {}
    impl RawTableInner {
        const NEW: Self = RawTableInner::new();
        /// Creates a new empty hash table without allocating any memory.
        ///
        /// In effect this returns a table with exactly 1 bucket. However we can
        /// leave the data pointer dangling since that bucket is never accessed
        /// due to our load factor forcing us to always have at least 1 free bucket.
        #[inline]
        const fn new() -> Self {
            Self {
                ctrl: unsafe {
                    NonNull::new_unchecked(
                        Group::static_empty().as_ptr().cast_mut().cast(),
                    )
                },
                bucket_mask: 0,
                items: 0,
                growth_left: 0,
            }
        }
    }
    /// Find the previous power of 2. If it's already a power of 2, it's unchanged.
    /// Passing zero is undefined behavior.
    pub(crate) fn prev_pow2(z: usize) -> usize {
        let shift = mem::size_of::<usize>() * 8 - 1;
        1 << (shift - (z.leading_zeros() as usize))
    }
    /// Finds the largest number of buckets that can fit in `allocation_size`
    /// provided the given TableLayout.
    ///
    /// This relies on some invariants of `capacity_to_buckets`, so only feed in
    /// an `allocation_size` calculated from `capacity_to_buckets`.
    fn maximum_buckets_in(
        allocation_size: usize,
        table_layout: TableLayout,
        group_width: usize,
    ) -> usize {
        let x = (allocation_size - group_width) / (table_layout.size + 1);
        prev_pow2(x)
    }
    impl RawTableInner {
        /// Allocates a new [`RawTableInner`] with the given number of buckets.
        /// The control bytes and buckets are left uninitialized.
        ///
        /// # Safety
        ///
        /// The caller of this function must ensure that the `buckets` is power of two
        /// and also initialize all control bytes of the length `self.bucket_mask + 1 +
        /// Group::WIDTH` with the [`Tag::EMPTY`] bytes.
        ///
        /// See also [`Allocator`] API for other safety concerns.
        ///
        /// [`Allocator`]: https://doc.rust-lang.org/alloc/alloc/trait.Allocator.html
        unsafe fn new_uninitialized<A>(
            alloc: &A,
            table_layout: TableLayout,
            mut buckets: usize,
            fallibility: Fallibility,
        ) -> Result<Self, TryReserveError>
        where
            A: Allocator,
        {
            if true {
                if !buckets.is_power_of_two() {
                    ::core::panicking::panic(
                        "assertion failed: buckets.is_power_of_two()",
                    )
                }
            }
            let (layout, mut ctrl_offset) = match table_layout
                .calculate_layout_for(buckets)
            {
                Some(lco) => lco,
                None => return Err(fallibility.capacity_overflow()),
            };
            let ptr: NonNull<u8> = match do_alloc(alloc, layout) {
                Ok(block) => {
                    if block.len() != layout.size() {
                        let x = maximum_buckets_in(
                            block.len(),
                            table_layout,
                            Group::WIDTH,
                        );
                        if true {
                            if !(x >= buckets) {
                                ::core::panicking::panic("assertion failed: x >= buckets")
                            }
                        }
                        let (oversized_layout, oversized_ctrl_offset) = match table_layout
                            .calculate_layout_for(x)
                        {
                            Some(lco) => lco,
                            None => unsafe { hint::unreachable_unchecked() }
                        };
                        if true {
                            if !(oversized_layout.size() <= block.len()) {
                                ::core::panicking::panic(
                                    "assertion failed: oversized_layout.size() <= block.len()",
                                )
                            }
                        }
                        if true {
                            if !(oversized_ctrl_offset >= ctrl_offset) {
                                ::core::panicking::panic(
                                    "assertion failed: oversized_ctrl_offset >= ctrl_offset",
                                )
                            }
                        }
                        ctrl_offset = oversized_ctrl_offset;
                        buckets = x;
                    }
                    block.cast()
                }
                Err(_) => return Err(fallibility.alloc_err(layout)),
            };
            let ctrl = NonNull::new_unchecked(ptr.as_ptr().add(ctrl_offset));
            Ok(Self {
                ctrl,
                bucket_mask: buckets - 1,
                items: 0,
                growth_left: bucket_mask_to_capacity(buckets - 1),
            })
        }
        /// Attempts to allocate a new [`RawTableInner`] with at least enough
        /// capacity for inserting the given number of elements without reallocating.
        ///
        /// All the control bytes are initialized with the [`Tag::EMPTY`] bytes.
        #[inline]
        fn fallible_with_capacity<A>(
            alloc: &A,
            table_layout: TableLayout,
            capacity: usize,
            fallibility: Fallibility,
        ) -> Result<Self, TryReserveError>
        where
            A: Allocator,
        {
            if capacity == 0 {
                Ok(Self::NEW)
            } else {
                unsafe {
                    let buckets = capacity_to_buckets(capacity, table_layout)
                        .ok_or_else(|| fallibility.capacity_overflow())?;
                    let mut result = Self::new_uninitialized(
                        alloc,
                        table_layout,
                        buckets,
                        fallibility,
                    )?;
                    result.ctrl_slice().fill_empty();
                    Ok(result)
                }
            }
        }
        /// Allocates a new [`RawTableInner`] with at least enough capacity for inserting
        /// the given number of elements without reallocating.
        ///
        /// Panics if the new capacity exceeds [`isize::MAX`] bytes and [`abort`] the program
        /// in case of allocation error. Use [`fallible_with_capacity`] instead if you want to
        /// handle memory allocation failure.
        ///
        /// All the control bytes are initialized with the [`Tag::EMPTY`] bytes.
        ///
        /// [`fallible_with_capacity`]: RawTableInner::fallible_with_capacity
        /// [`abort`]: https://doc.rust-lang.org/alloc/alloc/fn.handle_alloc_error.html
        fn with_capacity<A>(
            alloc: &A,
            table_layout: TableLayout,
            capacity: usize,
        ) -> Self
        where
            A: Allocator,
        {
            match Self::fallible_with_capacity(
                alloc,
                table_layout,
                capacity,
                Fallibility::Infallible,
            ) {
                Ok(table_inner) => table_inner,
                Err(_) => unsafe { hint::unreachable_unchecked() }
            }
        }
        /// Fixes up an insertion index returned by the [`RawTableInner::find_insert_index_in_group`] method.
        ///
        /// In tables smaller than the group width (`self.buckets() < Group::WIDTH`), trailing control
        /// bytes outside the range of the table are filled with [`Tag::EMPTY`] entries. These will unfortunately
        /// trigger a match of [`RawTableInner::find_insert_index_in_group`] function. This is because
        /// the `Some(bit)` returned by `group.match_empty_or_deleted().lowest_set_bit()` after masking
        /// (`(probe_seq.pos + bit) & self.bucket_mask`) may point to a full bucket that is already occupied.
        /// We detect this situation here and perform a second scan starting at the beginning of the table.
        /// This second scan is guaranteed to find an empty slot (due to the load factor) before hitting the
        /// trailing control bytes (containing [`Tag::EMPTY`] bytes).
        ///
        /// If this function is called correctly, it is guaranteed to return an index of an empty or
        /// deleted bucket in the range `0..self.buckets()` (see `Warning` and `Safety`).
        ///
        /// # Warning
        ///
        /// The table must have at least 1 empty or deleted `bucket`, otherwise if the table is less than
        /// the group width (`self.buckets() < Group::WIDTH`) this function returns an index outside of the
        /// table indices range `0..self.buckets()` (`0..=self.bucket_mask`). Attempt to write data at that
        /// index will cause immediate [`undefined behavior`].
        ///
        /// # Safety
        ///
        /// The safety rules are directly derived from the safety rules for [`RawTableInner::ctrl`] method.
        /// Thus, in order to uphold those safety contracts, as well as for the correct logic of the work
        /// of this crate, the following rules are necessary and sufficient:
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes otherwise calling this
        ///   function results in [`undefined behavior`].
        ///
        /// * This function must only be used on insertion indices found by [`RawTableInner::find_insert_index_in_group`]
        ///   (after the `find_insert_index_in_group` function, but before insertion into the table).
        ///
        /// * The `index` must not be greater than the `self.bucket_mask`, i.e. `(index + 1) <= self.buckets()`
        ///   (this one is provided by the [`RawTableInner::find_insert_index_in_group`] function).
        ///
        /// Calling this function with an index not provided by [`RawTableInner::find_insert_index_in_group`]
        /// may result in [`undefined behavior`] even if the index satisfies the safety rules of the
        /// [`RawTableInner::ctrl`] function (`index < self.bucket_mask + 1 + Group::WIDTH`).
        ///
        /// [`RawTableInner::ctrl`]: RawTableInner::ctrl
        /// [`RawTableInner::find_insert_index_in_group`]: RawTableInner::find_insert_index_in_group
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn fix_insert_index(&self, mut index: usize) -> usize {
            if unlikely(self.is_bucket_full(index)) {
                if true {
                    if !(self.bucket_mask < Group::WIDTH) {
                        ::core::panicking::panic(
                            "assertion failed: self.bucket_mask < Group::WIDTH",
                        )
                    }
                }
                index = Group::load_aligned(self.ctrl(0))
                    .match_empty_or_deleted()
                    .lowest_set_bit()
                    .unwrap_unchecked();
            }
            index
        }
        /// Finds the position to insert something in a group.
        ///
        /// **This may have false positives and must be fixed up with `fix_insert_index`
        /// before it's used.**
        ///
        /// The function is guaranteed to return the index of an empty or deleted [`Bucket`]
        /// in the range `0..self.buckets()` (`0..=self.bucket_mask`).
        #[inline]
        fn find_insert_index_in_group(
            &self,
            group: &Group,
            probe_seq: &ProbeSeq,
        ) -> Option<usize> {
            let bit = group.match_empty_or_deleted().lowest_set_bit();
            if likely(bit.is_some()) {
                Some((probe_seq.pos + bit.unwrap()) & self.bucket_mask)
            } else {
                None
            }
        }
        /// Searches for an element in the table, or a potential slot where that element could
        /// be inserted (an empty or deleted [`Bucket`] index).
        ///
        /// This uses dynamic dispatch to reduce the amount of code generated, but that is
        /// eliminated by LLVM optimizations.
        ///
        /// This function does not make any changes to the `data` part of the table, or any
        /// changes to the `items` or `growth_left` field of the table.
        ///
        /// The table must have at least 1 empty or deleted `bucket`, otherwise, if the
        /// `eq: &mut dyn FnMut(usize) -> bool` function does not return `true`, this function
        /// will never return (will go into an infinite loop) for tables larger than the group
        /// width, or return an index outside of the table indices range if the table is less
        /// than the group width.
        ///
        /// This function is guaranteed to provide the `eq: &mut dyn FnMut(usize) -> bool`
        /// function with only `FULL` buckets' indices and return the `index` of the found
        /// element (as `Ok(index)`). If the element is not found and there is at least 1
        /// empty or deleted [`Bucket`] in the table, the function is guaranteed to return
        /// an index in the range `0..self.buckets()`, but in any case, if this function
        /// returns `Err`, it will contain an index in the range `0..=self.buckets()`.
        ///
        /// # Safety
        ///
        /// The [`RawTableInner`] must have properly initialized control bytes otherwise calling
        /// this function results in [`undefined behavior`].
        ///
        /// Attempt to write data at the index returned by this function when the table is less than
        /// the group width and if there was not at least one empty or deleted bucket in the table
        /// will cause immediate [`undefined behavior`]. This is because in this case the function
        /// will return `self.bucket_mask + 1` as an index due to the trailing [`Tag::EMPTY`] control
        /// bytes outside the table range.
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn find_or_find_insert_index_inner(
            &self,
            hash: u64,
            eq: &mut dyn FnMut(usize) -> bool,
        ) -> Result<usize, usize> {
            let mut insert_index = None;
            let tag_hash = Tag::full(hash);
            let mut probe_seq = self.probe_seq(hash);
            loop {
                let group = unsafe { Group::load(self.ctrl(probe_seq.pos)) };
                for bit in group.match_tag(tag_hash) {
                    let index = (probe_seq.pos + bit) & self.bucket_mask;
                    if likely(eq(index)) {
                        return Ok(index);
                    }
                }
                if likely(insert_index.is_none()) {
                    insert_index = self.find_insert_index_in_group(&group, &probe_seq);
                }
                if let Some(insert_index) = insert_index {
                    if likely(group.match_empty().any_bit_set()) {
                        unsafe {
                            return Err(self.fix_insert_index(insert_index));
                        }
                    }
                }
                probe_seq.move_next(self.bucket_mask);
            }
        }
        /// Searches for an empty or deleted bucket which is suitable for inserting a new
        /// element and sets the hash for that slot. Returns an index of that slot and the
        /// old control byte stored in the found index.
        ///
        /// This function does not check if the given element exists in the table. Also,
        /// this function does not check if there is enough space in the table to insert
        /// a new element. The caller of the function must make sure that the table has at
        /// least 1 empty or deleted `bucket`, otherwise this function will never return
        /// (will go into an infinite loop) for tables larger than the group width, or
        /// return an index outside of the table indices range if the table is less than
        /// the group width.
        ///
        /// If there is at least 1 empty or deleted `bucket` in the table, the function is
        /// guaranteed to return an `index` in the range `0..self.buckets()`, but in any case,
        /// if this function returns an `index` it will be in the range `0..=self.buckets()`.
        ///
        /// This function does not make any changes to the `data` parts of the table,
        /// or any changes to the `items` or `growth_left` field of the table.
        ///
        /// # Safety
        ///
        /// The safety rules are directly derived from the safety rules for the
        /// [`RawTableInner::set_ctrl_hash`] and [`RawTableInner::find_insert_index`] methods.
        /// Thus, in order to uphold the safety contracts for that methods, as well as for
        /// the correct logic of the work of this crate, you must observe the following rules
        /// when calling this function:
        ///
        /// * The [`RawTableInner`] has already been allocated and has properly initialized
        ///   control bytes otherwise calling this function results in [`undefined behavior`].
        ///
        /// * The caller of this function must ensure that the "data" parts of the table
        ///   will have an entry in the returned index (matching the given hash) right
        ///   after calling this function.
        ///
        /// Attempt to write data at the `index` returned by this function when the table is
        /// less than the group width and if there was not at least one empty or deleted bucket in
        /// the table will cause immediate [`undefined behavior`]. This is because in this case the
        /// function will return `self.bucket_mask + 1` as an index due to the trailing [`Tag::EMPTY`]
        /// control bytes outside the table range.
        ///
        /// The caller must independently increase the `items` field of the table, and also,
        /// if the old control byte was [`Tag::EMPTY`], then decrease the table's `growth_left`
        /// field, and do not change it if the old control byte was [`Tag::DELETED`].
        ///
        /// See also [`Bucket::as_ptr`] method, for more information about of properly removing
        /// or saving `element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        /// [`RawTableInner::ctrl`]: RawTableInner::ctrl
        /// [`RawTableInner::set_ctrl_hash`]: RawTableInner::set_ctrl_hash
        /// [`RawTableInner::find_insert_index`]: RawTableInner::find_insert_index
        #[inline]
        unsafe fn prepare_insert_index(&mut self, hash: u64) -> (usize, Tag) {
            let index: usize = self.find_insert_index(hash);
            let old_ctrl = *self.ctrl(index);
            self.set_ctrl_hash(index, hash);
            (index, old_ctrl)
        }
        /// Searches for an empty or deleted bucket which is suitable for inserting
        /// a new element, returning the `index` for the new [`Bucket`].
        ///
        /// This function does not make any changes to the `data` part of the table, or any
        /// changes to the `items` or `growth_left` field of the table.
        ///
        /// The table must have at least 1 empty or deleted `bucket`, otherwise this function
        /// will never return (will go into an infinite loop) for tables larger than the group
        /// width, or return an index outside of the table indices range if the table is less
        /// than the group width.
        ///
        /// If there is at least 1 empty or deleted `bucket` in the table, the function is
        /// guaranteed to return an index in the range `0..self.buckets()`, but in any case,
        /// it will contain an index in the range `0..=self.buckets()`.
        ///
        /// # Safety
        ///
        /// The [`RawTableInner`] must have properly initialized control bytes otherwise calling
        /// this function results in [`undefined behavior`].
        ///
        /// Attempt to write data at the index returned by this function when the table is
        /// less than the group width and if there was not at least one empty or deleted bucket in
        /// the table will cause immediate [`undefined behavior`]. This is because in this case the
        /// function will return `self.bucket_mask + 1` as an index due to the trailing [`Tag::EMPTY`]
        /// control bytes outside the table range.
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn find_insert_index(&self, hash: u64) -> usize {
            let mut probe_seq = self.probe_seq(hash);
            loop {
                let group = unsafe { Group::load(self.ctrl(probe_seq.pos)) };
                let index = self.find_insert_index_in_group(&group, &probe_seq);
                if likely(index.is_some()) {
                    unsafe {
                        return self.fix_insert_index(index.unwrap_unchecked());
                    }
                }
                probe_seq.move_next(self.bucket_mask);
            }
        }
        /// Searches for an element in a table, returning the `index` of the found element.
        /// This uses dynamic dispatch to reduce the amount of code generated, but it is
        /// eliminated by LLVM optimizations.
        ///
        /// This function does not make any changes to the `data` part of the table, or any
        /// changes to the `items` or `growth_left` field of the table.
        ///
        /// The table must have at least 1 empty `bucket`, otherwise, if the
        /// `eq: &mut dyn FnMut(usize) -> bool` function does not return `true`,
        /// this function will also never return (will go into an infinite loop).
        ///
        /// This function is guaranteed to provide the `eq: &mut dyn FnMut(usize) -> bool`
        /// function with only `FULL` buckets' indices and return the `index` of the found
        /// element as `Some(index)`, so the index will always be in the range
        /// `0..self.buckets()`.
        ///
        /// # Safety
        ///
        /// The [`RawTableInner`] must have properly initialized control bytes otherwise calling
        /// this function results in [`undefined behavior`].
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline(always)]
        unsafe fn find_inner(
            &self,
            hash: u64,
            eq: &mut dyn FnMut(usize) -> bool,
        ) -> Option<usize> {
            let tag_hash = Tag::full(hash);
            let mut probe_seq = self.probe_seq(hash);
            loop {
                let group = unsafe { Group::load(self.ctrl(probe_seq.pos)) };
                for bit in group.match_tag(tag_hash) {
                    let index = (probe_seq.pos + bit) & self.bucket_mask;
                    if likely(eq(index)) {
                        return Some(index);
                    }
                }
                if likely(group.match_empty().any_bit_set()) {
                    return None;
                }
                probe_seq.move_next(self.bucket_mask);
            }
        }
        /// Prepares for rehashing data in place (that is, without allocating new memory).
        /// Converts all full index `control bytes` to `Tag::DELETED` and all `Tag::DELETED` control
        /// bytes to `Tag::EMPTY`, i.e. performs the following conversion:
        ///
        /// - `Tag::EMPTY` control bytes   -> `Tag::EMPTY`;
        /// - `Tag::DELETED` control bytes -> `Tag::EMPTY`;
        /// - `FULL` control bytes    -> `Tag::DELETED`.
        ///
        /// This function does not make any changes to the `data` parts of the table,
        /// or any changes to the `items` or `growth_left` field of the table.
        ///
        /// # Safety
        ///
        /// You must observe the following safety rules when calling this function:
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * The caller of this function must convert the `Tag::DELETED` bytes back to `FULL`
        ///   bytes when re-inserting them into their ideal position (which was impossible
        ///   to do during the first insert due to tombstones). If the caller does not do
        ///   this, then calling this function may result in a memory leak.
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes otherwise
        ///   calling this function results in [`undefined behavior`].
        ///
        /// Calling this function on a table that has not been allocated results in
        /// [`undefined behavior`].
        ///
        /// See also [`Bucket::as_ptr`] method, for more information about of properly removing
        /// or saving `data element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[allow(clippy::mut_mut)]
        #[inline]
        unsafe fn prepare_rehash_in_place(&mut self) {
            for i in (0..self.buckets()).step_by(Group::WIDTH) {
                let group = Group::load_aligned(self.ctrl(i));
                let group = group.convert_special_to_empty_and_full_to_deleted();
                group.store_aligned(self.ctrl(i));
            }
            if unlikely(self.buckets() < Group::WIDTH) {
                self.ctrl(0).copy_to(self.ctrl(Group::WIDTH), self.buckets());
            } else {
                self.ctrl(0).copy_to(self.ctrl(self.buckets()), Group::WIDTH);
            }
        }
        /// Returns an iterator over every element in the table.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result
        /// is [`undefined behavior`]:
        ///
        /// * The caller has to ensure that the `RawTableInner` outlives the
        ///   `RawIter`. Because we cannot make the `next` method unsafe on
        ///   the `RawIter` struct, we have to make the `iter` method unsafe.
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes.
        ///
        /// The type `T` must be the actual type of the elements stored in the table,
        /// otherwise using the returned [`RawIter`] results in [`undefined behavior`].
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn iter<T>(&self) -> RawIter<T> {
            let data = Bucket::from_base_index(self.data_end(), 0);
            RawIter {
                iter: RawIterRange::new(self.ctrl.as_ptr(), data, self.buckets()),
                items: self.items,
            }
        }
        /// Executes the destructors (if any) of the values stored in the table.
        ///
        /// # Note
        ///
        /// This function does not erase the control bytes of the table and does
        /// not make any changes to the `items` or `growth_left` fields of the
        /// table. If necessary, the caller of this function must manually set
        /// up these table fields, for example using the [`clear_no_drop`] function.
        ///
        /// Be careful during calling this function, because drop function of
        /// the elements can panic, and this can leave table in an inconsistent
        /// state.
        ///
        /// # Safety
        ///
        /// The type `T` must be the actual type of the elements stored in the table,
        /// otherwise calling this function may result in [`undefined behavior`].
        ///
        /// If `T` is a type that should be dropped and **the table is not empty**,
        /// calling this function more than once results in [`undefined behavior`].
        ///
        /// If `T` is not [`Copy`], attempting to use values stored in the table after
        /// calling this function may result in [`undefined behavior`].
        ///
        /// It is safe to call this function on a table that has not been allocated,
        /// on a table with uninitialized control bytes, and on a table with no actual
        /// data but with `Full` control bytes if `self.items == 0`.
        ///
        /// See also [`Bucket::drop`] / [`Bucket::as_ptr`] methods, for more information
        /// about of properly removing or saving `element` from / into the [`RawTable`] /
        /// [`RawTableInner`].
        ///
        /// [`Bucket::drop`]: Bucket::drop
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`clear_no_drop`]: RawTableInner::clear_no_drop
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        unsafe fn drop_elements<T>(&mut self) {
            if T::NEEDS_DROP && self.items != 0 {
                for item in self.iter::<T>() {
                    item.drop();
                }
            }
        }
        /// Executes the destructors (if any) of the values stored in the table and than
        /// deallocates the table.
        ///
        /// # Note
        ///
        /// Calling this function automatically makes invalid (dangling) all instances of
        /// buckets ([`Bucket`]) and makes invalid (dangling) the `ctrl` field of the table.
        ///
        /// This function does not make any changes to the `bucket_mask`, `items` or `growth_left`
        /// fields of the table. If necessary, the caller of this function must manually set
        /// up these table fields.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is [`undefined behavior`]:
        ///
        /// * Calling this function more than once;
        ///
        /// * The type `T` must be the actual type of the elements stored in the table.
        ///
        /// * The `alloc` must be the same [`Allocator`] as the `Allocator` that was used
        ///   to allocate this table.
        ///
        /// * The `table_layout` must be the same [`TableLayout`] as the `TableLayout` that
        ///   was used to allocate this table.
        ///
        /// The caller of this function should pay attention to the possibility of the
        /// elements' drop function panicking, because this:
        ///
        ///    * May leave the table in an inconsistent state;
        ///
        ///    * Memory is never deallocated, so a memory leak may occur.
        ///
        /// Attempt to use the `ctrl` field of the table (dereference) after calling this
        /// function results in [`undefined behavior`].
        ///
        /// It is safe to call this function on a table that has not been allocated,
        /// on a table with uninitialized control bytes, and on a table with no actual
        /// data but with `Full` control bytes if `self.items == 0`.
        ///
        /// See also [`RawTableInner::drop_elements`] or [`RawTableInner::free_buckets`]
        /// for more  information.
        ///
        /// [`RawTableInner::drop_elements`]: RawTableInner::drop_elements
        /// [`RawTableInner::free_buckets`]: RawTableInner::free_buckets
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        unsafe fn drop_inner_table<T, A: Allocator>(
            &mut self,
            alloc: &A,
            table_layout: TableLayout,
        ) {
            if !self.is_empty_singleton() {
                unsafe {
                    self.drop_elements::<T>();
                    self.free_buckets(alloc, table_layout);
                }
            }
        }
        /// Returns a pointer to an element in the table (convenience for
        /// `Bucket::from_base_index(self.data_end::<T>(), index)`).
        ///
        /// The caller must ensure that the `RawTableInner` outlives the returned [`Bucket<T>`],
        /// otherwise using it may result in [`undefined behavior`].
        ///
        /// # Safety
        ///
        /// If `mem::size_of::<T>() != 0`, then the safety rules are directly derived from the
        /// safety rules of the [`Bucket::from_base_index`] function. Therefore, when calling
        /// this function, the following safety rules must be observed:
        ///
        /// * The table must already be allocated;
        ///
        /// * The `index` must not be greater than the number returned by the [`RawTableInner::buckets`]
        ///   function, i.e. `(index + 1) <= self.buckets()`.
        ///
        /// * The type `T` must be the actual type of the elements stored in the table, otherwise
        ///   using the returned [`Bucket`] may result in [`undefined behavior`].
        ///
        /// It is safe to call this function with index of zero (`index == 0`) on a table that has
        /// not been allocated, but using the returned [`Bucket`] results in [`undefined behavior`].
        ///
        /// If `mem::size_of::<T>() == 0`, then the only requirement is that the `index` must
        /// not be greater than the number returned by the [`RawTable::buckets`] function, i.e.
        /// `(index + 1) <= self.buckets()`.
        ///
        /// ```none
        /// If mem::size_of::<T>() != 0 then return a pointer to the `element` in the `data part` of the table
        /// (we start counting from "0", so that in the expression T[n], the "n" index actually one less than
        /// the "buckets" number of our `RawTableInner`, i.e. "n = RawTableInner::buckets() - 1"):
        ///
        ///           `table.bucket(3).as_ptr()` returns a pointer that points here in the `data`
        ///           part of the `RawTableInner`, i.e. to the start of T3 (see [`Bucket::as_ptr`])
        ///                  |
        ///                  |               `base = table.data_end::<T>()` points here
        ///                  |               (to the start of CT0 or to the end of T0)
        ///                  v                 v
        /// [Pad], T_n, ..., |T3|, T2, T1, T0, |CT0, CT1, CT2, CT3, ..., CT_n, CTa_0, CTa_1, ..., CTa_m
        ///                     ^                                              \__________  __________/
        ///        `table.bucket(3)` returns a pointer that points                        \/
        ///         here in the `data` part of the `RawTableInner`             additional control bytes
        ///         (to the end of T3)                                          `m = Group::WIDTH - 1`
        ///
        /// where: T0...T_n  - our stored data;
        ///        CT0...CT_n - control bytes or metadata for `data`;
        ///        CTa_0...CTa_m - additional control bytes (so that the search with loading `Group` bytes from
        ///                        the heap works properly, even if the result of `h1(hash) & self.bucket_mask`
        ///                        is equal to `self.bucket_mask`). See also `RawTableInner::set_ctrl` function.
        ///
        /// P.S. `h1(hash) & self.bucket_mask` is the same as `hash as usize % self.buckets()` because the number
        /// of buckets is a power of two, and `self.bucket_mask = self.buckets() - 1`.
        /// ```
        ///
        /// [`Bucket::from_base_index`]: Bucket::from_base_index
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn bucket<T>(&self, index: usize) -> Bucket<T> {
            if true {
                match (&self.bucket_mask, &0) {
                    (left_val, right_val) => {
                        if *left_val == *right_val {
                            let kind = ::core::panicking::AssertKind::Ne;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            if true {
                if !(index < self.buckets()) {
                    ::core::panicking::panic("assertion failed: index < self.buckets()")
                }
            }
            Bucket::from_base_index(self.data_end(), index)
        }
        /// Returns a raw `*mut u8` pointer to the start of the `data` element in the table
        /// (convenience for `self.data_end::<u8>().as_ptr().sub((index + 1) * size_of)`).
        ///
        /// The caller must ensure that the `RawTableInner` outlives the returned `*mut u8`,
        /// otherwise using it may result in [`undefined behavior`].
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is [`undefined behavior`]:
        ///
        /// * The table must already be allocated;
        ///
        /// * The `index` must not be greater than the number returned by the [`RawTableInner::buckets`]
        ///   function, i.e. `(index + 1) <= self.buckets()`;
        ///
        /// * The `size_of` must be equal to the size of the elements stored in the table;
        ///
        /// ```none
        /// If mem::size_of::<T>() != 0 then return a pointer to the `element` in the `data part` of the table
        /// (we start counting from "0", so that in the expression T[n], the "n" index actually one less than
        /// the "buckets" number of our `RawTableInner`, i.e. "n = RawTableInner::buckets() - 1"):
        ///
        ///           `table.bucket_ptr(3, mem::size_of::<T>())` returns a pointer that points here in the
        ///           `data` part of the `RawTableInner`, i.e. to the start of T3
        ///                  |
        ///                  |               `base = table.data_end::<u8>()` points here
        ///                  |               (to the start of CT0 or to the end of T0)
        ///                  v                 v
        /// [Pad], T_n, ..., |T3|, T2, T1, T0, |CT0, CT1, CT2, CT3, ..., CT_n, CTa_0, CTa_1, ..., CTa_m
        ///                                                                    \__________  __________/
        ///                                                                               \/
        ///                                                                    additional control bytes
        ///                                                                     `m = Group::WIDTH - 1`
        ///
        /// where: T0...T_n  - our stored data;
        ///        CT0...CT_n - control bytes or metadata for `data`;
        ///        CTa_0...CTa_m - additional control bytes (so that the search with loading `Group` bytes from
        ///                        the heap works properly, even if the result of `h1(hash) & self.bucket_mask`
        ///                        is equal to `self.bucket_mask`). See also `RawTableInner::set_ctrl` function.
        ///
        /// P.S. `h1(hash) & self.bucket_mask` is the same as `hash as usize % self.buckets()` because the number
        /// of buckets is a power of two, and `self.bucket_mask = self.buckets() - 1`.
        /// ```
        ///
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn bucket_ptr(&self, index: usize, size_of: usize) -> *mut u8 {
            if true {
                match (&self.bucket_mask, &0) {
                    (left_val, right_val) => {
                        if *left_val == *right_val {
                            let kind = ::core::panicking::AssertKind::Ne;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            if true {
                if !(index < self.buckets()) {
                    ::core::panicking::panic("assertion failed: index < self.buckets()")
                }
            }
            let base: *mut u8 = self.data_end().as_ptr();
            base.sub((index + 1) * size_of)
        }
        /// Returns pointer to one past last `data` element in the table as viewed from
        /// the start point of the allocation (convenience for `self.ctrl.cast()`).
        ///
        /// This function actually returns a pointer to the end of the `data element` at
        /// index "0" (zero).
        ///
        /// The caller must ensure that the `RawTableInner` outlives the returned [`NonNull<T>`],
        /// otherwise using it may result in [`undefined behavior`].
        ///
        /// # Note
        ///
        /// The type `T` must be the actual type of the elements stored in the table, otherwise
        /// using the returned [`NonNull<T>`] may result in [`undefined behavior`].
        ///
        /// ```none
        ///                        `table.data_end::<T>()` returns pointer that points here
        ///                        (to the end of `T0`)
        ///                          ∨
        /// [Pad], T_n, ..., T1, T0, |CT0, CT1, ..., CT_n|, CTa_0, CTa_1, ..., CTa_m
        ///                           \________  ________/
        ///                                    \/
        ///       `n = buckets - 1`, i.e. `RawTableInner::buckets() - 1`
        ///
        /// where: T0...T_n  - our stored data;
        ///        CT0...CT_n - control bytes or metadata for `data`.
        ///        CTa_0...CTa_m - additional control bytes, where `m = Group::WIDTH - 1` (so that the search
        ///                        with loading `Group` bytes from the heap works properly, even if the result
        ///                        of `h1(hash) & self.bucket_mask` is equal to `self.bucket_mask`). See also
        ///                        `RawTableInner::set_ctrl` function.
        ///
        /// P.S. `h1(hash) & self.bucket_mask` is the same as `hash as usize % self.buckets()` because the number
        /// of buckets is a power of two, and `self.bucket_mask = self.buckets() - 1`.
        /// ```
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        fn data_end<T>(&self) -> NonNull<T> {
            self.ctrl.cast()
        }
        /// Returns an iterator-like object for a probe sequence on the table.
        ///
        /// This iterator never terminates, but is guaranteed to visit each bucket
        /// group exactly once. The loop using `probe_seq` must terminate upon
        /// reaching a group containing an empty bucket.
        #[inline]
        fn probe_seq(&self, hash: u64) -> ProbeSeq {
            ProbeSeq {
                pos: h1(hash) & self.bucket_mask,
                stride: 0,
            }
        }
        #[inline]
        unsafe fn record_item_insert_at(
            &mut self,
            index: usize,
            old_ctrl: Tag,
            new_ctrl: Tag,
        ) {
            self.growth_left -= usize::from(old_ctrl.special_is_empty());
            self.set_ctrl(index, new_ctrl);
            self.items += 1;
        }
        #[inline]
        fn is_in_same_group(&self, i: usize, new_i: usize, hash: u64) -> bool {
            let probe_seq_pos = self.probe_seq(hash).pos;
            let probe_index = |pos: usize| {
                (pos.wrapping_sub(probe_seq_pos) & self.bucket_mask) / Group::WIDTH
            };
            probe_index(i) == probe_index(new_i)
        }
        /// Sets a control byte to the hash, and possibly also the replicated control byte at
        /// the end of the array.
        ///
        /// This function does not make any changes to the `data` parts of the table,
        /// or any changes to the `items` or `growth_left` field of the table.
        ///
        /// # Safety
        ///
        /// The safety rules are directly derived from the safety rules for [`RawTableInner::set_ctrl`]
        /// method. Thus, in order to uphold the safety contracts for the method, you must observe the
        /// following rules when calling this function:
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * The `index` must not be greater than the `RawTableInner.bucket_mask`, i.e.
        ///   `index <= RawTableInner.bucket_mask` or, in other words, `(index + 1)` must
        ///   be no greater than the number returned by the function [`RawTableInner::buckets`].
        ///
        /// Calling this function on a table that has not been allocated results in [`undefined behavior`].
        ///
        /// See also [`Bucket::as_ptr`] method, for more information about of properly removing
        /// or saving `data element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`RawTableInner::set_ctrl`]: RawTableInner::set_ctrl
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn set_ctrl_hash(&mut self, index: usize, hash: u64) {
            self.set_ctrl(index, Tag::full(hash));
        }
        /// Replaces the hash in the control byte at the given index with the provided one,
        /// and possibly also replicates the new control byte at the end of the array of control
        /// bytes, returning the old control byte.
        ///
        /// This function does not make any changes to the `data` parts of the table,
        /// or any changes to the `items` or `growth_left` field of the table.
        ///
        /// # Safety
        ///
        /// The safety rules are directly derived from the safety rules for [`RawTableInner::set_ctrl_hash`]
        /// and [`RawTableInner::ctrl`] methods. Thus, in order to uphold the safety contracts for both
        /// methods, you must observe the following rules when calling this function:
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * The `index` must not be greater than the `RawTableInner.bucket_mask`, i.e.
        ///   `index <= RawTableInner.bucket_mask` or, in other words, `(index + 1)` must
        ///   be no greater than the number returned by the function [`RawTableInner::buckets`].
        ///
        /// Calling this function on a table that has not been allocated results in [`undefined behavior`].
        ///
        /// See also [`Bucket::as_ptr`] method, for more information about of properly removing
        /// or saving `data element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`RawTableInner::set_ctrl_hash`]: RawTableInner::set_ctrl_hash
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn replace_ctrl_hash(&mut self, index: usize, hash: u64) -> Tag {
            let prev_ctrl = *self.ctrl(index);
            self.set_ctrl_hash(index, hash);
            prev_ctrl
        }
        /// Sets a control byte, and possibly also the replicated control byte at
        /// the end of the array.
        ///
        /// This function does not make any changes to the `data` parts of the table,
        /// or any changes to the `items` or `growth_left` field of the table.
        ///
        /// # Safety
        ///
        /// You must observe the following safety rules when calling this function:
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * The `index` must not be greater than the `RawTableInner.bucket_mask`, i.e.
        ///   `index <= RawTableInner.bucket_mask` or, in other words, `(index + 1)` must
        ///   be no greater than the number returned by the function [`RawTableInner::buckets`].
        ///
        /// Calling this function on a table that has not been allocated results in [`undefined behavior`].
        ///
        /// See also [`Bucket::as_ptr`] method, for more information about of properly removing
        /// or saving `data element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn set_ctrl(&mut self, index: usize, ctrl: Tag) {
            let index2 = ((index.wrapping_sub(Group::WIDTH)) & self.bucket_mask)
                + Group::WIDTH;
            *self.ctrl(index) = ctrl;
            *self.ctrl(index2) = ctrl;
        }
        /// Returns a pointer to a control byte.
        ///
        /// # Safety
        ///
        /// For the allocated [`RawTableInner`], the result is [`Undefined Behavior`],
        /// if the `index` is greater than the `self.bucket_mask + 1 + Group::WIDTH`.
        /// In that case, calling this function with `index == self.bucket_mask + 1 + Group::WIDTH`
        /// will return a pointer to the end of the allocated table and it is useless on its own.
        ///
        /// Calling this function with `index >= self.bucket_mask + 1 + Group::WIDTH` on a
        /// table that has not been allocated results in [`Undefined Behavior`].
        ///
        /// So to satisfy both requirements you should always follow the rule that
        /// `index < self.bucket_mask + 1 + Group::WIDTH`
        ///
        /// Calling this function on [`RawTableInner`] that are not already allocated is safe
        /// for read-only purpose.
        ///
        /// See also [`Bucket::as_ptr()`] method, for more information about of properly removing
        /// or saving `data element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`Bucket::as_ptr()`]: Bucket::as_ptr()
        /// [`Undefined Behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn ctrl(&self, index: usize) -> *mut Tag {
            if true {
                if !(index < self.num_ctrl_bytes()) {
                    ::core::panicking::panic(
                        "assertion failed: index < self.num_ctrl_bytes()",
                    )
                }
            }
            self.ctrl.as_ptr().add(index).cast()
        }
        /// Gets the slice of all control bytes.
        fn ctrl_slice(&mut self) -> &mut [Tag] {
            unsafe {
                slice::from_raw_parts_mut(
                    self.ctrl.as_ptr().cast(),
                    self.num_ctrl_bytes(),
                )
            }
        }
        #[inline]
        fn buckets(&self) -> usize {
            self.bucket_mask + 1
        }
        /// Checks whether the bucket at `index` is full.
        ///
        /// # Safety
        ///
        /// The caller must ensure `index` is less than the number of buckets.
        #[inline]
        unsafe fn is_bucket_full(&self, index: usize) -> bool {
            if true {
                if !(index < self.buckets()) {
                    ::core::panicking::panic("assertion failed: index < self.buckets()")
                }
            }
            (*self.ctrl(index)).is_full()
        }
        #[inline]
        fn num_ctrl_bytes(&self) -> usize {
            self.bucket_mask + 1 + Group::WIDTH
        }
        #[inline]
        fn is_empty_singleton(&self) -> bool {
            self.bucket_mask == 0
        }
        /// Attempts to allocate a new hash table with at least enough capacity
        /// for inserting the given number of elements without reallocating,
        /// and return it inside `ScopeGuard` to protect against panic in the hash
        /// function.
        ///
        /// # Note
        ///
        /// It is recommended (but not required):
        ///
        /// * That the new table's `capacity` be greater than or equal to `self.items`.
        ///
        /// * The `alloc` is the same [`Allocator`] as the `Allocator` used
        ///   to allocate this table.
        ///
        /// * The `table_layout` is the same [`TableLayout`] as the `TableLayout` used
        ///   to allocate this table.
        ///
        /// If `table_layout` does not match the `TableLayout` that was used to allocate
        /// this table, then using `mem::swap` with the `self` and the new table returned
        /// by this function results in [`undefined behavior`].
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[allow(clippy::mut_mut)]
        #[inline]
        fn prepare_resize<'a, A>(
            &self,
            alloc: &'a A,
            table_layout: TableLayout,
            capacity: usize,
            fallibility: Fallibility,
        ) -> Result<
            crate::scopeguard::ScopeGuard<Self, impl FnMut(&mut Self) + 'a>,
            TryReserveError,
        >
        where
            A: Allocator,
        {
            if true {
                if !(self.items <= capacity) {
                    ::core::panicking::panic("assertion failed: self.items <= capacity")
                }
            }
            let new_table = RawTableInner::fallible_with_capacity(
                alloc,
                table_layout,
                capacity,
                fallibility,
            )?;
            Ok(
                guard(
                    new_table,
                    move |self_| {
                        if !self_.is_empty_singleton() {
                            unsafe { self_.free_buckets(alloc, table_layout) };
                        }
                    },
                ),
            )
        }
        /// Reserves or rehashes to make room for `additional` more elements.
        ///
        /// This uses dynamic dispatch to reduce the amount of
        /// code generated, but it is eliminated by LLVM optimizations when inlined.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is
        /// [`undefined behavior`]:
        ///
        /// * The `alloc` must be the same [`Allocator`] as the `Allocator` used
        ///   to allocate this table.
        ///
        /// * The `layout` must be the same [`TableLayout`] as the `TableLayout`
        ///   used to allocate this table.
        ///
        /// * The `drop` function (`fn(*mut u8)`) must be the actual drop function of
        ///   the elements stored in the table.
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes.
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[allow(clippy::inline_always)]
        #[inline(always)]
        unsafe fn reserve_rehash_inner<A>(
            &mut self,
            alloc: &A,
            additional: usize,
            hasher: &dyn Fn(&mut Self, usize) -> u64,
            fallibility: Fallibility,
            layout: TableLayout,
            drop: Option<unsafe fn(*mut u8)>,
        ) -> Result<(), TryReserveError>
        where
            A: Allocator,
        {
            let new_items = match self.items.checked_add(additional) {
                Some(new_items) => new_items,
                None => return Err(fallibility.capacity_overflow()),
            };
            let full_capacity = bucket_mask_to_capacity(self.bucket_mask);
            if new_items <= full_capacity / 2 {
                self.rehash_in_place(hasher, layout.size, drop);
                Ok(())
            } else {
                self.resize_inner(
                    alloc,
                    usize::max(new_items, full_capacity + 1),
                    hasher,
                    fallibility,
                    layout,
                )
            }
        }
        /// Returns an iterator over full buckets indices in the table.
        ///
        /// # Safety
        ///
        /// Behavior is undefined if any of the following conditions are violated:
        ///
        /// * The caller has to ensure that the `RawTableInner` outlives the
        ///   `FullBucketsIndices`. Because we cannot make the `next` method
        ///   unsafe on the `FullBucketsIndices` struct, we have to make the
        ///   `full_buckets_indices` method unsafe.
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes.
        #[inline(always)]
        unsafe fn full_buckets_indices(&self) -> FullBucketsIndices {
            let ctrl = NonNull::new_unchecked(self.ctrl(0).cast::<u8>());
            FullBucketsIndices {
                current_group: Group::load_aligned(ctrl.as_ptr().cast())
                    .match_full()
                    .into_iter(),
                group_first_index: 0,
                ctrl,
                items: self.items,
            }
        }
        /// Allocates a new table of a different size and moves the contents of the
        /// current table into it.
        ///
        /// This uses dynamic dispatch to reduce the amount of
        /// code generated, but it is eliminated by LLVM optimizations when inlined.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is
        /// [`undefined behavior`]:
        ///
        /// * The `alloc` must be the same [`Allocator`] as the `Allocator` used
        ///   to allocate this table;
        ///
        /// * The `layout` must be the same [`TableLayout`] as the `TableLayout`
        ///   used to allocate this table;
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes.
        ///
        /// The caller of this function must ensure that `capacity >= self.items`
        /// otherwise:
        ///
        /// * If `self.items != 0`, calling of this function with `capacity == 0`
        ///   results in [`undefined behavior`].
        ///
        /// * If `capacity_to_buckets(capacity) < Group::WIDTH` and
        ///   `self.items > capacity_to_buckets(capacity)` calling this function
        ///   results in [`undefined behavior`].
        ///
        /// * If `capacity_to_buckets(capacity) >= Group::WIDTH` and
        ///   `self.items > capacity_to_buckets(capacity)` calling this function
        ///   are never return (will go into an infinite loop).
        ///
        /// Note: It is recommended (but not required) that the new table's `capacity`
        /// be greater than or equal to `self.items`. In case if `capacity <= self.items`
        /// this function can never return. See [`RawTableInner::find_insert_index`] for
        /// more information.
        ///
        /// [`RawTableInner::find_insert_index`]: RawTableInner::find_insert_index
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[allow(clippy::inline_always)]
        #[inline(always)]
        unsafe fn resize_inner<A>(
            &mut self,
            alloc: &A,
            capacity: usize,
            hasher: &dyn Fn(&mut Self, usize) -> u64,
            fallibility: Fallibility,
            layout: TableLayout,
        ) -> Result<(), TryReserveError>
        where
            A: Allocator,
        {
            let mut new_table = self
                .prepare_resize(alloc, layout, capacity, fallibility)?;
            for full_byte_index in self.full_buckets_indices() {
                let hash = hasher(self, full_byte_index);
                let (new_index, _) = new_table.prepare_insert_index(hash);
                ptr::copy_nonoverlapping(
                    self.bucket_ptr(full_byte_index, layout.size),
                    new_table.bucket_ptr(new_index, layout.size),
                    layout.size,
                );
            }
            new_table.growth_left -= self.items;
            new_table.items = self.items;
            mem::swap(self, &mut new_table);
            Ok(())
        }
        /// Rehashes the contents of the table in place (i.e. without changing the
        /// allocation).
        ///
        /// If `hasher` panics then some the table's contents may be lost.
        ///
        /// This uses dynamic dispatch to reduce the amount of
        /// code generated, but it is eliminated by LLVM optimizations when inlined.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is [`undefined behavior`]:
        ///
        /// * The `size_of` must be equal to the size of the elements stored in the table;
        ///
        /// * The `drop` function (`fn(*mut u8)`) must be the actual drop function of
        ///   the elements stored in the table.
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * The [`RawTableInner`] must have properly initialized control bytes.
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[allow(clippy::inline_always)]
        #[inline]
        unsafe fn rehash_in_place(
            &mut self,
            hasher: &dyn Fn(&mut Self, usize) -> u64,
            size_of: usize,
            drop: Option<unsafe fn(*mut u8)>,
        ) {
            self.prepare_rehash_in_place();
            let mut guard = guard(
                self,
                move |self_| {
                    if let Some(drop) = drop {
                        for i in 0..self_.buckets() {
                            if *self_.ctrl(i) == Tag::DELETED {
                                self_.set_ctrl(i, Tag::EMPTY);
                                drop(self_.bucket_ptr(i, size_of));
                                self_.items -= 1;
                            }
                        }
                    }
                    self_.growth_left = bucket_mask_to_capacity(self_.bucket_mask)
                        - self_.items;
                },
            );
            'outer: for i in 0..guard.buckets() {
                if *guard.ctrl(i) != Tag::DELETED {
                    continue;
                }
                let i_p = guard.bucket_ptr(i, size_of);
                'inner: loop {
                    let hash = hasher(*guard, i);
                    let new_i = guard.find_insert_index(hash);
                    if likely(guard.is_in_same_group(i, new_i, hash)) {
                        guard.set_ctrl_hash(i, hash);
                        continue 'outer;
                    }
                    let new_i_p = guard.bucket_ptr(new_i, size_of);
                    let prev_ctrl = guard.replace_ctrl_hash(new_i, hash);
                    if prev_ctrl == Tag::EMPTY {
                        guard.set_ctrl(i, Tag::EMPTY);
                        ptr::copy_nonoverlapping(i_p, new_i_p, size_of);
                        continue 'outer;
                    } else {
                        if true {
                            match (&prev_ctrl, &Tag::DELETED) {
                                (left_val, right_val) => {
                                    if !(*left_val == *right_val) {
                                        let kind = ::core::panicking::AssertKind::Eq;
                                        ::core::panicking::assert_failed(
                                            kind,
                                            &*left_val,
                                            &*right_val,
                                            ::core::option::Option::None,
                                        );
                                    }
                                }
                            };
                        }
                        ptr::swap_nonoverlapping(i_p, new_i_p, size_of);
                        continue 'inner;
                    }
                }
            }
            guard.growth_left = bucket_mask_to_capacity(guard.bucket_mask) - guard.items;
            mem::forget(guard);
        }
        /// Deallocates the table without dropping any entries.
        ///
        /// # Note
        ///
        /// This function must be called only after [`drop_elements`](RawTableInner::drop_elements),
        /// else it can lead to leaking of memory. Also calling this function automatically
        /// makes invalid (dangling) all instances of buckets ([`Bucket`]) and makes invalid
        /// (dangling) the `ctrl` field of the table.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is [`Undefined Behavior`]:
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * The `alloc` must be the same [`Allocator`] as the `Allocator` that was used
        ///   to allocate this table.
        ///
        /// * The `table_layout` must be the same [`TableLayout`] as the `TableLayout` that was used
        ///   to allocate this table.
        ///
        /// See also [`GlobalAlloc::dealloc`] or [`Allocator::deallocate`] for more  information.
        ///
        /// [`Undefined Behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        /// [`GlobalAlloc::dealloc`]: https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#tymethod.dealloc
        /// [`Allocator::deallocate`]: https://doc.rust-lang.org/alloc/alloc/trait.Allocator.html#tymethod.deallocate
        #[inline]
        unsafe fn free_buckets<A>(&mut self, alloc: &A, table_layout: TableLayout)
        where
            A: Allocator,
        {
            let (ptr, layout) = self.allocation_info(table_layout);
            alloc.deallocate(ptr, layout);
        }
        /// Returns a pointer to the allocated memory and the layout that was used to
        /// allocate the table.
        ///
        /// # Safety
        ///
        /// Caller of this function must observe the following safety rules:
        ///
        /// * The [`RawTableInner`] has already been allocated, otherwise
        ///   calling this function results in [`undefined behavior`]
        ///
        /// * The `table_layout` must be the same [`TableLayout`] as the `TableLayout`
        ///   that was used to allocate this table. Failure to comply with this condition
        ///   may result in [`undefined behavior`].
        ///
        /// See also [`GlobalAlloc::dealloc`] or [`Allocator::deallocate`] for more  information.
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        /// [`GlobalAlloc::dealloc`]: https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#tymethod.dealloc
        /// [`Allocator::deallocate`]: https://doc.rust-lang.org/alloc/alloc/trait.Allocator.html#tymethod.deallocate
        #[inline]
        unsafe fn allocation_info(
            &self,
            table_layout: TableLayout,
        ) -> (NonNull<u8>, Layout) {
            if true {
                if !!self.is_empty_singleton() {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "this function can only be called on non-empty tables",
                            ),
                        );
                    }
                }
            }
            let (layout, ctrl_offset) = match table_layout
                .calculate_layout_for(self.buckets())
            {
                Some(lco) => lco,
                None => unsafe { hint::unreachable_unchecked() }
            };
            (
                unsafe { NonNull::new_unchecked(self.ctrl.as_ptr().sub(ctrl_offset)) },
                layout,
            )
        }
        /// Returns the total amount of memory allocated internally by the hash
        /// table, in bytes.
        ///
        /// The returned number is informational only. It is intended to be
        /// primarily used for memory profiling.
        ///
        /// # Safety
        ///
        /// The `table_layout` must be the same [`TableLayout`] as the `TableLayout`
        /// that was used to allocate this table. Failure to comply with this condition
        /// may result in [`undefined behavior`].
        ///
        ///
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn allocation_size_or_zero(&self, table_layout: TableLayout) -> usize {
            if self.is_empty_singleton() {
                0
            } else {
                unsafe { self.allocation_info(table_layout).1.size() }
            }
        }
        /// Marks all table buckets as empty without dropping their contents.
        #[inline]
        fn clear_no_drop(&mut self) {
            if !self.is_empty_singleton() {
                self.ctrl_slice().fill_empty();
            }
            self.items = 0;
            self.growth_left = bucket_mask_to_capacity(self.bucket_mask);
        }
        /// Erases the [`Bucket`]'s control byte at the given index so that it does not
        /// triggered as full, decreases the `items` of the table and, if it can be done,
        /// increases `self.growth_left`.
        ///
        /// This function does not actually erase / drop the [`Bucket`] itself, i.e. it
        /// does not make any changes to the `data` parts of the table. The caller of this
        /// function must take care to properly drop the `data`, otherwise calling this
        /// function may result in a memory leak.
        ///
        /// # Safety
        ///
        /// You must observe the following safety rules when calling this function:
        ///
        /// * The [`RawTableInner`] has already been allocated;
        ///
        /// * It must be the full control byte at the given position;
        ///
        /// * The `index` must not be greater than the `RawTableInner.bucket_mask`, i.e.
        ///   `index <= RawTableInner.bucket_mask` or, in other words, `(index + 1)` must
        ///   be no greater than the number returned by the function [`RawTableInner::buckets`].
        ///
        /// Calling this function on a table that has not been allocated results in [`undefined behavior`].
        ///
        /// Calling this function on a table with no elements is unspecified, but calling subsequent
        /// functions is likely to result in [`undefined behavior`] due to overflow subtraction
        /// (`self.items -= 1 cause overflow when self.items == 0`).
        ///
        /// See also [`Bucket::as_ptr`] method, for more information about of properly removing
        /// or saving `data element` from / into the [`RawTable`] / [`RawTableInner`].
        ///
        /// [`RawTableInner::buckets`]: RawTableInner::buckets
        /// [`Bucket::as_ptr`]: Bucket::as_ptr
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline]
        unsafe fn erase(&mut self, index: usize) {
            if true {
                if !self.is_bucket_full(index) {
                    ::core::panicking::panic(
                        "assertion failed: self.is_bucket_full(index)",
                    )
                }
            }
            let index_before = index.wrapping_sub(Group::WIDTH) & self.bucket_mask;
            let empty_before = Group::load(self.ctrl(index_before)).match_empty();
            let empty_after = Group::load(self.ctrl(index)).match_empty();
            let ctrl = if empty_before.leading_zeros() + empty_after.trailing_zeros()
                >= Group::WIDTH
            {
                Tag::DELETED
            } else {
                self.growth_left += 1;
                Tag::EMPTY
            };
            self.set_ctrl(index, ctrl);
            self.items -= 1;
        }
    }
    impl<T: Clone, A: Allocator + Clone> Clone for RawTable<T, A> {
        fn clone(&self) -> Self {
            if self.table.is_empty_singleton() {
                Self::new_in(self.alloc.clone())
            } else {
                unsafe {
                    let mut new_table = match Self::new_uninitialized(
                        self.alloc.clone(),
                        self.table.buckets(),
                        Fallibility::Infallible,
                    ) {
                        Ok(table) => table,
                        Err(_) => hint::unreachable_unchecked(),
                    };
                    new_table.clone_from_spec(self);
                    new_table
                }
            }
        }
        fn clone_from(&mut self, source: &Self) {
            if source.table.is_empty_singleton() {
                let mut old_inner = mem::replace(&mut self.table, RawTableInner::NEW);
                unsafe {
                    old_inner.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
                }
            } else {
                unsafe {
                    let mut self_ = guard(
                        self,
                        |self_| {
                            self_.clear_no_drop();
                        },
                    );
                    self_.table.drop_elements::<T>();
                    if self_.buckets() != source.buckets() {
                        let new_inner = match RawTableInner::new_uninitialized(
                            &self_.alloc,
                            Self::TABLE_LAYOUT,
                            source.buckets(),
                            Fallibility::Infallible,
                        ) {
                            Ok(table) => table,
                            Err(_) => hint::unreachable_unchecked(),
                        };
                        let mut old_inner = mem::replace(&mut self_.table, new_inner);
                        if !old_inner.is_empty_singleton() {
                            old_inner.free_buckets(&self_.alloc, Self::TABLE_LAYOUT);
                        }
                    }
                    self_.clone_from_spec(source);
                    ScopeGuard::into_inner(self_);
                }
            }
        }
    }
    /// Specialization of `clone_from` for `Copy` types
    trait RawTableClone {
        unsafe fn clone_from_spec(&mut self, source: &Self);
    }
    impl<T: Clone, A: Allocator + Clone> RawTableClone for RawTable<T, A> {
        unsafe fn clone_from_spec(&mut self, source: &Self) {
            self.clone_from_impl(source);
        }
    }
    impl<T: Clone, A: Allocator + Clone> RawTable<T, A> {
        /// Common code for `clone` and `clone_from`. Assumes:
        /// - `self.buckets() == source.buckets()`.
        /// - Any existing elements have been dropped.
        /// - The control bytes are not initialized yet.
        unsafe fn clone_from_impl(&mut self, source: &Self) {
            source
                .table
                .ctrl(0)
                .copy_to_nonoverlapping(self.table.ctrl(0), self.table.num_ctrl_bytes());
            let mut guard = guard(
                (0, &mut *self),
                |(index, self_)| {
                    if T::NEEDS_DROP {
                        for i in 0..*index {
                            if self_.is_bucket_full(i) {
                                self_.bucket(i).drop();
                            }
                        }
                    }
                },
            );
            for from in source.iter() {
                let index = source.bucket_index(&from);
                let to = guard.1.bucket(index);
                to.write(from.as_ref().clone());
                guard.0 = index + 1;
            }
            mem::forget(guard);
            self.table.items = source.table.items;
            self.table.growth_left = source.table.growth_left;
        }
    }
    impl<T, A: Allocator + Default> Default for RawTable<T, A> {
        #[inline]
        fn default() -> Self {
            Self::new_in(Default::default())
        }
    }
    impl<T, A: Allocator> Drop for RawTable<T, A> {
        fn drop(&mut self) {
            unsafe {
                self.table.drop_inner_table::<T, _>(&self.alloc, Self::TABLE_LAYOUT);
            }
        }
    }
    impl<T, A: Allocator> IntoIterator for RawTable<T, A> {
        type Item = T;
        type IntoIter = RawIntoIter<T, A>;
        fn into_iter(self) -> RawIntoIter<T, A> {
            unsafe {
                let iter = self.iter();
                self.into_iter_from(iter)
            }
        }
    }
    /// Iterator over a sub-range of a table. Unlike `RawIter` this iterator does
    /// not track an item count.
    pub(crate) struct RawIterRange<T> {
        current_group: BitMaskIter,
        data: Bucket<T>,
        next_ctrl: *const u8,
        end: *const u8,
    }
    impl<T> RawIterRange<T> {
        /// Returns a `RawIterRange` covering a subset of a table.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is
        /// [`undefined behavior`]:
        ///
        /// * `ctrl` must be [valid] for reads, i.e. table outlives the `RawIterRange`;
        ///
        /// * `ctrl` must be properly aligned to the group size (`Group::WIDTH`);
        ///
        /// * `ctrl` must point to the array of properly initialized control bytes;
        ///
        /// * `data` must be the [`Bucket`] at the `ctrl` index in the table;
        ///
        /// * the value of `len` must be less than or equal to the number of table buckets,
        ///   and the returned value of `ctrl.as_ptr().add(len).offset_from(ctrl.as_ptr())`
        ///   must be positive.
        ///
        /// * The `ctrl.add(len)` pointer must be either in bounds or one
        ///   byte past the end of the same [allocated table].
        ///
        /// * The `len` must be a power of two.
        ///
        /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
        /// [`undefined behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        unsafe fn new(ctrl: *const u8, data: Bucket<T>, len: usize) -> Self {
            if true {
                match (&len, &0) {
                    (left_val, right_val) => {
                        if *left_val == *right_val {
                            let kind = ::core::panicking::AssertKind::Ne;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            if true {
                match (&(ctrl as usize % Group::WIDTH), &0) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
            let end = ctrl.add(len);
            let current_group = Group::load_aligned(ctrl.cast()).match_full();
            let next_ctrl = ctrl.add(Group::WIDTH);
            Self {
                current_group: current_group.into_iter(),
                data,
                next_ctrl,
                end,
            }
        }
        /// # Safety
        /// If `DO_CHECK_PTR_RANGE` is false, caller must ensure that we never try to iterate
        /// after yielding all elements.
        unsafe fn next_impl<const DO_CHECK_PTR_RANGE: bool>(
            &mut self,
        ) -> Option<Bucket<T>> {
            loop {
                if let Some(index) = self.current_group.next() {
                    return Some(self.data.next_n(index));
                }
                if DO_CHECK_PTR_RANGE && self.next_ctrl >= self.end {
                    return None;
                }
                self.current_group = Group::load_aligned(self.next_ctrl.cast())
                    .match_full()
                    .into_iter();
                self.data = self.data.next_n(Group::WIDTH);
                self.next_ctrl = self.next_ctrl.add(Group::WIDTH);
            }
        }
        /// Folds every element into an accumulator by applying an operation,
        /// returning the final result.
        ///
        /// `fold_impl()` takes three arguments: the number of items remaining in
        /// the iterator, an initial value, and a closure with two arguments: an
        /// 'accumulator', and an element. The closure returns the value that the
        /// accumulator should have for the next iteration.
        ///
        /// The initial value is the value the accumulator will have on the first call.
        ///
        /// After applying this closure to every element of the iterator, `fold_impl()`
        /// returns the accumulator.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is
        /// [`Undefined Behavior`]:
        ///
        /// * The [`RawTableInner`] / [`RawTable`] must be alive and not moved,
        ///   i.e. table outlives the `RawIterRange`;
        ///
        /// * The provided `n` value must match the actual number of items
        ///   in the table.
        ///
        /// [`Undefined Behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[allow(clippy::while_let_on_iterator)]
        unsafe fn fold_impl<F, B>(mut self, mut n: usize, mut acc: B, mut f: F) -> B
        where
            F: FnMut(B, Bucket<T>) -> B,
        {
            loop {
                while let Some(index) = self.current_group.next() {
                    if true {
                        if !(n != 0) {
                            ::core::panicking::panic("assertion failed: n != 0")
                        }
                    }
                    let bucket = self.data.next_n(index);
                    acc = f(acc, bucket);
                    n -= 1;
                }
                if n == 0 {
                    return acc;
                }
                self.current_group = Group::load_aligned(self.next_ctrl.cast())
                    .match_full()
                    .into_iter();
                self.data = self.data.next_n(Group::WIDTH);
                self.next_ctrl = self.next_ctrl.add(Group::WIDTH);
            }
        }
    }
    unsafe impl<T> Send for RawIterRange<T> {}
    unsafe impl<T> Sync for RawIterRange<T> {}
    impl<T> Clone for RawIterRange<T> {
        fn clone(&self) -> Self {
            Self {
                data: self.data.clone(),
                next_ctrl: self.next_ctrl,
                current_group: self.current_group.clone(),
                end: self.end,
            }
        }
    }
    impl<T> Iterator for RawIterRange<T> {
        type Item = Bucket<T>;
        fn next(&mut self) -> Option<Bucket<T>> {
            unsafe { self.next_impl::<true>() }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let remaining_buckets = if self.end > self.next_ctrl {
                unsafe { offset_from(self.end, self.next_ctrl) }
            } else {
                0
            };
            (0, Some(Group::WIDTH + remaining_buckets))
        }
    }
    impl<T> FusedIterator for RawIterRange<T> {}
    /// Iterator which returns a raw pointer to every full bucket in the table.
    ///
    /// For maximum flexibility this iterator is not bound by a lifetime, but you
    /// must observe several rules when using it:
    /// - You must not free the hash table while iterating (including via growing/shrinking).
    /// - It is fine to erase a bucket that has been yielded by the iterator.
    /// - Erasing a bucket that has not yet been yielded by the iterator may still
    ///   result in the iterator yielding that bucket (unless `reflect_remove` is called).
    /// - It is unspecified whether an element inserted after the iterator was
    ///   created will be yielded by that iterator (unless `reflect_insert` is called).
    /// - The order in which the iterator yields bucket is unspecified and may
    ///   change in the future.
    pub struct RawIter<T> {
        pub(crate) iter: RawIterRange<T>,
        items: usize,
    }
    impl<T> RawIter<T> {
        unsafe fn drop_elements(&mut self) {
            if T::NEEDS_DROP && self.items != 0 {
                for item in self {
                    item.drop();
                }
            }
        }
    }
    impl<T> Clone for RawIter<T> {
        fn clone(&self) -> Self {
            Self {
                iter: self.iter.clone(),
                items: self.items,
            }
        }
    }
    impl<T> Default for RawIter<T> {
        fn default() -> Self {
            unsafe { RawTableInner::NEW.iter() }
        }
    }
    impl<T> Iterator for RawIter<T> {
        type Item = Bucket<T>;
        fn next(&mut self) -> Option<Bucket<T>> {
            if self.items == 0 {
                return None;
            }
            let nxt = unsafe { self.iter.next_impl::<false>() };
            if true {
                if !nxt.is_some() {
                    ::core::panicking::panic("assertion failed: nxt.is_some()")
                }
            }
            self.items -= 1;
            nxt
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.items, Some(self.items))
        }
        #[inline]
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            unsafe { self.iter.fold_impl(self.items, init, f) }
        }
    }
    impl<T> ExactSizeIterator for RawIter<T> {}
    impl<T> FusedIterator for RawIter<T> {}
    /// Iterator which returns an index of every full bucket in the table.
    ///
    /// For maximum flexibility this iterator is not bound by a lifetime, but you
    /// must observe several rules when using it:
    /// - You must not free the hash table while iterating (including via growing/shrinking).
    /// - It is fine to erase a bucket that has been yielded by the iterator.
    /// - Erasing a bucket that has not yet been yielded by the iterator may still
    ///   result in the iterator yielding index of that bucket.
    /// - It is unspecified whether an element inserted after the iterator was
    ///   created will be yielded by that iterator.
    /// - The order in which the iterator yields indices of the buckets is unspecified
    ///   and may change in the future.
    pub(crate) struct FullBucketsIndices {
        current_group: BitMaskIter,
        group_first_index: usize,
        ctrl: NonNull<u8>,
        items: usize,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for FullBucketsIndices {
        #[inline]
        fn clone(&self) -> FullBucketsIndices {
            FullBucketsIndices {
                current_group: ::core::clone::Clone::clone(&self.current_group),
                group_first_index: ::core::clone::Clone::clone(&self.group_first_index),
                ctrl: ::core::clone::Clone::clone(&self.ctrl),
                items: ::core::clone::Clone::clone(&self.items),
            }
        }
    }
    impl Default for FullBucketsIndices {
        fn default() -> Self {
            unsafe { RawTableInner::NEW.full_buckets_indices() }
        }
    }
    impl FullBucketsIndices {
        /// Advances the iterator and returns the next value.
        ///
        /// # Safety
        ///
        /// If any of the following conditions are violated, the result is
        /// [`Undefined Behavior`]:
        ///
        /// * The [`RawTableInner`] / [`RawTable`] must be alive and not moved,
        ///   i.e. table outlives the `FullBucketsIndices`;
        ///
        /// * It never tries to iterate after getting all elements.
        ///
        /// [`Undefined Behavior`]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        #[inline(always)]
        unsafe fn next_impl(&mut self) -> Option<usize> {
            loop {
                if let Some(index) = self.current_group.next() {
                    return Some(self.group_first_index + index);
                }
                self.ctrl = NonNull::new_unchecked(self.ctrl.as_ptr().add(Group::WIDTH));
                self.current_group = Group::load_aligned(self.ctrl.as_ptr().cast())
                    .match_full()
                    .into_iter();
                self.group_first_index += Group::WIDTH;
            }
        }
    }
    impl Iterator for FullBucketsIndices {
        type Item = usize;
        /// Advances the iterator and returns the next value. It is up to
        /// the caller to ensure that the `RawTable` outlives the `FullBucketsIndices`,
        /// because we cannot make the `next` method unsafe.
        #[inline(always)]
        fn next(&mut self) -> Option<usize> {
            if self.items == 0 {
                return None;
            }
            let nxt = unsafe { self.next_impl() };
            if true {
                if !nxt.is_some() {
                    ::core::panicking::panic("assertion failed: nxt.is_some()")
                }
            }
            self.items -= 1;
            nxt
        }
        #[inline(always)]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.items, Some(self.items))
        }
    }
    impl ExactSizeIterator for FullBucketsIndices {}
    impl FusedIterator for FullBucketsIndices {}
    /// Iterator which consumes a table and returns elements.
    pub struct RawIntoIter<T, A: Allocator = Global> {
        iter: RawIter<T>,
        allocation: Option<(NonNull<u8>, Layout, A)>,
        marker: PhantomData<T>,
    }
    impl<T, A: Allocator> RawIntoIter<T, A> {
        pub fn iter(&self) -> RawIter<T> {
            self.iter.clone()
        }
    }
    unsafe impl<T, A: Allocator> Send for RawIntoIter<T, A>
    where
        T: Send,
        A: Send,
    {}
    unsafe impl<T, A: Allocator> Sync for RawIntoIter<T, A>
    where
        T: Sync,
        A: Sync,
    {}
    impl<T, A: Allocator> Drop for RawIntoIter<T, A> {
        fn drop(&mut self) {
            unsafe {
                self.iter.drop_elements();
                if let Some((ptr, layout, ref alloc)) = self.allocation {
                    alloc.deallocate(ptr, layout);
                }
            }
        }
    }
    impl<T, A: Allocator> Default for RawIntoIter<T, A> {
        fn default() -> Self {
            Self {
                iter: Default::default(),
                allocation: None,
                marker: PhantomData,
            }
        }
    }
    impl<T, A: Allocator> Iterator for RawIntoIter<T, A> {
        type Item = T;
        fn next(&mut self) -> Option<T> {
            unsafe { Some(self.iter.next()?.read()) }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<T, A: Allocator> ExactSizeIterator for RawIntoIter<T, A> {}
    impl<T, A: Allocator> FusedIterator for RawIntoIter<T, A> {}
    /// Iterator which consumes elements without freeing the table storage.
    pub struct RawDrain<'a, T, A: Allocator = Global> {
        iter: RawIter<T>,
        table: RawTableInner,
        orig_table: NonNull<RawTableInner>,
        marker: PhantomData<&'a RawTable<T, A>>,
    }
    impl<T, A: Allocator> RawDrain<'_, T, A> {
        pub fn iter(&self) -> RawIter<T> {
            self.iter.clone()
        }
    }
    unsafe impl<T, A: Allocator> Send for RawDrain<'_, T, A>
    where
        T: Send,
        A: Send,
    {}
    unsafe impl<T, A: Allocator> Sync for RawDrain<'_, T, A>
    where
        T: Sync,
        A: Sync,
    {}
    impl<T, A: Allocator> Drop for RawDrain<'_, T, A> {
        fn drop(&mut self) {
            unsafe {
                self.iter.drop_elements();
                self.table.clear_no_drop();
                self.orig_table.as_ptr().copy_from_nonoverlapping(&self.table, 1);
            }
        }
    }
    impl<T, A: Allocator> Iterator for RawDrain<'_, T, A> {
        type Item = T;
        fn next(&mut self) -> Option<T> {
            unsafe {
                let item = self.iter.next()?;
                Some(item.read())
            }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<T, A: Allocator> ExactSizeIterator for RawDrain<'_, T, A> {}
    impl<T, A: Allocator> FusedIterator for RawDrain<'_, T, A> {}
    /// Iterator over occupied buckets that could match a given hash.
    ///
    /// `RawTable` only stores 7 bits of the hash value, so this iterator may return
    /// items that have a hash value different than the one provided. You should
    /// always validate the returned values before using them.
    ///
    /// For maximum flexibility this iterator is not bound by a lifetime, but you
    /// must observe several rules when using it:
    /// - You must not free the hash table while iterating (including via growing/shrinking).
    /// - It is fine to erase a bucket that has been yielded by the iterator.
    /// - Erasing a bucket that has not yet been yielded by the iterator may still
    ///   result in the iterator yielding that bucket.
    /// - It is unspecified whether an element inserted after the iterator was
    ///   created will be yielded by that iterator.
    /// - The order in which the iterator yields buckets is unspecified and may
    ///   change in the future.
    pub struct RawIterHash<T> {
        inner: RawIterHashIndices,
        _marker: PhantomData<T>,
    }
    pub(crate) struct RawIterHashIndices {
        bucket_mask: usize,
        ctrl: NonNull<u8>,
        tag_hash: Tag,
        probe_seq: ProbeSeq,
        group: Group,
        bitmask: BitMaskIter,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for RawIterHashIndices {
        #[inline]
        fn clone(&self) -> RawIterHashIndices {
            RawIterHashIndices {
                bucket_mask: ::core::clone::Clone::clone(&self.bucket_mask),
                ctrl: ::core::clone::Clone::clone(&self.ctrl),
                tag_hash: ::core::clone::Clone::clone(&self.tag_hash),
                probe_seq: ::core::clone::Clone::clone(&self.probe_seq),
                group: ::core::clone::Clone::clone(&self.group),
                bitmask: ::core::clone::Clone::clone(&self.bitmask),
            }
        }
    }
    impl<T> RawIterHash<T> {
        unsafe fn new<A: Allocator>(table: &RawTable<T, A>, hash: u64) -> Self {
            RawIterHash {
                inner: RawIterHashIndices::new(&table.table, hash),
                _marker: PhantomData,
            }
        }
    }
    impl<T> Clone for RawIterHash<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                _marker: PhantomData,
            }
        }
    }
    impl<T> Default for RawIterHash<T> {
        fn default() -> Self {
            Self {
                inner: RawIterHashIndices::default(),
                _marker: PhantomData,
            }
        }
    }
    impl Default for RawIterHashIndices {
        fn default() -> Self {
            unsafe { RawIterHashIndices::new(&RawTableInner::NEW, 0) }
        }
    }
    impl RawIterHashIndices {
        unsafe fn new(table: &RawTableInner, hash: u64) -> Self {
            let tag_hash = Tag::full(hash);
            let probe_seq = table.probe_seq(hash);
            let group = Group::load(table.ctrl(probe_seq.pos));
            let bitmask = group.match_tag(tag_hash).into_iter();
            RawIterHashIndices {
                bucket_mask: table.bucket_mask,
                ctrl: table.ctrl,
                tag_hash,
                probe_seq,
                group,
                bitmask,
            }
        }
    }
    impl<T> Iterator for RawIterHash<T> {
        type Item = Bucket<T>;
        fn next(&mut self) -> Option<Bucket<T>> {
            unsafe {
                match self.inner.next() {
                    Some(index) => {
                        if true {
                            if !(index <= self.inner.bucket_mask) {
                                ::core::panicking::panic(
                                    "assertion failed: index <= self.inner.bucket_mask",
                                )
                            }
                        }
                        let bucket = Bucket::from_base_index(
                            self.inner.ctrl.cast(),
                            index,
                        );
                        Some(bucket)
                    }
                    None => None,
                }
            }
        }
    }
    impl Iterator for RawIterHashIndices {
        type Item = usize;
        fn next(&mut self) -> Option<Self::Item> {
            unsafe {
                loop {
                    if let Some(bit) = self.bitmask.next() {
                        let index = (self.probe_seq.pos + bit) & self.bucket_mask;
                        return Some(index);
                    }
                    if likely(self.group.match_empty().any_bit_set()) {
                        return None;
                    }
                    self.probe_seq.move_next(self.bucket_mask);
                    let index = self.probe_seq.pos;
                    if true {
                        if !(index < self.bucket_mask + 1 + Group::WIDTH) {
                            ::core::panicking::panic(
                                "assertion failed: index < self.bucket_mask + 1 + Group::WIDTH",
                            )
                        }
                    }
                    let group_ctrl = self.ctrl.as_ptr().add(index).cast();
                    self.group = Group::load(group_ctrl);
                    self.bitmask = self.group.match_tag(self.tag_hash).into_iter();
                }
            }
        }
    }
    pub(crate) struct RawExtractIf<'a, T, A: Allocator> {
        pub iter: RawIter<T>,
        pub table: &'a mut RawTable<T, A>,
    }
    impl<T, A: Allocator> RawExtractIf<'_, T, A> {
        pub(crate) fn next<F>(&mut self, mut f: F) -> Option<T>
        where
            F: FnMut(&mut T) -> bool,
        {
            unsafe {
                for item in &mut self.iter {
                    if f(item.as_mut()) {
                        return Some(self.table.remove(item).0);
                    }
                }
            }
            None
        }
    }
}
mod util {
    #[inline(always)]
    #[cold]
    fn cold_path() {}
    #[inline(always)]
    pub(crate) fn likely(b: bool) -> bool {
        if b {
            true
        } else {
            cold_path();
            false
        }
    }
    #[inline(always)]
    pub(crate) fn unlikely(b: bool) -> bool {
        if b {
            cold_path();
            true
        } else {
            false
        }
    }
    #[inline(always)]
    #[allow(clippy::useless_transmute)]
    pub(crate) fn invalid_mut<T>(addr: usize) -> *mut T {
        unsafe { core::mem::transmute(addr) }
    }
}
mod external_trait_impls {}
mod map {
    use crate::raw::{
        Allocator, Bucket, Global, RawDrain, RawExtractIf, RawIntoIter, RawIter, RawTable,
    };
    use crate::{DefaultHashBuilder, Equivalent, TryReserveError};
    use core::borrow::Borrow;
    use core::fmt::{self, Debug};
    use core::hash::{BuildHasher, Hash};
    use core::iter::FusedIterator;
    use core::marker::PhantomData;
    use core::mem;
    use core::ops::Index;
    /// A hash map implemented with quadratic probing and SIMD lookup.
    ///
    /// The default hashing algorithm is currently [`foldhash`], though this is
    /// subject to change at any point in the future. This hash function is very
    /// fast for all types of keys, but this algorithm will typically *not* protect
    /// against attacks such as HashDoS.
    ///
    /// The hashing algorithm can be replaced on a per-`HashMap` basis using the
    /// [`default`], [`with_hasher`], and [`with_capacity_and_hasher`] methods. Many
    /// alternative algorithms are available on crates.io, such as the [`fnv`] crate.
    ///
    /// It is required that the keys implement the [`Eq`] and [`Hash`] traits, although
    /// this can frequently be achieved by using `#[derive(PartialEq, Eq, Hash)]`.
    /// If you implement these yourself, it is important that the following
    /// property holds:
    ///
    /// ```text
    /// k1 == k2 -> hash(k1) == hash(k2)
    /// ```
    ///
    /// In other words, if two keys are equal, their hashes must be equal.
    ///
    /// It is a logic error for a key to be modified in such a way that the key's
    /// hash, as determined by the [`Hash`] trait, or its equality, as determined by
    /// the [`Eq`] trait, changes while it is in the map. This is normally only
    /// possible through [`Cell`], [`RefCell`], global state, I/O, or unsafe code.
    ///
    /// It is also a logic error for the [`Hash`] implementation of a key to panic.
    /// This is generally only possible if the trait is implemented manually. If a
    /// panic does occur then the contents of the `HashMap` may become corrupted and
    /// some items may be dropped from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// // Type inference lets us omit an explicit type signature (which
    /// // would be `HashMap<String, String>` in this example).
    /// let mut book_reviews = HashMap::new();
    ///
    /// // Review some books.
    /// book_reviews.insert(
    ///     "Adventures of Huckleberry Finn".to_string(),
    ///     "My favorite book.".to_string(),
    /// );
    /// book_reviews.insert(
    ///     "Grimms' Fairy Tales".to_string(),
    ///     "Masterpiece.".to_string(),
    /// );
    /// book_reviews.insert(
    ///     "Pride and Prejudice".to_string(),
    ///     "Very enjoyable.".to_string(),
    /// );
    /// book_reviews.insert(
    ///     "The Adventures of Sherlock Holmes".to_string(),
    ///     "Eye lyked it alot.".to_string(),
    /// );
    ///
    /// // Check for a specific one.
    /// // When collections store owned values (String), they can still be
    /// // queried using references (&str).
    /// if !book_reviews.contains_key("Les Misérables") {
    ///     println!("We've got {} reviews, but Les Misérables ain't one.",
    ///              book_reviews.len());
    /// }
    ///
    /// // oops, this review has a lot of spelling mistakes, let's delete it.
    /// book_reviews.remove("The Adventures of Sherlock Holmes");
    ///
    /// // Look up the values associated with some keys.
    /// let to_find = ["Pride and Prejudice", "Alice's Adventure in Wonderland"];
    /// for &book in &to_find {
    ///     match book_reviews.get(book) {
    ///         Some(review) => println!("{}: {}", book, review),
    ///         None => println!("{} is unreviewed.", book)
    ///     }
    /// }
    ///
    /// // Look up the value for a key (will panic if the key is not found).
    /// println!("Review for Jane: {}", book_reviews["Pride and Prejudice"]);
    ///
    /// // Iterate over everything.
    /// for (book, review) in &book_reviews {
    ///     println!("{}: \"{}\"", book, review);
    /// }
    /// ```
    ///
    /// `HashMap` also implements an [`Entry API`](#method.entry), which allows
    /// for more complex methods of getting, setting, updating and removing keys and
    /// their values:
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// // type inference lets us omit an explicit type signature (which
    /// // would be `HashMap<&str, u8>` in this example).
    /// let mut player_stats = HashMap::new();
    ///
    /// fn random_stat_buff() -> u8 {
    ///     // could actually return some random value here - let's just return
    ///     // some fixed value for now
    ///     42
    /// }
    ///
    /// // insert a key only if it doesn't already exist
    /// player_stats.entry("health").or_insert(100);
    ///
    /// // insert a key using a function that provides a new value only if it
    /// // doesn't already exist
    /// player_stats.entry("defence").or_insert_with(random_stat_buff);
    ///
    /// // update a key, guarding against the key possibly not being set
    /// let stat = player_stats.entry("attack").or_insert(100);
    /// *stat += random_stat_buff();
    /// ```
    ///
    /// The easiest way to use `HashMap` with a custom key type is to derive [`Eq`] and [`Hash`].
    /// We must also derive [`PartialEq`].
    ///
    /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
    /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
    /// [`PartialEq`]: https://doc.rust-lang.org/std/cmp/trait.PartialEq.html
    /// [`RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
    /// [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
    /// [`default`]: #method.default
    /// [`with_hasher`]: #method.with_hasher
    /// [`with_capacity_and_hasher`]: #method.with_capacity_and_hasher
    /// [`fnv`]: https://crates.io/crates/fnv
    /// [`foldhash`]: https://crates.io/crates/foldhash
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// #[derive(Hash, Eq, PartialEq, Debug)]
    /// struct Viking {
    ///     name: String,
    ///     country: String,
    /// }
    ///
    /// impl Viking {
    ///     /// Creates a new Viking.
    ///     fn new(name: &str, country: &str) -> Viking {
    ///         Viking { name: name.to_string(), country: country.to_string() }
    ///     }
    /// }
    ///
    /// // Use a HashMap to store the vikings' health points.
    /// let mut vikings = HashMap::new();
    ///
    /// vikings.insert(Viking::new("Einar", "Norway"), 25);
    /// vikings.insert(Viking::new("Olaf", "Denmark"), 24);
    /// vikings.insert(Viking::new("Harald", "Iceland"), 12);
    ///
    /// // Use derived implementation to print the status of the vikings.
    /// for (viking, health) in &vikings {
    ///     println!("{:?} has {} hp", viking, health);
    /// }
    /// ```
    ///
    /// A `HashMap` with fixed list of elements can be initialized from an array:
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let timber_resources: HashMap<&str, i32> = [("Norway", 100), ("Denmark", 50), ("Iceland", 10)]
    ///     .into_iter().collect();
    /// // use the values stored in map
    /// ```
    pub struct HashMap<K, V, S = DefaultHashBuilder, A: Allocator = Global> {
        pub(crate) hash_builder: S,
        pub(crate) table: RawTable<(K, V), A>,
    }
    impl<K: Clone, V: Clone, S: Clone, A: Allocator + Clone> Clone
    for HashMap<K, V, S, A> {
        fn clone(&self) -> Self {
            HashMap {
                hash_builder: self.hash_builder.clone(),
                table: self.table.clone(),
            }
        }
        fn clone_from(&mut self, source: &Self) {
            self.table.clone_from(&source.table);
            self.hash_builder.clone_from(&source.hash_builder);
        }
    }
    /// Ensures that a single closure type across uses of this which, in turn prevents multiple
    /// instances of any functions like `RawTable::reserve` from being generated
    pub(crate) fn make_hasher<Q, V, S>(hash_builder: &S) -> impl Fn(&(Q, V)) -> u64 + '_
    where
        Q: Hash,
        S: BuildHasher,
    {
        move |val| make_hash::<Q, S>(hash_builder, &val.0)
    }
    /// Ensures that a single closure type across uses of this which, in turn prevents multiple
    /// instances of any functions like `RawTable::reserve` from being generated
    pub(crate) fn equivalent_key<Q, K, V>(k: &Q) -> impl Fn(&(K, V)) -> bool + '_
    where
        Q: Equivalent<K> + ?Sized,
    {
        move |x| k.equivalent(&x.0)
    }
    /// Ensures that a single closure type across uses of this which, in turn prevents multiple
    /// instances of any functions like `RawTable::reserve` from being generated
    #[allow(dead_code)]
    pub(crate) fn equivalent<Q, K>(k: &Q) -> impl Fn(&K) -> bool + '_
    where
        Q: Equivalent<K> + ?Sized,
    {
        move |x| k.equivalent(x)
    }
    pub(crate) fn make_hash<Q, S>(hash_builder: &S, val: &Q) -> u64
    where
        Q: Hash + ?Sized,
        S: BuildHasher,
    {
        use core::hash::Hasher;
        let mut state = hash_builder.build_hasher();
        val.hash(&mut state);
        state.finish()
    }
    impl<K, V, S> HashMap<K, V, S> {
        /// Creates an empty `HashMap` which will use the given hash builder to hash
        /// keys.
        ///
        /// The hash map is initially created with a capacity of 0, so it will not
        /// allocate until it is first inserted into.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashMap` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashMap`].
        ///
        /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
        /// the `HashMap` to be useful, see its documentation for details.
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut map = HashMap::with_hasher(s);
        /// assert_eq!(map.len(), 0);
        /// assert_eq!(map.capacity(), 0);
        ///
        /// map.insert(1, 2);
        /// ```
        pub const fn with_hasher(hash_builder: S) -> Self {
            Self {
                hash_builder,
                table: RawTable::new(),
            }
        }
        /// Creates an empty `HashMap` with the specified capacity, using `hash_builder`
        /// to hash the keys.
        ///
        /// The hash map will be able to hold at least `capacity` elements without
        /// reallocating. If `capacity` is 0, the hash map will not allocate.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashMap` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashMap`].
        ///
        /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
        /// the `HashMap` to be useful, see its documentation for details.
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut map = HashMap::with_capacity_and_hasher(10, s);
        /// assert_eq!(map.len(), 0);
        /// assert!(map.capacity() >= 10);
        ///
        /// map.insert(1, 2);
        /// ```
        pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
            Self {
                hash_builder,
                table: RawTable::with_capacity(capacity),
            }
        }
    }
    impl<K, V, S, A: Allocator> HashMap<K, V, S, A> {
        /// Returns a reference to the underlying allocator.
        #[inline]
        pub fn allocator(&self) -> &A {
            self.table.allocator()
        }
        /// Creates an empty `HashMap` which will use the given hash builder to hash
        /// keys. It will be allocated with the given allocator.
        ///
        /// The hash map is initially created with a capacity of 0, so it will not allocate until it
        /// is first inserted into.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashMap` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashMap`].
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut map = HashMap::with_hasher(s);
        /// map.insert(1, 2);
        /// ```
        pub const fn with_hasher_in(hash_builder: S, alloc: A) -> Self {
            Self {
                hash_builder,
                table: RawTable::new_in(alloc),
            }
        }
        /// Creates an empty `HashMap` with the specified capacity, using `hash_builder`
        /// to hash the keys. It will be allocated with the given allocator.
        ///
        /// The hash map will be able to hold at least `capacity` elements without
        /// reallocating. If `capacity` is 0, the hash map will not allocate.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashMap` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashMap`].
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut map = HashMap::with_capacity_and_hasher(10, s);
        /// map.insert(1, 2);
        /// ```
        pub fn with_capacity_and_hasher_in(
            capacity: usize,
            hash_builder: S,
            alloc: A,
        ) -> Self {
            Self {
                hash_builder,
                table: RawTable::with_capacity_in(capacity, alloc),
            }
        }
        /// Returns a reference to the map's [`BuildHasher`].
        ///
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let hasher = DefaultHashBuilder::default();
        /// let map: HashMap<i32, i32> = HashMap::with_hasher(hasher);
        /// let hasher: &DefaultHashBuilder = map.hasher();
        /// ```
        pub fn hasher(&self) -> &S {
            &self.hash_builder
        }
        /// Returns the number of elements the map can hold without reallocating.
        ///
        /// This number is a lower bound; the `HashMap<K, V>` might be able to hold
        /// more, but is guaranteed to be able to hold at least this many.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// let map: HashMap<i32, i32> = HashMap::with_capacity(100);
        /// assert_eq!(map.len(), 0);
        /// assert!(map.capacity() >= 100);
        /// ```
        pub fn capacity(&self) -> usize {
            self.table.capacity()
        }
        /// An iterator visiting all keys in arbitrary order.
        /// The iterator element type is `&'a K`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        /// assert_eq!(map.len(), 3);
        /// let mut vec: Vec<&str> = Vec::new();
        ///
        /// for key in map.keys() {
        ///     println!("{}", key);
        ///     vec.push(*key);
        /// }
        ///
        /// // The `Keys` iterator produces keys in arbitrary order, so the
        /// // keys must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, ["a", "b", "c"]);
        ///
        /// assert_eq!(map.len(), 3);
        /// ```
        pub fn keys(&self) -> Keys<'_, K, V> {
            Keys { inner: self.iter() }
        }
        /// An iterator visiting all values in arbitrary order.
        /// The iterator element type is `&'a V`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        /// assert_eq!(map.len(), 3);
        /// let mut vec: Vec<i32> = Vec::new();
        ///
        /// for val in map.values() {
        ///     println!("{}", val);
        ///     vec.push(*val);
        /// }
        ///
        /// // The `Values` iterator produces values in arbitrary order, so the
        /// // values must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [1, 2, 3]);
        ///
        /// assert_eq!(map.len(), 3);
        /// ```
        pub fn values(&self) -> Values<'_, K, V> {
            Values { inner: self.iter() }
        }
        /// An iterator visiting all values mutably in arbitrary order.
        /// The iterator element type is `&'a mut V`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        ///
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        ///
        /// for val in map.values_mut() {
        ///     *val = *val + 10;
        /// }
        ///
        /// assert_eq!(map.len(), 3);
        /// let mut vec: Vec<i32> = Vec::new();
        ///
        /// for val in map.values() {
        ///     println!("{}", val);
        ///     vec.push(*val);
        /// }
        ///
        /// // The `Values` iterator produces values in arbitrary order, so the
        /// // values must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [11, 12, 13]);
        ///
        /// assert_eq!(map.len(), 3);
        /// ```
        pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
            ValuesMut {
                inner: self.iter_mut(),
            }
        }
        /// An iterator visiting all key-value pairs in arbitrary order.
        /// The iterator element type is `(&'a K, &'a V)`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        /// assert_eq!(map.len(), 3);
        /// let mut vec: Vec<(&str, i32)> = Vec::new();
        ///
        /// for (key, val) in map.iter() {
        ///     println!("key: {} val: {}", key, val);
        ///     vec.push((*key, *val));
        /// }
        ///
        /// // The `Iter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [("a", 1), ("b", 2), ("c", 3)]);
        ///
        /// assert_eq!(map.len(), 3);
        /// ```
        pub fn iter(&self) -> Iter<'_, K, V> {
            unsafe {
                Iter {
                    inner: self.table.iter(),
                    marker: PhantomData,
                }
            }
        }
        /// An iterator visiting all key-value pairs in arbitrary order,
        /// with mutable references to the values.
        /// The iterator element type is `(&'a K, &'a mut V)`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        ///
        /// // Update all values
        /// for (_, val) in map.iter_mut() {
        ///     *val *= 2;
        /// }
        ///
        /// assert_eq!(map.len(), 3);
        /// let mut vec: Vec<(&str, i32)> = Vec::new();
        ///
        /// for (key, val) in &map {
        ///     println!("key: {} val: {}", key, val);
        ///     vec.push((*key, *val));
        /// }
        ///
        /// // The `Iter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [("a", 2), ("b", 4), ("c", 6)]);
        ///
        /// assert_eq!(map.len(), 3);
        /// ```
        pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
            unsafe {
                IterMut {
                    inner: self.table.iter(),
                    marker: PhantomData,
                }
            }
        }
        /// Returns the number of elements in the map.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut a = HashMap::new();
        /// assert_eq!(a.len(), 0);
        /// a.insert(1, "a");
        /// assert_eq!(a.len(), 1);
        /// ```
        pub fn len(&self) -> usize {
            self.table.len()
        }
        /// Returns `true` if the map contains no elements.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut a = HashMap::new();
        /// assert!(a.is_empty());
        /// a.insert(1, "a");
        /// assert!(!a.is_empty());
        /// ```
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
        /// Clears the map, returning all key-value pairs as an iterator. Keeps the
        /// allocated memory for reuse.
        ///
        /// If the returned iterator is dropped before being fully consumed, it
        /// drops the remaining key-value pairs. The returned iterator keeps a
        /// mutable borrow on the vector to optimize its implementation.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut a = HashMap::new();
        /// a.insert(1, "a");
        /// a.insert(2, "b");
        /// let capacity_before_drain = a.capacity();
        ///
        /// for (k, v) in a.drain().take(1) {
        ///     assert!(k == 1 || k == 2);
        ///     assert!(v == "a" || v == "b");
        /// }
        ///
        /// // As we can see, the map is empty and contains no element.
        /// assert!(a.is_empty() && a.len() == 0);
        /// // But map capacity is equal to old one.
        /// assert_eq!(a.capacity(), capacity_before_drain);
        ///
        /// let mut a = HashMap::new();
        /// a.insert(1, "a");
        /// a.insert(2, "b");
        ///
        /// {   // Iterator is dropped without being consumed.
        ///     let d = a.drain();
        /// }
        ///
        /// // But the map is empty even if we do not use Drain iterator.
        /// assert!(a.is_empty());
        /// ```
        pub fn drain(&mut self) -> Drain<'_, K, V, A> {
            Drain { inner: self.table.drain() }
        }
        /// Retains only the elements specified by the predicate. Keeps the
        /// allocated memory for reuse.
        ///
        /// In other words, remove all pairs `(k, v)` such that `f(&k, &mut v)` returns `false`.
        /// The elements are visited in unsorted (and unspecified) order.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<i32, i32> = (0..8).map(|x|(x, x*10)).collect();
        /// assert_eq!(map.len(), 8);
        ///
        /// map.retain(|&k, _| k % 2 == 0);
        ///
        /// // We can see, that the number of elements inside map is changed.
        /// assert_eq!(map.len(), 4);
        ///
        /// let mut vec: Vec<(i32, i32)> = map.iter().map(|(&k, &v)| (k, v)).collect();
        /// vec.sort_unstable();
        /// assert_eq!(vec, [(0, 0), (2, 20), (4, 40), (6, 60)]);
        /// ```
        pub fn retain<F>(&mut self, mut f: F)
        where
            F: FnMut(&K, &mut V) -> bool,
        {
            unsafe {
                for item in self.table.iter() {
                    let &mut (ref key, ref mut value) = item.as_mut();
                    if !f(key, value) {
                        self.table.erase(item);
                    }
                }
            }
        }
        /// Drains elements which are true under the given predicate,
        /// and returns an iterator over the removed items.
        ///
        /// In other words, move all pairs `(k, v)` such that `f(&k, &mut v)` returns `true` out
        /// into another iterator.
        ///
        /// Note that `extract_if` lets you mutate every value in the filter closure, regardless of
        /// whether you choose to keep or remove it.
        ///
        /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
        /// or the iteration short-circuits, then the remaining elements will be retained.
        /// Use [`retain()`] with a negated predicate if you do not need the returned iterator.
        ///
        /// Keeps the allocated memory for reuse.
        ///
        /// [`retain()`]: HashMap::retain
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<i32, i32> = (0..8).map(|x| (x, x)).collect();
        ///
        /// let drained: HashMap<i32, i32> = map.extract_if(|k, _v| k % 2 == 0).collect();
        ///
        /// let mut evens = drained.keys().cloned().collect::<Vec<_>>();
        /// let mut odds = map.keys().cloned().collect::<Vec<_>>();
        /// evens.sort();
        /// odds.sort();
        ///
        /// assert_eq!(evens, vec![0, 2, 4, 6]);
        /// assert_eq!(odds, vec![1, 3, 5, 7]);
        ///
        /// let mut map: HashMap<i32, i32> = (0..8).map(|x| (x, x)).collect();
        ///
        /// {   // Iterator is dropped without being consumed.
        ///     let d = map.extract_if(|k, _v| k % 2 != 0);
        /// }
        ///
        /// // ExtractIf was not exhausted, therefore no elements were drained.
        /// assert_eq!(map.len(), 8);
        /// ```
        pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, K, V, F, A>
        where
            F: FnMut(&K, &mut V) -> bool,
        {
            ExtractIf {
                f,
                inner: RawExtractIf {
                    iter: unsafe { self.table.iter() },
                    table: &mut self.table,
                },
            }
        }
        /// Clears the map, removing all key-value pairs. Keeps the allocated memory
        /// for reuse.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut a = HashMap::new();
        /// a.insert(1, "a");
        /// let capacity_before_clear = a.capacity();
        ///
        /// a.clear();
        ///
        /// // Map is empty.
        /// assert!(a.is_empty());
        /// // But map capacity is equal to old one.
        /// assert_eq!(a.capacity(), capacity_before_clear);
        /// ```
        pub fn clear(&mut self) {
            self.table.clear();
        }
        /// Creates a consuming iterator visiting all the keys in arbitrary order.
        /// The map cannot be used after calling this.
        /// The iterator element type is `K`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        ///
        /// let mut vec: Vec<&str> = map.into_keys().collect();
        ///
        /// // The `IntoKeys` iterator produces keys in arbitrary order, so the
        /// // keys must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, ["a", "b", "c"]);
        /// ```
        #[inline]
        pub fn into_keys(self) -> IntoKeys<K, V, A> {
            IntoKeys {
                inner: self.into_iter(),
            }
        }
        /// Creates a consuming iterator visiting all the values in arbitrary order.
        /// The map cannot be used after calling this.
        /// The iterator element type is `V`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert("a", 1);
        /// map.insert("b", 2);
        /// map.insert("c", 3);
        ///
        /// let mut vec: Vec<i32> = map.into_values().collect();
        ///
        /// // The `IntoValues` iterator produces values in arbitrary order, so
        /// // the values must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [1, 2, 3]);
        /// ```
        #[inline]
        pub fn into_values(self) -> IntoValues<K, V, A> {
            IntoValues {
                inner: self.into_iter(),
            }
        }
    }
    impl<K, V, S, A> HashMap<K, V, S, A>
    where
        K: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        /// Reserves capacity for at least `additional` more elements to be inserted
        /// in the `HashMap`. The collection may reserve more space to avoid
        /// frequent reallocations.
        ///
        /// # Panics
        ///
        /// Panics if the new capacity exceeds [`isize::MAX`] bytes and [`abort`] the program
        /// in case of allocation error. Use [`try_reserve`](HashMap::try_reserve) instead
        /// if you want to handle memory allocation failure.
        ///
        /// [`isize::MAX`]: https://doc.rust-lang.org/std/primitive.isize.html
        /// [`abort`]: https://doc.rust-lang.org/alloc/alloc/fn.handle_alloc_error.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// let mut map: HashMap<&str, i32> = HashMap::new();
        /// // Map is empty and doesn't allocate memory
        /// assert_eq!(map.capacity(), 0);
        ///
        /// map.reserve(10);
        ///
        /// // And now map can hold at least 10 elements
        /// assert!(map.capacity() >= 10);
        /// ```
        pub fn reserve(&mut self, additional: usize) {
            self.table.reserve(additional, make_hasher::<_, V, S>(&self.hash_builder));
        }
        /// Tries to reserve capacity for at least `additional` more elements to be inserted
        /// in the given `HashMap<K,V>`. The collection may reserve more space to avoid
        /// frequent reallocations.
        ///
        /// # Errors
        ///
        /// If the capacity overflows, or the allocator reports a failure, then an error
        /// is returned.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, isize> = HashMap::new();
        /// // Map is empty and doesn't allocate memory
        /// assert_eq!(map.capacity(), 0);
        ///
        /// map.try_reserve(10).expect("why is the test harness OOMing on 10 bytes?");
        ///
        /// // And now map can hold at least 10 elements
        /// assert!(map.capacity() >= 10);
        /// ```
        /// If the capacity overflows, or the allocator reports a failure, then an error
        /// is returned:
        /// ```
        /// # fn test() {
        /// use hashbrown::HashMap;
        /// use hashbrown::TryReserveError;
        /// let mut map: HashMap<i32, i32> = HashMap::new();
        ///
        /// match map.try_reserve(usize::MAX) {
        ///     Err(error) => match error {
        ///         TryReserveError::CapacityOverflow => {}
        ///         _ => panic!("TryReserveError::AllocError ?"),
        ///     },
        ///     _ => panic!(),
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(not(miri))]
        /// #     test()
        /// # }
        /// ```
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
            self.table
                .try_reserve(additional, make_hasher::<_, V, S>(&self.hash_builder))
        }
        /// Shrinks the capacity of the map as much as possible. It will drop
        /// down as much as possible while maintaining the internal rules
        /// and possibly leaving some space in accordance with the resize policy.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<i32, i32> = HashMap::with_capacity(100);
        /// map.insert(1, 2);
        /// map.insert(3, 4);
        /// assert!(map.capacity() >= 100);
        /// map.shrink_to_fit();
        /// assert!(map.capacity() >= 2);
        /// ```
        pub fn shrink_to_fit(&mut self) {
            self.table.shrink_to(0, make_hasher::<_, V, S>(&self.hash_builder));
        }
        /// Shrinks the capacity of the map with a lower limit. It will drop
        /// down no lower than the supplied limit while maintaining the internal rules
        /// and possibly leaving some space in accordance with the resize policy.
        ///
        /// This function does nothing if the current capacity is smaller than the
        /// supplied minimum capacity.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<i32, i32> = HashMap::with_capacity(100);
        /// map.insert(1, 2);
        /// map.insert(3, 4);
        /// assert!(map.capacity() >= 100);
        /// map.shrink_to(10);
        /// assert!(map.capacity() >= 10);
        /// map.shrink_to(0);
        /// assert!(map.capacity() >= 2);
        /// map.shrink_to(10);
        /// assert!(map.capacity() >= 2);
        /// ```
        pub fn shrink_to(&mut self, min_capacity: usize) {
            self.table
                .shrink_to(min_capacity, make_hasher::<_, V, S>(&self.hash_builder));
        }
        /// Gets the given key's corresponding entry in the map for in-place manipulation.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut letters = HashMap::new();
        ///
        /// for ch in "a short treatise on fungi".chars() {
        ///     let counter = letters.entry(ch).or_insert(0);
        ///     *counter += 1;
        /// }
        ///
        /// assert_eq!(letters[&'s'], 2);
        /// assert_eq!(letters[&'t'], 3);
        /// assert_eq!(letters[&'u'], 1);
        /// assert_eq!(letters.get(&'y'), None);
        /// ```
        pub fn entry(&mut self, key: K) -> Entry<'_, K, V, S, A> {
            let hash = make_hash::<K, S>(&self.hash_builder, &key);
            if let Some(elem) = self.table.find(hash, equivalent_key(&key)) {
                Entry::Occupied(OccupiedEntry {
                    hash,
                    elem,
                    table: self,
                })
            } else {
                Entry::Vacant(VacantEntry {
                    hash,
                    key,
                    table: self,
                })
            }
        }
        /// Gets the given key's corresponding entry by reference in the map for in-place manipulation.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut words: HashMap<String, usize> = HashMap::new();
        /// let source = ["poneyland", "horseyland", "poneyland", "poneyland"];
        /// for (i, &s) in source.iter().enumerate() {
        ///     let counter = words.entry_ref(s).or_insert(0);
        ///     *counter += 1;
        /// }
        ///
        /// assert_eq!(words["poneyland"], 3);
        /// assert_eq!(words["horseyland"], 1);
        /// ```
        pub fn entry_ref<'a, 'b, Q>(
            &'a mut self,
            key: &'b Q,
        ) -> EntryRef<'a, 'b, K, Q, V, S, A>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            let hash = make_hash::<Q, S>(&self.hash_builder, key);
            if let Some(elem) = self.table.find(hash, equivalent_key(key)) {
                EntryRef::Occupied(OccupiedEntry {
                    hash,
                    elem,
                    table: self,
                })
            } else {
                EntryRef::Vacant(VacantEntryRef {
                    hash,
                    key,
                    table: self,
                })
            }
        }
        /// Returns a reference to the value corresponding to the key.
        ///
        /// The key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, "a");
        /// assert_eq!(map.get(&1), Some(&"a"));
        /// assert_eq!(map.get(&2), None);
        /// ```
        #[inline]
        pub fn get<Q>(&self, k: &Q) -> Option<&V>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            if !self.table.is_empty() {
                let hash = make_hash::<Q, S>(&self.hash_builder, k);
                match self.table.get(hash, equivalent_key(k)) {
                    Some((_, v)) => Some(v),
                    None => None,
                }
            } else {
                None
            }
        }
        /// Returns the key-value pair corresponding to the supplied key.
        ///
        /// The supplied key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, "a");
        /// assert_eq!(map.get_key_value(&1), Some((&1, &"a")));
        /// assert_eq!(map.get_key_value(&2), None);
        /// ```
        #[inline]
        pub fn get_key_value<Q>(&self, k: &Q) -> Option<(&K, &V)>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            if !self.table.is_empty() {
                let hash = make_hash::<Q, S>(&self.hash_builder, k);
                match self.table.get(hash, equivalent_key(k)) {
                    Some((key, value)) => Some((key, value)),
                    None => None,
                }
            } else {
                None
            }
        }
        /// Returns the key-value pair corresponding to the supplied key, with a mutable reference to value.
        ///
        /// The supplied key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, "a");
        /// let (k, v) = map.get_key_value_mut(&1).unwrap();
        /// assert_eq!(k, &1);
        /// assert_eq!(v, &mut "a");
        /// *v = "b";
        /// assert_eq!(map.get_key_value_mut(&1), Some((&1, &mut "b")));
        /// assert_eq!(map.get_key_value_mut(&2), None);
        /// ```
        #[inline]
        pub fn get_key_value_mut<Q>(&mut self, k: &Q) -> Option<(&K, &mut V)>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            if !self.table.is_empty() {
                let hash = make_hash::<Q, S>(&self.hash_builder, k);
                match self.table.get_mut(hash, equivalent_key(k)) {
                    Some(&mut (ref key, ref mut value)) => Some((key, value)),
                    None => None,
                }
            } else {
                None
            }
        }
        /// Returns `true` if the map contains a value for the specified key.
        ///
        /// The key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, "a");
        /// assert_eq!(map.contains_key(&1), true);
        /// assert_eq!(map.contains_key(&2), false);
        /// ```
        pub fn contains_key<Q>(&self, k: &Q) -> bool
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            if !self.table.is_empty() {
                let hash = make_hash::<Q, S>(&self.hash_builder, k);
                self.table.get(hash, equivalent_key(k)).is_some()
            } else {
                false
            }
        }
        /// Returns a mutable reference to the value corresponding to the key.
        ///
        /// The key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, "a");
        /// if let Some(x) = map.get_mut(&1) {
        ///     *x = "b";
        /// }
        /// assert_eq!(map[&1], "b");
        ///
        /// assert_eq!(map.get_mut(&2), None);
        /// ```
        pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            if !self.table.is_empty() {
                let hash = make_hash::<Q, S>(&self.hash_builder, k);
                match self.table.get_mut(hash, equivalent_key(k)) {
                    Some(&mut (_, ref mut v)) => Some(v),
                    None => None,
                }
            } else {
                None
            }
        }
        /// Attempts to get mutable references to `N` values in the map at once.
        ///
        /// Returns an array of length `N` with the results of each query. For soundness, at most one
        /// mutable reference will be returned to any value. `None` will be used if the key is missing.
        ///
        /// # Panics
        ///
        /// Panics if any keys are overlapping.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut libraries = HashMap::new();
        /// libraries.insert("Bodleian Library".to_string(), 1602);
        /// libraries.insert("Athenæum".to_string(), 1807);
        /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
        /// libraries.insert("Library of Congress".to_string(), 1800);
        ///
        /// // Get Athenæum and Bodleian Library
        /// let [Some(a), Some(b)] = libraries.get_disjoint_mut([
        ///     "Athenæum",
        ///     "Bodleian Library",
        /// ]) else { panic!() };
        ///
        /// // Assert values of Athenæum and Library of Congress
        /// let got = libraries.get_disjoint_mut([
        ///     "Athenæum",
        ///     "Library of Congress",
        /// ]);
        /// assert_eq!(
        ///     got,
        ///     [
        ///         Some(&mut 1807),
        ///         Some(&mut 1800),
        ///     ],
        /// );
        ///
        /// // Missing keys result in None
        /// let got = libraries.get_disjoint_mut([
        ///     "Athenæum",
        ///     "New York Public Library",
        /// ]);
        /// assert_eq!(
        ///     got,
        ///     [
        ///         Some(&mut 1807),
        ///         None
        ///     ]
        /// );
        /// ```
        ///
        /// ```should_panic
        /// use hashbrown::HashMap;
        ///
        /// let mut libraries = HashMap::new();
        /// libraries.insert("Athenæum".to_string(), 1807);
        ///
        /// // Duplicate keys panic!
        /// let got = libraries.get_disjoint_mut([
        ///     "Athenæum",
        ///     "Athenæum",
        /// ]);
        /// ```
        pub fn get_disjoint_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<&'_ mut V>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_mut_inner(ks).map(|res| res.map(|(_, v)| v))
        }
        /// Attempts to get mutable references to `N` values in the map at once.
        #[deprecated(note = "use `get_disjoint_mut` instead")]
        pub fn get_many_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<&'_ mut V>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_mut(ks)
        }
        /// Attempts to get mutable references to `N` values in the map at once, without validating that
        /// the values are unique.
        ///
        /// Returns an array of length `N` with the results of each query. `None` will be used if
        /// the key is missing.
        ///
        /// For a safe alternative see [`get_disjoint_mut`](`HashMap::get_disjoint_mut`).
        ///
        /// # Safety
        ///
        /// Calling this method with overlapping keys is *[undefined behavior]* even if the resulting
        /// references are not used.
        ///
        /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut libraries = HashMap::new();
        /// libraries.insert("Bodleian Library".to_string(), 1602);
        /// libraries.insert("Athenæum".to_string(), 1807);
        /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
        /// libraries.insert("Library of Congress".to_string(), 1800);
        ///
        /// // SAFETY: The keys do not overlap.
        /// let [Some(a), Some(b)] = (unsafe { libraries.get_disjoint_unchecked_mut([
        ///     "Athenæum",
        ///     "Bodleian Library",
        /// ]) }) else { panic!() };
        ///
        /// // SAFETY: The keys do not overlap.
        /// let got = unsafe { libraries.get_disjoint_unchecked_mut([
        ///     "Athenæum",
        ///     "Library of Congress",
        /// ]) };
        /// assert_eq!(
        ///     got,
        ///     [
        ///         Some(&mut 1807),
        ///         Some(&mut 1800),
        ///     ],
        /// );
        ///
        /// // SAFETY: The keys do not overlap.
        /// let got = unsafe { libraries.get_disjoint_unchecked_mut([
        ///     "Athenæum",
        ///     "New York Public Library",
        /// ]) };
        /// // Missing keys result in None
        /// assert_eq!(got, [Some(&mut 1807), None]);
        /// ```
        pub unsafe fn get_disjoint_unchecked_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<&'_ mut V>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_unchecked_mut_inner(ks).map(|res| res.map(|(_, v)| v))
        }
        /// Attempts to get mutable references to `N` values in the map at once, without validating that
        /// the values are unique.
        #[deprecated(note = "use `get_disjoint_unchecked_mut` instead")]
        pub unsafe fn get_many_unchecked_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<&'_ mut V>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_unchecked_mut(ks)
        }
        /// Attempts to get mutable references to `N` values in the map at once, with immutable
        /// references to the corresponding keys.
        ///
        /// Returns an array of length `N` with the results of each query. For soundness, at most one
        /// mutable reference will be returned to any value. `None` will be used if the key is missing.
        ///
        /// # Panics
        ///
        /// Panics if any keys are overlapping.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut libraries = HashMap::new();
        /// libraries.insert("Bodleian Library".to_string(), 1602);
        /// libraries.insert("Athenæum".to_string(), 1807);
        /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
        /// libraries.insert("Library of Congress".to_string(), 1800);
        ///
        /// let got = libraries.get_disjoint_key_value_mut([
        ///     "Bodleian Library",
        ///     "Herzogin-Anna-Amalia-Bibliothek",
        /// ]);
        /// assert_eq!(
        ///     got,
        ///     [
        ///         Some((&"Bodleian Library".to_string(), &mut 1602)),
        ///         Some((&"Herzogin-Anna-Amalia-Bibliothek".to_string(), &mut 1691)),
        ///     ],
        /// );
        /// // Missing keys result in None
        /// let got = libraries.get_disjoint_key_value_mut([
        ///     "Bodleian Library",
        ///     "Gewandhaus",
        /// ]);
        /// assert_eq!(got, [Some((&"Bodleian Library".to_string(), &mut 1602)), None]);
        /// ```
        ///
        /// ```should_panic
        /// use hashbrown::HashMap;
        ///
        /// let mut libraries = HashMap::new();
        /// libraries.insert("Bodleian Library".to_string(), 1602);
        /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
        ///
        /// // Duplicate keys result in panic!
        /// let got = libraries.get_disjoint_key_value_mut([
        ///     "Bodleian Library",
        ///     "Herzogin-Anna-Amalia-Bibliothek",
        ///     "Herzogin-Anna-Amalia-Bibliothek",
        /// ]);
        /// ```
        pub fn get_disjoint_key_value_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<(&'_ K, &'_ mut V)>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_mut_inner(ks).map(|res| res.map(|(k, v)| (&*k, v)))
        }
        /// Attempts to get mutable references to `N` values in the map at once, with immutable
        /// references to the corresponding keys.
        #[deprecated(note = "use `get_disjoint_key_value_mut` instead")]
        pub fn get_many_key_value_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<(&'_ K, &'_ mut V)>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_key_value_mut(ks)
        }
        /// Attempts to get mutable references to `N` values in the map at once, with immutable
        /// references to the corresponding keys, without validating that the values are unique.
        ///
        /// Returns an array of length `N` with the results of each query. `None` will be returned if
        /// any of the keys are missing.
        ///
        /// For a safe alternative see [`get_disjoint_key_value_mut`](`HashMap::get_disjoint_key_value_mut`).
        ///
        /// # Safety
        ///
        /// Calling this method with overlapping keys is *[undefined behavior]* even if the resulting
        /// references are not used.
        ///
        /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut libraries = HashMap::new();
        /// libraries.insert("Bodleian Library".to_string(), 1602);
        /// libraries.insert("Athenæum".to_string(), 1807);
        /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
        /// libraries.insert("Library of Congress".to_string(), 1800);
        ///
        /// let got = libraries.get_disjoint_key_value_mut([
        ///     "Bodleian Library",
        ///     "Herzogin-Anna-Amalia-Bibliothek",
        /// ]);
        /// assert_eq!(
        ///     got,
        ///     [
        ///         Some((&"Bodleian Library".to_string(), &mut 1602)),
        ///         Some((&"Herzogin-Anna-Amalia-Bibliothek".to_string(), &mut 1691)),
        ///     ],
        /// );
        /// // Missing keys result in None
        /// let got = libraries.get_disjoint_key_value_mut([
        ///     "Bodleian Library",
        ///     "Gewandhaus",
        /// ]);
        /// assert_eq!(
        ///     got,
        ///     [
        ///         Some((&"Bodleian Library".to_string(), &mut 1602)),
        ///         None,
        ///     ],
        /// );
        /// ```
        pub unsafe fn get_disjoint_key_value_unchecked_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<(&'_ K, &'_ mut V)>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_unchecked_mut_inner(ks)
                .map(|res| res.map(|(k, v)| (&*k, v)))
        }
        /// Attempts to get mutable references to `N` values in the map at once, with immutable
        /// references to the corresponding keys, without validating that the values are unique.
        #[deprecated(note = "use `get_disjoint_key_value_unchecked_mut` instead")]
        pub unsafe fn get_many_key_value_unchecked_mut<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<(&'_ K, &'_ mut V)>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            self.get_disjoint_key_value_unchecked_mut(ks)
        }
        fn get_disjoint_mut_inner<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<&'_ mut (K, V)>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            let hashes = self.build_hashes_inner(ks);
            self.table.get_disjoint_mut(hashes, |i, (k, _)| ks[i].equivalent(k))
        }
        unsafe fn get_disjoint_unchecked_mut_inner<Q, const N: usize>(
            &mut self,
            ks: [&Q; N],
        ) -> [Option<&'_ mut (K, V)>; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            let hashes = self.build_hashes_inner(ks);
            self.table
                .get_disjoint_unchecked_mut(hashes, |i, (k, _)| ks[i].equivalent(k))
        }
        fn build_hashes_inner<Q, const N: usize>(&self, ks: [&Q; N]) -> [u64; N]
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            let mut hashes = [0_u64; N];
            for i in 0..N {
                hashes[i] = make_hash::<Q, S>(&self.hash_builder, ks[i]);
            }
            hashes
        }
        /// Inserts a key-value pair into the map.
        ///
        /// If the map did not have this key present, [`None`] is returned.
        ///
        /// If the map did have this key present, the value is updated, and the old
        /// value is returned. The key is not updated, though; this matters for
        /// types that can be `==` without being identical. See the [`std::collections`]
        /// [module-level documentation] for more.
        ///
        /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
        /// [`std::collections`]: https://doc.rust-lang.org/std/collections/index.html
        /// [module-level documentation]: https://doc.rust-lang.org/std/collections/index.html#insert-and-complex-keys
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// assert_eq!(map.insert(37, "a"), None);
        /// assert_eq!(map.is_empty(), false);
        ///
        /// map.insert(37, "b");
        /// assert_eq!(map.insert(37, "c"), Some("b"));
        /// assert_eq!(map[&37], "c");
        /// ```
        pub fn insert(&mut self, k: K, v: V) -> Option<V> {
            let hash = make_hash::<K, S>(&self.hash_builder, &k);
            match self.find_or_find_insert_index(hash, &k) {
                Ok(bucket) => Some(mem::replace(unsafe { &mut bucket.as_mut().1 }, v)),
                Err(index) => {
                    unsafe {
                        self.table.insert_at_index(hash, index, (k, v));
                    }
                    None
                }
            }
        }
        pub(crate) fn find_or_find_insert_index<Q>(
            &mut self,
            hash: u64,
            key: &Q,
        ) -> Result<Bucket<(K, V)>, usize>
        where
            Q: Equivalent<K> + ?Sized,
        {
            self.table
                .find_or_find_insert_index(
                    hash,
                    equivalent_key(key),
                    make_hasher(&self.hash_builder),
                )
        }
        /// Insert a key-value pair into the map without checking
        /// if the key already exists in the map.
        ///
        /// This operation is faster than regular insert, because it does not perform
        /// lookup before insertion.
        ///
        /// This operation is useful during initial population of the map.
        /// For example, when constructing a map from another map, we know
        /// that keys are unique.
        ///
        /// Returns a reference to the key and value just inserted.
        ///
        /// # Safety
        ///
        /// This operation is safe if a key does not exist in the map.
        ///
        /// However, if a key exists in the map already, the behavior is unspecified:
        /// this operation may panic, loop forever, or any following operation with the map
        /// may panic, loop forever or return arbitrary result.
        ///
        /// That said, this operation (and following operations) are guaranteed to
        /// not violate memory safety.
        ///
        /// However this operation is still unsafe because the resulting `HashMap`
        /// may be passed to unsafe code which does expect the map to behave
        /// correctly, and would cause unsoundness as a result.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map1 = HashMap::new();
        /// assert_eq!(map1.insert(1, "a"), None);
        /// assert_eq!(map1.insert(2, "b"), None);
        /// assert_eq!(map1.insert(3, "c"), None);
        /// assert_eq!(map1.len(), 3);
        ///
        /// let mut map2 = HashMap::new();
        ///
        /// for (key, value) in map1.into_iter() {
        ///     unsafe {
        ///         map2.insert_unique_unchecked(key, value);
        ///     }
        /// }
        ///
        /// let (key, value) = unsafe { map2.insert_unique_unchecked(4, "d") };
        /// assert_eq!(key, &4);
        /// assert_eq!(value, &mut "d");
        /// *value = "e";
        ///
        /// assert_eq!(map2[&1], "a");
        /// assert_eq!(map2[&2], "b");
        /// assert_eq!(map2[&3], "c");
        /// assert_eq!(map2[&4], "e");
        /// assert_eq!(map2.len(), 4);
        /// ```
        pub unsafe fn insert_unique_unchecked(&mut self, k: K, v: V) -> (&K, &mut V) {
            let hash = make_hash::<K, S>(&self.hash_builder, &k);
            let bucket = self
                .table
                .insert(hash, (k, v), make_hasher::<_, V, S>(&self.hash_builder));
            let (k_ref, v_ref) = unsafe { bucket.as_mut() };
            (k_ref, v_ref)
        }
        /// Tries to insert a key-value pair into the map, and returns
        /// a mutable reference to the value in the entry.
        ///
        /// # Errors
        ///
        /// If the map already had this key present, nothing is updated, and
        /// an error containing the occupied entry and the value is returned.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::OccupiedError;
        ///
        /// let mut map = HashMap::new();
        /// assert_eq!(map.try_insert(37, "a").unwrap(), &"a");
        ///
        /// match map.try_insert(37, "b") {
        ///     Err(OccupiedError { entry, value }) => {
        ///         assert_eq!(entry.key(), &37);
        ///         assert_eq!(entry.get(), &"a");
        ///         assert_eq!(value, "b");
        ///     }
        ///     _ => panic!()
        /// }
        /// ```
        pub fn try_insert(
            &mut self,
            key: K,
            value: V,
        ) -> Result<&mut V, OccupiedError<'_, K, V, S, A>> {
            match self.entry(key) {
                Entry::Occupied(entry) => Err(OccupiedError { entry, value }),
                Entry::Vacant(entry) => Ok(entry.insert(value)),
            }
        }
        /// Removes a key from the map, returning the value at the key if the key
        /// was previously in the map. Keeps the allocated memory for reuse.
        ///
        /// The key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// // The map is empty
        /// assert!(map.is_empty() && map.capacity() == 0);
        ///
        /// map.insert(1, "a");
        ///
        /// assert_eq!(map.remove(&1), Some("a"));
        /// assert_eq!(map.remove(&1), None);
        ///
        /// // Now map holds none elements
        /// assert!(map.is_empty());
        /// ```
        pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            match self.remove_entry(k) {
                Some((_, v)) => Some(v),
                None => None,
            }
        }
        /// Removes a key from the map, returning the stored key and value if the
        /// key was previously in the map. Keeps the allocated memory for reuse.
        ///
        /// The key may be any borrowed form of the map's key type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the key type.
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// // The map is empty
        /// assert!(map.is_empty() && map.capacity() == 0);
        ///
        /// map.insert(1, "a");
        ///
        /// assert_eq!(map.remove_entry(&1), Some((1, "a")));
        /// assert_eq!(map.remove(&1), None);
        ///
        /// // Now map hold none elements
        /// assert!(map.is_empty());
        /// ```
        pub fn remove_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
        where
            Q: Hash + Equivalent<K> + ?Sized,
        {
            let hash = make_hash::<Q, S>(&self.hash_builder, k);
            self.table.remove_entry(hash, equivalent_key(k))
        }
        /// Returns the total amount of memory allocated internally by the hash
        /// set, in bytes.
        ///
        /// The returned number is informational only. It is intended to be
        /// primarily used for memory profiling.
        #[inline]
        pub fn allocation_size(&self) -> usize {
            self.table.allocation_size()
        }
    }
    impl<K, V, S, A> PartialEq for HashMap<K, V, S, A>
    where
        K: Eq + Hash,
        V: PartialEq,
        S: BuildHasher,
        A: Allocator,
    {
        fn eq(&self, other: &Self) -> bool {
            if self.len() != other.len() {
                return false;
            }
            self.iter()
                .all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
        }
    }
    impl<K, V, S, A> Eq for HashMap<K, V, S, A>
    where
        K: Eq + Hash,
        V: Eq,
        S: BuildHasher,
        A: Allocator,
    {}
    impl<K, V, S, A> Debug for HashMap<K, V, S, A>
    where
        K: Debug,
        V: Debug,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map().entries(self.iter()).finish()
        }
    }
    impl<K, V, S, A> Default for HashMap<K, V, S, A>
    where
        S: Default,
        A: Default + Allocator,
    {
        /// Creates an empty `HashMap<K, V, S, A>`, with the `Default` value for the hasher and allocator.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use std::collections::hash_map::RandomState;
        ///
        /// // You can specify all types of HashMap, including hasher and allocator.
        /// // Created map is empty and don't allocate memory
        /// let map: HashMap<u32, String> = Default::default();
        /// assert_eq!(map.capacity(), 0);
        /// let map: HashMap<u32, String, RandomState> = HashMap::default();
        /// assert_eq!(map.capacity(), 0);
        /// ```
        fn default() -> Self {
            Self::with_hasher_in(Default::default(), Default::default())
        }
    }
    impl<K, Q, V, S, A> Index<&Q> for HashMap<K, V, S, A>
    where
        K: Eq + Hash,
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
        A: Allocator,
    {
        type Output = V;
        /// Returns a reference to the value corresponding to the supplied key.
        ///
        /// # Panics
        ///
        /// Panics if the key is not present in the `HashMap`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let map: HashMap<_, _> = [("a", "One"), ("b", "Two")].into();
        ///
        /// assert_eq!(map[&"a"], "One");
        /// assert_eq!(map[&"b"], "Two");
        /// ```
        fn index(&self, key: &Q) -> &V {
            self.get(key).expect("no entry found for key")
        }
    }
    /// An iterator over the entries of a `HashMap` in arbitrary order.
    /// The iterator element type is `(&'a K, &'a V)`.
    ///
    /// This `struct` is created by the [`iter`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`iter`]: struct.HashMap.html#method.iter
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut iter = map.iter();
    /// let mut vec = vec![iter.next(), iter.next(), iter.next()];
    ///
    /// // The `Iter` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some((&1, &"a")), Some((&2, &"b")), Some((&3, &"c"))]);
    ///
    /// // It is fused iterator
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(iter.next(), None);
    /// ```
    pub struct Iter<'a, K, V> {
        inner: RawIter<(K, V)>,
        marker: PhantomData<(&'a K, &'a V)>,
    }
    impl<K, V> Clone for Iter<'_, K, V> {
        fn clone(&self) -> Self {
            Iter {
                inner: self.inner.clone(),
                marker: PhantomData,
            }
        }
    }
    impl<K: Debug, V: Debug> fmt::Debug for Iter<'_, K, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// A mutable iterator over the entries of a `HashMap` in arbitrary order.
    /// The iterator element type is `(&'a K, &'a mut V)`.
    ///
    /// This `struct` is created by the [`iter_mut`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`iter_mut`]: struct.HashMap.html#method.iter_mut
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let mut map: HashMap<_, _> = [(1, "One".to_owned()), (2, "Two".into())].into();
    ///
    /// let mut iter = map.iter_mut();
    /// iter.next().map(|(_, v)| v.push_str(" Mississippi"));
    /// iter.next().map(|(_, v)| v.push_str(" Mississippi"));
    ///
    /// // It is fused iterator
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(iter.next(), None);
    ///
    /// assert_eq!(map.get(&1).unwrap(), &"One Mississippi".to_owned());
    /// assert_eq!(map.get(&2).unwrap(), &"Two Mississippi".to_owned());
    /// ```
    pub struct IterMut<'a, K, V> {
        inner: RawIter<(K, V)>,
        marker: PhantomData<(&'a K, &'a mut V)>,
    }
    unsafe impl<K: Send, V: Send> Send for IterMut<'_, K, V> {}
    impl<K, V> IterMut<'_, K, V> {
        /// Returns a iterator of references over the remaining items.
        pub(super) fn iter(&self) -> Iter<'_, K, V> {
            Iter {
                inner: self.inner.clone(),
                marker: PhantomData,
            }
        }
    }
    /// An owning iterator over the entries of a `HashMap` in arbitrary order.
    /// The iterator element type is `(K, V)`.
    ///
    /// This `struct` is created by the [`into_iter`] method on [`HashMap`]
    /// (provided by the [`IntoIterator`] trait). See its documentation for more.
    /// The map cannot be used after calling that method.
    ///
    /// [`into_iter`]: struct.HashMap.html#method.into_iter
    /// [`HashMap`]: struct.HashMap.html
    /// [`IntoIterator`]: https://doc.rust-lang.org/core/iter/trait.IntoIterator.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut iter = map.into_iter();
    /// let mut vec = vec![iter.next(), iter.next(), iter.next()];
    ///
    /// // The `IntoIter` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some((1, "a")), Some((2, "b")), Some((3, "c"))]);
    ///
    /// // It is fused iterator
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(iter.next(), None);
    /// ```
    pub struct IntoIter<K, V, A: Allocator = Global> {
        inner: RawIntoIter<(K, V), A>,
    }
    impl<K, V, A: Allocator> IntoIter<K, V, A> {
        /// Returns a iterator of references over the remaining items.
        pub(super) fn iter(&self) -> Iter<'_, K, V> {
            Iter {
                inner: self.inner.iter(),
                marker: PhantomData,
            }
        }
    }
    /// An owning iterator over the keys of a `HashMap` in arbitrary order.
    /// The iterator element type is `K`.
    ///
    /// This `struct` is created by the [`into_keys`] method on [`HashMap`].
    /// See its documentation for more.
    /// The map cannot be used after calling that method.
    ///
    /// [`into_keys`]: struct.HashMap.html#method.into_keys
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut keys = map.into_keys();
    /// let mut vec = vec![keys.next(), keys.next(), keys.next()];
    ///
    /// // The `IntoKeys` iterator produces keys in arbitrary order, so the
    /// // keys must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some(1), Some(2), Some(3)]);
    ///
    /// // It is fused iterator
    /// assert_eq!(keys.next(), None);
    /// assert_eq!(keys.next(), None);
    /// ```
    pub struct IntoKeys<K, V, A: Allocator = Global> {
        inner: IntoIter<K, V, A>,
    }
    impl<K, V, A: Allocator> Default for IntoKeys<K, V, A> {
        fn default() -> Self {
            Self { inner: Default::default() }
        }
    }
    impl<K, V, A: Allocator> Iterator for IntoKeys<K, V, A> {
        type Item = K;
        #[inline]
        fn next(&mut self) -> Option<K> {
            self.inner.next().map(|(k, _)| k)
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        #[inline]
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, (k, _)| f(acc, k))
        }
    }
    impl<K, V, A: Allocator> ExactSizeIterator for IntoKeys<K, V, A> {
        #[inline]
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V, A: Allocator> FusedIterator for IntoKeys<K, V, A> {}
    impl<K: Debug, V: Debug, A: Allocator> fmt::Debug for IntoKeys<K, V, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.inner.iter().map(|(k, _)| k)).finish()
        }
    }
    /// An owning iterator over the values of a `HashMap` in arbitrary order.
    /// The iterator element type is `V`.
    ///
    /// This `struct` is created by the [`into_values`] method on [`HashMap`].
    /// See its documentation for more. The map cannot be used after calling that method.
    ///
    /// [`into_values`]: struct.HashMap.html#method.into_values
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut values = map.into_values();
    /// let mut vec = vec![values.next(), values.next(), values.next()];
    ///
    /// // The `IntoValues` iterator produces values in arbitrary order, so
    /// // the values must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some("a"), Some("b"), Some("c")]);
    ///
    /// // It is fused iterator
    /// assert_eq!(values.next(), None);
    /// assert_eq!(values.next(), None);
    /// ```
    pub struct IntoValues<K, V, A: Allocator = Global> {
        inner: IntoIter<K, V, A>,
    }
    impl<K, V, A: Allocator> Default for IntoValues<K, V, A> {
        fn default() -> Self {
            Self { inner: Default::default() }
        }
    }
    impl<K, V, A: Allocator> Iterator for IntoValues<K, V, A> {
        type Item = V;
        #[inline]
        fn next(&mut self) -> Option<V> {
            self.inner.next().map(|(_, v)| v)
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        #[inline]
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, (_, v)| f(acc, v))
        }
    }
    impl<K, V, A: Allocator> ExactSizeIterator for IntoValues<K, V, A> {
        #[inline]
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V, A: Allocator> FusedIterator for IntoValues<K, V, A> {}
    impl<K, V: Debug, A: Allocator> fmt::Debug for IntoValues<K, V, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.inner.iter().map(|(_, v)| v)).finish()
        }
    }
    /// An iterator over the keys of a `HashMap` in arbitrary order.
    /// The iterator element type is `&'a K`.
    ///
    /// This `struct` is created by the [`keys`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`keys`]: struct.HashMap.html#method.keys
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut keys = map.keys();
    /// let mut vec = vec![keys.next(), keys.next(), keys.next()];
    ///
    /// // The `Keys` iterator produces keys in arbitrary order, so the
    /// // keys must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some(&1), Some(&2), Some(&3)]);
    ///
    /// // It is fused iterator
    /// assert_eq!(keys.next(), None);
    /// assert_eq!(keys.next(), None);
    /// ```
    pub struct Keys<'a, K, V> {
        inner: Iter<'a, K, V>,
    }
    impl<K, V> Clone for Keys<'_, K, V> {
        fn clone(&self) -> Self {
            Keys { inner: self.inner.clone() }
        }
    }
    impl<K: Debug, V> fmt::Debug for Keys<'_, K, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// An iterator over the values of a `HashMap` in arbitrary order.
    /// The iterator element type is `&'a V`.
    ///
    /// This `struct` is created by the [`values`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`values`]: struct.HashMap.html#method.values
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut values = map.values();
    /// let mut vec = vec![values.next(), values.next(), values.next()];
    ///
    /// // The `Values` iterator produces values in arbitrary order, so the
    /// // values must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some(&"a"), Some(&"b"), Some(&"c")]);
    ///
    /// // It is fused iterator
    /// assert_eq!(values.next(), None);
    /// assert_eq!(values.next(), None);
    /// ```
    pub struct Values<'a, K, V> {
        inner: Iter<'a, K, V>,
    }
    impl<K, V> Clone for Values<'_, K, V> {
        fn clone(&self) -> Self {
            Values {
                inner: self.inner.clone(),
            }
        }
    }
    impl<K, V: Debug> fmt::Debug for Values<'_, K, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// A draining iterator over the entries of a `HashMap` in arbitrary
    /// order. The iterator element type is `(K, V)`.
    ///
    /// This `struct` is created by the [`drain`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`drain`]: struct.HashMap.html#method.drain
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let mut map: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut drain_iter = map.drain();
    /// let mut vec = vec![drain_iter.next(), drain_iter.next(), drain_iter.next()];
    ///
    /// // The `Drain` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some((1, "a")), Some((2, "b")), Some((3, "c"))]);
    ///
    /// // It is fused iterator
    /// assert_eq!(drain_iter.next(), None);
    /// assert_eq!(drain_iter.next(), None);
    /// ```
    pub struct Drain<'a, K, V, A: Allocator = Global> {
        inner: RawDrain<'a, (K, V), A>,
    }
    impl<K, V, A: Allocator> Drain<'_, K, V, A> {
        /// Returns a iterator of references over the remaining items.
        pub(super) fn iter(&self) -> Iter<'_, K, V> {
            Iter {
                inner: self.inner.iter(),
                marker: PhantomData,
            }
        }
    }
    /// A draining iterator over entries of a `HashMap` which don't satisfy the predicate
    /// `f(&k, &mut v)` in arbitrary order. The iterator element type is `(K, V)`.
    ///
    /// This `struct` is created by the [`extract_if`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`extract_if`]: struct.HashMap.html#method.extract_if
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let mut map: HashMap<i32, &str> = [(1, "a"), (2, "b"), (3, "c")].into();
    ///
    /// let mut extract_if = map.extract_if(|k, _v| k % 2 != 0);
    /// let mut vec = vec![extract_if.next(), extract_if.next()];
    ///
    /// // The `ExtractIf` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [Some((1, "a")),Some((3, "c"))]);
    ///
    /// // It is fused iterator
    /// assert_eq!(extract_if.next(), None);
    /// assert_eq!(extract_if.next(), None);
    /// drop(extract_if);
    ///
    /// assert_eq!(map.len(), 1);
    /// ```
    #[must_use = "Iterators are lazy unless consumed"]
    pub struct ExtractIf<'a, K, V, F, A: Allocator = Global> {
        f: F,
        inner: RawExtractIf<'a, (K, V), A>,
    }
    impl<K, V, F, A> Iterator for ExtractIf<'_, K, V, F, A>
    where
        F: FnMut(&K, &mut V) -> bool,
        A: Allocator,
    {
        type Item = (K, V);
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next(|&mut (ref k, ref mut v)| (self.f)(k, v))
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.inner.iter.size_hint().1)
        }
    }
    impl<K, V, F> FusedIterator for ExtractIf<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
    {}
    /// A mutable iterator over the values of a `HashMap` in arbitrary order.
    /// The iterator element type is `&'a mut V`.
    ///
    /// This `struct` is created by the [`values_mut`] method on [`HashMap`]. See its
    /// documentation for more.
    ///
    /// [`values_mut`]: struct.HashMap.html#method.values_mut
    /// [`HashMap`]: struct.HashMap.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashMap;
    ///
    /// let mut map: HashMap<_, _> = [(1, "One".to_owned()), (2, "Two".into())].into();
    ///
    /// let mut values = map.values_mut();
    /// values.next().map(|v| v.push_str(" Mississippi"));
    /// values.next().map(|v| v.push_str(" Mississippi"));
    ///
    /// // It is fused iterator
    /// assert_eq!(values.next(), None);
    /// assert_eq!(values.next(), None);
    ///
    /// assert_eq!(map.get(&1).unwrap(), &"One Mississippi".to_owned());
    /// assert_eq!(map.get(&2).unwrap(), &"Two Mississippi".to_owned());
    /// ```
    pub struct ValuesMut<'a, K, V> {
        inner: IterMut<'a, K, V>,
    }
    /// A view into a single entry in a map, which may either be vacant or occupied.
    ///
    /// This `enum` is constructed from the [`entry`] method on [`HashMap`].
    ///
    /// [`HashMap`]: struct.HashMap.html
    /// [`entry`]: struct.HashMap.html#method.entry
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_map::{Entry, HashMap, OccupiedEntry};
    ///
    /// let mut map = HashMap::new();
    /// map.extend([("a", 10), ("b", 20), ("c", 30)]);
    /// assert_eq!(map.len(), 3);
    ///
    /// // Existing key (insert)
    /// let entry: Entry<_, _, _> = map.entry("a");
    /// let _raw_o: OccupiedEntry<_, _, _> = entry.insert(1);
    /// assert_eq!(map.len(), 3);
    /// // Nonexistent key (insert)
    /// map.entry("d").insert(4);
    ///
    /// // Existing key (or_insert)
    /// let v = map.entry("b").or_insert(2);
    /// assert_eq!(std::mem::replace(v, 2), 20);
    /// // Nonexistent key (or_insert)
    /// map.entry("e").or_insert(5);
    ///
    /// // Existing key (or_insert_with)
    /// let v = map.entry("c").or_insert_with(|| 3);
    /// assert_eq!(std::mem::replace(v, 3), 30);
    /// // Nonexistent key (or_insert_with)
    /// map.entry("f").or_insert_with(|| 6);
    ///
    /// println!("Our HashMap: {:?}", map);
    ///
    /// let mut vec: Vec<_> = map.iter().map(|(&k, &v)| (k, v)).collect();
    /// // The `Iter` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, [("a", 1), ("b", 2), ("c", 3), ("d", 4), ("e", 5), ("f", 6)]);
    /// ```
    pub enum Entry<'a, K, V, S, A = Global>
    where
        A: Allocator,
    {
        /// An occupied entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{Entry, HashMap};
        /// let mut map: HashMap<_, _> = [("a", 100), ("b", 200)].into();
        ///
        /// match map.entry("a") {
        ///     Entry::Vacant(_) => unreachable!(),
        ///     Entry::Occupied(_) => { }
        /// }
        /// ```
        Occupied(OccupiedEntry<'a, K, V, S, A>),
        /// A vacant entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{Entry, HashMap};
        /// let mut map: HashMap<&str, i32> = HashMap::new();
        ///
        /// match map.entry("a") {
        ///     Entry::Occupied(_) => unreachable!(),
        ///     Entry::Vacant(_) => { }
        /// }
        /// ```
        Vacant(VacantEntry<'a, K, V, S, A>),
    }
    impl<K: Debug, V: Debug, S, A: Allocator> Debug for Entry<'_, K, V, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
                Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
            }
        }
    }
    /// A view into an occupied entry in a [`HashMap`].
    /// It is part of the [`Entry`] and [`EntryRef`] enums.
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_map::{Entry, HashMap, OccupiedEntry};
    ///
    /// let mut map = HashMap::new();
    /// map.extend([("a", 10), ("b", 20), ("c", 30)]);
    ///
    /// let _entry_o: OccupiedEntry<_, _, _> = map.entry("a").insert(100);
    /// assert_eq!(map.len(), 3);
    ///
    /// // Existing key (insert and update)
    /// match map.entry("a") {
    ///     Entry::Vacant(_) => unreachable!(),
    ///     Entry::Occupied(mut view) => {
    ///         assert_eq!(view.get(), &100);
    ///         let v = view.get_mut();
    ///         *v *= 10;
    ///         assert_eq!(view.insert(1111), 1000);
    ///     }
    /// }
    ///
    /// assert_eq!(map[&"a"], 1111);
    /// assert_eq!(map.len(), 3);
    ///
    /// // Existing key (take)
    /// match map.entry("c") {
    ///     Entry::Vacant(_) => unreachable!(),
    ///     Entry::Occupied(view) => {
    ///         assert_eq!(view.remove_entry(), ("c", 30));
    ///     }
    /// }
    /// assert_eq!(map.get(&"c"), None);
    /// assert_eq!(map.len(), 2);
    /// ```
    pub struct OccupiedEntry<'a, K, V, S = DefaultHashBuilder, A: Allocator = Global> {
        hash: u64,
        elem: Bucket<(K, V)>,
        table: &'a mut HashMap<K, V, S, A>,
    }
    unsafe impl<K, V, S, A> Send for OccupiedEntry<'_, K, V, S, A>
    where
        K: Send,
        V: Send,
        S: Send,
        A: Send + Allocator,
    {}
    unsafe impl<K, V, S, A> Sync for OccupiedEntry<'_, K, V, S, A>
    where
        K: Sync,
        V: Sync,
        S: Sync,
        A: Sync + Allocator,
    {}
    impl<K: Debug, V: Debug, S, A: Allocator> Debug for OccupiedEntry<'_, K, V, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OccupiedEntry")
                .field("key", self.key())
                .field("value", self.get())
                .finish()
        }
    }
    /// A view into a vacant entry in a `HashMap`.
    /// It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_map::{Entry, HashMap, VacantEntry};
    ///
    /// let mut map = HashMap::<&str, i32>::new();
    ///
    /// let entry_v: VacantEntry<_, _, _> = match map.entry("a") {
    ///     Entry::Vacant(view) => view,
    ///     Entry::Occupied(_) => unreachable!(),
    /// };
    /// entry_v.insert(10);
    /// assert!(map[&"a"] == 10 && map.len() == 1);
    ///
    /// // Nonexistent key (insert and update)
    /// match map.entry("b") {
    ///     Entry::Occupied(_) => unreachable!(),
    ///     Entry::Vacant(view) => {
    ///         let value = view.insert(2);
    ///         assert_eq!(*value, 2);
    ///         *value = 20;
    ///     }
    /// }
    /// assert!(map[&"b"] == 20 && map.len() == 2);
    /// ```
    pub struct VacantEntry<'a, K, V, S = DefaultHashBuilder, A: Allocator = Global> {
        hash: u64,
        key: K,
        table: &'a mut HashMap<K, V, S, A>,
    }
    impl<K: Debug, V, S, A: Allocator> Debug for VacantEntry<'_, K, V, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("VacantEntry").field(self.key()).finish()
        }
    }
    /// A view into a single entry in a map, which may either be vacant or occupied,
    /// with any borrowed form of the map's key type.
    ///
    ///
    /// This `enum` is constructed from the [`entry_ref`] method on [`HashMap`].
    ///
    /// [`Hash`] and [`Eq`] on the borrowed form of the map's key type *must* match those
    /// for the key type. It also require that key may be constructed from the borrowed
    /// form through the [`From`] trait.
    ///
    /// [`HashMap`]: struct.HashMap.html
    /// [`entry_ref`]: struct.HashMap.html#method.entry_ref
    /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
    /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
    /// [`From`]: https://doc.rust-lang.org/std/convert/trait.From.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_map::{EntryRef, HashMap, OccupiedEntry};
    ///
    /// let mut map = HashMap::new();
    /// map.extend([("a".to_owned(), 10), ("b".into(), 20), ("c".into(), 30)]);
    /// assert_eq!(map.len(), 3);
    ///
    /// // Existing key (insert)
    /// let key = String::from("a");
    /// let entry: EntryRef<_, _, _, _> = map.entry_ref(&key);
    /// let _raw_o: OccupiedEntry<_, _, _, _> = entry.insert(1);
    /// assert_eq!(map.len(), 3);
    /// // Nonexistent key (insert)
    /// map.entry_ref("d").insert(4);
    ///
    /// // Existing key (or_insert)
    /// let v = map.entry_ref("b").or_insert(2);
    /// assert_eq!(std::mem::replace(v, 2), 20);
    /// // Nonexistent key (or_insert)
    /// map.entry_ref("e").or_insert(5);
    ///
    /// // Existing key (or_insert_with)
    /// let v = map.entry_ref("c").or_insert_with(|| 3);
    /// assert_eq!(std::mem::replace(v, 3), 30);
    /// // Nonexistent key (or_insert_with)
    /// map.entry_ref("f").or_insert_with(|| 6);
    ///
    /// println!("Our HashMap: {:?}", map);
    ///
    /// for (key, value) in ["a", "b", "c", "d", "e", "f"].into_iter().zip(1..=6) {
    ///     assert_eq!(map[key], value)
    /// }
    /// assert_eq!(map.len(), 6);
    /// ```
    pub enum EntryRef<'a, 'b, K, Q: ?Sized, V, S, A = Global>
    where
        A: Allocator,
    {
        /// An occupied entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{EntryRef, HashMap};
        /// let mut map: HashMap<_, _> = [("a".to_owned(), 100), ("b".into(), 200)].into();
        ///
        /// match map.entry_ref("a") {
        ///     EntryRef::Vacant(_) => unreachable!(),
        ///     EntryRef::Occupied(_) => { }
        /// }
        /// ```
        Occupied(OccupiedEntry<'a, K, V, S, A>),
        /// A vacant entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{EntryRef, HashMap};
        /// let mut map: HashMap<String, i32> = HashMap::new();
        ///
        /// match map.entry_ref("a") {
        ///     EntryRef::Occupied(_) => unreachable!(),
        ///     EntryRef::Vacant(_) => { }
        /// }
        /// ```
        Vacant(VacantEntryRef<'a, 'b, K, Q, V, S, A>),
    }
    impl<K, Q, V, S, A> Debug for EntryRef<'_, '_, K, Q, V, S, A>
    where
        K: Debug + Borrow<Q>,
        Q: Debug + ?Sized,
        V: Debug,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                EntryRef::Vacant(ref v) => f.debug_tuple("EntryRef").field(v).finish(),
                EntryRef::Occupied(ref o) => f.debug_tuple("EntryRef").field(o).finish(),
            }
        }
    }
    /// A view into a vacant entry in a `HashMap`.
    /// It is part of the [`EntryRef`] enum.
    ///
    /// [`EntryRef`]: enum.EntryRef.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_map::{EntryRef, HashMap, VacantEntryRef};
    ///
    /// let mut map = HashMap::<String, i32>::new();
    ///
    /// let entry_v: VacantEntryRef<_, _, _, _> = match map.entry_ref("a") {
    ///     EntryRef::Vacant(view) => view,
    ///     EntryRef::Occupied(_) => unreachable!(),
    /// };
    /// entry_v.insert(10);
    /// assert!(map["a"] == 10 && map.len() == 1);
    ///
    /// // Nonexistent key (insert and update)
    /// match map.entry_ref("b") {
    ///     EntryRef::Occupied(_) => unreachable!(),
    ///     EntryRef::Vacant(view) => {
    ///         let value = view.insert(2);
    ///         assert_eq!(*value, 2);
    ///         *value = 20;
    ///     }
    /// }
    /// assert!(map["b"] == 20 && map.len() == 2);
    /// ```
    pub struct VacantEntryRef<'map, 'key, K, Q: ?Sized, V, S, A: Allocator = Global> {
        hash: u64,
        key: &'key Q,
        table: &'map mut HashMap<K, V, S, A>,
    }
    impl<K, Q, V, S, A> Debug for VacantEntryRef<'_, '_, K, Q, V, S, A>
    where
        K: Borrow<Q>,
        Q: Debug + ?Sized,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("VacantEntryRef").field(&self.key()).finish()
        }
    }
    /// The error returned by [`try_insert`](HashMap::try_insert) when the key already exists.
    ///
    /// Contains the occupied entry, and the value that was not inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_map::{HashMap, OccupiedError};
    ///
    /// let mut map: HashMap<_, _> = [("a", 10), ("b", 20)].into();
    ///
    /// // try_insert method returns mutable reference to the value if keys are vacant,
    /// // but if the map did have key present, nothing is updated, and the provided
    /// // value is returned inside `Err(_)` variant
    /// match map.try_insert("a", 100) {
    ///     Err(OccupiedError { mut entry, value }) => {
    ///         assert_eq!(entry.key(), &"a");
    ///         assert_eq!(value, 100);
    ///         assert_eq!(entry.insert(100), 10)
    ///     }
    ///     _ => unreachable!(),
    /// }
    /// assert_eq!(map[&"a"], 100);
    /// ```
    pub struct OccupiedError<'a, K, V, S, A: Allocator = Global> {
        /// The entry in the map that was already occupied.
        pub entry: OccupiedEntry<'a, K, V, S, A>,
        /// The value which was not inserted, because the entry was already occupied.
        pub value: V,
    }
    impl<K: Debug, V: Debug, S, A: Allocator> Debug for OccupiedError<'_, K, V, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OccupiedError")
                .field("key", self.entry.key())
                .field("old_value", self.entry.get())
                .field("new_value", &self.value)
                .finish()
        }
    }
    impl<K: Debug, V: Debug, S, A: Allocator> fmt::Display
    for OccupiedError<'_, K, V, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_fmt(
                format_args!(
                    "failed to insert {0:?}, key {1:?} already exists with value {2:?}",
                    self.value,
                    self.entry.key(),
                    self.entry.get(),
                ),
            )
        }
    }
    impl<'a, K, V, S, A: Allocator> IntoIterator for &'a HashMap<K, V, S, A> {
        type Item = (&'a K, &'a V);
        type IntoIter = Iter<'a, K, V>;
        /// Creates an iterator over the entries of a `HashMap` in arbitrary order.
        /// The iterator element type is `(&'a K, &'a V)`.
        ///
        /// Return the same `Iter` struct as by the [`iter`] method on [`HashMap`].
        ///
        /// [`iter`]: struct.HashMap.html#method.iter
        /// [`HashMap`]: struct.HashMap.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// let map_one: HashMap<_, _> = [(1, "a"), (2, "b"), (3, "c")].into();
        /// let mut map_two = HashMap::new();
        ///
        /// for (key, value) in &map_one {
        ///     println!("Key: {}, Value: {}", key, value);
        ///     map_two.insert(*key, *value);
        /// }
        ///
        /// assert_eq!(map_one, map_two);
        /// ```
        fn into_iter(self) -> Iter<'a, K, V> {
            self.iter()
        }
    }
    impl<'a, K, V, S, A: Allocator> IntoIterator for &'a mut HashMap<K, V, S, A> {
        type Item = (&'a K, &'a mut V);
        type IntoIter = IterMut<'a, K, V>;
        /// Creates an iterator over the entries of a `HashMap` in arbitrary order
        /// with mutable references to the values. The iterator element type is
        /// `(&'a K, &'a mut V)`.
        ///
        /// Return the same `IterMut` struct as by the [`iter_mut`] method on
        /// [`HashMap`].
        ///
        /// [`iter_mut`]: struct.HashMap.html#method.iter_mut
        /// [`HashMap`]: struct.HashMap.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// let mut map: HashMap<_, _> = [("a", 1), ("b", 2), ("c", 3)].into();
        ///
        /// for (key, value) in &mut map {
        ///     println!("Key: {}, Value: {}", key, value);
        ///     *value *= 2;
        /// }
        ///
        /// let mut vec = map.iter().collect::<Vec<_>>();
        /// // The `Iter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [(&"a", &2), (&"b", &4), (&"c", &6)]);
        /// ```
        fn into_iter(self) -> IterMut<'a, K, V> {
            self.iter_mut()
        }
    }
    impl<K, V, S, A: Allocator> IntoIterator for HashMap<K, V, S, A> {
        type Item = (K, V);
        type IntoIter = IntoIter<K, V, A>;
        /// Creates a consuming iterator, that is, one that moves each key-value
        /// pair out of the map in arbitrary order. The map cannot be used after
        /// calling this.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let map: HashMap<_, _> = [("a", 1), ("b", 2), ("c", 3)].into();
        ///
        /// // Not possible with .iter()
        /// let mut vec: Vec<(&str, i32)> = map.into_iter().collect();
        /// // The `IntoIter` iterator produces items in arbitrary order, so
        /// // the items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [("a", 1), ("b", 2), ("c", 3)]);
        /// ```
        fn into_iter(self) -> IntoIter<K, V, A> {
            IntoIter {
                inner: self.table.into_iter(),
            }
        }
    }
    impl<K, V> Default for Iter<'_, K, V> {
        fn default() -> Self {
            Self {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<'a, K, V> Iterator for Iter<'a, K, V> {
        type Item = (&'a K, &'a V);
        fn next(&mut self) -> Option<(&'a K, &'a V)> {
            match self.inner.next() {
                Some(x) => {
                    unsafe {
                        let r = x.as_ref();
                        Some((&r.0, &r.1))
                    }
                }
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner
                .fold(
                    init,
                    |acc, x| unsafe {
                        let (k, v) = x.as_ref();
                        f(acc, (k, v))
                    },
                )
        }
    }
    impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V> FusedIterator for Iter<'_, K, V> {}
    impl<K, V> Default for IterMut<'_, K, V> {
        fn default() -> Self {
            Self {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<'a, K, V> Iterator for IterMut<'a, K, V> {
        type Item = (&'a K, &'a mut V);
        fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
            match self.inner.next() {
                Some(x) => {
                    unsafe {
                        let r = x.as_mut();
                        Some((&r.0, &mut r.1))
                    }
                }
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner
                .fold(
                    init,
                    |acc, x| unsafe {
                        let (k, v) = x.as_mut();
                        f(acc, (k, v))
                    },
                )
        }
    }
    impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V> FusedIterator for IterMut<'_, K, V> {}
    impl<K, V> fmt::Debug for IterMut<'_, K, V>
    where
        K: fmt::Debug,
        V: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.iter()).finish()
        }
    }
    impl<K, V, A: Allocator> Default for IntoIter<K, V, A> {
        fn default() -> Self {
            Self { inner: Default::default() }
        }
    }
    impl<K, V, A: Allocator> Iterator for IntoIter<K, V, A> {
        type Item = (K, V);
        fn next(&mut self) -> Option<(K, V)> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, f)
        }
    }
    impl<K, V, A: Allocator> ExactSizeIterator for IntoIter<K, V, A> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V, A: Allocator> FusedIterator for IntoIter<K, V, A> {}
    impl<K: Debug, V: Debug, A: Allocator> fmt::Debug for IntoIter<K, V, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.iter()).finish()
        }
    }
    impl<K, V> Default for Keys<'_, K, V> {
        fn default() -> Self {
            Self { inner: Default::default() }
        }
    }
    impl<'a, K, V> Iterator for Keys<'a, K, V> {
        type Item = &'a K;
        fn next(&mut self) -> Option<&'a K> {
            match self.inner.next() {
                Some((k, _)) => Some(k),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, (k, _)| f(acc, k))
        }
    }
    impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V> FusedIterator for Keys<'_, K, V> {}
    impl<K, V> Default for Values<'_, K, V> {
        fn default() -> Self {
            Self { inner: Default::default() }
        }
    }
    impl<'a, K, V> Iterator for Values<'a, K, V> {
        type Item = &'a V;
        fn next(&mut self) -> Option<&'a V> {
            match self.inner.next() {
                Some((_, v)) => Some(v),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, (_, v)| f(acc, v))
        }
    }
    impl<K, V> ExactSizeIterator for Values<'_, K, V> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V> FusedIterator for Values<'_, K, V> {}
    impl<K, V> Default for ValuesMut<'_, K, V> {
        fn default() -> Self {
            Self { inner: Default::default() }
        }
    }
    impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
        type Item = &'a mut V;
        fn next(&mut self) -> Option<&'a mut V> {
            match self.inner.next() {
                Some((_, v)) => Some(v),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, (_, v)| f(acc, v))
        }
    }
    impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V> FusedIterator for ValuesMut<'_, K, V> {}
    impl<K, V: Debug> fmt::Debug for ValuesMut<'_, K, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.inner.iter().map(|(_, val)| val)).finish()
        }
    }
    impl<K, V, A: Allocator> Iterator for Drain<'_, K, V, A> {
        type Item = (K, V);
        fn next(&mut self) -> Option<(K, V)> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, f)
        }
    }
    impl<K, V, A: Allocator> ExactSizeIterator for Drain<'_, K, V, A> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<K, V, A: Allocator> FusedIterator for Drain<'_, K, V, A> {}
    impl<K, V, A> fmt::Debug for Drain<'_, K, V, A>
    where
        K: fmt::Debug,
        V: fmt::Debug,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.iter()).finish()
        }
    }
    impl<'a, K, V, S, A: Allocator> Entry<'a, K, V, S, A> {
        /// Sets the value of the entry, and returns an `OccupiedEntry`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// let entry = map.entry("horseyland").insert(37);
        ///
        /// assert_eq!(entry.key(), &"horseyland");
        /// ```
        pub fn insert(self, value: V) -> OccupiedEntry<'a, K, V, S, A>
        where
            K: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(mut entry) => {
                    entry.insert(value);
                    entry
                }
                Entry::Vacant(entry) => entry.insert_entry(value),
            }
        }
        /// Ensures a value is in the entry by inserting the default if empty, and returns
        /// a mutable reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry("poneyland").or_insert(3);
        /// assert_eq!(map["poneyland"], 3);
        ///
        /// // existing key
        /// *map.entry("poneyland").or_insert(10) *= 2;
        /// assert_eq!(map["poneyland"], 6);
        /// ```
        pub fn or_insert(self, default: V) -> &'a mut V
        where
            K: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(default),
            }
        }
        /// Ensures a value is in the entry by inserting the default if empty,
        /// and returns an [`OccupiedEntry`].
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// // nonexistent key
        /// let entry = map.entry("poneyland").or_insert_entry(3);
        /// assert_eq!(entry.key(), &"poneyland");
        /// assert_eq!(entry.get(), &3);
        ///
        /// // existing key
        /// let mut entry = map.entry("poneyland").or_insert_entry(10);
        /// assert_eq!(entry.key(), &"poneyland");
        /// assert_eq!(entry.get(), &3);
        /// ```
        pub fn or_insert_entry(self, default: V) -> OccupiedEntry<'a, K, V, S, A>
        where
            K: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(entry) => entry,
                Entry::Vacant(entry) => entry.insert_entry(default),
            }
        }
        /// Ensures a value is in the entry by inserting the result of the default function if empty,
        /// and returns a mutable reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry("poneyland").or_insert_with(|| 3);
        /// assert_eq!(map["poneyland"], 3);
        ///
        /// // existing key
        /// *map.entry("poneyland").or_insert_with(|| 10) *= 2;
        /// assert_eq!(map["poneyland"], 6);
        /// ```
        pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V
        where
            K: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(default()),
            }
        }
        /// Ensures a value is in the entry by inserting, if empty, the result of the default function.
        /// This method allows for generating key-derived values for insertion by providing the default
        /// function a reference to the key that was moved during the `.entry(key)` method call.
        ///
        /// The reference to the moved key is provided so that cloning or copying the key is
        /// unnecessary, unlike with `.or_insert_with(|| ... )`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, usize> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry("poneyland").or_insert_with_key(|key| key.chars().count());
        /// assert_eq!(map["poneyland"], 9);
        ///
        /// // existing key
        /// *map.entry("poneyland").or_insert_with_key(|key| key.chars().count() * 10) *= 2;
        /// assert_eq!(map["poneyland"], 18);
        /// ```
        pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V
        where
            K: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let value = default(entry.key());
                    entry.insert(value)
                }
            }
        }
        /// Returns a reference to this entry's key.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.entry("poneyland").or_insert(3);
        /// // existing key
        /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
        /// // nonexistent key
        /// assert_eq!(map.entry("horseland").key(), &"horseland");
        /// ```
        pub fn key(&self) -> &K {
            match *self {
                Entry::Occupied(ref entry) => entry.key(),
                Entry::Vacant(ref entry) => entry.key(),
            }
        }
        /// Provides in-place mutable access to an occupied entry before any
        /// potential inserts into the map.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// map.entry("poneyland")
        ///    .and_modify(|e| { *e += 1 })
        ///    .or_insert(42);
        /// assert_eq!(map["poneyland"], 42);
        ///
        /// map.entry("poneyland")
        ///    .and_modify(|e| { *e += 1 })
        ///    .or_insert(42);
        /// assert_eq!(map["poneyland"], 43);
        /// ```
        pub fn and_modify<F>(self, f: F) -> Self
        where
            F: FnOnce(&mut V),
        {
            match self {
                Entry::Occupied(mut entry) => {
                    f(entry.get_mut());
                    Entry::Occupied(entry)
                }
                Entry::Vacant(entry) => Entry::Vacant(entry),
            }
        }
        /// Provides shared access to the key and owned access to the value of
        /// an occupied entry and allows to replace or remove it based on the
        /// value of the returned option.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// let entry = map
        ///     .entry("poneyland")
        ///     .and_replace_entry_with(|_k, _v| panic!());
        ///
        /// match entry {
        ///     Entry::Vacant(e) => {
        ///         assert_eq!(e.key(), &"poneyland");
        ///     }
        ///     Entry::Occupied(_) => panic!(),
        /// }
        ///
        /// map.insert("poneyland", 42);
        ///
        /// let entry = map
        ///     .entry("poneyland")
        ///     .and_replace_entry_with(|k, v| {
        ///         assert_eq!(k, &"poneyland");
        ///         assert_eq!(v, 42);
        ///         Some(v + 1)
        ///     });
        ///
        /// match entry {
        ///     Entry::Occupied(e) => {
        ///         assert_eq!(e.key(), &"poneyland");
        ///         assert_eq!(e.get(), &43);
        ///     }
        ///     Entry::Vacant(_) => panic!(),
        /// }
        ///
        /// assert_eq!(map["poneyland"], 43);
        ///
        /// let entry = map
        ///     .entry("poneyland")
        ///     .and_replace_entry_with(|_k, _v| None);
        ///
        /// match entry {
        ///     Entry::Vacant(e) => assert_eq!(e.key(), &"poneyland"),
        ///     Entry::Occupied(_) => panic!(),
        /// }
        ///
        /// assert!(!map.contains_key("poneyland"));
        /// ```
        pub fn and_replace_entry_with<F>(self, f: F) -> Self
        where
            F: FnOnce(&K, V) -> Option<V>,
        {
            match self {
                Entry::Occupied(entry) => entry.replace_entry_with(f),
                Entry::Vacant(_) => self,
            }
        }
    }
    impl<'a, K, V: Default, S, A: Allocator> Entry<'a, K, V, S, A> {
        /// Ensures a value is in the entry by inserting the default value if empty,
        /// and returns a mutable reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, Option<u32>> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry("poneyland").or_default();
        /// assert_eq!(map["poneyland"], None);
        ///
        /// map.insert("horseland", Some(3));
        ///
        /// // existing key
        /// assert_eq!(map.entry("horseland").or_default(), &mut Some(3));
        /// ```
        pub fn or_default(self) -> &'a mut V
        where
            K: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(Default::default()),
            }
        }
    }
    impl<'a, K, V, S, A: Allocator> OccupiedEntry<'a, K, V, S, A> {
        /// Gets a reference to the key in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{Entry, HashMap};
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.entry("poneyland").or_insert(12);
        ///
        /// match map.entry("poneyland") {
        ///     Entry::Vacant(_) => panic!(),
        ///     Entry::Occupied(entry) => assert_eq!(entry.key(), &"poneyland"),
        /// }
        /// ```
        pub fn key(&self) -> &K {
            unsafe { &self.elem.as_ref().0 }
        }
        /// Take the ownership of the key and value from the map.
        /// Keeps the allocated memory for reuse.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// // The map is empty
        /// assert!(map.is_empty() && map.capacity() == 0);
        ///
        /// map.entry("poneyland").or_insert(12);
        ///
        /// if let Entry::Occupied(o) = map.entry("poneyland") {
        ///     // We delete the entry from the map.
        ///     assert_eq!(o.remove_entry(), ("poneyland", 12));
        /// }
        ///
        /// assert_eq!(map.contains_key("poneyland"), false);
        /// // Now map hold none elements
        /// assert!(map.is_empty());
        /// ```
        pub fn remove_entry(self) -> (K, V) {
            unsafe { self.table.table.remove(self.elem).0 }
        }
        /// Gets a reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.entry("poneyland").or_insert(12);
        ///
        /// match map.entry("poneyland") {
        ///     Entry::Vacant(_) => panic!(),
        ///     Entry::Occupied(entry) => assert_eq!(entry.get(), &12),
        /// }
        /// ```
        pub fn get(&self) -> &V {
            unsafe { &self.elem.as_ref().1 }
        }
        /// Gets a mutable reference to the value in the entry.
        ///
        /// If you need a reference to the `OccupiedEntry` which may outlive the
        /// destruction of the `Entry` value, see [`into_mut`].
        ///
        /// [`into_mut`]: #method.into_mut
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.entry("poneyland").or_insert(12);
        ///
        /// assert_eq!(map["poneyland"], 12);
        /// if let Entry::Occupied(mut o) = map.entry("poneyland") {
        ///     *o.get_mut() += 10;
        ///     assert_eq!(*o.get(), 22);
        ///
        ///     // We can use the same Entry multiple times.
        ///     *o.get_mut() += 2;
        /// }
        ///
        /// assert_eq!(map["poneyland"], 24);
        /// ```
        pub fn get_mut(&mut self) -> &mut V {
            unsafe { &mut self.elem.as_mut().1 }
        }
        /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry
        /// with a lifetime bound to the map itself.
        ///
        /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
        ///
        /// [`get_mut`]: #method.get_mut
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{Entry, HashMap};
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.entry("poneyland").or_insert(12);
        ///
        /// assert_eq!(map["poneyland"], 12);
        ///
        /// let value: &mut u32;
        /// match map.entry("poneyland") {
        ///     Entry::Occupied(entry) => value = entry.into_mut(),
        ///     Entry::Vacant(_) => panic!(),
        /// }
        /// *value += 10;
        ///
        /// assert_eq!(map["poneyland"], 22);
        /// ```
        pub fn into_mut(self) -> &'a mut V {
            unsafe { &mut self.elem.as_mut().1 }
        }
        /// Sets the value of the entry, and returns the entry's old value.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.entry("poneyland").or_insert(12);
        ///
        /// if let Entry::Occupied(mut o) = map.entry("poneyland") {
        ///     assert_eq!(o.insert(15), 12);
        /// }
        ///
        /// assert_eq!(map["poneyland"], 15);
        /// ```
        pub fn insert(&mut self, value: V) -> V {
            mem::replace(self.get_mut(), value)
        }
        /// Takes the value out of the entry, and returns it.
        /// Keeps the allocated memory for reuse.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// // The map is empty
        /// assert!(map.is_empty() && map.capacity() == 0);
        ///
        /// map.entry("poneyland").or_insert(12);
        ///
        /// if let Entry::Occupied(o) = map.entry("poneyland") {
        ///     assert_eq!(o.remove(), 12);
        /// }
        ///
        /// assert_eq!(map.contains_key("poneyland"), false);
        /// // Now map hold none elements
        /// assert!(map.is_empty());
        /// ```
        pub fn remove(self) -> V {
            self.remove_entry().1
        }
        /// Provides shared access to the key and owned access to the value of
        /// the entry and allows to replace or remove it based on the
        /// value of the returned option.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// map.insert("poneyland", 42);
        ///
        /// let entry = match map.entry("poneyland") {
        ///     Entry::Occupied(e) => {
        ///         e.replace_entry_with(|k, v| {
        ///             assert_eq!(k, &"poneyland");
        ///             assert_eq!(v, 42);
        ///             Some(v + 1)
        ///         })
        ///     }
        ///     Entry::Vacant(_) => panic!(),
        /// };
        ///
        /// match entry {
        ///     Entry::Occupied(e) => {
        ///         assert_eq!(e.key(), &"poneyland");
        ///         assert_eq!(e.get(), &43);
        ///     }
        ///     Entry::Vacant(_) => panic!(),
        /// }
        ///
        /// assert_eq!(map["poneyland"], 43);
        ///
        /// let entry = match map.entry("poneyland") {
        ///     Entry::Occupied(e) => e.replace_entry_with(|_k, _v| None),
        ///     Entry::Vacant(_) => panic!(),
        /// };
        ///
        /// match entry {
        ///     Entry::Vacant(e) => {
        ///         assert_eq!(e.key(), &"poneyland");
        ///     }
        ///     Entry::Occupied(_) => panic!(),
        /// }
        ///
        /// assert!(!map.contains_key("poneyland"));
        /// ```
        pub fn replace_entry_with<F>(self, f: F) -> Entry<'a, K, V, S, A>
        where
            F: FnOnce(&K, V) -> Option<V>,
        {
            unsafe {
                let mut spare_key = None;
                self.table
                    .table
                    .replace_bucket_with(
                        self.elem.clone(),
                        |(key, value)| {
                            if let Some(new_value) = f(&key, value) {
                                Some((key, new_value))
                            } else {
                                spare_key = Some(key);
                                None
                            }
                        },
                    );
                if let Some(key) = spare_key {
                    Entry::Vacant(VacantEntry {
                        hash: self.hash,
                        key,
                        table: self.table,
                    })
                } else {
                    Entry::Occupied(self)
                }
            }
        }
    }
    impl<'a, K, V, S, A: Allocator> VacantEntry<'a, K, V, S, A> {
        /// Gets a reference to the key that would be used when inserting a value
        /// through the `VacantEntry`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
        /// ```
        pub fn key(&self) -> &K {
            &self.key
        }
        /// Take ownership of the key.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::{Entry, HashMap};
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// match map.entry("poneyland") {
        ///     Entry::Occupied(_) => panic!(),
        ///     Entry::Vacant(v) => assert_eq!(v.into_key(), "poneyland"),
        /// }
        /// ```
        pub fn into_key(self) -> K {
            self.key
        }
        /// Sets the value of the entry with the [`VacantEntry`]'s key,
        /// and returns a mutable reference to it.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// if let Entry::Vacant(o) = map.entry("poneyland") {
        ///     o.insert(37);
        /// }
        /// assert_eq!(map["poneyland"], 37);
        /// ```
        pub fn insert(self, value: V) -> &'a mut V
        where
            K: Hash,
            S: BuildHasher,
        {
            let table = &mut self.table.table;
            let entry = table
                .insert_entry(
                    self.hash,
                    (self.key, value),
                    make_hasher::<_, V, S>(&self.table.hash_builder),
                );
            &mut entry.1
        }
        /// Sets the value of the entry with the [`VacantEntry`]'s key,
        /// and returns an [`OccupiedEntry`].
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::Entry;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// if let Entry::Vacant(v) = map.entry("poneyland") {
        ///     let o = v.insert_entry(37);
        ///     assert_eq!(o.get(), &37);
        /// }
        /// ```
        pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V, S, A>
        where
            K: Hash,
            S: BuildHasher,
        {
            let elem = self
                .table
                .table
                .insert(
                    self.hash,
                    (self.key, value),
                    make_hasher::<_, V, S>(&self.table.hash_builder),
                );
            OccupiedEntry {
                hash: self.hash,
                elem,
                table: self.table,
            }
        }
    }
    impl<'a, 'b, K, Q: ?Sized, V, S, A: Allocator> EntryRef<'a, 'b, K, Q, V, S, A> {
        /// Sets the value of the entry, and returns an `OccupiedEntry`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        /// let entry = map.entry_ref("horseyland").insert(37);
        ///
        /// assert_eq!(entry.key(), "horseyland");
        /// ```
        pub fn insert(self, value: V) -> OccupiedEntry<'a, K, V, S, A>
        where
            K: Hash,
            &'b Q: Into<K>,
            S: BuildHasher,
        {
            match self {
                EntryRef::Occupied(mut entry) => {
                    entry.insert(value);
                    entry
                }
                EntryRef::Vacant(entry) => entry.insert_entry(value),
            }
        }
        /// Ensures a value is in the entry by inserting the default if empty, and returns
        /// a mutable reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry_ref("poneyland").or_insert(3);
        /// assert_eq!(map["poneyland"], 3);
        ///
        /// // existing key
        /// *map.entry_ref("poneyland").or_insert(10) *= 2;
        /// assert_eq!(map["poneyland"], 6);
        /// ```
        pub fn or_insert(self, default: V) -> &'a mut V
        where
            K: Hash,
            &'b Q: Into<K>,
            S: BuildHasher,
        {
            match self {
                EntryRef::Occupied(entry) => entry.into_mut(),
                EntryRef::Vacant(entry) => entry.insert(default),
            }
        }
        /// Ensures a value is in the entry by inserting the result of the default function if empty,
        /// and returns a mutable reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry_ref("poneyland").or_insert_with(|| 3);
        /// assert_eq!(map["poneyland"], 3);
        ///
        /// // existing key
        /// *map.entry_ref("poneyland").or_insert_with(|| 10) *= 2;
        /// assert_eq!(map["poneyland"], 6);
        /// ```
        pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V
        where
            K: Hash,
            &'b Q: Into<K>,
            S: BuildHasher,
        {
            match self {
                EntryRef::Occupied(entry) => entry.into_mut(),
                EntryRef::Vacant(entry) => entry.insert(default()),
            }
        }
        /// Ensures a value is in the entry by inserting, if empty, the result of the default function.
        /// This method allows for generating key-derived values for insertion by providing the default
        /// function an access to the borrower form of the key.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, usize> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry_ref("poneyland").or_insert_with_key(|key| key.chars().count());
        /// assert_eq!(map["poneyland"], 9);
        ///
        /// // existing key
        /// *map.entry_ref("poneyland").or_insert_with_key(|key| key.chars().count() * 10) *= 2;
        /// assert_eq!(map["poneyland"], 18);
        /// ```
        pub fn or_insert_with_key<F: FnOnce(&Q) -> V>(self, default: F) -> &'a mut V
        where
            K: Hash + Borrow<Q>,
            &'b Q: Into<K>,
            S: BuildHasher,
        {
            match self {
                EntryRef::Occupied(entry) => entry.into_mut(),
                EntryRef::Vacant(entry) => {
                    let value = default(entry.key);
                    entry.insert(value)
                }
            }
        }
        /// Returns a reference to this entry's key.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        /// map.entry_ref("poneyland").or_insert(3);
        /// // existing key
        /// assert_eq!(map.entry_ref("poneyland").key(), "poneyland");
        /// // nonexistent key
        /// assert_eq!(map.entry_ref("horseland").key(), "horseland");
        /// ```
        pub fn key(&self) -> &Q
        where
            K: Borrow<Q>,
        {
            match *self {
                EntryRef::Occupied(ref entry) => entry.key().borrow(),
                EntryRef::Vacant(ref entry) => entry.key(),
            }
        }
        /// Provides in-place mutable access to an occupied entry before any
        /// potential inserts into the map.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        ///
        /// map.entry_ref("poneyland")
        ///    .and_modify(|e| { *e += 1 })
        ///    .or_insert(42);
        /// assert_eq!(map["poneyland"], 42);
        ///
        /// map.entry_ref("poneyland")
        ///    .and_modify(|e| { *e += 1 })
        ///    .or_insert(42);
        /// assert_eq!(map["poneyland"], 43);
        /// ```
        pub fn and_modify<F>(self, f: F) -> Self
        where
            F: FnOnce(&mut V),
        {
            match self {
                EntryRef::Occupied(mut entry) => {
                    f(entry.get_mut());
                    EntryRef::Occupied(entry)
                }
                EntryRef::Vacant(entry) => EntryRef::Vacant(entry),
            }
        }
    }
    impl<
        'a,
        'b,
        K,
        Q: ?Sized,
        V: Default,
        S,
        A: Allocator,
    > EntryRef<'a, 'b, K, Q, V, S, A> {
        /// Ensures a value is in the entry by inserting the default value if empty,
        /// and returns a mutable reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, Option<u32>> = HashMap::new();
        ///
        /// // nonexistent key
        /// map.entry_ref("poneyland").or_default();
        /// assert_eq!(map["poneyland"], None);
        ///
        /// map.insert("horseland".to_string(), Some(3));
        ///
        /// // existing key
        /// assert_eq!(map.entry_ref("horseland").or_default(), &mut Some(3));
        /// ```
        pub fn or_default(self) -> &'a mut V
        where
            K: Hash,
            &'b Q: Into<K>,
            S: BuildHasher,
        {
            match self {
                EntryRef::Occupied(entry) => entry.into_mut(),
                EntryRef::Vacant(entry) => entry.insert(Default::default()),
            }
        }
        /// Ensures a value is in the entry by inserting the default value if empty,
        /// and returns an [`OccupiedEntry`].
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, Option<u32>> = HashMap::new();
        ///
        /// // nonexistent key
        /// let entry = map.entry_ref("poneyland").or_default_entry();
        /// assert_eq!(entry.key(), &"poneyland");
        /// assert_eq!(entry.get(), &None);
        ///
        /// // existing key
        /// map.insert("horseland".to_string(), Some(3));
        /// let entry = map.entry_ref("horseland").or_default_entry();
        /// assert_eq!(entry.key(), &"horseland");
        /// assert_eq!(entry.get(), &Some(3));
        /// ```
        pub fn or_default_entry(self) -> OccupiedEntry<'a, K, V, S, A>
        where
            K: Hash + From<&'b Q>,
            S: BuildHasher,
        {
            match self {
                EntryRef::Occupied(entry) => entry,
                EntryRef::Vacant(entry) => entry.insert_entry(Default::default()),
            }
        }
    }
    impl<
        'map,
        'key,
        K,
        Q: ?Sized,
        V,
        S,
        A: Allocator,
    > VacantEntryRef<'map, 'key, K, Q, V, S, A> {
        /// Gets a reference to the key that would be used when inserting a value
        /// through the `VacantEntryRef`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        /// let key: &str = "poneyland";
        /// assert_eq!(map.entry_ref(key).key(), "poneyland");
        /// ```
        pub fn key(&self) -> &'key Q {
            self.key
        }
        /// Sets the value of the entry with the `VacantEntryRef`'s key,
        /// and returns a mutable reference to it.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::EntryRef;
        ///
        /// let mut map: HashMap<String, u32> = HashMap::new();
        /// let key: &str = "poneyland";
        ///
        /// if let EntryRef::Vacant(o) = map.entry_ref(key) {
        ///     o.insert(37);
        /// }
        /// assert_eq!(map["poneyland"], 37);
        /// ```
        pub fn insert(self, value: V) -> &'map mut V
        where
            K: Hash,
            &'key Q: Into<K>,
            S: BuildHasher,
        {
            let table = &mut self.table.table;
            let entry = table
                .insert_entry(
                    self.hash,
                    (self.key.into(), value),
                    make_hasher::<_, V, S>(&self.table.hash_builder),
                );
            &mut entry.1
        }
        /// Sets the key and value of the entry and returns a mutable reference to
        /// the inserted value.
        ///
        /// Unlike [`VacantEntryRef::insert`], this method allows the key to be
        /// explicitly specified, which is useful for key types that don't implement
        /// `K: From<&Q>`.
        ///
        /// # Panics
        ///
        /// This method panics if `key` is not equivalent to the key used to create
        /// the `VacantEntryRef`.
        ///
        /// # Example
        ///
        /// ```
        /// use hashbrown::hash_map::EntryRef;
        /// use hashbrown::HashMap;
        ///
        /// let mut map = HashMap::<(String, String), char>::new();
        /// let k = ("c".to_string(), "C".to_string());
        /// let v =  match map.entry_ref(&k) {
        ///   // Insert cannot be used here because tuples do not implement From.
        ///   // However this works because we can manually clone instead.
        ///   EntryRef::Vacant(r) => r.insert_with_key(k.clone(), 'c'),
        ///   // In this branch we avoid the clone.
        ///   EntryRef::Occupied(r) => r.into_mut(),
        /// };
        /// assert_eq!(*v, 'c');
        /// ```
        pub fn insert_with_key(self, key: K, value: V) -> &'map mut V
        where
            K: Hash,
            Q: Equivalent<K>,
            S: BuildHasher,
        {
            let table = &mut self.table.table;
            if !(self.key).equivalent(&key) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "key used for Entry creation is not equivalent to the one used for insertion",
                        ),
                    );
                }
            }
            let entry = table
                .insert_entry(
                    self.hash,
                    (key, value),
                    make_hasher::<_, V, S>(&self.table.hash_builder),
                );
            &mut entry.1
        }
        /// Sets the value of the entry with the [`VacantEntryRef`]'s key,
        /// and returns an [`OccupiedEntry`].
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashMap;
        /// use hashbrown::hash_map::EntryRef;
        ///
        /// let mut map: HashMap<&str, u32> = HashMap::new();
        ///
        /// if let EntryRef::Vacant(v) = map.entry_ref("poneyland") {
        ///     let o = v.insert_entry(37);
        ///     assert_eq!(o.get(), &37);
        /// }
        /// ```
        pub fn insert_entry(self, value: V) -> OccupiedEntry<'map, K, V, S, A>
        where
            K: Hash,
            &'key Q: Into<K>,
            S: BuildHasher,
        {
            let elem = self
                .table
                .table
                .insert(
                    self.hash,
                    (self.key.into(), value),
                    make_hasher::<_, V, S>(&self.table.hash_builder),
                );
            OccupiedEntry {
                hash: self.hash,
                elem,
                table: self.table,
            }
        }
    }
    impl<K, V, S, A> FromIterator<(K, V)> for HashMap<K, V, S, A>
    where
        K: Eq + Hash,
        S: BuildHasher + Default,
        A: Default + Allocator,
    {
        fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
            let iter = iter.into_iter();
            let mut map = Self::with_capacity_and_hasher_in(
                iter.size_hint().0,
                S::default(),
                A::default(),
            );
            iter.for_each(|(k, v)| {
                map.insert(k, v);
            });
            map
        }
    }
    /// Inserts all new key-values from the iterator and replaces values with existing
    /// keys with new values returned from the iterator.
    impl<K, V, S, A> Extend<(K, V)> for HashMap<K, V, S, A>
    where
        K: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        /// Inserts all new key-values from the iterator to existing `HashMap<K, V, S, A>`.
        /// Replace values with existing keys with new values returned from the iterator.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, 100);
        ///
        /// let some_iter = [(1, 1), (2, 2)].into_iter();
        /// map.extend(some_iter);
        /// // Replace values with existing keys with new values returned from the iterator.
        /// // So that the map.get(&1) doesn't return Some(&100).
        /// assert_eq!(map.get(&1), Some(&1));
        ///
        /// let some_vec: Vec<_> = vec![(3, 3), (4, 4)];
        /// map.extend(some_vec);
        ///
        /// let some_arr = [(5, 5), (6, 6)];
        /// map.extend(some_arr);
        /// let old_map_len = map.len();
        ///
        /// // You can also extend from another HashMap
        /// let mut new_map = HashMap::new();
        /// new_map.extend(map);
        /// assert_eq!(new_map.len(), old_map_len);
        ///
        /// let mut vec: Vec<_> = new_map.into_iter().collect();
        /// // The `IntoIter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)]);
        /// ```
        fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
            let iter = iter.into_iter();
            let reserve = if self.is_empty() {
                iter.size_hint().0
            } else {
                (iter.size_hint().0 + 1) / 2
            };
            self.reserve(reserve);
            iter.for_each(move |(k, v)| {
                self.insert(k, v);
            });
        }
    }
    /// Inserts all new key-values from the iterator and replaces values with existing
    /// keys with new values returned from the iterator.
    impl<'a, K, V, S, A> Extend<(&'a K, &'a V)> for HashMap<K, V, S, A>
    where
        K: Eq + Hash + Copy,
        V: Copy,
        S: BuildHasher,
        A: Allocator,
    {
        /// Inserts all new key-values from the iterator to existing `HashMap<K, V, S, A>`.
        /// Replace values with existing keys with new values returned from the iterator.
        /// The keys and values must implement [`Copy`] trait.
        ///
        /// [`Copy`]: https://doc.rust-lang.org/core/marker/trait.Copy.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, 100);
        ///
        /// let arr = [(1, 1), (2, 2)];
        /// let some_iter = arr.iter().map(|(k, v)| (k, v));
        /// map.extend(some_iter);
        /// // Replace values with existing keys with new values returned from the iterator.
        /// // So that the map.get(&1) doesn't return Some(&100).
        /// assert_eq!(map.get(&1), Some(&1));
        ///
        /// let some_vec: Vec<_> = vec![(3, 3), (4, 4)];
        /// map.extend(some_vec.iter().map(|(k, v)| (k, v)));
        ///
        /// let some_arr = [(5, 5), (6, 6)];
        /// map.extend(some_arr.iter().map(|(k, v)| (k, v)));
        ///
        /// // You can also extend from another HashMap
        /// let mut new_map = HashMap::new();
        /// new_map.extend(&map);
        /// assert_eq!(new_map, map);
        ///
        /// let mut vec: Vec<_> = new_map.into_iter().collect();
        /// // The `IntoIter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)]);
        /// ```
        fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
            self.extend(iter.into_iter().map(|(&key, &value)| (key, value)));
        }
    }
    /// Inserts all new key-values from the iterator and replaces values with existing
    /// keys with new values returned from the iterator.
    impl<'a, K, V, S, A> Extend<&'a (K, V)> for HashMap<K, V, S, A>
    where
        K: Eq + Hash + Copy,
        V: Copy,
        S: BuildHasher,
        A: Allocator,
    {
        /// Inserts all new key-values from the iterator to existing `HashMap<K, V, S, A>`.
        /// Replace values with existing keys with new values returned from the iterator.
        /// The keys and values must implement [`Copy`] trait.
        ///
        /// [`Copy`]: https://doc.rust-lang.org/core/marker/trait.Copy.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_map::HashMap;
        ///
        /// let mut map = HashMap::new();
        /// map.insert(1, 100);
        ///
        /// let arr = [(1, 1), (2, 2)];
        /// let some_iter = arr.iter();
        /// map.extend(some_iter);
        /// // Replace values with existing keys with new values returned from the iterator.
        /// // So that the map.get(&1) doesn't return Some(&100).
        /// assert_eq!(map.get(&1), Some(&1));
        ///
        /// let some_vec: Vec<_> = vec![(3, 3), (4, 4)];
        /// map.extend(&some_vec);
        ///
        /// let some_arr = [(5, 5), (6, 6)];
        /// map.extend(&some_arr);
        ///
        /// let mut vec: Vec<_> = map.into_iter().collect();
        /// // The `IntoIter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)]);
        /// ```
        fn extend<T: IntoIterator<Item = &'a (K, V)>>(&mut self, iter: T) {
            self.extend(iter.into_iter().map(|&(key, value)| (key, value)));
        }
    }
    #[allow(dead_code)]
    fn assert_covariance() {
        fn map_key<'new>(v: HashMap<&'static str, u8>) -> HashMap<&'new str, u8> {
            v
        }
        fn map_val<'new>(v: HashMap<u8, &'static str>) -> HashMap<u8, &'new str> {
            v
        }
        fn iter_key<'a, 'new>(v: Iter<'a, &'static str, u8>) -> Iter<'a, &'new str, u8> {
            v
        }
        fn iter_val<'a, 'new>(v: Iter<'a, u8, &'static str>) -> Iter<'a, u8, &'new str> {
            v
        }
        fn into_iter_key<'new, A: Allocator>(
            v: IntoIter<&'static str, u8, A>,
        ) -> IntoIter<&'new str, u8, A> {
            v
        }
        fn into_iter_val<'new, A: Allocator>(
            v: IntoIter<u8, &'static str, A>,
        ) -> IntoIter<u8, &'new str, A> {
            v
        }
        fn keys_key<'a, 'new>(v: Keys<'a, &'static str, u8>) -> Keys<'a, &'new str, u8> {
            v
        }
        fn keys_val<'a, 'new>(v: Keys<'a, u8, &'static str>) -> Keys<'a, u8, &'new str> {
            v
        }
        fn values_key<'a, 'new>(
            v: Values<'a, &'static str, u8>,
        ) -> Values<'a, &'new str, u8> {
            v
        }
        fn values_val<'a, 'new>(
            v: Values<'a, u8, &'static str>,
        ) -> Values<'a, u8, &'new str> {
            v
        }
        fn drain<'new>(
            d: Drain<'static, &'static str, &'static str>,
        ) -> Drain<'new, &'new str, &'new str> {
            d
        }
    }
}
mod scopeguard {
    use core::{
        mem::ManuallyDrop, ops::{Deref, DerefMut},
        ptr,
    };
    pub struct ScopeGuard<T, F>
    where
        F: FnMut(&mut T),
    {
        dropfn: F,
        value: T,
    }
    #[inline]
    pub fn guard<T, F>(value: T, dropfn: F) -> ScopeGuard<T, F>
    where
        F: FnMut(&mut T),
    {
        ScopeGuard { dropfn, value }
    }
    impl<T, F> ScopeGuard<T, F>
    where
        F: FnMut(&mut T),
    {
        #[inline]
        pub fn into_inner(guard: Self) -> T {
            let guard = ManuallyDrop::new(guard);
            unsafe {
                let value = ptr::read(&guard.value);
                let _ = ptr::read(&guard.dropfn);
                value
            }
        }
    }
    impl<T, F> Deref for ScopeGuard<T, F>
    where
        F: FnMut(&mut T),
    {
        type Target = T;
        #[inline]
        fn deref(&self) -> &T {
            &self.value
        }
    }
    impl<T, F> DerefMut for ScopeGuard<T, F>
    where
        F: FnMut(&mut T),
    {
        #[inline]
        fn deref_mut(&mut self) -> &mut T {
            &mut self.value
        }
    }
    impl<T, F> Drop for ScopeGuard<T, F>
    where
        F: FnMut(&mut T),
    {
        #[inline]
        fn drop(&mut self) {
            (self.dropfn)(&mut self.value);
        }
    }
}
mod set {
    use crate::{Equivalent, TryReserveError};
    use core::hash::{BuildHasher, Hash};
    use core::iter::{Chain, FusedIterator};
    use core::ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Sub, SubAssign,
    };
    use core::{fmt, mem};
    use map::make_hash;
    use super::map::{self, HashMap, Keys};
    use crate::raw::{Allocator, Global, RawExtractIf};
    use crate::DefaultHashBuilder;
    /// A hash set implemented as a `HashMap` where the value is `()`.
    ///
    /// As with the [`HashMap`] type, a `HashSet` requires that the elements
    /// implement the [`Eq`] and [`Hash`] traits. This can frequently be achieved by
    /// using `#[derive(PartialEq, Eq, Hash)]`. If you implement these yourself,
    /// it is important that the following property holds:
    ///
    /// ```text
    /// k1 == k2 -> hash(k1) == hash(k2)
    /// ```
    ///
    /// In other words, if two keys are equal, their hashes must be equal.
    ///
    ///
    /// It is a logic error for an item to be modified in such a way that the
    /// item's hash, as determined by the [`Hash`] trait, or its equality, as
    /// determined by the [`Eq`] trait, changes while it is in the set. This is
    /// normally only possible through [`Cell`], [`RefCell`], global state, I/O, or
    /// unsafe code.
    ///
    /// It is also a logic error for the [`Hash`] implementation of a key to panic.
    /// This is generally only possible if the trait is implemented manually. If a
    /// panic does occur then the contents of the `HashSet` may become corrupted and
    /// some items may be dropped from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::HashSet;
    /// // Type inference lets us omit an explicit type signature (which
    /// // would be `HashSet<String>` in this example).
    /// let mut books = HashSet::new();
    ///
    /// // Add some books.
    /// books.insert("A Dance With Dragons".to_string());
    /// books.insert("To Kill a Mockingbird".to_string());
    /// books.insert("The Odyssey".to_string());
    /// books.insert("The Great Gatsby".to_string());
    ///
    /// // Check for a specific one.
    /// if !books.contains("The Winds of Winter") {
    ///     println!("We have {} books, but The Winds of Winter ain't one.",
    ///              books.len());
    /// }
    ///
    /// // Remove a book.
    /// books.remove("The Odyssey");
    ///
    /// // Iterate over everything.
    /// for book in &books {
    ///     println!("{}", book);
    /// }
    /// ```
    ///
    /// The easiest way to use `HashSet` with a custom type is to derive
    /// [`Eq`] and [`Hash`]. We must also derive [`PartialEq`]. This will in the
    /// future be implied by [`Eq`].
    ///
    /// ```
    /// use hashbrown::HashSet;
    /// #[derive(Hash, Eq, PartialEq, Debug)]
    /// struct Viking {
    ///     name: String,
    ///     power: usize,
    /// }
    ///
    /// let mut vikings = HashSet::new();
    ///
    /// vikings.insert(Viking { name: "Einar".to_string(), power: 9 });
    /// vikings.insert(Viking { name: "Einar".to_string(), power: 9 });
    /// vikings.insert(Viking { name: "Olaf".to_string(), power: 4 });
    /// vikings.insert(Viking { name: "Harald".to_string(), power: 8 });
    ///
    /// // Use derived implementation to print the vikings.
    /// for x in &vikings {
    ///     println!("{:?}", x);
    /// }
    /// ```
    ///
    /// A `HashSet` with fixed list of elements can be initialized from an array:
    ///
    /// ```
    /// use hashbrown::HashSet;
    ///
    /// let viking_names: HashSet<&'static str> =
    ///     [ "Einar", "Olaf", "Harald" ].into_iter().collect();
    /// // use the values stored in the set
    /// ```
    ///
    /// [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
    /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
    /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
    /// [`HashMap`]: struct.HashMap.html
    /// [`PartialEq`]: https://doc.rust-lang.org/std/cmp/trait.PartialEq.html
    /// [`RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
    pub struct HashSet<T, S = DefaultHashBuilder, A: Allocator = Global> {
        pub(crate) map: HashMap<T, (), S, A>,
    }
    impl<T: Clone, S: Clone, A: Allocator + Clone> Clone for HashSet<T, S, A> {
        fn clone(&self) -> Self {
            HashSet { map: self.map.clone() }
        }
        fn clone_from(&mut self, source: &Self) {
            self.map.clone_from(&source.map);
        }
    }
    impl<T, S, A: Allocator> HashSet<T, S, A> {
        /// Returns the number of elements the set can hold without reallocating.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let set: HashSet<i32> = HashSet::with_capacity(100);
        /// assert!(set.capacity() >= 100);
        /// ```
        pub fn capacity(&self) -> usize {
            self.map.capacity()
        }
        /// An iterator visiting all elements in arbitrary order.
        /// The iterator element type is `&'a T`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let mut set = HashSet::new();
        /// set.insert("a");
        /// set.insert("b");
        ///
        /// // Will print in an arbitrary order.
        /// for x in set.iter() {
        ///     println!("{}", x);
        /// }
        /// ```
        pub fn iter(&self) -> Iter<'_, T> {
            Iter { iter: self.map.keys() }
        }
        /// Returns the number of elements in the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut v = HashSet::new();
        /// assert_eq!(v.len(), 0);
        /// v.insert(1);
        /// assert_eq!(v.len(), 1);
        /// ```
        pub fn len(&self) -> usize {
            self.map.len()
        }
        /// Returns `true` if the set contains no elements.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut v = HashSet::new();
        /// assert!(v.is_empty());
        /// v.insert(1);
        /// assert!(!v.is_empty());
        /// ```
        pub fn is_empty(&self) -> bool {
            self.map.is_empty()
        }
        /// Clears the set, returning all elements in an iterator.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// assert!(!set.is_empty());
        ///
        /// // print 1, 2, 3 in an arbitrary order
        /// for i in set.drain() {
        ///     println!("{}", i);
        /// }
        ///
        /// assert!(set.is_empty());
        /// ```
        pub fn drain(&mut self) -> Drain<'_, T, A> {
            Drain { iter: self.map.drain() }
        }
        /// Retains only the elements specified by the predicate.
        ///
        /// In other words, remove all elements `e` such that `f(&e)` returns `false`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let xs = [1,2,3,4,5,6];
        /// let mut set: HashSet<i32> = xs.into_iter().collect();
        /// set.retain(|&k| k % 2 == 0);
        /// assert_eq!(set.len(), 3);
        /// ```
        pub fn retain<F>(&mut self, mut f: F)
        where
            F: FnMut(&T) -> bool,
        {
            self.map.retain(|k, _| f(k));
        }
        /// Drains elements which are true under the given predicate,
        /// and returns an iterator over the removed items.
        ///
        /// In other words, move all elements `e` such that `f(&e)` returns `true` out
        /// into another iterator.
        ///
        /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
        /// or the iteration short-circuits, then the remaining elements will be retained.
        /// Use [`retain()`] with a negated predicate if you do not need the returned iterator.
        ///
        /// [`retain()`]: HashSet::retain
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<i32> = (0..8).collect();
        /// let drained: HashSet<i32> = set.extract_if(|v| v % 2 == 0).collect();
        ///
        /// let mut evens = drained.into_iter().collect::<Vec<_>>();
        /// let mut odds = set.into_iter().collect::<Vec<_>>();
        /// evens.sort();
        /// odds.sort();
        ///
        /// assert_eq!(evens, vec![0, 2, 4, 6]);
        /// assert_eq!(odds, vec![1, 3, 5, 7]);
        /// ```
        pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, T, F, A>
        where
            F: FnMut(&T) -> bool,
        {
            ExtractIf {
                f,
                inner: RawExtractIf {
                    iter: unsafe { self.map.table.iter() },
                    table: &mut self.map.table,
                },
            }
        }
        /// Clears the set, removing all values.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut v = HashSet::new();
        /// v.insert(1);
        /// v.clear();
        /// assert!(v.is_empty());
        /// ```
        pub fn clear(&mut self) {
            self.map.clear();
        }
    }
    impl<T, S> HashSet<T, S, Global> {
        /// Creates a new empty hash set which will use the given hasher to hash
        /// keys.
        ///
        /// The hash set is initially created with a capacity of 0, so it will not
        /// allocate until it is first inserted into.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashSet` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashSet`].
        ///
        /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
        /// the `HashSet` to be useful, see its documentation for details.
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut set = HashSet::with_hasher(s);
        /// set.insert(2);
        /// ```
        pub const fn with_hasher(hasher: S) -> Self {
            Self {
                map: HashMap::with_hasher(hasher),
            }
        }
        /// Creates an empty `HashSet` with the specified capacity, using
        /// `hasher` to hash the keys.
        ///
        /// The hash set will be able to hold at least `capacity` elements without
        /// reallocating. If `capacity` is 0, the hash set will not allocate.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashSet` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashSet`].
        ///
        /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
        /// the `HashSet` to be useful, see its documentation for details.
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut set = HashSet::with_capacity_and_hasher(10, s);
        /// set.insert(1);
        /// ```
        pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
            Self {
                map: HashMap::with_capacity_and_hasher(capacity, hasher),
            }
        }
    }
    impl<T, S, A> HashSet<T, S, A>
    where
        A: Allocator,
    {
        /// Returns a reference to the underlying allocator.
        #[inline]
        pub fn allocator(&self) -> &A {
            self.map.allocator()
        }
        /// Creates a new empty hash set which will use the given hasher to hash
        /// keys.
        ///
        /// The hash set is initially created with a capacity of 0, so it will not
        /// allocate until it is first inserted into.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashSet` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashSet`].
        ///
        /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
        /// the `HashSet` to be useful, see its documentation for details.
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut set = HashSet::with_hasher(s);
        /// set.insert(2);
        /// ```
        pub const fn with_hasher_in(hasher: S, alloc: A) -> Self {
            Self {
                map: HashMap::with_hasher_in(hasher, alloc),
            }
        }
        /// Creates an empty `HashSet` with the specified capacity, using
        /// `hasher` to hash the keys.
        ///
        /// The hash set will be able to hold at least `capacity` elements without
        /// reallocating. If `capacity` is 0, the hash set will not allocate.
        ///
        /// # HashDoS resistance
        ///
        /// The `hash_builder` normally use a fixed key by default and that does
        /// not allow the `HashSet` to be protected against attacks such as [`HashDoS`].
        /// Users who require HashDoS resistance should explicitly use
        /// [`std::collections::hash_map::RandomState`]
        /// as the hasher when creating a [`HashSet`].
        ///
        /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
        /// the `HashSet` to be useful, see its documentation for details.
        ///
        /// [`HashDoS`]: https://en.wikipedia.org/wiki/Collision_attack
        /// [`std::collections::hash_map::RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let s = DefaultHashBuilder::default();
        /// let mut set = HashSet::with_capacity_and_hasher(10, s);
        /// set.insert(1);
        /// ```
        pub fn with_capacity_and_hasher_in(
            capacity: usize,
            hasher: S,
            alloc: A,
        ) -> Self {
            Self {
                map: HashMap::with_capacity_and_hasher_in(capacity, hasher, alloc),
            }
        }
        /// Returns a reference to the set's [`BuildHasher`].
        ///
        /// [`BuildHasher`]: https://doc.rust-lang.org/std/hash/trait.BuildHasher.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::DefaultHashBuilder;
        ///
        /// let hasher = DefaultHashBuilder::default();
        /// let set: HashSet<i32> = HashSet::with_hasher(hasher);
        /// let hasher: &DefaultHashBuilder = set.hasher();
        /// ```
        pub fn hasher(&self) -> &S {
            self.map.hasher()
        }
    }
    impl<T, S, A> HashSet<T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        /// Reserves capacity for at least `additional` more elements to be inserted
        /// in the `HashSet`. The collection may reserve more space to avoid
        /// frequent reallocations.
        ///
        /// # Panics
        ///
        /// Panics if the new capacity exceeds [`isize::MAX`] bytes and [`abort`] the program
        /// in case of allocation error. Use [`try_reserve`](HashSet::try_reserve) instead
        /// if you want to handle memory allocation failure.
        ///
        /// [`isize::MAX`]: https://doc.rust-lang.org/std/primitive.isize.html
        /// [`abort`]: https://doc.rust-lang.org/alloc/alloc/fn.handle_alloc_error.html
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let mut set: HashSet<i32> = HashSet::new();
        /// set.reserve(10);
        /// assert!(set.capacity() >= 10);
        /// ```
        pub fn reserve(&mut self, additional: usize) {
            self.map.reserve(additional);
        }
        /// Tries to reserve capacity for at least `additional` more elements to be inserted
        /// in the given `HashSet<K,V>`. The collection may reserve more space to avoid
        /// frequent reallocations.
        ///
        /// # Errors
        ///
        /// If the capacity overflows, or the allocator reports a failure, then an error
        /// is returned.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let mut set: HashSet<i32> = HashSet::new();
        /// set.try_reserve(10).expect("why is the test harness OOMing on 10 bytes?");
        /// ```
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
            self.map.try_reserve(additional)
        }
        /// Shrinks the capacity of the set as much as possible. It will drop
        /// down as much as possible while maintaining the internal rules
        /// and possibly leaving some space in accordance with the resize policy.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set = HashSet::with_capacity(100);
        /// set.insert(1);
        /// set.insert(2);
        /// assert!(set.capacity() >= 100);
        /// set.shrink_to_fit();
        /// assert!(set.capacity() >= 2);
        /// ```
        pub fn shrink_to_fit(&mut self) {
            self.map.shrink_to_fit();
        }
        /// Shrinks the capacity of the set with a lower limit. It will drop
        /// down no lower than the supplied limit while maintaining the internal rules
        /// and possibly leaving some space in accordance with the resize policy.
        ///
        /// Panics if the current capacity is smaller than the supplied
        /// minimum capacity.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set = HashSet::with_capacity(100);
        /// set.insert(1);
        /// set.insert(2);
        /// assert!(set.capacity() >= 100);
        /// set.shrink_to(10);
        /// assert!(set.capacity() >= 10);
        /// set.shrink_to(0);
        /// assert!(set.capacity() >= 2);
        /// ```
        pub fn shrink_to(&mut self, min_capacity: usize) {
            self.map.shrink_to(min_capacity);
        }
        /// Visits the values representing the difference,
        /// i.e., the values that are in `self` but not in `other`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let a: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = [4, 2, 3, 4].into_iter().collect();
        ///
        /// // Can be seen as `a - b`.
        /// for x in a.difference(&b) {
        ///     println!("{}", x); // Print 1
        /// }
        ///
        /// let diff: HashSet<_> = a.difference(&b).collect();
        /// assert_eq!(diff, [1].iter().collect());
        ///
        /// // Note that difference is not symmetric,
        /// // and `b - a` means something else:
        /// let diff: HashSet<_> = b.difference(&a).collect();
        /// assert_eq!(diff, [4].iter().collect());
        /// ```
        pub fn difference<'a>(&'a self, other: &'a Self) -> Difference<'a, T, S, A> {
            Difference {
                iter: self.iter(),
                other,
            }
        }
        /// Visits the values representing the symmetric difference,
        /// i.e., the values that are in `self` or in `other` but not in both.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let a: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = [4, 2, 3, 4].into_iter().collect();
        ///
        /// // Print 1, 4 in arbitrary order.
        /// for x in a.symmetric_difference(&b) {
        ///     println!("{}", x);
        /// }
        ///
        /// let diff1: HashSet<_> = a.symmetric_difference(&b).collect();
        /// let diff2: HashSet<_> = b.symmetric_difference(&a).collect();
        ///
        /// assert_eq!(diff1, diff2);
        /// assert_eq!(diff1, [1, 4].iter().collect());
        /// ```
        pub fn symmetric_difference<'a>(
            &'a self,
            other: &'a Self,
        ) -> SymmetricDifference<'a, T, S, A> {
            SymmetricDifference {
                iter: self.difference(other).chain(other.difference(self)),
            }
        }
        /// Visits the values representing the intersection,
        /// i.e., the values that are both in `self` and `other`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let a: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = [4, 2, 3, 4].into_iter().collect();
        ///
        /// // Print 2, 3 in arbitrary order.
        /// for x in a.intersection(&b) {
        ///     println!("{}", x);
        /// }
        ///
        /// let intersection: HashSet<_> = a.intersection(&b).collect();
        /// assert_eq!(intersection, [2, 3].iter().collect());
        /// ```
        pub fn intersection<'a>(&'a self, other: &'a Self) -> Intersection<'a, T, S, A> {
            let (smaller, larger) = if self.len() <= other.len() {
                (self, other)
            } else {
                (other, self)
            };
            Intersection {
                iter: smaller.iter(),
                other: larger,
            }
        }
        /// Visits the values representing the union,
        /// i.e., all the values in `self` or `other`, without duplicates.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let a: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = [4, 2, 3, 4].into_iter().collect();
        ///
        /// // Print 1, 2, 3, 4 in arbitrary order.
        /// for x in a.union(&b) {
        ///     println!("{}", x);
        /// }
        ///
        /// let union: HashSet<_> = a.union(&b).collect();
        /// assert_eq!(union, [1, 2, 3, 4].iter().collect());
        /// ```
        pub fn union<'a>(&'a self, other: &'a Self) -> Union<'a, T, S, A> {
            let (smaller, larger) = if self.len() <= other.len() {
                (self, other)
            } else {
                (other, self)
            };
            Union {
                iter: larger.iter().chain(smaller.difference(larger)),
            }
        }
        /// Returns `true` if the set contains a value.
        ///
        /// The value may be any borrowed form of the set's value type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the value type.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let set: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// assert_eq!(set.contains(&1), true);
        /// assert_eq!(set.contains(&4), false);
        /// ```
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        pub fn contains<Q>(&self, value: &Q) -> bool
        where
            Q: Hash + Equivalent<T> + ?Sized,
        {
            self.map.contains_key(value)
        }
        /// Returns a reference to the value in the set, if any, that is equal to the given value.
        ///
        /// The value may be any borrowed form of the set's value type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the value type.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let set: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// assert_eq!(set.get(&2), Some(&2));
        /// assert_eq!(set.get(&4), None);
        /// ```
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        pub fn get<Q>(&self, value: &Q) -> Option<&T>
        where
            Q: Hash + Equivalent<T> + ?Sized,
        {
            match self.map.get_key_value(value) {
                Some((k, _)) => Some(k),
                None => None,
            }
        }
        /// Inserts the given `value` into the set if it is not present, then
        /// returns a reference to the value in the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// assert_eq!(set.len(), 3);
        /// assert_eq!(set.get_or_insert(2), &2);
        /// assert_eq!(set.get_or_insert(100), &100);
        /// assert_eq!(set.len(), 4); // 100 was inserted
        /// ```
        pub fn get_or_insert(&mut self, value: T) -> &T {
            let hash = make_hash(&self.map.hash_builder, &value);
            let bucket = match self.map.find_or_find_insert_index(hash, &value) {
                Ok(bucket) => bucket,
                Err(index) => {
                    unsafe { self.map.table.insert_at_index(hash, index, (value, ())) }
                }
            };
            unsafe { &bucket.as_ref().0 }
        }
        /// Inserts a value computed from `f` into the set if the given `value` is
        /// not present, then returns a reference to the value in the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<String> = ["cat", "dog", "horse"]
        ///     .iter().map(|&pet| pet.to_owned()).collect();
        ///
        /// assert_eq!(set.len(), 3);
        /// for &pet in &["cat", "dog", "fish"] {
        ///     let value = set.get_or_insert_with(pet, str::to_owned);
        ///     assert_eq!(value, pet);
        /// }
        /// assert_eq!(set.len(), 4); // a new "fish" was inserted
        /// ```
        ///
        /// The following example will panic because the new value doesn't match.
        ///
        /// ```should_panic
        /// let mut set = hashbrown::HashSet::new();
        /// set.get_or_insert_with("rust", |_| String::new());
        /// ```
        pub fn get_or_insert_with<Q, F>(&mut self, value: &Q, f: F) -> &T
        where
            Q: Hash + Equivalent<T> + ?Sized,
            F: FnOnce(&Q) -> T,
        {
            let hash = make_hash(&self.map.hash_builder, value);
            let bucket = match self.map.find_or_find_insert_index(hash, value) {
                Ok(bucket) => bucket,
                Err(index) => {
                    let new = f(value);
                    if !value.equivalent(&new) {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!("new value is not equivalent"),
                            );
                        }
                    }
                    unsafe { self.map.table.insert_at_index(hash, index, (new, ())) }
                }
            };
            unsafe { &bucket.as_ref().0 }
        }
        /// Gets the given value's corresponding entry in the set for in-place manipulation.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::hash_set::Entry::*;
        ///
        /// let mut singles = HashSet::new();
        /// let mut dupes = HashSet::new();
        ///
        /// for ch in "a short treatise on fungi".chars() {
        ///     if let Vacant(dupe_entry) = dupes.entry(ch) {
        ///         // We haven't already seen a duplicate, so
        ///         // check if we've at least seen it once.
        ///         match singles.entry(ch) {
        ///             Vacant(single_entry) => {
        ///                 // We found a new character for the first time.
        ///                 single_entry.insert();
        ///             }
        ///             Occupied(single_entry) => {
        ///                 // We've already seen this once, "move" it to dupes.
        ///                 single_entry.remove();
        ///                 dupe_entry.insert();
        ///             }
        ///         }
        ///     }
        /// }
        ///
        /// assert!(!singles.contains(&'t') && dupes.contains(&'t'));
        /// assert!(singles.contains(&'u') && !dupes.contains(&'u'));
        /// assert!(!singles.contains(&'v') && !dupes.contains(&'v'));
        /// ```
        pub fn entry(&mut self, value: T) -> Entry<'_, T, S, A> {
            match self.map.entry(value) {
                map::Entry::Occupied(entry) => {
                    Entry::Occupied(OccupiedEntry { inner: entry })
                }
                map::Entry::Vacant(entry) => Entry::Vacant(VacantEntry { inner: entry }),
            }
        }
        /// Returns `true` if `self` has no elements in common with `other`.
        /// This is equivalent to checking for an empty intersection.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let a: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// let mut b = HashSet::new();
        ///
        /// assert_eq!(a.is_disjoint(&b), true);
        /// b.insert(4);
        /// assert_eq!(a.is_disjoint(&b), true);
        /// b.insert(1);
        /// assert_eq!(a.is_disjoint(&b), false);
        /// ```
        pub fn is_disjoint(&self, other: &Self) -> bool {
            self.intersection(other).next().is_none()
        }
        /// Returns `true` if the set is a subset of another,
        /// i.e., `other` contains at least all the values in `self`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let sup: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// let mut set = HashSet::new();
        ///
        /// assert_eq!(set.is_subset(&sup), true);
        /// set.insert(2);
        /// assert_eq!(set.is_subset(&sup), true);
        /// set.insert(4);
        /// assert_eq!(set.is_subset(&sup), false);
        /// ```
        pub fn is_subset(&self, other: &Self) -> bool {
            self.len() <= other.len() && self.iter().all(|v| other.contains(v))
        }
        /// Returns `true` if the set is a superset of another,
        /// i.e., `self` contains at least all the values in `other`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let sub: HashSet<_> = [1, 2].into_iter().collect();
        /// let mut set = HashSet::new();
        ///
        /// assert_eq!(set.is_superset(&sub), false);
        ///
        /// set.insert(0);
        /// set.insert(1);
        /// assert_eq!(set.is_superset(&sub), false);
        ///
        /// set.insert(2);
        /// assert_eq!(set.is_superset(&sub), true);
        /// ```
        pub fn is_superset(&self, other: &Self) -> bool {
            other.is_subset(self)
        }
        /// Adds a value to the set.
        ///
        /// If the set did not have this value present, `true` is returned.
        ///
        /// If the set did have this value present, `false` is returned.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set = HashSet::new();
        ///
        /// assert_eq!(set.insert(2), true);
        /// assert_eq!(set.insert(2), false);
        /// assert_eq!(set.len(), 1);
        /// ```
        pub fn insert(&mut self, value: T) -> bool {
            self.map.insert(value, ()).is_none()
        }
        /// Insert a value the set without checking if the value already exists in the set.
        ///
        /// This operation is faster than regular insert, because it does not perform
        /// lookup before insertion.
        ///
        /// This operation is useful during initial population of the set.
        /// For example, when constructing a set from another set, we know
        /// that values are unique.
        ///
        /// # Safety
        ///
        /// This operation is safe if a value does not exist in the set.
        ///
        /// However, if a value exists in the set already, the behavior is unspecified:
        /// this operation may panic, loop forever, or any following operation with the set
        /// may panic, loop forever or return arbitrary result.
        ///
        /// That said, this operation (and following operations) are guaranteed to
        /// not violate memory safety.
        ///
        /// However this operation is still unsafe because the resulting `HashSet`
        /// may be passed to unsafe code which does expect the set to behave
        /// correctly, and would cause unsoundness as a result.
        pub unsafe fn insert_unique_unchecked(&mut self, value: T) -> &T {
            self.map.insert_unique_unchecked(value, ()).0
        }
        /// Adds a value to the set, replacing the existing value, if any, that is equal to the given
        /// one. Returns the replaced value.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set = HashSet::new();
        /// set.insert(Vec::<i32>::new());
        ///
        /// assert_eq!(set.get(&[][..]).unwrap().capacity(), 0);
        /// set.replace(Vec::with_capacity(10));
        /// assert_eq!(set.get(&[][..]).unwrap().capacity(), 10);
        /// ```
        pub fn replace(&mut self, value: T) -> Option<T> {
            let hash = make_hash(&self.map.hash_builder, &value);
            match self.map.find_or_find_insert_index(hash, &value) {
                Ok(bucket) => {
                    Some(mem::replace(unsafe { &mut bucket.as_mut().0 }, value))
                }
                Err(index) => {
                    unsafe {
                        self.map.table.insert_at_index(hash, index, (value, ()));
                    }
                    None
                }
            }
        }
        /// Removes a value from the set. Returns whether the value was
        /// present in the set.
        ///
        /// The value may be any borrowed form of the set's value type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the value type.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set = HashSet::new();
        ///
        /// set.insert(2);
        /// assert_eq!(set.remove(&2), true);
        /// assert_eq!(set.remove(&2), false);
        /// ```
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        pub fn remove<Q>(&mut self, value: &Q) -> bool
        where
            Q: Hash + Equivalent<T> + ?Sized,
        {
            self.map.remove(value).is_some()
        }
        /// Removes and returns the value in the set, if any, that is equal to the given one.
        ///
        /// The value may be any borrowed form of the set's value type, but
        /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
        /// the value type.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<_> = [1, 2, 3].into_iter().collect();
        /// assert_eq!(set.take(&2), Some(2));
        /// assert_eq!(set.take(&2), None);
        /// ```
        ///
        /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
        /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
        pub fn take<Q>(&mut self, value: &Q) -> Option<T>
        where
            Q: Hash + Equivalent<T> + ?Sized,
        {
            match self.map.remove_entry(value) {
                Some((k, _)) => Some(k),
                None => None,
            }
        }
        /// Returns the total amount of memory allocated internally by the hash
        /// set, in bytes.
        ///
        /// The returned number is informational only. It is intended to be
        /// primarily used for memory profiling.
        #[inline]
        pub fn allocation_size(&self) -> usize {
            self.map.allocation_size()
        }
    }
    impl<T, S, A> PartialEq for HashSet<T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        fn eq(&self, other: &Self) -> bool {
            if self.len() != other.len() {
                return false;
            }
            self.iter().all(|key| other.contains(key))
        }
    }
    impl<T, S, A> Eq for HashSet<T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {}
    impl<T, S, A> fmt::Debug for HashSet<T, S, A>
    where
        T: fmt::Debug,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_set().entries(self.iter()).finish()
        }
    }
    impl<T, S, A> From<HashMap<T, (), S, A>> for HashSet<T, S, A>
    where
        A: Allocator,
    {
        fn from(map: HashMap<T, (), S, A>) -> Self {
            Self { map }
        }
    }
    impl<T, S, A> FromIterator<T> for HashSet<T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher + Default,
        A: Default + Allocator,
    {
        fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
            let mut set = Self::with_hasher_in(Default::default(), Default::default());
            set.extend(iter);
            set
        }
    }
    impl<T, S, A> Extend<T> for HashSet<T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
            self.map.extend(iter.into_iter().map(|k| (k, ())));
        }
    }
    impl<'a, T, S, A> Extend<&'a T> for HashSet<T, S, A>
    where
        T: 'a + Eq + Hash + Copy,
        S: BuildHasher,
        A: Allocator,
    {
        fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
            self.extend(iter.into_iter().copied());
        }
    }
    impl<T, S, A> Default for HashSet<T, S, A>
    where
        S: Default,
        A: Default + Allocator,
    {
        /// Creates an empty `HashSet<T, S>` with the `Default` value for the hasher.
        fn default() -> Self {
            Self { map: HashMap::default() }
        }
    }
    impl<T, S, A> BitOr<&HashSet<T, S, A>> for &HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher + Default,
        A: Allocator + Default,
    {
        type Output = HashSet<T, S, A>;
        /// Returns the union of `self` and `rhs` as a new `HashSet<T, S>`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![3, 4, 5].into_iter().collect();
        ///
        /// let set = &a | &b;
        ///
        /// let mut i = 0;
        /// let expected = [1, 2, 3, 4, 5];
        /// for x in &set {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn bitor(self, rhs: &HashSet<T, S, A>) -> HashSet<T, S, A> {
            self.union(rhs).cloned().collect()
        }
    }
    impl<T, S, A> BitAnd<&HashSet<T, S, A>> for &HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher + Default,
        A: Allocator + Default,
    {
        type Output = HashSet<T, S, A>;
        /// Returns the intersection of `self` and `rhs` as a new `HashSet<T, S>`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![2, 3, 4].into_iter().collect();
        ///
        /// let set = &a & &b;
        ///
        /// let mut i = 0;
        /// let expected = [2, 3];
        /// for x in &set {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn bitand(self, rhs: &HashSet<T, S, A>) -> HashSet<T, S, A> {
            self.intersection(rhs).cloned().collect()
        }
    }
    impl<T, S, A> BitXor<&HashSet<T, S, A>> for &HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher + Default,
        A: Allocator + Default,
    {
        type Output = HashSet<T, S, A>;
        /// Returns the symmetric difference of `self` and `rhs` as a new `HashSet<T, S>`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![3, 4, 5].into_iter().collect();
        ///
        /// let set = &a ^ &b;
        ///
        /// let mut i = 0;
        /// let expected = [1, 2, 4, 5];
        /// for x in &set {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn bitxor(self, rhs: &HashSet<T, S, A>) -> HashSet<T, S, A> {
            self.symmetric_difference(rhs).cloned().collect()
        }
    }
    impl<T, S, A> Sub<&HashSet<T, S, A>> for &HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher + Default,
        A: Allocator + Default,
    {
        type Output = HashSet<T, S, A>;
        /// Returns the difference of `self` and `rhs` as a new `HashSet<T, S>`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![3, 4, 5].into_iter().collect();
        ///
        /// let set = &a - &b;
        ///
        /// let mut i = 0;
        /// let expected = [1, 2];
        /// for x in &set {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn sub(self, rhs: &HashSet<T, S, A>) -> HashSet<T, S, A> {
            self.difference(rhs).cloned().collect()
        }
    }
    impl<T, S, A> BitOrAssign<&HashSet<T, S, A>> for HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher,
        A: Allocator,
    {
        /// Modifies this set to contain the union of `self` and `rhs`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![3, 4, 5].into_iter().collect();
        ///
        /// a |= &b;
        ///
        /// let mut i = 0;
        /// let expected = [1, 2, 3, 4, 5];
        /// for x in &a {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn bitor_assign(&mut self, rhs: &HashSet<T, S, A>) {
            for item in rhs {
                if !self.contains(item) {
                    self.insert(item.clone());
                }
            }
        }
    }
    impl<T, S, A> BitAndAssign<&HashSet<T, S, A>> for HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher,
        A: Allocator,
    {
        /// Modifies this set to contain the intersection of `self` and `rhs`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![2, 3, 4].into_iter().collect();
        ///
        /// a &= &b;
        ///
        /// let mut i = 0;
        /// let expected = [2, 3];
        /// for x in &a {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn bitand_assign(&mut self, rhs: &HashSet<T, S, A>) {
            self.retain(|item| rhs.contains(item));
        }
    }
    impl<T, S, A> BitXorAssign<&HashSet<T, S, A>> for HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher,
        A: Allocator,
    {
        /// Modifies this set to contain the symmetric difference of `self` and `rhs`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![3, 4, 5].into_iter().collect();
        ///
        /// a ^= &b;
        ///
        /// let mut i = 0;
        /// let expected = [1, 2, 4, 5];
        /// for x in &a {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn bitxor_assign(&mut self, rhs: &HashSet<T, S, A>) {
            for item in rhs {
                let hash = make_hash(&self.map.hash_builder, item);
                match self.map.find_or_find_insert_index(hash, item) {
                    Ok(bucket) => {
                        unsafe {
                            self.map.table.remove(bucket);
                        }
                    }
                    Err(index) => {
                        unsafe {
                            self.map
                                .table
                                .insert_at_index(hash, index, (item.clone(), ()));
                        }
                    }
                }
            }
        }
    }
    impl<T, S, A> SubAssign<&HashSet<T, S, A>> for HashSet<T, S, A>
    where
        T: Eq + Hash + Clone,
        S: BuildHasher,
        A: Allocator,
    {
        /// Modifies this set to contain the difference of `self` and `rhs`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut a: HashSet<_> = vec![1, 2, 3].into_iter().collect();
        /// let b: HashSet<_> = vec![3, 4, 5].into_iter().collect();
        ///
        /// a -= &b;
        ///
        /// let mut i = 0;
        /// let expected = [1, 2];
        /// for x in &a {
        ///     assert!(expected.contains(x));
        ///     i += 1;
        /// }
        /// assert_eq!(i, expected.len());
        /// ```
        fn sub_assign(&mut self, rhs: &HashSet<T, S, A>) {
            if rhs.len() < self.len() {
                for item in rhs {
                    self.remove(item);
                }
            } else {
                self.retain(|item| !rhs.contains(item));
            }
        }
    }
    /// An iterator over the items of a `HashSet`.
    ///
    /// This `struct` is created by the [`iter`] method on [`HashSet`].
    /// See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`iter`]: struct.HashSet.html#method.iter
    pub struct Iter<'a, K> {
        iter: Keys<'a, K, ()>,
    }
    /// An owning iterator over the items of a `HashSet`.
    ///
    /// This `struct` is created by the [`into_iter`] method on [`HashSet`]
    /// (provided by the `IntoIterator` trait). See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`into_iter`]: struct.HashSet.html#method.into_iter
    pub struct IntoIter<K, A: Allocator = Global> {
        iter: map::IntoIter<K, (), A>,
    }
    /// A draining iterator over the items of a `HashSet`.
    ///
    /// This `struct` is created by the [`drain`] method on [`HashSet`].
    /// See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`drain`]: struct.HashSet.html#method.drain
    pub struct Drain<'a, K, A: Allocator = Global> {
        iter: map::Drain<'a, K, (), A>,
    }
    /// A draining iterator over entries of a `HashSet` which don't satisfy the predicate `f`.
    ///
    /// This `struct` is created by the [`extract_if`] method on [`HashSet`]. See its
    /// documentation for more.
    ///
    /// [`extract_if`]: struct.HashSet.html#method.extract_if
    /// [`HashSet`]: struct.HashSet.html
    #[must_use = "Iterators are lazy unless consumed"]
    pub struct ExtractIf<'a, K, F, A: Allocator = Global> {
        f: F,
        inner: RawExtractIf<'a, (K, ()), A>,
    }
    /// A lazy iterator producing elements in the intersection of `HashSet`s.
    ///
    /// This `struct` is created by the [`intersection`] method on [`HashSet`].
    /// See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`intersection`]: struct.HashSet.html#method.intersection
    pub struct Intersection<'a, T, S, A: Allocator = Global> {
        iter: Iter<'a, T>,
        other: &'a HashSet<T, S, A>,
    }
    /// A lazy iterator producing elements in the difference of `HashSet`s.
    ///
    /// This `struct` is created by the [`difference`] method on [`HashSet`].
    /// See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`difference`]: struct.HashSet.html#method.difference
    pub struct Difference<'a, T, S, A: Allocator = Global> {
        iter: Iter<'a, T>,
        other: &'a HashSet<T, S, A>,
    }
    /// A lazy iterator producing elements in the symmetric difference of `HashSet`s.
    ///
    /// This `struct` is created by the [`symmetric_difference`] method on
    /// [`HashSet`]. See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`symmetric_difference`]: struct.HashSet.html#method.symmetric_difference
    pub struct SymmetricDifference<'a, T, S, A: Allocator = Global> {
        iter: Chain<Difference<'a, T, S, A>, Difference<'a, T, S, A>>,
    }
    /// A lazy iterator producing elements in the union of `HashSet`s.
    ///
    /// This `struct` is created by the [`union`] method on [`HashSet`].
    /// See its documentation for more.
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`union`]: struct.HashSet.html#method.union
    pub struct Union<'a, T, S, A: Allocator = Global> {
        iter: Chain<Iter<'a, T>, Difference<'a, T, S, A>>,
    }
    impl<'a, T, S, A: Allocator> IntoIterator for &'a HashSet<T, S, A> {
        type Item = &'a T;
        type IntoIter = Iter<'a, T>;
        fn into_iter(self) -> Iter<'a, T> {
            self.iter()
        }
    }
    impl<T, S, A: Allocator> IntoIterator for HashSet<T, S, A> {
        type Item = T;
        type IntoIter = IntoIter<T, A>;
        /// Creates a consuming iterator, that is, one that moves each value out
        /// of the set in arbitrary order. The set cannot be used after calling
        /// this.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// let mut set = HashSet::new();
        /// set.insert("a".to_string());
        /// set.insert("b".to_string());
        ///
        /// // Not possible to collect to a Vec<String> with a regular `.iter()`.
        /// let v: Vec<String> = set.into_iter().collect();
        ///
        /// // Will print in an arbitrary order.
        /// for x in &v {
        ///     println!("{}", x);
        /// }
        /// ```
        fn into_iter(self) -> IntoIter<T, A> {
            IntoIter {
                iter: self.map.into_iter(),
            }
        }
    }
    impl<K> Clone for Iter<'_, K> {
        fn clone(&self) -> Self {
            Iter { iter: self.iter.clone() }
        }
    }
    impl<K> Default for Iter<'_, K> {
        fn default() -> Self {
            Iter { iter: Default::default() }
        }
    }
    impl<'a, K> Iterator for Iter<'a, K> {
        type Item = &'a K;
        fn next(&mut self) -> Option<&'a K> {
            self.iter.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.fold(init, f)
        }
    }
    impl<K> ExactSizeIterator for Iter<'_, K> {
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    impl<K> FusedIterator for Iter<'_, K> {}
    impl<K: fmt::Debug> fmt::Debug for Iter<'_, K> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl<K, A: Allocator> Default for IntoIter<K, A> {
        fn default() -> Self {
            IntoIter {
                iter: Default::default(),
            }
        }
    }
    impl<K, A: Allocator> Iterator for IntoIter<K, A> {
        type Item = K;
        fn next(&mut self) -> Option<K> {
            match self.iter.next() {
                Some((k, _)) => Some(k),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.fold(init, |acc, (k, ())| f(acc, k))
        }
    }
    impl<K, A: Allocator> ExactSizeIterator for IntoIter<K, A> {
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    impl<K, A: Allocator> FusedIterator for IntoIter<K, A> {}
    impl<K: fmt::Debug, A: Allocator> fmt::Debug for IntoIter<K, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let entries_iter = self.iter.iter().map(|(k, _)| k);
            f.debug_list().entries(entries_iter).finish()
        }
    }
    impl<K, A: Allocator> Iterator for Drain<'_, K, A> {
        type Item = K;
        fn next(&mut self) -> Option<K> {
            match self.iter.next() {
                Some((k, _)) => Some(k),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.fold(init, |acc, (k, ())| f(acc, k))
        }
    }
    impl<K, A: Allocator> ExactSizeIterator for Drain<'_, K, A> {
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    impl<K, A: Allocator> FusedIterator for Drain<'_, K, A> {}
    impl<K: fmt::Debug, A: Allocator> fmt::Debug for Drain<'_, K, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let entries_iter = self.iter.iter().map(|(k, _)| k);
            f.debug_list().entries(entries_iter).finish()
        }
    }
    impl<K, F, A: Allocator> Iterator for ExtractIf<'_, K, F, A>
    where
        F: FnMut(&K) -> bool,
    {
        type Item = K;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next(|&mut (ref k, ())| (self.f)(k)).map(|(k, ())| k)
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.inner.iter.size_hint().1)
        }
    }
    impl<K, F, A: Allocator> FusedIterator for ExtractIf<'_, K, F, A>
    where
        F: FnMut(&K) -> bool,
    {}
    impl<T, S, A: Allocator> Clone for Intersection<'_, T, S, A> {
        fn clone(&self) -> Self {
            Intersection {
                iter: self.iter.clone(),
                ..*self
            }
        }
    }
    impl<'a, T, S, A> Iterator for Intersection<'a, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        type Item = &'a T;
        fn next(&mut self) -> Option<&'a T> {
            loop {
                let elt = self.iter.next()?;
                if self.other.contains(elt) {
                    return Some(elt);
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (_, upper) = self.iter.size_hint();
            (0, upper)
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter
                .fold(
                    init,
                    |acc, elt| {
                        if self.other.contains(elt) { f(acc, elt) } else { acc }
                    },
                )
        }
    }
    impl<T, S, A> fmt::Debug for Intersection<'_, T, S, A>
    where
        T: fmt::Debug + Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl<T, S, A> FusedIterator for Intersection<'_, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {}
    impl<T, S, A: Allocator> Clone for Difference<'_, T, S, A> {
        fn clone(&self) -> Self {
            Difference {
                iter: self.iter.clone(),
                ..*self
            }
        }
    }
    impl<'a, T, S, A> Iterator for Difference<'a, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        type Item = &'a T;
        fn next(&mut self) -> Option<&'a T> {
            loop {
                let elt = self.iter.next()?;
                if !self.other.contains(elt) {
                    return Some(elt);
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (lower, upper) = self.iter.size_hint();
            (lower.saturating_sub(self.other.len()), upper)
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter
                .fold(
                    init,
                    |acc, elt| {
                        if self.other.contains(elt) { acc } else { f(acc, elt) }
                    },
                )
        }
    }
    impl<T, S, A> FusedIterator for Difference<'_, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {}
    impl<T, S, A> fmt::Debug for Difference<'_, T, S, A>
    where
        T: fmt::Debug + Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl<T, S, A: Allocator> Clone for SymmetricDifference<'_, T, S, A> {
        fn clone(&self) -> Self {
            SymmetricDifference {
                iter: self.iter.clone(),
            }
        }
    }
    impl<'a, T, S, A> Iterator for SymmetricDifference<'a, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        type Item = &'a T;
        fn next(&mut self) -> Option<&'a T> {
            self.iter.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.fold(init, f)
        }
    }
    impl<T, S, A> FusedIterator for SymmetricDifference<'_, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {}
    impl<T, S, A> fmt::Debug for SymmetricDifference<'_, T, S, A>
    where
        T: fmt::Debug + Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl<T, S, A: Allocator> Clone for Union<'_, T, S, A> {
        fn clone(&self) -> Self {
            Union { iter: self.iter.clone() }
        }
    }
    impl<T, S, A> FusedIterator for Union<'_, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {}
    impl<T, S, A> fmt::Debug for Union<'_, T, S, A>
    where
        T: fmt::Debug + Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl<'a, T, S, A> Iterator for Union<'a, T, S, A>
    where
        T: Eq + Hash,
        S: BuildHasher,
        A: Allocator,
    {
        type Item = &'a T;
        fn next(&mut self) -> Option<&'a T> {
            self.iter.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.fold(init, f)
        }
    }
    /// A view into a single entry in a set, which may either be vacant or occupied.
    ///
    /// This `enum` is constructed from the [`entry`] method on [`HashSet`].
    ///
    /// [`HashSet`]: struct.HashSet.html
    /// [`entry`]: struct.HashSet.html#method.entry
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_set::{Entry, HashSet, OccupiedEntry};
    ///
    /// let mut set = HashSet::new();
    /// set.extend(["a", "b", "c"]);
    /// assert_eq!(set.len(), 3);
    ///
    /// // Existing value (insert)
    /// let entry: Entry<_, _> = set.entry("a");
    /// let _raw_o: OccupiedEntry<_, _> = entry.insert();
    /// assert_eq!(set.len(), 3);
    /// // Nonexistent value (insert)
    /// set.entry("d").insert();
    ///
    /// // Existing value (or_insert)
    /// set.entry("b").or_insert();
    /// // Nonexistent value (or_insert)
    /// set.entry("e").or_insert();
    ///
    /// println!("Our HashSet: {:?}", set);
    ///
    /// let mut vec: Vec<_> = set.iter().copied().collect();
    /// // The `Iter` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, ["a", "b", "c", "d", "e"]);
    /// ```
    pub enum Entry<'a, T, S, A = Global>
    where
        A: Allocator,
    {
        /// An occupied entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_set::{Entry, HashSet};
        /// let mut set: HashSet<_> = ["a", "b"].into();
        ///
        /// match set.entry("a") {
        ///     Entry::Vacant(_) => unreachable!(),
        ///     Entry::Occupied(_) => { }
        /// }
        /// ```
        Occupied(OccupiedEntry<'a, T, S, A>),
        /// A vacant entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_set::{Entry, HashSet};
        /// let mut set: HashSet<&str> = HashSet::new();
        ///
        /// match set.entry("a") {
        ///     Entry::Occupied(_) => unreachable!(),
        ///     Entry::Vacant(_) => { }
        /// }
        /// ```
        Vacant(VacantEntry<'a, T, S, A>),
    }
    impl<T: fmt::Debug, S, A: Allocator> fmt::Debug for Entry<'_, T, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
                Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
            }
        }
    }
    /// A view into an occupied entry in a `HashSet`.
    /// It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_set::{Entry, HashSet, OccupiedEntry};
    ///
    /// let mut set = HashSet::new();
    /// set.extend(["a", "b", "c"]);
    ///
    /// let _entry_o: OccupiedEntry<_, _> = set.entry("a").insert();
    /// assert_eq!(set.len(), 3);
    ///
    /// // Existing key
    /// match set.entry("a") {
    ///     Entry::Vacant(_) => unreachable!(),
    ///     Entry::Occupied(view) => {
    ///         assert_eq!(view.get(), &"a");
    ///     }
    /// }
    ///
    /// assert_eq!(set.len(), 3);
    ///
    /// // Existing key (take)
    /// match set.entry("c") {
    ///     Entry::Vacant(_) => unreachable!(),
    ///     Entry::Occupied(view) => {
    ///         assert_eq!(view.remove(), "c");
    ///     }
    /// }
    /// assert_eq!(set.get(&"c"), None);
    /// assert_eq!(set.len(), 2);
    /// ```
    pub struct OccupiedEntry<'a, T, S, A: Allocator = Global> {
        inner: map::OccupiedEntry<'a, T, (), S, A>,
    }
    impl<T: fmt::Debug, S, A: Allocator> fmt::Debug for OccupiedEntry<'_, T, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OccupiedEntry").field("value", self.get()).finish()
        }
    }
    /// A view into a vacant entry in a `HashSet`.
    /// It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    ///
    /// # Examples
    ///
    /// ```
    /// use hashbrown::hash_set::{Entry, HashSet, VacantEntry};
    ///
    /// let mut set = HashSet::<&str>::new();
    ///
    /// let entry_v: VacantEntry<_, _> = match set.entry("a") {
    ///     Entry::Vacant(view) => view,
    ///     Entry::Occupied(_) => unreachable!(),
    /// };
    /// entry_v.insert();
    /// assert!(set.contains("a") && set.len() == 1);
    ///
    /// // Nonexistent key (insert)
    /// match set.entry("b") {
    ///     Entry::Vacant(view) => { view.insert(); },
    ///     Entry::Occupied(_) => unreachable!(),
    /// }
    /// assert!(set.contains("b") && set.len() == 2);
    /// ```
    pub struct VacantEntry<'a, T, S, A: Allocator = Global> {
        inner: map::VacantEntry<'a, T, (), S, A>,
    }
    impl<T: fmt::Debug, S, A: Allocator> fmt::Debug for VacantEntry<'_, T, S, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("VacantEntry").field(self.get()).finish()
        }
    }
    impl<'a, T, S, A: Allocator> Entry<'a, T, S, A> {
        /// Sets the value of the entry, and returns an `OccupiedEntry`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        /// let entry = set.entry("horseyland").insert();
        ///
        /// assert_eq!(entry.get(), &"horseyland");
        /// ```
        pub fn insert(self) -> OccupiedEntry<'a, T, S, A>
        where
            T: Hash,
            S: BuildHasher,
        {
            match self {
                Entry::Occupied(entry) => entry,
                Entry::Vacant(entry) => entry.insert(),
            }
        }
        /// Ensures a value is in the entry by inserting if it was vacant.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        ///
        /// // nonexistent key
        /// set.entry("poneyland").or_insert();
        /// assert!(set.contains("poneyland"));
        ///
        /// // existing key
        /// set.entry("poneyland").or_insert();
        /// assert!(set.contains("poneyland"));
        /// assert_eq!(set.len(), 1);
        /// ```
        pub fn or_insert(self)
        where
            T: Hash,
            S: BuildHasher,
        {
            if let Entry::Vacant(entry) = self {
                entry.insert();
            }
        }
        /// Returns a reference to this entry's value.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        /// set.entry("poneyland").or_insert();
        /// // existing key
        /// assert_eq!(set.entry("poneyland").get(), &"poneyland");
        /// // nonexistent key
        /// assert_eq!(set.entry("horseland").get(), &"horseland");
        /// ```
        pub fn get(&self) -> &T {
            match *self {
                Entry::Occupied(ref entry) => entry.get(),
                Entry::Vacant(ref entry) => entry.get(),
            }
        }
    }
    impl<T, S, A: Allocator> OccupiedEntry<'_, T, S, A> {
        /// Gets a reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_set::{Entry, HashSet};
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        /// set.entry("poneyland").or_insert();
        ///
        /// match set.entry("poneyland") {
        ///     Entry::Vacant(_) => panic!(),
        ///     Entry::Occupied(entry) => assert_eq!(entry.get(), &"poneyland"),
        /// }
        /// ```
        pub fn get(&self) -> &T {
            self.inner.key()
        }
        /// Takes the value out of the entry, and returns it.
        /// Keeps the allocated memory for reuse.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::hash_set::Entry;
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        /// // The set is empty
        /// assert!(set.is_empty() && set.capacity() == 0);
        ///
        /// set.entry("poneyland").or_insert();
        /// let capacity_before_remove = set.capacity();
        ///
        /// if let Entry::Occupied(o) = set.entry("poneyland") {
        ///     assert_eq!(o.remove(), "poneyland");
        /// }
        ///
        /// assert_eq!(set.contains("poneyland"), false);
        /// // Now set hold none elements but capacity is equal to the old one
        /// assert!(set.len() == 0 && set.capacity() == capacity_before_remove);
        /// ```
        pub fn remove(self) -> T {
            self.inner.remove_entry().0
        }
    }
    impl<'a, T, S, A: Allocator> VacantEntry<'a, T, S, A> {
        /// Gets a reference to the value that would be used when inserting
        /// through the `VacantEntry`.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        /// assert_eq!(set.entry("poneyland").get(), &"poneyland");
        /// ```
        pub fn get(&self) -> &T {
            self.inner.key()
        }
        /// Take ownership of the value.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::hash_set::{Entry, HashSet};
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        ///
        /// match set.entry("poneyland") {
        ///     Entry::Occupied(_) => panic!(),
        ///     Entry::Vacant(v) => assert_eq!(v.into_value(), "poneyland"),
        /// }
        /// ```
        pub fn into_value(self) -> T {
            self.inner.into_key()
        }
        /// Sets the value of the entry with the `VacantEntry`'s value.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashSet;
        /// use hashbrown::hash_set::Entry;
        ///
        /// let mut set: HashSet<&str> = HashSet::new();
        ///
        /// if let Entry::Vacant(o) = set.entry("poneyland") {
        ///     o.insert();
        /// }
        /// assert!(set.contains("poneyland"));
        /// ```
        pub fn insert(self) -> OccupiedEntry<'a, T, S, A>
        where
            T: Hash,
            S: BuildHasher,
        {
            OccupiedEntry {
                inner: self.inner.insert_entry(()),
            }
        }
    }
    #[allow(dead_code)]
    fn assert_covariance() {
        fn set<'new>(v: HashSet<&'static str>) -> HashSet<&'new str> {
            v
        }
        fn iter<'a, 'new>(v: Iter<'a, &'static str>) -> Iter<'a, &'new str> {
            v
        }
        fn into_iter<'new, A: Allocator>(
            v: IntoIter<&'static str, A>,
        ) -> IntoIter<&'new str, A> {
            v
        }
        fn difference<'a, 'new, A: Allocator>(
            v: Difference<'a, &'static str, DefaultHashBuilder, A>,
        ) -> Difference<'a, &'new str, DefaultHashBuilder, A> {
            v
        }
        fn symmetric_difference<'a, 'new, A: Allocator>(
            v: SymmetricDifference<'a, &'static str, DefaultHashBuilder, A>,
        ) -> SymmetricDifference<'a, &'new str, DefaultHashBuilder, A> {
            v
        }
        fn intersection<'a, 'new, A: Allocator>(
            v: Intersection<'a, &'static str, DefaultHashBuilder, A>,
        ) -> Intersection<'a, &'new str, DefaultHashBuilder, A> {
            v
        }
        fn union<'a, 'new, A: Allocator>(
            v: Union<'a, &'static str, DefaultHashBuilder, A>,
        ) -> Union<'a, &'new str, DefaultHashBuilder, A> {
            v
        }
        fn drain<'new, A: Allocator>(
            d: Drain<'static, &'static str, A>,
        ) -> Drain<'new, &'new str, A> {
            d
        }
    }
}
mod table {
    use core::{fmt, iter::FusedIterator, marker::PhantomData};
    use crate::{
        control::Tag,
        raw::{
            Allocator, Bucket, FullBucketsIndices, Global, RawDrain, RawExtractIf,
            RawIntoIter, RawIter, RawIterHash, RawIterHashIndices, RawTable,
        },
        TryReserveError,
    };
    /// Low-level hash table with explicit hashing.
    ///
    /// The primary use case for this type over [`HashMap`] or [`HashSet`] is to
    /// support types that do not implement the [`Hash`] and [`Eq`] traits, but
    /// instead require additional data not contained in the key itself to compute a
    /// hash and compare two elements for equality.
    ///
    /// Examples of when this can be useful include:
    /// - An `IndexMap` implementation where indices into a `Vec` are stored as
    ///   elements in a `HashTable<usize>`. Hashing and comparing the elements
    ///   requires indexing the associated `Vec` to get the actual value referred to
    ///   by the index.
    /// - Avoiding re-computing a hash when it is already known.
    /// - Mutating the key of an element in a way that doesn't affect its hash.
    ///
    /// To achieve this, `HashTable` methods that search for an element in the table
    /// require a hash value and equality function to be explicitly passed in as
    /// arguments. The method will then iterate over the elements with the given
    /// hash and call the equality function on each of them, until a match is found.
    ///
    /// In most cases, a `HashTable` will not be exposed directly in an API. It will
    /// instead be wrapped in a helper type which handles the work of calculating
    /// hash values and comparing elements.
    ///
    /// Due to its low-level nature, this type provides fewer guarantees than
    /// [`HashMap`] and [`HashSet`]. Specifically, the API allows you to shoot
    /// yourself in the foot by having multiple elements with identical keys in the
    /// table. The table itself will still function correctly and lookups will
    /// arbitrarily return one of the matching elements. However you should avoid
    /// doing this because it changes the runtime of hash table operations from
    /// `O(1)` to `O(k)` where `k` is the number of duplicate entries.
    ///
    /// [`HashMap`]: super::HashMap
    /// [`HashSet`]: super::HashSet
    /// [`Eq`]: https://doc.rust-lang.org/std/cmp/trait.Eq.html
    /// [`Hash`]: https://doc.rust-lang.org/std/hash/trait.Hash.html
    pub struct HashTable<T, A = Global>
    where
        A: Allocator,
    {
        pub(crate) raw: RawTable<T, A>,
    }
    impl<T> HashTable<T, Global> {
        /// Creates an empty `HashTable`.
        ///
        /// The hash table is initially created with a capacity of 0, so it will not allocate until it
        /// is first inserted into.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashTable;
        /// let mut table: HashTable<&str> = HashTable::new();
        /// assert_eq!(table.len(), 0);
        /// assert_eq!(table.capacity(), 0);
        /// ```
        pub const fn new() -> Self {
            Self { raw: RawTable::new() }
        }
        /// Creates an empty `HashTable` with the specified capacity.
        ///
        /// The hash table will be able to hold at least `capacity` elements without
        /// reallocating. If `capacity` is 0, the hash table will not allocate.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashTable;
        /// let mut table: HashTable<&str> = HashTable::with_capacity(10);
        /// assert_eq!(table.len(), 0);
        /// assert!(table.capacity() >= 10);
        /// ```
        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                raw: RawTable::with_capacity(capacity),
            }
        }
    }
    impl<T, A> HashTable<T, A>
    where
        A: Allocator,
    {
        /// Creates an empty `HashTable` using the given allocator.
        ///
        /// The hash table is initially created with a capacity of 0, so it will not allocate until it
        /// is first inserted into.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use bumpalo::Bump;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let bump = Bump::new();
        /// let mut table = HashTable::new_in(&bump);
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// // The created HashTable holds none elements
        /// assert_eq!(table.len(), 0);
        ///
        /// // The created HashTable also doesn't allocate memory
        /// assert_eq!(table.capacity(), 0);
        ///
        /// // Now we insert element inside created HashTable
        /// table.insert_unique(hasher(&"One"), "One", hasher);
        /// // We can see that the HashTable holds 1 element
        /// assert_eq!(table.len(), 1);
        /// // And it also allocates some capacity
        /// assert!(table.capacity() > 1);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub const fn new_in(alloc: A) -> Self {
            Self {
                raw: RawTable::new_in(alloc),
            }
        }
        /// Creates an empty `HashTable` with the specified capacity using the given allocator.
        ///
        /// The hash table will be able to hold at least `capacity` elements without
        /// reallocating. If `capacity` is 0, the hash table will not allocate.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use bumpalo::Bump;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let bump = Bump::new();
        /// let mut table = HashTable::with_capacity_in(5, &bump);
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// // The created HashTable holds none elements
        /// assert_eq!(table.len(), 0);
        /// // But it can hold at least 5 elements without reallocating
        /// let empty_map_capacity = table.capacity();
        /// assert!(empty_map_capacity >= 5);
        ///
        /// // Now we insert some 5 elements inside created HashTable
        /// table.insert_unique(hasher(&"One"), "One", hasher);
        /// table.insert_unique(hasher(&"Two"), "Two", hasher);
        /// table.insert_unique(hasher(&"Three"), "Three", hasher);
        /// table.insert_unique(hasher(&"Four"), "Four", hasher);
        /// table.insert_unique(hasher(&"Five"), "Five", hasher);
        ///
        /// // We can see that the HashTable holds 5 elements
        /// assert_eq!(table.len(), 5);
        /// // But its capacity isn't changed
        /// assert_eq!(table.capacity(), empty_map_capacity)
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
            Self {
                raw: RawTable::with_capacity_in(capacity, alloc),
            }
        }
        /// Returns a reference to the underlying allocator.
        pub fn allocator(&self) -> &A {
            self.raw.allocator()
        }
        /// Returns a reference to an entry in the table with the given hash and
        /// which satisfies the equality function passed.
        ///
        /// This method will call `eq` for all entries with the given hash, but may
        /// also call it for entries with a different hash. `eq` should only return
        /// true for the desired entry, at which point the search is stopped.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), 1, hasher);
        /// table.insert_unique(hasher(&2), 2, hasher);
        /// table.insert_unique(hasher(&3), 3, hasher);
        /// assert_eq!(table.find(hasher(&2), |&val| val == 2), Some(&2));
        /// assert_eq!(table.find(hasher(&4), |&val| val == 4), None);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn find(&self, hash: u64, eq: impl FnMut(&T) -> bool) -> Option<&T> {
            self.raw.get(hash, eq)
        }
        /// Returns a mutable reference to an entry in the table with the given hash
        /// and which satisfies the equality function passed.
        ///
        /// This method will call `eq` for all entries with the given hash, but may
        /// also call it for entries with a different hash. `eq` should only return
        /// true for the desired entry, at which point the search is stopped.
        ///
        /// When mutating an entry, you should ensure that it still retains the same
        /// hash value as when it was inserted, otherwise lookups of that entry may
        /// fail to find it.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, "a"), |val| hasher(&val.0));
        /// if let Some(val) = table.find_mut(hasher(&1), |val| val.0 == 1) {
        ///     val.1 = "b";
        /// }
        /// assert_eq!(table.find(hasher(&1), |val| val.0 == 1), Some(&(1, "b")));
        /// assert_eq!(table.find(hasher(&2), |val| val.0 == 2), None);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn find_mut(
            &mut self,
            hash: u64,
            eq: impl FnMut(&T) -> bool,
        ) -> Option<&mut T> {
            self.raw.get_mut(hash, eq)
        }
        /// Returns an `OccupiedEntry` for an entry in the table with the given hash
        /// and which satisfies the equality function passed.
        ///
        /// This can be used to remove the entry from the table. Call
        /// [`HashTable::entry`] instead if you wish to insert an entry if the
        /// lookup fails.
        ///
        /// This method will call `eq` for all entries with the given hash, but may
        /// also call it for entries with a different hash. `eq` should only return
        /// true for the desired entry, at which point the search is stopped.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, "a"), |val| hasher(&val.0));
        /// if let Ok(entry) = table.find_entry(hasher(&1), |val| val.0 == 1) {
        ///     entry.remove();
        /// }
        /// assert_eq!(table.find(hasher(&1), |val| val.0 == 1), None);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn find_entry(
            &mut self,
            hash: u64,
            eq: impl FnMut(&T) -> bool,
        ) -> Result<OccupiedEntry<'_, T, A>, AbsentEntry<'_, T, A>> {
            match self.raw.find(hash, eq) {
                Some(bucket) => {
                    Ok(OccupiedEntry {
                        bucket,
                        table: self,
                    })
                }
                None => Err(AbsentEntry { table: self }),
            }
        }
        /// Returns the bucket index in the table for an entry with the given hash
        /// and which satisfies the equality function passed.
        ///
        /// This can be used to store a borrow-free "reference" to the entry, later using
        /// [`get_bucket`][Self::get_bucket], [`get_bucket_mut`][Self::get_bucket_mut], or
        /// [`get_bucket_entry`][Self::get_bucket_entry] to access it again without hash probing.
        ///
        /// The index is only meaningful as long as the table is not resized and no entries are added
        /// or removed. After such changes, it may end up pointing to a different entry or none at all.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 1), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 2), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 3), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        /// assert_eq!(table.get_bucket(index), Some(&(2, 2)));
        ///
        /// // Mutation would invalidate any normal reference
        /// for (_key, value) in &mut table {
        ///     *value *= 11;
        /// }
        ///
        /// // The index still reaches the same key with the updated value
        /// assert_eq!(table.get_bucket(index), Some(&(2, 22)));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn find_bucket_index(
            &self,
            hash: u64,
            eq: impl FnMut(&T) -> bool,
        ) -> Option<usize> {
            match self.raw.find(hash, eq) {
                Some(bucket) => Some(unsafe { self.raw.bucket_index(&bucket) }),
                None => None,
            }
        }
        /// Returns an `Entry` for an entry in the table with the given hash
        /// and which satisfies the equality function passed.
        ///
        /// This can be used to remove the entry from the table, or insert a new
        /// entry with the given hash if one doesn't already exist.
        ///
        /// This method will call `eq` for all entries with the given hash, but may
        /// also call it for entries with a different hash. `eq` should only return
        /// true for the desired entry, at which point the search is stopped.
        ///
        /// This method may grow the table in preparation for an insertion. Call
        /// [`HashTable::find_entry`] if this is undesirable.
        ///
        /// `hasher` is called if entries need to be moved or copied to a new table.
        /// This must return the same hash value that each entry was inserted with.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, "a"), |val| hasher(&val.0));
        /// if let Entry::Occupied(entry) = table.entry(hasher(&1), |val| val.0 == 1, |val| hasher(&val.0))
        /// {
        ///     entry.remove();
        /// }
        /// if let Entry::Vacant(entry) = table.entry(hasher(&2), |val| val.0 == 2, |val| hasher(&val.0)) {
        ///     entry.insert((2, "b"));
        /// }
        /// assert_eq!(table.find(hasher(&1), |val| val.0 == 1), None);
        /// assert_eq!(table.find(hasher(&2), |val| val.0 == 2), Some(&(2, "b")));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn entry(
            &mut self,
            hash: u64,
            eq: impl FnMut(&T) -> bool,
            hasher: impl Fn(&T) -> u64,
        ) -> Entry<'_, T, A> {
            match self.raw.find_or_find_insert_index(hash, eq, hasher) {
                Ok(bucket) => {
                    Entry::Occupied(OccupiedEntry {
                        bucket,
                        table: self,
                    })
                }
                Err(insert_index) => {
                    Entry::Vacant(VacantEntry {
                        tag: Tag::full(hash),
                        index: insert_index,
                        table: self,
                    })
                }
            }
        }
        /// Returns an `OccupiedEntry` for the given bucket index in the table,
        /// or `AbsentEntry` if it is unoccupied or out of bounds.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        ///
        /// assert!(table.get_bucket_entry(usize::MAX).is_err());
        ///
        /// let occupied_entry = table.get_bucket_entry(index).unwrap();
        /// assert_eq!(occupied_entry.get(), &(2, 'b'));
        /// assert_eq!(occupied_entry.remove().0, (2, 'b'));
        ///
        /// assert!(table.find(hasher(&2), |val| val.0 == 2).is_none());
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub fn get_bucket_entry(
            &mut self,
            index: usize,
        ) -> Result<OccupiedEntry<'_, T, A>, AbsentEntry<'_, T, A>> {
            match self.raw.checked_bucket(index) {
                Some(bucket) => {
                    Ok(OccupiedEntry {
                        bucket,
                        table: self,
                    })
                }
                None => Err(AbsentEntry { table: self }),
            }
        }
        /// Returns an `OccupiedEntry` for the given bucket index in the table,
        /// without checking whether the index is in-bounds or occupied.
        ///
        /// For a safe alternative, see [`get_bucket_entry`](Self::get_bucket_entry).
        ///
        /// # Safety
        ///
        /// It is *[undefined behavior]* to call this method with an index that is
        /// out-of-bounds or unoccupied, even if the resulting entry is not used.
        ///
        /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        /// assert!(std::ptr::eq(
        ///     table.get_bucket_entry(index).unwrap().into_mut(),
        ///     unsafe { table.get_bucket_entry_unchecked(index).into_mut() },
        /// ));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub unsafe fn get_bucket_entry_unchecked(
            &mut self,
            index: usize,
        ) -> OccupiedEntry<'_, T, A> {
            OccupiedEntry {
                bucket: self.raw.bucket(index),
                table: self,
            }
        }
        /// Gets a reference to an entry in the table at the given bucket index,
        /// or `None` if it is unoccupied or out of bounds.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        /// assert_eq!(table.get_bucket(index), Some(&(2, 'b')));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub fn get_bucket(&self, index: usize) -> Option<&T> {
            self.raw.get_bucket(index)
        }
        /// Gets a reference to an entry in the table at the given bucket index,
        /// without checking whether the index is in-bounds or occupied.
        ///
        /// For a safe alternative, see [`get_bucket`](Self::get_bucket).
        ///
        /// # Safety
        ///
        /// It is *[undefined behavior]* to call this method with an index that is
        /// out-of-bounds or unoccupied, even if the resulting reference is not used.
        ///
        /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        /// assert!(std::ptr::eq(
        ///     table.get_bucket(index).unwrap(),
        ///     unsafe { table.get_bucket_unchecked(index) },
        /// ));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub unsafe fn get_bucket_unchecked(&self, index: usize) -> &T {
            self.raw.bucket(index).as_ref()
        }
        /// Gets a mutable reference to an entry in the table at the given bucket index,
        /// or `None` if it is unoccupied or out of bounds.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        /// assert_eq!(table.get_bucket(index), Some(&(2, 'b')));
        /// if let Some((_key, value)) = table.get_bucket_mut(index) {
        ///     *value = 'B';
        /// }
        /// assert_eq!(table.get_bucket(index), Some(&(2, 'B')));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub fn get_bucket_mut(&mut self, index: usize) -> Option<&mut T> {
            self.raw.get_bucket_mut(index)
        }
        /// Gets a mutable reference to an entry in the table at the given bucket index,
        /// without checking whether the index is in-bounds or occupied.
        ///
        /// For a safe alternative, see [`get_bucket_mut`](Self::get_bucket_mut).
        ///
        /// # Safety
        ///
        /// It is *[undefined behavior]* to call this method with an index that is
        /// out-of-bounds or unoccupied, even if the resulting reference is not used.
        ///
        /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// let index = table.find_bucket_index(hasher(&2), |val| val.0 == 2).unwrap();
        /// assert!(std::ptr::eq(
        ///     table.get_bucket_mut(index).unwrap(),
        ///     unsafe { table.get_bucket_unchecked_mut(index) },
        /// ));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub unsafe fn get_bucket_unchecked_mut(&mut self, index: usize) -> &mut T {
            self.raw.bucket(index).as_mut()
        }
        /// Inserts an element into the `HashTable` with the given hash value, but
        /// without checking whether an equivalent element already exists within the
        /// table.
        ///
        /// `hasher` is called if entries need to be moved or copied to a new table.
        /// This must return the same hash value that each entry was inserted with.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut v = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// v.insert_unique(hasher(&1), 1, hasher);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn insert_unique(
            &mut self,
            hash: u64,
            value: T,
            hasher: impl Fn(&T) -> u64,
        ) -> OccupiedEntry<'_, T, A> {
            let bucket = self.raw.insert(hash, value, hasher);
            OccupiedEntry {
                bucket,
                table: self,
            }
        }
        /// Clears the table, removing all values.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut v = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// v.insert_unique(hasher(&1), 1, hasher);
        /// v.clear();
        /// assert!(v.is_empty());
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn clear(&mut self) {
            self.raw.clear();
        }
        /// Shrinks the capacity of the table as much as possible. It will drop
        /// down as much as possible while maintaining the internal rules
        /// and possibly leaving some space in accordance with the resize policy.
        ///
        /// `hasher` is called if entries need to be moved or copied to a new table.
        /// This must return the same hash value that each entry was inserted with.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::with_capacity(100);
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), 1, hasher);
        /// table.insert_unique(hasher(&2), 2, hasher);
        /// assert!(table.capacity() >= 100);
        /// table.shrink_to_fit(hasher);
        /// assert!(table.capacity() >= 2);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn shrink_to_fit(&mut self, hasher: impl Fn(&T) -> u64) {
            self.raw.shrink_to(self.len(), hasher)
        }
        /// Shrinks the capacity of the table with a lower limit. It will drop
        /// down no lower than the supplied limit while maintaining the internal rules
        /// and possibly leaving some space in accordance with the resize policy.
        ///
        /// `hasher` is called if entries need to be moved or copied to a new table.
        /// This must return the same hash value that each entry was inserted with.
        ///
        /// Panics if the current capacity is smaller than the supplied
        /// minimum capacity.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::with_capacity(100);
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), 1, hasher);
        /// table.insert_unique(hasher(&2), 2, hasher);
        /// assert!(table.capacity() >= 100);
        /// table.shrink_to(10, hasher);
        /// assert!(table.capacity() >= 10);
        /// table.shrink_to(0, hasher);
        /// assert!(table.capacity() >= 2);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn shrink_to(&mut self, min_capacity: usize, hasher: impl Fn(&T) -> u64) {
            self.raw.shrink_to(min_capacity, hasher);
        }
        /// Reserves capacity for at least `additional` more elements to be inserted
        /// in the `HashTable`. The collection may reserve more space to avoid
        /// frequent reallocations.
        ///
        /// `hasher` is called if entries need to be moved or copied to a new table.
        /// This must return the same hash value that each entry was inserted with.
        ///
        /// # Panics
        ///
        /// Panics if the new capacity exceeds [`isize::MAX`] bytes and [`abort`] the program
        /// in case of allocation error. Use [`try_reserve`](HashTable::try_reserve) instead
        /// if you want to handle memory allocation failure.
        ///
        /// [`isize::MAX`]: https://doc.rust-lang.org/std/primitive.isize.html
        /// [`abort`]: https://doc.rust-lang.org/alloc/alloc/fn.handle_alloc_error.html
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<i32> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.reserve(10, hasher);
        /// assert!(table.capacity() >= 10);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn reserve(&mut self, additional: usize, hasher: impl Fn(&T) -> u64) {
            self.raw.reserve(additional, hasher)
        }
        /// Tries to reserve capacity for at least `additional` more elements to be inserted
        /// in the given `HashTable`. The collection may reserve more space to avoid
        /// frequent reallocations.
        ///
        /// `hasher` is called if entries need to be moved or copied to a new table.
        /// This must return the same hash value that each entry was inserted with.
        ///
        /// # Errors
        ///
        /// If the capacity overflows, or the allocator reports a failure, then an error
        /// is returned.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<i32> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table
        ///     .try_reserve(10, hasher)
        ///     .expect("why is the test harness OOMing on 10 bytes?");
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn try_reserve(
            &mut self,
            additional: usize,
            hasher: impl Fn(&T) -> u64,
        ) -> Result<(), TryReserveError> {
            self.raw.try_reserve(additional, hasher)
        }
        /// Returns the raw number of buckets allocated in the table.
        ///
        /// This is an upper bound on any methods that take or return a bucket index,
        /// as opposed to the usable [`capacity`](Self::capacity) for entries which is
        /// reduced by an unspecified load factor.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 'a'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 'b'), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 'c'), |val| hasher(&val.0));
        ///
        /// // Each entry is available at some index in the bucket range.
        /// let count = (0..table.num_buckets())
        ///     .filter_map(|i| table.get_bucket(i))
        ///     .count();
        /// assert_eq!(count, 3);
        ///
        /// assert_eq!(table.get_bucket(table.num_buckets()), None);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn num_buckets(&self) -> usize {
            self.raw.buckets()
        }
        /// Returns the number of elements the table can hold without reallocating.
        ///
        /// # Examples
        ///
        /// ```
        /// use hashbrown::HashTable;
        /// let table: HashTable<i32> = HashTable::with_capacity(100);
        /// assert!(table.capacity() >= 100);
        /// ```
        pub fn capacity(&self) -> usize {
            self.raw.capacity()
        }
        /// Returns the number of elements in the table.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// let mut v = HashTable::new();
        /// assert_eq!(v.len(), 0);
        /// v.insert_unique(hasher(&1), 1, hasher);
        /// assert_eq!(v.len(), 1);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn len(&self) -> usize {
            self.raw.len()
        }
        /// Returns `true` if the set contains no elements.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// let mut v = HashTable::new();
        /// assert!(v.is_empty());
        /// v.insert_unique(hasher(&1), 1, hasher);
        /// assert!(!v.is_empty());
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn is_empty(&self) -> bool {
            self.raw.is_empty()
        }
        /// An iterator visiting all elements in arbitrary order.
        /// The iterator element type is `&'a T`.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"a"), "a", hasher);
        /// table.insert_unique(hasher(&"b"), "b", hasher);
        ///
        /// // Will print in an arbitrary order.
        /// for x in table.iter() {
        ///     println!("{}", x);
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn iter(&self) -> Iter<'_, T> {
            Iter {
                inner: unsafe { self.raw.iter() },
                marker: PhantomData,
            }
        }
        /// An iterator visiting all elements in arbitrary order,
        /// with mutable references to the elements.
        /// The iterator element type is `&'a mut T`.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), 1, hasher);
        /// table.insert_unique(hasher(&2), 2, hasher);
        /// table.insert_unique(hasher(&3), 3, hasher);
        ///
        /// // Update all values
        /// for val in table.iter_mut() {
        ///     *val *= 2;
        /// }
        ///
        /// assert_eq!(table.len(), 3);
        /// let mut vec: Vec<i32> = Vec::new();
        ///
        /// for val in &table {
        ///     println!("val: {}", val);
        ///     vec.push(*val);
        /// }
        ///
        /// // The `Iter` iterator produces items in arbitrary order, so the
        /// // items must be sorted to test them against a sorted array.
        /// vec.sort_unstable();
        /// assert_eq!(vec, [2, 4, 6]);
        ///
        /// assert_eq!(table.len(), 3);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn iter_mut(&mut self) -> IterMut<'_, T> {
            IterMut {
                inner: unsafe { self.raw.iter() },
                marker: PhantomData,
            }
        }
        /// An iterator producing the `usize` indices of all occupied buckets.
        ///
        /// The order in which the iterator yields indices is unspecified
        /// and may change in the future.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"a"), "a", hasher);
        /// table.insert_unique(hasher(&"b"), "b", hasher);
        ///
        /// // Will print in an arbitrary order.
        /// for index in table.iter_buckets() {
        ///     println!("{index}: {}", table.get_bucket(index).unwrap());
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn iter_buckets(&self) -> IterBuckets<'_, T> {
            IterBuckets {
                inner: unsafe { self.raw.full_buckets_indices() },
                marker: PhantomData,
            }
        }
        /// An iterator visiting all elements which may match a hash.
        /// The iterator element type is `&'a T`.
        ///
        /// This iterator may return elements from the table that have a hash value
        /// different than the one provided. You should always validate the returned
        /// values before using them.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"a"), "a", hasher);
        /// table.insert_unique(hasher(&"a"), "b", hasher);
        /// table.insert_unique(hasher(&"b"), "c", hasher);
        ///
        /// // Will print "a" and "b" (and possibly "c") in an arbitrary order.
        /// for x in table.iter_hash(hasher(&"a")) {
        ///     println!("{}", x);
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn iter_hash(&self, hash: u64) -> IterHash<'_, T> {
            IterHash {
                inner: unsafe { self.raw.iter_hash(hash) },
                marker: PhantomData,
            }
        }
        /// A mutable iterator visiting all elements which may match a hash.
        /// The iterator element type is `&'a mut T`.
        ///
        /// This iterator may return elements from the table that have a hash value
        /// different than the one provided. You should always validate the returned
        /// values before using them.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), 2, hasher);
        /// table.insert_unique(hasher(&1), 3, hasher);
        /// table.insert_unique(hasher(&2), 5, hasher);
        ///
        /// // Update matching values
        /// for val in table.iter_hash_mut(hasher(&1)) {
        ///     *val *= 2;
        /// }
        ///
        /// assert_eq!(table.len(), 3);
        /// let mut vec: Vec<i32> = Vec::new();
        ///
        /// for val in &table {
        ///     println!("val: {}", val);
        ///     vec.push(*val);
        /// }
        ///
        /// // The values will contain 4 and 6 and may contain either 5 or 10.
        /// assert!(vec.contains(&4));
        /// assert!(vec.contains(&6));
        ///
        /// assert_eq!(table.len(), 3);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn iter_hash_mut(&mut self, hash: u64) -> IterHashMut<'_, T> {
            IterHashMut {
                inner: unsafe { self.raw.iter_hash(hash) },
                marker: PhantomData,
            }
        }
        /// An iterator producing the `usize` indices of all buckets which may match a hash.
        ///
        /// This iterator may return indices from the table that have a hash value
        /// different than the one provided. You should always validate the returned
        /// values before using them.
        ///
        /// The order in which the iterator yields indices is unspecified
        /// and may change in the future.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"a"), "a", hasher);
        /// table.insert_unique(hasher(&"a"), "b", hasher);
        /// table.insert_unique(hasher(&"b"), "c", hasher);
        ///
        /// // Will print the indices with "a" and "b" (and possibly "c") in an arbitrary order.
        /// for index in table.iter_hash_buckets(hasher(&"a")) {
        ///     println!("{index}: {}", table.get_bucket(index).unwrap());
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn iter_hash_buckets(&self, hash: u64) -> IterHashBuckets<'_, T> {
            IterHashBuckets {
                inner: unsafe { self.raw.iter_hash_buckets(hash) },
                marker: PhantomData,
            }
        }
        /// Retains only the elements specified by the predicate.
        ///
        /// In other words, remove all elements `e` such that `f(&e)` returns `false`.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for x in 1..=6 {
        ///     table.insert_unique(hasher(&x), x, hasher);
        /// }
        /// table.retain(|&mut x| x % 2 == 0);
        /// assert_eq!(table.len(), 3);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn retain(&mut self, mut f: impl FnMut(&mut T) -> bool) {
            unsafe {
                for item in self.raw.iter() {
                    if !f(item.as_mut()) {
                        self.raw.erase(item);
                    }
                }
            }
        }
        /// Clears the set, returning all elements in an iterator.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for x in 1..=3 {
        ///     table.insert_unique(hasher(&x), x, hasher);
        /// }
        /// assert!(!table.is_empty());
        ///
        /// // print 1, 2, 3 in an arbitrary order
        /// for i in table.drain() {
        ///     println!("{}", i);
        /// }
        ///
        /// assert!(table.is_empty());
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn drain(&mut self) -> Drain<'_, T, A> {
            Drain { inner: self.raw.drain() }
        }
        /// Drains elements which are true under the given predicate,
        /// and returns an iterator over the removed items.
        ///
        /// In other words, move all elements `e` such that `f(&e)` returns `true` out
        /// into another iterator.
        ///
        /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
        /// or the iteration short-circuits, then the remaining elements will be retained.
        /// Use [`retain()`] with a negated predicate if you do not need the returned iterator.
        ///
        /// [`retain()`]: HashTable::retain
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for x in 0..8 {
        ///     table.insert_unique(hasher(&x), x, hasher);
        /// }
        /// let drained: Vec<i32> = table.extract_if(|&mut v| v % 2 == 0).collect();
        ///
        /// let mut evens = drained.into_iter().collect::<Vec<_>>();
        /// let mut odds = table.into_iter().collect::<Vec<_>>();
        /// evens.sort();
        /// odds.sort();
        ///
        /// assert_eq!(evens, vec![0, 2, 4, 6]);
        /// assert_eq!(odds, vec![1, 3, 5, 7]);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn extract_if<F>(&mut self, f: F) -> ExtractIf<'_, T, F, A>
        where
            F: FnMut(&mut T) -> bool,
        {
            ExtractIf {
                f,
                inner: RawExtractIf {
                    iter: unsafe { self.raw.iter() },
                    table: &mut self.raw,
                },
            }
        }
        /// Attempts to get mutable references to `N` values in the map at once.
        ///
        /// The `eq` argument should be a closure such that `eq(i, k)` returns true if `k` is equal to
        /// the `i`th key to be looked up.
        ///
        /// Returns an array of length `N` with the results of each query. For soundness, at most one
        /// mutable reference will be returned to any value. `None` will be used if the key is missing.
        ///
        /// # Panics
        ///
        /// Panics if any keys are overlapping.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut libraries: HashTable<(&str, u32)> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for (k, v) in [
        ///     ("Bodleian Library", 1602),
        ///     ("Athenæum", 1807),
        ///     ("Herzogin-Anna-Amalia-Bibliothek", 1691),
        ///     ("Library of Congress", 1800),
        /// ] {
        ///     libraries.insert_unique(hasher(&k), (k, v), |(k, _)| hasher(&k));
        /// }
        ///
        /// let keys = ["Athenæum", "Library of Congress"];
        /// let got = libraries.get_disjoint_mut(keys.map(|k| hasher(&k)), |i, val| keys[i] == val.0);
        /// assert_eq!(
        ///     got,
        ///     [Some(&mut ("Athenæum", 1807)), Some(&mut ("Library of Congress", 1800))],
        /// );
        ///
        /// // Missing keys result in None
        /// let keys = ["Athenæum", "New York Public Library"];
        /// let got = libraries.get_disjoint_mut(keys.map(|k| hasher(&k)), |i, val| keys[i] == val.0);
        /// assert_eq!(got, [Some(&mut ("Athenæum", 1807)), None]);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        ///
        /// ```should_panic
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// # use hashbrown::{HashTable, DefaultHashBuilder};
        /// # use std::hash::BuildHasher;
        ///
        /// let mut libraries: HashTable<(&str, u32)> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for (k, v) in [
        ///     ("Athenæum", 1807),
        ///     ("Library of Congress", 1800),
        /// ] {
        ///     libraries.insert_unique(hasher(&k), (k, v), |(k, _)| hasher(&k));
        /// }
        ///
        /// // Duplicate keys result in a panic!
        /// let keys = ["Athenæum", "Athenæum"];
        /// let got = libraries.get_disjoint_mut(keys.map(|k| hasher(&k)), |i, val| keys[i] == val.0);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test();
        /// #     #[cfg(not(feature = "nightly"))]
        /// #     panic!();
        /// # }
        /// ```
        pub fn get_disjoint_mut<const N: usize>(
            &mut self,
            hashes: [u64; N],
            eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<&'_ mut T>; N] {
            self.raw.get_disjoint_mut(hashes, eq)
        }
        /// Attempts to get mutable references to `N` values in the map at once.
        #[deprecated(note = "use `get_disjoint_mut` instead")]
        pub fn get_many_mut<const N: usize>(
            &mut self,
            hashes: [u64; N],
            eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<&'_ mut T>; N] {
            self.raw.get_disjoint_mut(hashes, eq)
        }
        /// Attempts to get mutable references to `N` values in the map at once, without validating that
        /// the values are unique.
        ///
        /// The `eq` argument should be a closure such that `eq(i, k)` returns true if `k` is equal to
        /// the `i`th key to be looked up.
        ///
        /// Returns an array of length `N` with the results of each query. `None` will be returned if
        /// any of the keys are missing.
        ///
        /// For a safe alternative see [`get_disjoint_mut`](`HashTable::get_disjoint_mut`).
        ///
        /// # Safety
        ///
        /// Calling this method with overlapping keys is *[undefined behavior]* even if the resulting
        /// references are not used.
        ///
        /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut libraries: HashTable<(&str, u32)> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for (k, v) in [
        ///     ("Bodleian Library", 1602),
        ///     ("Athenæum", 1807),
        ///     ("Herzogin-Anna-Amalia-Bibliothek", 1691),
        ///     ("Library of Congress", 1800),
        /// ] {
        ///     libraries.insert_unique(hasher(&k), (k, v), |(k, _)| hasher(&k));
        /// }
        ///
        /// let keys = ["Athenæum", "Library of Congress"];
        /// let got = libraries.get_disjoint_mut(keys.map(|k| hasher(&k)), |i, val| keys[i] == val.0);
        /// assert_eq!(
        ///     got,
        ///     [Some(&mut ("Athenæum", 1807)), Some(&mut ("Library of Congress", 1800))],
        /// );
        ///
        /// // Missing keys result in None
        /// let keys = ["Athenæum", "New York Public Library"];
        /// let got = libraries.get_disjoint_mut(keys.map(|k| hasher(&k)), |i, val| keys[i] == val.0);
        /// assert_eq!(got, [Some(&mut ("Athenæum", 1807)), None]);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub unsafe fn get_disjoint_unchecked_mut<const N: usize>(
            &mut self,
            hashes: [u64; N],
            eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<&'_ mut T>; N] {
            self.raw.get_disjoint_unchecked_mut(hashes, eq)
        }
        /// Attempts to get mutable references to `N` values in the map at once, without validating that
        /// the values are unique.
        #[deprecated(note = "use `get_disjoint_unchecked_mut` instead")]
        pub unsafe fn get_many_unchecked_mut<const N: usize>(
            &mut self,
            hashes: [u64; N],
            eq: impl FnMut(usize, &T) -> bool,
        ) -> [Option<&'_ mut T>; N] {
            self.raw.get_disjoint_unchecked_mut(hashes, eq)
        }
        /// Returns the total amount of memory allocated internally by the hash
        /// table, in bytes.
        ///
        /// The returned number is informational only. It is intended to be
        /// primarily used for memory profiling.
        #[inline]
        pub fn allocation_size(&self) -> usize {
            self.raw.allocation_size()
        }
    }
    impl<T, A> IntoIterator for HashTable<T, A>
    where
        A: Allocator,
    {
        type Item = T;
        type IntoIter = IntoIter<T, A>;
        fn into_iter(self) -> IntoIter<T, A> {
            IntoIter {
                inner: self.raw.into_iter(),
            }
        }
    }
    impl<'a, T, A> IntoIterator for &'a HashTable<T, A>
    where
        A: Allocator,
    {
        type Item = &'a T;
        type IntoIter = Iter<'a, T>;
        fn into_iter(self) -> Iter<'a, T> {
            self.iter()
        }
    }
    impl<'a, T, A> IntoIterator for &'a mut HashTable<T, A>
    where
        A: Allocator,
    {
        type Item = &'a mut T;
        type IntoIter = IterMut<'a, T>;
        fn into_iter(self) -> IterMut<'a, T> {
            self.iter_mut()
        }
    }
    impl<T, A> Default for HashTable<T, A>
    where
        A: Allocator + Default,
    {
        fn default() -> Self {
            Self { raw: Default::default() }
        }
    }
    impl<T, A> Clone for HashTable<T, A>
    where
        T: Clone,
        A: Allocator + Clone,
    {
        fn clone(&self) -> Self {
            Self { raw: self.raw.clone() }
        }
    }
    impl<T, A> fmt::Debug for HashTable<T, A>
    where
        T: fmt::Debug,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_set().entries(self.iter()).finish()
        }
    }
    /// A view into a single entry in a table, which may either be vacant or occupied.
    ///
    /// This `enum` is constructed from the [`entry`] method on [`HashTable`].
    ///
    /// [`HashTable`]: struct.HashTable.html
    /// [`entry`]: struct.HashTable.html#method.entry
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "nightly")]
    /// # fn test() {
    /// use hashbrown::hash_table::{Entry, OccupiedEntry};
    /// use hashbrown::{HashTable, DefaultHashBuilder};
    /// use std::hash::BuildHasher;
    ///
    /// let mut table = HashTable::new();
    /// let hasher = DefaultHashBuilder::default();
    /// let hasher = |val: &_| hasher.hash_one(val);
    /// for x in ["a", "b", "c"] {
    ///     table.insert_unique(hasher(&x), x, hasher);
    /// }
    /// assert_eq!(table.len(), 3);
    ///
    /// // Existing value (insert)
    /// let entry: Entry<_> = table.entry(hasher(&"a"), |&x| x == "a", hasher);
    /// let _raw_o: OccupiedEntry<_, _> = entry.insert("a");
    /// assert_eq!(table.len(), 3);
    /// // Nonexistent value (insert)
    /// table.entry(hasher(&"d"), |&x| x == "d", hasher).insert("d");
    ///
    /// // Existing value (or_insert)
    /// table
    ///     .entry(hasher(&"b"), |&x| x == "b", hasher)
    ///     .or_insert("b");
    /// // Nonexistent value (or_insert)
    /// table
    ///     .entry(hasher(&"e"), |&x| x == "e", hasher)
    ///     .or_insert("e");
    ///
    /// println!("Our HashTable: {:?}", table);
    ///
    /// let mut vec: Vec<_> = table.iter().copied().collect();
    /// // The `Iter` iterator produces items in arbitrary order, so the
    /// // items must be sorted to test them against a sorted array.
    /// vec.sort_unstable();
    /// assert_eq!(vec, ["a", "b", "c", "d", "e"]);
    /// # }
    /// # fn main() {
    /// #     #[cfg(feature = "nightly")]
    /// #     test()
    /// # }
    /// ```
    pub enum Entry<'a, T, A = Global>
    where
        A: Allocator,
    {
        /// An occupied entry.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::{Entry, OccupiedEntry};
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// for x in ["a", "b"] {
        ///     table.insert_unique(hasher(&x), x, hasher);
        /// }
        ///
        /// match table.entry(hasher(&"a"), |&x| x == "a", hasher) {
        ///     Entry::Vacant(_) => unreachable!(),
        ///     Entry::Occupied(_) => {}
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        Occupied(OccupiedEntry<'a, T, A>),
        /// A vacant entry.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::{Entry, OccupiedEntry};
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::<&str>::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// match table.entry(hasher(&"a"), |&x| x == "a", hasher) {
        ///     Entry::Vacant(_) => {}
        ///     Entry::Occupied(_) => unreachable!(),
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        Vacant(VacantEntry<'a, T, A>),
    }
    impl<T: fmt::Debug, A: Allocator> fmt::Debug for Entry<'_, T, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
                Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
            }
        }
    }
    impl<'a, T, A> Entry<'a, T, A>
    where
        A: Allocator,
    {
        /// Sets the value of the entry, replacing any existing value if there is
        /// one, and returns an [`OccupiedEntry`].
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<&str> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// let entry = table
        ///     .entry(hasher(&"horseyland"), |&x| x == "horseyland", hasher)
        ///     .insert("horseyland");
        ///
        /// assert_eq!(entry.get(), &"horseyland");
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn insert(self, value: T) -> OccupiedEntry<'a, T, A> {
            match self {
                Entry::Occupied(mut entry) => {
                    *entry.get_mut() = value;
                    entry
                }
                Entry::Vacant(entry) => entry.insert(value),
            }
        }
        /// Ensures a value is in the entry by inserting if it was vacant.
        ///
        /// Returns an [`OccupiedEntry`] pointing to the now-occupied entry.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<&str> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// // nonexistent key
        /// table
        ///     .entry(hasher(&"poneyland"), |&x| x == "poneyland", hasher)
        ///     .or_insert("poneyland");
        /// assert!(table
        ///     .find(hasher(&"poneyland"), |&x| x == "poneyland")
        ///     .is_some());
        ///
        /// // existing key
        /// table
        ///     .entry(hasher(&"poneyland"), |&x| x == "poneyland", hasher)
        ///     .or_insert("poneyland");
        /// assert!(table
        ///     .find(hasher(&"poneyland"), |&x| x == "poneyland")
        ///     .is_some());
        /// assert_eq!(table.len(), 1);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn or_insert(self, default: T) -> OccupiedEntry<'a, T, A> {
            match self {
                Entry::Occupied(entry) => entry,
                Entry::Vacant(entry) => entry.insert(default),
            }
        }
        /// Ensures a value is in the entry by inserting the result of the default function if empty..
        ///
        /// Returns an [`OccupiedEntry`] pointing to the now-occupied entry.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<String> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// table
        ///     .entry(hasher("poneyland"), |x| x == "poneyland", |val| hasher(val))
        ///     .or_insert_with(|| "poneyland".to_string());
        ///
        /// assert!(table
        ///     .find(hasher(&"poneyland"), |x| x == "poneyland")
        ///     .is_some());
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn or_insert_with(
            self,
            default: impl FnOnce() -> T,
        ) -> OccupiedEntry<'a, T, A> {
            match self {
                Entry::Occupied(entry) => entry,
                Entry::Vacant(entry) => entry.insert(default()),
            }
        }
        /// Provides in-place mutable access to an occupied entry before any
        /// potential inserts into the table.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<(&str, u32)> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// table
        ///     .entry(
        ///         hasher(&"poneyland"),
        ///         |&(x, _)| x == "poneyland",
        ///         |(k, _)| hasher(&k),
        ///     )
        ///     .and_modify(|(_, v)| *v += 1)
        ///     .or_insert(("poneyland", 42));
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&(k, _)| k == "poneyland"),
        ///     Some(&("poneyland", 42))
        /// );
        ///
        /// table
        ///     .entry(
        ///         hasher(&"poneyland"),
        ///         |&(x, _)| x == "poneyland",
        ///         |(k, _)| hasher(&k),
        ///     )
        ///     .and_modify(|(_, v)| *v += 1)
        ///     .or_insert(("poneyland", 42));
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&(k, _)| k == "poneyland"),
        ///     Some(&("poneyland", 43))
        /// );
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn and_modify(self, f: impl FnOnce(&mut T)) -> Self {
            match self {
                Entry::Occupied(mut entry) => {
                    f(entry.get_mut());
                    Entry::Occupied(entry)
                }
                Entry::Vacant(entry) => Entry::Vacant(entry),
            }
        }
    }
    /// A view into an occupied entry in a `HashTable`.
    /// It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "nightly")]
    /// # fn test() {
    /// use hashbrown::hash_table::{Entry, OccupiedEntry};
    /// use hashbrown::{HashTable, DefaultHashBuilder};
    /// use std::hash::BuildHasher;
    ///
    /// let mut table = HashTable::new();
    /// let hasher = DefaultHashBuilder::default();
    /// let hasher = |val: &_| hasher.hash_one(val);
    /// for x in ["a", "b", "c"] {
    ///     table.insert_unique(hasher(&x), x, hasher);
    /// }
    /// assert_eq!(table.len(), 3);
    ///
    /// let _entry_o: OccupiedEntry<_, _> = table.find_entry(hasher(&"a"), |&x| x == "a").unwrap();
    /// assert_eq!(table.len(), 3);
    ///
    /// // Existing key
    /// match table.entry(hasher(&"a"), |&x| x == "a", hasher) {
    ///     Entry::Vacant(_) => unreachable!(),
    ///     Entry::Occupied(view) => {
    ///         assert_eq!(view.get(), &"a");
    ///     }
    /// }
    ///
    /// assert_eq!(table.len(), 3);
    ///
    /// // Existing key (take)
    /// match table.entry(hasher(&"c"), |&x| x == "c", hasher) {
    ///     Entry::Vacant(_) => unreachable!(),
    ///     Entry::Occupied(view) => {
    ///         assert_eq!(view.remove().0, "c");
    ///     }
    /// }
    /// assert_eq!(table.find(hasher(&"c"), |&x| x == "c"), None);
    /// assert_eq!(table.len(), 2);
    /// # }
    /// # fn main() {
    /// #     #[cfg(feature = "nightly")]
    /// #     test()
    /// # }
    /// ```
    pub struct OccupiedEntry<'a, T, A = Global>
    where
        A: Allocator,
    {
        bucket: Bucket<T>,
        table: &'a mut HashTable<T, A>,
    }
    unsafe impl<T, A> Send for OccupiedEntry<'_, T, A>
    where
        T: Send,
        A: Send + Allocator,
    {}
    unsafe impl<T, A> Sync for OccupiedEntry<'_, T, A>
    where
        T: Sync,
        A: Sync + Allocator,
    {}
    impl<T: fmt::Debug, A: Allocator> fmt::Debug for OccupiedEntry<'_, T, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OccupiedEntry").field("value", self.get()).finish()
        }
    }
    impl<'a, T, A> OccupiedEntry<'a, T, A>
    where
        A: Allocator,
    {
        /// Takes the value out of the entry, and returns it along with a
        /// `VacantEntry` that can be used to insert another value with the same
        /// hash as the one that was just removed.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<&str> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// // The table is empty
        /// assert!(table.is_empty() && table.capacity() == 0);
        ///
        /// table.insert_unique(hasher(&"poneyland"), "poneyland", hasher);
        /// let capacity_before_remove = table.capacity();
        ///
        /// if let Entry::Occupied(o) = table.entry(hasher(&"poneyland"), |&x| x == "poneyland", hasher) {
        ///     assert_eq!(o.remove().0, "poneyland");
        /// }
        ///
        /// assert!(table
        ///     .find(hasher(&"poneyland"), |&x| x == "poneyland")
        ///     .is_none());
        /// // Now table hold none elements but capacity is equal to the old one
        /// assert!(table.len() == 0 && table.capacity() == capacity_before_remove);
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn remove(self) -> (T, VacantEntry<'a, T, A>) {
            let (val, index, tag) = unsafe { self.table.raw.remove_tagged(self.bucket) };
            (
                val,
                VacantEntry {
                    tag,
                    index,
                    table: self.table,
                },
            )
        }
        /// Gets a reference to the value in the entry.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<&str> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"poneyland"), "poneyland", hasher);
        ///
        /// match table.entry(hasher(&"poneyland"), |&x| x == "poneyland", hasher) {
        ///     Entry::Vacant(_) => panic!(),
        ///     Entry::Occupied(entry) => assert_eq!(entry.get(), &"poneyland"),
        /// }
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub fn get(&self) -> &T {
            unsafe { self.bucket.as_ref() }
        }
        /// Gets a mutable reference to the value in the entry.
        ///
        /// If you need a reference to the `OccupiedEntry` which may outlive the
        /// destruction of the `Entry` value, see [`into_mut`].
        ///
        /// [`into_mut`]: #method.into_mut
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<(&str, u32)> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"poneyland"), ("poneyland", 12), |(k, _)| hasher(&k));
        ///
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&(x, _)| x == "poneyland",),
        ///     Some(&("poneyland", 12))
        /// );
        ///
        /// if let Entry::Occupied(mut o) = table.entry(
        ///     hasher(&"poneyland"),
        ///     |&(x, _)| x == "poneyland",
        ///     |(k, _)| hasher(&k),
        /// ) {
        ///     o.get_mut().1 += 10;
        ///     assert_eq!(o.get().1, 22);
        ///
        ///     // We can use the same Entry multiple times.
        ///     o.get_mut().1 += 2;
        /// }
        ///
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&(x, _)| x == "poneyland",),
        ///     Some(&("poneyland", 24))
        /// );
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub fn get_mut(&mut self) -> &mut T {
            unsafe { self.bucket.as_mut() }
        }
        /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry
        /// with a lifetime bound to the table itself.
        ///
        /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
        ///
        /// [`get_mut`]: #method.get_mut
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<(&str, u32)> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&"poneyland"), ("poneyland", 12), |(k, _)| hasher(&k));
        ///
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&(x, _)| x == "poneyland",),
        ///     Some(&("poneyland", 12))
        /// );
        ///
        /// let value: &mut (&str, u32);
        /// match table.entry(
        ///     hasher(&"poneyland"),
        ///     |&(x, _)| x == "poneyland",
        ///     |(k, _)| hasher(&k),
        /// ) {
        ///     Entry::Occupied(entry) => value = entry.into_mut(),
        ///     Entry::Vacant(_) => panic!(),
        /// }
        /// value.1 += 10;
        ///
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&(x, _)| x == "poneyland",),
        ///     Some(&("poneyland", 22))
        /// );
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn into_mut(self) -> &'a mut T {
            unsafe { self.bucket.as_mut() }
        }
        /// Converts the `OccupiedEntry` into a mutable reference to the underlying
        /// table.
        pub fn into_table(self) -> &'a mut HashTable<T, A> {
            self.table
        }
        /// Returns the bucket index in the table for this entry.
        ///
        /// This can be used to store a borrow-free "reference" to the entry, later using
        /// [`HashTable::get_bucket`], [`HashTable::get_bucket_mut`], or
        /// [`HashTable::get_bucket_entry`] to access it again without hash probing.
        ///
        /// The index is only meaningful as long as the table is not resized and no entries are added
        /// or removed. After such changes, it may end up pointing to a different entry or none at all.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        /// table.insert_unique(hasher(&1), (1, 1), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&2), (2, 2), |val| hasher(&val.0));
        /// table.insert_unique(hasher(&3), (3, 3), |val| hasher(&val.0));
        ///
        /// let index = table
        ///     .entry(hasher(&2), |val| val.0 == 2, |val| hasher(&val.0))
        ///     .or_insert((2, -2))
        ///     .bucket_index();
        /// assert_eq!(table.get_bucket(index), Some(&(2, 2)));
        ///
        /// // Full mutation would invalidate any normal reference
        /// for (_key, value) in &mut table {
        ///     *value *= 11;
        /// }
        ///
        /// // The index still reaches the same key with the updated value
        /// assert_eq!(table.get_bucket(index), Some(&(2, 22)));
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        pub fn bucket_index(&self) -> usize {
            unsafe { self.table.raw.bucket_index(&self.bucket) }
        }
    }
    /// A view into a vacant entry in a `HashTable`.
    /// It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "nightly")]
    /// # fn test() {
    /// use hashbrown::hash_table::{Entry, VacantEntry};
    /// use hashbrown::{HashTable, DefaultHashBuilder};
    /// use std::hash::BuildHasher;
    ///
    /// let mut table: HashTable<&str> = HashTable::new();
    /// let hasher = DefaultHashBuilder::default();
    /// let hasher = |val: &_| hasher.hash_one(val);
    ///
    /// let entry_v: VacantEntry<_, _> = match table.entry(hasher(&"a"), |&x| x == "a", hasher) {
    ///     Entry::Vacant(view) => view,
    ///     Entry::Occupied(_) => unreachable!(),
    /// };
    /// entry_v.insert("a");
    /// assert!(table.find(hasher(&"a"), |&x| x == "a").is_some() && table.len() == 1);
    ///
    /// // Nonexistent key (insert)
    /// match table.entry(hasher(&"b"), |&x| x == "b", hasher) {
    ///     Entry::Vacant(view) => {
    ///         view.insert("b");
    ///     }
    ///     Entry::Occupied(_) => unreachable!(),
    /// }
    /// assert!(table.find(hasher(&"b"), |&x| x == "b").is_some() && table.len() == 2);
    /// # }
    /// # fn main() {
    /// #     #[cfg(feature = "nightly")]
    /// #     test()
    /// # }
    /// ```
    pub struct VacantEntry<'a, T, A = Global>
    where
        A: Allocator,
    {
        tag: Tag,
        index: usize,
        table: &'a mut HashTable<T, A>,
    }
    impl<T: fmt::Debug, A: Allocator> fmt::Debug for VacantEntry<'_, T, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("VacantEntry")
        }
    }
    impl<'a, T, A> VacantEntry<'a, T, A>
    where
        A: Allocator,
    {
        /// Inserts a new element into the table with the hash that was used to
        /// obtain the `VacantEntry`.
        ///
        /// An `OccupiedEntry` is returned for the newly inserted element.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "nightly")]
        /// # fn test() {
        /// use hashbrown::hash_table::Entry;
        /// use hashbrown::{HashTable, DefaultHashBuilder};
        /// use std::hash::BuildHasher;
        ///
        /// let mut table: HashTable<&str> = HashTable::new();
        /// let hasher = DefaultHashBuilder::default();
        /// let hasher = |val: &_| hasher.hash_one(val);
        ///
        /// if let Entry::Vacant(o) = table.entry(hasher(&"poneyland"), |&x| x == "poneyland", hasher) {
        ///     o.insert("poneyland");
        /// }
        /// assert_eq!(
        ///     table.find(hasher(&"poneyland"), |&x| x == "poneyland"),
        ///     Some(&"poneyland")
        /// );
        /// # }
        /// # fn main() {
        /// #     #[cfg(feature = "nightly")]
        /// #     test()
        /// # }
        /// ```
        #[inline]
        pub fn insert(self, value: T) -> OccupiedEntry<'a, T, A> {
            let bucket = unsafe {
                self.table.raw.insert_tagged_at_index(self.tag, self.index, value)
            };
            OccupiedEntry {
                bucket,
                table: self.table,
            }
        }
        /// Converts the `VacantEntry` into a mutable reference to the underlying
        /// table.
        pub fn into_table(self) -> &'a mut HashTable<T, A> {
            self.table
        }
    }
    /// Type representing the absence of an entry, as returned by [`HashTable::find_entry`]
    /// and [`HashTable::get_bucket_entry`].
    ///
    /// This type only exists due to [limitations] in Rust's NLL borrow checker. In
    /// the future, those methods will return an `Option<OccupiedEntry>` and this
    /// type will be removed.
    ///
    /// [limitations]: https://smallcultfollowing.com/babysteps/blog/2018/06/15/mir-based-borrow-check-nll-status-update/#polonius
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "nightly")]
    /// # fn test() {
    /// use hashbrown::hash_table::{AbsentEntry, Entry};
    /// use hashbrown::{HashTable, DefaultHashBuilder};
    /// use std::hash::BuildHasher;
    ///
    /// let mut table: HashTable<&str> = HashTable::new();
    /// let hasher = DefaultHashBuilder::default();
    /// let hasher = |val: &_| hasher.hash_one(val);
    ///
    /// let entry_v: AbsentEntry<_, _> = table.find_entry(hasher(&"a"), |&x| x == "a").unwrap_err();
    /// entry_v
    ///     .into_table()
    ///     .insert_unique(hasher(&"a"), "a", hasher);
    /// assert!(table.find(hasher(&"a"), |&x| x == "a").is_some() && table.len() == 1);
    ///
    /// // Nonexistent key (insert)
    /// match table.entry(hasher(&"b"), |&x| x == "b", hasher) {
    ///     Entry::Vacant(view) => {
    ///         view.insert("b");
    ///     }
    ///     Entry::Occupied(_) => unreachable!(),
    /// }
    /// assert!(table.find(hasher(&"b"), |&x| x == "b").is_some() && table.len() == 2);
    /// # }
    /// # fn main() {
    /// #     #[cfg(feature = "nightly")]
    /// #     test()
    /// # }
    /// ```
    pub struct AbsentEntry<'a, T, A = Global>
    where
        A: Allocator,
    {
        table: &'a mut HashTable<T, A>,
    }
    impl<T: fmt::Debug, A: Allocator> fmt::Debug for AbsentEntry<'_, T, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("AbsentEntry")
        }
    }
    impl<'a, T, A> AbsentEntry<'a, T, A>
    where
        A: Allocator,
    {
        /// Converts the `AbsentEntry` into a mutable reference to the underlying
        /// table.
        pub fn into_table(self) -> &'a mut HashTable<T, A> {
            self.table
        }
    }
    /// An iterator over the entries of a `HashTable` in arbitrary order.
    /// The iterator element type is `&'a T`.
    ///
    /// This `struct` is created by the [`iter`] method on [`HashTable`]. See its
    /// documentation for more.
    ///
    /// [`iter`]: struct.HashTable.html#method.iter
    /// [`HashTable`]: struct.HashTable.html
    pub struct Iter<'a, T> {
        inner: RawIter<T>,
        marker: PhantomData<&'a T>,
    }
    impl<T> Default for Iter<'_, T> {
        fn default() -> Self {
            Iter {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<'a, T> Iterator for Iter<'a, T> {
        type Item = &'a T;
        fn next(&mut self) -> Option<Self::Item> {
            match self.inner.next() {
                Some(bucket) => Some(unsafe { bucket.as_ref() }),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, bucket| unsafe { f(acc, bucket.as_ref()) })
        }
    }
    impl<T> ExactSizeIterator for Iter<'_, T> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<T> FusedIterator for Iter<'_, T> {}
    impl<'a, T> Clone for Iter<'a, T> {
        fn clone(&self) -> Iter<'a, T> {
            Iter {
                inner: self.inner.clone(),
                marker: PhantomData,
            }
        }
    }
    impl<T: fmt::Debug> fmt::Debug for Iter<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// A mutable iterator over the entries of a `HashTable` in arbitrary order.
    /// The iterator element type is `&'a mut T`.
    ///
    /// This `struct` is created by the [`iter_mut`] method on [`HashTable`]. See its
    /// documentation for more.
    ///
    /// [`iter_mut`]: struct.HashTable.html#method.iter_mut
    /// [`HashTable`]: struct.HashTable.html
    pub struct IterMut<'a, T> {
        inner: RawIter<T>,
        marker: PhantomData<&'a mut T>,
    }
    impl<T> Default for IterMut<'_, T> {
        fn default() -> Self {
            IterMut {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<'a, T> Iterator for IterMut<'a, T> {
        type Item = &'a mut T;
        fn next(&mut self) -> Option<Self::Item> {
            match self.inner.next() {
                Some(bucket) => Some(unsafe { bucket.as_mut() }),
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, bucket| unsafe { f(acc, bucket.as_mut()) })
        }
    }
    impl<T> ExactSizeIterator for IterMut<'_, T> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<T> FusedIterator for IterMut<'_, T> {}
    impl<T> fmt::Debug for IterMut<'_, T>
    where
        T: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(Iter {
                    inner: self.inner.clone(),
                    marker: PhantomData,
                })
                .finish()
        }
    }
    /// An iterator producing the `usize` indices of all occupied buckets,
    /// within the range `0..table.num_buckets()`.
    ///
    /// The order in which the iterator yields indices is unspecified
    /// and may change in the future.
    ///
    /// This `struct` is created by the [`HashTable::iter_buckets`] method. See its
    /// documentation for more.
    pub struct IterBuckets<'a, T> {
        inner: FullBucketsIndices,
        marker: PhantomData<&'a T>,
    }
    impl<T> Clone for IterBuckets<'_, T> {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                marker: PhantomData,
            }
        }
    }
    impl<T> Default for IterBuckets<'_, T> {
        #[inline]
        fn default() -> Self {
            Self {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<T> Iterator for IterBuckets<'_, T> {
        type Item = usize;
        #[inline]
        fn next(&mut self) -> Option<usize> {
            self.inner.next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }
    impl<T> ExactSizeIterator for IterBuckets<'_, T> {
        #[inline]
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<T> FusedIterator for IterBuckets<'_, T> {}
    impl<T> fmt::Debug for IterBuckets<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// An iterator over the entries of a `HashTable` that could match a given hash.
    /// The iterator element type is `&'a T`.
    ///
    /// This `struct` is created by the [`iter_hash`] method on [`HashTable`]. See its
    /// documentation for more.
    ///
    /// [`iter_hash`]: struct.HashTable.html#method.iter_hash
    /// [`HashTable`]: struct.HashTable.html
    pub struct IterHash<'a, T> {
        inner: RawIterHash<T>,
        marker: PhantomData<&'a T>,
    }
    impl<T> Default for IterHash<'_, T> {
        fn default() -> Self {
            IterHash {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<'a, T> Iterator for IterHash<'a, T> {
        type Item = &'a T;
        fn next(&mut self) -> Option<Self::Item> {
            match self.inner.next() {
                Some(bucket) => Some(unsafe { bucket.as_ref() }),
                None => None,
            }
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, bucket| unsafe { f(acc, bucket.as_ref()) })
        }
    }
    impl<T> FusedIterator for IterHash<'_, T> {}
    impl<'a, T> Clone for IterHash<'a, T> {
        fn clone(&self) -> IterHash<'a, T> {
            IterHash {
                inner: self.inner.clone(),
                marker: PhantomData,
            }
        }
    }
    impl<T> fmt::Debug for IterHash<'_, T>
    where
        T: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// A mutable iterator over the entries of a `HashTable` that could match a given hash.
    /// The iterator element type is `&'a mut T`.
    ///
    /// This `struct` is created by the [`iter_hash_mut`] method on [`HashTable`]. See its
    /// documentation for more.
    ///
    /// [`iter_hash_mut`]: struct.HashTable.html#method.iter_hash_mut
    /// [`HashTable`]: struct.HashTable.html
    pub struct IterHashMut<'a, T> {
        inner: RawIterHash<T>,
        marker: PhantomData<&'a mut T>,
    }
    impl<T> Default for IterHashMut<'_, T> {
        fn default() -> Self {
            IterHashMut {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<'a, T> Iterator for IterHashMut<'a, T> {
        type Item = &'a mut T;
        fn next(&mut self) -> Option<Self::Item> {
            match self.inner.next() {
                Some(bucket) => Some(unsafe { bucket.as_mut() }),
                None => None,
            }
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, |acc, bucket| unsafe { f(acc, bucket.as_mut()) })
        }
    }
    impl<T> FusedIterator for IterHashMut<'_, T> {}
    impl<T> fmt::Debug for IterHashMut<'_, T>
    where
        T: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(IterHash {
                    inner: self.inner.clone(),
                    marker: PhantomData,
                })
                .finish()
        }
    }
    /// An iterator producing the `usize` indices of all buckets which may match a hash.
    ///
    /// This `struct` is created by the [`HashTable::iter_hash_buckets`] method. See its
    /// documentation for more.
    pub struct IterHashBuckets<'a, T> {
        inner: RawIterHashIndices,
        marker: PhantomData<&'a T>,
    }
    impl<T> Clone for IterHashBuckets<'_, T> {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                marker: PhantomData,
            }
        }
    }
    impl<T> Default for IterHashBuckets<'_, T> {
        #[inline]
        fn default() -> Self {
            Self {
                inner: Default::default(),
                marker: PhantomData,
            }
        }
    }
    impl<T> Iterator for IterHashBuckets<'_, T> {
        type Item = usize;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
    }
    impl<T> FusedIterator for IterHashBuckets<'_, T> {}
    impl<T> fmt::Debug for IterHashBuckets<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }
    /// An owning iterator over the entries of a `HashTable` in arbitrary order.
    /// The iterator element type is `T`.
    ///
    /// This `struct` is created by the [`into_iter`] method on [`HashTable`]
    /// (provided by the [`IntoIterator`] trait). See its documentation for more.
    /// The table cannot be used after calling that method.
    ///
    /// [`into_iter`]: struct.HashTable.html#method.into_iter
    /// [`HashTable`]: struct.HashTable.html
    /// [`IntoIterator`]: https://doc.rust-lang.org/core/iter/trait.IntoIterator.html
    pub struct IntoIter<T, A = Global>
    where
        A: Allocator,
    {
        inner: RawIntoIter<T, A>,
    }
    impl<T, A: Allocator> Default for IntoIter<T, A> {
        fn default() -> Self {
            IntoIter {
                inner: Default::default(),
            }
        }
    }
    impl<T, A> Iterator for IntoIter<T, A>
    where
        A: Allocator,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, f)
        }
    }
    impl<T, A> ExactSizeIterator for IntoIter<T, A>
    where
        A: Allocator,
    {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<T, A> FusedIterator for IntoIter<T, A>
    where
        A: Allocator,
    {}
    impl<T, A> fmt::Debug for IntoIter<T, A>
    where
        T: fmt::Debug,
        A: Allocator,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(Iter {
                    inner: self.inner.iter(),
                    marker: PhantomData,
                })
                .finish()
        }
    }
    /// A draining iterator over the items of a `HashTable`.
    ///
    /// This `struct` is created by the [`drain`] method on [`HashTable`].
    /// See its documentation for more.
    ///
    /// [`HashTable`]: struct.HashTable.html
    /// [`drain`]: struct.HashTable.html#method.drain
    pub struct Drain<'a, T, A: Allocator = Global> {
        inner: RawDrain<'a, T, A>,
    }
    impl<T, A: Allocator> Iterator for Drain<'_, T, A> {
        type Item = T;
        fn next(&mut self) -> Option<T> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.inner.fold(init, f)
        }
    }
    impl<T, A: Allocator> ExactSizeIterator for Drain<'_, T, A> {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }
    impl<T, A: Allocator> FusedIterator for Drain<'_, T, A> {}
    impl<T: fmt::Debug, A: Allocator> fmt::Debug for Drain<'_, T, A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(Iter {
                    inner: self.inner.iter(),
                    marker: PhantomData,
                })
                .finish()
        }
    }
    /// A draining iterator over entries of a `HashTable` which don't satisfy the predicate `f`.
    ///
    /// This `struct` is created by [`HashTable::extract_if`]. See its
    /// documentation for more.
    #[must_use = "Iterators are lazy unless consumed"]
    pub struct ExtractIf<'a, T, F, A: Allocator = Global> {
        f: F,
        inner: RawExtractIf<'a, T, A>,
    }
    impl<T, F, A: Allocator> Iterator for ExtractIf<'_, T, F, A>
    where
        F: FnMut(&mut T) -> bool,
    {
        type Item = T;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next(|val| (self.f)(val))
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.inner.iter.size_hint().1)
        }
    }
    impl<T, F, A: Allocator> FusedIterator for ExtractIf<'_, T, F, A>
    where
        F: FnMut(&mut T) -> bool,
    {}
}
pub use crate::hasher::DefaultHashBuilder;
pub mod hash_map {
    //! A hash map implemented with quadratic probing and SIMD lookup.
    pub use crate::map::*;
}
pub mod hash_set {
    //! A hash set implemented as a `HashMap` where the value is `()`.
    pub use crate::set::*;
}
pub mod hash_table {
    //! A hash table implemented with quadratic probing and SIMD lookup.
    pub use crate::table::*;
}
pub use crate::map::HashMap;
pub use crate::set::HashSet;
pub use crate::table::HashTable;
/// Key equivalence trait.
///
/// This trait defines the function used to compare the input value with the
/// map keys (or set values) during a lookup operation such as [`HashMap::get`]
/// or [`HashSet::contains`].
/// It is provided with a blanket implementation based on the
/// [`Borrow`](core::borrow::Borrow) trait.
///
/// # Correctness
///
/// Equivalent values must hash to the same value.
pub trait Equivalent<K: ?Sized> {
    /// Checks if this value is equivalent to the given key.
    ///
    /// Returns `true` if both values are equivalent, and `false` otherwise.
    ///
    /// # Correctness
    ///
    /// When this function returns `true`, both `self` and `key` must hash to
    /// the same value.
    fn equivalent(&self, key: &K) -> bool;
}
impl<Q: ?Sized, K: ?Sized> Equivalent<K> for Q
where
    Q: Eq,
    K: core::borrow::Borrow<Q>,
{
    fn equivalent(&self, key: &K) -> bool {
        self == key.borrow()
    }
}
/// The error type for `try_reserve` methods.
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,
    /// The memory allocator returned an error
    AllocError {
        /// The layout of the allocation request that failed.
        layout: alloc::alloc::Layout,
    },
}
#[automatically_derived]
impl ::core::clone::Clone for TryReserveError {
    #[inline]
    fn clone(&self) -> TryReserveError {
        match self {
            TryReserveError::CapacityOverflow => TryReserveError::CapacityOverflow,
            TryReserveError::AllocError { layout: __self_0 } => {
                TryReserveError::AllocError {
                    layout: ::core::clone::Clone::clone(__self_0),
                }
            }
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for TryReserveError {}
#[automatically_derived]
impl ::core::cmp::PartialEq for TryReserveError {
    #[inline]
    fn eq(&self, other: &TryReserveError) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (
                    TryReserveError::AllocError { layout: __self_0 },
                    TryReserveError::AllocError { layout: __arg1_0 },
                ) => __self_0 == __arg1_0,
                _ => true,
            }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for TryReserveError {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<alloc::alloc::Layout>;
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for TryReserveError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            TryReserveError::CapacityOverflow => {
                ::core::fmt::Formatter::write_str(f, "CapacityOverflow")
            }
            TryReserveError::AllocError { layout: __self_0 } => {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "AllocError",
                    "layout",
                    &__self_0,
                )
            }
        }
    }
}
