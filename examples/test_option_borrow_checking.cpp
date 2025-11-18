#include <rusty/option.hpp>
#include <string>
#include <iostream>

// Test 1: Multiple immutable borrows - SHOULD BE OK in Rust
// @safe
void test_multiple_immutable_borrows() {
    std::cout << "Test 1: Multiple immutable borrows\n";

    auto opt = rusty::Some<std::string>("hello");

    auto ref1 = opt.as_ref();
    auto ref2 = opt.as_ref();

    if (ref1.is_some() && ref2.is_some()) {
        const auto& s1 = ref1.unwrap();
        const auto& s2 = ref2.unwrap();

        std::cout << "  ref1: " << s1 << "\n";
        std::cout << "  ref2: " << s2 << "\n";
    }

    // Should be OK - immutable borrows ended
    auto value = opt.unwrap();
    std::cout << "  Moved out: " << value << "\n";
}

// Test 2: Multiple mutable borrows - SHOULD ERROR in Rust
// @safe
void test_multiple_mutable_borrows() {
    std::cout << "\nTest 2: Multiple mutable borrows (should ERROR)\n";

    auto opt = rusty::Some<std::string>("hello");

    auto mut1 = opt.as_mut();
    auto mut2 = opt.as_mut();  // ERROR: second mutable borrow

    if (mut1.is_some()) {
        auto& s1 = mut1.unwrap();
        s1.append(" world");
    }

    if (mut2.is_some()) {
        auto& s2 = mut2.unwrap();
        s2.append("!");
    }
}

// Test 3: Mixing immutable and mutable borrows - SHOULD ERROR in Rust
// @safe
void test_mixed_borrows() {
    std::cout << "\nTest 3: Mixed immutable and mutable borrows (should ERROR)\n";

    auto opt = rusty::Some<std::string>("hello");

    auto ref_opt = opt.as_ref();
    auto mut_opt = opt.as_mut();  // ERROR: can't have mut borrow while immutable exists

    if (ref_opt.is_some()) {
        const auto& s = ref_opt.unwrap();
        std::cout << "  Immutable: " << s << "\n";
    }

    if (mut_opt.is_some()) {
        auto& s = mut_opt.unwrap();
        s.append(" world");
    }
}

// Test 4: Moving after immutable borrow - SHOULD ERROR in Rust
// @safe
void test_move_after_immutable_borrow() {
    std::cout << "\nTest 4: Moving after immutable borrow (should ERROR)\n";

    auto opt = rusty::Some<std::string>("hello");

    auto ref_opt = opt.as_ref();

    // ERROR: can't move while borrowed
    auto moved = opt.unwrap();

    if (ref_opt.is_some()) {
        const auto& s = ref_opt.unwrap();  // Would be dangling!
        std::cout << "  Borrowed: " << s << "\n";
    }
}

// Test 5: Moving after mutable borrow - SHOULD ERROR in Rust
// @safe
void test_move_after_mutable_borrow() {
    std::cout << "\nTest 5: Moving after mutable borrow (should ERROR)\n";

    auto opt = rusty::Some<std::string>("hello");

    auto mut_opt = opt.as_mut();

    // ERROR: can't move while borrowed
    auto moved = opt.unwrap();

    if (mut_opt.is_some()) {
        auto& s = mut_opt.unwrap();  // Would be dangling!
        s.append(" world");
    }
}

// Test 6: Borrow scope ends - SHOULD BE OK in Rust
// @safe
void test_borrow_scope_ends() {
    std::cout << "\nTest 6: Borrow scope ends (should be OK)\n";

    auto opt = rusty::Some<std::string>("hello");

    {
        auto ref_opt = opt.as_ref();
        if (ref_opt.is_some()) {
            const auto& s = ref_opt.unwrap();
            std::cout << "  Borrowed: " << s << "\n";
        }
        // ref_opt goes out of scope here
    }

    // Should be OK - borrow ended
    auto value = opt.unwrap();
    std::cout << "  Moved out: " << value << "\n";
}

// Test 7: Using original while borrowed - SHOULD ERROR in Rust
// @safe
void test_use_original_while_borrowed() {
    std::cout << "\nTest 7: Using original while borrowed (should ERROR)\n";

    auto opt = rusty::Some<std::string>("hello");

    auto ref_opt = opt.as_ref();

    // ERROR: can't call methods on opt while borrowed
    auto another_ref = opt.as_ref();  // This creates another borrow

    if (ref_opt.is_some() && another_ref.is_some()) {
        std::cout << "  Both valid\n";
    }
}

// Test 8: Mutable borrow then immutable - SHOULD ERROR in Rust
// @safe
void test_mutable_then_immutable() {
    std::cout << "\nTest 8: Mutable then immutable borrow (should ERROR)\n";

    auto opt = rusty::Some<std::string>("hello");

    auto mut_opt = opt.as_mut();
    auto ref_opt = opt.as_ref();  // ERROR: can't borrow as immutable while mut borrow exists

    if (mut_opt.is_some()) {
        auto& s = mut_opt.unwrap();
        s.append(" world");
    }

    if (ref_opt.is_some()) {
        const auto& s = ref_opt.unwrap();
        std::cout << "  Immutable: " << s << "\n";
    }
}

// Test 9: Sequential mutable borrows - SHOULD BE OK in Rust
// @safe
void test_sequential_mutable_borrows() {
    std::cout << "\nTest 9: Sequential mutable borrows (should be OK)\n";

    auto opt = rusty::Some<std::string>("hello");

    {
        auto mut1 = opt.as_mut();
        if (mut1.is_some()) {
            auto& s = mut1.unwrap();
            s.append(" world");
        }
        // mut1 goes out of scope
    }

    {
        auto mut2 = opt.as_mut();
        if (mut2.is_some()) {
            auto& s = mut2.unwrap();
            s.append("!");
        }
        // mut2 goes out of scope
    }

    std::cout << "  Final: " << opt.unwrap() << "\n";
}

// Test 10: Borrow in one branch, move in another - Complex in Rust
// @safe
void test_conditional_borrow(bool condition) {
    std::cout << "\nTest 10: Conditional borrow\n";

    auto opt = rusty::Some<std::string>("hello");

    if (condition) {
        auto ref_opt = opt.as_ref();
        if (ref_opt.is_some()) {
            const auto& s = ref_opt.unwrap();
            std::cout << "  Borrowed: " << s << "\n";
        }
    } else {
        auto value = opt.unwrap();
        std::cout << "  Moved: " << value << "\n";
    }
}

// @safe
int main() {
    std::cout << "=== Testing Option Borrow Checking ===\n\n";

    test_multiple_immutable_borrows();

    // These should show errors:
    // test_multiple_mutable_borrows();
    // test_mixed_borrows();
    // test_move_after_immutable_borrow();
    // test_move_after_mutable_borrow();

    test_borrow_scope_ends();

    // test_use_original_while_borrowed();
    // test_mutable_then_immutable();

    test_sequential_mutable_borrows();

    test_conditional_borrow(true);
    test_conditional_borrow(false);

    std::cout << "\n=== Tests completed ===\n";
    return 0;
}
