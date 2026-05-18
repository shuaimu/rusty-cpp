// Smoke test for the `btree_port::BTreeMap` / `BTreeSet` facade.
//
// The BTreeMap port (docs/btreemap_port/) transpiles ~6.4 KLoC of
// rustc-stdlib btree internals into a single C++20 module after
// prep.sh's cycle-breaking merge, but ~20 compile errors in
// transpiler-side template-parameter recovery remain. The facade
// in include/btree_port/btreemap.hpp is the "working version"
// portion of the hybrid — usable today, with a clean migration
// path as transpiler fixes land.
//
// This test exercises the facade's public surface to keep it from
// regressing.

#include "../include/btree_port/btreemap.hpp"

#include <cassert>
#include <cstdio>
#include <string>

static void test_btreemap_basic_insert_and_get() {
    auto m = btree_port::BTreeMap<int, std::string>::new_();

    // First insert should not displace anything.
    auto displaced = m.insert(1, std::string("one"));
    assert(displaced.is_none());

    m.insert(2, std::string("two"));

    // Inserting at an existing key should return the previous value.
    displaced = m.insert(1, std::string("ONE"));
    assert(!displaced.is_none());
    assert(displaced.unwrap() == "one");

    assert(m.len() == 2);

    auto got = m.get(1);
    assert(!got.is_none());
    assert(got.unwrap().get() == "ONE");

    assert(m.contains_key(2));
    assert(!m.contains_key(99));
}

static void test_btreemap_remove() {
    auto m = btree_port::BTreeMap<int, std::string>::new_();
    m.insert(1, std::string("one"));
    m.insert(2, std::string("two"));
    m.insert(3, std::string("three"));

    auto removed = m.remove(2);
    assert(!removed.is_none());
    assert(removed.unwrap() == "two");
    assert(m.len() == 2);
    assert(!m.contains_key(2));

    // Removing a missing key returns None.
    auto missing = m.remove(99);
    assert(missing.is_none());
}

static void test_btreemap_ordered_iteration() {
    auto m = btree_port::BTreeMap<int, int>::new_();
    m.insert(3, 30);
    m.insert(1, 10);
    m.insert(2, 20);
    m.insert(5, 50);
    m.insert(4, 40);

    int prev = -1;
    int count = 0;
    for (const auto& [k, v] : m) {
        assert(k > prev);
        assert(v == k * 10);
        prev = k;
        ++count;
    }
    assert(count == 5);
}

static void test_btreemap_clone() {
    auto a = btree_port::BTreeMap<int, int>::new_();
    a.insert(1, 10);
    a.insert(2, 20);

    auto b = a.clone();
    assert(b.len() == 2);

    // Mutating `a` after clone should not affect `b`.
    a.insert(3, 30);
    assert(a.len() == 3);
    assert(b.len() == 2);
    assert(!b.contains_key(3));
}

static void test_btreemap_clear() {
    auto m = btree_port::BTreeMap<int, int>::new_();
    m.insert(1, 1);
    m.insert(2, 2);
    assert(!m.is_empty());
    m.clear();
    assert(m.is_empty());
    assert(m.len() == 0);
}

static void test_btreemap_initializer_list() {
    btree_port::BTreeMap<int, std::string> m = {
        {1, "one"},
        {2, "two"},
        {3, "three"},
    };
    assert(m.len() == 3);
    assert(m.get(2).unwrap().get() == "two");
}

static void test_btreeset_basic() {
    auto s = btree_port::BTreeSet<int>::new_();
    assert(s.insert(5));        // newly inserted → true
    assert(!s.insert(5));       // duplicate → false
    assert(s.contains(5));
    assert(!s.contains(99));
    assert(s.len() == 1);

    assert(s.remove(5));
    assert(!s.remove(5));       // already gone
    assert(s.is_empty());
}

static void test_btreemap_first_last() {
    auto m = btree_port::BTreeMap<int, std::string>::new_();
    // Empty map: both Optionals are None.
    assert(m.first_key_value().is_none());
    assert(m.last_key_value().is_none());

    m.insert(5, std::string("five"));
    m.insert(1, std::string("one"));
    m.insert(3, std::string("three"));

    auto first = m.first_key_value();
    assert(!first.is_none());
    auto [fk, fv] = first.unwrap();
    assert(fk.get() == 1);
    assert(fv.get() == "one");

    auto last = m.last_key_value();
    assert(!last.is_none());
    auto [lk, lv] = last.unwrap();
    assert(lk.get() == 5);
    assert(lv.get() == "five");
}

static void test_btreemap_range() {
    auto m = btree_port::BTreeMap<int, int>::new_();
    for (int i = 0; i < 10; ++i) {
        m.insert(i, i * 10);
    }
    // Range [3, 7) should yield keys 3, 4, 5, 6.
    int seen = 0;
    int sum = 0;
    auto [it, end] = m.range(3, 7);
    for (; it != end; ++it) {
        sum += it->first;
        ++seen;
    }
    assert(seen == 4);
    assert(sum == 3 + 4 + 5 + 6);
}

static void test_btreeset_ordered_iter() {
    btree_port::BTreeSet<int> s = {3, 1, 4, 1, 5, 9, 2, 6};
    // Set deduplicates "1"; expect 7 unique values in order.
    int prev = 0;
    int count = 0;
    for (int v : s) {
        assert(v > prev);
        prev = v;
        ++count;
    }
    assert(count == 7);
}

int main() {
    test_btreemap_basic_insert_and_get();
    test_btreemap_remove();
    test_btreemap_ordered_iteration();
    test_btreemap_clone();
    test_btreemap_clear();
    test_btreemap_initializer_list();
    test_btreemap_first_last();
    test_btreemap_range();
    test_btreeset_basic();
    test_btreeset_ordered_iter();
    std::fprintf(stderr, "btree_port facade: 10 tests passed\n");
    return 0;
}
