#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
pub mod module_spec {
    use vstd::prelude::*;
    pub struct ModuleSpec<L> {
        pub well_formed: ::vstd::prelude::FnSpec<(L,), bool>,
        pub progress: ::vstd::prelude::FnSpec<(L, L), bool>,
    }
}
pub mod async_contract {
    use vstd::prelude::*;
    pub struct AsyncContract<L, T> {
        pub acceptance: ::vstd::prelude::FnSpec<(L, T), bool>,
        pub fulfillment: ::vstd::prelude::FnSpec<(L, T), bool>,
        pub assumption: ::vstd::prelude::FnSpec<(L, T), bool>,
    }
}
pub mod local_liveness {
    use vstd::prelude::*;
    pub struct LocalLiveness<L> {
        pub acceptance: ::vstd::prelude::FnSpec<(L, int), bool>,
        pub fulfillment: ::vstd::prelude::FnSpec<(L, int, int), bool>,
        pub timely: ::vstd::prelude::FnSpec<(L, int, int), bool>,
    }
}
pub mod action_safety {
    use vstd::prelude::*;
    pub struct ActionSafety<L> {
        pub acceptance: ::vstd::prelude::FnSpec<(L, int), bool>,
        pub validity: ::vstd::prelude::FnSpec<(L, int), bool>,
    }
}
#[allow(unused_imports)]
pub use module_spec::*;
#[allow(unused_imports)]
pub use async_contract::*;
#[allow(unused_imports)]
pub use local_liveness::*;
#[allow(unused_imports)]
pub use action_safety::*;
