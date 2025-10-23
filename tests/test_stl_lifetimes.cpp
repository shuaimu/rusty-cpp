// Test STL lifetime checking with annotations
#include "../include/stl_lifetimes.hpp"
#include <vector>
#include <map>
#include <memory>
#include <string>

// @safe
namespace test_vector {
    void iterator_invalidation() {
        std::vector<int> vec = {1, 2, 3};
        
        // Get reference to element
        int& ref = vec[0];  // Borrows &'vec mut
        
        // This should be an error - cannot modify vector while reference exists
        // vec.push_back(4);  // ERROR: would invalidate ref
        
        // Using ref is OK
        ref = 10;
        
        // After ref goes out of scope, we can modify again
        {
            int& temp_ref = vec[1];
            temp_ref = 20;
        } // temp_ref scope ends
        
        vec.push_back(4); // OK now
    }
    
    void iterator_example() {
        std::vector<int> vec = {1, 2, 3};
        
        // Iterator borrows from vector
        auto it = vec.begin();  // &'vec mut
        
        // Cannot modify vector while iterator exists
        // vec.push_back(4);  // ERROR: would invalidate iterator
        
        // Can use iterator
        *it = 10;
        ++it;
        
        // Once iterator is done, can modify
        it = vec.end();
        vec.push_back(4); // Should be OK if iterator not used after
    }
    
    void const_correctness() {
        const std::vector<int> vec = {1, 2, 3};
        
        // Can get const references
        const int& ref1 = vec[0];  // &'vec
        const int& ref2 = vec[1];  // &'vec - multiple const refs OK
        
        // Cannot get mutable reference from const vector
        // int& mut_ref = vec[0];  // ERROR: cannot get mut ref from const
        
        // Can call const methods
        size_t s = vec.size();
        
        // Cannot call non-const methods
        // vec.push_back(4);  // ERROR: cannot modify const vector
    }
    
    void data_pointer() {
        std::vector<int> vec = {1, 2, 3};
        
        // data() returns raw pointer
        int* ptr = vec.data();  // *mut - requires unsafe
        
        // Using raw pointer should require unsafe
        // *ptr = 10;  // ERROR: dereference requires unsafe
        
        // Getting pointer is OK, using it needs unsafe
        const int* cptr = vec.data();  // *const
    }
}

// @safe
namespace test_map {
    void reference_stability() {
        std::map<int, std::string> m;
        m[1] = "one";
        m[2] = "two";
        
        // Map references are stable (unlike vector)
        std::string& ref = m[1];  // &'m mut
        
        // Can insert new elements without invalidating existing refs
        m[3] = "three";  // OK - doesn't invalidate ref
        
        // But cannot erase the referenced element
        // m.erase(1);  // ERROR: would invalidate ref
        
        // Can erase other elements
        m.erase(2);  // OK - ref still valid
        
        ref = "ONE";  // Can still use ref
    }
    
    void find_lifetime() {
        std::map<int, std::string> m;
        m[1] = "one";
        
        // find returns iterator with map's lifetime
        auto it = m.find(1);  // &'m mut
        
        if (it != m.end()) {
            // Iterator prevents certain modifications
            // m.clear();  // ERROR: would invalidate iterator
            
            // Can modify through iterator
            it->second = "ONE";
        }
    }
}

// @safe
namespace test_unique_ptr {
    void ownership_transfer() {
        std::unique_ptr<int> ptr1 = std::make_unique<int>(42);
        
        // Move transfers ownership
        std::unique_ptr<int> ptr2 = std::move(ptr1);  // ptr1 moved
        
        // Cannot use ptr1 after move
        // int val = *ptr1;  // ERROR: use after move
        
        // ptr2 owns the value now
        int val = *ptr2;  // OK
        
        // get() returns raw pointer
        int* raw = ptr2.get();  // *mut - requires unsafe
        
        // Using raw pointer should require unsafe
        // *raw = 100;  // ERROR: dereference requires unsafe
    }
    
    void reference_from_unique_ptr() {
        std::unique_ptr<int> ptr = std::make_unique<int>(42);
        
        // Can get reference through operator*
        int& ref = *ptr;  // &'ptr mut
        
        // Cannot move ptr while reference exists
        // std::unique_ptr<int> ptr2 = std::move(ptr);  // ERROR: cannot move while borrowed
        
        // Can use reference
        ref = 100;
        
        // After ref scope ends, can move
        {
            int& temp = *ptr;
            temp = 200;
        }
        
        std::unique_ptr<int> ptr2 = std::move(ptr);  // OK now
    }
}

// @safe
namespace test_string {
    void string_references() {
        std::string str = "hello";
        
        // Can get character references
        char& ch = str[0];  // &'str mut
        
        // Cannot modify string structure while reference exists
        // str.push_back('!');  // ERROR: would invalidate reference
        
        // Can modify through reference
        ch = 'H';
        
        // c_str() returns raw pointer
        const char* cstr = str.c_str();  // *const - requires unsafe
        
        // Using raw pointer should require unsafe
        // char c = *cstr;  // ERROR: dereference requires unsafe
    }
}

// @safe
namespace test_pair {
    void pair_members() {
        std::pair<int, std::string> p = {1, "one"};
        
        // Can access members
        int& first = p.first;  // &'p mut
        std::string& second = p.second;  // &'p mut - ERROR: already borrowed p
        
        // Multiple mutable borrows not allowed
        // Should detect this as an error
    }
}

// Examples that should require @unsafe
namespace unsafe_examples {
    // @unsafe
    void raw_pointer_arithmetic() {
        std::vector<int> vec = {1, 2, 3};
        int* ptr = vec.data();
        
        // Raw pointer operations are unsafe
        ptr++;
        *ptr = 20;
        
        // Pointer arithmetic is inherently unsafe
        int* end = ptr + vec.size();
        while (ptr < end) {
            *ptr++ = 0;
        }
    }
    
    // @unsafe  
    void manual_memory_management() {
        int* ptr = new int(42);
        
        // Manual memory management is unsafe
        *ptr = 100;
        delete ptr;
        
        // Use after free would be undefined behavior
        // *ptr = 200;  // Still an error even in unsafe
    }
}

int main() {
    // Run safe tests
    test_vector::iterator_invalidation();
    test_vector::iterator_example();
    test_vector::const_correctness();
    test_vector::data_pointer();
    
    test_map::reference_stability();
    test_map::find_lifetime();
    
    test_unique_ptr::ownership_transfer();
    test_unique_ptr::reference_from_unique_ptr();
    
    test_string::string_references();
    
    test_pair::pair_members();
    
    // Unsafe examples
    unsafe_examples::raw_pointer_arithmetic();
    unsafe_examples::manual_memory_management();
    
    return 0;
}