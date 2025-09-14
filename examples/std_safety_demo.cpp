// This example demonstrates that STL functions are undeclared by default
// and therefore cannot be called from safe functions without explicit marking

#include <iostream>
#include <vector>
#include <string>
#include <memory>
#include <algorithm>

// ============================================================================
// Safe function trying to use STL - should fail
// ============================================================================

// @safe
void safe_with_stl_attempt() {
    // All of these should be errors - STL functions are undeclared
    
    // std::vector operations
    std::vector<int> vec;  // ERROR: constructor is undeclared
    vec.push_back(42);     // ERROR: push_back is undeclared
    vec.size();            // ERROR: size is undeclared
    
    // std::string operations  
    // std::string str = "test";  // ERROR: constructor is undeclared
    // str.length();              // ERROR: length is undeclared
    
    // std::unique_ptr
    // auto ptr = std::make_unique<int>(42);  // ERROR: make_unique is undeclared
    
    // Algorithms
    // std::vector<int> v = {1, 2, 3};
    // std::sort(v.begin(), v.end());  // ERROR: sort is undeclared
    
    // Even cout is undeclared (except we whitelisted it)
    std::cout << "This might work if we whitelisted cout\n";
}

// ============================================================================
// Unsafe function can use STL freely
// ============================================================================

// @unsafe
void unsafe_with_stl() {
    // All STL usage is allowed in unsafe functions
    std::vector<int> vec;
    vec.push_back(42);
    
    std::string str = "test";
    auto len = str.length();
    
    auto ptr = std::make_unique<int>(42);
    
    std::vector<int> v = {3, 1, 2};
    std::sort(v.begin(), v.end());
    
    std::cout << "Unsafe function can use all STL freely\n";
}

// ============================================================================
// Undeclared function (default) can also use STL
// ============================================================================

void undeclared_with_stl() {
    // Undeclared functions are not checked, so STL usage is fine
    std::vector<std::string> names = {"Alice", "Bob", "Charlie"};
    for (const auto& name : names) {
        std::cout << "Hello, " << name << "\n";
    }
}

// ============================================================================
// Solution: Create safe wrappers or mark STL functions as safe/unsafe
// ============================================================================

// If you need STL in safe code, you have options:
// 1. Create safe wrapper functions marked @safe
// 2. Use external annotations to mark specific STL functions as @safe
// 3. Move STL usage to @unsafe functions and expose safe interfaces

// Example safe wrapper:
// @safe
int safe_vector_size(/* would need safe reference type */) {
    // In practice, you'd need proper lifetime annotations
    // This is just conceptual
    return 0;
}

int main() {
    std::cout << "STL Safety Demo\n";
    std::cout << "================\n";
    
    // Main is undeclared, so it can call anything
    safe_with_stl_attempt();
    unsafe_with_stl();
    undeclared_with_stl();
    
    return 0;
}

// ============================================================================
// Key Insight:
// 
// The STL is massive and mostly unaudited from a borrow-checking perspective.
// By treating it as undeclared by default, we force developers to:
// 1. Explicitly audit and mark STL functions they want to use
// 2. Create safe wrappers with proper lifetime management
// 3. Isolate STL usage in unsafe boundaries
//
// This prevents accidental misuse of STL functions that might violate
// borrow checking rules (like iterator invalidation).
// ============================================================================