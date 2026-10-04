#![feature(prelude_import)]
#![allow(unused_imports)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod slab {
    use std::collections::HashMap;
    use std::collections::hash_map::Values;
    use vstd::prelude::*;
    const _: () = ();
    pub const SLAB_CAPACITY: usize = 65536;
    pub struct Slab<V: View> {
        pub inner: HashMap<u64, V>,
    }
    impl<V: View> View for Slab<V> {
        type V = Map<nat, V::V>;
    }
    impl<V: View> Slab<V> {
        pub fn new() -> Self {
            let result = Slab { inner: HashMap::new() };
            {}
            result
        }
        #[inline(always)]
        pub fn insert(&mut self, key: u64, value: V) {
            let _ = self.inner.insert(key, value);
            {}
        }
        #[inline(always)]
        pub fn get(&self, key: u64) -> Option<&V> {
            self.inner.get(&key)
        }
        #[inline(always)]
        pub fn get_mut(&mut self, key: u64) -> Option<&mut V> {
            self.inner.get_mut(&key)
        }
        #[inline(always)]
        pub fn values(&self) -> Values<'_, u64, V> {
            self.inner.values()
        }
        #[inline(always)]
        pub fn remove(&mut self, key: u64) -> Option<V> {
            let result = self.inner.remove(&key);
            {}
            result
        }
    }
}
pub use slab::Slab;
