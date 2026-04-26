#![feature(prelude_import)]
#![no_std]
//! This crate provides several functions for handling `&mut T` including `take()`.
//!
//! `take()` allows for taking `T` out of a `&mut T`, doing anything with it including consuming it, and producing another `T` to put back in the `&mut T`.
//!
//! During `take()`, if a panic occurs, the entire process will be aborted, as there's no valid `T` to put back into the `&mut T`.
//! Use `take_or_recover()` to replace the `&mut T` with a recovery value before continuing the panic.
//!
//! Contrast with `std::mem::replace()`, which allows for putting a different `T` into a `&mut T`, but requiring the new `T` to be available before being able to consume the old `T`.
extern crate std;
#[prelude_import]
use ::std::prelude::rust_2015::*;
use std::panic;
pub mod scoped {
    //! This module provides a scoped API, allowing for taking an arbitrary number of `&mut T` into `T` within one closure.
    //! The references are all required to outlive the closure.
    //!
    //! # Example
    //! ```
    //! use take_mut::scoped;
    //! struct Foo;
    //! let mut foo = Foo; // Must outlive scope
    //! scoped::scope(|scope| {
    //!     let (t, hole) = scope.take(&mut foo);
    //!     drop(t);
    //!     hole.fill(Foo); // If not called before the closure ends, causes an abort.
    //! });
    //! ```
    //!
    //! # Invalid Example (does not compile)
    //! ```ignore
    //! use take_mut::scoped;
    //! struct Foo;
    //! scoped::scope(|scope| {
    //!     let mut foo = Foo; // Invalid because foo must come from outside the scope.
    //!     let (t, hole) = scope.take(&mut foo);
    //!     drop(t);
    //!     hole.fill(Foo);
    //! });
    //! ```
    //!
    //! `Scope` also offers `take_or_recover`, which takes a function to call in the event the hole isn't filled.
    #![warn(missing_docs)]
    use std;
    use std::panic;
    use std::cell::Cell;
    use std::marker::PhantomData;
    /// Represents a scope within which, it is possible to take a `T` from a `&mut T` as long as the `&mut T` outlives the scope.
    pub struct Scope<'s> {
        active_holes: Cell<usize>,
        marker: PhantomData<Cell<&'s mut ()>>,
    }
    impl<'s> Scope<'s> {
        /// Takes a `(T, Hole<'c, 'm, T, F>)` from an `&'m mut T`.
        ///
        /// If the `Hole` is dropped without being filled, either due to panic or forgetting to fill, will run the `recovery` function to obtain a `T` to fill itself with.
        pub fn take_or_recover<'c, 'm: 's, T: 'm, F: FnOnce() -> T>(
            &'c self,
            mut_ref: &'m mut T,
            recovery: F,
        ) -> (T, Hole<'c, 'm, T, F>) {
            use std::ptr;
            let t: T;
            let hole: Hole<'c, 'm, T, F>;
            let num_of_holes = self.active_holes.get();
            if num_of_holes == std::usize::MAX {
                {
                    ::std::rt::begin_panic("Too many holes!");
                };
            }
            self.active_holes.set(num_of_holes + 1);
            unsafe {
                t = ptr::read(mut_ref as *mut T);
                hole = Hole {
                    active_holes: &self.active_holes,
                    hole: mut_ref as *mut T,
                    phantom: PhantomData,
                    recovery: Some(recovery),
                };
            };
            (t, hole)
        }
        /// Takes a `(T, Hole<'c, 'm, T, F>)` from an `&'m mut T`.
        pub fn take<'c, 'm: 's, T: 'm>(
            &'c self,
            mut_ref: &'m mut T,
        ) -> (T, Hole<'c, 'm, T, fn() -> T>) {
            #[allow(missing_docs)]
            fn panic<T>() -> T {
                {
                    ::std::rt::begin_panic("Failed to recover a Hole!");
                }
            }
            self.take_or_recover(mut_ref, panic)
        }
    }
    /// Main function to create a `Scope`.
    ///
    /// If the given closure ends without all Holes filled, will abort the program.
    pub fn scope<'s, F, R>(f: F) -> R
    where
        F: FnOnce(&Scope<'s>) -> R,
    {
        let this = Scope {
            active_holes: Cell::new(0),
            marker: PhantomData,
        };
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| { f(&this) }));
        if this.active_holes.get() != 0 {
            std::process::abort();
        }
        match result {
            Ok(r) => r,
            Err(p) => panic::resume_unwind(p),
        }
    }
    /// A `Hole<'c, 'm, T, F>` represents an unfilled `&'m mut T` which must be filled before the end of the `Scope` with lifetime `'c` and recovery closure `F`.
    ///
    /// An unfilled `Hole<'c, 'm, T, F> that is destructed will try to use `F` to fill the hole.
    ///
    /// If the scope ends without the `Hole` being filled, the program will `std::process::abort()`.
    #[must_use]
    pub struct Hole<'c, 'm, T: 'm, F: FnOnce() -> T> {
        active_holes: &'c Cell<usize>,
        hole: *mut T,
        phantom: PhantomData<&'m mut T>,
        recovery: Option<F>,
    }
    impl<'c, 'm, T: 'm, F: FnOnce() -> T> Hole<'c, 'm, T, F> {
        /// Fills the Hole.
        pub fn fill(self, t: T) {
            use std::ptr;
            use std::mem;
            unsafe {
                ptr::write(self.hole, t);
            }
            let num_holes = self.active_holes.get();
            self.active_holes.set(num_holes - 1);
            mem::forget(self);
        }
    }
    impl<'c, 'm, T: 'm, F: FnOnce() -> T> Drop for Hole<'c, 'm, T, F> {
        fn drop(&mut self) {
            use std::ptr;
            let t = (self.recovery.take().expect("No recovery function in Hole!"))();
            unsafe {
                ptr::write(self.hole, t);
            }
            let num_holes = self.active_holes.get();
            self.active_holes.set(num_holes - 1);
        }
    }
    extern crate test;
    #[rustc_test_marker = "scoped::scope_based_take"]
    #[doc(hidden)]
    pub const scope_based_take: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("scoped::scope_based_take"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/scoped.rs",
            start_line: 142usize,
            start_col: 4usize,
            end_line: 142usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(scope_based_take()),
        ),
    };
    fn scope_based_take() {
        struct Foo;
        #[automatically_derived]
        impl ::core::fmt::Debug for Foo {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "Foo")
            }
        }
        struct Bar {
            a: Foo,
            b: Foo,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Bar {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Bar",
                    "a",
                    &self.a,
                    "b",
                    &&self.b,
                )
            }
        }
        let mut bar = Bar { a: Foo, b: Foo };
        scope(|scope| {
            let (a, a_hole) = scope.take(&mut bar.a);
            let (b, b_hole) = scope.take(&mut bar.b);
            a_hole.fill(Foo);
            b_hole.fill(Foo);
        });
        {
            ::std::io::_print(format_args!("{0:?}\n", &bar));
        };
    }
    extern crate test;
    #[rustc_test_marker = "scoped::panic_on_recovered_panic"]
    #[doc(hidden)]
    pub const panic_on_recovered_panic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("scoped::panic_on_recovered_panic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/scoped.rs",
            start_line: 163usize,
            start_col: 4usize,
            end_line: 163usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(panic_on_recovered_panic()),
        ),
    };
    fn panic_on_recovered_panic() {
        use std::panic;
        struct Foo;
        let mut foo = Foo;
        let result = panic::catch_unwind(
            panic::AssertUnwindSafe(|| {
                scope(|scope| {
                    let (t, hole) = scope.take_or_recover(&mut foo, || Foo);
                    {
                        ::std::rt::begin_panic("Oops!");
                    };
                });
            }),
        );
        if !result.is_err() {
            ::core::panicking::panic("assertion failed: result.is_err()")
        }
    }
}
/// Allows use of a value pointed to by `&mut T` as though it was owned, as long as a `T` is made available afterwards.
///
/// The closure must return a valid T.
/// # Important
/// Will abort the program if the closure panics.
///
/// # Example
/// ```
/// struct Foo;
/// let mut foo = Foo;
/// take_mut::take(&mut foo, |foo| {
///     // Can now consume the Foo, and provide a new value later
///     drop(foo);
///     // Do more stuff
///     Foo // Return new Foo from closure, which goes back into the &mut Foo
/// });
/// ```
pub fn take<T, F>(mut_ref: &mut T, closure: F)
where
    F: FnOnce(T) -> T,
{
    use std::ptr;
    unsafe {
        let old_t = ptr::read(mut_ref);
        let new_t = panic::catch_unwind(panic::AssertUnwindSafe(|| closure(old_t)))
            .unwrap_or_else(|_| ::std::process::abort());
        ptr::write(mut_ref, new_t);
    }
}
extern crate test;
#[rustc_test_marker = "it_works"]
#[doc(hidden)]
pub const it_works: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("it_works"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 44usize,
        start_col: 4usize,
        end_line: 44usize,
        end_col: 12usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(it_works())),
};
fn it_works() {
    enum Foo {
        A,
        B,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Foo {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Foo {
        #[inline]
        fn eq(&self, other: &Foo) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Foo {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Foo {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Foo::A => "A",
                    Foo::B => "B",
                },
            )
        }
    }
    impl Drop for Foo {
        fn drop(&mut self) {
            match *self {
                Foo::A => {
                    ::std::io::_print(format_args!("Foo::A dropped\n"));
                }
                Foo::B => {
                    ::std::io::_print(format_args!("Foo::B dropped\n"));
                }
            }
        }
    }
    let mut foo = Foo::A;
    take(
        &mut foo,
        |f| {
            drop(f);
            Foo::B
        },
    );
    match (&&foo, &&Foo::B) {
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
/// Allows use of a value pointed to by `&mut T` as though it was owned, as long as a `T` is made available afterwards.
///
/// The closure must return a valid T.
/// # Important
/// Will replace `&mut T` with `recover` if the closure panics, then continues the panic.
///
/// # Example
/// ```
/// struct Foo;
/// let mut foo = Foo;
/// take_mut::take_or_recover(&mut foo, || Foo, |foo| {
///     // Can now consume the Foo, and provide a new value later
///     drop(foo);
///     // Do more stuff
///     Foo // Return new Foo from closure, which goes back into the &mut Foo
/// });
/// ```
pub fn take_or_recover<T, F, R>(mut_ref: &mut T, recover: R, closure: F)
where
    F: FnOnce(T) -> T,
    R: FnOnce() -> T,
{
    use std::ptr;
    unsafe {
        let old_t = ptr::read(mut_ref);
        let new_t = panic::catch_unwind(panic::AssertUnwindSafe(|| closure(old_t)));
        match new_t {
            Err(err) => {
                let r = panic::catch_unwind(panic::AssertUnwindSafe(|| recover()))
                    .unwrap_or_else(|_| ::std::process::abort());
                ptr::write(mut_ref, r);
                panic::resume_unwind(err);
            }
            Ok(new_t) => ptr::write(mut_ref, new_t),
        }
    }
}
extern crate test;
#[rustc_test_marker = "it_works_recover"]
#[doc(hidden)]
pub const it_works_recover: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("it_works_recover"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 103usize,
        start_col: 4usize,
        end_line: 103usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(it_works_recover()),
    ),
};
fn it_works_recover() {
    enum Foo {
        A,
        B,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Foo {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Foo {
        #[inline]
        fn eq(&self, other: &Foo) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Foo {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Foo {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Foo::A => "A",
                    Foo::B => "B",
                },
            )
        }
    }
    impl Drop for Foo {
        fn drop(&mut self) {
            match *self {
                Foo::A => {
                    ::std::io::_print(format_args!("Foo::A dropped\n"));
                }
                Foo::B => {
                    ::std::io::_print(format_args!("Foo::B dropped\n"));
                }
            }
        }
    }
    let mut foo = Foo::A;
    take_or_recover(
        &mut foo,
        || Foo::A,
        |f| {
            drop(f);
            Foo::B
        },
    );
    match (&&foo, &&Foo::B) {
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
extern crate test;
#[rustc_test_marker = "it_works_recover_panic"]
#[doc(hidden)]
pub const it_works_recover_panic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("it_works_recover_panic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 123usize,
        start_col: 4usize,
        end_line: 123usize,
        end_col: 26usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(it_works_recover_panic()),
    ),
};
fn it_works_recover_panic() {
    enum Foo {
        A,
        B,
        C,
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Foo {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Foo {
        #[inline]
        fn eq(&self, other: &Foo) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Foo {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Foo {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Foo::A => "A",
                    Foo::B => "B",
                    Foo::C => "C",
                },
            )
        }
    }
    impl Drop for Foo {
        fn drop(&mut self) {
            match *self {
                Foo::A => {
                    ::std::io::_print(format_args!("Foo::A dropped\n"));
                }
                Foo::B => {
                    ::std::io::_print(format_args!("Foo::B dropped\n"));
                }
                Foo::C => {
                    ::std::io::_print(format_args!("Foo::C dropped\n"));
                }
            }
        }
    }
    let mut foo = Foo::A;
    let res = panic::catch_unwind(
        panic::AssertUnwindSafe(|| {
            take_or_recover(
                &mut foo,
                || Foo::C,
                |f| {
                    drop(f);
                    {
                        ::std::rt::begin_panic("panic");
                    };
                    Foo::B
                },
            );
        }),
    );
    if !res.is_err() {
        ::core::panicking::panic("assertion failed: res.is_err()")
    }
    match (&&foo, &&Foo::C) {
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
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &it_works,
            &it_works_recover,
            &it_works_recover_panic,
            &panic_on_recovered_panic,
            &scope_based_take,
        ],
    )
}
