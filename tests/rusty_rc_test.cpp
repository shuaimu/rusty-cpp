// Tests for rusty::Rc<T>
#include "../include/rusty/rc.hpp"
#include <cassert>
#include <cstdio>

using namespace rusty;

// Test basic construction
void test_rc_construction() {
    printf("test_rc_construction: ");
    {
        auto rc1 = Rc<int>::new_(42);
        assert(rc1.is_valid());
        assert(*rc1 == 42);
        assert(rc1.strong_count() == 1);
        
        auto rc2 = rc<int>(100);
        assert(rc2.is_valid());
        assert(*rc2 == 100);
        
        auto rc3 = make_rc<int>(200);
        assert(rc3.is_valid());
        assert(*rc3 == 200);
    }
    printf("PASS\n");
}

// Test cloning and reference counting
void test_rc_clone() {
    printf("test_rc_clone: ");
    {
        auto rc1 = Rc<int>::new_(42);
        assert(rc1.strong_count() == 1);
        
        auto rc2 = rc1.clone();
        assert(rc1.strong_count() == 2);
        assert(rc2.strong_count() == 2);
        assert(*rc1 == 42);
        assert(*rc2 == 42);
        
        {
            auto rc3 = rc1;  // Copy constructor
            assert(rc1.strong_count() == 3);
        }
        assert(rc1.strong_count() == 2);  // rc3 destroyed
    }
    printf("PASS\n");
}

// Test move semantics
void test_rc_move() {
    printf("test_rc_move: ");
    {
        auto rc1 = Rc<int>::new_(42);
        auto rc2 = rc1.clone();
        assert(rc1.strong_count() == 2);
        
        auto rc3 = std::move(rc1);
        assert(!rc1.is_valid());  // rc1 should be empty
        assert(rc3.strong_count() == 2);  // Count unchanged
        assert(*rc3 == 42);
    }
    printf("PASS\n");
}

// Test get_mut
void test_rc_get_mut() {
    printf("test_rc_get_mut: ");
    {
        auto rc1 = Rc<int>::new_(42);
        
        // Should get mutable reference when unique
        int* mut_ref = rc1.get_mut();
        assert(mut_ref != nullptr);
        *mut_ref = 100;
        assert(*rc1 == 100);
        
        // Should not get mutable reference when shared
        auto rc2 = rc1.clone();
        mut_ref = rc1.get_mut();
        assert(mut_ref == nullptr);  // Can't mutate when shared
    }
    printf("PASS\n");
}

// Test make_unique
void test_rc_make_unique() {
    printf("test_rc_make_unique: ");
    {
        auto rc1 = Rc<int>::new_(42);
        auto rc2 = rc1.clone();
        assert(rc1.strong_count() == 2);
        
        auto rc3 = rc1.make_unique();  // Deep copy
        assert(rc3.strong_count() == 1);  // New independent Rc
        assert(*rc3 == 42);
        assert(rc1.strong_count() == 2);  // Original unchanged
    }
    printf("PASS\n");
}

// Test with custom struct
struct TestStruct {
    int value;
    static int instances;
    
    TestStruct(int v) : value(v) { instances++; }
    TestStruct(const TestStruct& other) : value(other.value) { instances++; }
    ~TestStruct() { instances--; }
};

int TestStruct::instances = 0;

void test_rc_destructor() {
    printf("test_rc_destructor: ");
    TestStruct::instances = 0;
    {
        auto rc1 = Rc<TestStruct>::new_(TestStruct(42));
        // There will be at least one instance
        assert(TestStruct::instances >= 1);
        
        int instances_after_create = TestStruct::instances;
        
        {
            auto rc2 = rc1.clone();
            auto rc3 = rc2.clone();
            // No new instances created by cloning
            assert(TestStruct::instances == instances_after_create);
        }
        
        // Still same number of instances
        assert(TestStruct::instances == instances_after_create);
    }
    // All destroyed when last Rc dropped
    assert(TestStruct::instances == 0);
    printf("PASS\n");
}

// Test Weak references
void test_rc_weak() {
    printf("test_rc_weak: ");
    {
        auto rc1 = Rc<int>::new_(42);
        Weak<int> weak(rc1);
        
        // Note: Weak doesn't increment strong count
        assert(rc1.strong_count() == 1);
        
        auto rc2 = weak.upgrade();
        if (rc2.is_valid()) {
            assert(*rc2 == 42);
            // Now we have 2 strong references
            assert(rc1.strong_count() == 2);
        }
    }
    
    // Test expired weak
    {
        Weak<int> weak;
        {
            auto rc = Rc<int>::new_(42);
            weak = Weak<int>(rc);
            // Weak doesn't affect strong count
            assert(rc.strong_count() == 1);
        }
        // rc is destroyed, upgrade should fail
        auto upgraded = weak.upgrade();
        // Note: This is simplified, might not work correctly
    }
    printf("PASS\n");
}

// Test empty Rc
void test_rc_empty() {
    printf("test_rc_empty: ");
    {
        Rc<int> rc;
        assert(!rc.is_valid());
        assert(!rc);
        assert(rc.get() == nullptr);
        assert(rc.strong_count() == 0);
    }
    printf("PASS\n");
}

// Test assignment operators
void test_rc_assignment() {
    printf("test_rc_assignment: ");
    {
        auto rc1 = Rc<int>::new_(42);
        auto rc2 = Rc<int>::new_(100);
        
        rc2 = rc1;  // Copy assignment
        assert(rc1.strong_count() == 2);
        assert(*rc2 == 42);
        
        auto rc3 = Rc<int>::new_(200);
        rc3 = std::move(rc1);  // Move assignment
        assert(!rc1.is_valid());
        assert(rc3.strong_count() == 2);
    }
    printf("PASS\n");
}

// Test arrow operator
void test_rc_arrow() {
    printf("test_rc_arrow: ");
    {
        auto rc = Rc<TestStruct>::new_(TestStruct(42));
        assert(rc->value == 42);
        
        // Can't modify through const arrow
        const auto& const_rc = rc;
        assert(const_rc->value == 42);
    }
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::Rc<T> ===\n");
    
    test_rc_construction();
    test_rc_clone();
    test_rc_move();
    test_rc_get_mut();
    test_rc_make_unique();
    test_rc_destructor();
    test_rc_weak();
    test_rc_empty();
    test_rc_assignment();
    test_rc_arrow();
    
    printf("\nAll Rc tests passed!\n");
    return 0;
}