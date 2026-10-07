//! Differential oracle for the trait lowering (book §3.2, tier 2 carrier) — `thin` family.
//!
//! Each test is a cell of one of the six 2026-10-04 carrier probes
//! (book §3.2.15, "Measurements added by the 2026-10-04 carrier probes"),
//! rewritten from a `main` with `println!` into asserts against the rustc
//! oracle values recorded there. The parity harness runs the same tests under
//! rustc and under the transpiled C++; a cell that the shipped lane gets
//! wrong fails here until the §3.2.16 phase-2 step that fixes it lands.
//! Cases that are rustc *errors* (E0034, E0117, E0283) cannot be tests and
//! are not here.

pub mod thin {
    // ===========================================================================
    // thin — supertrait with a default, defaults calling required and
    // supertrait methods, an INHERENT same-name method the default must not see,
    // a blanket impl over a local marker trait, &mut dyn, Box<dyn>.
    // ===========================================================================
    trait Super {
        fn s(&self) -> i32 {
            1100
        }
    }
    trait Tr: Super {
        fn m(&self) -> i32;
        fn n(&mut self, d: i32);
        fn k(self) -> i32
        where
            Self: Sized;
        fn twice(&self) -> i32 {
            self.m() * 2
        }
        fn ssum(&self) -> i32 {
            self.s() + self.m()
        }
    }
    impl Super for i32 {
        fn s(&self) -> i32 {
            99900
        }
    }
    impl Tr for i32 {
        fn m(&self) -> i32 {
            *self + 1
        }
        fn n(&mut self, d: i32) {
            *self += d;
        }
        fn k(self) -> i32 {
            self * 100
        }
    }
    struct W<T>(T);
    impl W<i32> {
        #[allow(dead_code)]
        fn m(&self) -> i32 {
            2002
        }
    }
    impl Super for W<i32> {}
    impl Tr for W<i32> {
        fn m(&self) -> i32 {
            self.0 + 10
        }
        fn n(&mut self, d: i32) {
            self.0 += d;
        }
        fn k(self) -> i32 {
            self.0 * 1000
        }
        fn twice(&self) -> i32 {
            self.m() * 3
        }
    }
    trait Score {
        fn score(&self) -> i32;
    }
    struct Sc(i32);
    impl Score for Sc {
        fn score(&self) -> i32 {
            self.0
        }
    }
    impl<T: Score> Super for T {
        fn s(&self) -> i32 {
            7
        }
    }
    impl<T: Score> Tr for T {
        fn m(&self) -> i32 {
            self.score() + 100
        }
        fn n(&mut self, _d: i32) {}
        fn k(self) -> i32 {
            5
        }
    }
    fn thin_f(t: &dyn Tr) -> String {
        format!("m={} twice={} ssum={} s={}", t.m(), t.twice(), t.ssum(), t.s())
    }
    fn thin_up(t: &dyn Tr) -> i32 {
        let sup: &dyn Super = t;
        sup.s()
    }
    fn thin_g(t: &mut dyn Tr) {
        t.n(3);
    }

    #[test]
    fn thin_dyn_and_upcast() {
        let x: i32 = 5;
        let w = W(1);
        let sc = Sc(40);
        assert_eq!(thin_f(&x), "m=6 twice=12 ssum=99906 s=99900");
        assert_eq!(thin_up(&x), 99900);
        assert_eq!(thin_f(&w), "m=11 twice=6006 ssum=1111 s=1100");
        assert_eq!(thin_up(&w), 1100);
        assert_eq!(thin_f(&sc), "m=140 twice=280 ssum=147 s=7");
        assert_eq!(thin_up(&sc), 7);
    }

    #[test]
    fn thin_static_route() {
        let x: i32 = 5;
        let w = W(1);
        let sc = Sc(40);
        assert_eq!(
            format!("m={} twice={} ssum={} k={}", x.m(), x.twice(), x.ssum(), x.k()),
            "m=6 twice=12 ssum=99906 k=500"
        );
        // w.m() is the INHERENT m (2002); twice's override calls self.m() and
        // sees the inherent too (6006); the default ssum sees the trait's m (1111).
        assert_eq!(
            format!("m={} twice={} ssum={} k={}", w.m(), w.twice(), w.ssum(), W(1).k()),
            "m=2002 twice=6006 ssum=1111 k=1000"
        );
        assert_eq!(
            format!("m={} twice={} ssum={} k={}", sc.m(), sc.twice(), sc.ssum(), Sc(40).k()),
            "m=140 twice=280 ssum=147 k=5"
        );
    }

    #[test]
    fn thin_mut_dyn_and_box() {
        let mut y: i32 = 7;
        let mut w2 = W(2);
        thin_g(&mut y);
        thin_g(&mut w2);
        assert_eq!(format!("y={} w2={}", y, w2.0), "y=10 w2=5");
        let mut zoo: Vec<Box<dyn Tr>> = vec![Box::new(3i32), Box::new(W(4)), Box::new(Sc(1))];
        for b in zoo.iter_mut() {
            b.n(1);
        }
        let v: Vec<String> = zoo
            .iter()
            .map(|b| format!("{}/{}/{}", b.m(), b.twice(), b.ssum()))
            .collect();
        assert_eq!(v.join(" "), "5/10/99905 15/6006/1115 101/202/108");
    }
}
