#include "math.h"

// Implementation of safe_add - inherits @safe from header
int safe_add(int a, int b) {
    // This would be an error: pointer operations in safe function
    int* ptr = &a;
    return a + b;
}

// Implementation of unsafe_divide - inherits @unsafe from header
int unsafe_divide(int a, int b) {
    if (b == 0) {
        // Pointer operations allowed in unsafe function
        int* error = nullptr;
        return *error;  
    }
    return a / b;
}

// Implementation of regular_multiply - no annotation means unsafe
int regular_multiply(int a, int b) {
    // Pointer operations allowed - unsafe by default
    int result = a * b;
    int* ptr = &result;
    return *ptr;
}