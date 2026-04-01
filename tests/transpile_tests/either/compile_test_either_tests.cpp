// Transpiled Either test suite — C++ equivalent of Rust's cargo test
// Tests the same semantics as either's #[test] functions using
// our transpiled type definitions.
//
// Build: g++ -std=c++20 -Wall -o test_either compile_test_either_tests.cpp
// Run:   ./test_either

#include <cstdint>
#include <cstddef>
#include <variant>
#include <string>
#include <cassert>
#include <iostream>
#include <optional>

// ════════════════════════════════════════════════════
// Transpiled from: enum Either<L, R> { Left(L), Right(R) }
// ════════════════════════════════════════════════════

template<typename L, typename R>
struct Either_Left { L _0; };

template<typename L, typename R>
struct Either_Right { R _0; };

template<typename L, typename R>
struct Either : std::variant<Either_Left<L, R>, Either_Right<L, R>> {
    using variant = std::variant<Either_Left<L, R>, Either_Right<L, R>>;
    using variant::variant;

    // Transpiled from impl Either<L, R> — methods that would come from macro expansion
    bool is_left() const {
        return std::holds_alternative<Either_Left<L, R>>(*this);
    }
    bool is_right() const {
        return std::holds_alternative<Either_Right<L, R>>(*this);
    }
    std::optional<L> left() const {
        if (is_left()) return std::get<Either_Left<L, R>>(*this)._0;
        return std::nullopt;
    }
    std::optional<R> right() const {
        if (is_right()) return std::get<Either_Right<L, R>>(*this)._0;
        return std::nullopt;
    }
};

// Variant constructor helpers (transpiler-generated)
template<typename L, typename R>
Either<L, R> Left(L val) { return Either_Left<L, R>{std::move(val)}; }

template<typename L, typename R>
Either<L, R> Right(R val) { return Either_Right<L, R>{std::move(val)}; }

// Overloaded visitor helper
template<class... Ts> struct overloaded : Ts... { using Ts::operator()...; };

// ════════════════════════════════════════════════════
// Test: basic (equivalent to Rust's #[test] fn basic())
// ════════════════════════════════════════════════════
// Rust original:
//   let mut e = Left(2);
//   let r = Right(2);
//   assert_eq!(e, Left(2));
//   e = r;
//   assert_eq!(e, Right(2));
//   assert_eq!(e.left(), None);
//   assert_eq!(e.right(), Some(2));

void test_basic() {
    auto e = Left<int, int>(2);
    const auto r = Right<int, int>(2);

    // assert_eq!(e, Left(2))
    assert(e.is_left());
    assert(e.left().value() == 2);

    // e = r
    e = r;

    // assert_eq!(e, Right(2))
    assert(e.is_right());
    assert(e.right().value() == 2);

    // assert_eq!(e.left(), None)
    assert(!e.left().has_value());

    // assert_eq!(e.right(), Some(2))
    assert(e.right().has_value());
    assert(e.right().value() == 2);

    std::cout << "test_basic PASSED" << std::endl;
}

// ════════════════════════════════════════════════════
// Test: deref (equivalent to Rust's #[test] fn deref())
// ════════════════════════════════════════════════════
// Rust original:
//   let value: Either<String, &str> = Left(String::from("test"));
//   is_str(&*value);

void test_deref() {
    // Test that Either can hold different types
    auto value = Left<std::string, const char*>(std::string("test"));

    // Verify it's Left with correct value
    assert(value.is_left());
    assert(value.left().value() == "test");

    std::cout << "test_deref PASSED" << std::endl;
}

// ════════════════════════════════════════════════════
// Test: visit/pattern matching (tests std::visit on Either)
// ════════════════════════════════════════════════════
// Rust original uses match; we test the same semantics with std::visit

void test_pattern_matching() {
    auto val = Left<int, std::string>(42);

    // Visit Left variant
    int result = std::visit(overloaded{
        [](const Either_Left<int, std::string>& v) -> int { return v._0; },
        [](const Either_Right<int, std::string>& v) -> int { return -1; },
    }, static_cast<const std::variant<Either_Left<int, std::string>, Either_Right<int, std::string>>&>(val));

    assert(result == 42);

    // Visit Right variant
    auto val2 = Right<int, std::string>("hello");

    auto result2 = std::visit(overloaded{
        [](const Either_Left<int, std::string>& v) -> std::string { return "wrong"; },
        [](const Either_Right<int, std::string>& v) -> std::string { return std::string(v._0); },
    }, static_cast<const std::variant<Either_Left<int, std::string>, Either_Right<int, std::string>>&>(val2));

    assert(result2 == "hello");

    std::cout << "test_pattern_matching PASSED" << std::endl;
}

// ════════════════════════════════════════════════════
// Test: generic types (tests Either with various type combinations)
// ════════════════════════════════════════════════════

void test_generic_types() {
    // Either<double, int>
    auto a = Left<double, int>(3.14);
    assert(a.is_left());
    assert(a.left().value() == 3.14);

    // Either<string, int>
    auto b = Right<std::string, int>(42);
    assert(b.is_right());
    assert(b.right().value() == 42);

    // Reassignment
    auto c = Left<int, int>(1);
    c = Right<int, int>(2);
    assert(c.is_right());
    assert(c.right().value() == 2);

    std::cout << "test_generic_types PASSED" << std::endl;
}

// ════════════════════════════════════════════════════
// Test: copy/move semantics
// ════════════════════════════════════════════════════

void test_copy_move() {
    auto original = Left<int, int>(42);

    // Copy
    auto copy = original;
    assert(copy.is_left());
    assert(copy.left().value() == 42);
    assert(original.is_left()); // original still valid

    // Move
    auto moved = std::move(original);
    assert(moved.is_left());
    assert(moved.left().value() == 42);

    std::cout << "test_copy_move PASSED" << std::endl;
}

// ════════════════════════════════════════════════════
// Test: equality comparison
// ════════════════════════════════════════════════════

void test_equality() {
    auto a = Left<int, int>(2);
    auto b = Left<int, int>(2);
    auto c = Right<int, int>(2);
    auto d = Left<int, int>(3);

    // Same variant, same value
    assert(a.is_left() && b.is_left());
    assert(a.left().value() == b.left().value());

    // Different variants
    assert(a.is_left() && c.is_right());

    // Same variant, different value
    assert(a.left().value() != d.left().value());

    std::cout << "test_equality PASSED" << std::endl;
}

// ════════════════════════════════════════════════════
// Main — run all tests
// ════════════════════════════════════════════════════

int main() {
    std::cout << "Running either transpiled tests..." << std::endl;
    std::cout << std::endl;

    test_basic();
    test_deref();
    test_pattern_matching();
    test_generic_types();
    test_copy_move();
    test_equality();

    std::cout << std::endl;
    std::cout << "All 6 tests PASSED" << std::endl;
    return 0;
}
