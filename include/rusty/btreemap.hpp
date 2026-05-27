#ifndef RUSTY_BTREEMAP_HPP
#define RUSTY_BTREEMAP_HPP

// rusty::BTreeMap was historically a hand-written `std::map` facade
// (~900 LOC). It has been replaced with an alias to
// `btree_port::BTreeMap`, the canonical implementation maintained in
// the BTreeMap-port effort (see `docs/btreemap_port/`). Both were
// `std::map` wrappers, so the runtime behavior is unchanged — but
// `btree_port::BTreeMap` is the namespace that will eventually
// switch over to the actual transpiled rustc B-tree once the
// transpiled module is fully wired in. Keeping `rusty::BTreeMap`
// as an alias means existing call sites continue to compile.

#include <btree_port/btreemap.hpp>

namespace rusty {

template <typename K, typename V, typename Compare = std::less<K>>
using BTreeMap = btree_port::BTreeMap<K, V, Compare>;

}  // namespace rusty

#endif  // RUSTY_BTREEMAP_HPP
