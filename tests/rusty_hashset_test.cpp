#include "../include/rusty/hashset.hpp"
#include "../include/rusty/string.hpp"
#include <iostream>
#include <cassert>
#include <string>
#include <vector>
#include <algorithm>

using namespace rusty;

void test_basic_operations() {
    std::cout << "Testing basic operations..." << std::endl;
    
    HashSet<int> set;
    assert(set.is_empty());
    assert(set.len() == 0);
    
    // Insert elements
    assert(set.insert(1) == true);  // New element
    assert(set.insert(2) == true);
    assert(set.insert(3) == true);
    assert(set.insert(1) == false); // Already exists
    
    assert(!set.is_empty());
    assert(set.len() == 3);
    
    // Contains
    assert(set.contains(1));
    assert(set.contains(2));
    assert(set.contains(3));
    assert(!set.contains(4));
    
    // Remove
    assert(set.remove(2) == true);
    assert(set.remove(2) == false);  // Already removed
    assert(set.len() == 2);
    assert(!set.contains(2));
    
    // Clear
    set.clear();
    assert(set.is_empty());
    
    std::cout << "✓ Basic operations tests passed" << std::endl;
}

void test_move_semantics() {
    std::cout << "Testing move semantics..." << std::endl;
    
    HashSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    
    // Move constructor
    HashSet<int> set2 = std::move(set1);
    assert(set2.len() == 2);
    assert(set2.contains(1));
    assert(set2.contains(2));
    
    // Move assignment
    HashSet<int> set3;
    set3 = std::move(set2);
    assert(set3.len() == 2);
    
    std::cout << "✓ Move semantics tests passed" << std::endl;
}

void test_set_operations() {
    std::cout << "Testing set operations..." << std::endl;
    
    HashSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);
    
    HashSet<int> set2;
    set2.insert(2);
    set2.insert(3);
    set2.insert(4);
    
    // Union
    HashSet<int> union_set = set1.union_with(set2);
    assert(union_set.len() == 4);
    assert(union_set.contains(1));
    assert(union_set.contains(2));
    assert(union_set.contains(3));
    assert(union_set.contains(4));
    
    // Intersection
    HashSet<int> intersect = set1.intersection(set2);
    assert(intersect.len() == 2);
    assert(intersect.contains(2));
    assert(intersect.contains(3));
    assert(!intersect.contains(1));
    assert(!intersect.contains(4));
    
    // Difference
    HashSet<int> diff = set1.difference(set2);
    assert(diff.len() == 1);
    assert(diff.contains(1));
    assert(!diff.contains(2));
    assert(!diff.contains(3));
    
    // Symmetric difference
    HashSet<int> sym_diff = set1.symmetric_difference(set2);
    assert(sym_diff.len() == 2);
    assert(sym_diff.contains(1));
    assert(sym_diff.contains(4));
    assert(!sym_diff.contains(2));
    assert(!sym_diff.contains(3));
    
    std::cout << "✓ Set operations tests passed" << std::endl;
}

void test_subset_superset() {
    std::cout << "Testing subset/superset..." << std::endl;
    
    HashSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    
    HashSet<int> set2;
    set2.insert(1);
    set2.insert(2);
    set2.insert(3);
    
    HashSet<int> set3;
    set3.insert(4);
    set3.insert(5);
    
    // Subset
    assert(set1.is_subset(set2));
    assert(!set2.is_subset(set1));
    assert(!set1.is_subset(set3));
    
    // Superset
    assert(set2.is_superset(set1));
    assert(!set1.is_superset(set2));
    assert(!set2.is_superset(set3));
    
    // Disjoint
    assert(!set1.is_disjoint(set2));
    assert(set1.is_disjoint(set3));
    assert(set2.is_disjoint(set3));
    
    // Equal sets
    HashSet<int> set4;
    set4.insert(2);
    set4.insert(1);  // Different order
    assert(set1 == set4);
    assert(set1 != set2);
    
    std::cout << "✓ Subset/superset tests passed" << std::endl;
}

void test_iteration() {
    std::cout << "Testing iteration..." << std::endl;
    
    HashSet<int> set;
    set.insert(1);
    set.insert(2);
    set.insert(3);
    
    int count = 0;
    int sum = 0;
    for (const int& val : set) {
        count++;
        sum += val;
        assert(val >= 1 && val <= 3);
    }
    assert(count == 3);
    assert(sum == 6);
    
    // Const iteration
    const HashSet<int>& const_set = set;
    count = 0;
    for (const int& val : const_set) {
        count++;
    }
    assert(count == 3);
    
    std::cout << "✓ Iteration tests passed" << std::endl;
}

void test_get_and_take() {
    std::cout << "Testing get and take..." << std::endl;
    
    HashSet<int> set;
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
    
    auto taken2 = set.take(4);
    assert(taken2.is_none());
    
    std::cout << "✓ Get and take tests passed" << std::endl;
}

void test_replace() {
    std::cout << "Testing replace..." << std::endl;
    
    HashSet<std::string> set;
    set.insert("hello");
    set.insert("world");
    
    // Replace existing
    auto old = set.replace("hello");
    assert(old.is_some());
    assert(old.unwrap() == "hello");
    assert(set.contains("hello"));
    
    // Replace non-existing
    auto old2 = set.replace("new");
    assert(old2.is_none());
    assert(set.contains("new"));
    assert(set.len() == 3);
    
    std::cout << "✓ Replace tests passed" << std::endl;
}

void test_extend_and_retain() {
    std::cout << "Testing extend and retain..." << std::endl;
    
    HashSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    
    HashSet<int> set2;
    set2.insert(3);
    set2.insert(4);
    
    // Extend
    set1.extend(std::move(set2));
    assert(set1.len() == 4);
    assert(set1.contains(1));
    assert(set1.contains(2));
    assert(set1.contains(3));
    assert(set1.contains(4));
    
    // Retain
    set1.retain([](const int& x) { return x % 2 == 0; });
    assert(set1.len() == 2);
    assert(!set1.contains(1));
    assert(set1.contains(2));
    assert(!set1.contains(3));
    assert(set1.contains(4));
    
    std::cout << "✓ Extend and retain tests passed" << std::endl;
}

void test_drain_and_to_vec() {
    std::cout << "Testing drain and to_vec..." << std::endl;
    
    HashSet<int> set;
    set.insert(3);
    set.insert(1);
    set.insert(2);
    
    // to_vec
    auto vec = set.to_vec();
    assert(vec.len() == 3);
    // Convert to std::vector for sorting
    std::vector<int> std_vec;
    for (size_t i = 0; i < vec.len(); i++) {
        std_vec.push_back(vec[i]);
    }
    std::sort(std_vec.begin(), std_vec.end());
    assert(std_vec[0] == 1);
    assert(std_vec[1] == 2);
    assert(std_vec[2] == 3);
    assert(set.len() == 3);  // Set unchanged
    
    // drain
    auto drained = set.drain();
    assert(drained.len() == 3);
    assert(set.is_empty());  // Set is now empty
    std::vector<int> drained_std;
    for (size_t i = 0; i < drained.len(); i++) {
        drained_std.push_back(drained[i]);
    }
    std::sort(drained_std.begin(), drained_std.end());
    assert(drained_std[0] == 1);
    assert(drained_std[1] == 2);
    assert(drained_std[2] == 3);
    
    std::cout << "✓ Drain and to_vec tests passed" << std::endl;
}

void test_clone() {
    std::cout << "Testing clone..." << std::endl;
    
    HashSet<int> set1;
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);
    
    HashSet<int> set2 = set1.clone();
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
    
    // Test with std::vector (commented out since hashset_from_std_vec is now optional)
    // std::vector<int> std_vec = {1, 2, 3, 2, 1};  // Duplicates
    // HashSet<int> set1 = hashset_from_std_vec(std::move(std_vec));
    // assert(set1.len() == 3);  // Duplicates removed
    // assert(set1.contains(1));
    // assert(set1.contains(2));
    // assert(set1.contains(3));
    
    // Test with rusty::Vec
    Vec<int> vec = Vec<int>::make();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    vec.push(2);
    vec.push(1);
    HashSet<int> set2 = hashset_from_vec(std::move(vec));
    
    assert(set2.len() == 3);  // Duplicates removed
    assert(set2.contains(1));
    assert(set2.contains(2));
    assert(set2.contains(3));
    
    std::cout << "✓ from_vec tests passed" << std::endl;
}

void test_with_capacity() {
    std::cout << "Testing with_capacity..." << std::endl;
    
    HashSet<int> set = HashSet<int>::with_capacity(100);
    assert(set.is_empty());
    assert(set.capacity() >= 100);
    
    for (int i = 0; i < 50; i++) {
        set.insert(i);
    }
    assert(set.len() == 50);
    
    std::cout << "✓ with_capacity tests passed" << std::endl;
}

void test_stress() {
    std::cout << "Testing stress..." << std::endl;
    
    HashSet<int> set;
    const int N = 10000;
    
    // Insert many
    for (int i = 0; i < N; i++) {
        set.insert(i);
    }
    assert(set.len() == N);
    
    // Verify all present
    for (int i = 0; i < N; i++) {
        assert(set.contains(i));
    }
    
    // Remove half
    for (int i = 0; i < N / 2; i++) {
        assert(set.remove(i * 2));
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

void test_with_strings() {
    std::cout << "Testing with strings..." << std::endl;
    
    HashSet<std::string> set;
    set.insert("apple");
    set.insert("banana");
    set.insert("cherry");
    
    assert(set.len() == 3);
    assert(set.contains("apple"));
    assert(set.contains("banana"));
    assert(set.contains("cherry"));
    assert(!set.contains("date"));
    
    set.remove("banana");
    assert(set.len() == 2);
    assert(!set.contains("banana"));
    
    std::cout << "✓ String tests passed" << std::endl;
}

void test_empty_sets() {
    std::cout << "Testing empty sets..." << std::endl;
    
    HashSet<int> empty1;
    HashSet<int> empty2;
    HashSet<int> non_empty;
    non_empty.insert(1);
    
    // Operations with empty sets
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
    std::cout << "Running rusty::HashSet tests..." << std::endl;
    std::cout << "================================" << std::endl;
    
    test_basic_operations();
    test_move_semantics();
    test_set_operations();
    test_subset_superset();
    test_iteration();
    test_get_and_take();
    test_replace();
    test_extend_and_retain();
    test_drain_and_to_vec();
    test_clone();
    test_from_vec();
    test_with_capacity();
    test_stress();
    test_with_strings();
    test_empty_sets();
    
    std::cout << "================================" << std::endl;
    std::cout << "✅ All HashSet tests passed!" << std::endl;
    
    return 0;
}