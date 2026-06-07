// Hand-translated bodies for rustc btree/{map,set}/tests.rs tests
// that we've moved from "skip" to "real exercise" of btree_port.
//
// Why a separate TU instead of putting these in btree_tests_port.cppm:
// when BTreeMap<int,int> is instantiated inside the module purview
// (i.e. inside the .cppm itself), the destructor path hits a static
// assert — `ManuallyDrop<Global>` ends up being clone()d in the drop
// codegen and `is_copy_constructible_v<ManuallyDrop<Global>>` is
// false. Same instantiation from a regular .cpp that imports the
// module compiles fine (the existing btree_port_module_test.out works
// this way). Until the in-module-purview instantiation bug is fixed,
// translated test bodies live here.
//
// Test name convention: `<rust_test_name>_unstubbed` so registration
// doesn't collide with the corresponding stub in btree_tests_port.cppm.

import btree_port.btree.map;
import btree_port.btree.set;

#include <cassert>
#include <cstdio>
#include <tuple>
#include <utility>
#include <rusty/alloc.hpp>
#include <rusty/test_runner.hpp>

namespace {

template<typename K, typename V>
using BTreeMap = ::btree_port::btree::map::BTreeMap<K, V, ::rusty::alloc::Global>;
template<typename T>
using BTreeSet = ::btree_port::btree::set::BTreeSet<T, ::rusty::alloc::Global>;

template<typename K, typename V> auto make_map() {
    return BTreeMap<K, V>::new_in(::rusty::alloc::Global{});
}
template<typename T> auto make_set() {
    return BTreeSet<T>::new_in(::rusty::alloc::Global{});
}

} // anonymous

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_get_key_value (trimmed)
// Full Rust source also exercises map.remove + post-remove checks; the
// remove call triggers a stale-codegen bug in btree_port — see
// docs/btree_tests_port/STATUS.md "ManuallyDrop<Global>::clone".
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_get_key_value_unstubbed") {
    auto map = make_map<int, int>();

    assert(map.is_empty());

    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);

    assert(map.len() == 3);
    {
        auto kv = map.get_key_value(1);
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 1);
        assert(std::get<1>(t) == 10);
    }
    {
        auto kv = map.get_key_value(3);
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 3);
        assert(std::get<1>(t) == 30);
    }
    assert(map.get_key_value(4).is_none());
}

// rustc set/tests.rs::test_clear blocked: BTreeSet::clear → BTreeMap::clear
// → `rusty::clone(this->alloc)` where `this->alloc` is ManuallyDrop<Global>,
// whose copy ctor is deleted. Latent btree_port bug — needs map.cppm:5579
// rewritten to unwrap+clone+re-wrap instead of cloning the ManuallyDrop
// directly. Same bug also blocks into_keys/into_values on the destructor
// path. See docs/btree_tests_port/STATUS.md.
