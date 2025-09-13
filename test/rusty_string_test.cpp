#include "../include/rusty/string.hpp"
#include <iostream>
#include <cassert>
#include <vector>

using namespace rusty;

void test_basic_construction() {
    std::cout << "Testing basic construction..." << std::endl;
    
    // Default construction
    String s1;
    assert(s1.len() == 0);
    assert(s1.is_empty());
    
    // From C string
    String s2 = String::from("Hello");
    assert(s2.len() == 5);
    assert(!s2.is_empty());
    assert(s2 == "Hello");
    
    // From std::string
    std::string std_str = "World";
    String s3 = String::from(std_str);
    assert(s3.len() == 5);
    assert(s3 == "World");
    
    // With capacity
    String s4 = String::with_capacity(100);
    assert(s4.is_empty());
    assert(s4.capacity() >= 100);
    
    std::cout << "✓ Basic construction tests passed" << std::endl;
}

void test_move_semantics() {
    std::cout << "Testing move semantics..." << std::endl;
    
    String s1 = String::from("Move me");
    assert(s1.len() == 7);
    
    String s2 = std::move(s1);
    assert(s2.len() == 7);
    assert(s2 == "Move me");
    // s1 is now moved-from, shouldn't use it
    
    String s3;
    s3 = std::move(s2);
    assert(s3.len() == 7);
    assert(s3 == "Move me");
    
    std::cout << "✓ Move semantics tests passed" << std::endl;
}

void test_push_and_pop() {
    std::cout << "Testing push and pop..." << std::endl;
    
    String s;
    s.push('H');
    s.push('e');
    s.push('l');
    s.push('l');
    s.push('o');
    assert(s == "Hello");
    assert(s.len() == 5);
    
    char c = s.pop();
    assert(c == 'o');
    assert(s == "Hell");
    assert(s.len() == 4);
    
    s.push_str(" World");
    assert(s == "Hell World");
    assert(s.len() == 10);
    
    String s2 = String::from("!");
    s.push_str(s2);
    assert(s == "Hell World!");
    
    std::cout << "✓ Push and pop tests passed" << std::endl;
}

void test_string_operations() {
    std::cout << "Testing string operations..." << std::endl;
    
    // Concatenation
    String s1 = String::from("Hello");
    String s2 = String::from(" World");
    String s3 = s1 + s2;
    assert(s3 == "Hello World");
    assert(s1 == "Hello"); // s1 not consumed
    assert(s2 == " World"); // s2 not consumed
    
    // Append
    String s4 = String::from("Foo");
    s4 += "Bar";
    assert(s4 == "FooBar");
    s4 += '!';
    assert(s4 == "FooBar!");
    
    // Clone
    String s5 = String::from("Original");
    String s6 = s5.clone();
    assert(s5 == s6);
    assert(s5 == "Original");
    assert(s6 == "Original");
    
    std::cout << "✓ String operation tests passed" << std::endl;
}

void test_string_manipulation() {
    std::cout << "Testing string manipulation..." << std::endl;
    
    // Insert
    String s1 = String::from("Heo");
    s1.insert(2, "ll");
    assert(s1 == "Hello");
    
    s1.insert(0, "Oh ");
    assert(s1 == "Oh Hello");
    
    s1.insert(s1.len(), "!");
    assert(s1 == "Oh Hello!");
    
    // Drain (remove range)
    String s2 = String::from("Hello World");
    s2.drain(5, 11);
    assert(s2 == "Hello");
    
    s2.drain(0, 2);
    assert(s2 == "llo");
    
    // Truncate
    String s3 = String::from("Too long string");
    s3.truncate(8);
    assert(s3 == "Too long");
    assert(s3.len() == 8);
    
    // Clear
    s3.clear();
    assert(s3.is_empty());
    assert(s3.len() == 0);
    
    std::cout << "✓ String manipulation tests passed" << std::endl;
}

void test_string_search() {
    std::cout << "Testing string search..." << std::endl;
    
    String s = String::from("Hello World Hello");
    
    // Contains
    assert(s.contains("World"));
    assert(s.contains("Hello"));
    assert(!s.contains("Goodbye"));
    
    // Starts with / ends with
    assert(s.starts_with("Hello"));
    assert(!s.starts_with("World"));
    assert(s.ends_with("Hello"));
    assert(!s.ends_with("World"));
    
    // Find
    size_t pos = s.find("World");
    assert(pos == 6);
    
    pos = s.find("Hello");
    assert(pos == 0); // Finds first occurrence
    
    pos = s.find("NotFound");
    assert(pos == static_cast<size_t>(-1));
    
    std::cout << "✓ String search tests passed" << std::endl;
}

void test_string_transform() {
    std::cout << "Testing string transformations..." << std::endl;
    
    // Replace
    String s1 = String::from("Hello World Hello");
    String s2 = s1.replace("Hello", "Hi");
    assert(s2 == "Hi World Hi");
    assert(s1 == "Hello World Hello"); // Original unchanged
    
    // Trim
    String s3 = String::from("  Hello World  \t\n");
    String s4 = s3.trim();
    assert(s4 == "Hello World");
    assert(s3 == "  Hello World  \t\n"); // Original unchanged
    
    // Case conversion
    String s5 = String::from("Hello World");
    String s6 = s5.to_uppercase();
    assert(s6 == "HELLO WORLD");
    
    String s7 = s5.to_lowercase();
    assert(s7 == "hello world");
    
    assert(s5 == "Hello World"); // Original unchanged
    
    std::cout << "✓ String transformation tests passed" << std::endl;
}

void test_string_split() {
    std::cout << "Testing string split..." << std::endl;
    
    String s = String::from("one,two,three,four");
    std::vector<String> parts = s.split(',');
    
    assert(parts.size() == 4);
    assert(parts[0] == "one");
    assert(parts[1] == "two");
    assert(parts[2] == "three");
    assert(parts[3] == "four");
    
    // Empty parts
    String s2 = String::from("a,,b");
    std::vector<String> parts2 = s2.split(',');
    assert(parts2.size() == 3);
    assert(parts2[0] == "a");
    assert(parts2[1].is_empty());
    assert(parts2[2] == "b");
    
    // No delimiter found
    String s3 = String::from("hello");
    std::vector<String> parts3 = s3.split(',');
    assert(parts3.size() == 1);
    assert(parts3[0] == "hello");
    
    std::cout << "✓ String split tests passed" << std::endl;
}

void test_string_access() {
    std::cout << "Testing string access..." << std::endl;
    
    String s = String::from("Hello");
    
    // Index access
    assert(s[0] == 'H');
    assert(s[4] == 'o');
    
    // Mutable access
    s[0] = 'J';
    assert(s == "Jello");
    
    // C string access
    const char* cstr = s.c_str();
    assert(std::strcmp(cstr, "Jello") == 0);
    
    // String view
    std::string_view sv = s.as_str();
    assert(sv == "Jello");
    assert(sv.length() == 5);
    
    // Slice
    std::string_view slice = s.slice(1, 4);
    assert(slice == "ell");
    
    // Iterators
    String s2;
    for (char c : s) {
        s2.push(c);
    }
    assert(s2 == s);
    
    std::cout << "✓ String access tests passed" << std::endl;
}

void test_str_type() {
    std::cout << "Testing str type..." << std::endl;
    
    // Create from various sources
    str s1("Hello");
    assert(s1.len() == 5);
    assert(!s1.is_empty());
    
    String owned = String::from("World");
    str s2(owned);
    assert(s2.len() == 5);
    
    // Convert to String
    String owned2 = s1.to_string();
    assert(owned2 == "Hello");
    
    // Access
    assert(s1[0] == 'H');
    assert(s1[4] == 'o');
    
    // Comparison
    str s3("Hello");
    assert(s1 == s3);
    assert(s1 != s2);
    
    std::cout << "✓ str type tests passed" << std::endl;
}

void test_edge_cases() {
    std::cout << "Testing edge cases..." << std::endl;
    
    // Empty string operations
    String empty;
    assert(empty.is_empty());
    assert(empty == "");
    assert(empty.c_str()[0] == '\0');
    
    String empty2 = empty.clone();
    assert(empty2.is_empty());
    
    empty.push_str("");
    assert(empty.is_empty());
    
    // Very long strings
    String long_str;
    for (int i = 0; i < 1000; i++) {
        long_str.push_str("Hello ");
    }
    assert(long_str.len() == 6000);
    assert(long_str.contains("Hello"));
    
    // Null handling
    String s1 = String::from(nullptr);
    assert(s1.is_empty());
    
    s1.push_str(nullptr);
    assert(s1.is_empty());
    
    std::cout << "✓ Edge case tests passed" << std::endl;
}

void test_comparison() {
    std::cout << "Testing comparison..." << std::endl;
    
    String s1 = String::from("abc");
    String s2 = String::from("abc");
    String s3 = String::from("abd");
    String s4 = String::from("ab");
    
    assert(s1 == s2);
    assert(s1 != s3);
    assert(s1 < s3);
    assert(!(s3 < s1));
    assert(s4 < s1);
    
    // Comparison with C strings
    assert(s1 == "abc");
    assert(s1 != "abd");
    
    std::cout << "✓ Comparison tests passed" << std::endl;
}

void test_reserve_and_capacity() {
    std::cout << "Testing reserve and capacity..." << std::endl;
    
    String s = String::with_capacity(10);
    assert(s.capacity() >= 10);
    assert(s.is_empty());
    
    s.push_str("Hello");
    assert(s.len() == 5);
    assert(s.capacity() >= 10);
    
    s.reserve(100);
    assert(s.capacity() >= 105); // At least current + requested
    assert(s == "Hello"); // Content unchanged
    
    std::cout << "✓ Reserve and capacity tests passed" << std::endl;
}

int main() {
    std::cout << "Running rusty::String tests..." << std::endl;
    std::cout << "================================" << std::endl;
    
    test_basic_construction();
    test_move_semantics();
    test_push_and_pop();
    test_string_operations();
    test_string_manipulation();
    test_string_search();
    test_string_transform();
    test_string_split();
    test_string_access();
    test_str_type();
    test_edge_cases();
    test_comparison();
    test_reserve_and_capacity();
    
    std::cout << "================================" << std::endl;
    std::cout << "✅ All String tests passed!" << std::endl;
    
    return 0;
}