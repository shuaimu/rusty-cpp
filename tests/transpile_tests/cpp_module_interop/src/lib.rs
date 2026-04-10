use cpp::custom::math as cpp_math;
use cpp::std as cpp_std;

pub fn max_plus_one(lo: i32, hi: i32) -> i32 {
    unsafe { cpp_math::add_one(cpp_std::max(lo, hi)) }
}

pub fn module_constant() -> i32 {
    cpp_math::DEFAULT_BIAS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(max_plus_one(1, 2), 3);
        assert_eq!(module_constant(), 1);
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
    }

    pub mod custom {
        pub mod math {
            pub const DEFAULT_BIAS: i32 = 1;

            pub unsafe fn add_one(v: i32) -> i32 {
                v + DEFAULT_BIAS
            }
        }
    }
}
