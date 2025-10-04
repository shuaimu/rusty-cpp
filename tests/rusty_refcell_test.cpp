// Test suite for RefCell<T> - interior mutability with runtime borrow checking

#include "rusty/refcell.hpp"
#include <iostream>
#include <cassert>
#include <string>
#include <vector>

using namespace rusty;

// @safe
void test_refcell_basic() {
    std::cout << "Testing RefCell basic operations..." << std::endl;
    
    RefCell<int> cell(42);
    
    // Immutable borrow
    {
        auto ref = cell.borrow();
        assert(*ref == 42);
    }
    
    // Mutable borrow
    {
        auto mut_ref = cell.borrow_mut();
        *mut_ref = 100;
    }
    
    // Check the value changed
    {
        auto ref = cell.borrow();
        assert(*ref == 100);
    }
    
    std::cout << "✓ RefCell basic operations work" << std::endl;
}

// @safe
void test_refcell_multiple_readers() {
    std::cout << "Testing RefCell multiple immutable borrows..." << std::endl;
    
    RefCell<std::string> cell("hello");
    
    // Multiple immutable borrows should work
    auto ref1 = cell.borrow();
    auto ref2 = cell.borrow();
    auto ref3 = cell.borrow();
    
    assert(*ref1 == "hello");
    assert(*ref2 == "hello");
    assert(*ref3 == "hello");
    
    std::cout << "✓ Multiple immutable borrows work" << std::endl;
}

// @safe
void test_refcell_borrow_rules() {
    std::cout << "Testing RefCell borrow checking rules..." << std::endl;
    
    RefCell<int> cell(42);
    
    // Test 1: Can't mutably borrow while immutably borrowed
    {
        auto ref = cell.borrow();
        
        try {
            auto mut_ref = cell.borrow_mut();
            assert(false && "Should have thrown - mutable borrow while immutably borrowed");
        } catch (const std::runtime_error& e) {
            std::cout << "  ✓ Correctly prevented mutable borrow while immutably borrowed" << std::endl;
        }
    }
    
    // Test 2: Can't immutably borrow while mutably borrowed
    {
        auto mut_ref = cell.borrow_mut();
        
        try {
            auto ref = cell.borrow();
            assert(false && "Should have thrown - immutable borrow while mutably borrowed");
        } catch (const std::runtime_error& e) {
            std::cout << "  ✓ Correctly prevented immutable borrow while mutably borrowed" << std::endl;
        }
    }
    
    // Test 3: Can't have two mutable borrows
    {
        auto mut_ref1 = cell.borrow_mut();
        
        try {
            auto mut_ref2 = cell.borrow_mut();
            assert(false && "Should have thrown - second mutable borrow");
        } catch (const std::runtime_error& e) {
            std::cout << "  ✓ Correctly prevented second mutable borrow" << std::endl;
        }
    }
    
    std::cout << "✓ RefCell borrow rules enforced" << std::endl;
}

// @safe
void test_refcell_try_borrow() {
    std::cout << "Testing RefCell can_borrow checks..." << std::endl;
    
    RefCell<int> cell(42);
    
    // can_borrow should return true when not borrowed
    assert(cell.can_borrow() == true);
    assert(cell.can_borrow_mut() == true);
    
    // can_borrow should return false when mutably borrowed
    {
        auto mut_ref = cell.borrow_mut();
        assert(cell.can_borrow() == false);
        assert(cell.can_borrow_mut() == false);
    }
    
    // can_borrow_mut should return false when immutably borrowed
    {
        auto ref = cell.borrow();
        assert(cell.can_borrow() == true);  // Can have multiple readers
        assert(cell.can_borrow_mut() == false);  // But no writers
    }
    
    std::cout << "✓ RefCell can_borrow checks work" << std::endl;
}

// @safe
void test_refcell_replace() {
    std::cout << "Testing RefCell replace..." << std::endl;
    
    RefCell<std::string> cell("hello");
    
    // Replace when not borrowed
    std::string old = cell.replace("world");
    assert(old == "hello");
    assert(cell.get() == "world");
    
    // Can't replace when borrowed
    {
        auto ref = cell.borrow();
        
        try {
            cell.replace("foo");
            assert(false && "Should have thrown - replace while borrowed");
        } catch (const std::runtime_error& e) {
            std::cout << "  ✓ Correctly prevented replace while borrowed" << std::endl;
        }
    }
    
    std::cout << "✓ RefCell replace works" << std::endl;
}

// @safe
void test_refcell_swap() {
    std::cout << "Testing RefCell swap..." << std::endl;
    
    RefCell<int> cell1(10);
    RefCell<int> cell2(20);
    
    cell1.swap(cell2);
    
    assert(cell1.get() == 20);
    assert(cell2.get() == 10);
    
    // Can't swap when borrowed
    {
        auto ref = cell1.borrow();
        
        try {
            cell1.swap(cell2);
            assert(false && "Should have thrown - swap while borrowed");
        } catch (const std::runtime_error& e) {
            std::cout << "  ✓ Correctly prevented swap while borrowed" << std::endl;
        }
    }
    
    std::cout << "✓ RefCell swap works" << std::endl;
}

// @safe
void test_refcell_with_non_copyable() {
    std::cout << "Testing RefCell with non-copyable type..." << std::endl;
    
    struct Resource {
        int id;
        std::unique_ptr<int> data;
        
        Resource(int i) : id(i), data(std::make_unique<int>(i * 10)) {}
        Resource(Resource&&) = default;
        Resource& operator=(Resource&&) = default;
        
        // Not copyable
        Resource(const Resource&) = delete;
        Resource& operator=(const Resource&) = delete;
    };
    
    RefCell<Resource> cell(Resource(42));
    
    // Can borrow and modify
    {
        auto ref = cell.borrow();
        assert(ref->id == 42);
        assert(*ref->data == 420);
    }
    
    {
        auto mut_ref = cell.borrow_mut();
        mut_ref->id = 100;
        *mut_ref->data = 1000;
    }
    
    // Can replace with move semantics
    Resource old = cell.replace(Resource(200));
    assert(old.id == 100);
    assert(*old.data == 1000);
    
    std::cout << "✓ RefCell works with non-copyable types" << std::endl;
}

// @safe
void test_refcell_nested_borrows() {
    std::cout << "Testing RefCell nested borrow behavior..." << std::endl;
    
    RefCell<std::vector<int>> cell(std::vector<int>{1, 2, 3});
    
    // Nested immutable borrows
    {
        auto ref1 = cell.borrow();
        assert(ref1->size() == 3);
        
        {
            auto ref2 = cell.borrow();
            assert(ref2->size() == 3);
        }
        
        assert(ref1->size() == 3);  // ref1 still valid
    }
    
    // After all borrows released, can mutably borrow
    {
        auto mut_ref = cell.borrow_mut();
        mut_ref->push_back(4);
        assert(mut_ref->size() == 4);
    }
    
    std::cout << "✓ RefCell nested borrows work correctly" << std::endl;
}

// @safe
void test_refcell_move_semantics() {
    std::cout << "Testing RefCell borrow guard move semantics..." << std::endl;
    
    RefCell<int> cell(42);
    
    // Ref can be moved
    {
        auto ref1 = cell.borrow();
        auto ref2 = std::move(ref1);
        assert(*ref2 == 42);
        // ref1 is now invalid (moved from)
    }  // ref2 destroyed here, releasing borrow
    
    // RefMut can be moved
    {
        auto mut_ref1 = cell.borrow_mut();
        auto mut_ref2 = std::move(mut_ref1);
        *mut_ref2 = 100;
        // mut_ref1 is now invalid (moved from)
    }  // mut_ref2 destroyed here, releasing borrow
    
    assert(cell.get() == 100);
    
    std::cout << "✓ RefCell borrow guards have proper move semantics" << std::endl;
}

// @safe
void test_refcell_const_correctness() {
    std::cout << "Testing RefCell const correctness..." << std::endl;
    
    const RefCell<int> cell(42);
    
    // Can still borrow mutably from const RefCell (interior mutability)
    {
        auto mut_ref = cell.borrow_mut();
        *mut_ref = 100;
    }
    
    assert(cell.get() == 100);
    
    std::cout << "✓ RefCell provides interior mutability through const" << std::endl;
}

int main() {
    std::cout << "\n=== RefCell<T> Test Suite ===" << std::endl;
    
    test_refcell_basic();
    test_refcell_multiple_readers();
    test_refcell_borrow_rules();
    test_refcell_try_borrow();
    test_refcell_replace();
    test_refcell_swap();
    test_refcell_with_non_copyable();
    test_refcell_nested_borrows();
    test_refcell_move_semantics();
    test_refcell_const_correctness();
    
    std::cout << "\n✅ All RefCell tests passed!" << std::endl;
    return 0;
}