#include "../include/rusty/hashmap.hpp"
#include "../include/rusty/string.hpp"
#include <iostream>
#include <cassert>
#include <string>
#include <vector>

using namespace rusty;

void test_basic_operations() {
    std::cout << "Testing basic operations..." << std::endl;
    
    // Create empty hashmap
    HashMap<int, std::string> map;
    assert(map.is_empty());
    assert(map.len() == 0);
    
    // Insert some values
    map.insert(1, "one");
    map.insert(2, "two");
    map.insert(3, "three");
    
    assert(!map.is_empty());
    assert(map.len() == 3);
    
    // Get values
    auto val1 = map.get(1);
    assert(val1.is_some());
    assert(*val1.unwrap() == "one");
    
    auto val2 = map.get(2);
    assert(val2.is_some());
    assert(*val2.unwrap() == "two");
    
    // Get non-existent key
    auto val4 = map.get(4);
    assert(val4.is_none());
    
    // Contains key
    assert(map.contains_key(1));
    assert(map.contains_key(2));
    assert(map.contains_key(3));
    assert(!map.contains_key(4));
    
    std::cout << "✓ Basic operations tests passed" << std::endl;
}

void test_move_semantics() {
    std::cout << "Testing move semantics..." << std::endl;
    
    HashMap<int, std::string> map1;
    map1.insert(1, "one");
    map1.insert(2, "two");
    
    // Move constructor
    HashMap<int, std::string> map2 = std::move(map1);
    assert(map2.len() == 2);
    assert(map2.contains_key(1));
    assert(map2.contains_key(2));
    
    // Move assignment
    HashMap<int, std::string> map3;
    map3 = std::move(map2);
    assert(map3.len() == 2);
    assert(*map3.get(1).unwrap() == "one");
    
    std::cout << "✓ Move semantics tests passed" << std::endl;
}

void test_update_and_remove() {
    std::cout << "Testing update and remove..." << std::endl;
    
    HashMap<int, std::string> map;
    map.insert(1, "one");
    map.insert(2, "two");
    
    // Update existing key
    map.insert(1, "ONE");
    assert(map.len() == 2);
    assert(*map.get(1).unwrap() == "ONE");
    
    // Remove key
    auto removed = map.remove(1);
    assert(removed.is_some());
    assert(removed.unwrap() == "ONE");
    assert(map.len() == 1);
    assert(!map.contains_key(1));
    
    // Remove non-existent key
    auto removed2 = map.remove(10);
    assert(removed2.is_none());
    assert(map.len() == 1);
    
    // Clear
    map.clear();
    assert(map.is_empty());
    assert(map.len() == 0);
    
    std::cout << "✓ Update and remove tests passed" << std::endl;
}

void test_get_mut() {
    std::cout << "Testing get_mut..." << std::endl;
    
    HashMap<int, std::string> map;
    map.insert(1, "one");
    
    // Get mutable reference
    auto val_mut = map.get_mut(1);
    assert(val_mut.is_some());
    *val_mut.unwrap() = "ONE";
    
    // Verify change
    auto val = map.get(1);
    assert(*val.unwrap() == "ONE");
    
    // Get mutable for non-existent key
    auto val_mut2 = map.get_mut(2);
    assert(val_mut2.is_none());
    
    std::cout << "✓ get_mut tests passed" << std::endl;
}

void test_entry_api() {
    std::cout << "Testing entry API..." << std::endl;
    
    HashMap<int, int> map;
    
    // Entry creates default value if not exists
    map.entry(1) = 10;
    assert(map.get(1).is_some());
    assert(*map.get(1).unwrap() == 10);
    
    // Entry returns existing value
    map.entry(1) += 5;
    assert(*map.get(1).unwrap() == 15);
    
    // or_insert with default
    int& val = map.or_insert(2, 20);
    assert(val == 20);
    val += 5;
    assert(*map.get(2).unwrap() == 25);
    
    // or_insert doesn't override existing
    int& val2 = map.or_insert(2, 100);
    assert(val2 == 25);
    
    std::cout << "✓ Entry API tests passed" << std::endl;
}

void test_iteration() {
    std::cout << "Testing iteration..." << std::endl;
    
    HashMap<int, std::string> map;
    map.insert(1, "one");
    map.insert(2, "two");
    map.insert(3, "three");
    
    // Count iterations
    int count = 0;
    for (auto [key, value] : map) {
        count++;
        assert(key >= 1 && key <= 3);
        assert(!value.empty());
    }
    assert(count == 3);
    
    // Const iteration
    const HashMap<int, std::string>& const_map = map;
    count = 0;
    for (auto [key, value] : const_map) {
        count++;
        assert(key >= 1 && key <= 3);
    }
    assert(count == 3);
    
    std::cout << "✓ Iteration tests passed" << std::endl;
}

void test_keys_and_values() {
    std::cout << "Testing keys and values..." << std::endl;
    
    HashMap<int, std::string> map;
    map.insert(1, "one");
    map.insert(2, "two");
    map.insert(3, "three");
    
    // Get all keys
    auto keys = map.keys();
    assert(keys.size() == 3);
    std::sort(keys.begin(), keys.end());
    assert(keys[0] == 1);
    assert(keys[1] == 2);
    assert(keys[2] == 3);
    
    // Get all values
    auto values = map.values();
    assert(values.size() == 3);
    std::sort(values.begin(), values.end());
    assert(values[0] == "one");
    assert(values[1] == "three");
    assert(values[2] == "two");
    
    std::cout << "✓ Keys and values tests passed" << std::endl;
}

void test_extend() {
    std::cout << "Testing extend..." << std::endl;
    
    HashMap<int, std::string> map1;
    map1.insert(1, "one");
    map1.insert(2, "two");
    
    HashMap<int, std::string> map2;
    map2.insert(3, "three");
    map2.insert(4, "four");
    
    map1.extend(std::move(map2));
    assert(map1.len() == 4);
    assert(map1.contains_key(1));
    assert(map1.contains_key(2));
    assert(map1.contains_key(3));
    assert(map1.contains_key(4));
    
    std::cout << "✓ Extend tests passed" << std::endl;
}

void test_retain() {
    std::cout << "Testing retain..." << std::endl;
    
    HashMap<int, int> map;
    for (int i = 1; i <= 10; i++) {
        map.insert(i, i * 10);
    }
    
    // Keep only even keys
    map.retain([](const int& k, const int&) { return k % 2 == 0; });
    
    assert(map.len() == 5);
    assert(!map.contains_key(1));
    assert(map.contains_key(2));
    assert(!map.contains_key(3));
    assert(map.contains_key(4));
    
    std::cout << "✓ Retain tests passed" << std::endl;
}

void test_clone() {
    std::cout << "Testing clone..." << std::endl;
    
    HashMap<int, std::string> map1;
    map1.insert(1, "one");
    map1.insert(2, "two");
    
    HashMap<int, std::string> map2 = map1.clone();
    assert(map2.len() == 2);
    assert(*map2.get(1).unwrap() == "one");
    assert(*map2.get(2).unwrap() == "two");
    
    // Modify original, clone should be unchanged
    map1.insert(3, "three");
    assert(map1.len() == 3);
    assert(map2.len() == 2);
    
    std::cout << "✓ Clone tests passed" << std::endl;
}

void test_equality() {
    std::cout << "Testing equality..." << std::endl;
    
    HashMap<int, std::string> map1;
    map1.insert(1, "one");
    map1.insert(2, "two");
    
    HashMap<int, std::string> map2;
    map2.insert(2, "two");
    map2.insert(1, "one");
    
    assert(map1 == map2);
    
    map2.insert(3, "three");
    assert(map1 != map2);
    
    HashMap<int, std::string> map3;
    map3.insert(1, "ONE");
    map3.insert(2, "two");
    assert(map1 != map3);
    
    std::cout << "✓ Equality tests passed" << std::endl;
}

void test_with_capacity() {
    std::cout << "Testing with_capacity..." << std::endl;
    
    HashMap<int, int> map = HashMap<int, int>::with_capacity(100);
    assert(map.is_empty());
    assert(map.capacity() >= 100);
    
    // Insert should not trigger resize
    for (int i = 0; i < 50; i++) {
        map.insert(i, i * 2);
    }
    assert(map.len() == 50);
    
    std::cout << "✓ with_capacity tests passed" << std::endl;
}

void test_stress() {
    std::cout << "Testing stress (large number of operations)..." << std::endl;
    
    HashMap<int, int> map;
    const int N = 10000;
    
    // Insert many items
    for (int i = 0; i < N; i++) {
        map.insert(i, i * 2);
    }
    assert(map.len() == N);
    
    // Verify all items
    for (int i = 0; i < N; i++) {
        assert(map.contains_key(i));
        assert(*map.get(i).unwrap() == i * 2);
    }
    
    // Remove half
    for (int i = 0; i < N / 2; i++) {
        auto removed = map.remove(i * 2);
        assert(removed.is_some());
    }
    assert(map.len() == N / 2);
    
    // Verify remaining
    for (int i = 0; i < N; i++) {
        if (i % 2 == 0) {
            assert(!map.contains_key(i));
        } else {
            assert(map.contains_key(i));
        }
    }
    
    std::cout << "✓ Stress tests passed" << std::endl;
}

void test_with_string_keys() {
    std::cout << "Testing with string keys..." << std::endl;
    
    HashMap<std::string, int> map;
    map.insert("apple", 1);
    map.insert("banana", 2);
    map.insert("cherry", 3);
    
    assert(map.len() == 3);
    assert(*map.get("apple").unwrap() == 1);
    assert(*map.get("banana").unwrap() == 2);
    assert(*map.get("cherry").unwrap() == 3);
    
    // Test with longer strings
    std::string long_key = "this_is_a_very_long_key_to_test_hashing";
    map.insert(long_key, 42);
    assert(*map.get(long_key).unwrap() == 42);
    
    std::cout << "✓ String keys tests passed" << std::endl;
}

void test_with_rusty_string() {
    std::cout << "Testing with rusty::String..." << std::endl;
    
    HashMap<int, String> map;
    map.insert(1, String::from("one"));
    map.insert(2, String::from("two"));
    
    assert(map.len() == 2);
    auto val = map.get(1);
    assert(val.is_some());
    assert(*val.unwrap() == "one");
    
    // Update with longer string
    map.insert(1, String::from("ONE HUNDRED"));
    assert(*map.get(1).unwrap() == "ONE HUNDRED");
    
    std::cout << "✓ rusty::String tests passed" << std::endl;
}

void test_collision_handling() {
    std::cout << "Testing collision handling..." << std::endl;
    
    // Custom hash that causes collisions
    struct BadHash {
        size_t operator()(int x) const {
            return x % 10;  // Many collisions
        }
    };
    
    HashMap<int, int, BadHash> map;
    
    // Insert values that will collide
    for (int i = 0; i < 100; i++) {
        map.insert(i, i * 10);
        // Debug output
        if (map.len() != static_cast<size_t>(i + 1)) {
            std::cout << "Failed at i=" << i << ", map.len()=" << map.len() 
                     << ", expected=" << (i + 1) << std::endl;
            break;
        }
    }
    
    assert(map.len() == 100);
    
    // Verify all values are still accessible
    for (int i = 0; i < 100; i++) {
        assert(map.contains_key(i));
        assert(*map.get(i).unwrap() == i * 10);
    }
    
    std::cout << "✓ Collision handling tests passed" << std::endl;
}

int main() {
    std::cout << "Running rusty::HashMap tests..." << std::endl;
    std::cout << "================================" << std::endl;
    
    test_basic_operations();
    test_move_semantics();
    test_update_and_remove();
    test_get_mut();
    test_entry_api();
    test_iteration();
    test_keys_and_values();
    test_extend();
    test_retain();
    test_clone();
    test_equality();
    test_with_capacity();
    test_stress();
    test_with_string_keys();
    test_with_rusty_string();
    test_collision_handling();
    
    std::cout << "================================" << std::endl;
    std::cout << "✅ All HashMap tests passed!" << std::endl;
    
    return 0;
}