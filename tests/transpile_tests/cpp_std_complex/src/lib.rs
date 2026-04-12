use cpp::std as cpp_std;

pub fn normalize_window(lo: i32, hi: i32, probe: i32) -> i32 {
    unsafe {
        let lower = cpp_std::min(lo, hi);
        let upper = cpp_std::max(lo, hi);
        let clamped = cpp_std::clamp(probe, lower, upper);
        let spread = cpp_std::abs(upper - lower);
        cpp_std::max(clamped + spread, 0)
    }
}

pub fn clamp_delta(a: i32, b: i32, value: i32) -> i32 {
    unsafe {
        let lower = cpp_std::min(a, b);
        let upper = cpp_std::max(a, b);
        cpp_std::abs(cpp_std::clamp(value, lower, upper) - value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(normalize_window(-4, 10, 7), 21);
        assert_eq!(normalize_window(10, -4, -8), 10);
        assert_eq!(clamp_delta(-3, 8, 6), 0);
        assert_eq!(clamp_delta(-3, 8, 11), 3);
        assert_eq!(clamp_delta(-3, 8, -9), 6);
    }
}

// Keep cargo-expand/cargo-test happy for this fixture. The transpiler still
// treats `use cpp::...` as reserved C++-module imports.
mod cpp {
    pub mod std {
        pub unsafe fn max(lo: i32, hi: i32) -> i32 {
            if lo > hi {
                lo
            } else {
                hi
            }
        }

        pub unsafe fn min(lo: i32, hi: i32) -> i32 {
            if lo < hi {
                lo
            } else {
                hi
            }
        }

        pub unsafe fn clamp(value: i32, lo: i32, hi: i32) -> i32 {
            if value < lo {
                lo
            } else if value > hi {
                hi
            } else {
                value
            }
        }

        pub unsafe fn abs(v: i32) -> i32 {
            if v < 0 {
                -v
            } else {
                v
            }
        }
    }
}
