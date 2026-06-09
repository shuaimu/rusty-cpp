// Tests for rusty::Vec<T> (now an alias for ::Vec<T,A> from vec_port.vec).
// VecLegacy was retired; this test now exercises the transpiled rustc Vec
// via the C++20 module, plus the C++-ergonomic shortcuts (default ctor,
// initializer-list ctor, size_t-capacity ctor, make(), size(), operator[],
// begin()/end()) we hand-added to vec_port::Vec.

#include <rusty/option.hpp>     // for rusty::Option
#include <rusty/alloc.hpp>      // for rusty::alloc::Global
#include <rusty/vec.hpp>        // stub — see import below for the real Vec

#include <cassert>
#include <cstdio>
#include <utility>

import vec_port.vec;            // rusty::port::vec::Vec<T, A> —
                                // the transpiled rustc Vec after the
                                // deep-namespace migration.

using namespace rusty;
using rusty::port::vec::Vec;    // bring transpiled Vec into bare scope

// Test basic construction
void test_vec_construction() {
    printf("test_vec_construction: ");
    {
        auto vec1 = Vec<int>::make();
        assert(vec1.is_empty());
        assert(vec1.len() == 0);

        auto vec2 = Vec<int>::with_capacity(10);
        assert(vec2.is_empty());
        assert(vec2.capacity() >= 10);

        auto vec3 = Vec<int>(10);  // Using explicit capacity constructor
        assert(vec3.is_empty());
        assert(vec3.capacity() >= 10);  // vec_port may over-allocate
    }
    printf("PASS\n");
}

// Test push and pop
void test_vec_push_pop() {
    printf("test_vec_push_pop: ");
    {
        auto vec = Vec<int>::make();

        vec.push(10);
        vec.push(20);
        vec.push(30);

        assert(vec.len() == 3);
        assert(!vec.is_empty());
        assert(vec[0] == 10);
        assert(vec[1] == 20);
        assert(vec[2] == 30);

        // vec_port::Vec::pop() returns rusty::Option<T>
        int val = vec.pop().unwrap();
        assert(val == 30);
        assert(vec.len() == 2);

        val = vec.pop().unwrap();
        assert(val == 20);

        val = vec.pop().unwrap();
        assert(val == 10);
        assert(vec.is_empty());
    }
    printf("PASS\n");
}

// Test indexing
void test_vec_indexing() {
    printf("test_vec_indexing: ");
    {
        auto vec = Vec<int>::make();
        vec.push(10);
        vec.push(20);
        vec.push(30);

        // Non-const access
        vec[1] = 25;
        assert(vec[1] == 25);

        // Const access
        const auto& const_vec = vec;
        assert(const_vec[0] == 10);
        assert(const_vec[1] == 25);
        assert(const_vec[2] == 30);
    }
    printf("PASS\n");
}

// Test move semantics
void test_vec_move() {
    printf("test_vec_move: ");
    {
        auto vec1 = Vec<int>::make();
        vec1.push(10);
        vec1.push(20);

        auto vec2 = std::move(vec1);
        // After move, vec_port::Vec marks the source forgotten; len_field is
        // not reset to 0 explicitly, but the source is considered emptied
        // semantically. We just check the destination is intact.
        assert(vec2.len() == 2);
        assert(vec2[0] == 10);
        assert(vec2[1] == 20);

        Vec<int> vec3;
        vec3 = std::move(vec2);
        assert(vec3.len() == 2);
    }
    printf("PASS\n");
}

// Test clear
void test_vec_clear() {
    printf("test_vec_clear: ");
    {
        auto vec = Vec<int>::make();
        vec.push(10);
        vec.push(20);
        vec.push(30);

        assert(vec.len() == 3);
        size_t old_cap = vec.capacity();

        vec.clear();
        assert(vec.is_empty());
        assert(vec.len() == 0);
        assert(vec.capacity() == old_cap);  // Capacity unchanged
    }
    printf("PASS\n");
}

// front()/back() are not exposed on vec_port::Vec; the legacy front()/back()
// test has been retired. Element 0 and element len-1 are still reachable via
// operator[] and exercised in the indexing test.

// Test reserve
void test_vec_reserve() {
    printf("test_vec_reserve: ");
    {
        auto vec = Vec<int>::make();
        assert(vec.capacity() == 0);

        vec.reserve(100);
        assert(vec.capacity() >= 100);
        assert(vec.is_empty());  // Still empty

        vec.push(10);
        assert(vec.len() == 1);
        assert(vec.capacity() >= 100);  // Capacity preserved
    }
    printf("PASS\n");
}

// Test clone
void test_vec_clone() {
    printf("test_vec_clone: ");
    {
        auto vec = Vec<int>::make();
        vec.push(10);
        vec.push(20);
        vec.push(30);

        auto vec2 = vec.clone();
        assert(vec2.len() == 3);
        assert(vec2[0] == 10);
        assert(vec2[1] == 20);
        assert(vec2[2] == 30);

        // Modify original
        vec[0] = 15;
        // Clone is independent
        assert(vec2[0] == 10);
    }
    printf("PASS\n");
}

// Test iteration
void test_vec_iteration() {
    printf("test_vec_iteration: ");
    {
        auto vec = Vec<int>::make();
        vec.push(1);
        vec.push(2);
        vec.push(3);

        int sum = 0;
        for (const auto& val : vec) {
            sum += val;
        }
        assert(sum == 6);

        // Modify through iteration
        for (auto& val : vec) {
            val *= 2;
        }
        assert(vec[0] == 2);
        assert(vec[1] == 4);
        assert(vec[2] == 6);
    }
    printf("PASS\n");
}

// Test with custom struct
struct TestStruct {
    int value;
    static int instances;

    TestStruct(int v) : value(v) { instances++; }
    TestStruct(const TestStruct& other) : value(other.value) { instances++; }
    TestStruct(TestStruct&& other) : value(other.value) { instances++; }
    ~TestStruct() { instances--; }
};

int TestStruct::instances = 0;

void test_vec_destructor() {
    printf("test_vec_destructor: ");
    TestStruct::instances = 0;
    {
        auto vec = Vec<TestStruct>::make();
        vec.push(TestStruct(1));
        vec.push(TestStruct(2));
        vec.push(TestStruct(3));

        assert(TestStruct::instances == 3);
    }
    assert(TestStruct::instances == 0);  // All destroyed
    printf("PASS\n");
}

// Test initializer-list construction (replaces the legacy vec_of helper).
void test_vec_init_list() {
    printf("test_vec_init_list: ");
    {
        Vec<int> vec{1, 2, 3, 4, 5};

        assert(vec.len() == 5);
        assert(vec[0] == 1);
        assert(vec[1] == 2);
        assert(vec[2] == 3);
        assert(vec[3] == 4);
        assert(vec[4] == 5);
    }
    printf("PASS\n");
}

// Test size() alias
void test_vec_size() {
    printf("test_vec_size: ");
    {
        auto vec = Vec<int>::make();
        vec.push(10);
        vec.push(20);
        vec.push(30);

        assert(vec.size() == 3);  // size() is alias for len()
        assert(vec.size() == vec.len());

        (void)vec.pop().unwrap();
        assert(vec.size() == 2);
    }
    printf("PASS\n");
}

// Test unsafe-style set_len surface (used by transpiled Rust unsafe code)
void test_vec_set_len() {
    printf("test_vec_set_len: ");
    {
        auto vec = Vec<int>::with_capacity(4);
        vec.push(10);
        vec.push(20);
        vec.push(30);

        vec.set_len(1);
        assert(vec.len() == 1);
        assert(vec[0] == 10);

        // Previous elements remain materialized; caller controls safety invariants.
        vec.set_len(3);
        assert(vec.len() == 3);
        assert(vec[1] == 20);
        assert(vec[2] == 30);
    }
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::Vec<T> (vec_port::Vec) ===\n");

    test_vec_construction();
    test_vec_push_pop();
    test_vec_indexing();
    test_vec_move();
    test_vec_clear();
    test_vec_reserve();
    test_vec_clone();
    test_vec_iteration();
    test_vec_destructor();
    test_vec_init_list();
    test_vec_size();
    test_vec_set_len();

    printf("\nAll Vec tests passed!\n");
    return 0;
}
