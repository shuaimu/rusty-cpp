#include "../include/rusty/btreemap.hpp"
#include "../include/rusty/vec.hpp"
#include "../include/rusty/string.hpp"
#include <iostream>
#include <cassert>
#include <string>
#include <vector>

using namespace rusty;

void test_basic_operations() {
    std::cout << "Testing basic operations..." << std::endl;
    
    BTreeMap<int, std::string> map;
    assert(map.is_empty());
    assert(map.len() == 0);
    
    // Insert in random order
    map.insert(3, "three");
    map.insert(1, "one");
    map.insert(4, "four");
    map.insert(2, "two");
    
    assert(!map.is_empty());
    assert(map.len() == 4);
    
    // Get values
    assert(*map.get(1).unwrap() == "one");
    assert(*map.get(2).unwrap() == "two");
    assert(*map.get(3).unwrap() == "three");
    assert(*map.get(4).unwrap() == "four");
    assert(map.get(5).is_none());
    
    // Contains key
    assert(map.contains_key(1));
    assert(map.contains_key(4));
    assert(!map.contains_key(5));
    
    // Keys are sorted
    auto keys = map.keys();
    assert(keys[0] == 1);
    assert(keys[1] == 2);
    assert(keys[2] == 3);
    assert(keys[3] == 4);
    
    std::cout << "✓ Basic operations tests passed" << std::endl;
}

void test_move_semantics() {
    std::cout << "Testing move semantics..." << std::endl;
    
    BTreeMap<int, int> map1;
    map1.insert(1, 10);
    map1.insert(2, 20);
    
    // Move constructor
    BTreeMap<int, int> map2 = std::move(map1);
    assert(map2.len() == 2);
    assert(*map2.get(1).unwrap() == 10);
    
    // Move assignment
    BTreeMap<int, int> map3;
    map3 = std::move(map2);
    assert(map3.len() == 2);
    
    std::cout << "✓ Move semantics tests passed" << std::endl;
}

void test_update_and_remove() {
    std::cout << "Testing update and remove..." << std::endl;
    
    BTreeMap<int, std::string> map;
    map.insert(1, "one");
    map.insert(2, "two");
    map.insert(3, "three");
    
    // Update existing
    map.insert(2, "TWO");
    assert(*map.get(2).unwrap() == "TWO");
    assert(map.len() == 3);
    
    // Remove
    auto removed = map.remove(2);
    assert(removed.is_some());
    assert(removed.unwrap() == "TWO");
    assert(map.len() == 2);
    assert(!map.contains_key(2));
    
    // Remove entry (key and value)
    auto entry = map.remove_entry(1);
    assert(entry.is_some());
    auto [key, value] = entry.unwrap();
    assert(key == 1);
    assert(value == "one");
    
    // Clear
    map.clear();
    assert(map.is_empty());
    
    std::cout << "✓ Update and remove tests passed" << std::endl;
}

void test_get_operations() {
    std::cout << "Testing get operations..." << std::endl;
    
    BTreeMap<int, std::string> map;
    map.insert(1, "one");
    map.insert(2, "two");
    map.insert(3, "three");
    
    // Get mutable
    auto val_mut = map.get_mut(2);
    assert(val_mut.is_some());
    *val_mut.unwrap() = "TWO";
    assert(*map.get(2).unwrap() == "TWO");
    
    // Get key-value pair
    auto kv = map.get_key_value(2);
    assert(kv.is_some());
    auto [key_ptr, val_ptr] = kv.unwrap();
    assert(*key_ptr == 2);
    assert(*val_ptr == "TWO");
    
    std::cout << "✓ Get operations tests passed" << std::endl;
}

void test_first_last() {
    std::cout << "Testing first/last operations..." << std::endl;
    
    BTreeMap<int, std::string> map;
    
    // Empty map
    assert(map.first_key_value().is_none());
    assert(map.last_key_value().is_none());
    
    map.insert(3, "three");
    map.insert(1, "one");
    map.insert(5, "five");
    map.insert(2, "two");
    map.insert(4, "four");
    
    // First (minimum)
    auto first = map.first_key_value();
    assert(first.is_some());
    auto [first_key, first_val] = first.unwrap();
    assert(*first_key == 1);
    assert(*first_val == "one");
    
    // Last (maximum)
    auto last = map.last_key_value();
    assert(last.is_some());
    auto [last_key, last_val] = last.unwrap();
    assert(*last_key == 5);
    assert(*last_val == "five");
    
    // Pop first
    auto popped_first = map.pop_first();
    assert(popped_first.is_some());
    auto [pf_key, pf_val] = popped_first.unwrap();
    assert(pf_key == 1);
    assert(pf_val == "one");
    assert(map.len() == 4);
    assert(!map.contains_key(1));
    
    // Pop last
    auto popped_last = map.pop_last();
    assert(popped_last.is_some());
    auto [pl_key, pl_val] = popped_last.unwrap();
    assert(pl_key == 5);
    assert(pl_val == "five");
    assert(map.len() == 3);
    assert(!map.contains_key(5));
    
    std::cout << "✓ First/last operations tests passed" << std::endl;
}

void test_range_operations() {
    std::cout << "Testing range operations..." << std::endl;
    
    BTreeMap<int, std::string> map;
    for (int i = 1; i <= 10; i++) {
        map.insert(i, std::to_string(i));
    }
    
    // Range query
    auto range = map.range(3, 7);
    assert(range.size() == 5);  // 3, 4, 5, 6, 7
    assert(range[0].first == 3);
    assert(range[4].first == 7);
    
    // Split off
    BTreeMap<int, std::string> upper = map.split_off(6);
    assert(map.len() == 5);  // 1-5
    assert(upper.len() == 5); // 6-10
    
    assert(map.contains_key(5));
    assert(!map.contains_key(6));
    assert(upper.contains_key(6));
    assert(upper.contains_key(10));
    
    // Append (keys in upper > keys in map)
    map.append(std::move(upper));
    assert(map.len() == 10);
    assert(upper.len() == 0);
    
    std::cout << "✓ Range operations tests passed" << std::endl;
}

void test_entry_api() {
    std::cout << "Testing entry API..." << std::endl;
    
    BTreeMap<std::string, int> map;
    
    // Entry creates default if not exists
    map.entry("hello") = 1;
    assert(map.get("hello").is_some());
    assert(*map.get("hello").unwrap() == 1);
    
    // Entry returns existing
    map.entry("hello") += 10;
    assert(*map.get("hello").unwrap() == 11);
    
    // or_insert
    int& val = map.or_insert("world", 42);
    assert(val == 42);
    val += 8;
    assert(*map.get("world").unwrap() == 50);
    
    // or_insert doesn't override
    int& val2 = map.or_insert("world", 100);
    assert(val2 == 50);
    
    std::cout << "✓ Entry API tests passed" << std::endl;
}

void test_iteration() {
    std::cout << "Testing iteration..." << std::endl;
    
    BTreeMap<int, std::string> map;
    map.insert(3, "three");
    map.insert(1, "one");
    map.insert(2, "two");
    
    // Iteration is in sorted order
    int prev_key = 0;
    for (const auto& [key, value] : map) {
        assert(key > prev_key);
        prev_key = key;
        assert(!value.empty());
    }
    
    // Mutable iteration
    for (auto& [key, value] : map) {
        value = "modified";
    }
    
    for (const auto& [key, value] : map) {
        assert(value == "modified");
    }
    
    std::cout << "✓ Iteration tests passed" << std::endl;
}

void test_keys_and_values() {
    std::cout << "Testing keys and values..." << std::endl;
    
    BTreeMap<int, std::string> map;
    map.insert(2, "two");
    map.insert(1, "one");
    map.insert(3, "three");
    
    // Keys are sorted
    auto keys = map.keys();
    assert(keys.size() == 3);
    assert(keys[0] == 1);
    assert(keys[1] == 2);
    assert(keys[2] == 3);
    
    // Values in key order
    auto values = map.values();
    assert(values.size() == 3);
    assert(values[0] == "one");
    assert(values[1] == "two");
    assert(values[2] == "three");
    
    std::cout << "✓ Keys and values tests passed" << std::endl;
}

void test_extend_and_retain() {
    std::cout << "Testing extend and retain..." << std::endl;
    
    BTreeMap<int, int> map1;
    map1.insert(1, 10);
    map1.insert(3, 30);
    
    BTreeMap<int, int> map2;
    map2.insert(2, 20);
    map2.insert(4, 40);
    map2.insert(3, 300);  // Will override
    
    // Extend
    map1.extend(std::move(map2));
    assert(map1.len() == 4);
    assert(*map1.get(2).unwrap() == 20);
    assert(*map1.get(3).unwrap() == 300);  // Overridden
    
    // Retain
    map1.retain([](const int& k, const int&) { return k % 2 == 0; });
    assert(map1.len() == 2);
    assert(map1.contains_key(2));
    assert(map1.contains_key(4));
    assert(!map1.contains_key(1));
    assert(!map1.contains_key(3));
    
    std::cout << "✓ Extend and retain tests passed" << std::endl;
}

void test_clone() {
    std::cout << "Testing clone..." << std::endl;
    
    BTreeMap<int, std::string> map1;
    map1.insert(1, "one");
    map1.insert(2, "two");
    
    BTreeMap<int, std::string> map2 = map1.clone();
    assert(map2.len() == 2);
    assert(*map2.get(1).unwrap() == "one");
    
    // Modify original
    map1.insert(3, "three");
    assert(map1.len() == 3);
    assert(map2.len() == 2);  // Clone unchanged
    
    std::cout << "✓ Clone tests passed" << std::endl;
}

void test_equality() {
    std::cout << "Testing equality..." << std::endl;
    
    BTreeMap<int, std::string> map1;
    map1.insert(1, "one");
    map1.insert(2, "two");
    
    BTreeMap<int, std::string> map2;
    map2.insert(2, "two");
    map2.insert(1, "one");  // Different insertion order
    
    assert(map1 == map2);  // Still equal (sorted)
    
    map2.insert(3, "three");
    assert(map1 != map2);
    
    BTreeMap<int, std::string> map3;
    map3.insert(1, "ONE");
    map3.insert(2, "two");
    assert(map1 != map3);  // Different values
    
    std::cout << "✓ Equality tests passed" << std::endl;
}

void test_from_vec() {
    std::cout << "Testing from_vec..." << std::endl;
    
    Vec<std::pair<int, std::string>> vec = Vec<std::pair<int, std::string>>::new_();
    vec.push(std::make_pair(3, std::string("three")));
    vec.push(std::make_pair(1, std::string("one")));
    vec.push(std::make_pair(2, std::string("two")));
    vec.push(std::make_pair(2, std::string("TWO")));  // Duplicate key
    
    BTreeMap<int, std::string> map = btreemap_from_vec(std::move(vec));
    assert(map.len() == 3);
    assert(*map.get(1).unwrap() == "one");
    assert(*map.get(2).unwrap() == "TWO");  // Last value wins
    assert(*map.get(3).unwrap() == "three");
    
    // Verify sorted
    auto keys = map.keys();
    assert(keys[0] == 1);
    assert(keys[1] == 2);
    assert(keys[2] == 3);
    
    std::cout << "✓ from_vec tests passed" << std::endl;
}

void test_stress() {
    std::cout << "Testing stress..." << std::endl;
    
    BTreeMap<int, int> map;
    const int N = 10000;
    
    // Insert in random order
    for (int i = N - 1; i >= 0; i--) {
        map.insert(i, i * 10);
    }
    assert(map.len() == N);
    
    // Verify all present and sorted
    auto keys = map.keys();
    for (int i = 0; i < N; i++) {
        assert(keys[i] == i);
        assert(*map.get(i).unwrap() == i * 10);
    }
    
    // Remove half
    for (int i = 0; i < N; i += 2) {
        assert(map.remove(i).is_some());
    }
    assert(map.len() == N / 2);
    
    // Verify pattern
    for (int i = 0; i < N; i++) {
        if (i % 2 == 0) {
            assert(!map.contains_key(i));
        } else {
            assert(map.contains_key(i));
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
    
    BTreeMap<int, std::string, ReverseCompare> map;
    map.insert(1, "one");
    map.insert(3, "three");
    map.insert(2, "two");
    
    // Keys should be in reverse order
    auto keys = map.keys();
    assert(keys[0] == 3);
    assert(keys[1] == 2);
    assert(keys[2] == 1);
    
    std::cout << "✓ Custom comparator tests passed" << std::endl;
}

int main() {
    std::cout << "Running rusty::BTreeMap tests..." << std::endl;
    std::cout << "================================" << std::endl;
    
    test_basic_operations();
    test_move_semantics();
    test_update_and_remove();
    test_get_operations();
    test_first_last();
    test_range_operations();
    test_entry_api();
    test_iteration();
    test_keys_and_values();
    test_extend_and_retain();
    test_clone();
    test_equality();
    test_from_vec();
    test_stress();
    test_custom_comparator();
    
    std::cout << "================================" << std::endl;
    std::cout << "✅ All BTreeMap tests passed!" << std::endl;
    
    return 0;
}