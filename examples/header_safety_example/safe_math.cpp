#include "safe_math.h"
#include <iostream>

// Implementation of SafeMath namespace functions
// These inherit the @safe annotation from the namespace in the header

namespace SafeMath {
    int add(int a, int b) {
        // This function is safe - no pointer operations allowed
        return a + b;
    }
    
    int subtract(int a, int b) {
        // Also safe
        return a - b;
    }
    
    int multiply(int a, int b) {
        // Safe function
        return a * b;
    }
    
    int divide(int a, int b) {
        // This is marked @unsafe in the header, so pointer operations are allowed
        if (b == 0) {
            int* error_ptr = nullptr;
            *error_ptr = -1;  // OK - unsafe function
            return 0;
        }
        return a / b;
    }
}

// SafeCalculator methods inherit safety from the class declaration

void SafeCalculator::set_value(int val) {
    // This is a safe method - inherited from header
    value = val;
    // int* ptr = &value;  // This would be an error - raw pointer in safe function
}

int SafeCalculator::get_value() const {
    // Safe method
    return value;
}

void SafeCalculator::increment() {
    // Safe method
    value++;
}

void SafeCalculator::raw_pointer_operation() {
    // This is marked unsafe in the header, so pointer operations are allowed
    int* ptr = &value;
    *ptr = *ptr + 1;
}

// Individual function implementations

int safe_factorial(int n) {
    // This function is marked @safe in the header
    if (n <= 1) return 1;
    return n * safe_factorial(n - 1);
    // int* ptr = &n;  // This would be an error - raw pointer in safe function
}

void unsafe_memory_operation(void* ptr) {
    // This is marked unsafe in the header, pointer operations allowed
    int* int_ptr = static_cast<int*>(ptr);
    if (int_ptr) {
        *int_ptr = 42;
    }
}

void regular_function() {
    // No annotation in header means unsafe by default
    int x = 10;
    int* ptr = &x;
    *ptr = 20;
}

// This function is not declared in the header - it's only in the implementation
// It can have its own safety annotation
// @safe
void implementation_only_safe_function() {
    int a = 5, b = 10;
    int result = SafeMath::add(a, b);
}