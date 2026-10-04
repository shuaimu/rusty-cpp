use vstd::prelude::*;
use crate::generic::log::Log;
use crate::framework::module_spec::ModuleSpec;

verus! {

// Generic utility ModuleSpec template. Each concrete utility instantiates it
// with its own `inv` predicate (typically `utility_inv<M, R>`, optionally
// conjoined with the utility's own well-formedness clauses). The default
// progress relation is permissive ("next state is well-formed"); a utility may
// supply a stronger one (e.g. "log grew by exactly one tick cycle") by building
// its own ModuleSpec directly.
pub open spec fn utility_module_spec<M, R>(
  inv: spec_fn(Log<M, R>) -> bool,
) -> ModuleSpec<Log<M, R>> {
  ModuleSpec {
    well_formed: inv,
    progress: |_l: Log<M, R>, l_prime: Log<M, R>| inv(l_prime),
  }
}

}
