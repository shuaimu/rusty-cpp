#include <iostream>
#include <vector>
#include <string>

// =============================================================================
// Example: Safe Functions with Borrow Checking
// =============================================================================

// @safe
void demonstrate_borrowing() {
    int value = 42;

    // Multiple immutable borrows are allowed within a scope
    {
        const int& ref1 = value;
        const int& ref2 = value;

        // @unsafe
        {
            std::cout << "Multiple immutable borrows: " << ref1 << ", " << ref2 << "\n";
        }
    }
    // Borrows end here, now we can mutate

    value = 100;

    // @unsafe
    {
        std::cout << "After mutation: " << value << "\n";
    }
}

// @safe
void demonstrate_move_detection() {
    // @unsafe
    {
        // STL operations require unsafe blocks
        std::string s = "hello";
        std::string s2 = std::move(s);
        std::cout << "Moved string: " << s2 << "\n";
        // Using 's' here would be caught by rusty-cpp
    }
}

// @safe
void demonstrate_scope_safety() {
    int outer = 10;

    {
        int inner = 20;
        const int& ref_to_inner = inner;

        // @unsafe
        {
            std::cout << "Inner scope: " << ref_to_inner << "\n";
        }
        // ref_to_inner goes out of scope here, which is safe
    }

    // outer is still valid
    // @unsafe
    {
        std::cout << "Outer value: " << outer << "\n";
    }
}

// Entry point called from main
// @safe
void demonstrate_safe_code() {
    // @unsafe
    {
        std::cout << "1. Demonstrating borrow checking:\n";
    }
    demonstrate_borrowing();

    // @unsafe
    {
        std::cout << "\n2. Demonstrating move detection:\n";
    }
    demonstrate_move_detection();

    // @unsafe
    {
        std::cout << "\n3. Demonstrating scope safety:\n";
    }
    demonstrate_scope_safety();

    // @unsafe
    {
        std::cout << "\nAll safety checks passed!\n";
    }
}
