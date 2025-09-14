// This example demonstrates the calling rules for undeclared functions
// Key rule: Undeclared functions can call other undeclared functions

#include <iostream>
#include <vector>
#include <memory>

// ============================================================================
// Undeclared functions (no annotation) - can call anything
// ============================================================================

// Helper function - undeclared
void log_message(const char* msg) {
    std::cout << "[LOG] " << msg << std::endl;
}

// Another undeclared function
void process_data() {
    log_message("Processing data");  // OK: undeclared calling undeclared
    
    std::vector<int> data = {1, 2, 3};  // OK: undeclared can use STL
    for (int val : data) {
        std::cout << val << " ";
    }
    std::cout << std::endl;
}

// Undeclared function calling chain
void initialize_system() {
    log_message("Initializing system");  // OK: undeclared calling undeclared
    process_data();                      // OK: undeclared calling undeclared
}

// Main function (undeclared by default)
int main() {
    std::cout << "=== Undeclared Function Calling Demo ===" << std::endl;
    
    // Main is undeclared, so it can call anything
    initialize_system();  // OK: undeclared calling undeclared
    process_data();      // OK: undeclared calling undeclared
    log_message("Done"); // OK: undeclared calling undeclared
    
    // Can also call explicitly marked functions
    safe_operation();    // OK: undeclared can call safe
    unsafe_operation();  // OK: undeclared can call unsafe
    
    return 0;
}

// ============================================================================
// Safe functions - CANNOT call undeclared functions
// ============================================================================

// @safe
void safe_operation() {
    // log_message("Safe op");  // ERROR: safe cannot call undeclared
    // process_data();          // ERROR: safe cannot call undeclared
    
    // Can only call:
    // 1. Other safe functions
    // 2. Explicitly unsafe functions
    // 3. Whitelisted standard functions
    printf("Safe operation\n");  // OK: printf is whitelisted
}

// ============================================================================
// Unsafe functions - can call anything
// ============================================================================

// @unsafe
void unsafe_operation() {
    log_message("Unsafe op");     // OK: unsafe can call undeclared
    process_data();               // OK: unsafe can call undeclared
    initialize_system();          // OK: unsafe can call undeclared
    
    // Can use raw pointers and do anything
    int* ptr = new int(42);
    *ptr = 100;
    delete ptr;
}

// ============================================================================
// The Three-State System Rationale:
//
// 1. UNDECLARED (default): Legacy/unaudited code
//    - Not checked by the borrow checker
//    - Can call any functions (undeclared, safe, or unsafe)
//    - Represents existing codebases that haven't been audited yet
//
// 2. SAFE: Audited and verified safe
//    - Full borrow checking enforced
//    - Can only call safe or explicitly unsafe functions
//    - CANNOT call undeclared (forces explicit auditing)
//
// 3. UNSAFE: Audited but known to be unsafe
//    - Not checked by the borrow checker
//    - Can call any functions
//    - Explicitly documented as containing unsafe operations
//
// This creates an "audit ratchet" - once you mark something as safe,
// you must audit everything it depends on.
// ============================================================================