//! Differential oracle for the trait lowering (book §3.2, tier 2 carrier) — `collapse` family.
//!
//! Each test is a cell of one of the six 2026-10-04 carrier probes
//! (book §3.2.15, "Measurements added by the 2026-10-04 carrier probes"),
//! rewritten from a `main` with `println!` into asserts against the rustc
//! oracle values recorded there. The parity harness runs the same tests under
//! rustc and under the transpiled C++; a cell that the shipped lane gets
//! wrong fails here until the §3.2.16 phase-2 step that fixes it lands.
//! Cases that are rustc *errors* (E0034, E0117, E0283) cannot be tests and
//! are not here.

pub mod collapse {
    // ===========================================================================
    // collapse — two instantiations of one generic trait on one type (the
    // shipped lane drops the second); the trait argument recovered from a
    // literal, a let annotation, an explicit path, a bound, a variable from a
    // function return, and a callee's parameter type.
    // ===========================================================================
    trait ConvT<A> {
        fn m(&self, a: A) -> i32;
        fn name(&self) -> &str;
        fn conv(&self) -> A;
    }
    struct Tv {
        v: i32,
    }
    impl ConvT<i32> for Tv {
        fn m(&self, a: i32) -> i32 {
            self.v + a
        }
        fn name(&self) -> &str {
            "name via i32"
        }
        fn conv(&self) -> i32 {
            self.v * 10
        }
    }
    impl ConvT<u8> for Tv {
        fn m(&self, a: u8) -> i32 {
            self.v + 100 * a as i32
        }
        fn name(&self) -> &str {
            "name via u8"
        }
        fn conv(&self) -> u8 {
            (self.v + 7) as u8
        }
    }
    fn bound_u8<X: ConvT<u8>>(x: &X) -> String {
        format!("{} / {}", x.name(), x.conv())
    }
    fn bound_i32<X: ConvT<i32>>(x: &X) -> String {
        format!("{} / {}", x.name(), x.conv())
    }
    fn takes_u8(x: u8) -> u8 {
        x
    }
    fn mk_u8() -> u8 {
        3
    }
    fn two_bounds<X: ConvT<u8> + ConvT<i32>>(x: &X) -> String {
        format!("{} {}", <X as ConvT<u8>>::name(x), <X as ConvT<i32>>::conv(x))
    }

    #[test]
    fn collapse_generic_trait_two_instantiations() {
        let t = Tv { v: 4 };
        assert_eq!(format!("{} {}", t.m(5i32), t.m(5u8)), "9 504");
        let a: i32 = t.conv();
        let b: u8 = t.conv();
        assert_eq!(format!("{} {}", a, b), "40 11");
        assert_eq!(
            format!("{} | {}", <Tv as ConvT<i32>>::name(&t), <Tv as ConvT<u8>>::name(&t)),
            "name via i32 | name via u8"
        );
        assert_eq!(
            format!("{} | {}", <Tv as ConvT<i32>>::conv(&t), <Tv as ConvT<u8>>::conv(&t)),
            "40 | 11"
        );
        assert_eq!(
            format!("{} | {}", bound_i32(&t), bound_u8(&t)),
            "name via i32 / 40 | name via u8 / 11"
        );
    }

    #[test]
    fn collapse_generic_trait_arg_inferred_from_argument() {
        let t = Tv { v: 4 };
        let a = mk_u8();
        assert_eq!(t.m(a), 304);
        assert_eq!(takes_u8(t.conv()), 11);
        assert_eq!(two_bounds(&t), "name via u8 40");
    }

    trait RefTr {
        fn m(&self) -> i32;
    }
    struct Tr2 {
        v: i32,
    }
    impl RefTr for Tr2 {
        fn m(&self) -> i32 {
            self.v
        }
    }
    impl RefTr for &Tr2 {
        fn m(&self) -> i32 {
            self.v + 1000
        }
    }
    fn ref_via_bound<X: RefTr>(x: &X) -> i32 {
        x.m()
    }

    #[test]
    fn collapse_ref_and_value_impls_are_distinct() {
        let x = Tr2 { v: 7 };
        let r: &Tr2 = &x;
        assert_eq!(format!("{} {}", x.m(), r.m()), "7 7");
        assert_eq!(format!("{} {}", (&x).m(), (&r).m()), "7 1007");
        assert_eq!(format!("{} {}", RefTr::m(&x), RefTr::m(&r)), "7 1007");
        assert_eq!(
            format!("{} {}", <Tr2 as RefTr>::m(&x), <&Tr2 as RefTr>::m(&r)),
            "7 1007"
        );
        assert_eq!(format!("{} {}", ref_via_bound(&x), ref_via_bound(&r)), "7 1007");
        let d1: &dyn RefTr = &x;
        let d2: &dyn RefTr = &r;
        assert_eq!(format!("{} {}", d1.m(), d2.m()), "7 1007");
    }

    #[test]
    fn collapse_ref_impl_through_coercion_and_box() {
        let x = Tr2 { v: 7 };
        let r: &Tr2 = &x;
        let d: &dyn RefTr = r;
        assert_eq!(d.m(), 7);
        let rr = &r;
        assert_eq!(rr.m(), 1007);
        let bx: Box<dyn RefTr> = Box::new(r);
        assert_eq!(bx.m(), 1007);
    }

    trait PrimRef {
        fn pm(&self) -> i32;
    }
    impl PrimRef for i32 {
        fn pm(&self) -> i32 {
            1
        }
    }
    impl PrimRef for &i32 {
        fn pm(&self) -> i32 {
            2
        }
    }

    #[test]
    fn collapse_primitive_ref_and_value_impls() {
        let x = 5i32;
        let r: &i32 = &x;
        let rr: &&i32 = &r;
        assert_eq!(format!("{} {} {}", x.pm(), r.pm(), rr.pm()), "1 1 2");
    }

    trait ConvF<A> {
        fn convf(&self) -> A;
    }
    impl ConvF<i32> for f64 {
        fn convf(&self) -> i32 {
            *self as i32
        }
    }
    impl ConvF<String> for f64 {
        fn convf(&self) -> String {
            format!("{}", self)
        }
    }

    #[test]
    fn collapse_generic_trait_on_primitive_by_let_annotation() {
        let x = 2.5f64;
        let a: i32 = x.convf();
        let b: String = x.convf();
        assert_eq!(format!("{} {}", a, b), "2 2.5");
    }
}
