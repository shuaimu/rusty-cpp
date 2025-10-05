// Tests for rusty::Box<T>
#include "../include/rusty/box.hpp"
#include <cassert>
#include <cstdio>

using namespace rusty;

// Test basic construction and destruction
void test_box_construction() {
    printf("test_box_construction: ");
    {
        auto box1 = Box<int>::new_(42);
        assert(box1.is_valid());
        assert(*box1 == 42);
        
        auto box2 = make_box<int>(100);
        assert(box2.is_valid());
        assert(*box2 == 100);
        
        auto box3 = make_box<int>(200);
        assert(box3.is_valid());
        assert(*box3 == 200);
    }
    printf("PASS\n");
}

// Test move semantics
void test_box_move() {
    printf("test_box_move: ");
    {
        auto box1 = Box<int>::new_(42);
        assert(box1.is_valid());
        
        auto box2 = std::move(box1);
        assert(!box1.is_valid());  // box1 should be empty after move
        assert(box2.is_valid());
        assert(*box2 == 42);
        
        Box<int> box3;
        box3 = std::move(box2);
        assert(!box2.is_valid());  // box2 should be empty after move
        assert(box3.is_valid());
        assert(*box3 == 42);
    }
    printf("PASS\n");
}

// Test into_raw and release
void test_box_raw_pointer() {
    printf("test_box_raw_pointer: ");
    {
        auto box1 = Box<int>::new_(42);
        int* raw = box1.into_raw();
        assert(!box1.is_valid());  // box1 should be empty after into_raw
        assert(*raw == 42);
        delete raw;  // Manual cleanup required after into_raw
        
        auto box2 = Box<int>::new_(100);
        int* raw2 = box2.release();  // C++ style alias
        assert(!box2.is_valid());
        assert(*raw2 == 100);
        delete raw2;
    }
    printf("PASS\n");
}

// Test reset
void test_box_reset() {
    printf("test_box_reset: ");
    {
        auto box1 = Box<int>::new_(42);
        assert(box1.is_valid());
        
        box1.reset(new int(100));
        assert(box1.is_valid());
        assert(*box1 == 100);
        
        box1.reset();  // Reset to nullptr
        assert(!box1.is_valid());
    }
    printf("PASS\n");
}

// Test with custom struct
struct TestStruct {
    int value;
    bool* destroyed;
    
    TestStruct(int v, bool* d) : value(v), destroyed(d) { *destroyed = false; }
    ~TestStruct() { *destroyed = true; }
};

void test_box_destructor() {
    printf("test_box_destructor: ");
    bool destroyed = false;
    {
        // Create the Box directly with a pointer to avoid copy
        Box<TestStruct> box(new TestStruct(42, &destroyed));
        assert(!destroyed);
    }
    assert(destroyed);  // Should be destroyed when box goes out of scope
    printf("PASS\n");
}

// Test arrow operator
void test_box_arrow() {
    printf("test_box_arrow: ");
    {
        bool destroyed = false;
        Box<TestStruct> box(new TestStruct(42, &destroyed));
        assert(box->value == 42);
        box->value = 100;
        assert(box->value == 100);
    }
    printf("PASS\n");
}

// Test empty box
void test_box_empty() {
    printf("test_box_empty: ");
    {
        Box<int> box;
        assert(!box.is_valid());
        assert(!box);  // Test explicit bool conversion
        assert(box.get() == nullptr);
    }
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::Box<T> ===\n");
    
    test_box_construction();
    test_box_move();
    test_box_raw_pointer();
    test_box_reset();
    test_box_destructor();
    test_box_arrow();
    test_box_empty();
    
    printf("\nAll Box tests passed!\n");
    return 0;
}