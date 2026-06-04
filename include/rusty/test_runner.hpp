#pragma once
// rusty::test_runner — TEST_CASE macro + auto-registering test harness for
// transpiled rustc test files. The transpiler emits `#[test] fn name() { ... }`
// as `TEST_CASE("name") { ... }`; this header defines that macro to register
// each test in a global vector at static-init time, runnable via
// rusty_test_runner::run_all().

#include <cstdio>
#include <cstdlib>
#include <exception>
#include <string>
#include <utility>
#include <vector>

namespace rusty_test_runner {

using TestFn = void (*)();

struct Registry {
    std::vector<std::pair<std::string, TestFn>> tests;
};

inline Registry& registry() {
    static Registry r;
    return r;
}

inline bool register_test(const char* name, TestFn fn) {
    registry().tests.emplace_back(name, fn);
    return true;
}

inline int run_all() {
    auto& tests = registry().tests;
    std::printf("running %zu test(s)\n", tests.size());
    int failed = 0;
    for (auto& [name, fn] : tests) {
        std::printf("  test %s ... ", name.c_str());
        std::fflush(stdout);
        try {
            fn();
            std::printf("ok\n");
        } catch (const std::exception& e) {
            std::printf("FAILED: %s\n", e.what());
            ++failed;
        } catch (...) {
            std::printf("FAILED: unknown exception\n");
            ++failed;
        }
    }
    if (failed == 0) {
        std::printf("\nall %zu test(s) passed\n", tests.size());
        return 0;
    }
    std::printf("\n%d of %zu test(s) FAILED\n", failed, tests.size());
    return 1;
}

} // namespace rusty_test_runner

#define RUSTY_TEST_CONCAT_(a, b) a##b
#define RUSTY_TEST_CONCAT(a, b) RUSTY_TEST_CONCAT_(a, b)

#define TEST_CASE(name)                                                            \
    static void RUSTY_TEST_CONCAT(rusty_test_body_, __LINE__)();                   \
    static const bool RUSTY_TEST_CONCAT(rusty_test_reg_, __LINE__) =               \
        ::rusty_test_runner::register_test(                                        \
            name, RUSTY_TEST_CONCAT(rusty_test_body_, __LINE__));                  \
    static void RUSTY_TEST_CONCAT(rusty_test_body_, __LINE__)()
