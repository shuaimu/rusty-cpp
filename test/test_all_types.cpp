#include "../include/rusty/rusty.hpp"
#include <iostream>
#include <cassert>

using namespace rusty;

void test_all_types_together() {
    std::cout << "Testing all rusty types working together..." << std::endl;
    
    // Create a HashMap with String keys and Vec<int> values
    HashMap<String, Vec<int>> map;
    
    String key1 = String::from("numbers");
    Vec<int> vec1 = Vec<int>::new_();
    vec1.push(1);
    vec1.push(2);
    vec1.push(3);
    map.insert(std::move(key1), std::move(vec1));
    
    String key2 = String::from("more_numbers");
    Vec<int> vec2 = Vec<int>::new_();
    vec2.push(10);
    vec2.push(20);
    map.insert(std::move(key2), std::move(vec2));
    
    // Create a HashSet of Strings
    HashSet<String> set;
    set.insert(String::from("apple"));
    set.insert(String::from("banana"));
    set.insert(String::from("cherry"));
    
    // Create a BTreeMap with int keys and String values
    BTreeMap<int, String> btree;
    btree.insert(3, String::from("three"));
    btree.insert(1, String::from("one"));
    btree.insert(2, String::from("two"));
    
    // Create a BTreeSet of ints
    BTreeSet<int> btree_set;
    btree_set.insert(5);
    btree_set.insert(1);
    btree_set.insert(3);
    btree_set.insert(2);
    btree_set.insert(4);
    
    // Test accessing the HashMap
    String search_key = String::from("numbers");
    auto vec_opt = map.get(search_key);
    assert(vec_opt.is_some());
    auto vec_ptr = vec_opt.unwrap();
    assert(vec_ptr->len() == 3);
    assert((*vec_ptr)[0] == 1);
    
    // Test the HashSet
    assert(set.contains(String::from("apple")));
    assert(!set.contains(String::from("orange")));
    assert(set.len() == 3);
    
    // Test the BTreeMap (should be sorted)
    auto keys = btree.keys();
    assert(keys.len() == 3);
    assert(keys[0] == 1);
    assert(keys[1] == 2);
    assert(keys[2] == 3);
    
    // Test the BTreeSet (should be sorted)
    auto sorted = btree_set.to_vec();
    assert(sorted.len() == 5);
    for (size_t i = 0; i < sorted.len(); i++) {
        assert(sorted[i] == static_cast<int>(i + 1));
    }
    
    // Create a Vec of Strings
    Vec<String> string_vec = Vec<String>::new_();
    string_vec.push(String::from("hello"));
    string_vec.push(String::from("world"));
    string_vec.push(String::from("from"));
    string_vec.push(String::from("rusty"));
    
    // Use Vec with HashSet
    HashSet<String> set_from_vec = hashset_from_vec(std::move(string_vec));
    assert(set_from_vec.len() == 4);
    assert(set_from_vec.contains(String::from("hello")));
    assert(set_from_vec.contains(String::from("rusty")));
    
    // Test move semantics - this should work without copying
    HashMap<String, BTreeSet<int>> complex_map;
    String key = String::from("sorted_numbers");
    complex_map.insert(std::move(key), std::move(btree_set));
    
    // Verify the complex map
    String complex_key = String::from("sorted_numbers");
    auto bset_opt = complex_map.get(complex_key);
    assert(bset_opt.is_some());
    assert(bset_opt.unwrap()->len() == 5);
    
    std::cout << "✓ All types work together correctly!" << std::endl;
}

void test_nested_containers() {
    std::cout << "Testing nested containers..." << std::endl;
    
    // Vec of Vecs
    Vec<Vec<int>> matrix = Vec<Vec<int>>::new_();
    for (int i = 0; i < 3; i++) {
        Vec<int> row = Vec<int>::new_();
        for (int j = 0; j < 3; j++) {
            row.push(i * 3 + j);
        }
        matrix.push(std::move(row));
    }
    assert(matrix.len() == 3);
    assert(matrix[0][0] == 0);
    assert(matrix[2][2] == 8);
    
    // HashMap of HashMaps
    HashMap<String, HashMap<String, int>> nested_map;
    
    HashMap<String, int> inner1;
    inner1.insert(String::from("x"), 10);
    inner1.insert(String::from("y"), 20);
    nested_map.insert(String::from("point1"), std::move(inner1));
    
    HashMap<String, int> inner2;
    inner2.insert(String::from("x"), 30);
    inner2.insert(String::from("y"), 40);
    nested_map.insert(String::from("point2"), std::move(inner2));
    
    String point1_key = String::from("point1");
    auto point1 = nested_map.get(point1_key);
    assert(point1.is_some());
    String x_key = String::from("x");
    auto x = point1.unwrap()->get(x_key);
    assert(x.is_some());
    assert(*x.unwrap() == 10);
    
    std::cout << "✓ Nested containers work correctly!" << std::endl;
}

void test_memory_safety() {
    std::cout << "Testing memory safety with move semantics..." << std::endl;
    
    // Test that moved-from objects are properly handled
    String s1 = String::from("hello");
    String s2 = std::move(s1);
    // s1 is now moved-from, s2 owns the data
    assert(s2.len() == 5);
    
    // Test Vec move
    Vec<int> v1 = Vec<int>::new_();
    v1.push(1);
    v1.push(2);
    Vec<int> v2 = std::move(v1);
    // v1 is now moved-from, v2 owns the data
    assert(v2.len() == 2);
    
    // Test HashMap move
    HashMap<int, String> m1;
    m1.insert(1, String::from("one"));
    HashMap<int, String> m2 = std::move(m1);
    // m1 is now moved-from, m2 owns the data
    assert(m2.len() == 1);
    
    // Test that containers properly manage memory of contained objects
    {
        Vec<String> vec_of_strings = Vec<String>::new_();
        for (int i = 0; i < 100; i++) {
            vec_of_strings.push(String::from("test string"));
        }
        // All strings will be properly destroyed when vec goes out of scope
    }
    
    std::cout << "✓ Memory safety with move semantics verified!" << std::endl;
}

int main() {
    std::cout << "========================================" << std::endl;
    std::cout << "Testing all rusty types integration" << std::endl;
    std::cout << "========================================" << std::endl;
    
    test_all_types_together();
    test_nested_containers();
    test_memory_safety();
    
    std::cout << "========================================" << std::endl;
    std::cout << "✅ All integration tests passed!" << std::endl;
    std::cout << "========================================" << std::endl;
    
    return 0;
}