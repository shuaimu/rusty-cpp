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
#include <vector>

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

static void test_btreemap_pop_first_last() {
    auto m = btree_port::BTreeMap<int, std::string>::new_();
    assert(m.pop_first().is_none());
    assert(m.pop_last().is_none());

    m.insert(2, std::string("two"));
    m.insert(1, std::string("one"));
    m.insert(3, std::string("three"));

    auto first = m.pop_first();
    assert(!first.is_none());
    auto [fk, fv] = first.unwrap();
    assert(fk == 1);
    assert(fv == "one");
    assert(m.len() == 2);

    auto last = m.pop_last();
    assert(!last.is_none());
    auto [lk, lv] = last.unwrap();
    assert(lk == 3);
    assert(lv == "three");
    assert(m.len() == 1);
    assert(m.contains_key(2));
}

static void test_btreemap_retain() {
    auto m = btree_port::BTreeMap<int, int>::new_();
    for (int i = 1; i <= 10; ++i) {
        m.insert(i, i * 10);
    }
    // Retain even keys only.
    m.retain([](const int& k, int& /*v*/) { return k % 2 == 0; });
    assert(m.len() == 5);
    assert(!m.contains_key(1));
    assert(m.contains_key(2));
    assert(!m.contains_key(3));
    assert(m.contains_key(10));
}

static void test_btreemap_entry_or_insert() {
    auto m = btree_port::BTreeMap<std::string, int>::new_();

    // Vacant entry — inserts default.
    int& v = m.entry(std::string("apple")).or_insert(0);
    assert(v == 0);
    v = 5;
    assert(m.get(std::string("apple")).unwrap().get() == 5);

    // Occupied entry — leaves existing value alone.
    int& v2 = m.entry(std::string("apple")).or_insert(99);
    assert(v2 == 5);

    // Idiomatic counter pattern.
    auto counts = btree_port::BTreeMap<std::string, int>::new_();
    for (const auto& w : {std::string("a"), std::string("b"),
                          std::string("a"), std::string("c"),
                          std::string("a"), std::string("b")}) {
        counts.entry(w).or_insert(0) += 1;
    }
    assert(counts.get(std::string("a")).unwrap().get() == 3);
    assert(counts.get(std::string("b")).unwrap().get() == 2);
    assert(counts.get(std::string("c")).unwrap().get() == 1);
}

static void test_btreemap_entry_or_insert_with_and_modify() {
    auto m = btree_port::BTreeMap<int, int>::new_();

    int call_count = 0;
    auto make_default = [&]() { ++call_count; return 100; };

    // Vacant: factory called.
    int& v = m.entry(7).or_insert_with(make_default);
    assert(v == 100);
    assert(call_count == 1);

    // Occupied: factory NOT called.
    int& v2 = m.entry(7).or_insert_with(make_default);
    assert(v2 == 100);
    assert(call_count == 1);  // unchanged

    // and_modify on occupied: closure runs.
    m.entry(7).and_modify([](int& x) { x *= 2; }).or_insert(0);
    assert(m.get(7).unwrap().get() == 200);

    // and_modify on vacant: closure does NOT run, then or_insert.
    m.entry(99).and_modify([](int& x) { x = -1; }).or_insert(42);
    assert(m.get(99).unwrap().get() == 42);
}

static void test_btreemap_keys_values() {
    auto m = btree_port::BTreeMap<int, std::string>::new_();
    m.insert(3, std::string("three"));
    m.insert(1, std::string("one"));
    m.insert(2, std::string("two"));

    // Keys: ascending order.
    int expected_k = 1;
    int seen_k = 0;
    for (const auto& k : m.keys()) {
        assert(k == expected_k);
        ++expected_k;
        ++seen_k;
    }
    assert(seen_k == 3);

    // Values: in key-ascending order.
    std::string expected_v[] = {"one", "two", "three"};
    int idx = 0;
    for (const auto& v : m.values()) {
        assert(v == expected_v[idx]);
        ++idx;
    }
    assert(idx == 3);

    // values_mut: mutate through the view.
    for (auto& v : m.values_mut()) {
        v += "!";
    }
    assert(m.get(1).unwrap().get() == "one!");
    assert(m.get(2).unwrap().get() == "two!");
    assert(m.get(3).unwrap().get() == "three!");
}

static void test_btreemap_extend() {
    auto m = btree_port::BTreeMap<int, int>::new_();
    m.insert(1, 10);
    m.insert(2, 20);

    // extend overwrites existing keys (Rust behavior).
    std::vector<std::pair<int, int>> more = {{2, 999}, {3, 30}, {4, 40}};
    m.extend(more.begin(), more.end());
    assert(m.len() == 4);
    assert(m.get(1).unwrap().get() == 10);
    assert(m.get(2).unwrap().get() == 999);  // overwritten
    assert(m.get(3).unwrap().get() == 30);
    assert(m.get(4).unwrap().get() == 40);
}

static void test_btreemap_append() {
    auto a = btree_port::BTreeMap<int, std::string>::new_();
    a.insert(1, std::string("a-one"));
    a.insert(2, std::string("a-two"));

    auto b = btree_port::BTreeMap<int, std::string>::new_();
    b.insert(2, std::string("b-two"));  // collides with a
    b.insert(3, std::string("b-three"));

    a.append(b);
    assert(b.is_empty());
    assert(a.len() == 3);
    assert(a.get(1).unwrap().get() == "a-one");
    assert(a.get(2).unwrap().get() == "b-two");  // b wins on collision
    assert(a.get(3).unwrap().get() == "b-three");
}

static void test_btreemap_split_off() {
    auto m = btree_port::BTreeMap<int, int>::new_();
    for (int i = 0; i < 10; ++i) {
        m.insert(i, i * 100);
    }
    auto upper = m.split_off(5);
    // `m` keeps 0..5, `upper` gets 5..10.
    assert(m.len() == 5);
    assert(upper.len() == 5);
    assert(m.contains_key(4) && !m.contains_key(5));
    assert(upper.contains_key(5) && upper.contains_key(9));
    assert(!upper.contains_key(4));
    assert(upper.get(7).unwrap().get() == 700);
}

static void test_btreeset_pop_and_retain() {
    auto s = btree_port::BTreeSet<int>::new_();
    assert(s.pop_first().is_none());
    assert(s.pop_last().is_none());

    for (int x : {5, 1, 3, 2, 4}) s.insert(x);

    auto first = s.pop_first();
    assert(!first.is_none() && first.unwrap() == 1);
    auto last = s.pop_last();
    assert(!last.is_none() && last.unwrap() == 5);

    // 2, 3, 4 left.
    assert(s.len() == 3);

    // Retain only odd numbers — drops 2, 4.
    s.retain([](const int& x) { return x % 2 != 0; });
    assert(s.len() == 1);
    assert(s.contains(3));
}

static void test_btreeset_range() {
    btree_port::BTreeSet<int> s = {1, 3, 5, 7, 9};
    auto [it, end] = s.range(3, 8);
    int count = 0;
    int last = -1;
    for (; it != end; ++it) {
        assert(*it > last);
        last = *it;
        ++count;
    }
    // [3, 8): expect 3, 5, 7 → 3 elements.
    assert(count == 3);
}

static void test_btreeset_union_intersection_difference() {
    btree_port::BTreeSet<int> a = {1, 2, 3, 4};
    btree_port::BTreeSet<int> b = {3, 4, 5, 6};

    auto u = a.union_set(b);
    // 1..6 expected.
    assert(u.len() == 6);
    for (int v : {1, 2, 3, 4, 5, 6}) assert(u.contains(v));

    auto i = a.intersection(b);
    assert(i.len() == 2);
    assert(i.contains(3) && i.contains(4));

    auto d = a.difference(b);
    assert(d.len() == 2);
    assert(d.contains(1) && d.contains(2));

    auto sd = a.symmetric_difference(b);
    assert(sd.len() == 4);
    for (int v : {1, 2, 5, 6}) assert(sd.contains(v));
    assert(!sd.contains(3));
}

static void test_btreeset_subset_superset_disjoint() {
    btree_port::BTreeSet<int> small_set = {2, 3};
    btree_port::BTreeSet<int> big_set = {1, 2, 3, 4};
    btree_port::BTreeSet<int> other = {10, 11};

    assert(small_set.is_subset(big_set));
    assert(big_set.is_superset(small_set));
    assert(!big_set.is_subset(small_set));
    assert(small_set.is_disjoint(other));
    assert(!small_set.is_disjoint(big_set));

    // A set is always a subset and superset of itself.
    assert(small_set.is_subset(small_set));
    assert(small_set.is_superset(small_set));
}

static void test_from_iter() {
    // BTreeMap::from_iter — last-write-wins on duplicates.
    std::vector<std::pair<int, std::string>> pairs = {
        {3, "three"}, {1, "one"}, {2, "two"}, {3, "THREE"},  // dup key
    };
    auto m = btree_port::BTreeMap<int, std::string>::from_iter(
        pairs.begin(), pairs.end());
    assert(m.len() == 3);
    assert(m.get(1).unwrap().get() == "one");
    assert(m.get(2).unwrap().get() == "two");
    assert(m.get(3).unwrap().get() == "THREE");  // last-write-wins

    // BTreeSet::from_iter — duplicates dropped.
    std::vector<int> ints = {5, 1, 3, 1, 4, 5, 9, 2, 6};
    auto s = btree_port::BTreeSet<int>::from_iter(ints.begin(), ints.end());
    assert(s.len() == 7);
    for (int v : {1, 2, 3, 4, 5, 6, 9}) assert(s.contains(v));
}

static void test_realistic_workflow() {
    // Mimics a small log-line frequency analyzer: count word
    // occurrences, then pick the top-K alphabetically-sorted keys.
    // Exercises insert / entry().or_insert() upsert / iter / clone /
    // keys() / values() / pop_first() in one workflow.
    auto counts = btree_port::BTreeMap<std::string, int>::new_();

    const char* words[] = {
        "alpha", "beta", "alpha", "gamma", "beta", "alpha",
        "delta", "epsilon", "gamma", "alpha",
    };
    for (const char* w : words) {
        counts.entry(std::string(w)).or_insert(0) += 1;
    }

    assert(counts.len() == 5);
    assert(counts.get(std::string("alpha")).unwrap().get() == 4);
    assert(counts.get(std::string("beta")).unwrap().get() == 2);
    assert(counts.get(std::string("delta")).unwrap().get() == 1);

    // Snapshot via clone — original stays usable.
    auto snapshot = counts.clone();
    assert(snapshot.len() == counts.len());

    // Keep only words seen >= 2 times.
    counts.retain([](const std::string&, int& n) { return n >= 2; });
    assert(counts.len() == 3);
    assert(!counts.contains_key(std::string("delta")));

    // Snapshot wasn't mutated.
    assert(snapshot.contains_key(std::string("delta")));

    // Iteration produces alphabetical order: alpha, beta, gamma.
    std::vector<std::string> seen_keys;
    for (const auto& [k, _v] : counts) {
        seen_keys.push_back(k);
    }
    assert(seen_keys.size() == 3);
    assert(seen_keys[0] == "alpha");
    assert(seen_keys[1] == "beta");
    assert(seen_keys[2] == "gamma");

    // Sum the values via values() view.
    int total = 0;
    for (int n : counts.values()) total += n;
    assert(total == 4 + 2 + 2);

    // Drain to a vector in order via pop_first.
    std::vector<std::pair<std::string, int>> drained;
    while (true) {
        auto opt = counts.pop_first();
        if (opt.is_none()) break;
        drained.push_back(opt.unwrap());
    }
    assert(counts.is_empty());
    assert(drained.size() == 3);
    assert(drained[0].first == "alpha");
    assert(drained[2].first == "gamma");
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
    test_btreemap_pop_first_last();
    test_btreemap_retain();
    test_btreemap_entry_or_insert();
    test_btreemap_entry_or_insert_with_and_modify();
    test_btreemap_keys_values();
    test_btreemap_extend();
    test_btreemap_append();
    test_btreemap_split_off();
    test_btreeset_pop_and_retain();
    test_btreeset_range();
    test_btreeset_union_intersection_difference();
    test_btreeset_subset_superset_disjoint();
    test_from_iter();
    test_realistic_workflow();
    std::fprintf(stderr, "btree_port facade: 24 tests passed\n");
    return 0;
}
