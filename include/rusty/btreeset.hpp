#ifndef RUSTY_BTREESET_HPP
#define RUSTY_BTREESET_HPP

// rusty::BTreeSet was historically a hand-written `std::set` facade
// (~440 LOC). It has been replaced with an alias to
// `btree_port::BTreeSet`, the canonical implementation maintained in
// the BTreeMap-port effort (see `docs/btreemap_port/`). See the
// matching note in `rusty/btreemap.hpp` for the migration rationale.

#include <btree_port/btreemap.hpp>

namespace rusty {

template <typename T, typename Compare = std::less<T>>
using BTreeSet = btree_port::BTreeSet<T, Compare>;

}  // namespace rusty

#endif  // RUSTY_BTREESET_HPP
