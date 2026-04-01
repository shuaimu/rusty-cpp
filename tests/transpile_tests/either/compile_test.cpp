// Minimal compile test for transpiled either core types
// Tests only the variant/struct definitions, not trait impls

#include <cstdint>
#include <cstddef>
#include <variant>
#include <string>
#include <iostream>

// ── Transpiled from enum Either<L, R> { Left(L), Right(R) } ──

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

// Helper to construct variants (like Rust's Left() and Right())
template<typename L, typename R>
Either<L, R> Left(L val) {
    return Either_Left<L, R>{std::move(val)};
}

template<typename L, typename R>
Either<L, R> Right(R val) {
    return Either_Right<L, R>{std::move(val)};
}

// ── Test ──

int main() {
    // Create Either values
    Either<int32_t, std::string> a = Left<int32_t, std::string>(42);
    Either<int32_t, std::string> b = Right<int32_t, std::string>("hello");

    // Pattern match with std::visit
    std::visit([](const auto& v) {
        if constexpr (requires { v._0; }) {
            std::cout << "value: " << v._0 << std::endl;
        }
    }, a);

    std::visit([](const auto& v) {
        if constexpr (requires { v._0; }) {
            std::cout << "value: " << v._0 << std::endl;
        }
    }, b);

    std::cout << "Either compile test PASSED" << std::endl;
    return 0;
}
