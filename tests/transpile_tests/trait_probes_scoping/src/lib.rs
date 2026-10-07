//! Differential oracle for the trait lowering (book §3.2, tier 2 carrier) — `scoping` family.
//!
//! Each test is a cell of one of the six 2026-10-04 carrier probes
//! (book §3.2.15, "Measurements added by the 2026-10-04 carrier probes"),
//! rewritten from a `main` with `println!` into asserts against the rustc
//! oracle values recorded there. The parity harness runs the same tests under
//! rustc and under the transpiled C++; a cell that the shipped lane gets
//! wrong fails here until the §3.2.16 phase-2 step that fixes it lands.
//! Cases that are rustc *errors* (E0034, E0117, E0283) cannot be tests and
//! are not here.

pub mod scoping_probe {
    // ===========================================================================
    // scoping — which trait a bare `x.foo()` resolves to is decided by the
    // traits in scope at the call site (module `use`), not by which impls exist.
    // Error cases (E0034) are not representable here.
    // ===========================================================================
    pub mod scoping {
        pub trait A {
            fn foo(&self) -> i32 {
                1000
            }
        }
        pub trait B {
            fn foo(&self) -> i32 {
                2000
            }
        }
        impl A for i32 {
            fn foo(&self) -> i32 {
                1
            }
        }
        impl B for i32 {
            fn foo(&self) -> i32 {
                2
            }
        }
        impl A for i64 {
            fn foo(&self) -> i32 {
                11
            }
        }
        impl A for u8 {}

        pub mod only_a {
            use super::A;
            pub fn t(x: i32) -> i32 {
                x.foo()
            }
            pub fn u(x: i64) -> i32 {
                x.foo()
            }
        }
        pub mod only_b {
            use super::B;
            pub fn t(x: i32) -> i32 {
                x.foo()
            }
        }
        pub mod both {
            use super::{A, B};
            pub fn u(x: i64) -> i32 {
                x.foo()
            }
            pub fn u8(x: u8) -> i32 {
                x.foo()
            }
        }
        pub mod bound {
            #[allow(unused_imports)]
            use super::{A, B};
            pub fn g<X: A>(x: &X) -> i32 {
                x.foo()
            }
        }
    }

    #[test]
    fn scoping_same_name_resolved_by_use_scope() {
        assert_eq!(scoping::only_a::t(5), 1);
        assert_eq!(scoping::only_a::u(5), 11);
        assert_eq!(scoping::only_b::t(5), 2);
        assert_eq!(scoping::both::u(5), 11);
        assert_eq!(scoping::both::u8(5), 1000);
    }

    #[test]
    fn scoping_type_parameter_receiver_resolves_from_bound() {
        assert_eq!(scoping::bound::g(&5i32), 1);
        assert_eq!(scoping::bound::g(&5i64), 11);
        assert_eq!(scoping::bound::g(&5u8), 1000);
    }

    trait GreetD {
        fn hello(&self) -> i32;
        fn describe(&self) -> i32 {
            self.hello() * 1000
        }
    }
    struct Dog;
    impl Dog {
        #[allow(dead_code)]
        fn hello(&self) -> i32 {
            2002
        }
    }
    impl GreetD for Dog {
        fn hello(&self) -> i32 {
            2
        }
    }

    #[test]
    fn scoping_default_body_sees_trait_method_not_inherent() {
        assert_eq!(GreetD::describe(&Dog), 2000);
        assert_eq!(Dog.hello(), 2002);
    }
}
