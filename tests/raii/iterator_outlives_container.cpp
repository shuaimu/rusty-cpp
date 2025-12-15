// Test: Iterator Outlives Container
// Status: PARTIALLY DETECTED (STL lifetime annotations exist)
//
// Iterators are essentially borrows from containers. When the container
// is destroyed, all iterators become invalid.

#include <vector>
#include <map>
#include <string>
#include <list>

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @safe
void bad_iterator_outlives_vector() {
    std::vector<int>::iterator it;
    {
        std::vector<int> v = {1, 2, 3};
        it = v.begin();  // it borrows from v
    }  // v destroyed - iterator invalidated

    // ERROR: it is now invalid
    // @unsafe
    int val = *it;
}

// @safe
void bad_end_iterator_outlives() {
    std::vector<int>::iterator end_it;
    {
        std::vector<int> v = {1, 2, 3};
        end_it = v.end();
    }  // v destroyed

    // ERROR: end_it is invalid (can't even compare)
}

// @safe
void bad_multiple_iterators_outlive() {
    std::vector<int>::iterator it1, it2;
    {
        std::vector<int> v = {1, 2, 3, 4, 5};
        it1 = v.begin();
        it2 = v.begin() + 2;
    }  // v destroyed - ALL iterators invalidated

    // ERROR: both it1 and it2 are invalid
    // @unsafe
    {
        int a = *it1;
        int b = *it2;
    }
}

// @safe
void bad_map_iterator_outlives() {
    std::map<int, std::string>::iterator it;
    {
        std::map<int, std::string> m;
        m[1] = "one";
        m[2] = "two";
        it = m.find(1);
    }  // m destroyed

    // ERROR: it is invalid
    // @unsafe
    auto& val = it->second;
}

// @safe
void bad_string_iterator_outlives() {
    std::string::iterator it;
    {
        std::string s = "hello";
        it = s.begin();
    }  // s destroyed

    // ERROR: it is invalid
    // @unsafe
    char c = *it;
}

// Invalidation through clear (different from destruction but related)
// @safe
void bad_clear_invalidates() {
    std::vector<int> v = {1, 2, 3};
    auto it = v.begin();

    v.clear();  // Invalidates ALL iterators

    // ERROR: it is invalid
    // @unsafe
    int val = *it;
}

// Pointer to element (similar to iterator)
// @safe
void bad_pointer_to_element_outlives() {
    int* ptr;
    {
        std::vector<int> v = {1, 2, 3};
        ptr = &v[0];  // ptr points into vector storage
    }  // v destroyed, storage freed

    // ERROR: ptr is dangling
    // @unsafe
    *ptr = 10;
}

// Reference to element
// @safe
void bad_reference_to_element_outlives() {
    // @unsafe
    int* ref_addr;
    {
        std::vector<int> v = {1, 2, 3};
        int& ref = v[0];
        ref_addr = &ref;
    }  // v destroyed

    // ERROR: ref_addr points to freed memory
    // @unsafe
    *ref_addr = 10;
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors
// =============================================================================

// @safe
void good_iterator_in_scope() {
    std::vector<int> v = {1, 2, 3};
    auto it = v.begin();
    int val = *it;  // OK: iterator and vector have same lifetime
}

// @safe
void good_iterate_in_loop() {
    std::vector<int> v = {1, 2, 3};
    for (auto it = v.begin(); it != v.end(); ++it) {
        int val = *it;  // OK: iterator doesn't escape loop
    }
}

// @safe
void good_range_for() {
    std::vector<int> v = {1, 2, 3};
    for (int& val : v) {
        val *= 2;  // OK: reference valid during iteration
    }
}

// @safe
int good_copy_element() {
    int result;
    {
        std::vector<int> v = {1, 2, 3};
        result = v[0];  // Copy, not reference
    }
    return result;  // OK: result is a copy
}

// @safe
void good_iterator_passed_to_function(std::vector<int>& v) {
    auto it = v.begin();
    // Iterator valid because v is owned by caller
    if (it != v.end()) {
        int val = *it;
    }
}
