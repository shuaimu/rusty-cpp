// Full compile test: stripped from transpiled either.cppm output
// Removed: module declarations, using declarations for non-existent std:: namespaces,
//          proxy facades, trait impls
// Kept: core Either type, test cases

#include <cstdint>
#include <cstddef>
#include <variant>
#include <string>
#include <cassert>
#include <iostream>
#include <optional>
#include <functional>

// ════════════════════════════════════════════════════
// From either.cppm — core Either type
// ════════════════════════════════════════════════════

// Algebraic data type
template<typename L, typename R>
struct Either_Left {
    L _0;
};
template<typename L, typename R>
struct Either_Right {
    R _0;
};
template<typename L, typename R>
using Either = std::variant<Either_Left<L, R>, Either_Right<L, R>>;

// Overloaded visitor helper
template<class... Ts> struct overloaded : Ts... { using Ts::operator()...; };

// Constructor helpers (equivalent to Rust's Left() and Right())
template<typename L, typename R>
Either<L, R> Left(L val) {
    return Either_Left<L, R>{std::move(val)};
}
template<typename L, typename R>
Either<L, R> Right(R val) {
    return Either_Right<L, R>{std::move(val)};
}

// is_left / is_right helpers
template<typename L, typename R>
bool is_left(const Either<L, R>& e) {
    return std::holds_alternative<Either_Left<L, R>>(e);
}
template<typename L, typename R>
bool is_right(const Either<L, R>& e) {
    return std::holds_alternative<Either_Right<L, R>>(e);
}

// ════════════════════════════════════════════════════
// Tests (transpiled from either's #[test] functions)
// ════════════════════════════════════════════════════

void test_basic() {
    auto e = Left<int32_t, int32_t>(2);
    const auto r = Right<int32_t, int32_t>(2);

    assert(is_left(e));
    assert((std::get<Either_Left<int32_t, int32_t>>(e)._0 == 2));

    e = r;
    assert(is_right(e));
    assert((std::get<Either_Right<int32_t, int32_t>>(e)._0 == 2));

    std::cout << "test_basic PASSED" << std::endl;
}

void test_visit() {
    Either<int32_t, std::string> val = Left<int32_t, std::string>(42);

    std::visit(overloaded{
        [](const Either_Left<int32_t, std::string>& v) {
            assert(v._0 == 42);
        },
        [](const Either_Right<int32_t, std::string>& v) {
            assert(false && "should be Left");
        },
    }, val);

    val = Right<int32_t, std::string>("hello");

    std::visit(overloaded{
        [](const Either_Left<int32_t, std::string>& v) {
            assert(false && "should be Right");
        },
        [](const Either_Right<int32_t, std::string>& v) {
            assert(v._0 == "hello");
        },
    }, val);

    std::cout << "test_visit PASSED" << std::endl;
}

void test_generic() {
    // Either with different types
    Either<double, int32_t> a = Left<double, int32_t>(3.14);
    Either<double, int32_t> b = Right<double, int32_t>(42);

    assert(is_left(a));
    assert(is_right(b));

    std::cout << "test_generic PASSED" << std::endl;
}

int main() {
    test_basic();
    test_visit();
    test_generic();
    std::cout << "\nAll either compile tests PASSED" << std::endl;
    return 0;
}
