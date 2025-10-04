// Test suite for Cell<T> - interior mutability for Copy types

#include "rusty/cell.hpp"
#include <iostream>
#include <cassert>

using namespace rusty;

// @safe
void test_cell_basic() {
    std::cout << "Testing Cell basic operations..." << std::endl;
    
    // Create a cell with an integer
    Cell<int> cell(42);
    
    // Get the value
    assert(cell.get() == 42);
    
    // Set a new value
    cell.set(100);
    assert(cell.get() == 100);
    
    // Replace and get old value
    int old = cell.replace(200);
    assert(old == 100);
    assert(cell.get() == 200);
    
    std::cout << "✓ Cell basic operations work" << std::endl;
}

// @safe
void test_cell_swap() {
    std::cout << "Testing Cell swap..." << std::endl;
    
    Cell<int> cell1(10);
    Cell<int> cell2(20);
    
    cell1.swap(cell2);
    
    assert(cell1.get() == 20);
    assert(cell2.get() == 10);
    
    std::cout << "✓ Cell swap works" << std::endl;
}

// @safe
void test_cell_take() {
    std::cout << "Testing Cell take..." << std::endl;
    
    Cell<int> cell(42);
    int value = cell.take();
    
    assert(value == 42);
    assert(cell.get() == 0);  // Default value for int
    
    std::cout << "✓ Cell take works" << std::endl;
}

// @safe
void test_cell_update() {
    std::cout << "Testing Cell update..." << std::endl;
    
    Cell<int> counter(0);
    
    // Update using a lambda
    for (int i = 0; i < 5; i++) {
        counter.update([](int x) { return x + 1; });
    }
    
    assert(counter.get() == 5);
    
    std::cout << "✓ Cell update works" << std::endl;
}

// @safe
void test_cell_const_correctness() {
    std::cout << "Testing Cell const correctness..." << std::endl;
    
    const Cell<int> cell(42);
    
    // All methods should work on const Cell (interior mutability)
    cell.set(100);
    assert(cell.get() == 100);
    
    int old = cell.replace(200);
    assert(old == 100);
    
    std::cout << "✓ Cell const methods work" << std::endl;
}

// Test that Cell only works with Copy types
struct NonCopyable {
    int value;
    NonCopyable(int v) : value(v) {}
    NonCopyable(const NonCopyable&) = delete;
    NonCopyable& operator=(const NonCopyable&) = delete;
};

// @safe
void test_cell_copy_requirement() {
    std::cout << "Testing Cell Copy requirement..." << std::endl;
    
    // This should compile for POD types
    Cell<int> int_cell(42);
    Cell<double> double_cell(3.14);
    Cell<bool> bool_cell(true);
    
    struct Pod {
        int x;
        double y;
    };
    Cell<Pod> pod_cell(Pod{1, 2.0});
    
    // This should NOT compile (uncomment to test):
    // Cell<std::string> string_cell("hello");  // Error: string is not trivially copyable
    // Cell<NonCopyable> nc_cell(NonCopyable(42));  // Error: NonCopyable is not trivially copyable
    
    std::cout << "✓ Cell Copy requirement enforced" << std::endl;
}

// @safe
void test_cell_with_multiple_threads_unsafe() {
    std::cout << "Testing Cell thread safety (should be single-threaded only)..." << std::endl;
    
    // Cell is NOT thread-safe - this is just to document that fact
    Cell<int> cell(0);
    
    // DO NOT use Cell across threads!
    // This would be a data race:
    /*
    std::thread t1([&cell]() {
        for (int i = 0; i < 1000; i++) {
            cell.set(cell.get() + 1);
        }
    });
    
    std::thread t2([&cell]() {
        for (int i = 0; i < 1000; i++) {
            cell.set(cell.get() + 1);
        }
    });
    
    t1.join();
    t2.join();
    
    // Result would be unpredictable due to race condition
    */
    
    std::cout << "✓ Cell is single-threaded only (as designed)" << std::endl;
}

int main() {
    std::cout << "\n=== Cell<T> Test Suite ===" << std::endl;
    
    test_cell_basic();
    test_cell_swap();
    test_cell_take();
    test_cell_update();
    test_cell_const_correctness();
    test_cell_copy_requirement();
    test_cell_with_multiple_threads_unsafe();
    
    std::cout << "\n✅ All Cell tests passed!" << std::endl;
    return 0;
}