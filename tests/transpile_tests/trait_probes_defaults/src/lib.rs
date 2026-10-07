//! Differential oracle for the trait lowering (book §3.2, tier 2 carrier) — `defaults` family.
//!
//! Each test is a cell of one of the six 2026-10-04 carrier probes
//! (book §3.2.15, "Measurements added by the 2026-10-04 carrier probes"),
//! rewritten from a `main` with `println!` into asserts against the rustc
//! oracle values recorded there. The parity harness runs the same tests under
//! rustc and under the transpiled C++; a cell that the shipped lane gets
//! wrong fails here until the §3.2.16 phase-2 step that fixes it lands.
//! Cases that are rustc *errors* (E0034, E0117, E0283) cannot be tests and
//! are not here.

pub mod defaults {
    // ===========================================================================
    // defaults — inherent shadow inside a default body, non-template override of
    // a default, a primitive impl, generic default + a default calling it, dyn.
    // ===========================================================================
    trait Greet {
        fn hello(&self) -> i32;
        fn describe(&self) -> i32 {
            self.hello() * 2
        }
        fn each<F: Fn(i32) -> i32>(&self, f: F) -> i32
        where
            Self: Sized,
        {
            f(self.hello())
        }
        fn via(&self) -> i32
        where
            Self: Sized,
        {
            self.each(|x| x + 1)
        }
    }
    struct Foo;
    impl Foo {
        fn hello(&self) -> i32 {
            1001
        }
    }
    impl Greet for Foo {
        fn hello(&self) -> i32 {
            1
        }
    }
    struct Bar;
    impl Greet for Bar {
        fn hello(&self) -> i32 {
            5
        }
        fn describe(&self) -> i32 {
            777
        }
    }
    impl Greet for i32 {
        fn hello(&self) -> i32 {
            *self
        }
    }

    #[test]
    fn defaults_inherent_shadow_is_not_seen_by_default_body() {
        let foo = Foo;
        assert_eq!(foo.hello(), 1001);
        assert_eq!(<Foo as Greet>::hello(&foo), 1);
        assert_eq!(foo.describe(), 2);
        assert_eq!(foo.each(|x| x + 10), 11);
        assert_eq!(foo.via(), 2);
    }

    #[test]
    fn defaults_override_primitive_and_dyn() {
        let bar = Bar;
        assert_eq!(format!("describe={} via={}", bar.describe(), bar.via()), "describe=777 via=6");
        let n: i32 = 21;
        assert_eq!(format!("describe={} via={}", n.describe(), n.via()), "describe=42 via=22");
        let foo = Foo;
        let d: &dyn Greet = &foo;
        let e: &dyn Greet = &bar;
        let g: &dyn Greet = &n;
        assert_eq!(format!("{}/{}/{}", d.describe(), e.describe(), g.describe()), "2/777/42");
    }
}
