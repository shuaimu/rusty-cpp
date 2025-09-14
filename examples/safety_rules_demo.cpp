// This example demonstrates the refined safety rules:
// 1. Functions can be @safe, @unsafe, or undeclared (default)
// 2. Safe functions CAN call @safe functions
// 3. Safe functions CAN call @unsafe functions (they're explicitly marked)
// 4. Safe functions CANNOT call undeclared functions (must be audited first)
// 5. Undeclared and unsafe functions can call anything

#include <iostream>

// ============================================================================
// Undeclared functions (no annotation - the default)
// ============================================================================

void undeclared_helper() {
    std::cout << "This is an undeclared function\n";
}

void another_undeclared() {
    // Undeclared functions are treated like unsafe for checking purposes
    // They can do pointer operations
    int x = 42;
    int* ptr = &x;
    *ptr = 100;
    
    // They can call other undeclared functions
    undeclared_helper();
}

// ============================================================================
// Explicitly unsafe functions
// ============================================================================

// @unsafe
void explicitly_unsafe_function() {
    // Can do dangerous operations
    int* dangerous = nullptr;
    // *dangerous = 42;  // Would crash but allowed by type system
    
    // Can call undeclared functions
    undeclared_helper();
    
    std::cout << "Explicitly unsafe function\n";
}

// @unsafe
void another_unsafe() {
    // Unsafe can call other unsafe
    explicitly_unsafe_function();
    
    // Unsafe can call undeclared
    another_undeclared();
}

// ============================================================================
// Safe functions with strict rules
// ============================================================================

// @safe
void safe_helper() {
    std::cout << "Safe helper function\n";
}

// @safe
void safe_function() {
    // Safe functions can call other safe functions
    safe_helper();  // OK
    
    // Safe functions CAN call explicitly unsafe functions
    explicitly_unsafe_function();  // OK: it's explicitly marked as unsafe
    
    // But CANNOT call undeclared functions
    // undeclared_helper();  // ERROR: must be explicitly marked @safe or @unsafe
    
    // Safe functions also cannot do pointer operations themselves
    // int x = 5;
    // int* ptr = &x;  // ERROR: pointer operations require unsafe context
}

// ============================================================================
// Mixed scenarios
// ============================================================================

// @safe
void process_data(int value) {
    std::cout << "Processing: " << value << "\n";
}

// This function has no annotation - it's undeclared
void application_logic() {
    // Can call safe functions
    process_data(42);  // OK
    safe_helper();     // OK
    
    // Can call unsafe functions  
    explicitly_unsafe_function();  // OK
    
    // Can call other undeclared
    undeclared_helper();  // OK
    
    // Can do pointer operations (treated as unsafe for checking)
    int* data = new int(100);
    delete data;
}

// ============================================================================
// Main function (undeclared by default)
// ============================================================================

int main() {
    std::cout << "Safety Rules Demo\n";
    std::cout << "==================\n\n";
    
    // Main is undeclared, so it can call anything
    safe_function();
    explicitly_unsafe_function();
    application_logic();
    
    return 0;
}

// ============================================================================
// Key Takeaways:
// 
// 1. Default (undeclared) != Explicitly unsafe
//    - Undeclared functions are "legacy code" that hasn't been audited
//    - Explicitly unsafe functions have been reviewed and marked as unsafe
//
// 2. Safe functions enforce auditing
//    - Can call other safe functions (obviously safe)
//    - Can call explicitly unsafe functions (risks are known and documented)
//    - CANNOT call undeclared functions (forces you to audit them first)
//
// 3. This creates a "ratchet" effect
//    - You must gradually audit and mark functions as @safe or @unsafe
//    - Safe code is isolated from unaudited code
//    - Forces explicit decisions about function safety
//    - Unsafe functions serve as documented boundaries of unsafety
// ============================================================================