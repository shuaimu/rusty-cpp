#![feature(prelude_import)]
/*!
This library provides heavily optimized routines for string search primitives.

# Overview

This section gives a brief high level overview of what this crate offers.

* The top-level module provides routines for searching for 1, 2 or 3 bytes
  in the forward or reverse direction. When searching for more than one byte,
  positions are considered a match if the byte at that position matches any
  of the bytes.
* The [`memmem`] sub-module provides forward and reverse substring search
  routines.

In all such cases, routines operate on `&[u8]` without regard to encoding. This
is exactly what you want when searching either UTF-8 or arbitrary bytes.

# Example: using `memchr`

This example shows how to use `memchr` to find the first occurrence of `z` in
a haystack:

```
use memchr::memchr;

let haystack = b"foo bar baz quuz";
assert_eq!(Some(10), memchr(b'z', haystack));
```

# Example: matching one of three possible bytes

This examples shows how to use `memrchr3` to find occurrences of `a`, `b` or
`c`, starting at the end of the haystack.

```
use memchr::memchr3_iter;

let haystack = b"xyzaxyzbxyzc";

let mut it = memchr3_iter(b'a', b'b', b'c', haystack).rev();
assert_eq!(Some(11), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(Some(3), it.next());
assert_eq!(None, it.next());
```

# Example: iterating over substring matches

This example shows how to use the [`memmem`] sub-module to find occurrences of
a substring in a haystack.

```
use memchr::memmem;

let haystack = b"foo bar foo baz foo";

let mut it = memmem::find_iter(haystack, "foo");
assert_eq!(Some(0), it.next());
assert_eq!(Some(8), it.next());
assert_eq!(Some(16), it.next());
assert_eq!(None, it.next());
```

# Example: repeating a search for the same needle

It may be possible for the overhead of constructing a substring searcher to be
measurable in some workloads. In cases where the same needle is used to search
many haystacks, it is possible to do construction once and thus to avoid it for
subsequent searches. This can be done with a [`memmem::Finder`]:

```
use memchr::memmem;

let finder = memmem::Finder::new("foo");

assert_eq!(Some(4), finder.find(b"baz foo quux"));
assert_eq!(None, finder.find(b"quux baz bar"));
```

# Why use this crate?

At first glance, the APIs provided by this crate might seem weird. Why provide
a dedicated routine like `memchr` for something that could be implemented
clearly and trivially in one line:

```
fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}
```

Or similarly, why does this crate provide substring search routines when Rust's
core library already provides them?

```
fn search(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}
```

The primary reason for both of them to exist is performance. When it comes to
performance, at a high level at least, there are two primary ways to look at
it:

* **Throughput**: For this, think about it as, "given some very large haystack
  and a byte that never occurs in that haystack, how long does it take to
  search through it and determine that it, in fact, does not occur?"
* **Latency**: For this, think about it as, "given a tiny haystack---just a
  few bytes---how long does it take to determine if a byte is in it?"

The `memchr` routine in this crate has _slightly_ worse latency than the
solution presented above, however, its throughput can easily be over an
order of magnitude faster. This is a good general purpose trade off to make.
You rarely lose, but often gain big.

**NOTE:** The name `memchr` comes from the corresponding routine in `libc`. A
key advantage of using this library is that its performance is not tied to its
quality of implementation in the `libc` you happen to be using, which can vary
greatly from platform to platform.

But what about substring search? This one is a bit more complicated. The
primary reason for its existence is still indeed performance, but it's also
useful because Rust's core library doesn't actually expose any substring
search routine on arbitrary bytes. The only substring search routine that
exists works exclusively on valid UTF-8.

So if you have valid UTF-8, is there a reason to use this over the standard
library substring search routine? Yes. This routine is faster on almost every
metric, including latency. The natural question then, is why isn't this
implementation in the standard library, even if only for searching on UTF-8?
The reason is that the implementation details for using SIMD in the standard
library haven't quite been worked out yet.

**NOTE:** Currently, only `x86_64`, `wasm32` and `aarch64` targets have vector
accelerated implementations of `memchr` (and friends) and `memmem`.

# Crate features

* **std** - When enabled (the default), this will permit features specific to
the standard library. Currently, the only thing used from the standard library
is runtime SIMD CPU feature detection. This means that this feature must be
enabled to get AVX2 accelerated routines on `x86_64` targets without enabling
the `avx2` feature at compile time, for example. When `std` is not enabled,
this crate will still attempt to use SSE2 accelerated routines on `x86_64`. It
will also use AVX2 accelerated routines when the `avx2` feature is enabled at
compile time. In general, enable this feature if you can.
* **alloc** - When enabled (the default), APIs in this crate requiring some
kind of allocation will become available. For example, the
[`memmem::Finder::into_owned`](crate::memmem::Finder::into_owned) API and the
[`arch::all::shiftor`](crate::arch::all::shiftor) substring search
implementation. Otherwise, this crate is designed from the ground up to be
usable in core-only contexts, so the `alloc` feature doesn't add much
currently. Notably, disabling `std` but enabling `alloc` will **not** result
in the use of AVX2 on `x86_64` targets unless the `avx2` feature is enabled
at compile time. (With `std` enabled, AVX2 can be used even without the `avx2`
feature enabled at compile time by way of runtime CPU feature detection.)
* **logging** - When enabled (disabled by default), the `log` crate is used
to emit log messages about what kinds of `memchr` and `memmem` algorithms
are used. Namely, both `memchr` and `memmem` have a number of different
implementation choices depending on the target and CPU, and the log messages
can help show what specific implementations are being used. Generally, this is
useful for debugging performance issues.
* **libc** - **DEPRECATED**. Previously, this enabled the use of the target's
`memchr` function from whatever `libc` was linked into the program. This
feature is now a no-op because this crate's implementation of `memchr` should
now be sufficiently fast on a number of platforms that `libc` should no longer
be needed. (This feature is somewhat of a holdover from this crate's origins.
Originally, this crate was literally just a safe wrapper function around the
`memchr` function from `libc`.)
*/
#![deny(missing_docs)]
#![no_std]
extern crate core;
#[prelude_import]
use core::prelude::rust_2021::*;
extern crate std;
extern crate alloc;
pub use crate::memchr::{
    memchr, memchr2, memchr2_iter, memchr3, memchr3_iter, memchr_iter, memrchr, memrchr2,
    memrchr2_iter, memrchr3, memrchr3_iter, memrchr_iter, Memchr, Memchr2, Memchr3,
};
#[macro_use]
mod macros {
    #![allow(unused_macros)]
}
pub mod arch {
    /*!
A module with low-level architecture dependent routines.

These routines are useful as primitives for tasks not covered by the higher
level crate API.
*/
    pub mod all {
        /*!
Contains architecture independent routines.

These routines are often used as a "fallback" implementation when the more
specialized architecture dependent routines are unavailable.
*/
        pub mod memchr {
            /*!
Provides architecture independent implementations of `memchr` and friends.

The main types in this module are [`One`], [`Two`] and [`Three`]. They are for
searching for one, two or three distinct bytes, respectively, in a haystack.
Each type also has corresponding double ended iterators. These searchers
are typically slower than hand-coded vector routines accomplishing the same
task, but are also typically faster than naive scalar code. These routines
effectively work by treating a `usize` as a vector of 8-bit lanes, and thus
achieves some level of data parallelism even without explicit vector support.

The `One` searcher also provides a [`One::count`] routine for efficiently
counting the number of times a single byte occurs in a haystack. This is
useful, for example, for counting the number of lines in a haystack. This
routine exists because it is usually faster, especially with a high match
count, than using [`One::find`] repeatedly. ([`OneIter`] specializes its
`Iterator::count` implementation to use this routine.)

Only one, two and three bytes are supported because three bytes is about
the point where one sees diminishing returns. Beyond this point and it's
probably (but not necessarily) better to just use a simple `[bool; 256]` array
or similar. However, it depends mightily on the specific work-load and the
expected match frequency.
*/
            use crate::{arch::generic::memchr as generic, ext::Pointer};
            /// The number of bytes in a single `usize` value.
            const USIZE_BYTES: usize = (usize::BITS / 8) as usize;
            /// The bits that must be zero for a `*const usize` to be properly aligned.
            const USIZE_ALIGN: usize = USIZE_BYTES - 1;
            /// Finds all occurrences of a single byte in a haystack.
            pub struct One {
                s1: u8,
                v1: usize,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for One {}
            #[automatically_derived]
            impl ::core::clone::Clone for One {
                #[inline]
                fn clone(&self) -> One {
                    let _: ::core::clone::AssertParamIsClone<u8>;
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for One {}
            #[automatically_derived]
            impl ::core::fmt::Debug for One {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "One",
                        "s1",
                        &self.s1,
                        "v1",
                        &&self.v1,
                    )
                }
            }
            impl One {
                /// The number of bytes we examine per each iteration of our search loop.
                const LOOP_BYTES: usize = 2 * USIZE_BYTES;
                /// Create a new searcher that finds occurrences of the byte given.
                #[inline]
                pub fn new(needle: u8) -> One {
                    One {
                        s1: needle,
                        v1: splat(needle),
                    }
                }
                /// Return the first occurrence of the needle in the given haystack. If no
                /// such occurrence exists, then `None` is returned.
                ///
                /// The occurrence is reported as an offset into `haystack`. Its maximum
                /// value for a non-empty haystack is `haystack.len() - 1`.
                #[inline]
                pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                    unsafe {
                        generic::search_slice_with_raw(
                            haystack,
                            |s, e| { self.find_raw(s, e) },
                        )
                    }
                }
                /// Return the last occurrence of the needle in the given haystack. If no
                /// such occurrence exists, then `None` is returned.
                ///
                /// The occurrence is reported as an offset into `haystack`. Its maximum
                /// value for a non-empty haystack is `haystack.len() - 1`.
                #[inline]
                pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                    unsafe {
                        generic::search_slice_with_raw(
                            haystack,
                            |s, e| { self.rfind_raw(s, e) },
                        )
                    }
                }
                /// Counts all occurrences of this byte in the given haystack.
                #[inline]
                pub fn count(&self, haystack: &[u8]) -> usize {
                    unsafe {
                        let start = haystack.as_ptr();
                        let end = start.add(haystack.len());
                        self.count_raw(start, end)
                    }
                }
                /// Like `find`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `None` will always be returned.
                #[inline]
                pub unsafe fn find_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if start >= end {
                        return None;
                    }
                    let confirm = |b| self.confirm(b);
                    let len = end.distance(start);
                    if len < USIZE_BYTES {
                        return generic::fwd_byte_by_byte(start, end, confirm);
                    }
                    let chunk = start.cast::<usize>().read_unaligned();
                    if self.has_needle(chunk) {
                        return generic::fwd_byte_by_byte(start, end, confirm);
                    }
                    let mut cur = start
                        .add(USIZE_BYTES - (start.as_usize() & USIZE_ALIGN));
                    if true {
                        if !(cur > start) {
                            ::core::panicking::panic("assertion failed: cur > start")
                        }
                    }
                    if len <= One::LOOP_BYTES {
                        return generic::fwd_byte_by_byte(cur, end, confirm);
                    }
                    if true {
                        if !(end.sub(One::LOOP_BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: end.sub(One::LOOP_BYTES) >= start",
                            )
                        }
                    }
                    while cur <= end.sub(One::LOOP_BYTES) {
                        if true {
                            match (&0, &(cur.as_usize() % USIZE_BYTES)) {
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
                        let a = cur.cast::<usize>().read();
                        let b = cur.add(USIZE_BYTES).cast::<usize>().read();
                        if self.has_needle(a) || self.has_needle(b) {
                            break;
                        }
                        cur = cur.add(One::LOOP_BYTES);
                    }
                    generic::fwd_byte_by_byte(cur, end, confirm)
                }
                /// Like `rfind`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `None` will always be returned.
                #[inline]
                pub unsafe fn rfind_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if start >= end {
                        return None;
                    }
                    let confirm = |b| self.confirm(b);
                    let len = end.distance(start);
                    if len < USIZE_BYTES {
                        return generic::rev_byte_by_byte(start, end, confirm);
                    }
                    let chunk = end.sub(USIZE_BYTES).cast::<usize>().read_unaligned();
                    if self.has_needle(chunk) {
                        return generic::rev_byte_by_byte(start, end, confirm);
                    }
                    let mut cur = end.sub(end.as_usize() & USIZE_ALIGN);
                    if true {
                        if !(start <= cur && cur <= end) {
                            ::core::panicking::panic(
                                "assertion failed: start <= cur && cur <= end",
                            )
                        }
                    }
                    if len <= One::LOOP_BYTES {
                        return generic::rev_byte_by_byte(start, cur, confirm);
                    }
                    while cur >= start.add(One::LOOP_BYTES) {
                        if true {
                            match (&0, &(cur.as_usize() % USIZE_BYTES)) {
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
                        let a = cur.sub(2 * USIZE_BYTES).cast::<usize>().read();
                        let b = cur.sub(1 * USIZE_BYTES).cast::<usize>().read();
                        if self.has_needle(a) || self.has_needle(b) {
                            break;
                        }
                        cur = cur.sub(One::LOOP_BYTES);
                    }
                    generic::rev_byte_by_byte(start, cur, confirm)
                }
                /// Counts all occurrences of this byte in the given haystack represented
                /// by raw pointers.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `0` will always be returned.
                #[inline]
                pub unsafe fn count_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> usize {
                    if start >= end {
                        return 0;
                    }
                    let mut ptr = start;
                    let mut count = 0;
                    while ptr < end {
                        count += (ptr.read() == self.s1) as usize;
                        ptr = ptr.offset(1);
                    }
                    count
                }
                /// Returns an iterator over all occurrences of the needle byte in the
                /// given haystack.
                ///
                /// The iterator returned implements `DoubleEndedIterator`. This means it
                /// can also be used to find occurrences in reverse order.
                pub fn iter<'a, 'h>(&'a self, haystack: &'h [u8]) -> OneIter<'a, 'h> {
                    OneIter {
                        searcher: self,
                        it: generic::Iter::new(haystack),
                    }
                }
                #[inline(always)]
                fn has_needle(&self, chunk: usize) -> bool {
                    has_zero_byte(self.v1 ^ chunk)
                }
                #[inline(always)]
                fn confirm(&self, haystack_byte: u8) -> bool {
                    self.s1 == haystack_byte
                }
            }
            /// An iterator over all occurrences of a single byte in a haystack.
            ///
            /// This iterator implements `DoubleEndedIterator`, which means it can also be
            /// used to find occurrences in reverse order.
            ///
            /// This iterator is created by the [`One::iter`] method.
            ///
            /// The lifetime parameters are as follows:
            ///
            /// * `'a` refers to the lifetime of the underlying [`One`] searcher.
            /// * `'h` refers to the lifetime of the haystack being searched.
            pub struct OneIter<'a, 'h> {
                /// The underlying memchr searcher.
                searcher: &'a One,
                /// Generic iterator implementation.
                it: generic::Iter<'h>,
            }
            #[automatically_derived]
            impl<'a, 'h> ::core::clone::Clone for OneIter<'a, 'h> {
                #[inline]
                fn clone(&self) -> OneIter<'a, 'h> {
                    OneIter {
                        searcher: ::core::clone::Clone::clone(&self.searcher),
                        it: ::core::clone::Clone::clone(&self.it),
                    }
                }
            }
            #[automatically_derived]
            impl<'a, 'h> ::core::fmt::Debug for OneIter<'a, 'h> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "OneIter",
                        "searcher",
                        &self.searcher,
                        "it",
                        &&self.it,
                    )
                }
            }
            impl<'a, 'h> Iterator for OneIter<'a, 'h> {
                type Item = usize;
                #[inline]
                fn next(&mut self) -> Option<usize> {
                    unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                }
                #[inline]
                fn count(self) -> usize {
                    self.it.count(|s, e| { unsafe { self.searcher.count_raw(s, e) } })
                }
                #[inline]
                fn size_hint(&self) -> (usize, Option<usize>) {
                    self.it.size_hint()
                }
            }
            impl<'a, 'h> DoubleEndedIterator for OneIter<'a, 'h> {
                #[inline]
                fn next_back(&mut self) -> Option<usize> {
                    unsafe { self.it.next_back(|s, e| self.searcher.rfind_raw(s, e)) }
                }
            }
            /// Finds all occurrences of two bytes in a haystack.
            ///
            /// That is, this reports matches of one of two possible bytes. For example,
            /// searching for `a` or `b` in `afoobar` would report matches at offsets `0`,
            /// `4` and `5`.
            pub struct Two {
                s1: u8,
                s2: u8,
                v1: usize,
                v2: usize,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Two {}
            #[automatically_derived]
            impl ::core::clone::Clone for Two {
                #[inline]
                fn clone(&self) -> Two {
                    let _: ::core::clone::AssertParamIsClone<u8>;
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Two {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Two {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field4_finish(
                        f,
                        "Two",
                        "s1",
                        &self.s1,
                        "s2",
                        &self.s2,
                        "v1",
                        &self.v1,
                        "v2",
                        &&self.v2,
                    )
                }
            }
            impl Two {
                /// Create a new searcher that finds occurrences of the two needle bytes
                /// given.
                #[inline]
                pub fn new(needle1: u8, needle2: u8) -> Two {
                    Two {
                        s1: needle1,
                        s2: needle2,
                        v1: splat(needle1),
                        v2: splat(needle2),
                    }
                }
                /// Return the first occurrence of one of the needle bytes in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// The occurrence is reported as an offset into `haystack`. Its maximum
                /// value for a non-empty haystack is `haystack.len() - 1`.
                #[inline]
                pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                    unsafe {
                        generic::search_slice_with_raw(
                            haystack,
                            |s, e| { self.find_raw(s, e) },
                        )
                    }
                }
                /// Return the last occurrence of one of the needle bytes in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// The occurrence is reported as an offset into `haystack`. Its maximum
                /// value for a non-empty haystack is `haystack.len() - 1`.
                #[inline]
                pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                    unsafe {
                        generic::search_slice_with_raw(
                            haystack,
                            |s, e| { self.rfind_raw(s, e) },
                        )
                    }
                }
                /// Like `find`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `None` will always be returned.
                #[inline]
                pub unsafe fn find_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if start >= end {
                        return None;
                    }
                    let confirm = |b| self.confirm(b);
                    let len = end.distance(start);
                    if len < USIZE_BYTES {
                        return generic::fwd_byte_by_byte(start, end, confirm);
                    }
                    let chunk = start.cast::<usize>().read_unaligned();
                    if self.has_needle(chunk) {
                        return generic::fwd_byte_by_byte(start, end, confirm);
                    }
                    let mut cur = start
                        .add(USIZE_BYTES - (start.as_usize() & USIZE_ALIGN));
                    if true {
                        if !(cur > start) {
                            ::core::panicking::panic("assertion failed: cur > start")
                        }
                    }
                    if true {
                        if !(end.sub(USIZE_BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: end.sub(USIZE_BYTES) >= start",
                            )
                        }
                    }
                    while cur <= end.sub(USIZE_BYTES) {
                        if true {
                            match (&0, &(cur.as_usize() % USIZE_BYTES)) {
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
                        let chunk = cur.cast::<usize>().read();
                        if self.has_needle(chunk) {
                            break;
                        }
                        cur = cur.add(USIZE_BYTES);
                    }
                    generic::fwd_byte_by_byte(cur, end, confirm)
                }
                /// Like `rfind`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `None` will always be returned.
                #[inline]
                pub unsafe fn rfind_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if start >= end {
                        return None;
                    }
                    let confirm = |b| self.confirm(b);
                    let len = end.distance(start);
                    if len < USIZE_BYTES {
                        return generic::rev_byte_by_byte(start, end, confirm);
                    }
                    let chunk = end.sub(USIZE_BYTES).cast::<usize>().read_unaligned();
                    if self.has_needle(chunk) {
                        return generic::rev_byte_by_byte(start, end, confirm);
                    }
                    let mut cur = end.sub(end.as_usize() & USIZE_ALIGN);
                    if true {
                        if !(start <= cur && cur <= end) {
                            ::core::panicking::panic(
                                "assertion failed: start <= cur && cur <= end",
                            )
                        }
                    }
                    while cur >= start.add(USIZE_BYTES) {
                        if true {
                            match (&0, &(cur.as_usize() % USIZE_BYTES)) {
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
                        let chunk = cur.sub(USIZE_BYTES).cast::<usize>().read();
                        if self.has_needle(chunk) {
                            break;
                        }
                        cur = cur.sub(USIZE_BYTES);
                    }
                    generic::rev_byte_by_byte(start, cur, confirm)
                }
                /// Returns an iterator over all occurrences of one of the needle bytes in
                /// the given haystack.
                ///
                /// The iterator returned implements `DoubleEndedIterator`. This means it
                /// can also be used to find occurrences in reverse order.
                pub fn iter<'a, 'h>(&'a self, haystack: &'h [u8]) -> TwoIter<'a, 'h> {
                    TwoIter {
                        searcher: self,
                        it: generic::Iter::new(haystack),
                    }
                }
                #[inline(always)]
                fn has_needle(&self, chunk: usize) -> bool {
                    has_zero_byte(self.v1 ^ chunk) || has_zero_byte(self.v2 ^ chunk)
                }
                #[inline(always)]
                fn confirm(&self, haystack_byte: u8) -> bool {
                    self.s1 == haystack_byte || self.s2 == haystack_byte
                }
            }
            /// An iterator over all occurrences of two possible bytes in a haystack.
            ///
            /// This iterator implements `DoubleEndedIterator`, which means it can also be
            /// used to find occurrences in reverse order.
            ///
            /// This iterator is created by the [`Two::iter`] method.
            ///
            /// The lifetime parameters are as follows:
            ///
            /// * `'a` refers to the lifetime of the underlying [`Two`] searcher.
            /// * `'h` refers to the lifetime of the haystack being searched.
            pub struct TwoIter<'a, 'h> {
                /// The underlying memchr searcher.
                searcher: &'a Two,
                /// Generic iterator implementation.
                it: generic::Iter<'h>,
            }
            #[automatically_derived]
            impl<'a, 'h> ::core::clone::Clone for TwoIter<'a, 'h> {
                #[inline]
                fn clone(&self) -> TwoIter<'a, 'h> {
                    TwoIter {
                        searcher: ::core::clone::Clone::clone(&self.searcher),
                        it: ::core::clone::Clone::clone(&self.it),
                    }
                }
            }
            #[automatically_derived]
            impl<'a, 'h> ::core::fmt::Debug for TwoIter<'a, 'h> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "TwoIter",
                        "searcher",
                        &self.searcher,
                        "it",
                        &&self.it,
                    )
                }
            }
            impl<'a, 'h> Iterator for TwoIter<'a, 'h> {
                type Item = usize;
                #[inline]
                fn next(&mut self) -> Option<usize> {
                    unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                }
                #[inline]
                fn size_hint(&self) -> (usize, Option<usize>) {
                    self.it.size_hint()
                }
            }
            impl<'a, 'h> DoubleEndedIterator for TwoIter<'a, 'h> {
                #[inline]
                fn next_back(&mut self) -> Option<usize> {
                    unsafe { self.it.next_back(|s, e| self.searcher.rfind_raw(s, e)) }
                }
            }
            /// Finds all occurrences of three bytes in a haystack.
            ///
            /// That is, this reports matches of one of three possible bytes. For example,
            /// searching for `a`, `b` or `o` in `afoobar` would report matches at offsets
            /// `0`, `2`, `3`, `4` and `5`.
            pub struct Three {
                s1: u8,
                s2: u8,
                s3: u8,
                v1: usize,
                v2: usize,
                v3: usize,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Three {}
            #[automatically_derived]
            impl ::core::clone::Clone for Three {
                #[inline]
                fn clone(&self) -> Three {
                    let _: ::core::clone::AssertParamIsClone<u8>;
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Three {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Three {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let names: &'static _ = &["s1", "s2", "s3", "v1", "v2", "v3"];
                    let values: &[&dyn ::core::fmt::Debug] = &[
                        &self.s1,
                        &self.s2,
                        &self.s3,
                        &self.v1,
                        &self.v2,
                        &&self.v3,
                    ];
                    ::core::fmt::Formatter::debug_struct_fields_finish(
                        f,
                        "Three",
                        names,
                        values,
                    )
                }
            }
            impl Three {
                /// Create a new searcher that finds occurrences of the three needle bytes
                /// given.
                #[inline]
                pub fn new(needle1: u8, needle2: u8, needle3: u8) -> Three {
                    Three {
                        s1: needle1,
                        s2: needle2,
                        s3: needle3,
                        v1: splat(needle1),
                        v2: splat(needle2),
                        v3: splat(needle3),
                    }
                }
                /// Return the first occurrence of one of the needle bytes in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// The occurrence is reported as an offset into `haystack`. Its maximum
                /// value for a non-empty haystack is `haystack.len() - 1`.
                #[inline]
                pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                    unsafe {
                        generic::search_slice_with_raw(
                            haystack,
                            |s, e| { self.find_raw(s, e) },
                        )
                    }
                }
                /// Return the last occurrence of one of the needle bytes in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// The occurrence is reported as an offset into `haystack`. Its maximum
                /// value for a non-empty haystack is `haystack.len() - 1`.
                #[inline]
                pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                    unsafe {
                        generic::search_slice_with_raw(
                            haystack,
                            |s, e| { self.rfind_raw(s, e) },
                        )
                    }
                }
                /// Like `find`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `None` will always be returned.
                #[inline]
                pub unsafe fn find_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if start >= end {
                        return None;
                    }
                    let confirm = |b| self.confirm(b);
                    let len = end.distance(start);
                    if len < USIZE_BYTES {
                        return generic::fwd_byte_by_byte(start, end, confirm);
                    }
                    let chunk = start.cast::<usize>().read_unaligned();
                    if self.has_needle(chunk) {
                        return generic::fwd_byte_by_byte(start, end, confirm);
                    }
                    let mut cur = start
                        .add(USIZE_BYTES - (start.as_usize() & USIZE_ALIGN));
                    if true {
                        if !(cur > start) {
                            ::core::panicking::panic("assertion failed: cur > start")
                        }
                    }
                    if true {
                        if !(end.sub(USIZE_BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: end.sub(USIZE_BYTES) >= start",
                            )
                        }
                    }
                    while cur <= end.sub(USIZE_BYTES) {
                        if true {
                            match (&0, &(cur.as_usize() % USIZE_BYTES)) {
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
                        let chunk = cur.cast::<usize>().read();
                        if self.has_needle(chunk) {
                            break;
                        }
                        cur = cur.add(USIZE_BYTES);
                    }
                    generic::fwd_byte_by_byte(cur, end, confirm)
                }
                /// Like `rfind`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                ///
                /// Note that callers may pass a pair of pointers such that `start >= end`.
                /// In that case, `None` will always be returned.
                #[inline]
                pub unsafe fn rfind_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if start >= end {
                        return None;
                    }
                    let confirm = |b| self.confirm(b);
                    let len = end.distance(start);
                    if len < USIZE_BYTES {
                        return generic::rev_byte_by_byte(start, end, confirm);
                    }
                    let chunk = end.sub(USIZE_BYTES).cast::<usize>().read_unaligned();
                    if self.has_needle(chunk) {
                        return generic::rev_byte_by_byte(start, end, confirm);
                    }
                    let mut cur = end.sub(end.as_usize() & USIZE_ALIGN);
                    if true {
                        if !(start <= cur && cur <= end) {
                            ::core::panicking::panic(
                                "assertion failed: start <= cur && cur <= end",
                            )
                        }
                    }
                    while cur >= start.add(USIZE_BYTES) {
                        if true {
                            match (&0, &(cur.as_usize() % USIZE_BYTES)) {
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
                        let chunk = cur.sub(USIZE_BYTES).cast::<usize>().read();
                        if self.has_needle(chunk) {
                            break;
                        }
                        cur = cur.sub(USIZE_BYTES);
                    }
                    generic::rev_byte_by_byte(start, cur, confirm)
                }
                /// Returns an iterator over all occurrences of one of the needle bytes in
                /// the given haystack.
                ///
                /// The iterator returned implements `DoubleEndedIterator`. This means it
                /// can also be used to find occurrences in reverse order.
                pub fn iter<'a, 'h>(&'a self, haystack: &'h [u8]) -> ThreeIter<'a, 'h> {
                    ThreeIter {
                        searcher: self,
                        it: generic::Iter::new(haystack),
                    }
                }
                #[inline(always)]
                fn has_needle(&self, chunk: usize) -> bool {
                    has_zero_byte(self.v1 ^ chunk) || has_zero_byte(self.v2 ^ chunk)
                        || has_zero_byte(self.v3 ^ chunk)
                }
                #[inline(always)]
                fn confirm(&self, haystack_byte: u8) -> bool {
                    self.s1 == haystack_byte || self.s2 == haystack_byte
                        || self.s3 == haystack_byte
                }
            }
            /// An iterator over all occurrences of three possible bytes in a haystack.
            ///
            /// This iterator implements `DoubleEndedIterator`, which means it can also be
            /// used to find occurrences in reverse order.
            ///
            /// This iterator is created by the [`Three::iter`] method.
            ///
            /// The lifetime parameters are as follows:
            ///
            /// * `'a` refers to the lifetime of the underlying [`Three`] searcher.
            /// * `'h` refers to the lifetime of the haystack being searched.
            pub struct ThreeIter<'a, 'h> {
                /// The underlying memchr searcher.
                searcher: &'a Three,
                /// Generic iterator implementation.
                it: generic::Iter<'h>,
            }
            #[automatically_derived]
            impl<'a, 'h> ::core::clone::Clone for ThreeIter<'a, 'h> {
                #[inline]
                fn clone(&self) -> ThreeIter<'a, 'h> {
                    ThreeIter {
                        searcher: ::core::clone::Clone::clone(&self.searcher),
                        it: ::core::clone::Clone::clone(&self.it),
                    }
                }
            }
            #[automatically_derived]
            impl<'a, 'h> ::core::fmt::Debug for ThreeIter<'a, 'h> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "ThreeIter",
                        "searcher",
                        &self.searcher,
                        "it",
                        &&self.it,
                    )
                }
            }
            impl<'a, 'h> Iterator for ThreeIter<'a, 'h> {
                type Item = usize;
                #[inline]
                fn next(&mut self) -> Option<usize> {
                    unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                }
                #[inline]
                fn size_hint(&self) -> (usize, Option<usize>) {
                    self.it.size_hint()
                }
            }
            impl<'a, 'h> DoubleEndedIterator for ThreeIter<'a, 'h> {
                #[inline]
                fn next_back(&mut self) -> Option<usize> {
                    unsafe { self.it.next_back(|s, e| self.searcher.rfind_raw(s, e)) }
                }
            }
            /// Return `true` if `x` contains any zero byte.
            ///
            /// That is, this routine treats `x` as a register of 8-bit lanes and returns
            /// true when any of those lanes is `0`.
            ///
            /// From "Matters Computational" by J. Arndt.
            #[inline(always)]
            fn has_zero_byte(x: usize) -> bool {
                const LO: usize = splat(0x01);
                const HI: usize = splat(0x80);
                (x.wrapping_sub(LO) & !x & HI) != 0
            }
            /// Repeat the given byte into a word size number. That is, every 8 bits
            /// is equivalent to the given byte. For example, if `b` is `\x4E` or
            /// `01001110` in binary, then the returned value on a 32-bit system would be:
            /// `01001110_01001110_01001110_01001110`.
            #[inline(always)]
            const fn splat(b: u8) -> usize {
                (b as usize) * (usize::MAX / 255)
            }
        }
        pub mod packedpair {
            /*!
Provides an architecture independent implementation of the "packed pair"
algorithm.

The "packed pair" algorithm is based on the [generic SIMD] algorithm. The main
difference is that it (by default) uses a background distribution of byte
frequencies to heuristically select the pair of bytes to search for. Note that
this module provides an architecture independent version that doesn't do as
good of a job keeping the search for candidates inside a SIMD hot path. It
however can be good enough in many circumstances.

[generic SIMD]: http://0x80.pl/articles/simd-strfind.html#first-and-last
*/
            use crate::memchr;
            mod default_rank {
                pub(crate) const RANK: [u8; 256] = [
                    55, 52, 51, 50, 49, 48, 47, 46, 45, 103, 242, 66, 67, 229, 44, 43,
                    42, 41, 40, 39, 38, 37, 36, 35, 34, 33, 56, 32, 31, 30, 29, 28, 255,
                    148, 164, 149, 136, 160, 155, 173, 221, 222, 134, 122, 232, 202, 215,
                    224, 208, 220, 204, 187, 183, 179, 177, 168, 178, 200, 226, 195, 154,
                    184, 174, 126, 120, 191, 157, 194, 170, 189, 162, 161, 150, 193, 142,
                    137, 171, 176, 185, 167, 186, 112, 175, 192, 188, 156, 140, 143, 123,
                    133, 128, 147, 138, 146, 114, 223, 151, 249, 216, 238, 236, 253, 227,
                    218, 230, 247, 135, 180, 241, 233, 246, 244, 231, 139, 245, 243, 251,
                    235, 201, 196, 240, 214, 152, 182, 205, 181, 127, 27, 212, 211, 210,
                    213, 228, 197, 169, 159, 131, 172, 105, 80, 98, 96, 97, 81, 207, 145,
                    116, 115, 144, 130, 153, 121, 107, 132, 109, 110, 124, 111, 82, 108,
                    118, 141, 113, 129, 119, 125, 165, 117, 92, 106, 83, 72, 99, 93, 65,
                    79, 166, 237, 163, 199, 190, 225, 209, 203, 198, 217, 219, 206, 234,
                    248, 158, 239, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                ];
            }
            /// An architecture independent "packed pair" finder.
            ///
            /// This finder picks two bytes that it believes have high predictive power for
            /// indicating an overall match of a needle. At search time, it reports offsets
            /// where the needle could match based on whether the pair of bytes it chose
            /// match.
            ///
            /// This is architecture independent because it utilizes `memchr` to find the
            /// occurrence of one of the bytes in the pair, and then checks whether the
            /// second byte matches. If it does, in the case of [`Finder::find_prefilter`],
            /// the location at which the needle could match is returned.
            ///
            /// It is generally preferred to use architecture specific routines for a
            /// "packed pair" prefilter, but this can be a useful fallback when the
            /// architecture independent routines are unavailable.
            pub struct Finder {
                pair: Pair,
                byte1: u8,
                byte2: u8,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Finder {}
            #[automatically_derived]
            impl ::core::clone::Clone for Finder {
                #[inline]
                fn clone(&self) -> Finder {
                    let _: ::core::clone::AssertParamIsClone<Pair>;
                    let _: ::core::clone::AssertParamIsClone<u8>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Finder {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Finder {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field3_finish(
                        f,
                        "Finder",
                        "pair",
                        &self.pair,
                        "byte1",
                        &self.byte1,
                        "byte2",
                        &&self.byte2,
                    )
                }
            }
            impl Finder {
                /// Create a new prefilter that reports possible locations where the given
                /// needle matches.
                #[inline]
                pub fn new(needle: &[u8]) -> Option<Finder> {
                    Finder::with_pair(needle, Pair::new(needle)?)
                }
                /// Create a new prefilter using the pair given.
                ///
                /// If the prefilter could not be constructed, then `None` is returned.
                ///
                /// This constructor permits callers to control precisely which pair of
                /// bytes is used as a predicate.
                #[inline]
                pub fn with_pair(needle: &[u8], pair: Pair) -> Option<Finder> {
                    let byte1 = needle[usize::from(pair.index1())];
                    let byte2 = needle[usize::from(pair.index2())];
                    Some(Finder { pair, byte1, byte2 })
                }
                /// Run this finder on the given haystack as a prefilter.
                ///
                /// If a candidate match is found, then an offset where the needle *could*
                /// begin in the haystack is returned.
                #[inline]
                pub fn find_prefilter(&self, haystack: &[u8]) -> Option<usize> {
                    let mut i = 0;
                    let index1 = usize::from(self.pair.index1());
                    let index2 = usize::from(self.pair.index2());
                    loop {
                        i += memchr(self.byte1, &haystack[i..])?;
                        let found = i;
                        i += 1;
                        let aligned1 = match found.checked_sub(index1) {
                            None => continue,
                            Some(aligned1) => aligned1,
                        };
                        let aligned2 = match aligned1.checked_add(index2) {
                            None => continue,
                            Some(aligned_index2) => aligned_index2,
                        };
                        if haystack.get(aligned2).map_or(true, |&b| b != self.byte2) {
                            continue;
                        }
                        return Some(aligned1);
                    }
                }
                /// Returns the pair of offsets (into the needle) used to check as a
                /// predicate before confirming whether a needle exists at a particular
                /// position.
                #[inline]
                pub fn pair(&self) -> &Pair {
                    &self.pair
                }
            }
            /// A pair of byte offsets into a needle to use as a predicate.
            ///
            /// This pair is used as a predicate to quickly filter out positions in a
            /// haystack in which a needle cannot match. In some cases, this pair can even
            /// be used in vector algorithms such that the vector algorithm only switches
            /// over to scalar code once this pair has been found.
            ///
            /// A pair of offsets can be used in both substring search implementations and
            /// in prefilters. The former will report matches of a needle in a haystack
            /// where as the latter will only report possible matches of a needle.
            ///
            /// The offsets are limited each to a maximum of 255 to keep memory usage low.
            /// Moreover, it's rarely advantageous to create a predicate using offsets
            /// greater than 255 anyway.
            ///
            /// The only guarantee enforced on the pair of offsets is that they are not
            /// equivalent. It is not necessarily the case that `index1 < index2` for
            /// example. By convention, `index1` corresponds to the byte in the needle
            /// that is believed to be most the predictive. Note also that because of the
            /// requirement that the indices be both valid for the needle used to build
            /// the pair and not equal, it follows that a pair can only be constructed for
            /// needles with length at least 2.
            pub struct Pair {
                index1: u8,
                index2: u8,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Pair {}
            #[automatically_derived]
            impl ::core::clone::Clone for Pair {
                #[inline]
                fn clone(&self) -> Pair {
                    let _: ::core::clone::AssertParamIsClone<u8>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Pair {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Pair {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Pair",
                        "index1",
                        &self.index1,
                        "index2",
                        &&self.index2,
                    )
                }
            }
            impl Pair {
                /// Create a new pair of offsets from the given needle.
                ///
                /// If a pair could not be created (for example, if the needle is too
                /// short), then `None` is returned.
                ///
                /// This chooses the pair in the needle that is believed to be as
                /// predictive of an overall match of the needle as possible.
                #[inline]
                pub fn new(needle: &[u8]) -> Option<Pair> {
                    Pair::with_ranker(needle, DefaultFrequencyRank)
                }
                /// Create a new pair of offsets from the given needle and ranker.
                ///
                /// This permits the caller to choose a background frequency distribution
                /// with which bytes are selected. The idea is to select a pair of bytes
                /// that is believed to strongly predict a match in the haystack. This
                /// usually means selecting bytes that occur rarely in a haystack.
                ///
                /// If a pair could not be created (for example, if the needle is too
                /// short), then `None` is returned.
                #[inline]
                pub fn with_ranker<R: HeuristicFrequencyRank>(
                    needle: &[u8],
                    ranker: R,
                ) -> Option<Pair> {
                    if needle.len() <= 1 {
                        return None;
                    }
                    let (mut rare1, mut index1) = (needle[0], 0);
                    let (mut rare2, mut index2) = (needle[1], 1);
                    if ranker.rank(rare2) < ranker.rank(rare1) {
                        core::mem::swap(&mut rare1, &mut rare2);
                        core::mem::swap(&mut index1, &mut index2);
                    }
                    let max = usize::from(core::u8::MAX);
                    for (i, &b) in needle.iter().enumerate().take(max).skip(2) {
                        if ranker.rank(b) < ranker.rank(rare1) {
                            rare2 = rare1;
                            index2 = index1;
                            rare1 = b;
                            index1 = u8::try_from(i).unwrap();
                        } else if b != rare1 && ranker.rank(b) < ranker.rank(rare2) {
                            rare2 = b;
                            index2 = u8::try_from(i).unwrap();
                        }
                    }
                    match (&index1, &index2) {
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
                    Some(Pair { index1, index2 })
                }
                /// Create a new pair using the offsets given for the needle given.
                ///
                /// This bypasses any sort of heuristic process for choosing the offsets
                /// and permits the caller to choose the offsets themselves.
                ///
                /// Indices are limited to valid `u8` values so that a `Pair` uses less
                /// memory. It is not possible to create a `Pair` with offsets bigger than
                /// `u8::MAX`. It's likely that such a thing is not needed, but if it is,
                /// it's suggested to build your own bespoke algorithm because you're
                /// likely working on a very niche case. (File an issue if this suggestion
                /// does not make sense to you.)
                ///
                /// If a pair could not be created (for example, if the needle is too
                /// short), then `None` is returned.
                #[inline]
                pub fn with_indices(
                    needle: &[u8],
                    index1: u8,
                    index2: u8,
                ) -> Option<Pair> {
                    if index1 == index2 {
                        return None;
                    }
                    if usize::from(index1) >= needle.len() {
                        return None;
                    }
                    if usize::from(index2) >= needle.len() {
                        return None;
                    }
                    Some(Pair { index1, index2 })
                }
                /// Returns the first offset of the pair.
                #[inline]
                pub fn index1(&self) -> u8 {
                    self.index1
                }
                /// Returns the second offset of the pair.
                #[inline]
                pub fn index2(&self) -> u8 {
                    self.index2
                }
            }
            /// This trait allows the user to customize the heuristic used to determine the
            /// relative frequency of a given byte in the dataset being searched.
            ///
            /// The use of this trait can have a dramatic impact on performance depending
            /// on the type of data being searched. The details of why are explained in the
            /// docs of [`crate::memmem::Prefilter`]. To summarize, the core algorithm uses
            /// a prefilter to quickly identify candidate matches that are later verified
            /// more slowly. This prefilter is implemented in terms of trying to find
            /// `rare` bytes at specific offsets that will occur less frequently in the
            /// dataset. While the concept of a `rare` byte is similar for most datasets,
            /// there are some specific datasets (like binary executables) that have
            /// dramatically different byte distributions. For these datasets customizing
            /// the byte frequency heuristic can have a massive impact on performance, and
            /// might even need to be done at runtime.
            ///
            /// The default implementation of `HeuristicFrequencyRank` reads from the
            /// static frequency table defined in `src/memmem/byte_frequencies.rs`. This
            /// is optimal for most inputs, so if you are unsure of the impact of using a
            /// custom `HeuristicFrequencyRank` you should probably just use the default.
            ///
            /// # Example
            ///
            /// ```
            /// use memchr::{
            ///     arch::all::packedpair::HeuristicFrequencyRank,
            ///     memmem::FinderBuilder,
            /// };
            ///
            /// /// A byte-frequency table that is good for scanning binary executables.
            /// struct Binary;
            ///
            /// impl HeuristicFrequencyRank for Binary {
            ///     fn rank(&self, byte: u8) -> u8 {
            ///         const TABLE: [u8; 256] = [
            ///             255, 128, 61, 43, 50, 41, 27, 28, 57, 15, 21, 13, 24, 17, 17,
            ///             89, 58, 16, 11, 7, 14, 23, 7, 6, 24, 9, 6, 5, 9, 4, 7, 16,
            ///             68, 11, 9, 6, 88, 7, 4, 4, 23, 9, 4, 8, 8, 5, 10, 4, 30, 11,
            ///             9, 24, 11, 5, 5, 5, 19, 11, 6, 17, 9, 9, 6, 8,
            ///             48, 58, 11, 14, 53, 40, 9, 9, 254, 35, 3, 6, 52, 23, 6, 6, 27,
            ///             4, 7, 11, 14, 13, 10, 11, 11, 5, 2, 10, 16, 12, 6, 19,
            ///             19, 20, 5, 14, 16, 31, 19, 7, 14, 20, 4, 4, 19, 8, 18, 20, 24,
            ///             1, 25, 19, 58, 29, 10, 5, 15, 20, 2, 2, 9, 4, 3, 5,
            ///             51, 11, 4, 53, 23, 39, 6, 4, 13, 81, 4, 186, 5, 67, 3, 2, 15,
            ///             0, 0, 1, 3, 2, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0,
            ///             12, 2, 1, 1, 3, 1, 1, 1, 6, 1, 2, 1, 3, 1, 1, 2, 9, 1, 1, 0,
            ///             2, 2, 4, 4, 11, 6, 7, 3, 6, 9, 4, 5,
            ///             46, 18, 8, 18, 17, 3, 8, 20, 16, 10, 3, 7, 175, 4, 6, 7, 13,
            ///             3, 7, 3, 3, 1, 3, 3, 10, 3, 1, 5, 2, 0, 1, 2,
            ///             16, 3, 5, 1, 6, 1, 1, 2, 58, 20, 3, 14, 12, 2, 1, 3, 16, 3, 5,
            ///             8, 3, 1, 8, 6, 17, 6, 5, 3, 8, 6, 13, 175,
            ///         ];
            ///         TABLE[byte as usize]
            ///     }
            /// }
            /// // Create a new finder with the custom heuristic.
            /// let finder = FinderBuilder::new()
            ///     .build_forward_with_ranker(Binary, b"\x00\x00\xdd\xdd");
            /// // Find needle with custom heuristic.
            /// assert!(finder.find(b"\x00\x00\x00\xdd\xdd").is_some());
            /// ```
            pub trait HeuristicFrequencyRank {
                /// Return the heuristic frequency rank of the given byte. A lower rank
                /// means the byte is believed to occur less frequently in the haystack.
                ///
                /// Some uses of this heuristic may treat arbitrary absolute rank values as
                /// significant. For example, an implementation detail in this crate may
                /// determine that heuristic prefilters are inappropriate if every byte in
                /// the needle has a "high" rank.
                fn rank(&self, byte: u8) -> u8;
            }
            /// The default byte frequency heuristic that is good for most haystacks.
            pub(crate) struct DefaultFrequencyRank;
            impl HeuristicFrequencyRank for DefaultFrequencyRank {
                fn rank(&self, byte: u8) -> u8 {
                    self::default_rank::RANK[usize::from(byte)]
                }
            }
            /// This permits passing any implementation of `HeuristicFrequencyRank` as a
            /// borrowed version of itself.
            impl<'a, R> HeuristicFrequencyRank for &'a R
            where
                R: HeuristicFrequencyRank,
            {
                fn rank(&self, byte: u8) -> u8 {
                    (**self).rank(byte)
                }
            }
        }
        pub mod rabinkarp {
            /*!
An implementation of the [Rabin-Karp substring search algorithm][rabinkarp].

Rabin-Karp works by creating a hash of the needle provided and then computing
a rolling hash for each needle sized window in the haystack. When the rolling
hash matches the hash of the needle, a byte-wise comparison is done to check
if a match exists. The worst case time complexity of Rabin-Karp is `O(m *
n)` where `m ~ len(needle)` and `n ~ len(haystack)`. Its worst case space
complexity is constant.

The main utility of Rabin-Karp is that the searcher can be constructed very
quickly with very little memory. This makes it especially useful when searching
for small needles in small haystacks, as it might finish its search before a
beefier algorithm (like Two-Way) even starts.

[rabinkarp]: https://en.wikipedia.org/wiki/Rabin%E2%80%93Karp_algorithm
*/
            use crate::ext::Pointer;
            /// A forward substring searcher using the Rabin-Karp algorithm.
            ///
            /// Note that, as a lower level API, a `Finder` does not have access to the
            /// needle it was constructed with. For this reason, executing a search
            /// with a `Finder` requires passing both the needle and the haystack,
            /// where the needle is exactly equivalent to the one given to the `Finder`
            /// at construction time. This design was chosen so that callers can have
            /// more precise control over where and how many times a needle is stored.
            /// For example, in cases where Rabin-Karp is just one of several possible
            /// substring search algorithms.
            pub struct Finder {
                /// The actual hash.
                hash: Hash,
                /// The factor needed to multiply a byte by in order to subtract it from
                /// the hash. It is defined to be 2^(n-1) (using wrapping exponentiation),
                /// where n is the length of the needle. This is how we "remove" a byte
                /// from the hash once the hash window rolls past it.
                hash_2pow: u32,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Finder {
                #[inline]
                fn clone(&self) -> Finder {
                    Finder {
                        hash: ::core::clone::Clone::clone(&self.hash),
                        hash_2pow: ::core::clone::Clone::clone(&self.hash_2pow),
                    }
                }
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Finder {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Finder",
                        "hash",
                        &self.hash,
                        "hash_2pow",
                        &&self.hash_2pow,
                    )
                }
            }
            impl Finder {
                /// Create a new Rabin-Karp forward searcher for the given `needle`.
                ///
                /// The needle may be empty. The empty needle matches at every byte offset.
                ///
                /// Note that callers must pass the same needle to all search calls using
                /// this `Finder`.
                #[inline]
                pub fn new(needle: &[u8]) -> Finder {
                    let mut s = Finder {
                        hash: Hash::new(),
                        hash_2pow: 1,
                    };
                    let first_byte = match needle.get(0) {
                        None => return s,
                        Some(&first_byte) => first_byte,
                    };
                    s.hash.add(first_byte);
                    for b in needle.iter().copied().skip(1) {
                        s.hash.add(b);
                        s.hash_2pow = s.hash_2pow.wrapping_shl(1);
                    }
                    s
                }
                /// Return the first occurrence of the `needle` in the `haystack`
                /// given. If no such occurrence exists, then `None` is returned.
                ///
                /// The `needle` provided must match the needle given to this finder at
                /// construction time.
                ///
                /// The maximum value this can return is `haystack.len()`, which can only
                /// occur when the needle and haystack both have length zero. Otherwise,
                /// for non-empty haystacks, the maximum value is `haystack.len() - 1`.
                #[inline]
                pub fn find(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                    unsafe {
                        let hstart = haystack.as_ptr();
                        let hend = hstart.add(haystack.len());
                        let nstart = needle.as_ptr();
                        let nend = nstart.add(needle.len());
                        let found = self.find_raw(hstart, hend, nstart, nend)?;
                        Some(found.distance(hstart))
                    }
                }
                /// Like `find`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `<= end`. The pointer returned is only ever equivalent
                /// to `end` when both the needle and haystack are empty. (That is, the
                /// empty string matches the empty string.)
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// Note that `start` and `end` below refer to both pairs of pointers given
                /// to this routine. That is, the conditions apply to both `hstart`/`hend`
                /// and `nstart`/`nend`.
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                /// * It must be the case that `start <= end`.
                #[inline]
                pub unsafe fn find_raw(
                    &self,
                    hstart: *const u8,
                    hend: *const u8,
                    nstart: *const u8,
                    nend: *const u8,
                ) -> Option<*const u8> {
                    let hlen = hend.distance(hstart);
                    let nlen = nend.distance(nstart);
                    if nlen > hlen {
                        return None;
                    }
                    let mut cur = hstart;
                    let end = hend.sub(nlen);
                    let mut hash = Hash::forward(cur, cur.add(nlen));
                    loop {
                        if self.hash == hash && is_equal_raw(cur, nstart, nlen) {
                            return Some(cur);
                        }
                        if cur >= end {
                            return None;
                        }
                        hash.roll(self, cur.read(), cur.add(nlen).read());
                        cur = cur.add(1);
                    }
                }
            }
            /// A reverse substring searcher using the Rabin-Karp algorithm.
            pub struct FinderRev(Finder);
            #[automatically_derived]
            impl ::core::clone::Clone for FinderRev {
                #[inline]
                fn clone(&self) -> FinderRev {
                    FinderRev(::core::clone::Clone::clone(&self.0))
                }
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for FinderRev {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "FinderRev",
                        &&self.0,
                    )
                }
            }
            impl FinderRev {
                /// Create a new Rabin-Karp reverse searcher for the given `needle`.
                #[inline]
                pub fn new(needle: &[u8]) -> FinderRev {
                    let mut s = FinderRev(Finder {
                        hash: Hash::new(),
                        hash_2pow: 1,
                    });
                    let last_byte = match needle.last() {
                        None => return s,
                        Some(&last_byte) => last_byte,
                    };
                    s.0.hash.add(last_byte);
                    for b in needle.iter().rev().copied().skip(1) {
                        s.0.hash.add(b);
                        s.0.hash_2pow = s.0.hash_2pow.wrapping_shl(1);
                    }
                    s
                }
                /// Return the last occurrence of the `needle` in the `haystack`
                /// given. If no such occurrence exists, then `None` is returned.
                ///
                /// The `needle` provided must match the needle given to this finder at
                /// construction time.
                ///
                /// The maximum value this can return is `haystack.len()`, which can only
                /// occur when the needle and haystack both have length zero. Otherwise,
                /// for non-empty haystacks, the maximum value is `haystack.len() - 1`.
                #[inline]
                pub fn rfind(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                    unsafe {
                        let hstart = haystack.as_ptr();
                        let hend = hstart.add(haystack.len());
                        let nstart = needle.as_ptr();
                        let nend = nstart.add(needle.len());
                        let found = self.rfind_raw(hstart, hend, nstart, nend)?;
                        Some(found.distance(hstart))
                    }
                }
                /// Like `rfind`, but accepts and returns raw pointers.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `<= end`. The pointer returned is only ever equivalent
                /// to `end` when both the needle and haystack are empty. (That is, the
                /// empty string matches the empty string.)
                ///
                /// This routine is useful if you're already using raw pointers and would
                /// like to avoid converting back to a slice before executing a search.
                ///
                /// # Safety
                ///
                /// Note that `start` and `end` below refer to both pairs of pointers given
                /// to this routine. That is, the conditions apply to both `hstart`/`hend`
                /// and `nstart`/`nend`.
                ///
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                /// * It must be the case that `start <= end`.
                #[inline]
                pub unsafe fn rfind_raw(
                    &self,
                    hstart: *const u8,
                    hend: *const u8,
                    nstart: *const u8,
                    nend: *const u8,
                ) -> Option<*const u8> {
                    let hlen = hend.distance(hstart);
                    let nlen = nend.distance(nstart);
                    if nlen > hlen {
                        return None;
                    }
                    let mut cur = hend.sub(nlen);
                    let start = hstart;
                    let mut hash = Hash::reverse(cur, cur.add(nlen));
                    loop {
                        if self.0.hash == hash && is_equal_raw(cur, nstart, nlen) {
                            return Some(cur);
                        }
                        if cur <= start {
                            return None;
                        }
                        cur = cur.sub(1);
                        hash.roll(&self.0, cur.add(nlen).read(), cur.read());
                    }
                }
            }
            /// Whether RK is believed to be very fast for the given needle/haystack.
            #[inline]
            pub(crate) fn is_fast(haystack: &[u8], _needle: &[u8]) -> bool {
                haystack.len() < 16
            }
            /// A Rabin-Karp hash. This might represent the hash of a needle, or the hash
            /// of a rolling window in the haystack.
            struct Hash(u32);
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Hash {}
            #[automatically_derived]
            impl ::core::clone::Clone for Hash {
                #[inline]
                fn clone(&self) -> Hash {
                    let _: ::core::clone::AssertParamIsClone<u32>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Hash {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Hash {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Hash",
                        &&self.0,
                    )
                }
            }
            #[automatically_derived]
            impl ::core::default::Default for Hash {
                #[inline]
                fn default() -> Hash {
                    Hash(::core::default::Default::default())
                }
            }
            #[automatically_derived]
            impl ::core::cmp::Eq for Hash {
                #[inline]
                #[doc(hidden)]
                #[coverage(off)]
                fn assert_receiver_is_total_eq(&self) {
                    let _: ::core::cmp::AssertParamIsEq<u32>;
                }
            }
            #[automatically_derived]
            impl ::core::marker::StructuralPartialEq for Hash {}
            #[automatically_derived]
            impl ::core::cmp::PartialEq for Hash {
                #[inline]
                fn eq(&self, other: &Hash) -> bool {
                    self.0 == other.0
                }
            }
            impl Hash {
                /// Create a new hash that represents the empty string.
                #[inline(always)]
                fn new() -> Hash {
                    Hash(0)
                }
                /// Create a new hash from the bytes given for use in forward searches.
                ///
                /// # Safety
                ///
                /// The given pointers must be valid to read from within their range.
                #[inline(always)]
                unsafe fn forward(mut start: *const u8, end: *const u8) -> Hash {
                    let mut hash = Hash::new();
                    while start < end {
                        hash.add(start.read());
                        start = start.add(1);
                    }
                    hash
                }
                /// Create a new hash from the bytes given for use in reverse searches.
                ///
                /// # Safety
                ///
                /// The given pointers must be valid to read from within their range.
                #[inline(always)]
                unsafe fn reverse(start: *const u8, mut end: *const u8) -> Hash {
                    let mut hash = Hash::new();
                    while start < end {
                        end = end.sub(1);
                        hash.add(end.read());
                    }
                    hash
                }
                /// Add 'new' and remove 'old' from this hash. The given needle hash should
                /// correspond to the hash computed for the needle being searched for.
                ///
                /// This is meant to be used when the rolling window of the haystack is
                /// advanced.
                #[inline(always)]
                fn roll(&mut self, finder: &Finder, old: u8, new: u8) {
                    self.del(finder, old);
                    self.add(new);
                }
                /// Add a byte to this hash.
                #[inline(always)]
                fn add(&mut self, byte: u8) {
                    self.0 = self.0.wrapping_shl(1).wrapping_add(u32::from(byte));
                }
                /// Remove a byte from this hash. The given needle hash should correspond
                /// to the hash computed for the needle being searched for.
                #[inline(always)]
                fn del(&mut self, finder: &Finder, byte: u8) {
                    let factor = finder.hash_2pow;
                    self.0 = self.0.wrapping_sub(u32::from(byte).wrapping_mul(factor));
                }
            }
            /// Returns true when `x[i] == y[i]` for all `0 <= i < n`.
            ///
            /// We forcefully don't inline this to hint at the compiler that it is unlikely
            /// to be called. This causes the inner rabinkarp loop above to be a bit
            /// tighter and leads to some performance improvement. See the
            /// memmem/krate/prebuilt/sliceslice-words/words benchmark.
            ///
            /// # Safety
            ///
            /// Same as `crate::arch::all::is_equal_raw`.
            #[cold]
            #[inline(never)]
            unsafe fn is_equal_raw(x: *const u8, y: *const u8, n: usize) -> bool {
                crate::arch::all::is_equal_raw(x, y, n)
            }
        }
        pub mod shiftor {
            /*!
An implementation of the [Shift-Or substring search algorithm][shiftor].

[shiftor]: https://en.wikipedia.org/wiki/Bitap_algorithm
*/
            use alloc::boxed::Box;
            /// The type of our mask.
            ///
            /// While we don't expose anyway to configure this in the public API, if one
            /// really needs less memory usage or support for longer needles, then it is
            /// suggested to copy the code from this module and modify it to fit your
            /// needs. The code below is written to be correct regardless of whether Mask
            /// is a u8, u16, u32, u64 or u128.
            type Mask = u16;
            /// A forward substring searcher using the Shift-Or algorithm.
            pub struct Finder {
                masks: Box<[Mask; 256]>,
                needle_len: usize,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Finder {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Finder",
                        "masks",
                        &self.masks,
                        "needle_len",
                        &&self.needle_len,
                    )
                }
            }
            impl Finder {
                const MAX_NEEDLE_LEN: usize = (Mask::BITS - 1) as usize;
                /// Create a new Shift-Or forward searcher for the given `needle`.
                ///
                /// The needle may be empty. The empty needle matches at every byte offset.
                #[inline]
                pub fn new(needle: &[u8]) -> Option<Finder> {
                    let needle_len = needle.len();
                    if needle_len > Finder::MAX_NEEDLE_LEN {
                        return None;
                    }
                    let mut searcher = Finder {
                        masks: Box::from([!0; 256]),
                        needle_len,
                    };
                    for (i, &byte) in needle.iter().enumerate() {
                        searcher.masks[usize::from(byte)] &= !(1 << i);
                    }
                    Some(searcher)
                }
                /// Return the first occurrence of the needle given to `Finder::new` in
                /// the `haystack` given. If no such occurrence exists, then `None` is
                /// returned.
                ///
                /// Unlike most other substring search implementations in this crate, this
                /// finder does not require passing the needle at search time. A match can
                /// be determined without the needle at all since the required information
                /// is already encoded into this finder at construction time.
                ///
                /// The maximum value this can return is `haystack.len()`, which can only
                /// occur when the needle and haystack both have length zero. Otherwise,
                /// for non-empty haystacks, the maximum value is `haystack.len() - 1`.
                #[inline]
                pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                    if self.needle_len == 0 {
                        return Some(0);
                    }
                    let mut result = !1;
                    for (i, &byte) in haystack.iter().enumerate() {
                        result |= self.masks[usize::from(byte)];
                        result <<= 1;
                        if result & (1 << self.needle_len) == 0 {
                            return Some(i + 1 - self.needle_len);
                        }
                    }
                    None
                }
            }
        }
        pub mod twoway {
            /*!
An implementation of the [Two-Way substring search algorithm][two-way].

[`Finder`] can be built for forward searches, while [`FinderRev`] can be built
for reverse searches.

Two-Way makes for a nice general purpose substring search algorithm because of
its time and space complexity properties. It also performs well in practice.
Namely, with `m = len(needle)` and `n = len(haystack)`, Two-Way takes `O(m)`
time to create a finder, `O(1)` space and `O(n)` search time. In other words,
the preprocessing step is quick, doesn't require any heap memory and the worst
case search time is guaranteed to be linear in the haystack regardless of the
size of the needle.

While vector algorithms will usually beat Two-Way handedly, vector algorithms
also usually have pathological or edge cases that are better handled by Two-Way.
Moreover, not all targets support vector algorithms or implementations for them
simply may not exist yet.

Two-Way can be found in the `memmem` implementations in at least [GNU libc] and
[musl].

[two-way]: https://en.wikipedia.org/wiki/Two-way_string-matching_algorithm
[GNU libc]: https://www.gnu.org/software/libc/
[musl]: https://www.musl-libc.org/
*/
            use core::cmp;
            use crate::{
                arch::all::{is_prefix, is_suffix},
                memmem::Pre,
            };
            /// A forward substring searcher that uses the Two-Way algorithm.
            pub struct Finder(TwoWay);
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Finder {}
            #[automatically_derived]
            impl ::core::clone::Clone for Finder {
                #[inline]
                fn clone(&self) -> Finder {
                    let _: ::core::clone::AssertParamIsClone<TwoWay>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Finder {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Finder {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Finder",
                        &&self.0,
                    )
                }
            }
            /// A reverse substring searcher that uses the Two-Way algorithm.
            pub struct FinderRev(TwoWay);
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for FinderRev {}
            #[automatically_derived]
            impl ::core::clone::Clone for FinderRev {
                #[inline]
                fn clone(&self) -> FinderRev {
                    let _: ::core::clone::AssertParamIsClone<TwoWay>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for FinderRev {}
            #[automatically_derived]
            impl ::core::fmt::Debug for FinderRev {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "FinderRev",
                        &&self.0,
                    )
                }
            }
            /// An implementation of the TwoWay substring search algorithm.
            ///
            /// This searcher supports forward and reverse search, although not
            /// simultaneously. It runs in `O(n + m)` time and `O(1)` space, where
            /// `n ~ len(needle)` and `m ~ len(haystack)`.
            ///
            /// The implementation here roughly matches that which was developed by
            /// Crochemore and Perrin in their 1991 paper "Two-way string-matching." The
            /// changes in this implementation are 1) the use of zero-based indices, 2) a
            /// heuristic skip table based on the last byte (borrowed from Rust's standard
            /// library) and 3) the addition of heuristics for a fast skip loop. For (3),
            /// callers can pass any kind of prefilter they want, but usually it's one
            /// based on a heuristic that uses an approximate background frequency of bytes
            /// to choose rare bytes to quickly look for candidate match positions. Note
            /// though that currently, this prefilter functionality is not exposed directly
            /// in the public API. (File an issue if you want it and provide a use case
            /// please.)
            ///
            /// The heuristic for fast skipping is automatically shut off if it's
            /// detected to be ineffective at search time. Generally, this only occurs in
            /// pathological cases. But this is generally necessary in order to preserve
            /// a `O(n + m)` time bound.
            ///
            /// The code below is fairly complex and not obviously correct at all. It's
            /// likely necessary to read the Two-Way paper cited above in order to fully
            /// grok this code. The essence of it is:
            ///
            /// 1. Do something to detect a "critical" position in the needle.
            /// 2. For the current position in the haystack, look if `needle[critical..]`
            /// matches at that position.
            /// 3. If so, look if `needle[..critical]` matches.
            /// 4. If a mismatch occurs, shift the search by some amount based on the
            /// critical position and a pre-computed shift.
            ///
            /// This type is wrapped in the forward and reverse finders that expose
            /// consistent forward or reverse APIs.
            struct TwoWay {
                /// A small bitset used as a quick prefilter (in addition to any prefilter
                /// given by the caller). Namely, a bit `i` is set if and only if `b%64==i`
                /// for any `b == needle[i]`.
                ///
                /// When used as a prefilter, if the last byte at the current candidate
                /// position is NOT in this set, then we can skip that entire candidate
                /// position (the length of the needle). This is essentially the shift
                /// trick found in Boyer-Moore, but only applied to bytes that don't appear
                /// in the needle.
                ///
                /// N.B. This trick was inspired by something similar in std's
                /// implementation of Two-Way.
                byteset: ApproximateByteSet,
                /// A critical position in needle. Specifically, this position corresponds
                /// to beginning of either the minimal or maximal suffix in needle. (N.B.
                /// See SuffixType below for why "minimal" isn't quite the correct word
                /// here.)
                ///
                /// This is the position at which every search begins. Namely, search
                /// starts by scanning text to the right of this position, and only if
                /// there's a match does the text to the left of this position get scanned.
                critical_pos: usize,
                /// The amount we shift by in the Two-Way search algorithm. This
                /// corresponds to the "small period" and "large period" cases.
                shift: Shift,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for TwoWay {}
            #[automatically_derived]
            impl ::core::clone::Clone for TwoWay {
                #[inline]
                fn clone(&self) -> TwoWay {
                    let _: ::core::clone::AssertParamIsClone<ApproximateByteSet>;
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    let _: ::core::clone::AssertParamIsClone<Shift>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for TwoWay {}
            #[automatically_derived]
            impl ::core::fmt::Debug for TwoWay {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field3_finish(
                        f,
                        "TwoWay",
                        "byteset",
                        &self.byteset,
                        "critical_pos",
                        &self.critical_pos,
                        "shift",
                        &&self.shift,
                    )
                }
            }
            impl Finder {
                /// Create a searcher that finds occurrences of the given `needle`.
                ///
                /// An empty `needle` results in a match at every position in a haystack,
                /// including at `haystack.len()`.
                #[inline]
                pub fn new(needle: &[u8]) -> Finder {
                    let byteset = ApproximateByteSet::new(needle);
                    let min_suffix = Suffix::forward(needle, SuffixKind::Minimal);
                    let max_suffix = Suffix::forward(needle, SuffixKind::Maximal);
                    let (period_lower_bound, critical_pos) = if min_suffix.pos
                        > max_suffix.pos
                    {
                        (min_suffix.period, min_suffix.pos)
                    } else {
                        (max_suffix.period, max_suffix.pos)
                    };
                    let shift = Shift::forward(needle, period_lower_bound, critical_pos);
                    Finder(TwoWay {
                        byteset,
                        critical_pos,
                        shift,
                    })
                }
                /// Returns the first occurrence of `needle` in the given `haystack`, or
                /// `None` if no such occurrence could be found.
                ///
                /// The `needle` given must be the same as the `needle` provided to
                /// [`Finder::new`].
                ///
                /// An empty `needle` results in a match at every position in a haystack,
                /// including at `haystack.len()`.
                #[inline]
                pub fn find(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                    self.find_with_prefilter(None, haystack, needle)
                }
                /// This is like [`Finder::find`], but it accepts a prefilter for
                /// accelerating searches.
                ///
                /// Currently this is not exposed in the public API because, at the time
                /// of writing, I didn't want to spend time thinking about how to expose
                /// the prefilter infrastructure (if at all). If you have a compelling use
                /// case for exposing this routine, please create an issue. Do *not* open
                /// a PR that just exposes `Pre` and friends. Exporting this routine will
                /// require API design.
                #[inline(always)]
                pub(crate) fn find_with_prefilter(
                    &self,
                    pre: Option<Pre<'_>>,
                    haystack: &[u8],
                    needle: &[u8],
                ) -> Option<usize> {
                    match self.0.shift {
                        Shift::Small { period } => {
                            self.find_small_imp(pre, haystack, needle, period)
                        }
                        Shift::Large { shift } => {
                            self.find_large_imp(pre, haystack, needle, shift)
                        }
                    }
                }
                #[inline(always)]
                fn find_small_imp(
                    &self,
                    mut pre: Option<Pre<'_>>,
                    haystack: &[u8],
                    needle: &[u8],
                    period: usize,
                ) -> Option<usize> {
                    let mut pos = 0;
                    let mut shift = 0;
                    let last_byte_pos = match needle.len().checked_sub(1) {
                        None => return Some(pos),
                        Some(last_byte) => last_byte,
                    };
                    while pos + needle.len() <= haystack.len() {
                        let mut i = cmp::max(self.0.critical_pos, shift);
                        if let Some(pre) = pre.as_mut() {
                            if pre.is_effective() {
                                pos += pre.find(&haystack[pos..])?;
                                shift = 0;
                                i = self.0.critical_pos;
                                if pos + needle.len() > haystack.len() {
                                    return None;
                                }
                            }
                        }
                        if !self.0.byteset.contains(haystack[pos + last_byte_pos]) {
                            pos += needle.len();
                            shift = 0;
                            continue;
                        }
                        while i < needle.len() && needle[i] == haystack[pos + i] {
                            i += 1;
                        }
                        if i < needle.len() {
                            pos += i - self.0.critical_pos + 1;
                            shift = 0;
                        } else {
                            let mut j = self.0.critical_pos;
                            while j > shift && needle[j] == haystack[pos + j] {
                                j -= 1;
                            }
                            if j <= shift && needle[shift] == haystack[pos + shift] {
                                return Some(pos);
                            }
                            pos += period;
                            shift = needle.len() - period;
                        }
                    }
                    None
                }
                #[inline(always)]
                fn find_large_imp(
                    &self,
                    mut pre: Option<Pre<'_>>,
                    haystack: &[u8],
                    needle: &[u8],
                    shift: usize,
                ) -> Option<usize> {
                    let mut pos = 0;
                    let last_byte_pos = match needle.len().checked_sub(1) {
                        None => return Some(pos),
                        Some(last_byte) => last_byte,
                    };
                    'outer: while pos + needle.len() <= haystack.len() {
                        if let Some(pre) = pre.as_mut() {
                            if pre.is_effective() {
                                pos += pre.find(&haystack[pos..])?;
                                if pos + needle.len() > haystack.len() {
                                    return None;
                                }
                            }
                        }
                        if !self.0.byteset.contains(haystack[pos + last_byte_pos]) {
                            pos += needle.len();
                            continue;
                        }
                        let mut i = self.0.critical_pos;
                        while i < needle.len() && needle[i] == haystack[pos + i] {
                            i += 1;
                        }
                        if i < needle.len() {
                            pos += i - self.0.critical_pos + 1;
                        } else {
                            for j in (0..self.0.critical_pos).rev() {
                                if needle[j] != haystack[pos + j] {
                                    pos += shift;
                                    continue 'outer;
                                }
                            }
                            return Some(pos);
                        }
                    }
                    None
                }
            }
            impl FinderRev {
                /// Create a searcher that finds occurrences of the given `needle`.
                ///
                /// An empty `needle` results in a match at every position in a haystack,
                /// including at `haystack.len()`.
                #[inline]
                pub fn new(needle: &[u8]) -> FinderRev {
                    let byteset = ApproximateByteSet::new(needle);
                    let min_suffix = Suffix::reverse(needle, SuffixKind::Minimal);
                    let max_suffix = Suffix::reverse(needle, SuffixKind::Maximal);
                    let (period_lower_bound, critical_pos) = if min_suffix.pos
                        < max_suffix.pos
                    {
                        (min_suffix.period, min_suffix.pos)
                    } else {
                        (max_suffix.period, max_suffix.pos)
                    };
                    let shift = Shift::reverse(needle, period_lower_bound, critical_pos);
                    FinderRev(TwoWay {
                        byteset,
                        critical_pos,
                        shift,
                    })
                }
                /// Returns the last occurrence of `needle` in the given `haystack`, or
                /// `None` if no such occurrence could be found.
                ///
                /// The `needle` given must be the same as the `needle` provided to
                /// [`FinderRev::new`].
                ///
                /// An empty `needle` results in a match at every position in a haystack,
                /// including at `haystack.len()`.
                #[inline]
                pub fn rfind(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                    match self.0.shift {
                        Shift::Small { period } => {
                            self.rfind_small_imp(haystack, needle, period)
                        }
                        Shift::Large { shift } => {
                            self.rfind_large_imp(haystack, needle, shift)
                        }
                    }
                }
                #[inline(always)]
                fn rfind_small_imp(
                    &self,
                    haystack: &[u8],
                    needle: &[u8],
                    period: usize,
                ) -> Option<usize> {
                    let nlen = needle.len();
                    let mut pos = haystack.len();
                    let mut shift = nlen;
                    let first_byte = match needle.get(0) {
                        None => return Some(pos),
                        Some(&first_byte) => first_byte,
                    };
                    while pos >= nlen {
                        if !self.0.byteset.contains(haystack[pos - nlen]) {
                            pos -= nlen;
                            shift = nlen;
                            continue;
                        }
                        let mut i = cmp::min(self.0.critical_pos, shift);
                        while i > 0 && needle[i - 1] == haystack[pos - nlen + i - 1] {
                            i -= 1;
                        }
                        if i > 0 || first_byte != haystack[pos - nlen] {
                            pos -= self.0.critical_pos - i + 1;
                            shift = nlen;
                        } else {
                            let mut j = self.0.critical_pos;
                            while j < shift && needle[j] == haystack[pos - nlen + j] {
                                j += 1;
                            }
                            if j >= shift {
                                return Some(pos - nlen);
                            }
                            pos -= period;
                            shift = period;
                        }
                    }
                    None
                }
                #[inline(always)]
                fn rfind_large_imp(
                    &self,
                    haystack: &[u8],
                    needle: &[u8],
                    shift: usize,
                ) -> Option<usize> {
                    let nlen = needle.len();
                    let mut pos = haystack.len();
                    let first_byte = match needle.get(0) {
                        None => return Some(pos),
                        Some(&first_byte) => first_byte,
                    };
                    while pos >= nlen {
                        if !self.0.byteset.contains(haystack[pos - nlen]) {
                            pos -= nlen;
                            continue;
                        }
                        let mut i = self.0.critical_pos;
                        while i > 0 && needle[i - 1] == haystack[pos - nlen + i - 1] {
                            i -= 1;
                        }
                        if i > 0 || first_byte != haystack[pos - nlen] {
                            pos -= self.0.critical_pos - i + 1;
                        } else {
                            let mut j = self.0.critical_pos;
                            while j < nlen && needle[j] == haystack[pos - nlen + j] {
                                j += 1;
                            }
                            if j == nlen {
                                return Some(pos - nlen);
                            }
                            pos -= shift;
                        }
                    }
                    None
                }
            }
            /// A representation of the amount we're allowed to shift by during Two-Way
            /// search.
            ///
            /// When computing a critical factorization of the needle, we find the position
            /// of the critical factorization by finding the needle's maximal (or minimal)
            /// suffix, along with the period of that suffix. It turns out that the period
            /// of that suffix is a lower bound on the period of the needle itself.
            ///
            /// This lower bound is equivalent to the actual period of the needle in
            /// some cases. To describe that case, we denote the needle as `x` where
            /// `x = uv` and `v` is the lexicographic maximal suffix of `v`. The lower
            /// bound given here is always the period of `v`, which is `<= period(x)`. The
            /// case where `period(v) == period(x)` occurs when `len(u) < (len(x) / 2)` and
            /// where `u` is a suffix of `v[0..period(v)]`.
            ///
            /// This case is important because the search algorithm for when the
            /// periods are equivalent is slightly different than the search algorithm
            /// for when the periods are not equivalent. In particular, when they aren't
            /// equivalent, we know that the period of the needle is no less than half its
            /// length. In this case, we shift by an amount less than or equal to the
            /// period of the needle (determined by the maximum length of the components
            /// of the critical factorization of `x`, i.e., `max(len(u), len(v))`)..
            ///
            /// The above two cases are represented by the variants below. Each entails
            /// a different instantiation of the Two-Way search algorithm.
            ///
            /// N.B. If we could find a way to compute the exact period in all cases,
            /// then we could collapse this case analysis and simplify the algorithm. The
            /// Two-Way paper suggests this is possible, but more reading is required to
            /// grok why the authors didn't pursue that path.
            enum Shift {
                Small { period: usize },
                Large { shift: usize },
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for Shift {}
            #[automatically_derived]
            impl ::core::clone::Clone for Shift {
                #[inline]
                fn clone(&self) -> Shift {
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for Shift {}
            #[automatically_derived]
            impl ::core::fmt::Debug for Shift {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    match self {
                        Shift::Small { period: __self_0 } => {
                            ::core::fmt::Formatter::debug_struct_field1_finish(
                                f,
                                "Small",
                                "period",
                                &__self_0,
                            )
                        }
                        Shift::Large { shift: __self_0 } => {
                            ::core::fmt::Formatter::debug_struct_field1_finish(
                                f,
                                "Large",
                                "shift",
                                &__self_0,
                            )
                        }
                    }
                }
            }
            impl Shift {
                /// Compute the shift for a given needle in the forward direction.
                ///
                /// This requires a lower bound on the period and a critical position.
                /// These can be computed by extracting both the minimal and maximal
                /// lexicographic suffixes, and choosing the right-most starting position.
                /// The lower bound on the period is then the period of the chosen suffix.
                fn forward(
                    needle: &[u8],
                    period_lower_bound: usize,
                    critical_pos: usize,
                ) -> Shift {
                    let large = cmp::max(critical_pos, needle.len() - critical_pos);
                    if critical_pos * 2 >= needle.len() {
                        return Shift::Large { shift: large };
                    }
                    let (u, v) = needle.split_at(critical_pos);
                    if !is_suffix(&v[..period_lower_bound], u) {
                        return Shift::Large { shift: large };
                    }
                    Shift::Small {
                        period: period_lower_bound,
                    }
                }
                /// Compute the shift for a given needle in the reverse direction.
                ///
                /// This requires a lower bound on the period and a critical position.
                /// These can be computed by extracting both the minimal and maximal
                /// lexicographic suffixes, and choosing the left-most starting position.
                /// The lower bound on the period is then the period of the chosen suffix.
                fn reverse(
                    needle: &[u8],
                    period_lower_bound: usize,
                    critical_pos: usize,
                ) -> Shift {
                    let large = cmp::max(critical_pos, needle.len() - critical_pos);
                    if (needle.len() - critical_pos) * 2 >= needle.len() {
                        return Shift::Large { shift: large };
                    }
                    let (v, u) = needle.split_at(critical_pos);
                    if !is_prefix(&v[v.len() - period_lower_bound..], u) {
                        return Shift::Large { shift: large };
                    }
                    Shift::Small {
                        period: period_lower_bound,
                    }
                }
            }
            /// A suffix extracted from a needle along with its period.
            struct Suffix {
                /// The starting position of this suffix.
                ///
                /// If this is a forward suffix, then `&bytes[pos..]` can be used. If this
                /// is a reverse suffix, then `&bytes[..pos]` can be used. That is, for
                /// forward suffixes, this is an inclusive starting position, where as for
                /// reverse suffixes, this is an exclusive ending position.
                pos: usize,
                /// The period of this suffix.
                ///
                /// Note that this is NOT necessarily the period of the string from which
                /// this suffix comes from. (It is always less than or equal to the period
                /// of the original string.)
                period: usize,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Suffix {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Suffix",
                        "pos",
                        &self.pos,
                        "period",
                        &&self.period,
                    )
                }
            }
            impl Suffix {
                fn forward(needle: &[u8], kind: SuffixKind) -> Suffix {
                    let mut suffix = Suffix { pos: 0, period: 1 };
                    let mut candidate_start = 1;
                    let mut offset = 0;
                    while candidate_start + offset < needle.len() {
                        let current = needle[suffix.pos + offset];
                        let candidate = needle[candidate_start + offset];
                        match kind.cmp(current, candidate) {
                            SuffixOrdering::Accept => {
                                suffix = Suffix {
                                    pos: candidate_start,
                                    period: 1,
                                };
                                candidate_start += 1;
                                offset = 0;
                            }
                            SuffixOrdering::Skip => {
                                candidate_start += offset + 1;
                                offset = 0;
                                suffix.period = candidate_start - suffix.pos;
                            }
                            SuffixOrdering::Push => {
                                if offset + 1 == suffix.period {
                                    candidate_start += suffix.period;
                                    offset = 0;
                                } else {
                                    offset += 1;
                                }
                            }
                        }
                    }
                    suffix
                }
                fn reverse(needle: &[u8], kind: SuffixKind) -> Suffix {
                    let mut suffix = Suffix {
                        pos: needle.len(),
                        period: 1,
                    };
                    if needle.len() == 1 {
                        return suffix;
                    }
                    let mut candidate_start = match needle.len().checked_sub(1) {
                        None => return suffix,
                        Some(candidate_start) => candidate_start,
                    };
                    let mut offset = 0;
                    while offset < candidate_start {
                        let current = needle[suffix.pos - offset - 1];
                        let candidate = needle[candidate_start - offset - 1];
                        match kind.cmp(current, candidate) {
                            SuffixOrdering::Accept => {
                                suffix = Suffix {
                                    pos: candidate_start,
                                    period: 1,
                                };
                                candidate_start -= 1;
                                offset = 0;
                            }
                            SuffixOrdering::Skip => {
                                candidate_start -= offset + 1;
                                offset = 0;
                                suffix.period = suffix.pos - candidate_start;
                            }
                            SuffixOrdering::Push => {
                                if offset + 1 == suffix.period {
                                    candidate_start -= suffix.period;
                                    offset = 0;
                                } else {
                                    offset += 1;
                                }
                            }
                        }
                    }
                    suffix
                }
            }
            /// The kind of suffix to extract.
            enum SuffixKind {
                /// Extract the smallest lexicographic suffix from a string.
                ///
                /// Technically, this doesn't actually pick the smallest lexicographic
                /// suffix. e.g., Given the choice between `a` and `aa`, this will choose
                /// the latter over the former, even though `a < aa`. The reasoning for
                /// this isn't clear from the paper, but it still smells like a minimal
                /// suffix.
                Minimal,
                /// Extract the largest lexicographic suffix from a string.
                ///
                /// Unlike `Minimal`, this really does pick the maximum suffix. e.g., Given
                /// the choice between `z` and `zz`, this will choose the latter over the
                /// former.
                Maximal,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for SuffixKind {}
            #[automatically_derived]
            impl ::core::clone::Clone for SuffixKind {
                #[inline]
                fn clone(&self) -> SuffixKind {
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for SuffixKind {}
            #[automatically_derived]
            impl ::core::fmt::Debug for SuffixKind {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(
                        f,
                        match self {
                            SuffixKind::Minimal => "Minimal",
                            SuffixKind::Maximal => "Maximal",
                        },
                    )
                }
            }
            /// The result of comparing corresponding bytes between two suffixes.
            enum SuffixOrdering {
                /// This occurs when the given candidate byte indicates that the candidate
                /// suffix is better than the current maximal (or minimal) suffix. That is,
                /// the current candidate suffix should supplant the current maximal (or
                /// minimal) suffix.
                Accept,
                /// This occurs when the given candidate byte excludes the candidate suffix
                /// from being better than the current maximal (or minimal) suffix. That
                /// is, the current candidate suffix should be dropped and the next one
                /// should be considered.
                Skip,
                /// This occurs when no decision to accept or skip the candidate suffix
                /// can be made, e.g., when corresponding bytes are equivalent. In this
                /// case, the next corresponding bytes should be compared.
                Push,
            }
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for SuffixOrdering {}
            #[automatically_derived]
            impl ::core::clone::Clone for SuffixOrdering {
                #[inline]
                fn clone(&self) -> SuffixOrdering {
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for SuffixOrdering {}
            #[automatically_derived]
            impl ::core::fmt::Debug for SuffixOrdering {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(
                        f,
                        match self {
                            SuffixOrdering::Accept => "Accept",
                            SuffixOrdering::Skip => "Skip",
                            SuffixOrdering::Push => "Push",
                        },
                    )
                }
            }
            impl SuffixKind {
                /// Returns true if and only if the given candidate byte indicates that
                /// it should replace the current suffix as the maximal (or minimal)
                /// suffix.
                fn cmp(self, current: u8, candidate: u8) -> SuffixOrdering {
                    use self::SuffixOrdering::*;
                    match self {
                        SuffixKind::Minimal if candidate < current => Accept,
                        SuffixKind::Minimal if candidate > current => Skip,
                        SuffixKind::Minimal => Push,
                        SuffixKind::Maximal if candidate > current => Accept,
                        SuffixKind::Maximal if candidate < current => Skip,
                        SuffixKind::Maximal => Push,
                    }
                }
            }
            /// A bitset used to track whether a particular byte exists in a needle or not.
            ///
            /// Namely, bit 'i' is set if and only if byte%64==i for any byte in the
            /// needle. If a particular byte in the haystack is NOT in this set, then one
            /// can conclude that it is also not in the needle, and thus, one can advance
            /// in the haystack by needle.len() bytes.
            struct ApproximateByteSet(u64);
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for ApproximateByteSet {}
            #[automatically_derived]
            impl ::core::clone::Clone for ApproximateByteSet {
                #[inline]
                fn clone(&self) -> ApproximateByteSet {
                    let _: ::core::clone::AssertParamIsClone<u64>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for ApproximateByteSet {}
            #[automatically_derived]
            impl ::core::fmt::Debug for ApproximateByteSet {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ApproximateByteSet",
                        &&self.0,
                    )
                }
            }
            impl ApproximateByteSet {
                /// Create a new set from the given needle.
                fn new(needle: &[u8]) -> ApproximateByteSet {
                    let mut bits = 0;
                    for &b in needle {
                        bits |= 1 << (b % 64);
                    }
                    ApproximateByteSet(bits)
                }
                /// Return true if and only if the given byte might be in this set. This
                /// may return a false positive, but will never return a false negative.
                #[inline(always)]
                fn contains(&self, byte: u8) -> bool {
                    self.0 & (1 << (byte % 64)) != 0
                }
            }
        }
        /// Returns true if and only if `needle` is a prefix of `haystack`.
        ///
        /// This uses a latency optimized variant of `memcmp` internally which *might*
        /// make this faster for very short strings.
        ///
        /// # Inlining
        ///
        /// This routine is marked `inline(always)`. If you want to call this function
        /// in a way that is not always inlined, you'll need to wrap a call to it in
        /// another function that is marked as `inline(never)` or just `inline`.
        #[inline(always)]
        pub fn is_prefix(haystack: &[u8], needle: &[u8]) -> bool {
            needle.len() <= haystack.len() && is_equal(&haystack[..needle.len()], needle)
        }
        /// Returns true if and only if `needle` is a suffix of `haystack`.
        ///
        /// This uses a latency optimized variant of `memcmp` internally which *might*
        /// make this faster for very short strings.
        ///
        /// # Inlining
        ///
        /// This routine is marked `inline(always)`. If you want to call this function
        /// in a way that is not always inlined, you'll need to wrap a call to it in
        /// another function that is marked as `inline(never)` or just `inline`.
        #[inline(always)]
        pub fn is_suffix(haystack: &[u8], needle: &[u8]) -> bool {
            needle.len() <= haystack.len()
                && is_equal(&haystack[haystack.len() - needle.len()..], needle)
        }
        /// Compare corresponding bytes in `x` and `y` for equality.
        ///
        /// That is, this returns true if and only if `x.len() == y.len()` and
        /// `x[i] == y[i]` for all `0 <= i < x.len()`.
        ///
        /// # Inlining
        ///
        /// This routine is marked `inline(always)`. If you want to call this function
        /// in a way that is not always inlined, you'll need to wrap a call to it in
        /// another function that is marked as `inline(never)` or just `inline`.
        ///
        /// # Motivation
        ///
        /// Why not use slice equality instead? Well, slice equality usually results in
        /// a call out to the current platform's `libc` which might not be inlineable
        /// or have other overhead. This routine isn't guaranteed to be a win, but it
        /// might be in some cases.
        #[inline(always)]
        pub fn is_equal(x: &[u8], y: &[u8]) -> bool {
            if x.len() != y.len() {
                return false;
            }
            unsafe { is_equal_raw(x.as_ptr(), y.as_ptr(), x.len()) }
        }
        /// Compare `n` bytes at the given pointers for equality.
        ///
        /// This returns true if and only if `*x.add(i) == *y.add(i)` for all
        /// `0 <= i < n`.
        ///
        /// # Inlining
        ///
        /// This routine is marked `inline(always)`. If you want to call this function
        /// in a way that is not always inlined, you'll need to wrap a call to it in
        /// another function that is marked as `inline(never)` or just `inline`.
        ///
        /// # Motivation
        ///
        /// Why not use slice equality instead? Well, slice equality usually results in
        /// a call out to the current platform's `libc` which might not be inlineable
        /// or have other overhead. This routine isn't guaranteed to be a win, but it
        /// might be in some cases.
        ///
        /// # Safety
        ///
        /// * Both `x` and `y` must be valid for reads of up to `n` bytes.
        /// * Both `x` and `y` must point to an initialized value.
        /// * Both `x` and `y` must each point to an allocated object and
        /// must either be in bounds or at most one byte past the end of the
        /// allocated object. `x` and `y` do not need to point to the same allocated
        /// object, but they may.
        /// * Both `x` and `y` must be _derived from_ a pointer to their respective
        /// allocated objects.
        /// * The distance between `x` and `x+n` must not overflow `isize`. Similarly
        /// for `y` and `y+n`.
        /// * The distance being in bounds must not rely on "wrapping around" the
        /// address space.
        #[inline(always)]
        pub unsafe fn is_equal_raw(
            mut x: *const u8,
            mut y: *const u8,
            mut n: usize,
        ) -> bool {
            while n >= 4 {
                let vx = x.cast::<u32>().read_unaligned();
                let vy = y.cast::<u32>().read_unaligned();
                if vx != vy {
                    return false;
                }
                x = x.add(4);
                y = y.add(4);
                n -= 4;
            }
            if n >= 2 {
                let vx = x.cast::<u16>().read_unaligned();
                let vy = y.cast::<u16>().read_unaligned();
                if vx != vy {
                    return false;
                }
                x = x.add(2);
                y = y.add(2);
                n -= 2;
            }
            if n > 0 {
                if x.read() != y.read() {
                    return false;
                }
            }
            true
        }
    }
    pub(crate) mod generic {
        /*!
This module defines "generic" routines that can be specialized to specific
architectures.

We don't expose this module primarily because it would require exposing all
of the internal infrastructure required to write these generic routines.
That infrastructure should be treated as an implementation detail so that
it is allowed to evolve. Instead, what we expose are architecture specific
instantiations of these generic implementations. The generic code just lets us
write the code once (usually).
*/
        pub(crate) mod memchr {
            /*!
Generic crate-internal routines for the `memchr` family of functions.
*/
            use crate::{ext::Pointer, vector::{MoveMask, Vector}};
            /// Finds all occurrences of a single byte in a haystack.
            pub(crate) struct One<V> {
                s1: u8,
                v1: V,
            }
            #[automatically_derived]
            impl<V: ::core::clone::Clone> ::core::clone::Clone for One<V> {
                #[inline]
                fn clone(&self) -> One<V> {
                    One {
                        s1: ::core::clone::Clone::clone(&self.s1),
                        v1: ::core::clone::Clone::clone(&self.v1),
                    }
                }
            }
            #[automatically_derived]
            impl<V: ::core::marker::Copy> ::core::marker::Copy for One<V> {}
            #[automatically_derived]
            impl<V: ::core::fmt::Debug> ::core::fmt::Debug for One<V> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "One",
                        "s1",
                        &self.s1,
                        "v1",
                        &&self.v1,
                    )
                }
            }
            impl<V: Vector> One<V> {
                /// The number of bytes we examine per each iteration of our search loop.
                const LOOP_SIZE: usize = 4 * V::BYTES;
                /// Create a new searcher that finds occurrences of the byte given.
                #[inline(always)]
                pub(crate) unsafe fn new(needle: u8) -> One<V> {
                    One {
                        s1: needle,
                        v1: V::splat(needle),
                    }
                }
                /// Returns the needle given to `One::new`.
                #[inline(always)]
                pub(crate) fn needle1(&self) -> u8 {
                    self.s1
                }
                /// Return a pointer to the first occurrence of the needle in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn find_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let topos = V::Mask::first_offset;
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(cur) = self.search_chunk(start, topos) {
                        return Some(cur);
                    }
                    let mut cur = start.add(V::BYTES - (start.as_usize() & V::ALIGN));
                    if true {
                        if !(cur > start && end.sub(V::BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: cur > start && end.sub(V::BYTES) >= start",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur <= end.sub(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(1 * V::BYTES));
                            let c = V::load_aligned(cur.add(2 * V::BYTES));
                            let d = V::load_aligned(cur.add(3 * V::BYTES));
                            let eqa = self.v1.cmpeq(a);
                            let eqb = self.v1.cmpeq(b);
                            let eqc = self.v1.cmpeq(c);
                            let eqd = self.v1.cmpeq(d);
                            let or1 = eqa.or(eqb);
                            let or2 = eqc.or(eqd);
                            let or3 = or1.or(or2);
                            if or3.movemask_will_have_non_zero() {
                                let mask = eqa.movemask();
                                if mask.has_non_zero() {
                                    return Some(cur.add(topos(mask)));
                                }
                                let mask = eqb.movemask();
                                if mask.has_non_zero() {
                                    return Some(cur.add(1 * V::BYTES).add(topos(mask)));
                                }
                                let mask = eqc.movemask();
                                if mask.has_non_zero() {
                                    return Some(cur.add(2 * V::BYTES).add(topos(mask)));
                                }
                                let mask = eqd.movemask();
                                if true {
                                    if !mask.has_non_zero() {
                                        ::core::panicking::panic(
                                            "assertion failed: mask.has_non_zero()",
                                        )
                                    }
                                }
                                return Some(cur.add(3 * V::BYTES).add(topos(mask)));
                            }
                            cur = cur.add(Self::LOOP_SIZE);
                        }
                    }
                    while cur <= end.sub(V::BYTES) {
                        if true {
                            if !(end.distance(cur) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) >= V::BYTES",
                                )
                            }
                        }
                        if let Some(cur) = self.search_chunk(cur, topos) {
                            return Some(cur);
                        }
                        cur = cur.add(V::BYTES);
                    }
                    if cur < end {
                        if true {
                            if !(end.distance(cur) < V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) < V::BYTES",
                                )
                            }
                        }
                        cur = cur.sub(V::BYTES - end.distance(cur));
                        if true {
                            match (&end.distance(cur), &V::BYTES) {
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
                        return self.search_chunk(cur, topos);
                    }
                    None
                }
                /// Return a pointer to the last occurrence of the needle in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn rfind_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let topos = V::Mask::last_offset;
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(cur) = self.search_chunk(end.sub(V::BYTES), topos) {
                        return Some(cur);
                    }
                    let mut cur = end.sub(end.as_usize() & V::ALIGN);
                    if true {
                        if !(start <= cur && cur <= end) {
                            ::core::panicking::panic(
                                "assertion failed: start <= cur && cur <= end",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur >= start.add(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            cur = cur.sub(Self::LOOP_SIZE);
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(1 * V::BYTES));
                            let c = V::load_aligned(cur.add(2 * V::BYTES));
                            let d = V::load_aligned(cur.add(3 * V::BYTES));
                            let eqa = self.v1.cmpeq(a);
                            let eqb = self.v1.cmpeq(b);
                            let eqc = self.v1.cmpeq(c);
                            let eqd = self.v1.cmpeq(d);
                            let or1 = eqa.or(eqb);
                            let or2 = eqc.or(eqd);
                            let or3 = or1.or(or2);
                            if or3.movemask_will_have_non_zero() {
                                let mask = eqd.movemask();
                                if mask.has_non_zero() {
                                    return Some(cur.add(3 * V::BYTES).add(topos(mask)));
                                }
                                let mask = eqc.movemask();
                                if mask.has_non_zero() {
                                    return Some(cur.add(2 * V::BYTES).add(topos(mask)));
                                }
                                let mask = eqb.movemask();
                                if mask.has_non_zero() {
                                    return Some(cur.add(1 * V::BYTES).add(topos(mask)));
                                }
                                let mask = eqa.movemask();
                                if true {
                                    if !mask.has_non_zero() {
                                        ::core::panicking::panic(
                                            "assertion failed: mask.has_non_zero()",
                                        )
                                    }
                                }
                                return Some(cur.add(topos(mask)));
                            }
                        }
                    }
                    while cur >= start.add(V::BYTES) {
                        if true {
                            if !(cur.distance(start) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: cur.distance(start) >= V::BYTES",
                                )
                            }
                        }
                        cur = cur.sub(V::BYTES);
                        if let Some(cur) = self.search_chunk(cur, topos) {
                            return Some(cur);
                        }
                    }
                    if cur > start {
                        if true {
                            if !(cur.distance(start) < V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: cur.distance(start) < V::BYTES",
                                )
                            }
                        }
                        return self.search_chunk(start, topos);
                    }
                    None
                }
                /// Return a count of all matching bytes in the given haystack.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn count_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> usize {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let confirm = |b| b == self.needle1();
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    let mut cur = start.add(V::BYTES - (start.as_usize() & V::ALIGN));
                    let mut count = count_byte_by_byte(start, cur, confirm);
                    if true {
                        if !(cur > start && end.sub(V::BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: cur > start && end.sub(V::BYTES) >= start",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur <= end.sub(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(1 * V::BYTES));
                            let c = V::load_aligned(cur.add(2 * V::BYTES));
                            let d = V::load_aligned(cur.add(3 * V::BYTES));
                            let eqa = self.v1.cmpeq(a);
                            let eqb = self.v1.cmpeq(b);
                            let eqc = self.v1.cmpeq(c);
                            let eqd = self.v1.cmpeq(d);
                            count += eqa.movemask().count_ones();
                            count += eqb.movemask().count_ones();
                            count += eqc.movemask().count_ones();
                            count += eqd.movemask().count_ones();
                            cur = cur.add(Self::LOOP_SIZE);
                        }
                    }
                    while cur <= end.sub(V::BYTES) {
                        if true {
                            if !(end.distance(cur) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) >= V::BYTES",
                                )
                            }
                        }
                        let chunk = V::load_unaligned(cur);
                        count += self.v1.cmpeq(chunk).movemask().count_ones();
                        cur = cur.add(V::BYTES);
                    }
                    count += count_byte_by_byte(cur, end, confirm);
                    count
                }
                /// Search `V::BYTES` starting at `cur` via an unaligned load.
                ///
                /// `mask_to_offset` should be a function that converts a `movemask` to
                /// an offset such that `cur.add(offset)` corresponds to a pointer to the
                /// match location if one is found. Generally it is expected to use either
                /// `mask_to_first_offset` or `mask_to_last_offset`, depending on whether
                /// one is implementing a forward or reverse search, respectively.
                ///
                /// # Safety
                ///
                /// `cur` must be a valid pointer and it must be valid to do an unaligned
                /// load of size `V::BYTES` at `cur`.
                #[inline(always)]
                unsafe fn search_chunk(
                    &self,
                    cur: *const u8,
                    mask_to_offset: impl Fn(V::Mask) -> usize,
                ) -> Option<*const u8> {
                    let chunk = V::load_unaligned(cur);
                    let mask = self.v1.cmpeq(chunk).movemask();
                    if mask.has_non_zero() {
                        Some(cur.add(mask_to_offset(mask)))
                    } else {
                        None
                    }
                }
            }
            /// Finds all occurrences of two bytes in a haystack.
            ///
            /// That is, this reports matches of one of two possible bytes. For example,
            /// searching for `a` or `b` in `afoobar` would report matches at offsets `0`,
            /// `4` and `5`.
            pub(crate) struct Two<V> {
                s1: u8,
                s2: u8,
                v1: V,
                v2: V,
            }
            #[automatically_derived]
            impl<V: ::core::clone::Clone> ::core::clone::Clone for Two<V> {
                #[inline]
                fn clone(&self) -> Two<V> {
                    Two {
                        s1: ::core::clone::Clone::clone(&self.s1),
                        s2: ::core::clone::Clone::clone(&self.s2),
                        v1: ::core::clone::Clone::clone(&self.v1),
                        v2: ::core::clone::Clone::clone(&self.v2),
                    }
                }
            }
            #[automatically_derived]
            impl<V: ::core::marker::Copy> ::core::marker::Copy for Two<V> {}
            #[automatically_derived]
            impl<V: ::core::fmt::Debug> ::core::fmt::Debug for Two<V> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field4_finish(
                        f,
                        "Two",
                        "s1",
                        &self.s1,
                        "s2",
                        &self.s2,
                        "v1",
                        &self.v1,
                        "v2",
                        &&self.v2,
                    )
                }
            }
            impl<V: Vector> Two<V> {
                /// The number of bytes we examine per each iteration of our search loop.
                const LOOP_SIZE: usize = 2 * V::BYTES;
                /// Create a new searcher that finds occurrences of the byte given.
                #[inline(always)]
                pub(crate) unsafe fn new(needle1: u8, needle2: u8) -> Two<V> {
                    Two {
                        s1: needle1,
                        s2: needle2,
                        v1: V::splat(needle1),
                        v2: V::splat(needle2),
                    }
                }
                /// Returns the first needle given to `Two::new`.
                #[inline(always)]
                pub(crate) fn needle1(&self) -> u8 {
                    self.s1
                }
                /// Returns the second needle given to `Two::new`.
                #[inline(always)]
                pub(crate) fn needle2(&self) -> u8 {
                    self.s2
                }
                /// Return a pointer to the first occurrence of one of the needles in the
                /// given haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn find_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let topos = V::Mask::first_offset;
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(cur) = self.search_chunk(start, topos) {
                        return Some(cur);
                    }
                    let mut cur = start.add(V::BYTES - (start.as_usize() & V::ALIGN));
                    if true {
                        if !(cur > start && end.sub(V::BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: cur > start && end.sub(V::BYTES) >= start",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur <= end.sub(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(V::BYTES));
                            let eqa1 = self.v1.cmpeq(a);
                            let eqb1 = self.v1.cmpeq(b);
                            let eqa2 = self.v2.cmpeq(a);
                            let eqb2 = self.v2.cmpeq(b);
                            let or1 = eqa1.or(eqb1);
                            let or2 = eqa2.or(eqb2);
                            let or3 = or1.or(or2);
                            if or3.movemask_will_have_non_zero() {
                                let mask = eqa1.movemask().or(eqa2.movemask());
                                if mask.has_non_zero() {
                                    return Some(cur.add(topos(mask)));
                                }
                                let mask = eqb1.movemask().or(eqb2.movemask());
                                if true {
                                    if !mask.has_non_zero() {
                                        ::core::panicking::panic(
                                            "assertion failed: mask.has_non_zero()",
                                        )
                                    }
                                }
                                return Some(cur.add(V::BYTES).add(topos(mask)));
                            }
                            cur = cur.add(Self::LOOP_SIZE);
                        }
                    }
                    while cur <= end.sub(V::BYTES) {
                        if true {
                            if !(end.distance(cur) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) >= V::BYTES",
                                )
                            }
                        }
                        if let Some(cur) = self.search_chunk(cur, topos) {
                            return Some(cur);
                        }
                        cur = cur.add(V::BYTES);
                    }
                    if cur < end {
                        if true {
                            if !(end.distance(cur) < V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) < V::BYTES",
                                )
                            }
                        }
                        cur = cur.sub(V::BYTES - end.distance(cur));
                        if true {
                            match (&end.distance(cur), &V::BYTES) {
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
                        return self.search_chunk(cur, topos);
                    }
                    None
                }
                /// Return a pointer to the last occurrence of the needle in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn rfind_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let topos = V::Mask::last_offset;
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(cur) = self.search_chunk(end.sub(V::BYTES), topos) {
                        return Some(cur);
                    }
                    let mut cur = end.sub(end.as_usize() & V::ALIGN);
                    if true {
                        if !(start <= cur && cur <= end) {
                            ::core::panicking::panic(
                                "assertion failed: start <= cur && cur <= end",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur >= start.add(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            cur = cur.sub(Self::LOOP_SIZE);
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(V::BYTES));
                            let eqa1 = self.v1.cmpeq(a);
                            let eqb1 = self.v1.cmpeq(b);
                            let eqa2 = self.v2.cmpeq(a);
                            let eqb2 = self.v2.cmpeq(b);
                            let or1 = eqa1.or(eqb1);
                            let or2 = eqa2.or(eqb2);
                            let or3 = or1.or(or2);
                            if or3.movemask_will_have_non_zero() {
                                let mask = eqb1.movemask().or(eqb2.movemask());
                                if mask.has_non_zero() {
                                    return Some(cur.add(V::BYTES).add(topos(mask)));
                                }
                                let mask = eqa1.movemask().or(eqa2.movemask());
                                if true {
                                    if !mask.has_non_zero() {
                                        ::core::panicking::panic(
                                            "assertion failed: mask.has_non_zero()",
                                        )
                                    }
                                }
                                return Some(cur.add(topos(mask)));
                            }
                        }
                    }
                    while cur >= start.add(V::BYTES) {
                        if true {
                            if !(cur.distance(start) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: cur.distance(start) >= V::BYTES",
                                )
                            }
                        }
                        cur = cur.sub(V::BYTES);
                        if let Some(cur) = self.search_chunk(cur, topos) {
                            return Some(cur);
                        }
                    }
                    if cur > start {
                        if true {
                            if !(cur.distance(start) < V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: cur.distance(start) < V::BYTES",
                                )
                            }
                        }
                        return self.search_chunk(start, topos);
                    }
                    None
                }
                /// Search `V::BYTES` starting at `cur` via an unaligned load.
                ///
                /// `mask_to_offset` should be a function that converts a `movemask` to
                /// an offset such that `cur.add(offset)` corresponds to a pointer to the
                /// match location if one is found. Generally it is expected to use either
                /// `mask_to_first_offset` or `mask_to_last_offset`, depending on whether
                /// one is implementing a forward or reverse search, respectively.
                ///
                /// # Safety
                ///
                /// `cur` must be a valid pointer and it must be valid to do an unaligned
                /// load of size `V::BYTES` at `cur`.
                #[inline(always)]
                unsafe fn search_chunk(
                    &self,
                    cur: *const u8,
                    mask_to_offset: impl Fn(V::Mask) -> usize,
                ) -> Option<*const u8> {
                    let chunk = V::load_unaligned(cur);
                    let eq1 = self.v1.cmpeq(chunk);
                    let eq2 = self.v2.cmpeq(chunk);
                    let mask = eq1.or(eq2).movemask();
                    if mask.has_non_zero() {
                        let mask1 = eq1.movemask();
                        let mask2 = eq2.movemask();
                        Some(cur.add(mask_to_offset(mask1.or(mask2))))
                    } else {
                        None
                    }
                }
            }
            /// Finds all occurrences of two bytes in a haystack.
            ///
            /// That is, this reports matches of one of two possible bytes. For example,
            /// searching for `a` or `b` in `afoobar` would report matches at offsets `0`,
            /// `4` and `5`.
            pub(crate) struct Three<V> {
                s1: u8,
                s2: u8,
                s3: u8,
                v1: V,
                v2: V,
                v3: V,
            }
            #[automatically_derived]
            impl<V: ::core::clone::Clone> ::core::clone::Clone for Three<V> {
                #[inline]
                fn clone(&self) -> Three<V> {
                    Three {
                        s1: ::core::clone::Clone::clone(&self.s1),
                        s2: ::core::clone::Clone::clone(&self.s2),
                        s3: ::core::clone::Clone::clone(&self.s3),
                        v1: ::core::clone::Clone::clone(&self.v1),
                        v2: ::core::clone::Clone::clone(&self.v2),
                        v3: ::core::clone::Clone::clone(&self.v3),
                    }
                }
            }
            #[automatically_derived]
            impl<V: ::core::marker::Copy> ::core::marker::Copy for Three<V> {}
            #[automatically_derived]
            impl<V: ::core::fmt::Debug> ::core::fmt::Debug for Three<V> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let names: &'static _ = &["s1", "s2", "s3", "v1", "v2", "v3"];
                    let values: &[&dyn ::core::fmt::Debug] = &[
                        &self.s1,
                        &self.s2,
                        &self.s3,
                        &self.v1,
                        &self.v2,
                        &&self.v3,
                    ];
                    ::core::fmt::Formatter::debug_struct_fields_finish(
                        f,
                        "Three",
                        names,
                        values,
                    )
                }
            }
            impl<V: Vector> Three<V> {
                /// The number of bytes we examine per each iteration of our search loop.
                const LOOP_SIZE: usize = 2 * V::BYTES;
                /// Create a new searcher that finds occurrences of the byte given.
                #[inline(always)]
                pub(crate) unsafe fn new(
                    needle1: u8,
                    needle2: u8,
                    needle3: u8,
                ) -> Three<V> {
                    Three {
                        s1: needle1,
                        s2: needle2,
                        s3: needle3,
                        v1: V::splat(needle1),
                        v2: V::splat(needle2),
                        v3: V::splat(needle3),
                    }
                }
                /// Returns the first needle given to `Three::new`.
                #[inline(always)]
                pub(crate) fn needle1(&self) -> u8 {
                    self.s1
                }
                /// Returns the second needle given to `Three::new`.
                #[inline(always)]
                pub(crate) fn needle2(&self) -> u8 {
                    self.s2
                }
                /// Returns the third needle given to `Three::new`.
                #[inline(always)]
                pub(crate) fn needle3(&self) -> u8 {
                    self.s3
                }
                /// Return a pointer to the first occurrence of one of the needles in the
                /// given haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn find_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let topos = V::Mask::first_offset;
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(cur) = self.search_chunk(start, topos) {
                        return Some(cur);
                    }
                    let mut cur = start.add(V::BYTES - (start.as_usize() & V::ALIGN));
                    if true {
                        if !(cur > start && end.sub(V::BYTES) >= start) {
                            ::core::panicking::panic(
                                "assertion failed: cur > start && end.sub(V::BYTES) >= start",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur <= end.sub(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(V::BYTES));
                            let eqa1 = self.v1.cmpeq(a);
                            let eqb1 = self.v1.cmpeq(b);
                            let eqa2 = self.v2.cmpeq(a);
                            let eqb2 = self.v2.cmpeq(b);
                            let eqa3 = self.v3.cmpeq(a);
                            let eqb3 = self.v3.cmpeq(b);
                            let or1 = eqa1.or(eqb1);
                            let or2 = eqa2.or(eqb2);
                            let or3 = eqa3.or(eqb3);
                            let or4 = or1.or(or2);
                            let or5 = or3.or(or4);
                            if or5.movemask_will_have_non_zero() {
                                let mask = eqa1
                                    .movemask()
                                    .or(eqa2.movemask())
                                    .or(eqa3.movemask());
                                if mask.has_non_zero() {
                                    return Some(cur.add(topos(mask)));
                                }
                                let mask = eqb1
                                    .movemask()
                                    .or(eqb2.movemask())
                                    .or(eqb3.movemask());
                                if true {
                                    if !mask.has_non_zero() {
                                        ::core::panicking::panic(
                                            "assertion failed: mask.has_non_zero()",
                                        )
                                    }
                                }
                                return Some(cur.add(V::BYTES).add(topos(mask)));
                            }
                            cur = cur.add(Self::LOOP_SIZE);
                        }
                    }
                    while cur <= end.sub(V::BYTES) {
                        if true {
                            if !(end.distance(cur) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) >= V::BYTES",
                                )
                            }
                        }
                        if let Some(cur) = self.search_chunk(cur, topos) {
                            return Some(cur);
                        }
                        cur = cur.add(V::BYTES);
                    }
                    if cur < end {
                        if true {
                            if !(end.distance(cur) < V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: end.distance(cur) < V::BYTES",
                                )
                            }
                        }
                        cur = cur.sub(V::BYTES - end.distance(cur));
                        if true {
                            match (&end.distance(cur), &V::BYTES) {
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
                        return self.search_chunk(cur, topos);
                    }
                    None
                }
                /// Return a pointer to the last occurrence of the needle in the given
                /// haystack. If no such occurrence exists, then `None` is returned.
                ///
                /// When a match is found, the pointer returned is guaranteed to be
                /// `>= start` and `< end`.
                ///
                /// # Safety
                ///
                /// * It must be the case that `start < end` and that the distance between
                /// them is at least equal to `V::BYTES`. That is, it must always be valid
                /// to do at least an unaligned load of `V` at `start`.
                /// * Both `start` and `end` must be valid for reads.
                /// * Both `start` and `end` must point to an initialized value.
                /// * Both `start` and `end` must point to the same allocated object and
                /// must either be in bounds or at most one byte past the end of the
                /// allocated object.
                /// * Both `start` and `end` must be _derived from_ a pointer to the same
                /// object.
                /// * The distance between `start` and `end` must not overflow `isize`.
                /// * The distance being in bounds must not rely on "wrapping around" the
                /// address space.
                #[inline(always)]
                pub(crate) unsafe fn rfind_raw(
                    &self,
                    start: *const u8,
                    end: *const u8,
                ) -> Option<*const u8> {
                    if true {
                        if !(V::BYTES <= 32) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!("vector cannot be bigger than 32 bytes"),
                                );
                            }
                        }
                    }
                    let topos = V::Mask::last_offset;
                    let len = end.distance(start);
                    if true {
                        if !(len >= V::BYTES) {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "haystack has length {0}, but must be at least {1}",
                                        len,
                                        V::BYTES,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(cur) = self.search_chunk(end.sub(V::BYTES), topos) {
                        return Some(cur);
                    }
                    let mut cur = end.sub(end.as_usize() & V::ALIGN);
                    if true {
                        if !(start <= cur && cur <= end) {
                            ::core::panicking::panic(
                                "assertion failed: start <= cur && cur <= end",
                            )
                        }
                    }
                    if len >= Self::LOOP_SIZE {
                        while cur >= start.add(Self::LOOP_SIZE) {
                            if true {
                                match (&0, &(cur.as_usize() % V::BYTES)) {
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
                            cur = cur.sub(Self::LOOP_SIZE);
                            let a = V::load_aligned(cur);
                            let b = V::load_aligned(cur.add(V::BYTES));
                            let eqa1 = self.v1.cmpeq(a);
                            let eqb1 = self.v1.cmpeq(b);
                            let eqa2 = self.v2.cmpeq(a);
                            let eqb2 = self.v2.cmpeq(b);
                            let eqa3 = self.v3.cmpeq(a);
                            let eqb3 = self.v3.cmpeq(b);
                            let or1 = eqa1.or(eqb1);
                            let or2 = eqa2.or(eqb2);
                            let or3 = eqa3.or(eqb3);
                            let or4 = or1.or(or2);
                            let or5 = or3.or(or4);
                            if or5.movemask_will_have_non_zero() {
                                let mask = eqb1
                                    .movemask()
                                    .or(eqb2.movemask())
                                    .or(eqb3.movemask());
                                if mask.has_non_zero() {
                                    return Some(cur.add(V::BYTES).add(topos(mask)));
                                }
                                let mask = eqa1
                                    .movemask()
                                    .or(eqa2.movemask())
                                    .or(eqa3.movemask());
                                if true {
                                    if !mask.has_non_zero() {
                                        ::core::panicking::panic(
                                            "assertion failed: mask.has_non_zero()",
                                        )
                                    }
                                }
                                return Some(cur.add(topos(mask)));
                            }
                        }
                    }
                    while cur >= start.add(V::BYTES) {
                        if true {
                            if !(cur.distance(start) >= V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: cur.distance(start) >= V::BYTES",
                                )
                            }
                        }
                        cur = cur.sub(V::BYTES);
                        if let Some(cur) = self.search_chunk(cur, topos) {
                            return Some(cur);
                        }
                    }
                    if cur > start {
                        if true {
                            if !(cur.distance(start) < V::BYTES) {
                                ::core::panicking::panic(
                                    "assertion failed: cur.distance(start) < V::BYTES",
                                )
                            }
                        }
                        return self.search_chunk(start, topos);
                    }
                    None
                }
                /// Search `V::BYTES` starting at `cur` via an unaligned load.
                ///
                /// `mask_to_offset` should be a function that converts a `movemask` to
                /// an offset such that `cur.add(offset)` corresponds to a pointer to the
                /// match location if one is found. Generally it is expected to use either
                /// `mask_to_first_offset` or `mask_to_last_offset`, depending on whether
                /// one is implementing a forward or reverse search, respectively.
                ///
                /// # Safety
                ///
                /// `cur` must be a valid pointer and it must be valid to do an unaligned
                /// load of size `V::BYTES` at `cur`.
                #[inline(always)]
                unsafe fn search_chunk(
                    &self,
                    cur: *const u8,
                    mask_to_offset: impl Fn(V::Mask) -> usize,
                ) -> Option<*const u8> {
                    let chunk = V::load_unaligned(cur);
                    let eq1 = self.v1.cmpeq(chunk);
                    let eq2 = self.v2.cmpeq(chunk);
                    let eq3 = self.v3.cmpeq(chunk);
                    let mask = eq1.or(eq2).or(eq3).movemask();
                    if mask.has_non_zero() {
                        let mask1 = eq1.movemask();
                        let mask2 = eq2.movemask();
                        let mask3 = eq3.movemask();
                        Some(cur.add(mask_to_offset(mask1.or(mask2).or(mask3))))
                    } else {
                        None
                    }
                }
            }
            /// An iterator over all occurrences of a set of bytes in a haystack.
            ///
            /// This iterator implements the routines necessary to provide a
            /// `DoubleEndedIterator` impl, which means it can also be used to find
            /// occurrences in reverse order.
            ///
            /// The lifetime parameters are as follows:
            ///
            /// * `'h` refers to the lifetime of the haystack being searched.
            ///
            /// This type is intended to be used to implement all iterators for the
            /// `memchr` family of functions. It handles a tiny bit of marginally tricky
            /// raw pointer math, but otherwise expects the caller to provide `find_raw`
            /// and `rfind_raw` routines for each call of `next` and `next_back`,
            /// respectively.
            pub(crate) struct Iter<'h> {
                /// The original starting point into the haystack. We use this to convert
                /// pointers to offsets.
                original_start: *const u8,
                /// The current starting point into the haystack. That is, where the next
                /// search will begin.
                start: *const u8,
                /// The current ending point into the haystack. That is, where the next
                /// reverse search will begin.
                end: *const u8,
                /// A marker for tracking the lifetime of the start/cur_start/cur_end
                /// pointers above, which all point into the haystack.
                haystack: core::marker::PhantomData<&'h [u8]>,
            }
            #[automatically_derived]
            impl<'h> ::core::clone::Clone for Iter<'h> {
                #[inline]
                fn clone(&self) -> Iter<'h> {
                    Iter {
                        original_start: ::core::clone::Clone::clone(
                            &self.original_start,
                        ),
                        start: ::core::clone::Clone::clone(&self.start),
                        end: ::core::clone::Clone::clone(&self.end),
                        haystack: ::core::clone::Clone::clone(&self.haystack),
                    }
                }
            }
            #[automatically_derived]
            impl<'h> ::core::fmt::Debug for Iter<'h> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field4_finish(
                        f,
                        "Iter",
                        "original_start",
                        &self.original_start,
                        "start",
                        &self.start,
                        "end",
                        &self.end,
                        "haystack",
                        &&self.haystack,
                    )
                }
            }
            unsafe impl<'h> Send for Iter<'h> {}
            unsafe impl<'h> Sync for Iter<'h> {}
            impl<'h> Iter<'h> {
                /// Create a new generic memchr iterator.
                #[inline(always)]
                pub(crate) fn new(haystack: &'h [u8]) -> Iter<'h> {
                    Iter {
                        original_start: haystack.as_ptr(),
                        start: haystack.as_ptr(),
                        end: haystack.as_ptr().wrapping_add(haystack.len()),
                        haystack: core::marker::PhantomData,
                    }
                }
                /// Returns the next occurrence in the forward direction.
                ///
                /// # Safety
                ///
                /// Callers must ensure that if a pointer is returned from the closure
                /// provided, then it must be greater than or equal to the start pointer
                /// and less than the end pointer.
                #[inline(always)]
                pub(crate) unsafe fn next(
                    &mut self,
                    mut find_raw: impl FnMut(*const u8, *const u8) -> Option<*const u8>,
                ) -> Option<usize> {
                    let found = find_raw(self.start, self.end)?;
                    let result = found.distance(self.original_start);
                    self.start = found.add(1);
                    Some(result)
                }
                /// Returns the number of remaining elements in this iterator.
                #[inline(always)]
                pub(crate) fn count(
                    self,
                    mut count_raw: impl FnMut(*const u8, *const u8) -> usize,
                ) -> usize {
                    count_raw(self.start, self.end)
                }
                /// Returns the next occurrence in reverse.
                ///
                /// # Safety
                ///
                /// Callers must ensure that if a pointer is returned from the closure
                /// provided, then it must be greater than or equal to the start pointer
                /// and less than the end pointer.
                #[inline(always)]
                pub(crate) unsafe fn next_back(
                    &mut self,
                    mut rfind_raw: impl FnMut(*const u8, *const u8) -> Option<*const u8>,
                ) -> Option<usize> {
                    let found = rfind_raw(self.start, self.end)?;
                    let result = found.distance(self.original_start);
                    self.end = found;
                    Some(result)
                }
                /// Provides an implementation of `Iterator::size_hint`.
                #[inline(always)]
                pub(crate) fn size_hint(&self) -> (usize, Option<usize>) {
                    (0, Some(self.end.as_usize().saturating_sub(self.start.as_usize())))
                }
            }
            /// Search a slice using a function that operates on raw pointers.
            ///
            /// Given a function to search a contiguous sequence of memory for the location
            /// of a non-empty set of bytes, this will execute that search on a slice of
            /// bytes. The pointer returned by the given function will be converted to an
            /// offset relative to the starting point of the given slice. That is, if a
            /// match is found, the offset returned by this routine is guaranteed to be a
            /// valid index into `haystack`.
            ///
            /// Callers may use this for a forward or reverse search.
            ///
            /// # Safety
            ///
            /// Callers must ensure that if a pointer is returned by `find_raw`, then the
            /// pointer must be greater than or equal to the starting pointer and less than
            /// the end pointer.
            #[inline(always)]
            pub(crate) unsafe fn search_slice_with_raw(
                haystack: &[u8],
                mut find_raw: impl FnMut(*const u8, *const u8) -> Option<*const u8>,
            ) -> Option<usize> {
                let start = haystack.as_ptr();
                let end = start.add(haystack.len());
                let found = find_raw(start, end)?;
                Some(found.distance(start))
            }
            /// Performs a forward byte-at-a-time loop until either `ptr >= end_ptr` or
            /// until `confirm(*ptr)` returns `true`. If the former occurs, then `None` is
            /// returned. If the latter occurs, then the pointer at which `confirm` returns
            /// `true` is returned.
            ///
            /// # Safety
            ///
            /// Callers must provide valid pointers and they must satisfy `start_ptr <=
            /// ptr` and `ptr <= end_ptr`.
            #[inline(always)]
            pub(crate) unsafe fn fwd_byte_by_byte<F: Fn(u8) -> bool>(
                start: *const u8,
                end: *const u8,
                confirm: F,
            ) -> Option<*const u8> {
                if true {
                    if !(start <= end) {
                        ::core::panicking::panic("assertion failed: start <= end")
                    }
                }
                let mut ptr = start;
                while ptr < end {
                    if confirm(*ptr) {
                        return Some(ptr);
                    }
                    ptr = ptr.offset(1);
                }
                None
            }
            /// Performs a reverse byte-at-a-time loop until either `ptr < start_ptr` or
            /// until `confirm(*ptr)` returns `true`. If the former occurs, then `None` is
            /// returned. If the latter occurs, then the pointer at which `confirm` returns
            /// `true` is returned.
            ///
            /// # Safety
            ///
            /// Callers must provide valid pointers and they must satisfy `start_ptr <=
            /// ptr` and `ptr <= end_ptr`.
            #[inline(always)]
            pub(crate) unsafe fn rev_byte_by_byte<F: Fn(u8) -> bool>(
                start: *const u8,
                end: *const u8,
                confirm: F,
            ) -> Option<*const u8> {
                if true {
                    if !(start <= end) {
                        ::core::panicking::panic("assertion failed: start <= end")
                    }
                }
                let mut ptr = end;
                while ptr > start {
                    ptr = ptr.offset(-1);
                    if confirm(*ptr) {
                        return Some(ptr);
                    }
                }
                None
            }
            /// Performs a forward byte-at-a-time loop until `ptr >= end_ptr` and returns
            /// the number of times `confirm(*ptr)` returns `true`.
            ///
            /// # Safety
            ///
            /// Callers must provide valid pointers and they must satisfy `start_ptr <=
            /// ptr` and `ptr <= end_ptr`.
            #[inline(always)]
            pub(crate) unsafe fn count_byte_by_byte<F: Fn(u8) -> bool>(
                start: *const u8,
                end: *const u8,
                confirm: F,
            ) -> usize {
                if true {
                    if !(start <= end) {
                        ::core::panicking::panic("assertion failed: start <= end")
                    }
                }
                let mut ptr = start;
                let mut count = 0;
                while ptr < end {
                    if confirm(*ptr) {
                        count += 1;
                    }
                    ptr = ptr.offset(1);
                }
                count
            }
        }
        pub(crate) mod packedpair {
            /*!
Generic crate-internal routines for the "packed pair" SIMD algorithm.

The "packed pair" algorithm is based on the [generic SIMD] algorithm. The main
difference is that it (by default) uses a background distribution of byte
frequencies to heuristically select the pair of bytes to search for.

[generic SIMD]: http://0x80.pl/articles/simd-strfind.html#first-and-last
*/
            use crate::{
                arch::all::{is_equal_raw, packedpair::Pair},
                ext::Pointer, vector::{MoveMask, Vector},
            };
            /// A generic architecture dependent "packed pair" finder.
            ///
            /// This finder picks two bytes that it believes have high predictive power
            /// for indicating an overall match of a needle. Depending on whether
            /// `Finder::find` or `Finder::find_prefilter` is used, it reports offsets
            /// where the needle matches or could match. In the prefilter case, candidates
            /// are reported whenever the [`Pair`] of bytes given matches.
            ///
            /// This is architecture dependent because it uses specific vector operations
            /// to look for occurrences of the pair of bytes.
            ///
            /// This type is not meant to be exported and is instead meant to be used as
            /// the implementation for architecture specific facades. Why? Because it's a
            /// bit of a quirky API that requires `inline(always)` annotations. And pretty
            /// much everything has safety obligations due (at least) to the caller needing
            /// to inline calls into routines marked with
            /// `#[target_feature(enable = "...")]`.
            pub(crate) struct Finder<V> {
                pair: Pair,
                v1: V,
                v2: V,
                min_haystack_len: usize,
            }
            #[automatically_derived]
            impl<V: ::core::clone::Clone> ::core::clone::Clone for Finder<V> {
                #[inline]
                fn clone(&self) -> Finder<V> {
                    Finder {
                        pair: ::core::clone::Clone::clone(&self.pair),
                        v1: ::core::clone::Clone::clone(&self.v1),
                        v2: ::core::clone::Clone::clone(&self.v2),
                        min_haystack_len: ::core::clone::Clone::clone(
                            &self.min_haystack_len,
                        ),
                    }
                }
            }
            #[automatically_derived]
            impl<V: ::core::marker::Copy> ::core::marker::Copy for Finder<V> {}
            #[automatically_derived]
            impl<V: ::core::fmt::Debug> ::core::fmt::Debug for Finder<V> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field4_finish(
                        f,
                        "Finder",
                        "pair",
                        &self.pair,
                        "v1",
                        &self.v1,
                        "v2",
                        &self.v2,
                        "min_haystack_len",
                        &&self.min_haystack_len,
                    )
                }
            }
            impl<V: Vector> Finder<V> {
                /// Create a new pair searcher. The searcher returned can either report
                /// exact matches of `needle` or act as a prefilter and report candidate
                /// positions of `needle`.
                ///
                /// # Safety
                ///
                /// Callers must ensure that whatever vector type this routine is called
                /// with is supported by the current environment.
                ///
                /// Callers must also ensure that `needle.len() >= 2`.
                #[inline(always)]
                pub(crate) unsafe fn new(needle: &[u8], pair: Pair) -> Finder<V> {
                    let max_index = pair.index1().max(pair.index2());
                    let min_haystack_len = core::cmp::max(
                        needle.len(),
                        usize::from(max_index) + V::BYTES,
                    );
                    let v1 = V::splat(needle[usize::from(pair.index1())]);
                    let v2 = V::splat(needle[usize::from(pair.index2())]);
                    Finder {
                        pair,
                        v1,
                        v2,
                        min_haystack_len,
                    }
                }
                /// Searches the given haystack for the given needle. The needle given
                /// should be the same as the needle that this finder was initialized
                /// with.
                ///
                /// # Panics
                ///
                /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                ///
                /// # Safety
                ///
                /// Since this is meant to be used with vector functions, callers need to
                /// specialize this inside of a function with a `target_feature` attribute.
                /// Therefore, callers must ensure that whatever target feature is being
                /// used supports the vector functions that this function is specialized
                /// for. (For the specific vector functions used, see the Vector trait
                /// implementations.)
                #[inline(always)]
                pub(crate) unsafe fn find(
                    &self,
                    haystack: &[u8],
                    needle: &[u8],
                ) -> Option<usize> {
                    if !(haystack.len() >= self.min_haystack_len) {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "haystack too small, should be at least {0} but got {1}",
                                    self.min_haystack_len,
                                    haystack.len(),
                                ),
                            );
                        }
                    }
                    let all = V::Mask::all_zeros_except_least_significant(0);
                    let start = haystack.as_ptr();
                    let end = start.add(haystack.len());
                    let max = end.sub(self.min_haystack_len);
                    let mut cur = start;
                    while cur <= max {
                        if let Some(chunki) = self.find_in_chunk(needle, cur, end, all) {
                            return Some(matched(start, cur, chunki));
                        }
                        cur = cur.add(V::BYTES);
                    }
                    if cur < end {
                        let remaining = end.distance(cur);
                        if true {
                            if !(remaining < self.min_haystack_len) {
                                {
                                    ::core::panicking::panic_fmt(
                                        format_args!(
                                            "remaining bytes should be smaller than the minimum haystack length of {0}, but there are {1} bytes remaining",
                                            self.min_haystack_len,
                                            remaining,
                                        ),
                                    );
                                }
                            }
                        }
                        if remaining < needle.len() {
                            return None;
                        }
                        if true {
                            if !(max < cur) {
                                {
                                    ::core::panicking::panic_fmt(
                                        format_args!(
                                            "after main loop, cur should have exceeded max",
                                        ),
                                    );
                                }
                            }
                        }
                        let overlap = cur.distance(max);
                        if true {
                            if !(overlap > 0) {
                                {
                                    ::core::panicking::panic_fmt(
                                        format_args!(
                                            "overlap ({0}) must always be non-zero",
                                            overlap,
                                        ),
                                    );
                                }
                            }
                        }
                        if true {
                            if !(overlap < V::BYTES) {
                                {
                                    ::core::panicking::panic_fmt(
                                        format_args!(
                                            "overlap ({0}) cannot possibly be >= than a vector ({1})",
                                            overlap,
                                            V::BYTES,
                                        ),
                                    );
                                }
                            }
                        }
                        let mask = V::Mask::all_zeros_except_least_significant(overlap);
                        cur = max;
                        let m = self.find_in_chunk(needle, cur, end, mask);
                        if let Some(chunki) = m {
                            return Some(matched(start, cur, chunki));
                        }
                    }
                    None
                }
                /// Searches the given haystack for offsets that represent candidate
                /// matches of the `needle` given to this finder's constructor. The offsets
                /// returned, if they are a match, correspond to the starting offset of
                /// `needle` in the given `haystack`.
                ///
                /// # Panics
                ///
                /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                ///
                /// # Safety
                ///
                /// Since this is meant to be used with vector functions, callers need to
                /// specialize this inside of a function with a `target_feature` attribute.
                /// Therefore, callers must ensure that whatever target feature is being
                /// used supports the vector functions that this function is specialized
                /// for. (For the specific vector functions used, see the Vector trait
                /// implementations.)
                #[inline(always)]
                pub(crate) unsafe fn find_prefilter(
                    &self,
                    haystack: &[u8],
                ) -> Option<usize> {
                    if !(haystack.len() >= self.min_haystack_len) {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "haystack too small, should be at least {0} but got {1}",
                                    self.min_haystack_len,
                                    haystack.len(),
                                ),
                            );
                        }
                    }
                    let start = haystack.as_ptr();
                    let end = start.add(haystack.len());
                    let max = end.sub(self.min_haystack_len);
                    let mut cur = start;
                    while cur <= max {
                        if let Some(chunki) = self.find_prefilter_in_chunk(cur) {
                            return Some(matched(start, cur, chunki));
                        }
                        cur = cur.add(V::BYTES);
                    }
                    if cur < end {
                        cur = max;
                        if let Some(chunki) = self.find_prefilter_in_chunk(cur) {
                            return Some(matched(start, cur, chunki));
                        }
                    }
                    None
                }
                /// Search for an occurrence of our byte pair from the needle in the chunk
                /// pointed to by cur, with the end of the haystack pointed to by end.
                /// When an occurrence is found, memcmp is run to check if a match occurs
                /// at the corresponding position.
                ///
                /// `mask` should have bits set corresponding the positions in the chunk
                /// in which matches are considered. This is only used for the last vector
                /// load where the beginning of the vector might have overlapped with the
                /// last load in the main loop. The mask lets us avoid visiting positions
                /// that have already been discarded as matches.
                ///
                /// # Safety
                ///
                /// It must be safe to do an unaligned read of size(V) bytes starting at
                /// both (cur + self.index1) and (cur + self.index2). It must also be safe
                /// to do unaligned loads on cur up to (end - needle.len()).
                #[inline(always)]
                unsafe fn find_in_chunk(
                    &self,
                    needle: &[u8],
                    cur: *const u8,
                    end: *const u8,
                    mask: V::Mask,
                ) -> Option<usize> {
                    let index1 = usize::from(self.pair.index1());
                    let index2 = usize::from(self.pair.index2());
                    let chunk1 = V::load_unaligned(cur.add(index1));
                    let chunk2 = V::load_unaligned(cur.add(index2));
                    let eq1 = chunk1.cmpeq(self.v1);
                    let eq2 = chunk2.cmpeq(self.v2);
                    let mut offsets = eq1.and(eq2).movemask().and(mask);
                    while offsets.has_non_zero() {
                        let offset = offsets.first_offset();
                        let cur = cur.add(offset);
                        if end.sub(needle.len()) < cur {
                            return None;
                        }
                        if is_equal_raw(needle.as_ptr(), cur, needle.len()) {
                            return Some(offset);
                        }
                        offsets = offsets.clear_least_significant_bit();
                    }
                    None
                }
                /// Search for an occurrence of our byte pair from the needle in the chunk
                /// pointed to by cur, with the end of the haystack pointed to by end.
                /// When an occurrence is found, memcmp is run to check if a match occurs
                /// at the corresponding position.
                ///
                /// # Safety
                ///
                /// It must be safe to do an unaligned read of size(V) bytes starting at
                /// both (cur + self.index1) and (cur + self.index2). It must also be safe
                /// to do unaligned reads on cur up to (end - needle.len()).
                #[inline(always)]
                unsafe fn find_prefilter_in_chunk(
                    &self,
                    cur: *const u8,
                ) -> Option<usize> {
                    let index1 = usize::from(self.pair.index1());
                    let index2 = usize::from(self.pair.index2());
                    let chunk1 = V::load_unaligned(cur.add(index1));
                    let chunk2 = V::load_unaligned(cur.add(index2));
                    let eq1 = chunk1.cmpeq(self.v1);
                    let eq2 = chunk2.cmpeq(self.v2);
                    let offsets = eq1.and(eq2).movemask();
                    if !offsets.has_non_zero() {
                        return None;
                    }
                    Some(offsets.first_offset())
                }
                /// Returns the pair of offsets (into the needle) used to check as a
                /// predicate before confirming whether a needle exists at a particular
                /// position.
                #[inline]
                pub(crate) fn pair(&self) -> &Pair {
                    &self.pair
                }
                /// Returns the minimum haystack length that this `Finder` can search.
                ///
                /// Providing a haystack to this `Finder` shorter than this length is
                /// guaranteed to result in a panic.
                #[inline(always)]
                pub(crate) fn min_haystack_len(&self) -> usize {
                    self.min_haystack_len
                }
            }
            /// Accepts a chunk-relative offset and returns a haystack relative offset.
            ///
            /// This used to be marked `#[cold]` and `#[inline(never)]`, but I couldn't
            /// observe a consistent measureable difference between that and just inlining
            /// it. So we go with inlining it.
            ///
            /// # Safety
            ///
            /// Same at `ptr::offset_from` in addition to `cur >= start`.
            #[inline(always)]
            unsafe fn matched(start: *const u8, cur: *const u8, chunki: usize) -> usize {
                cur.distance(start) + chunki
            }
        }
    }
    pub mod x86_64 {
        /*!
Vector algorithms for the `x86_64` target.
*/
        pub mod avx2 {
            /*!
Algorithms for the `x86_64` target using 256-bit vectors via AVX2.
*/
            pub mod memchr {
                /*!
This module defines 256-bit vector implementations of `memchr` and friends.

The main types in this module are [`One`], [`Two`] and [`Three`]. They are for
searching for one, two or three distinct bytes, respectively, in a haystack.
Each type also has corresponding double ended iterators. These searchers are
typically much faster than scalar routines accomplishing the same task.

The `One` searcher also provides a [`One::count`] routine for efficiently
counting the number of times a single byte occurs in a haystack. This is
useful, for example, for counting the number of lines in a haystack. This
routine exists because it is usually faster, especially with a high match
count, then using [`One::find`] repeatedly. ([`OneIter`] specializes its
`Iterator::count` implementation to use this routine.)

Only one, two and three bytes are supported because three bytes is about
the point where one sees diminishing returns. Beyond this point and it's
probably (but not necessarily) better to just use a simple `[bool; 256]` array
or similar. However, it depends mightily on the specific work-load and the
expected match frequency.
*/
                use core::arch::x86_64::{__m128i, __m256i};
                use crate::{
                    arch::generic::memchr as generic, ext::Pointer, vector::Vector,
                };
                /// Finds all occurrences of a single byte in a haystack.
                pub struct One {
                    /// Used for haystacks less than 32 bytes.
                    sse2: generic::One<__m128i>,
                    /// Used for haystacks bigger than 32 bytes.
                    avx2: generic::One<__m256i>,
                }
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for One {}
                #[automatically_derived]
                impl ::core::clone::Clone for One {
                    #[inline]
                    fn clone(&self) -> One {
                        let _: ::core::clone::AssertParamIsClone<generic::One<__m128i>>;
                        let _: ::core::clone::AssertParamIsClone<generic::One<__m256i>>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for One {}
                #[automatically_derived]
                impl ::core::fmt::Debug for One {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "One",
                            "sse2",
                            &self.sse2,
                            "avx2",
                            &&self.avx2,
                        )
                    }
                }
                impl One {
                    /// Create a new searcher that finds occurrences of the needle byte given.
                    ///
                    /// This particular searcher is specialized to use AVX2 vector instructions
                    /// that typically make it quite fast. (SSE2 is used for haystacks that
                    /// are too short to accommodate an AVX2 vector.)
                    ///
                    /// If either SSE2 or AVX2 is unavailable in the current environment, then
                    /// `None` is returned.
                    #[inline]
                    pub fn new(needle: u8) -> Option<One> {
                        if One::is_available() {
                            unsafe { Some(One::new_unchecked(needle)) }
                        } else {
                            None
                        }
                    }
                    /// Create a new finder specific to AVX2 vectors and routines without
                    /// checking that either SSE2 or AVX2 is available.
                    ///
                    /// # Safety
                    ///
                    /// Callers must guarantee that it is safe to execute both `sse2` and
                    /// `avx2` instructions in the current environment.
                    ///
                    /// Note that it is a common misconception that if one compiles for an
                    /// `x86_64` target, then they therefore automatically have access to SSE2
                    /// instructions. While this is almost always the case, it isn't true in
                    /// 100% of cases.
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    #[inline]
                    pub unsafe fn new_unchecked(needle: u8) -> One {
                        One {
                            sse2: generic::One::new(needle),
                            avx2: generic::One::new(needle),
                        }
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`One::new`] will return
                    /// a `Some` value. Similarly, when it is false, it is guaranteed that
                    /// `One::new` will return a `None` value.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        {
                            {
                                {
                                    false || ::std_detect::detect::__is_feature_detected::avx2()
                                }
                            }
                        }
                    }
                    /// Return the first occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.find_raw(s, e) },
                            )
                        }
                    }
                    /// Return the last occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.rfind_raw(s, e) },
                            )
                        }
                    }
                    /// Counts all occurrences of this byte in the given haystack.
                    #[inline]
                    pub fn count(&self, haystack: &[u8]) -> usize {
                        unsafe {
                            let start = haystack.as_ptr();
                            let end = start.add(haystack.len());
                            self.count_raw(start, end)
                        }
                    }
                    /// Like `find`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn find_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::fwd_byte_by_byte(
                                    start,
                                    end,
                                    |b| { b == self.sse2.needle1() },
                                )
                            } else {
                                self.find_raw_sse2(start, end)
                            };
                        }
                        self.find_raw_avx2(start, end)
                    }
                    /// Like `rfind`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn rfind_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::rev_byte_by_byte(
                                    start,
                                    end,
                                    |b| { b == self.sse2.needle1() },
                                )
                            } else {
                                self.rfind_raw_sse2(start, end)
                            };
                        }
                        self.rfind_raw_avx2(start, end)
                    }
                    /// Counts all occurrences of this byte in the given haystack represented
                    /// by raw pointers.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `0` will always be returned.
                    #[inline]
                    pub unsafe fn count_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        if start >= end {
                            return 0;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::count_byte_by_byte(
                                    start,
                                    end,
                                    |b| { b == self.sse2.needle1() },
                                )
                            } else {
                                self.count_raw_sse2(start, end)
                            };
                        }
                        self.count_raw_avx2(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.sse2.find_raw(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn rfind_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.sse2.rfind_raw(start, end)
                    }
                    /// Execute a count using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::count_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn count_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        self.sse2.count_raw(start, end)
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn find_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.avx2.find_raw(start, end)
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn rfind_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.avx2.rfind_raw(start, end)
                    }
                    /// Execute a count using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::count_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn count_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        self.avx2.count_raw(start, end)
                    }
                    /// Returns an iterator over all occurrences of the needle byte in the
                    /// given haystack.
                    ///
                    /// The iterator returned implements `DoubleEndedIterator`. This means it
                    /// can also be used to find occurrences in reverse order.
                    #[inline]
                    pub fn iter<'a, 'h>(
                        &'a self,
                        haystack: &'h [u8],
                    ) -> OneIter<'a, 'h> {
                        OneIter {
                            searcher: self,
                            it: generic::Iter::new(haystack),
                        }
                    }
                }
                /// An iterator over all occurrences of a single byte in a haystack.
                ///
                /// This iterator implements `DoubleEndedIterator`, which means it can also be
                /// used to find occurrences in reverse order.
                ///
                /// This iterator is created by the [`One::iter`] method.
                ///
                /// The lifetime parameters are as follows:
                ///
                /// * `'a` refers to the lifetime of the underlying [`One`] searcher.
                /// * `'h` refers to the lifetime of the haystack being searched.
                pub struct OneIter<'a, 'h> {
                    searcher: &'a One,
                    it: generic::Iter<'h>,
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::clone::Clone for OneIter<'a, 'h> {
                    #[inline]
                    fn clone(&self) -> OneIter<'a, 'h> {
                        OneIter {
                            searcher: ::core::clone::Clone::clone(&self.searcher),
                            it: ::core::clone::Clone::clone(&self.it),
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::fmt::Debug for OneIter<'a, 'h> {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "OneIter",
                            "searcher",
                            &self.searcher,
                            "it",
                            &&self.it,
                        )
                    }
                }
                impl<'a, 'h> Iterator for OneIter<'a, 'h> {
                    type Item = usize;
                    #[inline]
                    fn next(&mut self) -> Option<usize> {
                        unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                    }
                    #[inline]
                    fn count(self) -> usize {
                        self.it
                            .count(|s, e| { unsafe { self.searcher.count_raw(s, e) } })
                    }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.it.size_hint()
                    }
                }
                impl<'a, 'h> DoubleEndedIterator for OneIter<'a, 'h> {
                    #[inline]
                    fn next_back(&mut self) -> Option<usize> {
                        unsafe {
                            self.it.next_back(|s, e| self.searcher.rfind_raw(s, e))
                        }
                    }
                }
                impl<'a, 'h> core::iter::FusedIterator for OneIter<'a, 'h> {}
                /// Finds all occurrences of two bytes in a haystack.
                ///
                /// That is, this reports matches of one of two possible bytes. For example,
                /// searching for `a` or `b` in `afoobar` would report matches at offsets `0`,
                /// `4` and `5`.
                pub struct Two {
                    /// Used for haystacks less than 32 bytes.
                    sse2: generic::Two<__m128i>,
                    /// Used for haystacks bigger than 32 bytes.
                    avx2: generic::Two<__m256i>,
                }
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for Two {}
                #[automatically_derived]
                impl ::core::clone::Clone for Two {
                    #[inline]
                    fn clone(&self) -> Two {
                        let _: ::core::clone::AssertParamIsClone<generic::Two<__m128i>>;
                        let _: ::core::clone::AssertParamIsClone<generic::Two<__m256i>>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Two {}
                #[automatically_derived]
                impl ::core::fmt::Debug for Two {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "Two",
                            "sse2",
                            &self.sse2,
                            "avx2",
                            &&self.avx2,
                        )
                    }
                }
                impl Two {
                    /// Create a new searcher that finds occurrences of the needle bytes given.
                    ///
                    /// This particular searcher is specialized to use AVX2 vector instructions
                    /// that typically make it quite fast. (SSE2 is used for haystacks that
                    /// are too short to accommodate an AVX2 vector.)
                    ///
                    /// If either SSE2 or AVX2 is unavailable in the current environment, then
                    /// `None` is returned.
                    #[inline]
                    pub fn new(needle1: u8, needle2: u8) -> Option<Two> {
                        if Two::is_available() {
                            unsafe { Some(Two::new_unchecked(needle1, needle2)) }
                        } else {
                            None
                        }
                    }
                    /// Create a new finder specific to AVX2 vectors and routines without
                    /// checking that either SSE2 or AVX2 is available.
                    ///
                    /// # Safety
                    ///
                    /// Callers must guarantee that it is safe to execute both `sse2` and
                    /// `avx2` instructions in the current environment.
                    ///
                    /// Note that it is a common misconception that if one compiles for an
                    /// `x86_64` target, then they therefore automatically have access to SSE2
                    /// instructions. While this is almost always the case, it isn't true in
                    /// 100% of cases.
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    #[inline]
                    pub unsafe fn new_unchecked(needle1: u8, needle2: u8) -> Two {
                        Two {
                            sse2: generic::Two::new(needle1, needle2),
                            avx2: generic::Two::new(needle1, needle2),
                        }
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`Two::new`] will return
                    /// a `Some` value. Similarly, when it is false, it is guaranteed that
                    /// `Two::new` will return a `None` value.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        {
                            {
                                {
                                    false || ::std_detect::detect::__is_feature_detected::avx2()
                                }
                            }
                        }
                    }
                    /// Return the first occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.find_raw(s, e) },
                            )
                        }
                    }
                    /// Return the last occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.rfind_raw(s, e) },
                            )
                        }
                    }
                    /// Like `find`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn find_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::fwd_byte_by_byte(
                                    start,
                                    end,
                                    |b| { b == self.sse2.needle1() || b == self.sse2.needle2() },
                                )
                            } else {
                                self.find_raw_sse2(start, end)
                            };
                        }
                        self.find_raw_avx2(start, end)
                    }
                    /// Like `rfind`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn rfind_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::rev_byte_by_byte(
                                    start,
                                    end,
                                    |b| { b == self.sse2.needle1() || b == self.sse2.needle2() },
                                )
                            } else {
                                self.rfind_raw_sse2(start, end)
                            };
                        }
                        self.rfind_raw_avx2(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Two::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Two`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.sse2.find_raw(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Two::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Two`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn rfind_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.sse2.rfind_raw(start, end)
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Two::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Two`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn find_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.avx2.find_raw(start, end)
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Two::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Two`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn rfind_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.avx2.rfind_raw(start, end)
                    }
                    /// Returns an iterator over all occurrences of the needle bytes in the
                    /// given haystack.
                    ///
                    /// The iterator returned implements `DoubleEndedIterator`. This means it
                    /// can also be used to find occurrences in reverse order.
                    #[inline]
                    pub fn iter<'a, 'h>(
                        &'a self,
                        haystack: &'h [u8],
                    ) -> TwoIter<'a, 'h> {
                        TwoIter {
                            searcher: self,
                            it: generic::Iter::new(haystack),
                        }
                    }
                }
                /// An iterator over all occurrences of two possible bytes in a haystack.
                ///
                /// This iterator implements `DoubleEndedIterator`, which means it can also be
                /// used to find occurrences in reverse order.
                ///
                /// This iterator is created by the [`Two::iter`] method.
                ///
                /// The lifetime parameters are as follows:
                ///
                /// * `'a` refers to the lifetime of the underlying [`Two`] searcher.
                /// * `'h` refers to the lifetime of the haystack being searched.
                pub struct TwoIter<'a, 'h> {
                    searcher: &'a Two,
                    it: generic::Iter<'h>,
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::clone::Clone for TwoIter<'a, 'h> {
                    #[inline]
                    fn clone(&self) -> TwoIter<'a, 'h> {
                        TwoIter {
                            searcher: ::core::clone::Clone::clone(&self.searcher),
                            it: ::core::clone::Clone::clone(&self.it),
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::fmt::Debug for TwoIter<'a, 'h> {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "TwoIter",
                            "searcher",
                            &self.searcher,
                            "it",
                            &&self.it,
                        )
                    }
                }
                impl<'a, 'h> Iterator for TwoIter<'a, 'h> {
                    type Item = usize;
                    #[inline]
                    fn next(&mut self) -> Option<usize> {
                        unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                    }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.it.size_hint()
                    }
                }
                impl<'a, 'h> DoubleEndedIterator for TwoIter<'a, 'h> {
                    #[inline]
                    fn next_back(&mut self) -> Option<usize> {
                        unsafe {
                            self.it.next_back(|s, e| self.searcher.rfind_raw(s, e))
                        }
                    }
                }
                impl<'a, 'h> core::iter::FusedIterator for TwoIter<'a, 'h> {}
                /// Finds all occurrences of three bytes in a haystack.
                ///
                /// That is, this reports matches of one of three possible bytes. For example,
                /// searching for `a`, `b` or `o` in `afoobar` would report matches at offsets
                /// `0`, `2`, `3`, `4` and `5`.
                pub struct Three {
                    /// Used for haystacks less than 32 bytes.
                    sse2: generic::Three<__m128i>,
                    /// Used for haystacks bigger than 32 bytes.
                    avx2: generic::Three<__m256i>,
                }
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for Three {}
                #[automatically_derived]
                impl ::core::clone::Clone for Three {
                    #[inline]
                    fn clone(&self) -> Three {
                        let _: ::core::clone::AssertParamIsClone<
                            generic::Three<__m128i>,
                        >;
                        let _: ::core::clone::AssertParamIsClone<
                            generic::Three<__m256i>,
                        >;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Three {}
                #[automatically_derived]
                impl ::core::fmt::Debug for Three {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "Three",
                            "sse2",
                            &self.sse2,
                            "avx2",
                            &&self.avx2,
                        )
                    }
                }
                impl Three {
                    /// Create a new searcher that finds occurrences of the needle bytes given.
                    ///
                    /// This particular searcher is specialized to use AVX2 vector instructions
                    /// that typically make it quite fast. (SSE2 is used for haystacks that
                    /// are too short to accommodate an AVX2 vector.)
                    ///
                    /// If either SSE2 or AVX2 is unavailable in the current environment, then
                    /// `None` is returned.
                    #[inline]
                    pub fn new(needle1: u8, needle2: u8, needle3: u8) -> Option<Three> {
                        if Three::is_available() {
                            unsafe {
                                Some(Three::new_unchecked(needle1, needle2, needle3))
                            }
                        } else {
                            None
                        }
                    }
                    /// Create a new finder specific to AVX2 vectors and routines without
                    /// checking that either SSE2 or AVX2 is available.
                    ///
                    /// # Safety
                    ///
                    /// Callers must guarantee that it is safe to execute both `sse2` and
                    /// `avx2` instructions in the current environment.
                    ///
                    /// Note that it is a common misconception that if one compiles for an
                    /// `x86_64` target, then they therefore automatically have access to SSE2
                    /// instructions. While this is almost always the case, it isn't true in
                    /// 100% of cases.
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    #[inline]
                    pub unsafe fn new_unchecked(
                        needle1: u8,
                        needle2: u8,
                        needle3: u8,
                    ) -> Three {
                        Three {
                            sse2: generic::Three::new(needle1, needle2, needle3),
                            avx2: generic::Three::new(needle1, needle2, needle3),
                        }
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`Three::new`] will return
                    /// a `Some` value. Similarly, when it is false, it is guaranteed that
                    /// `Three::new` will return a `None` value.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        {
                            {
                                {
                                    false || ::std_detect::detect::__is_feature_detected::avx2()
                                }
                            }
                        }
                    }
                    /// Return the first occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.find_raw(s, e) },
                            )
                        }
                    }
                    /// Return the last occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.rfind_raw(s, e) },
                            )
                        }
                    }
                    /// Like `find`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn find_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::fwd_byte_by_byte(
                                    start,
                                    end,
                                    |b| {
                                        b == self.sse2.needle1() || b == self.sse2.needle2()
                                            || b == self.sse2.needle3()
                                    },
                                )
                            } else {
                                self.find_raw_sse2(start, end)
                            };
                        }
                        self.find_raw_avx2(start, end)
                    }
                    /// Like `rfind`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn rfind_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        let len = end.distance(start);
                        if len < __m256i::BYTES {
                            return if len < __m128i::BYTES {
                                generic::rev_byte_by_byte(
                                    start,
                                    end,
                                    |b| {
                                        b == self.sse2.needle1() || b == self.sse2.needle2()
                                            || b == self.sse2.needle3()
                                    },
                                )
                            } else {
                                self.rfind_raw_sse2(start, end)
                            };
                        }
                        self.rfind_raw_avx2(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Three::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Three`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.sse2.find_raw(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Three::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Three`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn rfind_raw_sse2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.sse2.rfind_raw(start, end)
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Three::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Three`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn find_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.avx2.find_raw(start, end)
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Three::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an AVX2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Three`, which can only be constructed
                    /// when it is safe to call `sse2`/`avx2` routines.)
                    #[target_feature(enable = "avx2")]
                    #[inline]
                    unsafe fn rfind_raw_avx2(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.avx2.rfind_raw(start, end)
                    }
                    /// Returns an iterator over all occurrences of the needle bytes in the
                    /// given haystack.
                    ///
                    /// The iterator returned implements `DoubleEndedIterator`. This means it
                    /// can also be used to find occurrences in reverse order.
                    #[inline]
                    pub fn iter<'a, 'h>(
                        &'a self,
                        haystack: &'h [u8],
                    ) -> ThreeIter<'a, 'h> {
                        ThreeIter {
                            searcher: self,
                            it: generic::Iter::new(haystack),
                        }
                    }
                }
                /// An iterator over all occurrences of three possible bytes in a haystack.
                ///
                /// This iterator implements `DoubleEndedIterator`, which means it can also be
                /// used to find occurrences in reverse order.
                ///
                /// This iterator is created by the [`Three::iter`] method.
                ///
                /// The lifetime parameters are as follows:
                ///
                /// * `'a` refers to the lifetime of the underlying [`Three`] searcher.
                /// * `'h` refers to the lifetime of the haystack being searched.
                pub struct ThreeIter<'a, 'h> {
                    searcher: &'a Three,
                    it: generic::Iter<'h>,
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::clone::Clone for ThreeIter<'a, 'h> {
                    #[inline]
                    fn clone(&self) -> ThreeIter<'a, 'h> {
                        ThreeIter {
                            searcher: ::core::clone::Clone::clone(&self.searcher),
                            it: ::core::clone::Clone::clone(&self.it),
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::fmt::Debug for ThreeIter<'a, 'h> {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "ThreeIter",
                            "searcher",
                            &self.searcher,
                            "it",
                            &&self.it,
                        )
                    }
                }
                impl<'a, 'h> Iterator for ThreeIter<'a, 'h> {
                    type Item = usize;
                    #[inline]
                    fn next(&mut self) -> Option<usize> {
                        unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                    }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.it.size_hint()
                    }
                }
                impl<'a, 'h> DoubleEndedIterator for ThreeIter<'a, 'h> {
                    #[inline]
                    fn next_back(&mut self) -> Option<usize> {
                        unsafe {
                            self.it.next_back(|s, e| self.searcher.rfind_raw(s, e))
                        }
                    }
                }
                impl<'a, 'h> core::iter::FusedIterator for ThreeIter<'a, 'h> {}
            }
            pub mod packedpair {
                /*!
A 256-bit vector implementation of the "packed pair" SIMD algorithm.

The "packed pair" algorithm is based on the [generic SIMD] algorithm. The main
difference is that it (by default) uses a background distribution of byte
frequencies to heuristically select the pair of bytes to search for.

[generic SIMD]: http://0x80.pl/articles/simd-strfind.html#first-and-last
*/
                use core::arch::x86_64::{__m128i, __m256i};
                use crate::arch::{all::packedpair::Pair, generic::packedpair};
                /// A "packed pair" finder that uses 256-bit vector operations.
                ///
                /// This finder picks two bytes that it believes have high predictive power
                /// for indicating an overall match of a needle. Depending on whether
                /// `Finder::find` or `Finder::find_prefilter` is used, it reports offsets
                /// where the needle matches or could match. In the prefilter case, candidates
                /// are reported whenever the [`Pair`] of bytes given matches.
                pub struct Finder {
                    sse2: packedpair::Finder<__m128i>,
                    avx2: packedpair::Finder<__m256i>,
                }
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for Finder {}
                #[automatically_derived]
                impl ::core::clone::Clone for Finder {
                    #[inline]
                    fn clone(&self) -> Finder {
                        let _: ::core::clone::AssertParamIsClone<
                            packedpair::Finder<__m128i>,
                        >;
                        let _: ::core::clone::AssertParamIsClone<
                            packedpair::Finder<__m256i>,
                        >;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Finder {}
                #[automatically_derived]
                impl ::core::fmt::Debug for Finder {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "Finder",
                            "sse2",
                            &self.sse2,
                            "avx2",
                            &&self.avx2,
                        )
                    }
                }
                impl Finder {
                    /// Create a new pair searcher. The searcher returned can either report
                    /// exact matches of `needle` or act as a prefilter and report candidate
                    /// positions of `needle`.
                    ///
                    /// If AVX2 is unavailable in the current environment or if a [`Pair`]
                    /// could not be constructed from the needle given, then `None` is
                    /// returned.
                    #[inline]
                    pub fn new(needle: &[u8]) -> Option<Finder> {
                        Finder::with_pair(needle, Pair::new(needle)?)
                    }
                    /// Create a new "packed pair" finder using the pair of bytes given.
                    ///
                    /// This constructor permits callers to control precisely which pair of
                    /// bytes is used as a predicate.
                    ///
                    /// If AVX2 is unavailable in the current environment, then `None` is
                    /// returned.
                    #[inline]
                    pub fn with_pair(needle: &[u8], pair: Pair) -> Option<Finder> {
                        if Finder::is_available() {
                            unsafe { Some(Finder::with_pair_impl(needle, pair)) }
                        } else {
                            None
                        }
                    }
                    /// Create a new `Finder` specific to SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as the safety for `packedpair::Finder::new`, and callers must also
                    /// ensure that both SSE2 and AVX2 are available.
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    #[inline]
                    unsafe fn with_pair_impl(needle: &[u8], pair: Pair) -> Finder {
                        let sse2 = packedpair::Finder::<__m128i>::new(needle, pair);
                        let avx2 = packedpair::Finder::<__m256i>::new(needle, pair);
                        Finder { sse2, avx2 }
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`Finder::with_pair`] will
                    /// return a `Some` value. Similarly, when it is false, it is guaranteed
                    /// that `Finder::with_pair` will return a `None` value. Notice that this
                    /// does not guarantee that [`Finder::new`] will return a `Finder`. Namely,
                    /// even when `Finder::is_available` is true, it is not guaranteed that a
                    /// valid [`Pair`] can be found from the needle given.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        {
                            {
                                {
                                    false || ::std_detect::detect::__is_feature_detected::avx2()
                                }
                            }
                        }
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    #[inline]
                    pub fn find(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                        unsafe { self.find_impl(haystack, needle) }
                    }
                    /// Run this finder on the given haystack as a prefilter.
                    ///
                    /// If a candidate match is found, then an offset where the needle *could*
                    /// begin in the haystack is returned.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    #[inline]
                    pub fn find_prefilter(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe { self.find_prefilter_impl(haystack) }
                    }
                    /// Execute a search using AVX2 vectors and routines.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    ///
                    /// # Safety
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Finder`, which can only be constructed
                    /// when it is safe to call `sse2` and `avx2` routines.)
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    #[inline]
                    unsafe fn find_impl(
                        &self,
                        haystack: &[u8],
                        needle: &[u8],
                    ) -> Option<usize> {
                        if haystack.len() < self.avx2.min_haystack_len() {
                            self.sse2.find(haystack, needle)
                        } else {
                            self.avx2.find(haystack, needle)
                        }
                    }
                    /// Execute a prefilter search using AVX2 vectors and routines.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    ///
                    /// # Safety
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Finder`, which can only be constructed
                    /// when it is safe to call `sse2` and `avx2` routines.)
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    #[inline]
                    unsafe fn find_prefilter_impl(
                        &self,
                        haystack: &[u8],
                    ) -> Option<usize> {
                        if haystack.len() < self.avx2.min_haystack_len() {
                            self.sse2.find_prefilter(haystack)
                        } else {
                            self.avx2.find_prefilter(haystack)
                        }
                    }
                    /// Returns the pair of offsets (into the needle) used to check as a
                    /// predicate before confirming whether a needle exists at a particular
                    /// position.
                    #[inline]
                    pub fn pair(&self) -> &Pair {
                        self.avx2.pair()
                    }
                    /// Returns the minimum haystack length that this `Finder` can search.
                    ///
                    /// Using a haystack with length smaller than this in a search will result
                    /// in a panic. The reason for this restriction is that this finder is
                    /// meant to be a low-level component that is part of a larger substring
                    /// strategy. In that sense, it avoids trying to handle all cases and
                    /// instead only handles the cases that it can handle very well.
                    #[inline]
                    pub fn min_haystack_len(&self) -> usize {
                        self.sse2.min_haystack_len()
                    }
                }
            }
        }
        pub mod sse2 {
            /*!
Algorithms for the `x86_64` target using 128-bit vectors via SSE2.
*/
            pub mod memchr {
                /*!
This module defines 128-bit vector implementations of `memchr` and friends.

The main types in this module are [`One`], [`Two`] and [`Three`]. They are for
searching for one, two or three distinct bytes, respectively, in a haystack.
Each type also has corresponding double ended iterators. These searchers are
typically much faster than scalar routines accomplishing the same task.

The `One` searcher also provides a [`One::count`] routine for efficiently
counting the number of times a single byte occurs in a haystack. This is
useful, for example, for counting the number of lines in a haystack. This
routine exists because it is usually faster, especially with a high match
count, than using [`One::find`] repeatedly. ([`OneIter`] specializes its
`Iterator::count` implementation to use this routine.)

Only one, two and three bytes are supported because three bytes is about
the point where one sees diminishing returns. Beyond this point and it's
probably (but not necessarily) better to just use a simple `[bool; 256]` array
or similar. However, it depends mightily on the specific work-load and the
expected match frequency.
*/
                use core::arch::x86_64::__m128i;
                use crate::{
                    arch::generic::memchr as generic, ext::Pointer, vector::Vector,
                };
                /// Finds all occurrences of a single byte in a haystack.
                pub struct One(generic::One<__m128i>);
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for One {}
                #[automatically_derived]
                impl ::core::clone::Clone for One {
                    #[inline]
                    fn clone(&self) -> One {
                        let _: ::core::clone::AssertParamIsClone<generic::One<__m128i>>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for One {}
                #[automatically_derived]
                impl ::core::fmt::Debug for One {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "One",
                            &&self.0,
                        )
                    }
                }
                impl One {
                    /// Create a new searcher that finds occurrences of the needle byte given.
                    ///
                    /// This particular searcher is specialized to use SSE2 vector instructions
                    /// that typically make it quite fast.
                    ///
                    /// If SSE2 is unavailable in the current environment, then `None` is
                    /// returned.
                    #[inline]
                    pub fn new(needle: u8) -> Option<One> {
                        if One::is_available() {
                            unsafe { Some(One::new_unchecked(needle)) }
                        } else {
                            None
                        }
                    }
                    /// Create a new finder specific to SSE2 vectors and routines without
                    /// checking that SSE2 is available.
                    ///
                    /// # Safety
                    ///
                    /// Callers must guarantee that it is safe to execute `sse2` instructions
                    /// in the current environment.
                    ///
                    /// Note that it is a common misconception that if one compiles for an
                    /// `x86_64` target, then they therefore automatically have access to SSE2
                    /// instructions. While this is almost always the case, it isn't true in
                    /// 100% of cases.
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    pub unsafe fn new_unchecked(needle: u8) -> One {
                        One(generic::One::new(needle))
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`One::new`] will return
                    /// a `Some` value. Similarly, when it is false, it is guaranteed that
                    /// `One::new` will return a `None` value.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        { true }
                    }
                    /// Return the first occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.find_raw(s, e) },
                            )
                        }
                    }
                    /// Return the last occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.rfind_raw(s, e) },
                            )
                        }
                    }
                    /// Counts all occurrences of this byte in the given haystack.
                    #[inline]
                    pub fn count(&self, haystack: &[u8]) -> usize {
                        unsafe {
                            let start = haystack.as_ptr();
                            let end = start.add(haystack.len());
                            self.count_raw(start, end)
                        }
                    }
                    /// Like `find`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn find_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::fwd_byte_by_byte(
                                start,
                                end,
                                |b| { b == self.0.needle1() },
                            );
                        }
                        self.find_raw_impl(start, end)
                    }
                    /// Like `rfind`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn rfind_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::rev_byte_by_byte(
                                start,
                                end,
                                |b| { b == self.0.needle1() },
                            );
                        }
                        self.rfind_raw_impl(start, end)
                    }
                    /// Counts all occurrences of this byte in the given haystack represented
                    /// by raw pointers.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `0` will always be returned.
                    #[inline]
                    pub unsafe fn count_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        if start >= end {
                            return 0;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::count_byte_by_byte(
                                start,
                                end,
                                |b| { b == self.0.needle1() },
                            );
                        }
                        self.count_raw_impl(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.0.find_raw(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn rfind_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.0.rfind_raw(start, end)
                    }
                    /// Execute a count using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`One::count_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `One`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn count_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        self.0.count_raw(start, end)
                    }
                    /// Returns an iterator over all occurrences of the needle byte in the
                    /// given haystack.
                    ///
                    /// The iterator returned implements `DoubleEndedIterator`. This means it
                    /// can also be used to find occurrences in reverse order.
                    #[inline]
                    pub fn iter<'a, 'h>(
                        &'a self,
                        haystack: &'h [u8],
                    ) -> OneIter<'a, 'h> {
                        OneIter {
                            searcher: self,
                            it: generic::Iter::new(haystack),
                        }
                    }
                }
                /// An iterator over all occurrences of a single byte in a haystack.
                ///
                /// This iterator implements `DoubleEndedIterator`, which means it can also be
                /// used to find occurrences in reverse order.
                ///
                /// This iterator is created by the [`One::iter`] method.
                ///
                /// The lifetime parameters are as follows:
                ///
                /// * `'a` refers to the lifetime of the underlying [`One`] searcher.
                /// * `'h` refers to the lifetime of the haystack being searched.
                pub struct OneIter<'a, 'h> {
                    searcher: &'a One,
                    it: generic::Iter<'h>,
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::clone::Clone for OneIter<'a, 'h> {
                    #[inline]
                    fn clone(&self) -> OneIter<'a, 'h> {
                        OneIter {
                            searcher: ::core::clone::Clone::clone(&self.searcher),
                            it: ::core::clone::Clone::clone(&self.it),
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::fmt::Debug for OneIter<'a, 'h> {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "OneIter",
                            "searcher",
                            &self.searcher,
                            "it",
                            &&self.it,
                        )
                    }
                }
                impl<'a, 'h> Iterator for OneIter<'a, 'h> {
                    type Item = usize;
                    #[inline]
                    fn next(&mut self) -> Option<usize> {
                        unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                    }
                    #[inline]
                    fn count(self) -> usize {
                        self.it
                            .count(|s, e| { unsafe { self.searcher.count_raw(s, e) } })
                    }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.it.size_hint()
                    }
                }
                impl<'a, 'h> DoubleEndedIterator for OneIter<'a, 'h> {
                    #[inline]
                    fn next_back(&mut self) -> Option<usize> {
                        unsafe {
                            self.it.next_back(|s, e| self.searcher.rfind_raw(s, e))
                        }
                    }
                }
                impl<'a, 'h> core::iter::FusedIterator for OneIter<'a, 'h> {}
                /// Finds all occurrences of two bytes in a haystack.
                ///
                /// That is, this reports matches of one of two possible bytes. For example,
                /// searching for `a` or `b` in `afoobar` would report matches at offsets `0`,
                /// `4` and `5`.
                pub struct Two(generic::Two<__m128i>);
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for Two {}
                #[automatically_derived]
                impl ::core::clone::Clone for Two {
                    #[inline]
                    fn clone(&self) -> Two {
                        let _: ::core::clone::AssertParamIsClone<generic::Two<__m128i>>;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Two {}
                #[automatically_derived]
                impl ::core::fmt::Debug for Two {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Two",
                            &&self.0,
                        )
                    }
                }
                impl Two {
                    /// Create a new searcher that finds occurrences of the needle bytes given.
                    ///
                    /// This particular searcher is specialized to use SSE2 vector instructions
                    /// that typically make it quite fast.
                    ///
                    /// If SSE2 is unavailable in the current environment, then `None` is
                    /// returned.
                    #[inline]
                    pub fn new(needle1: u8, needle2: u8) -> Option<Two> {
                        if Two::is_available() {
                            unsafe { Some(Two::new_unchecked(needle1, needle2)) }
                        } else {
                            None
                        }
                    }
                    /// Create a new finder specific to SSE2 vectors and routines without
                    /// checking that SSE2 is available.
                    ///
                    /// # Safety
                    ///
                    /// Callers must guarantee that it is safe to execute `sse2` instructions
                    /// in the current environment.
                    ///
                    /// Note that it is a common misconception that if one compiles for an
                    /// `x86_64` target, then they therefore automatically have access to SSE2
                    /// instructions. While this is almost always the case, it isn't true in
                    /// 100% of cases.
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    pub unsafe fn new_unchecked(needle1: u8, needle2: u8) -> Two {
                        Two(generic::Two::new(needle1, needle2))
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`Two::new`] will return
                    /// a `Some` value. Similarly, when it is false, it is guaranteed that
                    /// `Two::new` will return a `None` value.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        { true }
                    }
                    /// Return the first occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.find_raw(s, e) },
                            )
                        }
                    }
                    /// Return the last occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.rfind_raw(s, e) },
                            )
                        }
                    }
                    /// Like `find`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn find_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::fwd_byte_by_byte(
                                start,
                                end,
                                |b| { b == self.0.needle1() || b == self.0.needle2() },
                            );
                        }
                        self.find_raw_impl(start, end)
                    }
                    /// Like `rfind`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn rfind_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::rev_byte_by_byte(
                                start,
                                end,
                                |b| { b == self.0.needle1() || b == self.0.needle2() },
                            );
                        }
                        self.rfind_raw_impl(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Two::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Two`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.0.find_raw(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Two::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Two`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn rfind_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.0.rfind_raw(start, end)
                    }
                    /// Returns an iterator over all occurrences of the needle bytes in the
                    /// given haystack.
                    ///
                    /// The iterator returned implements `DoubleEndedIterator`. This means it
                    /// can also be used to find occurrences in reverse order.
                    #[inline]
                    pub fn iter<'a, 'h>(
                        &'a self,
                        haystack: &'h [u8],
                    ) -> TwoIter<'a, 'h> {
                        TwoIter {
                            searcher: self,
                            it: generic::Iter::new(haystack),
                        }
                    }
                }
                /// An iterator over all occurrences of two possible bytes in a haystack.
                ///
                /// This iterator implements `DoubleEndedIterator`, which means it can also be
                /// used to find occurrences in reverse order.
                ///
                /// This iterator is created by the [`Two::iter`] method.
                ///
                /// The lifetime parameters are as follows:
                ///
                /// * `'a` refers to the lifetime of the underlying [`Two`] searcher.
                /// * `'h` refers to the lifetime of the haystack being searched.
                pub struct TwoIter<'a, 'h> {
                    searcher: &'a Two,
                    it: generic::Iter<'h>,
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::clone::Clone for TwoIter<'a, 'h> {
                    #[inline]
                    fn clone(&self) -> TwoIter<'a, 'h> {
                        TwoIter {
                            searcher: ::core::clone::Clone::clone(&self.searcher),
                            it: ::core::clone::Clone::clone(&self.it),
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::fmt::Debug for TwoIter<'a, 'h> {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "TwoIter",
                            "searcher",
                            &self.searcher,
                            "it",
                            &&self.it,
                        )
                    }
                }
                impl<'a, 'h> Iterator for TwoIter<'a, 'h> {
                    type Item = usize;
                    #[inline]
                    fn next(&mut self) -> Option<usize> {
                        unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                    }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.it.size_hint()
                    }
                }
                impl<'a, 'h> DoubleEndedIterator for TwoIter<'a, 'h> {
                    #[inline]
                    fn next_back(&mut self) -> Option<usize> {
                        unsafe {
                            self.it.next_back(|s, e| self.searcher.rfind_raw(s, e))
                        }
                    }
                }
                impl<'a, 'h> core::iter::FusedIterator for TwoIter<'a, 'h> {}
                /// Finds all occurrences of three bytes in a haystack.
                ///
                /// That is, this reports matches of one of three possible bytes. For example,
                /// searching for `a`, `b` or `o` in `afoobar` would report matches at offsets
                /// `0`, `2`, `3`, `4` and `5`.
                pub struct Three(generic::Three<__m128i>);
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for Three {}
                #[automatically_derived]
                impl ::core::clone::Clone for Three {
                    #[inline]
                    fn clone(&self) -> Three {
                        let _: ::core::clone::AssertParamIsClone<
                            generic::Three<__m128i>,
                        >;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Three {}
                #[automatically_derived]
                impl ::core::fmt::Debug for Three {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Three",
                            &&self.0,
                        )
                    }
                }
                impl Three {
                    /// Create a new searcher that finds occurrences of the needle bytes given.
                    ///
                    /// This particular searcher is specialized to use SSE2 vector instructions
                    /// that typically make it quite fast.
                    ///
                    /// If SSE2 is unavailable in the current environment, then `None` is
                    /// returned.
                    #[inline]
                    pub fn new(needle1: u8, needle2: u8, needle3: u8) -> Option<Three> {
                        if Three::is_available() {
                            unsafe {
                                Some(Three::new_unchecked(needle1, needle2, needle3))
                            }
                        } else {
                            None
                        }
                    }
                    /// Create a new finder specific to SSE2 vectors and routines without
                    /// checking that SSE2 is available.
                    ///
                    /// # Safety
                    ///
                    /// Callers must guarantee that it is safe to execute `sse2` instructions
                    /// in the current environment.
                    ///
                    /// Note that it is a common misconception that if one compiles for an
                    /// `x86_64` target, then they therefore automatically have access to SSE2
                    /// instructions. While this is almost always the case, it isn't true in
                    /// 100% of cases.
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    pub unsafe fn new_unchecked(
                        needle1: u8,
                        needle2: u8,
                        needle3: u8,
                    ) -> Three {
                        Three(generic::Three::new(needle1, needle2, needle3))
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`Three::new`] will return
                    /// a `Some` value. Similarly, when it is false, it is guaranteed that
                    /// `Three::new` will return a `None` value.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        { true }
                    }
                    /// Return the first occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.find_raw(s, e) },
                            )
                        }
                    }
                    /// Return the last occurrence of one of the needle bytes in the given
                    /// haystack. If no such occurrence exists, then `None` is returned.
                    ///
                    /// The occurrence is reported as an offset into `haystack`. Its maximum
                    /// value is `haystack.len() - 1`.
                    #[inline]
                    pub fn rfind(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe {
                            generic::search_slice_with_raw(
                                haystack,
                                |s, e| { self.rfind_raw(s, e) },
                            )
                        }
                    }
                    /// Like `find`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn find_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::fwd_byte_by_byte(
                                start,
                                end,
                                |b| {
                                    b == self.0.needle1() || b == self.0.needle2()
                                        || b == self.0.needle3()
                                },
                            );
                        }
                        self.find_raw_impl(start, end)
                    }
                    /// Like `rfind`, but accepts and returns raw pointers.
                    ///
                    /// When a match is found, the pointer returned is guaranteed to be
                    /// `>= start` and `< end`.
                    ///
                    /// This routine is useful if you're already using raw pointers and would
                    /// like to avoid converting back to a slice before executing a search.
                    ///
                    /// # Safety
                    ///
                    /// * Both `start` and `end` must be valid for reads.
                    /// * Both `start` and `end` must point to an initialized value.
                    /// * Both `start` and `end` must point to the same allocated object and
                    /// must either be in bounds or at most one byte past the end of the
                    /// allocated object.
                    /// * Both `start` and `end` must be _derived from_ a pointer to the same
                    /// object.
                    /// * The distance between `start` and `end` must not overflow `isize`.
                    /// * The distance being in bounds must not rely on "wrapping around" the
                    /// address space.
                    ///
                    /// Note that callers may pass a pair of pointers such that `start >= end`.
                    /// In that case, `None` will always be returned.
                    #[inline]
                    pub unsafe fn rfind_raw(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        if start >= end {
                            return None;
                        }
                        if end.distance(start) < __m128i::BYTES {
                            return generic::rev_byte_by_byte(
                                start,
                                end,
                                |b| {
                                    b == self.0.needle1() || b == self.0.needle2()
                                        || b == self.0.needle3()
                                },
                            );
                        }
                        self.rfind_raw_impl(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Three::find_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Three`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.0.find_raw(start, end)
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as [`Three::rfind_raw`], except the distance between `start` and
                    /// `end` must be at least the size of an SSE2 vector (in bytes).
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Three`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn rfind_raw_impl(
                        &self,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        self.0.rfind_raw(start, end)
                    }
                    /// Returns an iterator over all occurrences of the needle byte in the
                    /// given haystack.
                    ///
                    /// The iterator returned implements `DoubleEndedIterator`. This means it
                    /// can also be used to find occurrences in reverse order.
                    #[inline]
                    pub fn iter<'a, 'h>(
                        &'a self,
                        haystack: &'h [u8],
                    ) -> ThreeIter<'a, 'h> {
                        ThreeIter {
                            searcher: self,
                            it: generic::Iter::new(haystack),
                        }
                    }
                }
                /// An iterator over all occurrences of three possible bytes in a haystack.
                ///
                /// This iterator implements `DoubleEndedIterator`, which means it can also be
                /// used to find occurrences in reverse order.
                ///
                /// This iterator is created by the [`Three::iter`] method.
                ///
                /// The lifetime parameters are as follows:
                ///
                /// * `'a` refers to the lifetime of the underlying [`Three`] searcher.
                /// * `'h` refers to the lifetime of the haystack being searched.
                pub struct ThreeIter<'a, 'h> {
                    searcher: &'a Three,
                    it: generic::Iter<'h>,
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::clone::Clone for ThreeIter<'a, 'h> {
                    #[inline]
                    fn clone(&self) -> ThreeIter<'a, 'h> {
                        ThreeIter {
                            searcher: ::core::clone::Clone::clone(&self.searcher),
                            it: ::core::clone::Clone::clone(&self.it),
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, 'h> ::core::fmt::Debug for ThreeIter<'a, 'h> {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_struct_field2_finish(
                            f,
                            "ThreeIter",
                            "searcher",
                            &self.searcher,
                            "it",
                            &&self.it,
                        )
                    }
                }
                impl<'a, 'h> Iterator for ThreeIter<'a, 'h> {
                    type Item = usize;
                    #[inline]
                    fn next(&mut self) -> Option<usize> {
                        unsafe { self.it.next(|s, e| self.searcher.find_raw(s, e)) }
                    }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.it.size_hint()
                    }
                }
                impl<'a, 'h> DoubleEndedIterator for ThreeIter<'a, 'h> {
                    #[inline]
                    fn next_back(&mut self) -> Option<usize> {
                        unsafe {
                            self.it.next_back(|s, e| self.searcher.rfind_raw(s, e))
                        }
                    }
                }
                impl<'a, 'h> core::iter::FusedIterator for ThreeIter<'a, 'h> {}
            }
            pub mod packedpair {
                /*!
A 128-bit vector implementation of the "packed pair" SIMD algorithm.

The "packed pair" algorithm is based on the [generic SIMD] algorithm. The main
difference is that it (by default) uses a background distribution of byte
frequencies to heuristically select the pair of bytes to search for.

[generic SIMD]: http://0x80.pl/articles/simd-strfind.html#first-and-last
*/
                use core::arch::x86_64::__m128i;
                use crate::arch::{all::packedpair::Pair, generic::packedpair};
                /// A "packed pair" finder that uses 128-bit vector operations.
                ///
                /// This finder picks two bytes that it believes have high predictive power
                /// for indicating an overall match of a needle. Depending on whether
                /// `Finder::find` or `Finder::find_prefilter` is used, it reports offsets
                /// where the needle matches or could match. In the prefilter case, candidates
                /// are reported whenever the [`Pair`] of bytes given matches.
                pub struct Finder(packedpair::Finder<__m128i>);
                #[automatically_derived]
                #[doc(hidden)]
                unsafe impl ::core::clone::TrivialClone for Finder {}
                #[automatically_derived]
                impl ::core::clone::Clone for Finder {
                    #[inline]
                    fn clone(&self) -> Finder {
                        let _: ::core::clone::AssertParamIsClone<
                            packedpair::Finder<__m128i>,
                        >;
                        *self
                    }
                }
                #[automatically_derived]
                impl ::core::marker::Copy for Finder {}
                #[automatically_derived]
                impl ::core::fmt::Debug for Finder {
                    #[inline]
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter,
                    ) -> ::core::fmt::Result {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Finder",
                            &&self.0,
                        )
                    }
                }
                impl Finder {
                    /// Create a new pair searcher. The searcher returned can either report
                    /// exact matches of `needle` or act as a prefilter and report candidate
                    /// positions of `needle`.
                    ///
                    /// If SSE2 is unavailable in the current environment or if a [`Pair`]
                    /// could not be constructed from the needle given, then `None` is
                    /// returned.
                    #[inline]
                    pub fn new(needle: &[u8]) -> Option<Finder> {
                        Finder::with_pair(needle, Pair::new(needle)?)
                    }
                    /// Create a new "packed pair" finder using the pair of bytes given.
                    ///
                    /// This constructor permits callers to control precisely which pair of
                    /// bytes is used as a predicate.
                    ///
                    /// If SSE2 is unavailable in the current environment, then `None` is
                    /// returned.
                    #[inline]
                    pub fn with_pair(needle: &[u8], pair: Pair) -> Option<Finder> {
                        if Finder::is_available() {
                            unsafe { Some(Finder::with_pair_impl(needle, pair)) }
                        } else {
                            None
                        }
                    }
                    /// Create a new `Finder` specific to SSE2 vectors and routines.
                    ///
                    /// # Safety
                    ///
                    /// Same as the safety for `packedpair::Finder::new`, and callers must also
                    /// ensure that SSE2 is available.
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn with_pair_impl(needle: &[u8], pair: Pair) -> Finder {
                        let finder = packedpair::Finder::<__m128i>::new(needle, pair);
                        Finder(finder)
                    }
                    /// Returns true when this implementation is available in the current
                    /// environment.
                    ///
                    /// When this is true, it is guaranteed that [`Finder::with_pair`] will
                    /// return a `Some` value. Similarly, when it is false, it is guaranteed
                    /// that `Finder::with_pair` will return a `None` value. Notice that this
                    /// does not guarantee that [`Finder::new`] will return a `Finder`. Namely,
                    /// even when `Finder::is_available` is true, it is not guaranteed that a
                    /// valid [`Pair`] can be found from the needle given.
                    ///
                    /// Note also that for the lifetime of a single program, if this returns
                    /// true then it will always return true.
                    #[inline]
                    pub fn is_available() -> bool {
                        { true }
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    #[inline]
                    pub fn find(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                        unsafe { self.find_impl(haystack, needle) }
                    }
                    /// Run this finder on the given haystack as a prefilter.
                    ///
                    /// If a candidate match is found, then an offset where the needle *could*
                    /// begin in the haystack is returned.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    #[inline]
                    pub fn find_prefilter(&self, haystack: &[u8]) -> Option<usize> {
                        unsafe { self.find_prefilter_impl(haystack) }
                    }
                    /// Execute a search using SSE2 vectors and routines.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    ///
                    /// # Safety
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Finder`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_impl(
                        &self,
                        haystack: &[u8],
                        needle: &[u8],
                    ) -> Option<usize> {
                        self.0.find(haystack, needle)
                    }
                    /// Execute a prefilter search using SSE2 vectors and routines.
                    ///
                    /// # Panics
                    ///
                    /// When `haystack.len()` is less than [`Finder::min_haystack_len`].
                    ///
                    /// # Safety
                    ///
                    /// (The target feature safety obligation is automatically fulfilled by
                    /// virtue of being a method on `Finder`, which can only be constructed
                    /// when it is safe to call `sse2` routines.)
                    #[target_feature(enable = "sse2")]
                    #[inline]
                    unsafe fn find_prefilter_impl(
                        &self,
                        haystack: &[u8],
                    ) -> Option<usize> {
                        self.0.find_prefilter(haystack)
                    }
                    /// Returns the pair of offsets (into the needle) used to check as a
                    /// predicate before confirming whether a needle exists at a particular
                    /// position.
                    #[inline]
                    pub fn pair(&self) -> &Pair {
                        self.0.pair()
                    }
                    /// Returns the minimum haystack length that this `Finder` can search.
                    ///
                    /// Using a haystack with length smaller than this in a search will result
                    /// in a panic. The reason for this restriction is that this finder is
                    /// meant to be a low-level component that is part of a larger substring
                    /// strategy. In that sense, it avoids trying to handle all cases and
                    /// instead only handles the cases that it can handle very well.
                    #[inline]
                    pub fn min_haystack_len(&self) -> usize {
                        self.0.min_haystack_len()
                    }
                }
            }
        }
        pub(crate) mod memchr {
            /*!
Wrapper routines for `memchr` and friends.

These routines efficiently dispatch to the best implementation based on what
the CPU supports.
*/
            /// memchr, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `One::find_raw`.
            #[inline(always)]
            pub(crate) fn memchr_raw(
                n1: u8,
                start: *const u8,
                end: *const u8,
            ) -> Option<*const u8> {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(
                        u8,
                        *const u8,
                        *const u8,
                    ) -> Option<*const u8>;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::avx2::memchr::One;
                        One::new_unchecked(n1).find_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::sse2::memchr::One;
                        One::new_unchecked(n1).find_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::all::memchr::One;
                        One::new(n1).find_raw(start, end)
                    }
                    unsafe fn detect(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::One::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::One::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, start, end)
                    }
                }
            }
            /// memrchr, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `One::rfind_raw`.
            #[inline(always)]
            pub(crate) fn memrchr_raw(
                n1: u8,
                start: *const u8,
                end: *const u8,
            ) -> Option<*const u8> {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(
                        u8,
                        *const u8,
                        *const u8,
                    ) -> Option<*const u8>;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::avx2::memchr::One;
                        One::new_unchecked(n1).rfind_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::sse2::memchr::One;
                        One::new_unchecked(n1).rfind_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::all::memchr::One;
                        One::new(n1).rfind_raw(start, end)
                    }
                    unsafe fn detect(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::One::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::One::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, start, end)
                    }
                }
            }
            /// memchr2, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `Two::find_raw`.
            #[inline(always)]
            pub(crate) fn memchr2_raw(
                n1: u8,
                n2: u8,
                start: *const u8,
                end: *const u8,
            ) -> Option<*const u8> {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(
                        u8,
                        u8,
                        *const u8,
                        *const u8,
                    ) -> Option<*const u8>;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::avx2::memchr::Two;
                        Two::new_unchecked(n1, n2).find_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::sse2::memchr::Two;
                        Two::new_unchecked(n1, n2).find_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::all::memchr::Two;
                        Two::new(n1, n2).find_raw(start, end)
                    }
                    unsafe fn detect(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::Two::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::Two::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, n2, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, n2, start, end)
                    }
                }
            }
            /// memrchr2, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `Two::rfind_raw`.
            #[inline(always)]
            pub(crate) fn memrchr2_raw(
                n1: u8,
                n2: u8,
                start: *const u8,
                end: *const u8,
            ) -> Option<*const u8> {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(
                        u8,
                        u8,
                        *const u8,
                        *const u8,
                    ) -> Option<*const u8>;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::avx2::memchr::Two;
                        Two::new_unchecked(n1, n2).rfind_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::sse2::memchr::Two;
                        Two::new_unchecked(n1, n2).rfind_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::all::memchr::Two;
                        Two::new(n1, n2).rfind_raw(start, end)
                    }
                    unsafe fn detect(
                        n1: u8,
                        n2: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::Two::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::Two::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, n2, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, n2, start, end)
                    }
                }
            }
            /// memchr3, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `Three::find_raw`.
            #[inline(always)]
            pub(crate) fn memchr3_raw(
                n1: u8,
                n2: u8,
                n3: u8,
                start: *const u8,
                end: *const u8,
            ) -> Option<*const u8> {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(
                        u8,
                        u8,
                        u8,
                        *const u8,
                        *const u8,
                    ) -> Option<*const u8>;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::avx2::memchr::Three;
                        Three::new_unchecked(n1, n2, n3).find_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::sse2::memchr::Three;
                        Three::new_unchecked(n1, n2, n3).find_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::all::memchr::Three;
                        Three::new(n1, n2, n3).find_raw(start, end)
                    }
                    unsafe fn detect(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::Three::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::Three::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, n2, n3, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, n2, n3, start, end)
                    }
                }
            }
            /// memrchr3, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `Three::rfind_raw`.
            #[inline(always)]
            pub(crate) fn memrchr3_raw(
                n1: u8,
                n2: u8,
                n3: u8,
                start: *const u8,
                end: *const u8,
            ) -> Option<*const u8> {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(
                        u8,
                        u8,
                        u8,
                        *const u8,
                        *const u8,
                    ) -> Option<*const u8>;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::avx2::memchr::Three;
                        Three::new_unchecked(n1, n2, n3).rfind_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::x86_64::sse2::memchr::Three;
                        Three::new_unchecked(n1, n2, n3).rfind_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        use crate::arch::all::memchr::Three;
                        Three::new(n1, n2, n3).rfind_raw(start, end)
                    }
                    unsafe fn detect(
                        n1: u8,
                        n2: u8,
                        n3: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> Option<*const u8> {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::Three::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::Three::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, n2, n3, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, n2, n3, start, end)
                    }
                }
            }
            /// Count all matching bytes, but using raw pointers to represent the haystack.
            ///
            /// # Safety
            ///
            /// Pointers must be valid. See `One::count_raw`.
            #[inline(always)]
            pub(crate) fn count_raw(n1: u8, start: *const u8, end: *const u8) -> usize {
                {
                    #![allow(unused_unsafe)]
                    use core::sync::atomic::{AtomicPtr, Ordering};
                    type Fn = *mut ();
                    type RealFn = unsafe fn(u8, *const u8, *const u8) -> usize;
                    static FN: AtomicPtr<()> = AtomicPtr::new(detect as Fn);
                    #[target_feature(enable = "sse2", enable = "avx2")]
                    unsafe fn find_avx2(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        use crate::arch::x86_64::avx2::memchr::One;
                        One::new_unchecked(n1).count_raw(start, end)
                    }
                    #[target_feature(enable = "sse2")]
                    unsafe fn find_sse2(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        use crate::arch::x86_64::sse2::memchr::One;
                        One::new_unchecked(n1).count_raw(start, end)
                    }
                    unsafe fn find_fallback(
                        n1: u8,
                        start: *const u8,
                        end: *const u8,
                    ) -> usize {
                        use crate::arch::all::memchr::One;
                        One::new(n1).count_raw(start, end)
                    }
                    unsafe fn detect(n1: u8, start: *const u8, end: *const u8) -> usize {
                        let fun = {
                            {
                                use crate::arch::x86_64::{sse2, avx2};
                                if avx2::memchr::One::is_available() {
                                    find_avx2 as RealFn
                                } else if sse2::memchr::One::is_available() {
                                    find_sse2 as RealFn
                                } else {
                                    find_fallback as RealFn
                                }
                            }
                        };
                        FN.store(fun as Fn, Ordering::Relaxed);
                        fun(n1, start, end)
                    }
                    unsafe {
                        let fun = FN.load(Ordering::Relaxed);
                        core::mem::transmute::<Fn, RealFn>(fun)(n1, start, end)
                    }
                }
            }
        }
    }
}
mod cow {
    use core::ops;
    /// A specialized copy-on-write byte string.
    ///
    /// The purpose of this type is to permit usage of a "borrowed or owned
    /// byte string" in a way that keeps std/no-std compatibility. That is, in
    /// no-std/alloc mode, this type devolves into a simple &[u8] with no owned
    /// variant available. We can't just use a plain Cow because Cow is not in
    /// core.
    pub struct CowBytes<'a>(Imp<'a>);
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for CowBytes<'a> {
        #[inline]
        fn clone(&self) -> CowBytes<'a> {
            CowBytes(::core::clone::Clone::clone(&self.0))
        }
    }
    #[automatically_derived]
    impl<'a> ::core::fmt::Debug for CowBytes<'a> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "CowBytes", &&self.0)
        }
    }
    enum Imp<'a> {
        Borrowed(&'a [u8]),
        Owned(alloc::boxed::Box<[u8]>),
    }
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for Imp<'a> {
        #[inline]
        fn clone(&self) -> Imp<'a> {
            match self {
                Imp::Borrowed(__self_0) => {
                    Imp::Borrowed(::core::clone::Clone::clone(__self_0))
                }
                Imp::Owned(__self_0) => Imp::Owned(::core::clone::Clone::clone(__self_0)),
            }
        }
    }
    #[automatically_derived]
    impl<'a> ::core::fmt::Debug for Imp<'a> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Imp::Borrowed(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Borrowed",
                        &__self_0,
                    )
                }
                Imp::Owned(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Owned",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl<'a> ops::Deref for CowBytes<'a> {
        type Target = [u8];
        #[inline(always)]
        fn deref(&self) -> &[u8] {
            self.as_slice()
        }
    }
    impl<'a> CowBytes<'a> {
        /// Create a new borrowed CowBytes.
        #[inline(always)]
        pub(crate) fn new<B: ?Sized + AsRef<[u8]>>(bytes: &'a B) -> CowBytes<'a> {
            CowBytes(Imp::new(bytes.as_ref()))
        }
        /// Create a new owned CowBytes.
        #[inline(always)]
        pub(crate) fn new_owned(bytes: alloc::boxed::Box<[u8]>) -> CowBytes<'static> {
            CowBytes(Imp::Owned(bytes))
        }
        /// Return a borrowed byte string, regardless of whether this is an owned
        /// or borrowed byte string internally.
        #[inline(always)]
        pub(crate) fn as_slice(&self) -> &[u8] {
            self.0.as_slice()
        }
        /// Return an owned version of this copy-on-write byte string.
        ///
        /// If this is already an owned byte string internally, then this is a
        /// no-op. Otherwise, the internal byte string is copied.
        #[inline(always)]
        pub(crate) fn into_owned(self) -> CowBytes<'static> {
            match self.0 {
                Imp::Borrowed(b) => CowBytes::new_owned(alloc::boxed::Box::from(b)),
                Imp::Owned(b) => CowBytes::new_owned(b),
            }
        }
    }
    impl<'a> Imp<'a> {
        #[inline(always)]
        pub fn new(bytes: &'a [u8]) -> Imp<'a> {
            { Imp::Borrowed(bytes) }
        }
        #[inline(always)]
        pub fn as_slice(&self) -> &[u8] {
            {
                match self {
                    Imp::Owned(ref x) => x,
                    Imp::Borrowed(x) => x,
                }
            }
        }
    }
}
mod ext {
    /// A trait for adding some helper routines to pointers.
    pub(crate) trait Pointer {
        /// Returns the distance, in units of `T`, between `self` and `origin`.
        ///
        /// # Safety
        ///
        /// Same as `ptr::offset_from` in addition to `self >= origin`.
        unsafe fn distance(self, origin: Self) -> usize;
        /// Casts this pointer to `usize`.
        ///
        /// Callers should not convert the `usize` back to a pointer if at all
        /// possible. (And if you believe it's necessary, open an issue to discuss
        /// why. Otherwise, it has the potential to violate pointer provenance.)
        /// The purpose of this function is just to be able to do arithmetic, i.e.,
        /// computing offsets or alignments.
        fn as_usize(self) -> usize;
    }
    impl<T> Pointer for *const T {
        unsafe fn distance(self, origin: *const T) -> usize {
            usize::try_from(self.offset_from(origin)).unwrap_unchecked()
        }
        fn as_usize(self) -> usize {
            self as usize
        }
    }
    impl<T> Pointer for *mut T {
        unsafe fn distance(self, origin: *mut T) -> usize {
            (self as *const T).distance(origin as *const T)
        }
        fn as_usize(self) -> usize {
            (self as *const T).as_usize()
        }
    }
}
mod memchr {
    use core::iter::Rev;
    use crate::arch::generic::memchr as generic;
    /// Search for the first occurrence of a byte in a slice.
    ///
    /// This returns the index corresponding to the first occurrence of `needle` in
    /// `haystack`, or `None` if one is not found. If an index is returned, it is
    /// guaranteed to be less than `haystack.len()`.
    ///
    /// While this is semantically the same as something like
    /// `haystack.iter().position(|&b| b == needle)`, this routine will attempt to
    /// use highly optimized vector operations that can be an order of magnitude
    /// faster (or more).
    ///
    /// # Example
    ///
    /// This shows how to find the first position of a byte in a byte string.
    ///
    /// ```
    /// use memchr::memchr;
    ///
    /// let haystack = b"the quick brown fox";
    /// assert_eq!(memchr(b'k', haystack), Some(8));
    /// ```
    #[inline]
    pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
        unsafe {
            generic::search_slice_with_raw(
                haystack,
                |start, end| { memchr_raw(needle, start, end) },
            )
        }
    }
    /// Search for the last occurrence of a byte in a slice.
    ///
    /// This returns the index corresponding to the last occurrence of `needle` in
    /// `haystack`, or `None` if one is not found. If an index is returned, it is
    /// guaranteed to be less than `haystack.len()`.
    ///
    /// While this is semantically the same as something like
    /// `haystack.iter().rposition(|&b| b == needle)`, this routine will attempt to
    /// use highly optimized vector operations that can be an order of magnitude
    /// faster (or more).
    ///
    /// # Example
    ///
    /// This shows how to find the last position of a byte in a byte string.
    ///
    /// ```
    /// use memchr::memrchr;
    ///
    /// let haystack = b"the quick brown fox";
    /// assert_eq!(memrchr(b'o', haystack), Some(17));
    /// ```
    #[inline]
    pub fn memrchr(needle: u8, haystack: &[u8]) -> Option<usize> {
        unsafe {
            generic::search_slice_with_raw(
                haystack,
                |start, end| { memrchr_raw(needle, start, end) },
            )
        }
    }
    /// Search for the first occurrence of two possible bytes in a haystack.
    ///
    /// This returns the index corresponding to the first occurrence of one of the
    /// needle bytes in `haystack`, or `None` if one is not found. If an index is
    /// returned, it is guaranteed to be less than `haystack.len()`.
    ///
    /// While this is semantically the same as something like
    /// `haystack.iter().position(|&b| b == needle1 || b == needle2)`, this routine
    /// will attempt to use highly optimized vector operations that can be an order
    /// of magnitude faster (or more).
    ///
    /// # Example
    ///
    /// This shows how to find the first position of one of two possible bytes in a
    /// haystack.
    ///
    /// ```
    /// use memchr::memchr2;
    ///
    /// let haystack = b"the quick brown fox";
    /// assert_eq!(memchr2(b'k', b'q', haystack), Some(4));
    /// ```
    #[inline]
    pub fn memchr2(needle1: u8, needle2: u8, haystack: &[u8]) -> Option<usize> {
        unsafe {
            generic::search_slice_with_raw(
                haystack,
                |start, end| { memchr2_raw(needle1, needle2, start, end) },
            )
        }
    }
    /// Search for the last occurrence of two possible bytes in a haystack.
    ///
    /// This returns the index corresponding to the last occurrence of one of the
    /// needle bytes in `haystack`, or `None` if one is not found. If an index is
    /// returned, it is guaranteed to be less than `haystack.len()`.
    ///
    /// While this is semantically the same as something like
    /// `haystack.iter().rposition(|&b| b == needle1 || b == needle2)`, this
    /// routine will attempt to use highly optimized vector operations that can be
    /// an order of magnitude faster (or more).
    ///
    /// # Example
    ///
    /// This shows how to find the last position of one of two possible bytes in a
    /// haystack.
    ///
    /// ```
    /// use memchr::memrchr2;
    ///
    /// let haystack = b"the quick brown fox";
    /// assert_eq!(memrchr2(b'k', b'o', haystack), Some(17));
    /// ```
    #[inline]
    pub fn memrchr2(needle1: u8, needle2: u8, haystack: &[u8]) -> Option<usize> {
        unsafe {
            generic::search_slice_with_raw(
                haystack,
                |start, end| { memrchr2_raw(needle1, needle2, start, end) },
            )
        }
    }
    /// Search for the first occurrence of three possible bytes in a haystack.
    ///
    /// This returns the index corresponding to the first occurrence of one of the
    /// needle bytes in `haystack`, or `None` if one is not found. If an index is
    /// returned, it is guaranteed to be less than `haystack.len()`.
    ///
    /// While this is semantically the same as something like
    /// `haystack.iter().position(|&b| b == needle1 || b == needle2 || b == needle3)`,
    /// this routine will attempt to use highly optimized vector operations that
    /// can be an order of magnitude faster (or more).
    ///
    /// # Example
    ///
    /// This shows how to find the first position of one of three possible bytes in
    /// a haystack.
    ///
    /// ```
    /// use memchr::memchr3;
    ///
    /// let haystack = b"the quick brown fox";
    /// assert_eq!(memchr3(b'k', b'q', b'u', haystack), Some(4));
    /// ```
    #[inline]
    pub fn memchr3(
        needle1: u8,
        needle2: u8,
        needle3: u8,
        haystack: &[u8],
    ) -> Option<usize> {
        unsafe {
            generic::search_slice_with_raw(
                haystack,
                |start, end| { memchr3_raw(needle1, needle2, needle3, start, end) },
            )
        }
    }
    /// Search for the last occurrence of three possible bytes in a haystack.
    ///
    /// This returns the index corresponding to the last occurrence of one of the
    /// needle bytes in `haystack`, or `None` if one is not found. If an index is
    /// returned, it is guaranteed to be less than `haystack.len()`.
    ///
    /// While this is semantically the same as something like
    /// `haystack.iter().rposition(|&b| b == needle1 || b == needle2 || b == needle3)`,
    /// this routine will attempt to use highly optimized vector operations that
    /// can be an order of magnitude faster (or more).
    ///
    /// # Example
    ///
    /// This shows how to find the last position of one of three possible bytes in
    /// a haystack.
    ///
    /// ```
    /// use memchr::memrchr3;
    ///
    /// let haystack = b"the quick brown fox";
    /// assert_eq!(memrchr3(b'k', b'o', b'n', haystack), Some(17));
    /// ```
    #[inline]
    pub fn memrchr3(
        needle1: u8,
        needle2: u8,
        needle3: u8,
        haystack: &[u8],
    ) -> Option<usize> {
        unsafe {
            generic::search_slice_with_raw(
                haystack,
                |start, end| { memrchr3_raw(needle1, needle2, needle3, start, end) },
            )
        }
    }
    /// Returns an iterator over all occurrences of the needle in a haystack.
    ///
    /// The iterator returned implements `DoubleEndedIterator`. This means it
    /// can also be used to find occurrences in reverse order.
    #[inline]
    pub fn memchr_iter<'h>(needle: u8, haystack: &'h [u8]) -> Memchr<'h> {
        Memchr::new(needle, haystack)
    }
    /// Returns an iterator over all occurrences of the needle in a haystack, in
    /// reverse.
    #[inline]
    pub fn memrchr_iter(needle: u8, haystack: &[u8]) -> Rev<Memchr<'_>> {
        Memchr::new(needle, haystack).rev()
    }
    /// Returns an iterator over all occurrences of the needles in a haystack.
    ///
    /// The iterator returned implements `DoubleEndedIterator`. This means it
    /// can also be used to find occurrences in reverse order.
    #[inline]
    pub fn memchr2_iter<'h>(
        needle1: u8,
        needle2: u8,
        haystack: &'h [u8],
    ) -> Memchr2<'h> {
        Memchr2::new(needle1, needle2, haystack)
    }
    /// Returns an iterator over all occurrences of the needles in a haystack, in
    /// reverse.
    #[inline]
    pub fn memrchr2_iter(needle1: u8, needle2: u8, haystack: &[u8]) -> Rev<Memchr2<'_>> {
        Memchr2::new(needle1, needle2, haystack).rev()
    }
    /// Returns an iterator over all occurrences of the needles in a haystack.
    ///
    /// The iterator returned implements `DoubleEndedIterator`. This means it
    /// can also be used to find occurrences in reverse order.
    #[inline]
    pub fn memchr3_iter<'h>(
        needle1: u8,
        needle2: u8,
        needle3: u8,
        haystack: &'h [u8],
    ) -> Memchr3<'h> {
        Memchr3::new(needle1, needle2, needle3, haystack)
    }
    /// Returns an iterator over all occurrences of the needles in a haystack, in
    /// reverse.
    #[inline]
    pub fn memrchr3_iter(
        needle1: u8,
        needle2: u8,
        needle3: u8,
        haystack: &[u8],
    ) -> Rev<Memchr3<'_>> {
        Memchr3::new(needle1, needle2, needle3, haystack).rev()
    }
    /// An iterator over all occurrences of a single byte in a haystack.
    ///
    /// This iterator implements `DoubleEndedIterator`, which means it can also be
    /// used to find occurrences in reverse order.
    ///
    /// This iterator is created by the [`memchr_iter`] or `[memrchr_iter`]
    /// functions. It can also be created with the [`Memchr::new`] method.
    ///
    /// The lifetime parameter `'h` refers to the lifetime of the haystack being
    /// searched.
    pub struct Memchr<'h> {
        needle1: u8,
        it: crate::arch::generic::memchr::Iter<'h>,
    }
    #[automatically_derived]
    impl<'h> ::core::clone::Clone for Memchr<'h> {
        #[inline]
        fn clone(&self) -> Memchr<'h> {
            Memchr {
                needle1: ::core::clone::Clone::clone(&self.needle1),
                it: ::core::clone::Clone::clone(&self.it),
            }
        }
    }
    #[automatically_derived]
    impl<'h> ::core::fmt::Debug for Memchr<'h> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Memchr",
                "needle1",
                &self.needle1,
                "it",
                &&self.it,
            )
        }
    }
    impl<'h> Memchr<'h> {
        /// Returns an iterator over all occurrences of the needle byte in the
        /// given haystack.
        ///
        /// The iterator returned implements `DoubleEndedIterator`. This means it
        /// can also be used to find occurrences in reverse order.
        #[inline]
        pub fn new(needle1: u8, haystack: &'h [u8]) -> Memchr<'h> {
            Memchr {
                needle1,
                it: crate::arch::generic::memchr::Iter::new(haystack),
            }
        }
    }
    impl<'h> Iterator for Memchr<'h> {
        type Item = usize;
        #[inline]
        fn next(&mut self) -> Option<usize> {
            unsafe { self.it.next(|s, e| memchr_raw(self.needle1, s, e)) }
        }
        #[inline]
        fn count(self) -> usize {
            self.it.count(|s, e| { unsafe { count_raw(self.needle1, s, e) } })
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.it.size_hint()
        }
    }
    impl<'h> DoubleEndedIterator for Memchr<'h> {
        #[inline]
        fn next_back(&mut self) -> Option<usize> {
            unsafe { self.it.next_back(|s, e| memrchr_raw(self.needle1, s, e)) }
        }
    }
    impl<'h> core::iter::FusedIterator for Memchr<'h> {}
    /// An iterator over all occurrences of two possible bytes in a haystack.
    ///
    /// This iterator implements `DoubleEndedIterator`, which means it can also be
    /// used to find occurrences in reverse order.
    ///
    /// This iterator is created by the [`memchr2_iter`] or `[memrchr2_iter`]
    /// functions. It can also be created with the [`Memchr2::new`] method.
    ///
    /// The lifetime parameter `'h` refers to the lifetime of the haystack being
    /// searched.
    pub struct Memchr2<'h> {
        needle1: u8,
        needle2: u8,
        it: crate::arch::generic::memchr::Iter<'h>,
    }
    #[automatically_derived]
    impl<'h> ::core::clone::Clone for Memchr2<'h> {
        #[inline]
        fn clone(&self) -> Memchr2<'h> {
            Memchr2 {
                needle1: ::core::clone::Clone::clone(&self.needle1),
                needle2: ::core::clone::Clone::clone(&self.needle2),
                it: ::core::clone::Clone::clone(&self.it),
            }
        }
    }
    #[automatically_derived]
    impl<'h> ::core::fmt::Debug for Memchr2<'h> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Memchr2",
                "needle1",
                &self.needle1,
                "needle2",
                &self.needle2,
                "it",
                &&self.it,
            )
        }
    }
    impl<'h> Memchr2<'h> {
        /// Returns an iterator over all occurrences of the needle bytes in the
        /// given haystack.
        ///
        /// The iterator returned implements `DoubleEndedIterator`. This means it
        /// can also be used to find occurrences in reverse order.
        #[inline]
        pub fn new(needle1: u8, needle2: u8, haystack: &'h [u8]) -> Memchr2<'h> {
            Memchr2 {
                needle1,
                needle2,
                it: crate::arch::generic::memchr::Iter::new(haystack),
            }
        }
    }
    impl<'h> Iterator for Memchr2<'h> {
        type Item = usize;
        #[inline]
        fn next(&mut self) -> Option<usize> {
            unsafe { self.it.next(|s, e| memchr2_raw(self.needle1, self.needle2, s, e)) }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.it.size_hint()
        }
    }
    impl<'h> DoubleEndedIterator for Memchr2<'h> {
        #[inline]
        fn next_back(&mut self) -> Option<usize> {
            unsafe {
                self.it
                    .next_back(|s, e| { memrchr2_raw(self.needle1, self.needle2, s, e) })
            }
        }
    }
    impl<'h> core::iter::FusedIterator for Memchr2<'h> {}
    /// An iterator over all occurrences of three possible bytes in a haystack.
    ///
    /// This iterator implements `DoubleEndedIterator`, which means it can also be
    /// used to find occurrences in reverse order.
    ///
    /// This iterator is created by the [`memchr2_iter`] or `[memrchr2_iter`]
    /// functions. It can also be created with the [`Memchr3::new`] method.
    ///
    /// The lifetime parameter `'h` refers to the lifetime of the haystack being
    /// searched.
    pub struct Memchr3<'h> {
        needle1: u8,
        needle2: u8,
        needle3: u8,
        it: crate::arch::generic::memchr::Iter<'h>,
    }
    #[automatically_derived]
    impl<'h> ::core::clone::Clone for Memchr3<'h> {
        #[inline]
        fn clone(&self) -> Memchr3<'h> {
            Memchr3 {
                needle1: ::core::clone::Clone::clone(&self.needle1),
                needle2: ::core::clone::Clone::clone(&self.needle2),
                needle3: ::core::clone::Clone::clone(&self.needle3),
                it: ::core::clone::Clone::clone(&self.it),
            }
        }
    }
    #[automatically_derived]
    impl<'h> ::core::fmt::Debug for Memchr3<'h> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "Memchr3",
                "needle1",
                &self.needle1,
                "needle2",
                &self.needle2,
                "needle3",
                &self.needle3,
                "it",
                &&self.it,
            )
        }
    }
    impl<'h> Memchr3<'h> {
        /// Returns an iterator over all occurrences of the needle bytes in the
        /// given haystack.
        ///
        /// The iterator returned implements `DoubleEndedIterator`. This means it
        /// can also be used to find occurrences in reverse order.
        #[inline]
        pub fn new(
            needle1: u8,
            needle2: u8,
            needle3: u8,
            haystack: &'h [u8],
        ) -> Memchr3<'h> {
            Memchr3 {
                needle1,
                needle2,
                needle3,
                it: crate::arch::generic::memchr::Iter::new(haystack),
            }
        }
    }
    impl<'h> Iterator for Memchr3<'h> {
        type Item = usize;
        #[inline]
        fn next(&mut self) -> Option<usize> {
            unsafe {
                self.it
                    .next(|s, e| {
                        memchr3_raw(self.needle1, self.needle2, self.needle3, s, e)
                    })
            }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.it.size_hint()
        }
    }
    impl<'h> DoubleEndedIterator for Memchr3<'h> {
        #[inline]
        fn next_back(&mut self) -> Option<usize> {
            unsafe {
                self.it
                    .next_back(|s, e| {
                        memrchr3_raw(self.needle1, self.needle2, self.needle3, s, e)
                    })
            }
        }
    }
    impl<'h> core::iter::FusedIterator for Memchr3<'h> {}
    /// memchr, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `One::find_raw`.
    #[inline]
    unsafe fn memchr_raw(
        needle: u8,
        start: *const u8,
        end: *const u8,
    ) -> Option<*const u8> {
        { crate::arch::x86_64::memchr::memchr_raw(needle, start, end) }
    }
    /// memrchr, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `One::rfind_raw`.
    #[inline]
    unsafe fn memrchr_raw(
        needle: u8,
        start: *const u8,
        end: *const u8,
    ) -> Option<*const u8> {
        { crate::arch::x86_64::memchr::memrchr_raw(needle, start, end) }
    }
    /// memchr2, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `Two::find_raw`.
    #[inline]
    unsafe fn memchr2_raw(
        needle1: u8,
        needle2: u8,
        start: *const u8,
        end: *const u8,
    ) -> Option<*const u8> {
        { crate::arch::x86_64::memchr::memchr2_raw(needle1, needle2, start, end) }
    }
    /// memrchr2, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `Two::rfind_raw`.
    #[inline]
    unsafe fn memrchr2_raw(
        needle1: u8,
        needle2: u8,
        start: *const u8,
        end: *const u8,
    ) -> Option<*const u8> {
        { crate::arch::x86_64::memchr::memrchr2_raw(needle1, needle2, start, end) }
    }
    /// memchr3, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `Three::find_raw`.
    #[inline]
    unsafe fn memchr3_raw(
        needle1: u8,
        needle2: u8,
        needle3: u8,
        start: *const u8,
        end: *const u8,
    ) -> Option<*const u8> {
        {
            crate::arch::x86_64::memchr::memchr3_raw(
                needle1,
                needle2,
                needle3,
                start,
                end,
            )
        }
    }
    /// memrchr3, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `Three::rfind_raw`.
    #[inline]
    unsafe fn memrchr3_raw(
        needle1: u8,
        needle2: u8,
        needle3: u8,
        start: *const u8,
        end: *const u8,
    ) -> Option<*const u8> {
        {
            crate::arch::x86_64::memchr::memrchr3_raw(
                needle1,
                needle2,
                needle3,
                start,
                end,
            )
        }
    }
    /// Count all matching bytes, but using raw pointers to represent the haystack.
    ///
    /// # Safety
    ///
    /// Pointers must be valid. See `One::count_raw`.
    #[inline]
    unsafe fn count_raw(needle: u8, start: *const u8, end: *const u8) -> usize {
        { crate::arch::x86_64::memchr::count_raw(needle, start, end) }
    }
}
pub mod memmem {
    /*!
This module provides forward and reverse substring search routines.

Unlike the standard library's substring search routines, these work on
arbitrary bytes. For all non-empty needles, these routines will report exactly
the same values as the corresponding routines in the standard library. For
the empty needle, the standard library reports matches only at valid UTF-8
boundaries, where as these routines will report matches at every position.

Other than being able to work on arbitrary bytes, the primary reason to prefer
these routines over the standard library routines is that these will generally
be faster. In some cases, significantly so.

# Example: iterating over substring matches

This example shows how to use [`find_iter`] to find occurrences of a substring
in a haystack.

```
use memchr::memmem;

let haystack = b"foo bar foo baz foo";

let mut it = memmem::find_iter(haystack, "foo");
assert_eq!(Some(0), it.next());
assert_eq!(Some(8), it.next());
assert_eq!(Some(16), it.next());
assert_eq!(None, it.next());
```

# Example: iterating over substring matches in reverse

This example shows how to use [`rfind_iter`] to find occurrences of a substring
in a haystack starting from the end of the haystack.

**NOTE:** This module does not implement double ended iterators, so reverse
searches aren't done by calling `rev` on a forward iterator.

```
use memchr::memmem;

let haystack = b"foo bar foo baz foo";

let mut it = memmem::rfind_iter(haystack, "foo");
assert_eq!(Some(16), it.next());
assert_eq!(Some(8), it.next());
assert_eq!(Some(0), it.next());
assert_eq!(None, it.next());
```

# Example: repeating a search for the same needle

It may be possible for the overhead of constructing a substring searcher to be
measurable in some workloads. In cases where the same needle is used to search
many haystacks, it is possible to do construction once and thus to avoid it for
subsequent searches. This can be done with a [`Finder`] (or a [`FinderRev`] for
reverse searches).

```
use memchr::memmem;

let finder = memmem::Finder::new("foo");

assert_eq!(Some(4), finder.find(b"baz foo quux"));
assert_eq!(None, finder.find(b"quux baz bar"));
```
*/
    pub use crate::memmem::searcher::PrefilterConfig as Prefilter;
    pub(crate) use crate::memmem::searcher::Pre;
    use crate::{
        arch::all::{
            packedpair::{DefaultFrequencyRank, HeuristicFrequencyRank},
            rabinkarp,
        },
        cow::CowBytes, memmem::searcher::{PrefilterState, Searcher, SearcherRev},
    };
    mod searcher {
        use crate::arch::all::{
            packedpair::{HeuristicFrequencyRank, Pair},
            rabinkarp, twoway,
        };
        use crate::arch::x86_64::{avx2::packedpair as avx2, sse2::packedpair as sse2};
        /// A "meta" substring searcher.
        ///
        /// To a first approximation, this chooses what it believes to be the "best"
        /// substring search implemnetation based on the needle at construction time.
        /// Then, every call to `find` will execute that particular implementation. To
        /// a second approximation, multiple substring search algorithms may be used,
        /// depending on the haystack. For example, for supremely short haystacks,
        /// Rabin-Karp is typically used.
        ///
        /// See the documentation on `Prefilter` for an explanation of the dispatching
        /// mechanism. The quick summary is that an enum has too much overhead and
        /// we can't use dynamic dispatch via traits because we need to work in a
        /// core-only environment. (Dynamic dispatch works in core-only, but you
        /// need `&dyn Trait` and we really need a `Box<dyn Trait>` here. The latter
        /// requires `alloc`.) So instead, we use a union and an appropriately paired
        /// free function to read from the correct field on the union and execute the
        /// chosen substring search implementation.
        pub(crate) struct Searcher {
            call: SearcherKindFn,
            kind: SearcherKind,
            rabinkarp: rabinkarp::Finder,
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Searcher {
            #[inline]
            fn clone(&self) -> Searcher {
                Searcher {
                    call: ::core::clone::Clone::clone(&self.call),
                    kind: ::core::clone::Clone::clone(&self.kind),
                    rabinkarp: ::core::clone::Clone::clone(&self.rabinkarp),
                }
            }
        }
        impl Searcher {
            /// Creates a new "meta" substring searcher that attempts to choose the
            /// best algorithm based on the needle, heuristics and what the current
            /// target supports.
            #[inline]
            pub(crate) fn new<R: HeuristicFrequencyRank>(
                prefilter: PrefilterConfig,
                ranker: R,
                needle: &[u8],
            ) -> Searcher {
                let rabinkarp = rabinkarp::Finder::new(needle);
                if needle.len() <= 1 {
                    return if needle.is_empty() {
                        Searcher {
                            call: searcher_kind_empty,
                            kind: SearcherKind { empty: () },
                            rabinkarp,
                        }
                    } else {
                        if true {
                            match (&1, &needle.len()) {
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
                        Searcher {
                            call: searcher_kind_one_byte,
                            kind: SearcherKind {
                                one_byte: needle[0],
                            },
                            rabinkarp,
                        }
                    };
                }
                let pair = match Pair::with_ranker(needle, &ranker) {
                    Some(pair) => pair,
                    None => return Searcher::twoway(needle, rabinkarp, None),
                };
                if true {
                    match (&(pair.index1()), &(pair.index2())) {
                        (left_val, right_val) => {
                            if *left_val == *right_val {
                                let kind = ::core::panicking::AssertKind::Ne;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::Some(
                                        format_args!("pair offsets should not be equivalent"),
                                    ),
                                );
                            }
                        }
                    };
                }
                {
                    if let Some(pp) = avx2::Finder::with_pair(needle, pair) {
                        if do_packed_search(needle) {
                            let kind = SearcherKind { avx2: pp };
                            Searcher {
                                call: searcher_kind_avx2,
                                kind,
                                rabinkarp,
                            }
                        } else if prefilter.is_none() {
                            Searcher::twoway(needle, rabinkarp, None)
                        } else {
                            let prestrat = Prefilter::avx2(pp, needle);
                            Searcher::twoway(needle, rabinkarp, Some(prestrat))
                        }
                    } else if let Some(pp) = sse2::Finder::with_pair(needle, pair) {
                        if do_packed_search(needle) {
                            let kind = SearcherKind { sse2: pp };
                            Searcher {
                                call: searcher_kind_sse2,
                                kind,
                                rabinkarp,
                            }
                        } else if prefilter.is_none() {
                            Searcher::twoway(needle, rabinkarp, None)
                        } else {
                            let prestrat = Prefilter::sse2(pp, needle);
                            Searcher::twoway(needle, rabinkarp, Some(prestrat))
                        }
                    } else if prefilter.is_none() {
                        Searcher::twoway(needle, rabinkarp, None)
                    } else {
                        let prestrat = Prefilter::fallback(ranker, pair, needle);
                        Searcher::twoway(needle, rabinkarp, prestrat)
                    }
                }
            }
            /// Creates a new searcher that always uses the Two-Way algorithm. This is
            /// typically used when vector algorithms are unavailable or inappropriate.
            /// (For example, when the needle is "too long.")
            ///
            /// If a prefilter is given, then the searcher returned will be accelerated
            /// by the prefilter.
            #[inline]
            fn twoway(
                needle: &[u8],
                rabinkarp: rabinkarp::Finder,
                prestrat: Option<Prefilter>,
            ) -> Searcher {
                let finder = twoway::Finder::new(needle);
                match prestrat {
                    None => {
                        let kind = SearcherKind { two_way: finder };
                        Searcher {
                            call: searcher_kind_two_way,
                            kind,
                            rabinkarp,
                        }
                    }
                    Some(prestrat) => {
                        let two_way_with_prefilter = TwoWayWithPrefilter {
                            finder,
                            prestrat,
                        };
                        let kind = SearcherKind {
                            two_way_with_prefilter,
                        };
                        Searcher {
                            call: searcher_kind_two_way_with_prefilter,
                            kind,
                            rabinkarp,
                        }
                    }
                }
            }
            /// Searches the given haystack for the given needle. The needle given
            /// should be the same as the needle that this finder was initialized
            /// with.
            ///
            /// Inlining this can lead to big wins for latency, and #[inline] doesn't
            /// seem to be enough in some cases.
            #[inline(always)]
            pub(crate) fn find(
                &self,
                prestate: &mut PrefilterState,
                haystack: &[u8],
                needle: &[u8],
            ) -> Option<usize> {
                if haystack.len() < needle.len() {
                    None
                } else {
                    unsafe { (self.call)(self, prestate, haystack, needle) }
                }
            }
        }
        impl core::fmt::Debug for Searcher {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("Searcher")
                    .field("call", &"<searcher function>")
                    .field("kind", &"<searcher kind union>")
                    .field("rabinkarp", &self.rabinkarp)
                    .finish()
            }
        }
        /// A union indicating one of several possible substring search implementations
        /// that are in active use.
        ///
        /// This union should only be read by one of the functions prefixed with
        /// `searcher_kind_`. Namely, the correct function is meant to be paired with
        /// the union by the caller, such that the function always reads from the
        /// designated union field.
        union SearcherKind {
            empty: (),
            one_byte: u8,
            two_way: twoway::Finder,
            two_way_with_prefilter: TwoWayWithPrefilter,
            sse2: crate::arch::x86_64::sse2::packedpair::Finder,
            avx2: crate::arch::x86_64::avx2::packedpair::Finder,
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for SearcherKind {}
        #[automatically_derived]
        impl ::core::clone::Clone for SearcherKind {
            #[inline]
            fn clone(&self) -> SearcherKind {
                let _: ::core::clone::AssertParamIsCopy<Self>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for SearcherKind {}
        /// A two-way substring searcher with a prefilter.
        struct TwoWayWithPrefilter {
            finder: twoway::Finder,
            prestrat: Prefilter,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for TwoWayWithPrefilter {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for TwoWayWithPrefilter {}
        #[automatically_derived]
        impl ::core::clone::Clone for TwoWayWithPrefilter {
            #[inline]
            fn clone(&self) -> TwoWayWithPrefilter {
                let _: ::core::clone::AssertParamIsClone<twoway::Finder>;
                let _: ::core::clone::AssertParamIsClone<Prefilter>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for TwoWayWithPrefilter {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "TwoWayWithPrefilter",
                    "finder",
                    &self.finder,
                    "prestrat",
                    &&self.prestrat,
                )
            }
        }
        /// The type of a substring search function.
        ///
        /// # Safety
        ///
        /// When using a function of this type, callers must ensure that the correct
        /// function is paired with the value populated in `SearcherKind` union.
        type SearcherKindFn = unsafe fn(
            searcher: &Searcher,
            prestate: &mut PrefilterState,
            haystack: &[u8],
            needle: &[u8],
        ) -> Option<usize>;
        /// Reads from the `empty` field of `SearcherKind` to handle the case of
        /// searching for the empty needle. Works on all platforms.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `searcher.kind.empty` union field is set.
        unsafe fn searcher_kind_empty(
            _searcher: &Searcher,
            _prestate: &mut PrefilterState,
            _haystack: &[u8],
            _needle: &[u8],
        ) -> Option<usize> {
            Some(0)
        }
        /// Reads from the `one_byte` field of `SearcherKind` to handle the case of
        /// searching for a single byte needle. Works on all platforms.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `searcher.kind.one_byte` union field is set.
        unsafe fn searcher_kind_one_byte(
            searcher: &Searcher,
            _prestate: &mut PrefilterState,
            haystack: &[u8],
            _needle: &[u8],
        ) -> Option<usize> {
            let needle = searcher.kind.one_byte;
            crate::memchr(needle, haystack)
        }
        /// Reads from the `two_way` field of `SearcherKind` to handle the case of
        /// searching for an arbitrary needle without prefilter acceleration. Works on
        /// all platforms.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `searcher.kind.two_way` union field is set.
        unsafe fn searcher_kind_two_way(
            searcher: &Searcher,
            _prestate: &mut PrefilterState,
            haystack: &[u8],
            needle: &[u8],
        ) -> Option<usize> {
            if rabinkarp::is_fast(haystack, needle) {
                searcher.rabinkarp.find(haystack, needle)
            } else {
                searcher.kind.two_way.find(haystack, needle)
            }
        }
        /// Reads from the `two_way_with_prefilter` field of `SearcherKind` to handle
        /// the case of searching for an arbitrary needle with prefilter acceleration.
        /// Works on all platforms.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `searcher.kind.two_way_with_prefilter` union
        /// field is set.
        unsafe fn searcher_kind_two_way_with_prefilter(
            searcher: &Searcher,
            prestate: &mut PrefilterState,
            haystack: &[u8],
            needle: &[u8],
        ) -> Option<usize> {
            if rabinkarp::is_fast(haystack, needle) {
                searcher.rabinkarp.find(haystack, needle)
            } else {
                let TwoWayWithPrefilter { ref finder, ref prestrat } = searcher
                    .kind
                    .two_way_with_prefilter;
                let pre = Pre { prestate, prestrat };
                finder.find_with_prefilter(Some(pre), haystack, needle)
            }
        }
        /// Reads from the `sse2` field of `SearcherKind` to execute the x86_64 SSE2
        /// vectorized substring search implementation.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `searcher.kind.sse2` union field is set.
        unsafe fn searcher_kind_sse2(
            searcher: &Searcher,
            _prestate: &mut PrefilterState,
            haystack: &[u8],
            needle: &[u8],
        ) -> Option<usize> {
            let finder = &searcher.kind.sse2;
            if haystack.len() < finder.min_haystack_len() {
                searcher.rabinkarp.find(haystack, needle)
            } else {
                finder.find(haystack, needle)
            }
        }
        /// Reads from the `avx2` field of `SearcherKind` to execute the x86_64 AVX2
        /// vectorized substring search implementation.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `searcher.kind.avx2` union field is set.
        unsafe fn searcher_kind_avx2(
            searcher: &Searcher,
            _prestate: &mut PrefilterState,
            haystack: &[u8],
            needle: &[u8],
        ) -> Option<usize> {
            let finder = &searcher.kind.avx2;
            if haystack.len() < finder.min_haystack_len() {
                searcher.rabinkarp.find(haystack, needle)
            } else {
                finder.find(haystack, needle)
            }
        }
        /// A reverse substring searcher.
        pub(crate) struct SearcherRev {
            kind: SearcherRevKind,
            rabinkarp: rabinkarp::FinderRev,
        }
        #[automatically_derived]
        impl ::core::clone::Clone for SearcherRev {
            #[inline]
            fn clone(&self) -> SearcherRev {
                SearcherRev {
                    kind: ::core::clone::Clone::clone(&self.kind),
                    rabinkarp: ::core::clone::Clone::clone(&self.rabinkarp),
                }
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for SearcherRev {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "SearcherRev",
                    "kind",
                    &self.kind,
                    "rabinkarp",
                    &&self.rabinkarp,
                )
            }
        }
        /// The kind of the reverse searcher.
        ///
        /// For the reverse case, we don't do any SIMD acceleration or prefilters.
        /// There is no specific technical reason why we don't, but rather don't do it
        /// because it's not clear it's worth the extra code to do so. If you have a
        /// use case for it, please file an issue.
        ///
        /// We also don't do the union trick as we do with the forward case and
        /// prefilters. Basically for the same reason we don't have prefilters or
        /// vector algorithms for reverse searching: it's not clear it's worth doing.
        /// Please file an issue if you have a compelling use case for fast reverse
        /// substring search.
        enum SearcherRevKind {
            Empty,
            OneByte { needle: u8 },
            TwoWay { finder: twoway::FinderRev },
        }
        #[automatically_derived]
        impl ::core::clone::Clone for SearcherRevKind {
            #[inline]
            fn clone(&self) -> SearcherRevKind {
                match self {
                    SearcherRevKind::Empty => SearcherRevKind::Empty,
                    SearcherRevKind::OneByte { needle: __self_0 } => {
                        SearcherRevKind::OneByte {
                            needle: ::core::clone::Clone::clone(__self_0),
                        }
                    }
                    SearcherRevKind::TwoWay { finder: __self_0 } => {
                        SearcherRevKind::TwoWay {
                            finder: ::core::clone::Clone::clone(__self_0),
                        }
                    }
                }
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for SearcherRevKind {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    SearcherRevKind::Empty => {
                        ::core::fmt::Formatter::write_str(f, "Empty")
                    }
                    SearcherRevKind::OneByte { needle: __self_0 } => {
                        ::core::fmt::Formatter::debug_struct_field1_finish(
                            f,
                            "OneByte",
                            "needle",
                            &__self_0,
                        )
                    }
                    SearcherRevKind::TwoWay { finder: __self_0 } => {
                        ::core::fmt::Formatter::debug_struct_field1_finish(
                            f,
                            "TwoWay",
                            "finder",
                            &__self_0,
                        )
                    }
                }
            }
        }
        impl SearcherRev {
            /// Creates a new searcher for finding occurrences of the given needle in
            /// reverse. That is, it reports the last (instead of the first) occurrence
            /// of a needle in a haystack.
            #[inline]
            pub(crate) fn new(needle: &[u8]) -> SearcherRev {
                let kind = if needle.len() <= 1 {
                    if needle.is_empty() {
                        SearcherRevKind::Empty
                    } else {
                        if true {
                            match (&1, &needle.len()) {
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
                        SearcherRevKind::OneByte {
                            needle: needle[0],
                        }
                    }
                } else {
                    let finder = twoway::FinderRev::new(needle);
                    SearcherRevKind::TwoWay { finder }
                };
                let rabinkarp = rabinkarp::FinderRev::new(needle);
                SearcherRev { kind, rabinkarp }
            }
            /// Searches the given haystack for the last occurrence of the given
            /// needle. The needle given should be the same as the needle that this
            /// finder was initialized with.
            #[inline]
            pub(crate) fn rfind(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
                if haystack.len() < needle.len() {
                    return None;
                }
                match self.kind {
                    SearcherRevKind::Empty => Some(haystack.len()),
                    SearcherRevKind::OneByte { needle } => {
                        crate::memrchr(needle, haystack)
                    }
                    SearcherRevKind::TwoWay { ref finder } => {
                        if rabinkarp::is_fast(haystack, needle) {
                            self.rabinkarp.rfind(haystack, needle)
                        } else {
                            finder.rfind(haystack, needle)
                        }
                    }
                }
            }
        }
        /// Prefilter controls whether heuristics are used to accelerate searching.
        ///
        /// A prefilter refers to the idea of detecting candidate matches very quickly,
        /// and then confirming whether those candidates are full matches. This
        /// idea can be quite effective since it's often the case that looking for
        /// candidates can be a lot faster than running a complete substring search
        /// over the entire input. Namely, looking for candidates can be done with
        /// extremely fast vectorized code.
        ///
        /// The downside of a prefilter is that it assumes false positives (which are
        /// candidates generated by a prefilter that aren't matches) are somewhat rare
        /// relative to the frequency of full matches. That is, if a lot of false
        /// positives are generated, then it's possible for search time to be worse
        /// than if the prefilter wasn't enabled in the first place.
        ///
        /// Another downside of a prefilter is that it can result in highly variable
        /// performance, where some cases are extraordinarily fast and others aren't.
        /// Typically, variable performance isn't a problem, but it may be for your use
        /// case.
        ///
        /// The use of prefilters in this implementation does use a heuristic to detect
        /// when a prefilter might not be carrying its weight, and will dynamically
        /// disable its use. Nevertheless, this configuration option gives callers
        /// the ability to disable prefilters if you have knowledge that they won't be
        /// useful.
        #[non_exhaustive]
        pub enum PrefilterConfig {
            /// Never used a prefilter in substring search.
            None,
            /// Automatically detect whether a heuristic prefilter should be used. If
            /// it is used, then heuristics will be used to dynamically disable the
            /// prefilter if it is believed to not be carrying its weight.
            Auto,
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for PrefilterConfig {}
        #[automatically_derived]
        impl ::core::clone::Clone for PrefilterConfig {
            #[inline]
            fn clone(&self) -> PrefilterConfig {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for PrefilterConfig {}
        #[automatically_derived]
        impl ::core::fmt::Debug for PrefilterConfig {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(
                    f,
                    match self {
                        PrefilterConfig::None => "None",
                        PrefilterConfig::Auto => "Auto",
                    },
                )
            }
        }
        impl Default for PrefilterConfig {
            fn default() -> PrefilterConfig {
                PrefilterConfig::Auto
            }
        }
        impl PrefilterConfig {
            /// Returns true when this prefilter is set to the `None` variant.
            fn is_none(&self) -> bool {
                #[allow(non_exhaustive_omitted_patterns)]
                match *self {
                    PrefilterConfig::None => true,
                    _ => false,
                }
            }
        }
        /// The implementation of a prefilter.
        ///
        /// This type encapsulates dispatch to one of several possible choices for a
        /// prefilter. Generally speaking, all prefilters have the same approximate
        /// algorithm: they choose a couple of bytes from the needle that are believed
        /// to be rare, use a fast vector algorithm to look for those bytes and return
        /// positions as candidates for some substring search algorithm (currently only
        /// Two-Way) to confirm as a match or not.
        ///
        /// The differences between the algorithms are actually at the vector
        /// implementation level. Namely, we need different routines based on both
        /// which target architecture we're on and what CPU features are supported.
        ///
        /// The straight-forwardly obvious approach here is to use an enum, and make
        /// `Prefilter::find` do case analysis to determine which algorithm was
        /// selected and invoke it. However, I've observed that this leads to poor
        /// codegen in some cases, especially in latency sensitive benchmarks. That is,
        /// this approach comes with overhead that I wasn't able to eliminate.
        ///
        /// The second obvious approach is to use dynamic dispatch with traits. Doing
        /// that in this context where `Prefilter` owns the selection generally
        /// requires heap allocation, and this code is designed to run in core-only
        /// environments.
        ///
        /// So we settle on using a union (that's `PrefilterKind`) and a function
        /// pointer (that's `PrefilterKindFn`). We select the right function pointer
        /// based on which field in the union we set, and that function in turn
        /// knows which field of the union to access. The downside of this approach
        /// is that it forces us to think about safety, but the upside is that
        /// there are some nice latency improvements to benchmarks. (Especially the
        /// `memmem/sliceslice/short` benchmark.)
        ///
        /// In cases where we've selected a vector algorithm and the haystack given
        /// is too short, we fallback to the scalar version of `memchr` on the
        /// `rarest_byte`. (The scalar version of `memchr` is still better than a naive
        /// byte-at-a-time loop because it will read in `usize`-sized chunks at a
        /// time.)
        struct Prefilter {
            call: PrefilterKindFn,
            kind: PrefilterKind,
            rarest_byte: u8,
            rarest_offset: u8,
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Prefilter {}
        #[automatically_derived]
        impl ::core::clone::Clone for Prefilter {
            #[inline]
            fn clone(&self) -> Prefilter {
                let _: ::core::clone::AssertParamIsClone<PrefilterKindFn>;
                let _: ::core::clone::AssertParamIsClone<PrefilterKind>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Prefilter {}
        impl Prefilter {
            /// Return a "fallback" prefilter, but only if it is believed to be
            /// effective.
            #[inline]
            fn fallback<R: HeuristicFrequencyRank>(
                ranker: R,
                pair: Pair,
                needle: &[u8],
            ) -> Option<Prefilter> {
                /// The maximum frequency rank permitted for the fallback prefilter.
                /// If the rarest byte in the needle has a frequency rank above this
                /// value, then no prefilter is used if the fallback prefilter would
                /// otherwise be selected.
                const MAX_FALLBACK_RANK: u8 = 250;
                let rarest_offset = pair.index1();
                let rarest_byte = needle[usize::from(rarest_offset)];
                let rarest_rank = ranker.rank(rarest_byte);
                if rarest_rank > MAX_FALLBACK_RANK {
                    None
                } else {
                    let finder = crate::arch::all::packedpair::Finder::with_pair(
                        needle,
                        pair.clone(),
                    )?;
                    let call = prefilter_kind_fallback;
                    let kind = PrefilterKind { fallback: finder };
                    Some(Prefilter {
                        call,
                        kind,
                        rarest_byte,
                        rarest_offset,
                    })
                }
            }
            /// Return a prefilter using a x86_64 SSE2 vector algorithm.
            #[inline]
            fn sse2(finder: sse2::Finder, needle: &[u8]) -> Prefilter {
                let rarest_offset = finder.pair().index1();
                let rarest_byte = needle[usize::from(rarest_offset)];
                Prefilter {
                    call: prefilter_kind_sse2,
                    kind: PrefilterKind { sse2: finder },
                    rarest_byte,
                    rarest_offset,
                }
            }
            /// Return a prefilter using a x86_64 AVX2 vector algorithm.
            #[inline]
            fn avx2(finder: avx2::Finder, needle: &[u8]) -> Prefilter {
                let rarest_offset = finder.pair().index1();
                let rarest_byte = needle[usize::from(rarest_offset)];
                Prefilter {
                    call: prefilter_kind_avx2,
                    kind: PrefilterKind { avx2: finder },
                    rarest_byte,
                    rarest_offset,
                }
            }
            /// Return a *candidate* position for a match.
            ///
            /// When this returns an offset, it implies that a match could begin at
            /// that offset, but it may not. That is, it is possible for a false
            /// positive to be returned.
            ///
            /// When `None` is returned, then it is guaranteed that there are no
            /// matches for the needle in the given haystack. That is, it is impossible
            /// for a false negative to be returned.
            ///
            /// The purpose of this routine is to look for candidate matching positions
            /// as quickly as possible before running a (likely) slower confirmation
            /// step.
            #[inline]
            fn find(&self, haystack: &[u8]) -> Option<usize> {
                unsafe { (self.call)(self, haystack) }
            }
            /// A "simple" prefilter that just looks for the occurrence of the rarest
            /// byte from the needle. This is generally only used for very small
            /// haystacks.
            #[inline]
            fn find_simple(&self, haystack: &[u8]) -> Option<usize> {
                crate::arch::all::memchr::One::new(self.rarest_byte)
                    .find(haystack)
                    .map(|i| i.saturating_sub(usize::from(self.rarest_offset)))
            }
        }
        impl core::fmt::Debug for Prefilter {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("Prefilter")
                    .field("call", &"<prefilter function>")
                    .field("kind", &"<prefilter kind union>")
                    .field("rarest_byte", &self.rarest_byte)
                    .field("rarest_offset", &self.rarest_offset)
                    .finish()
            }
        }
        /// A union indicating one of several possible prefilters that are in active
        /// use.
        ///
        /// This union should only be read by one of the functions prefixed with
        /// `prefilter_kind_`. Namely, the correct function is meant to be paired with
        /// the union by the caller, such that the function always reads from the
        /// designated union field.
        union PrefilterKind {
            fallback: crate::arch::all::packedpair::Finder,
            sse2: crate::arch::x86_64::sse2::packedpair::Finder,
            avx2: crate::arch::x86_64::avx2::packedpair::Finder,
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for PrefilterKind {}
        #[automatically_derived]
        impl ::core::clone::Clone for PrefilterKind {
            #[inline]
            fn clone(&self) -> PrefilterKind {
                let _: ::core::clone::AssertParamIsCopy<Self>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for PrefilterKind {}
        /// The type of a prefilter function.
        ///
        /// # Safety
        ///
        /// When using a function of this type, callers must ensure that the correct
        /// function is paired with the value populated in `PrefilterKind` union.
        type PrefilterKindFn = unsafe fn(
            strat: &Prefilter,
            haystack: &[u8],
        ) -> Option<usize>;
        /// Reads from the `fallback` field of `PrefilterKind` to execute the fallback
        /// prefilter. Works on all platforms.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `strat.kind.fallback` union field is set.
        unsafe fn prefilter_kind_fallback(
            strat: &Prefilter,
            haystack: &[u8],
        ) -> Option<usize> {
            strat.kind.fallback.find_prefilter(haystack)
        }
        /// Reads from the `sse2` field of `PrefilterKind` to execute the x86_64 SSE2
        /// prefilter.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `strat.kind.sse2` union field is set.
        unsafe fn prefilter_kind_sse2(
            strat: &Prefilter,
            haystack: &[u8],
        ) -> Option<usize> {
            let finder = &strat.kind.sse2;
            if haystack.len() < finder.min_haystack_len() {
                strat.find_simple(haystack)
            } else {
                finder.find_prefilter(haystack)
            }
        }
        /// Reads from the `avx2` field of `PrefilterKind` to execute the x86_64 AVX2
        /// prefilter.
        ///
        /// # Safety
        ///
        /// Callers must ensure that the `strat.kind.avx2` union field is set.
        unsafe fn prefilter_kind_avx2(
            strat: &Prefilter,
            haystack: &[u8],
        ) -> Option<usize> {
            let finder = &strat.kind.avx2;
            if haystack.len() < finder.min_haystack_len() {
                strat.find_simple(haystack)
            } else {
                finder.find_prefilter(haystack)
            }
        }
        /// PrefilterState tracks state associated with the effectiveness of a
        /// prefilter. It is used to track how many bytes, on average, are skipped by
        /// the prefilter. If this average dips below a certain threshold over time,
        /// then the state renders the prefilter inert and stops using it.
        ///
        /// A prefilter state should be created for each search. (Where creating an
        /// iterator is treated as a single search.) A prefilter state should only be
        /// created from a `Freqy`. e.g., An inert `Freqy` will produce an inert
        /// `PrefilterState`.
        pub(crate) struct PrefilterState {
            /// The number of skips that has been executed. This is always 1 greater
            /// than the actual number of skips. The special sentinel value of 0
            /// indicates that the prefilter is inert. This is useful to avoid
            /// additional checks to determine whether the prefilter is still
            /// "effective." Once a prefilter becomes inert, it should no longer be
            /// used (according to our heuristics).
            skips: u32,
            /// The total number of bytes that have been skipped.
            skipped: u32,
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for PrefilterState {}
        #[automatically_derived]
        impl ::core::clone::Clone for PrefilterState {
            #[inline]
            fn clone(&self) -> PrefilterState {
                let _: ::core::clone::AssertParamIsClone<u32>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for PrefilterState {}
        #[automatically_derived]
        impl ::core::fmt::Debug for PrefilterState {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "PrefilterState",
                    "skips",
                    &self.skips,
                    "skipped",
                    &&self.skipped,
                )
            }
        }
        impl PrefilterState {
            /// The minimum number of skip attempts to try before considering whether
            /// a prefilter is effective or not.
            const MIN_SKIPS: u32 = 50;
            /// The minimum amount of bytes that skipping must average.
            ///
            /// This value was chosen based on varying it and checking
            /// the microbenchmarks. In particular, this can impact the
            /// pathological/repeated-{huge,small} benchmarks quite a bit if it's set
            /// too low.
            const MIN_SKIP_BYTES: u32 = 8;
            /// Create a fresh prefilter state.
            #[inline]
            pub(crate) fn new() -> PrefilterState {
                PrefilterState {
                    skips: 1,
                    skipped: 0,
                }
            }
            /// Update this state with the number of bytes skipped on the last
            /// invocation of the prefilter.
            #[inline]
            fn update(&mut self, skipped: usize) {
                self.skips = self.skips.saturating_add(1);
                self.skipped = match u32::try_from(skipped) {
                    Err(_) => core::u32::MAX,
                    Ok(skipped) => self.skipped.saturating_add(skipped),
                };
            }
            /// Return true if and only if this state indicates that a prefilter is
            /// still effective.
            #[inline]
            fn is_effective(&mut self) -> bool {
                if self.is_inert() {
                    return false;
                }
                if self.skips() < PrefilterState::MIN_SKIPS {
                    return true;
                }
                if self.skipped >= PrefilterState::MIN_SKIP_BYTES * self.skips() {
                    return true;
                }
                self.skips = 0;
                false
            }
            /// Returns true if the prefilter this state represents should no longer
            /// be used.
            #[inline]
            fn is_inert(&self) -> bool {
                self.skips == 0
            }
            /// Returns the total number of times the prefilter has been used.
            #[inline]
            fn skips(&self) -> u32 {
                self.skips.saturating_sub(1)
            }
        }
        /// A combination of prefilter effectiveness state and the prefilter itself.
        pub(crate) struct Pre<'a> {
            /// State that tracks the effectiveness of a prefilter.
            prestate: &'a mut PrefilterState,
            /// The actual prefilter.
            prestrat: &'a Prefilter,
        }
        #[automatically_derived]
        impl<'a> ::core::fmt::Debug for Pre<'a> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Pre",
                    "prestate",
                    &self.prestate,
                    "prestrat",
                    &&self.prestrat,
                )
            }
        }
        impl<'a> Pre<'a> {
            /// Call this prefilter on the given haystack with the given needle.
            #[inline]
            pub(crate) fn find(&mut self, haystack: &[u8]) -> Option<usize> {
                let result = self.prestrat.find(haystack);
                self.prestate.update(result.unwrap_or(haystack.len()));
                result
            }
            /// Return true if and only if this prefilter should be used.
            #[inline]
            pub(crate) fn is_effective(&mut self) -> bool {
                self.prestate.is_effective()
            }
        }
        /// Returns true if the needle has the right characteristics for a vector
        /// algorithm to handle the entirety of substring search.
        ///
        /// Vector algorithms can be used for prefilters for other substring search
        /// algorithms (like Two-Way), but they can also be used for substring search
        /// on their own. When used for substring search, vector algorithms will
        /// quickly identify candidate match positions (just like in the prefilter
        /// case), but instead of returning the candidate position they will try to
        /// confirm the match themselves. Confirmation happens via `memcmp`. This
        /// works well for short needles, but can break down when many false candidate
        /// positions are generated for large needles. Thus, we only permit vector
        /// algorithms to own substring search when the needle is of a certain length.
        #[inline]
        fn do_packed_search(needle: &[u8]) -> bool {
            /// The minimum length of a needle required for this algorithm. The minimum
            /// is 2 since a length of 1 should just use memchr and a length of 0 isn't
            /// a case handled by this searcher.
            const MIN_LEN: usize = 2;
            /// The maximum length of a needle required for this algorithm.
            ///
            /// In reality, there is no hard max here. The code below can handle any
            /// length needle. (Perhaps that suggests there are missing optimizations.)
            /// Instead, this is a heuristic and a bound guaranteeing our linear time
            /// complexity.
            ///
            /// It is a heuristic because when a candidate match is found, memcmp is
            /// run. For very large needles with lots of false positives, memcmp can
            /// make the code run quite slow.
            ///
            /// It is a bound because the worst case behavior with memcmp is
            /// multiplicative in the size of the needle and haystack, and we want
            /// to keep that additive. This bound ensures we still meet that bound
            /// theoretically, since it's just a constant. We aren't acting in bad
            /// faith here, memcmp on tiny needles is so fast that even in pathological
            /// cases (see pathological vector benchmarks), this is still just as fast
            /// or faster in practice.
            ///
            /// This specific number was chosen by tweaking a bit and running
            /// benchmarks. The rare-medium-needle, for example, gets about 5% faster
            /// by using this algorithm instead of a prefilter-accelerated Two-Way.
            /// There's also a theoretical desire to keep this number reasonably
            /// low, to mitigate the impact of pathological cases. I did try 64, and
            /// some benchmarks got a little better, and others (particularly the
            /// pathological ones), got a lot worse. So... 32 it is?
            const MAX_LEN: usize = 32;
            MIN_LEN <= needle.len() && needle.len() <= MAX_LEN
        }
    }
    /// Returns an iterator over all non-overlapping occurrences of a substring in
    /// a haystack.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem;
    ///
    /// let haystack = b"foo bar foo baz foo";
    /// let mut it = memmem::find_iter(haystack, b"foo");
    /// assert_eq!(Some(0), it.next());
    /// assert_eq!(Some(8), it.next());
    /// assert_eq!(Some(16), it.next());
    /// assert_eq!(None, it.next());
    /// ```
    #[inline]
    pub fn find_iter<'h, 'n, N: 'n + ?Sized + AsRef<[u8]>>(
        haystack: &'h [u8],
        needle: &'n N,
    ) -> FindIter<'h, 'n> {
        FindIter::new(haystack, Finder::new(needle))
    }
    /// Returns a reverse iterator over all non-overlapping occurrences of a
    /// substring in a haystack.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem;
    ///
    /// let haystack = b"foo bar foo baz foo";
    /// let mut it = memmem::rfind_iter(haystack, b"foo");
    /// assert_eq!(Some(16), it.next());
    /// assert_eq!(Some(8), it.next());
    /// assert_eq!(Some(0), it.next());
    /// assert_eq!(None, it.next());
    /// ```
    #[inline]
    pub fn rfind_iter<'h, 'n, N: 'n + ?Sized + AsRef<[u8]>>(
        haystack: &'h [u8],
        needle: &'n N,
    ) -> FindRevIter<'h, 'n> {
        FindRevIter::new(haystack, FinderRev::new(needle))
    }
    /// Returns the index of the first occurrence of the given needle.
    ///
    /// Note that if you're are searching for the same needle in many different
    /// small haystacks, it may be faster to initialize a [`Finder`] once,
    /// and reuse it for each search.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem;
    ///
    /// let haystack = b"foo bar baz";
    /// assert_eq!(Some(0), memmem::find(haystack, b"foo"));
    /// assert_eq!(Some(4), memmem::find(haystack, b"bar"));
    /// assert_eq!(None, memmem::find(haystack, b"quux"));
    /// ```
    #[inline]
    pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if haystack.len() < 64 {
            rabinkarp::Finder::new(needle).find(haystack, needle)
        } else {
            Finder::new(needle).find(haystack)
        }
    }
    /// Returns the index of the last occurrence of the given needle.
    ///
    /// Note that if you're are searching for the same needle in many different
    /// small haystacks, it may be faster to initialize a [`FinderRev`] once,
    /// and reuse it for each search.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem;
    ///
    /// let haystack = b"foo bar baz";
    /// assert_eq!(Some(0), memmem::rfind(haystack, b"foo"));
    /// assert_eq!(Some(4), memmem::rfind(haystack, b"bar"));
    /// assert_eq!(Some(8), memmem::rfind(haystack, b"ba"));
    /// assert_eq!(None, memmem::rfind(haystack, b"quux"));
    /// ```
    #[inline]
    pub fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if haystack.len() < 64 {
            rabinkarp::FinderRev::new(needle).rfind(haystack, needle)
        } else {
            FinderRev::new(needle).rfind(haystack)
        }
    }
    /// An iterator over non-overlapping substring matches.
    ///
    /// Matches are reported by the byte offset at which they begin.
    ///
    /// `'h` is the lifetime of the haystack while `'n` is the lifetime of the
    /// needle.
    pub struct FindIter<'h, 'n> {
        haystack: &'h [u8],
        prestate: PrefilterState,
        finder: Finder<'n>,
        pos: usize,
    }
    #[automatically_derived]
    impl<'h, 'n> ::core::fmt::Debug for FindIter<'h, 'n> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "FindIter",
                "haystack",
                &self.haystack,
                "prestate",
                &self.prestate,
                "finder",
                &self.finder,
                "pos",
                &&self.pos,
            )
        }
    }
    #[automatically_derived]
    impl<'h, 'n> ::core::clone::Clone for FindIter<'h, 'n> {
        #[inline]
        fn clone(&self) -> FindIter<'h, 'n> {
            FindIter {
                haystack: ::core::clone::Clone::clone(&self.haystack),
                prestate: ::core::clone::Clone::clone(&self.prestate),
                finder: ::core::clone::Clone::clone(&self.finder),
                pos: ::core::clone::Clone::clone(&self.pos),
            }
        }
    }
    impl<'h, 'n> FindIter<'h, 'n> {
        #[inline(always)]
        pub(crate) fn new(haystack: &'h [u8], finder: Finder<'n>) -> FindIter<'h, 'n> {
            let prestate = PrefilterState::new();
            FindIter {
                haystack,
                prestate,
                finder,
                pos: 0,
            }
        }
        /// Convert this iterator into its owned variant, such that it no longer
        /// borrows the finder and needle.
        ///
        /// If this is already an owned iterator, then this is a no-op. Otherwise,
        /// this copies the needle.
        ///
        /// This is only available when the `alloc` feature is enabled.
        #[inline]
        pub fn into_owned(self) -> FindIter<'h, 'static> {
            FindIter {
                haystack: self.haystack,
                prestate: self.prestate,
                finder: self.finder.into_owned(),
                pos: self.pos,
            }
        }
    }
    impl<'h, 'n> Iterator for FindIter<'h, 'n> {
        type Item = usize;
        fn next(&mut self) -> Option<usize> {
            let needle = self.finder.needle();
            let haystack = self.haystack.get(self.pos..)?;
            let idx = self.finder.searcher.find(&mut self.prestate, haystack, needle)?;
            let pos = self.pos + idx;
            self.pos = pos + needle.len().max(1);
            Some(pos)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            match self.haystack.len().checked_sub(self.pos) {
                None => (0, Some(0)),
                Some(haystack_len) => {
                    match self.finder.needle().len() {
                        0 => {
                            (haystack_len.saturating_add(1), haystack_len.checked_add(1))
                        }
                        needle_len => (0, Some(haystack_len / needle_len)),
                    }
                }
            }
        }
    }
    /// An iterator over non-overlapping substring matches in reverse.
    ///
    /// Matches are reported by the byte offset at which they begin.
    ///
    /// `'h` is the lifetime of the haystack while `'n` is the lifetime of the
    /// needle.
    pub struct FindRevIter<'h, 'n> {
        haystack: &'h [u8],
        finder: FinderRev<'n>,
        /// When searching with an empty needle, this gets set to `None` after
        /// we've yielded the last element at `0`.
        pos: Option<usize>,
    }
    #[automatically_derived]
    impl<'h, 'n> ::core::clone::Clone for FindRevIter<'h, 'n> {
        #[inline]
        fn clone(&self) -> FindRevIter<'h, 'n> {
            FindRevIter {
                haystack: ::core::clone::Clone::clone(&self.haystack),
                finder: ::core::clone::Clone::clone(&self.finder),
                pos: ::core::clone::Clone::clone(&self.pos),
            }
        }
    }
    #[automatically_derived]
    impl<'h, 'n> ::core::fmt::Debug for FindRevIter<'h, 'n> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "FindRevIter",
                "haystack",
                &self.haystack,
                "finder",
                &self.finder,
                "pos",
                &&self.pos,
            )
        }
    }
    impl<'h, 'n> FindRevIter<'h, 'n> {
        #[inline(always)]
        pub(crate) fn new(
            haystack: &'h [u8],
            finder: FinderRev<'n>,
        ) -> FindRevIter<'h, 'n> {
            let pos = Some(haystack.len());
            FindRevIter {
                haystack,
                finder,
                pos,
            }
        }
        /// Convert this iterator into its owned variant, such that it no longer
        /// borrows the finder and needle.
        ///
        /// If this is already an owned iterator, then this is a no-op. Otherwise,
        /// this copies the needle.
        ///
        /// This is only available when the `std` feature is enabled.
        #[inline]
        pub fn into_owned(self) -> FindRevIter<'h, 'static> {
            FindRevIter {
                haystack: self.haystack,
                finder: self.finder.into_owned(),
                pos: self.pos,
            }
        }
    }
    impl<'h, 'n> Iterator for FindRevIter<'h, 'n> {
        type Item = usize;
        fn next(&mut self) -> Option<usize> {
            let pos = match self.pos {
                None => return None,
                Some(pos) => pos,
            };
            let result = self.finder.rfind(&self.haystack[..pos]);
            match result {
                None => None,
                Some(i) => {
                    if pos == i {
                        self.pos = pos.checked_sub(1);
                    } else {
                        self.pos = Some(i);
                    }
                    Some(i)
                }
            }
        }
    }
    /// A single substring searcher fixed to a particular needle.
    ///
    /// The purpose of this type is to permit callers to construct a substring
    /// searcher that can be used to search haystacks without the overhead of
    /// constructing the searcher in the first place. This is a somewhat niche
    /// concern when it's necessary to re-use the same needle to search multiple
    /// different haystacks with as little overhead as possible. In general, using
    /// [`find`] is good enough, but `Finder` is useful when you can meaningfully
    /// observe searcher construction time in a profile.
    ///
    /// When the `std` feature is enabled, then this type has an `into_owned`
    /// version which permits building a `Finder` that is not connected to
    /// the lifetime of its needle.
    pub struct Finder<'n> {
        needle: CowBytes<'n>,
        searcher: Searcher,
    }
    #[automatically_derived]
    impl<'n> ::core::clone::Clone for Finder<'n> {
        #[inline]
        fn clone(&self) -> Finder<'n> {
            Finder {
                needle: ::core::clone::Clone::clone(&self.needle),
                searcher: ::core::clone::Clone::clone(&self.searcher),
            }
        }
    }
    #[automatically_derived]
    impl<'n> ::core::fmt::Debug for Finder<'n> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Finder",
                "needle",
                &self.needle,
                "searcher",
                &&self.searcher,
            )
        }
    }
    impl<'n> Finder<'n> {
        /// Create a new finder for the given needle.
        #[inline]
        pub fn new<B: ?Sized + AsRef<[u8]>>(needle: &'n B) -> Finder<'n> {
            FinderBuilder::new().build_forward(needle)
        }
        /// Returns the index of the first occurrence of this needle in the given
        /// haystack.
        ///
        /// # Complexity
        ///
        /// This routine is guaranteed to have worst case linear time complexity
        /// with respect to both the needle and the haystack. That is, this runs
        /// in `O(needle.len() + haystack.len())` time.
        ///
        /// This routine is also guaranteed to have worst case constant space
        /// complexity.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use memchr::memmem::Finder;
        ///
        /// let haystack = b"foo bar baz";
        /// assert_eq!(Some(0), Finder::new("foo").find(haystack));
        /// assert_eq!(Some(4), Finder::new("bar").find(haystack));
        /// assert_eq!(None, Finder::new("quux").find(haystack));
        /// ```
        #[inline]
        pub fn find(&self, haystack: &[u8]) -> Option<usize> {
            let mut prestate = PrefilterState::new();
            let needle = self.needle.as_slice();
            self.searcher.find(&mut prestate, haystack, needle)
        }
        /// Returns an iterator over all occurrences of a substring in a haystack.
        ///
        /// # Complexity
        ///
        /// This routine is guaranteed to have worst case linear time complexity
        /// with respect to both the needle and the haystack. That is, this runs
        /// in `O(needle.len() + haystack.len())` time.
        ///
        /// This routine is also guaranteed to have worst case constant space
        /// complexity.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use memchr::memmem::Finder;
        ///
        /// let haystack = b"foo bar foo baz foo";
        /// let finder = Finder::new(b"foo");
        /// let mut it = finder.find_iter(haystack);
        /// assert_eq!(Some(0), it.next());
        /// assert_eq!(Some(8), it.next());
        /// assert_eq!(Some(16), it.next());
        /// assert_eq!(None, it.next());
        /// ```
        #[inline]
        pub fn find_iter<'a, 'h>(&'a self, haystack: &'h [u8]) -> FindIter<'h, 'a> {
            FindIter::new(haystack, self.as_ref())
        }
        /// Convert this finder into its owned variant, such that it no longer
        /// borrows the needle.
        ///
        /// If this is already an owned finder, then this is a no-op. Otherwise,
        /// this copies the needle.
        ///
        /// This is only available when the `alloc` feature is enabled.
        #[inline]
        pub fn into_owned(self) -> Finder<'static> {
            Finder {
                needle: self.needle.into_owned(),
                searcher: self.searcher.clone(),
            }
        }
        /// Convert this finder into its borrowed variant.
        ///
        /// This is primarily useful if your finder is owned and you'd like to
        /// store its borrowed variant in some intermediate data structure.
        ///
        /// Note that the lifetime parameter of the returned finder is tied to the
        /// lifetime of `self`, and may be shorter than the `'n` lifetime of the
        /// needle itself. Namely, a finder's needle can be either borrowed or
        /// owned, so the lifetime of the needle returned must necessarily be the
        /// shorter of the two.
        #[inline]
        pub fn as_ref(&self) -> Finder<'_> {
            Finder {
                needle: CowBytes::new(self.needle()),
                searcher: self.searcher.clone(),
            }
        }
        /// Returns the needle that this finder searches for.
        ///
        /// Note that the lifetime of the needle returned is tied to the lifetime
        /// of the finder, and may be shorter than the `'n` lifetime. Namely, a
        /// finder's needle can be either borrowed or owned, so the lifetime of the
        /// needle returned must necessarily be the shorter of the two.
        #[inline]
        pub fn needle(&self) -> &[u8] {
            self.needle.as_slice()
        }
    }
    /// A single substring reverse searcher fixed to a particular needle.
    ///
    /// The purpose of this type is to permit callers to construct a substring
    /// searcher that can be used to search haystacks without the overhead of
    /// constructing the searcher in the first place. This is a somewhat niche
    /// concern when it's necessary to re-use the same needle to search multiple
    /// different haystacks with as little overhead as possible. In general,
    /// using [`rfind`] is good enough, but `FinderRev` is useful when you can
    /// meaningfully observe searcher construction time in a profile.
    ///
    /// When the `std` feature is enabled, then this type has an `into_owned`
    /// version which permits building a `FinderRev` that is not connected to
    /// the lifetime of its needle.
    pub struct FinderRev<'n> {
        needle: CowBytes<'n>,
        searcher: SearcherRev,
    }
    #[automatically_derived]
    impl<'n> ::core::clone::Clone for FinderRev<'n> {
        #[inline]
        fn clone(&self) -> FinderRev<'n> {
            FinderRev {
                needle: ::core::clone::Clone::clone(&self.needle),
                searcher: ::core::clone::Clone::clone(&self.searcher),
            }
        }
    }
    #[automatically_derived]
    impl<'n> ::core::fmt::Debug for FinderRev<'n> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "FinderRev",
                "needle",
                &self.needle,
                "searcher",
                &&self.searcher,
            )
        }
    }
    impl<'n> FinderRev<'n> {
        /// Create a new reverse finder for the given needle.
        #[inline]
        pub fn new<B: ?Sized + AsRef<[u8]>>(needle: &'n B) -> FinderRev<'n> {
            FinderBuilder::new().build_reverse(needle)
        }
        /// Returns the index of the last occurrence of this needle in the given
        /// haystack.
        ///
        /// The haystack may be any type that can be cheaply converted into a
        /// `&[u8]`. This includes, but is not limited to, `&str` and `&[u8]`.
        ///
        /// # Complexity
        ///
        /// This routine is guaranteed to have worst case linear time complexity
        /// with respect to both the needle and the haystack. That is, this runs
        /// in `O(needle.len() + haystack.len())` time.
        ///
        /// This routine is also guaranteed to have worst case constant space
        /// complexity.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use memchr::memmem::FinderRev;
        ///
        /// let haystack = b"foo bar baz";
        /// assert_eq!(Some(0), FinderRev::new("foo").rfind(haystack));
        /// assert_eq!(Some(4), FinderRev::new("bar").rfind(haystack));
        /// assert_eq!(None, FinderRev::new("quux").rfind(haystack));
        /// ```
        pub fn rfind<B: AsRef<[u8]>>(&self, haystack: B) -> Option<usize> {
            self.searcher.rfind(haystack.as_ref(), self.needle.as_slice())
        }
        /// Returns a reverse iterator over all occurrences of a substring in a
        /// haystack.
        ///
        /// # Complexity
        ///
        /// This routine is guaranteed to have worst case linear time complexity
        /// with respect to both the needle and the haystack. That is, this runs
        /// in `O(needle.len() + haystack.len())` time.
        ///
        /// This routine is also guaranteed to have worst case constant space
        /// complexity.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use memchr::memmem::FinderRev;
        ///
        /// let haystack = b"foo bar foo baz foo";
        /// let finder = FinderRev::new(b"foo");
        /// let mut it = finder.rfind_iter(haystack);
        /// assert_eq!(Some(16), it.next());
        /// assert_eq!(Some(8), it.next());
        /// assert_eq!(Some(0), it.next());
        /// assert_eq!(None, it.next());
        /// ```
        #[inline]
        pub fn rfind_iter<'a, 'h>(&'a self, haystack: &'h [u8]) -> FindRevIter<'h, 'a> {
            FindRevIter::new(haystack, self.as_ref())
        }
        /// Convert this finder into its owned variant, such that it no longer
        /// borrows the needle.
        ///
        /// If this is already an owned finder, then this is a no-op. Otherwise,
        /// this copies the needle.
        ///
        /// This is only available when the `std` feature is enabled.
        #[inline]
        pub fn into_owned(self) -> FinderRev<'static> {
            FinderRev {
                needle: self.needle.into_owned(),
                searcher: self.searcher.clone(),
            }
        }
        /// Convert this finder into its borrowed variant.
        ///
        /// This is primarily useful if your finder is owned and you'd like to
        /// store its borrowed variant in some intermediate data structure.
        ///
        /// Note that the lifetime parameter of the returned finder is tied to the
        /// lifetime of `self`, and may be shorter than the `'n` lifetime of the
        /// needle itself. Namely, a finder's needle can be either borrowed or
        /// owned, so the lifetime of the needle returned must necessarily be the
        /// shorter of the two.
        #[inline]
        pub fn as_ref(&self) -> FinderRev<'_> {
            FinderRev {
                needle: CowBytes::new(self.needle()),
                searcher: self.searcher.clone(),
            }
        }
        /// Returns the needle that this finder searches for.
        ///
        /// Note that the lifetime of the needle returned is tied to the lifetime
        /// of the finder, and may be shorter than the `'n` lifetime. Namely, a
        /// finder's needle can be either borrowed or owned, so the lifetime of the
        /// needle returned must necessarily be the shorter of the two.
        #[inline]
        pub fn needle(&self) -> &[u8] {
            self.needle.as_slice()
        }
    }
    /// A builder for constructing non-default forward or reverse memmem finders.
    ///
    /// A builder is primarily useful for configuring a substring searcher.
    /// Currently, the only configuration exposed is the ability to disable
    /// heuristic prefilters used to speed up certain searches.
    pub struct FinderBuilder {
        prefilter: Prefilter,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for FinderBuilder {
        #[inline]
        fn clone(&self) -> FinderBuilder {
            FinderBuilder {
                prefilter: ::core::clone::Clone::clone(&self.prefilter),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for FinderBuilder {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "FinderBuilder",
                "prefilter",
                &&self.prefilter,
            )
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for FinderBuilder {
        #[inline]
        fn default() -> FinderBuilder {
            FinderBuilder {
                prefilter: ::core::default::Default::default(),
            }
        }
    }
    impl FinderBuilder {
        /// Create a new finder builder with default settings.
        pub fn new() -> FinderBuilder {
            FinderBuilder::default()
        }
        /// Build a forward finder using the given needle from the current
        /// settings.
        pub fn build_forward<'n, B: ?Sized + AsRef<[u8]>>(
            &self,
            needle: &'n B,
        ) -> Finder<'n> {
            self.build_forward_with_ranker(DefaultFrequencyRank, needle)
        }
        /// Build an owned forward finder using the given needle from the current
        /// settings.
        pub fn build_forward_owned<B: Into<alloc::boxed::Box<[u8]>>>(
            &self,
            needle: B,
        ) -> Finder<'static> {
            self.build_forward_with_ranker_owned(DefaultFrequencyRank, needle)
        }
        /// Build a forward finder using the given needle and a custom heuristic
        /// for determining the frequency of a given byte in the dataset. See
        /// [`HeuristicFrequencyRank`] for more details.
        pub fn build_forward_with_ranker<
            'n,
            R: HeuristicFrequencyRank,
            B: ?Sized + AsRef<[u8]>,
        >(&self, ranker: R, needle: &'n B) -> Finder<'n> {
            let needle = needle.as_ref();
            Finder {
                needle: CowBytes::new(needle),
                searcher: Searcher::new(self.prefilter, ranker, needle),
            }
        }
        /// Build an owned forward finder using the given needle and a custom
        /// heuristic for determining the frequency of a given byte in the dataset.
        /// See [`HeuristicFrequencyRank`] for more details.
        pub fn build_forward_with_ranker_owned<
            R: HeuristicFrequencyRank,
            B: Into<alloc::boxed::Box<[u8]>>,
        >(&self, ranker: R, needle: B) -> Finder<'static> {
            let needle = needle.into();
            let searcher = Searcher::new(self.prefilter, ranker, &needle);
            Finder {
                needle: CowBytes::new_owned(needle),
                searcher,
            }
        }
        /// Build a reverse finder using the given needle from the current
        /// settings.
        pub fn build_reverse<'n, B: ?Sized + AsRef<[u8]>>(
            &self,
            needle: &'n B,
        ) -> FinderRev<'n> {
            let needle = needle.as_ref();
            FinderRev {
                needle: CowBytes::new(needle),
                searcher: SearcherRev::new(needle),
            }
        }
        /// Build an owned reverse finder using the given needle from the current
        /// settings.
        pub fn build_reverse_owned<B: Into<alloc::boxed::Box<[u8]>>>(
            &self,
            needle: B,
        ) -> FinderRev<'static> {
            let needle = needle.into();
            let searcher = SearcherRev::new(&needle);
            FinderRev {
                needle: CowBytes::new_owned(needle),
                searcher,
            }
        }
        /// Configure the prefilter setting for the finder.
        ///
        /// See the documentation for [`Prefilter`] for more discussion on why
        /// you might want to configure this.
        pub fn prefilter(&mut self, prefilter: Prefilter) -> &mut FinderBuilder {
            self.prefilter = prefilter;
            self
        }
    }
}
mod vector {
    /// A trait for describing vector operations used by vectorized searchers.
    ///
    /// The trait is highly constrained to low level vector operations needed.
    /// In general, it was invented mostly to be generic over x86's __m128i and
    /// __m256i types. At time of writing, it also supports wasm and aarch64
    /// 128-bit vector types as well.
    ///
    /// # Safety
    ///
    /// All methods are not safe since they are intended to be implemented using
    /// vendor intrinsics, which are also not safe. Callers must ensure that the
    /// appropriate target features are enabled in the calling function, and that
    /// the current CPU supports them. All implementations should avoid marking the
    /// routines with #[target_feature] and instead mark them as #[inline(always)]
    /// to ensure they get appropriately inlined. (inline(always) cannot be used
    /// with target_feature.)
    pub(crate) trait Vector: Copy + core::fmt::Debug {
        /// The number of bytes in the vector. That is, this is the size of the
        /// vector in memory.
        const BYTES: usize;
        /// The bits that must be zero in order for a `*const u8` pointer to be
        /// correctly aligned to read vector values.
        const ALIGN: usize;
        /// The type of the value returned by `Vector::movemask`.
        ///
        /// This supports abstracting over the specific representation used in
        /// order to accommodate different representations in different ISAs.
        type Mask: MoveMask;
        /// Create a vector with 8-bit lanes with the given byte repeated into each
        /// lane.
        unsafe fn splat(byte: u8) -> Self;
        /// Read a vector-size number of bytes from the given pointer. The pointer
        /// must be aligned to the size of the vector.
        ///
        /// # Safety
        ///
        /// Callers must guarantee that at least `BYTES` bytes are readable from
        /// `data` and that `data` is aligned to a `BYTES` boundary.
        unsafe fn load_aligned(data: *const u8) -> Self;
        /// Read a vector-size number of bytes from the given pointer. The pointer
        /// does not need to be aligned.
        ///
        /// # Safety
        ///
        /// Callers must guarantee that at least `BYTES` bytes are readable from
        /// `data`.
        unsafe fn load_unaligned(data: *const u8) -> Self;
        /// _mm_movemask_epi8 or _mm256_movemask_epi8
        unsafe fn movemask(self) -> Self::Mask;
        /// _mm_cmpeq_epi8 or _mm256_cmpeq_epi8
        unsafe fn cmpeq(self, vector2: Self) -> Self;
        /// _mm_and_si128 or _mm256_and_si256
        unsafe fn and(self, vector2: Self) -> Self;
        /// _mm_or or _mm256_or_si256
        unsafe fn or(self, vector2: Self) -> Self;
        /// Returns true if and only if `Self::movemask` would return a mask that
        /// contains at least one non-zero bit.
        unsafe fn movemask_will_have_non_zero(self) -> bool {
            self.movemask().has_non_zero()
        }
    }
    /// A trait that abstracts over a vector-to-scalar operation called
    /// "move mask."
    ///
    /// On x86-64, this is `_mm_movemask_epi8` for SSE2 and `_mm256_movemask_epi8`
    /// for AVX2. It takes a vector of `u8` lanes and returns a scalar where the
    /// `i`th bit is set if and only if the most significant bit in the `i`th lane
    /// of the vector is set. The simd128 ISA for wasm32 also supports this
    /// exact same operation natively.
    ///
    /// ... But aarch64 doesn't. So we have to fake it with more instructions and
    /// a slightly different representation. We could do extra work to unify the
    /// representations, but then would require additional costs in the hot path
    /// for `memchr` and `packedpair`. So instead, we abstraction over the specific
    /// representation with this trait and define the operations we actually need.
    pub(crate) trait MoveMask: Copy + core::fmt::Debug {
        /// Return a mask that is all zeros except for the least significant `n`
        /// lanes in a corresponding vector.
        fn all_zeros_except_least_significant(n: usize) -> Self;
        /// Returns true if and only if this mask has a a non-zero bit anywhere.
        fn has_non_zero(self) -> bool;
        /// Returns the number of bits set to 1 in this mask.
        fn count_ones(self) -> usize;
        /// Does a bitwise `and` operation between `self` and `other`.
        fn and(self, other: Self) -> Self;
        /// Does a bitwise `or` operation between `self` and `other`.
        fn or(self, other: Self) -> Self;
        /// Returns a mask that is equivalent to `self` but with the least
        /// significant 1-bit set to 0.
        fn clear_least_significant_bit(self) -> Self;
        /// Returns the offset of the first non-zero lane this mask represents.
        fn first_offset(self) -> usize;
        /// Returns the offset of the last non-zero lane this mask represents.
        fn last_offset(self) -> usize;
    }
    /// This is a "sensible" movemask implementation where each bit represents
    /// whether the most significant bit is set in each corresponding lane of a
    /// vector. This is used on x86-64 and wasm, but such a mask is more expensive
    /// to get on aarch64 so we use something a little different.
    ///
    /// We call this "sensible" because this is what we get using native sse/avx
    /// movemask instructions. But neon has no such native equivalent.
    pub(crate) struct SensibleMoveMask(u32);
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for SensibleMoveMask {}
    #[automatically_derived]
    impl ::core::clone::Clone for SensibleMoveMask {
        #[inline]
        fn clone(&self) -> SensibleMoveMask {
            let _: ::core::clone::AssertParamIsClone<u32>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for SensibleMoveMask {}
    #[automatically_derived]
    impl ::core::fmt::Debug for SensibleMoveMask {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "SensibleMoveMask",
                &&self.0,
            )
        }
    }
    impl SensibleMoveMask {
        /// Get the mask in a form suitable for computing offsets.
        ///
        /// Basically, this normalizes to little endian. On big endian, this swaps
        /// the bytes.
        #[inline(always)]
        fn get_for_offset(self) -> u32 {
            { self.0 }
        }
    }
    impl MoveMask for SensibleMoveMask {
        #[inline(always)]
        fn all_zeros_except_least_significant(n: usize) -> SensibleMoveMask {
            if true {
                if !(n < 32) {
                    ::core::panicking::panic("assertion failed: n < 32")
                }
            }
            SensibleMoveMask(!((1 << n) - 1))
        }
        #[inline(always)]
        fn has_non_zero(self) -> bool {
            self.0 != 0
        }
        #[inline(always)]
        fn count_ones(self) -> usize {
            self.0.count_ones() as usize
        }
        #[inline(always)]
        fn and(self, other: SensibleMoveMask) -> SensibleMoveMask {
            SensibleMoveMask(self.0 & other.0)
        }
        #[inline(always)]
        fn or(self, other: SensibleMoveMask) -> SensibleMoveMask {
            SensibleMoveMask(self.0 | other.0)
        }
        #[inline(always)]
        fn clear_least_significant_bit(self) -> SensibleMoveMask {
            SensibleMoveMask(self.0 & (self.0 - 1))
        }
        #[inline(always)]
        fn first_offset(self) -> usize {
            self.get_for_offset().trailing_zeros() as usize
        }
        #[inline(always)]
        fn last_offset(self) -> usize {
            32 - self.get_for_offset().leading_zeros() as usize - 1
        }
    }
    mod x86sse2 {
        use core::arch::x86_64::*;
        use super::{SensibleMoveMask, Vector};
        impl Vector for __m128i {
            const BYTES: usize = 16;
            const ALIGN: usize = Self::BYTES - 1;
            type Mask = SensibleMoveMask;
            #[inline(always)]
            unsafe fn splat(byte: u8) -> __m128i {
                _mm_set1_epi8(byte as i8)
            }
            #[inline(always)]
            unsafe fn load_aligned(data: *const u8) -> __m128i {
                _mm_load_si128(data as *const __m128i)
            }
            #[inline(always)]
            unsafe fn load_unaligned(data: *const u8) -> __m128i {
                _mm_loadu_si128(data as *const __m128i)
            }
            #[inline(always)]
            unsafe fn movemask(self) -> SensibleMoveMask {
                SensibleMoveMask(_mm_movemask_epi8(self) as u32)
            }
            #[inline(always)]
            unsafe fn cmpeq(self, vector2: Self) -> __m128i {
                _mm_cmpeq_epi8(self, vector2)
            }
            #[inline(always)]
            unsafe fn and(self, vector2: Self) -> __m128i {
                _mm_and_si128(self, vector2)
            }
            #[inline(always)]
            unsafe fn or(self, vector2: Self) -> __m128i {
                _mm_or_si128(self, vector2)
            }
        }
    }
    mod x86avx2 {
        use core::arch::x86_64::*;
        use super::{SensibleMoveMask, Vector};
        impl Vector for __m256i {
            const BYTES: usize = 32;
            const ALIGN: usize = Self::BYTES - 1;
            type Mask = SensibleMoveMask;
            #[inline(always)]
            unsafe fn splat(byte: u8) -> __m256i {
                _mm256_set1_epi8(byte as i8)
            }
            #[inline(always)]
            unsafe fn load_aligned(data: *const u8) -> __m256i {
                _mm256_load_si256(data as *const __m256i)
            }
            #[inline(always)]
            unsafe fn load_unaligned(data: *const u8) -> __m256i {
                _mm256_loadu_si256(data as *const __m256i)
            }
            #[inline(always)]
            unsafe fn movemask(self) -> SensibleMoveMask {
                SensibleMoveMask(_mm256_movemask_epi8(self) as u32)
            }
            #[inline(always)]
            unsafe fn cmpeq(self, vector2: Self) -> __m256i {
                _mm256_cmpeq_epi8(self, vector2)
            }
            #[inline(always)]
            unsafe fn and(self, vector2: Self) -> __m256i {
                _mm256_and_si256(self, vector2)
            }
            #[inline(always)]
            unsafe fn or(self, vector2: Self) -> __m256i {
                _mm256_or_si256(self, vector2)
            }
        }
    }
}
