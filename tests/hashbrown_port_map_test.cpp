// HashMap tests against the transpiled hashbrown_port module.
// Replaces the old tests/rusty_hashmap_test.cpp which targeted the
// hand-written include/rusty/hashmap.hpp. API surface follows the
// hashbrown port (HashMap::new_, insert returns Option<V>, find via
// raw-table predicate, etc.) — same shape used in
// docs/hashbrown_port/smoke_test.cpp.

#include <cstdint>
#include <cstdio>
#include <cassert>
#include <string>
#include <rusty/rusty.hpp>

import hashbrown_port.raw;
import hashbrown_port.map;
import hashbrown_port.hasher;

using rusty::port::collections::hashbrown::HashMap;
using rusty::port::collections::hashbrown::DefaultHasher;
using rusty::port::collections::hashbrown::make_hash;

namespace {

void test_default_ctor_and_len() {
    auto m = HashMap<int, int>::new_();
    assert(rusty::len(m) == 0);
}

void test_with_capacity_and_basic_insert() {
    auto m = HashMap<int, int>::with_capacity(16);
    auto prev1 = m.insert(1, 10);
    auto prev2 = m.insert(2, 20);
    auto prev3 = m.insert(3, 30);
    assert(prev1.is_none());
    assert(prev2.is_none());
    assert(prev3.is_none());
    assert(rusty::len(m) == 3);
}

void test_insert_returns_old_on_replace() {
    auto m = HashMap<int, int>::with_capacity(16);
    auto first = m.insert(7, 100);
    assert(first.is_none());
    auto second = m.insert(7, 200);
    assert(second.is_some());
    assert(std::move(second).unwrap() == 100);
    assert(rusty::len(m) == 1);
}

void test_find_via_raw_table() {
    auto m = HashMap<int, int>::with_capacity(16);
    m.insert(42, 1000);
    m.insert(99, 2000);
    auto h42 = make_hash<int, DefaultHasher>(m.hash_builder, 42);
    auto b = m.table.find(h42, [](const auto& kv) {
        return std::get<0>(kv) == 42;
    });
    assert(b.is_some());
    auto& kv = b.unwrap().as_ref();
    assert(std::get<1>(kv) == 1000);
}

void test_growth_via_new_() {
    auto m = HashMap<int, int>::new_();
    for (int i = 0; i < 1000; ++i) m.insert(i, i * 7);
    assert(rusty::len(m) == 1000);
    int misses = 0;
    for (int i = 0; i < 1000; ++i) {
        auto h = make_hash<int, DefaultHasher>(m.hash_builder, i);
        auto b = m.table.find(h, [i](const auto& kv) {
            return std::get<0>(kv) == i;
        });
        if (!b.is_some() || std::get<1>(b.unwrap().as_ref()) != i * 7) ++misses;
    }
    assert(misses == 0);
}

void test_move_semantics() {
    auto m1 = HashMap<int, int>::with_capacity(8);
    m1.insert(1, 10);
    m1.insert(2, 20);
    auto m2 = std::move(m1);
    assert(rusty::len(m2) == 2);
    auto h2 = make_hash<int, DefaultHasher>(m2.hash_builder, 2);
    auto b = m2.table.find(h2, [](const auto& kv) {
        return std::get<0>(kv) == 2;
    });
    assert(b.is_some());
    assert(std::get<1>(b.unwrap().as_ref()) == 20);
}

}  // namespace

int main() {
    std::puts("hashbrown_port map test...");
    test_default_ctor_and_len();
    test_with_capacity_and_basic_insert();
    test_insert_returns_old_on_replace();
    test_find_via_raw_table();
    test_growth_via_new_();
    test_move_semantics();
    std::puts("hashbrown_port map test: all passed");
    return 0;
}
