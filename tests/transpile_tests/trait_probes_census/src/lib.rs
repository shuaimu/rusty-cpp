//! Differential oracle for the trait lowering (book §3.2, tier 2 carrier) — `census` family.
//!
//! Each test is a cell of one of the six 2026-10-04 carrier probes
//! (book §3.2.15, "Measurements added by the 2026-10-04 carrier probes"),
//! rewritten from a `main` with `println!` into asserts against the rustc
//! oracle values recorded there. The parity harness runs the same tests under
//! rustc and under the transpiled C++; a cell that the shipped lane gets
//! wrong fails here until the §3.2.16 phase-2 step that fixes it lands.
//! Cases that are rustc *errors* (E0034, E0117, E0283) cannot be tests and
//! are not here.

pub mod census {
    // ===========================================================================
    // census — one trait, four impl shapes (primitive, foreign generic with a
    // slot-default override, conditional impl on a local generic with a
    // generic-default override, tier-1 local struct), every use site.
    // ===========================================================================
    trait Shape {
        fn area(&self) -> f64;
        fn scale(&mut self, k: f64);
        fn describe(&self) -> String {
            format!("area={:.1}", self.area())
        }
        fn each<F: Fn(f64)>(&self, f: F)
        where
            Self: Sized,
        {
            f(self.area());
            f(self.area() * 2.0);
        }
    }

    impl Shape for i32 {
        fn area(&self) -> f64 {
            (*self as f64) * (*self as f64)
        }
        fn scale(&mut self, k: f64) {
            *self = ((*self as f64) * k) as i32;
        }
    }

    impl Shape for Vec<f64> {
        fn area(&self) -> f64 {
            self.iter().sum()
        }
        fn scale(&mut self, k: f64) {
            for x in self.iter_mut() {
                *x *= k;
            }
        }
        fn describe(&self) -> String {
            format!("vec[{}]={:.1}", self.len(), self.area())
        }
    }

    struct Wrapper<T>(T);
    impl<T: Into<f64> + Copy> Shape for Wrapper<T> {
        fn area(&self) -> f64 {
            let v: f64 = self.0.into();
            v * 3.0
        }
        fn scale(&mut self, _k: f64) {}
        fn each<F: Fn(f64)>(&self, f: F)
        where
            Self: Sized,
        {
            f(self.area() + 0.5);
        }
    }

    struct Sq {
        s: f64,
    }
    impl Shape for Sq {
        fn area(&self) -> f64 {
            self.s * self.s
        }
        fn scale(&mut self, k: f64) {
            self.s *= k;
        }
    }

    fn shape_f<S: Shape>(s: &S) -> f64 {
        s.area() + s.describe().len() as f64
    }
    fn shape_g<S: Shape>(s: &mut S) {
        s.scale(2.0);
    }
    fn shape_total(shapes: &[&dyn Shape]) -> f64 {
        shapes.iter().map(|s| s.area()).sum()
    }
    fn shape_dyn_describe(s: &dyn Shape) -> String {
        s.describe()
    }
    fn shape_grow(s: &mut dyn Shape) {
        s.scale(3.0);
    }

    #[test]
    fn census_static_calls_and_defaults() {
        let mut a: i32 = 4;
        let mut v: Vec<f64> = vec![1.0, 2.0, 3.5];
        let mut w: Wrapper<i32> = Wrapper(7);
        let mut wf: Wrapper<f32> = Wrapper(1.5);
        let sq: Sq = Sq { s: 1.5 };
        assert_eq!(
            format!("{} {} {} {}", a.area(), v.area(), w.area(), wf.area()),
            "16 6.5 21 4.5"
        );
        assert_eq!(
            format!("{} | {} | {}", a.describe(), v.describe(), w.describe()),
            "area=16.0 | vec[3]=6.5 | area=21.0"
        );
        a.scale(2.0);
        v.scale(2.0);
        w.scale(2.0);
        wf.scale(2.0);
        assert_eq!(
            format!("{} {} {} {}", a.area(), v.area(), w.area(), wf.area()),
            "64 13 21 4.5"
        );
        let acc = std::cell::RefCell::new(String::new());
        a.each(|x| acc.borrow_mut().push_str(&format!("{:.1},", x)));
        v.each(|x| acc.borrow_mut().push_str(&format!("{:.1},", x)));
        w.each(|x| acc.borrow_mut().push_str(&format!("{:.1},", x)));
        sq.each(|x| acc.borrow_mut().push_str(&format!("{:.1},", x)));
        assert_eq!(acc.borrow().as_str(), "64.0,128.0,13.0,26.0,21.5,2.2,4.5,");
    }

    #[test]
    fn census_generic_bound() {
        let mut a: i32 = 8;
        let mut v: Vec<f64> = vec![2.0, 4.0, 7.0];
        let mut w: Wrapper<i32> = Wrapper(7);
        let wf: Wrapper<f32> = Wrapper(1.5);
        let mut sq: Sq = Sq { s: 1.5 };
        assert_eq!(
            format!(
                "{} {} {} {} {}",
                shape_f(&a),
                shape_f(&v),
                shape_f(&w),
                shape_f(&wf),
                shape_f(&sq)
            ),
            "73 24 30 12.5 10.25"
        );
        shape_g(&mut a);
        shape_g(&mut v);
        shape_g(&mut w);
        shape_g(&mut sq);
        assert_eq!(
            format!("{} {} {} {}", a.area(), v.area(), w.area(), sq.area()),
            "256 26 21 9"
        );
    }

    #[test]
    fn census_dyn_refs() {
        let mut a: i32 = 16;
        let mut v: Vec<f64> = vec![4.0, 8.0, 14.0];
        let mut w: Wrapper<i32> = Wrapper(7);
        let wf: Wrapper<f32> = Wrapper(1.5);
        let sq: Sq = Sq { s: 3.0 };
        {
            let shapes: Vec<&dyn Shape> = vec![&a, &v, &w, &wf, &sq];
            assert_eq!(shape_total(&shapes), 316.5);
        }
        assert_eq!(
            format!(
                "{} | {} | {}",
                shape_dyn_describe(&a),
                shape_dyn_describe(&v),
                shape_dyn_describe(&w)
            ),
            "area=256.0 | vec[3]=26.0 | area=21.0"
        );
        shape_grow(&mut a);
        shape_grow(&mut v);
        shape_grow(&mut w);
        assert_eq!(
            format!("{} {} {}", a.area(), v.area(), w.area()),
            "2304 78 21"
        );
    }

    #[test]
    fn census_boxed() {
        let mut boxes: Vec<Box<dyn Shape>> = vec![
            Box::new(3i32),
            Box::new(vec![0.5, 0.25]),
            Box::new(Wrapper(2u8)),
            Box::new(Sq { s: 0.5 }),
        ];
        for b in boxes.iter_mut() {
            b.scale(4.0);
        }
        let mut out = String::new();
        for b in boxes.iter() {
            out.push_str(&format!("{}:{:.2} ", b.describe(), b.area()));
        }
        assert_eq!(
            out,
            "area=144.0:144.00 vec[2]=3.0:3.00 area=6.0:6.00 area=4.0:4.00 "
        );
    }
}
