//! Differential oracle for the trait lowering (book §3.2, tier 2 carrier) — `nonvtable` family.
//!
//! §3.2.2 "Non-vtable members" / §3.2.16 phase-2 step (8): associated consts,
//! no-receiver functions (`fn new() -> Self`), `-> Self` methods and generic
//! required methods reached through a *bounded type parameter* — the shapes no
//! vtable can carry. A local struct passes most cells through its static
//! members today; the cells that matter are the PRIMITIVE implementor's
//! (`impl Shape for i32`), which has no members for `T::new` / `T::SIDES` to
//! bind to. The parity harness runs the same tests under rustc and under the
//! transpiled C++.

pub mod nonvtable {
    trait Shape {
        const SIDES: u32;
        const TAG: &'static str = "shape";
        fn new(x: i32) -> Self;
        fn unit() -> Self
        where
            Self: Sized,
        {
            Self::new(1)
        }
        fn scaled(&self, k: i32) -> Self;
        fn area(&self) -> i32;
        fn describe<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result;
        fn label(&self) -> String {
            format!("{}:{}", Self::TAG, Self::SIDES)
        }
    }

    struct Sq {
        s: i32,
    }
    impl Shape for Sq {
        const SIDES: u32 = 4;
        fn new(x: i32) -> Self {
            Sq { s: x }
        }
        fn scaled(&self, k: i32) -> Self {
            Sq { s: self.s * k }
        }
        fn area(&self) -> i32 {
            self.s * self.s
        }
        fn describe<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
            write!(w, "sq{}", self.s)
        }
    }
    impl Shape for i32 {
        const SIDES: u32 = 1;
        const TAG: &'static str = "int";
        fn new(x: i32) -> Self {
            x
        }
        fn scaled(&self, k: i32) -> Self {
            *self * k
        }
        fn area(&self) -> i32 {
            *self + 100
        }
        fn describe<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
            write!(w, "i{}", self)
        }
    }

    fn make<T: Shape>(x: i32) -> (T, u32) {
        (T::new(x), T::SIDES)
    }
    fn grow<T: Shape>(t: &T) -> i32 {
        t.scaled(3).area()
    }
    fn tag_of<T: Shape>() -> &'static str {
        T::TAG
    }
    fn unit_area<T: Shape>() -> i32 {
        T::unit().area()
    }
    fn describe_to_string<T: Shape>(t: &T) -> String {
        let mut s = String::new();
        t.describe(&mut s).unwrap();
        s
    }

    #[test]
    fn nonvtable_const_and_new_on_local_struct() {
        let (q, n): (Sq, u32) = make::<Sq>(2);
        assert_eq!(format!("{} {} {}", q.area(), n, tag_of::<Sq>()), "4 4 shape");
    }

    #[test]
    fn nonvtable_const_and_new_on_primitive() {
        let (p, m): (i32, u32) = make::<i32>(5);
        assert_eq!(format!("{} {} {}", p, m, tag_of::<i32>()), "5 1 int");
    }

    #[test]
    fn nonvtable_explicit_path_forms() {
        assert_eq!(
            format!(
                "{} {} {}",
                <i32 as Shape>::SIDES,
                <i32 as Shape>::new(7),
                <Sq as Shape>::new(3).area()
            ),
            "1 7 9"
        );
    }

    #[test]
    fn nonvtable_self_returning_through_bound() {
        let q = Sq { s: 2 };
        assert_eq!(format!("{} {}", grow(&q), grow(&5)), "36 115");
    }

    #[test]
    fn nonvtable_generic_required_method_on_primitive() {
        assert_eq!(
            format!("{} {}", describe_to_string(&Sq { s: 2 }), describe_to_string(&9)),
            "sq2 i9"
        );
    }

    #[test]
    fn nonvtable_default_fn_and_default_const_through_bound() {
        assert_eq!(
            format!(
                "{} {} {} {}",
                unit_area::<Sq>(),
                unit_area::<i32>(),
                Sq { s: 1 }.label(),
                3i32.label()
            ),
            "1 101 shape:4 int:1"
        );
    }
}
