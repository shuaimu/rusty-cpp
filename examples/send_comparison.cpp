// Visual demonstration of Send trait differences between Rust and C++

#include <iostream>
#include <type_traits>
#include "rusty/rc.hpp"
#include "rusty/arc.hpp"

// Demonstrate what C++ type traits can and cannot detect

template<typename T>
void check_type(const char* name) {
    std::cout << "\nType: " << name << "\n";
    std::cout << "  is_move_constructible:   " << (std::is_move_constructible_v<T> ? "YES" : "NO") << "\n";
    std::cout << "  is_move_assignable:      " << (std::is_move_assignable_v<T> ? "YES" : "NO") << "\n";
    std::cout << "  is_trivially_copyable:   " << (std::is_trivially_copyable_v<T> ? "YES" : "NO") << "\n";
    std::cout << "  --> Our Send check:      " << (std::is_move_constructible_v<T> ? "PASS (but may be unsafe!)" : "FAIL") << "\n";
}

struct NotMovable {
    NotMovable() = default;
    NotMovable(NotMovable&&) = delete;  // Explicitly not movable
};

struct Movable {
    Movable() = default;
    Movable(Movable&&) = default;
};

struct HasRawPointer {
    int* ptr;  // Might point to thread-local data!
};

int main() {
    std::cout << "=== C++ Type Trait Limitations Demo ===\n";
    std::cout << "\nOur Send check only looks at move-constructibility.\n";
    std::cout << "This catches some errors but not all.\n";

    // ✅ Correctly rejected
    check_type<NotMovable>("NotMovable");
    std::cout << "  Rust:  !Send (not movable)\n";
    std::cout << "  C++:   FAIL - Correctly rejected! ✓\n";

    // ✅ Correctly accepted
    check_type<Movable>("Movable");
    std::cout << "  Rust:  Send (safe to move)\n";
    std::cout << "  C++:   PASS - Correctly accepted! ✓\n";

    // ✅ Correctly accepted
    check_type<int*>("int* (raw pointer)");
    std::cout << "  Rust:  Send (but unsafe to dereference)\n";
    std::cout << "  C++:   PASS - But we can't enforce unsafe! ⚠️\n";

    // ❌ Incorrectly accepted!
    check_type<rusty::Rc<int>>("rusty::Rc<int>");
    std::cout << "  Rust:  !Send (reference counting not thread-safe)\n";
    std::cout << "  C++:   PASS - WRONGLY ACCEPTED! ✗\n";
    std::cout << "         (Rc IS move-constructible, but NOT thread-safe)\n";

    // ✅ Correctly accepted
    check_type<rusty::Arc<int>>("rusty::Arc<int>");
    std::cout << "  Rust:  Send (atomic reference counting)\n";
    std::cout << "  C++:   PASS - Correctly accepted! ✓\n";

    // ❌ Cannot detect composite issues
    struct ContainsRc {
        rusty::Rc<int> rc;
    };
    check_type<ContainsRc>("struct { rusty::Rc<int> }");
    std::cout << "  Rust:  !Send (contains Rc which is !Send)\n";
    std::cout << "  C++:   PASS - WRONGLY ACCEPTED! ✗\n";
    std::cout << "         (C++ doesn't check nested types)\n";

    std::cout << "\n=== Summary ===\n";
    std::cout << "\nC++ type traits check SYNTAX (does it have move constructor?)\n";
    std::cout << "Rust Send trait checks SEMANTICS (is it safe to send?).\n";

    std::cout << "\nWhat C++ CANNOT detect:\n";
    std::cout << "  ✗ Rc<T> has non-atomic reference counting\n";
    std::cout << "  ✗ Raw pointers might reference thread-local data\n";
    std::cout << "  ✗ Nested types containing unsafe members\n";
    std::cout << "  ✗ Custom types with unsafe move semantics\n";

    std::cout << "\nWorkarounds:\n";
    std::cout << "  1. Use Arc<T> instead of Rc<T> for shared data\n";
    std::cout << "  2. Avoid raw pointers in channel types\n";
    std::cout << "  3. Test with ThreadSanitizer\n";
    std::cout << "  4. Document thread-safety requirements\n";

    std::cout << "\n=== Comparison Table ===\n";
    std::cout << "\n";
    std::cout << "Type                  | Rust Send | C++ Send Check | Safe? \n";
    std::cout << "----------------------|-----------|----------------|---------\n";
    std::cout << "NotMovable            | NO        | NO             | ✓ Caught\n";
    std::cout << "Movable               | YES       | YES            | ✓ Safe\n";
    std::cout << "int*                  | YES*      | YES            | ⚠️ Requires unsafe\n";
    std::cout << "Rc<int>               | NO        | YES            | ✗ Unsafe!\n";
    std::cout << "Arc<int>              | YES       | YES            | ✓ Safe\n";
    std::cout << "struct { Rc<int> }    | NO        | YES            | ✗ Unsafe!\n";
    std::cout << "\n* Raw pointers ARE Send in Rust, but require 'unsafe' to use\n";

    return 0;
}
