#include "../include/rusty/btreeset.hpp"
#include "../include/rusty/string.hpp"
#include <iostream>
#include <cassert>
#include <string>
#include <vector>

using namespace rusty;

void test_basic_operations() {
    std::cout << "Testing basic operations..." << std::endl;
    
    BTreeSet<int> set;
    assert(set.is_empty());
    assert(set.len() == 0);
    
    // Insert in random order
    assert(set.insert(3) == true);
    assert(set.insert(1) == true);
    assert(set.insert(4) == true);
    assert(set.insert(2) == true);
    assert(set.insert(2) == false);  // Already exists
    
    assert(!set.is_empty());
    assert(set.len() == 4);
    
    // Contains
    assert(set.contains(1));
    assert(set.contains(2));
    assert(set.contains(3));
    assert(set.contains(4));
    assert(!set.contains(5));
    
    // Elements are sorted
    auto vec = set.to_vec();
    assert(vec[0] == 1);
    assert(vec[1] == 2);
    assert(vec[2] == 3);
    assert(vec[3] == 4);
    
    // Remove
    assert(set.remove(2) == true);
    assert(set.remove(2) == false);  // Already removed
    assert(set.len() == 3);
    
    std::cout << "✓ Basic operations tests passed" << std::endl;
}

void test_first_last() {
    std::cout << "Testing first/last operations..." << std::endl;
    
    BTreeSet<int> set;
    
    // Empty set
    assert(set.first().is_none());
    assert(set.last().is_none());
    
    set.insert(3);
    set.insert(1);
    set.insert(5);
    set.insert(2);
    set.insert(4);
    
    // First (minimum)
    assert(set.first().is_some());
    assert(*set.first().unwrap() == 1);
    
    // Last (maximum)
    assert(set.last().is_some());
    assert(*set.last().unwrap() == 5);
    
    // Pop first
    auto first = set.pop_first();
    assert(first.is_some());
    assert(first.unwrap() == 1);
    assert(set.len() == 4);
    assert(!set.contains(1));
    
    // Pop last
    auto last = set.pop_last();
    assert(last.is_some());
    assert(last.unwrap() == 5);
    assert(set.len() == 3);
    assert(!set.contains(5));
    
    std::cout << "✓ First/last operations tests passed" << std::endl;
}

void test_range_operations() {
    std::cout << "Testing range operations..." << std::endl;
    
    BTreeSet<int> set;
    for (int i = 1; i <= 10; i++) {
        set.insert(i);
    }
    
    // Range query
    auto range = set.range(3, 7);
    assert(range.len() == 5);  // 3, 4, 5, 6, 7
    assert(range[0] == 3);
    assert(range[4] == 7);
    
    // Split off
    BTreeSet<int> upper = set.split_off(6);
    assert(set.len() == 5);   // 1-5
    assert(upper.len() == 5);  // 6-10
    
    assert(set.contains(5));
    assert(!set.contains(6));
    assert(upper.contains(6));
    assert(upper.contains(10));
    
    // Append (values in upper > values in set)
    set.append(std::move(upper));
    assert(set.len() == 10);
    assert(upper.len() == 0);
    
    std::cout << "✓ Range operations tests passed" << std::endl;
}

void test_set_operations() {
    std::cout << "Testing set operations..." << std::endl;
    
    BTreeSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);
    
    BTreeSet<int> set2;
    set2.insert(2);
    set2.insert(3);
    set2.insert(4);
    
    // Union
    BTreeSet<int> union_set = set1.union_with(set2);
    assert(union_set.len() == 4);
    auto union_vec = union_set.to_vec();
    assert(union_vec[0] == 1);
    assert(union_vec[1] == 2);
    assert(union_vec[2] == 3);
    assert(union_vec[3] == 4);
    
    // Intersection
    BTreeSet<int> intersect = set1.intersection(set2);
    assert(intersect.len() == 2);
    assert(intersect.contains(2));
    assert(intersect.contains(3));
    
    // Difference
    BTreeSet<int> diff = set1.difference(set2);
    assert(diff.len() == 1);
    assert(diff.contains(1));
    
    // Symmetric difference
    BTreeSet<int> sym_diff = set1.symmetric_difference(set2);
    assert(sym_diff.len() == 2);
    assert(sym_diff.contains(1));
    assert(sym_diff.contains(4));
    
    std::cout << "✓ Set operations tests passed" << std::endl;
}

void test_subset_superset() {
    std::cout << "Testing subset/superset..." << std::endl;
    
    BTreeSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    
    BTreeSet<int> set2;
    set2.insert(1);
    set2.insert(2);
    set2.insert(3);
    
    BTreeSet<int> set3;
    set3.insert(4);
    set3.insert(5);
    
    // Subset
    assert(set1.is_subset(set2));
    assert(!set2.is_subset(set1));
    assert(!set1.is_subset(set3));
    
    // Superset
    assert(set2.is_superset(set1));
    assert(!set1.is_superset(set2));
    
    // Disjoint
    assert(!set1.is_disjoint(set2));
    assert(set1.is_disjoint(set3));
    
    // Equality
    BTreeSet<int> set4;
    set4.insert(2);
    set4.insert(1);  // Different order
    assert(set1 == set4);
    assert(set1 != set2);
    
    std::cout << "✓ Subset/superset tests passed" << std::endl;
}

void test_iteration() {
    std::cout << "Testing iteration..." << std::endl;
    
    BTreeSet<int> set;
    set.insert(3);
    set.insert(1);
    set.insert(4);
    set.insert(2);
    
    // Iteration is in sorted order
    int prev = 0;
    int count = 0;
    for (const int& val : set) {
        assert(val > prev);
        prev = val;
        count++;
    }
    assert(count == 4);
    
    // Const iteration
    const BTreeSet<int>& const_set = set;
    count = 0;
    for (const int& val : const_set) {
        count++;
        (void)val;  // Suppress unused warning
    }
    assert(count == 4);
    
    std::cout << "✓ Iteration tests passed" << std::endl;
}

void test_get_take_replace() {
    std::cout << "Testing get, take, replace..." << std::endl;
    
    BTreeSet<int> set;
    set.insert(1);
    set.insert(2);
    set.insert(3);
    
    // Get
    auto val = set.get(2);
    assert(val.is_some());
    assert(*val.unwrap() == 2);
    
    auto val2 = set.get(4);
    assert(val2.is_none());
    
    // Take
    auto taken = set.take(2);
    assert(taken.is_some());
    assert(taken.unwrap() == 2);
    assert(!set.contains(2));
    assert(set.len() == 2);
    
    // Replace existing
    auto old = set.replace(1);
    assert(old.is_some());
    assert(old.unwrap() == 1);
    assert(set.contains(1));
    
    // Replace non-existing
    auto old2 = set.replace(5);
    assert(old2.is_none());
    assert(set.contains(5));
    
    std::cout << "✓ Get, take, replace tests passed" << std::endl;
}

void test_extend_retain() {
    std::cout << "Testing extend and retain..." << std::endl;
    
    BTreeSet<int> set1;
    set1.insert(1);
    set1.insert(3);
    
    BTreeSet<int> set2;
    set2.insert(2);
    set2.insert(4);
    
    // Extend
    set1.extend(std::move(set2));
    assert(set1.len() == 4);
    auto vec = set1.to_vec();
    assert(vec[0] == 1);
    assert(vec[1] == 2);
    assert(vec[2] == 3);
    assert(vec[3] == 4);
    
    // Retain
    set1.retain([](const int& x) { return x % 2 == 0; });
    assert(set1.len() == 2);
    assert(set1.contains(2));
    assert(set1.contains(4));
    
    std::cout << "✓ Extend and retain tests passed" << std::endl;
}

void test_drain() {
    std::cout << "Testing drain..." << std::endl;
    
    BTreeSet<int> set;
    set.insert(3);
    set.insert(1);
    set.insert(4);
    set.insert(2);
    
    auto drained = set.drain();
    assert(drained.len() == 4);
    assert(drained[0] == 1);  // Sorted order
    assert(drained[1] == 2);
    assert(drained[2] == 3);
    assert(drained[3] == 4);
    assert(set.is_empty());
    
    std::cout << "✓ Drain tests passed" << std::endl;
}

void test_clone() {
    std::cout << "Testing clone..." << std::endl;
    
    BTreeSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);
    
    BTreeSet<int> set2 = set1.clone();
    assert(set2.len() == 3);
    assert(set2.contains(1));
    assert(set2.contains(2));
    assert(set2.contains(3));
    
    // Modify original
    set1.insert(4);
    assert(set1.len() == 4);
    assert(set2.len() == 3);  // Clone unchanged
    
    std::cout << "✓ Clone tests passed" << std::endl;
}

void test_from_vec() {
    std::cout << "Testing from_vec..." << std::endl;
    
    // Test with std::vector (commented out since btreeset_from_std_vec is now optional)
    // std::vector<int> std_vec = {3, 1, 4, 1, 5, 9, 2, 6, 5};  // Duplicates
    // BTreeSet<int> set1 = btreeset_from_std_vec(std::move(std_vec));
    // assert(set1.len() == 7);  // Duplicates removed
    // auto sorted1 = set1.to_vec();
    // assert(sorted1[0] == 1);
    // assert(sorted1[1] == 2);
    // assert(sorted1[2] == 3);
    // assert(sorted1[3] == 4);
    // assert(sorted1[4] == 5);
    // assert(sorted1[5] == 6);
    // assert(sorted1[6] == 9);
    
    // Test with rusty::Vec
    Vec<int> vec = Vec<int>::make();
    vec.push(3); vec.push(1); vec.push(4);
    vec.push(1); vec.push(5); vec.push(9);
    vec.push(2); vec.push(6); vec.push(5);
    BTreeSet<int> set2 = btreeset_from_vec(std::move(vec));
    
    assert(set2.len() == 7);  // Duplicates removed
    auto sorted2 = set2.to_vec();
    assert(sorted2[0] == 1);
    assert(sorted2[1] == 2);
    assert(sorted2[2] == 3);
    assert(sorted2[3] == 4);
    assert(sorted2[4] == 5);
    assert(sorted2[5] == 6);
    assert(sorted2[6] == 9);
    
    std::cout << "✓ from_vec tests passed" << std::endl;
}

void test_stress() {
    std::cout << "Testing stress..." << std::endl;
    
    BTreeSet<int> set;
    const int N = 10000;
    
    // Insert in reverse order
    for (int i = N - 1; i >= 0; i--) {
        set.insert(i);
    }
    assert(set.len() == N);
    
    // Verify sorted and all present
    auto vec = set.to_vec();
    for (int i = 0; i < N; i++) {
        assert(vec[i] == i);
        assert(set.contains(i));
    }
    
    // Remove every other element
    for (int i = 0; i < N; i += 2) {
        assert(set.remove(i));
    }
    assert(set.len() == N / 2);
    
    // Verify pattern
    for (int i = 0; i < N; i++) {
        if (i % 2 == 0) {
            assert(!set.contains(i));
        } else {
            assert(set.contains(i));
        }
    }
    
    std::cout << "✓ Stress tests passed" << std::endl;
}

void test_custom_comparator() {
    std::cout << "Testing custom comparator..." << std::endl;
    
    // Reverse order comparator
    struct ReverseCompare {
        bool operator()(const int& a, const int& b) const {
            return a > b;
        }
    };
    
    BTreeSet<int, ReverseCompare> set;
    set.insert(1);
    set.insert(3);
    set.insert(2);
    
    // Elements should be in reverse order
    auto vec = set.to_vec();
    assert(vec[0] == 3);
    assert(vec[1] == 2);
    assert(vec[2] == 1);
    
    std::cout << "✓ Custom comparator tests passed" << std::endl;
}

void test_empty_sets() {
    std::cout << "Testing empty sets..." << std::endl;
    
    BTreeSet<int> empty1;
    BTreeSet<int> empty2;
    BTreeSet<int> non_empty;
    non_empty.insert(1);
    
    assert(empty1 == empty2);
    assert(empty1.is_subset(empty2));
    assert(empty1.is_superset(empty2));
    assert(empty1.is_subset(non_empty));
    assert(!empty1.is_superset(non_empty));
    assert(empty1.is_disjoint(empty2));
    assert(empty1.is_disjoint(non_empty));
    
    auto union_set = empty1.union_with(non_empty);
    assert(union_set.len() == 1);
    
    auto intersect = empty1.intersection(non_empty);
    assert(intersect.is_empty());
    
    std::cout << "✓ Empty sets tests passed" << std::endl;
}

int main() {
    std::cout << "Running rusty::BTreeSet tests..." << std::endl;
    std::cout << "================================" << std::endl;
    
    test_basic_operations();
    test_first_last();
    test_range_operations();
    test_set_operations();
    test_subset_superset();
    test_iteration();
    test_get_take_replace();
    test_extend_retain();
    test_drain();
    test_clone();
    test_from_vec();
    test_stress();
    test_custom_comparator();
    test_empty_sets();
    
    std::cout << "================================" << std::endl;
    std::cout << "✅ All BTreeSet tests passed!" << std::endl;
    
    return 0;
}