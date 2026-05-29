// HashSet tests against the transpiled hashbrown_port module.
// Replaces tests/rusty_hashset_test.cpp which targeted the
// hand-written include/rusty/hashset.hpp. HashSet here is the facade
// wrapping HashMap<T, std::monostate> defined in
// transpiled/hashbrown_port/hashbrown_port.set.cppm.

#include <cstdint>
#include <cstdio>
#include <cassert>
#include <string>

import hashbrown_port.set;
import hashbrown_port.map;
import hashbrown_port.hasher;

namespace {

void test_default_ctor() {
    auto s = HashSet<int>::new_();
    assert(s.len() == 0);
    assert(s.is_empty());
}

void test_insert_returns_bool() {
    auto s = HashSet<int>::with_capacity(16);
    assert(s.insert(42) == true);    // newly inserted
    assert(s.insert(42) == false);   // duplicate
    assert(s.len() == 1);
    assert(s.contains(42));
    assert(!s.contains(99));
}

void test_bulk_insert_contains() {
    auto s = HashSet<int>::with_capacity(128);
    for (int i = 0; i < 100; ++i) s.insert(i);
    assert(s.len() == 100);
    int found = 0;
    for (int i = 0; i < 100; ++i) if (s.contains(i)) ++found;
    assert(found == 100);
}

void test_remove() {
    auto s = HashSet<int>::with_capacity(16);
    for (int i = 0; i < 10; ++i) s.insert(i);
    assert(s.remove(5) == true);
    assert(s.contains(5) == false);
    assert(s.remove(5) == false);
    assert(s.len() == 9);
}

void test_clear() {
    auto s = HashSet<int>::with_capacity(16);
    for (int i = 0; i < 10; ++i) s.insert(i);
    assert(!s.is_empty());
    s.clear();
    assert(s.is_empty());
    assert(s.len() == 0);
    assert(!s.contains(0));
}

void test_growth_via_new_() {
    auto s = HashSet<int>::new_();
    for (int i = 0; i < 1000; ++i) s.insert(i * 3);
    assert(s.len() == 1000);
    int hits = 0;
    for (int i = 0; i < 1000; ++i) if (s.contains(i * 3)) ++hits;
    assert(hits == 1000);
}

void test_move_semantics() {
    auto s1 = HashSet<int>::new_();
    s1.insert(1);
    s1.insert(2);
    s1.insert(3);
    auto s2 = std::move(s1);
    assert(s2.len() == 3);
    assert(s2.contains(2));
}

}  // namespace

int main() {
    std::puts("hashbrown_port set test...");
    test_default_ctor();
    test_insert_returns_bool();
    test_bulk_insert_contains();
    test_remove();
    test_clear();
    test_growth_via_new_();
    test_move_semantics();
    std::puts("hashbrown_port set test: all passed");
    return 0;
}
