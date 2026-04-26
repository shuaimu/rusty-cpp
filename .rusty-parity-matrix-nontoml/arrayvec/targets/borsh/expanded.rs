#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
