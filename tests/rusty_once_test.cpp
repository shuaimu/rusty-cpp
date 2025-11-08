// Tests for rusty::Once and rusty::OnceCell<T>
#include "../include/rusty/once.hpp"
#include <cassert>
#include <cstdio>
#include <thread>
#include <vector>
#include <atomic>
#include <chrono>

using namespace rusty;
using namespace std::chrono_literals;

// Test Once basic functionality
void test_once_basic() {
    printf("test_once_basic: ");
    {
        Once once;
        int count = 0;

        once.call_once([&]() {
            count++;
        });

        assert(count == 1);

        // Second call should not execute
        once.call_once([&]() {
            count++;
        });

        assert(count == 1);  // Still 1, not 2
    }
    printf("PASS\n");
}

// Test Once with multiple threads
void test_once_multithreaded() {
    printf("test_once_multithreaded: ");
    {
        Once once;
        std::atomic<int> init_count{0};
        std::atomic<int> call_count{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < 10; ++i) {
            threads.emplace_back([&]() {
                call_count++;
                once.call_once([&]() {
                    init_count++;
                    std::this_thread::sleep_for(10ms);  // Simulate work
                });
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(call_count == 10);   // All threads called call_once
        assert(init_count == 1);    // But initialization happened exactly once
    }
    printf("PASS\n");
}

// Test Once for global initialization
static Once GLOBAL_INIT;
static int* global_resource = nullptr;

void test_once_global() {
    printf("test_once_global: ");
    {
        std::vector<std::thread> threads;

        for (int i = 0; i < 5; ++i) {
            threads.emplace_back([i]() {
                GLOBAL_INIT.call_once([]() {
                    global_resource = new int(42);
                });
                assert(global_resource != nullptr);
                assert(*global_resource == 42);
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        delete global_resource;
        global_resource = nullptr;
    }
    printf("PASS\n");
}

// Test OnceCell basic functionality
void test_once_cell_basic() {
    printf("test_once_cell_basic: ");
    {
        OnceCell<int> cell;

        assert(!cell.is_initialized());
        assert(cell.get() == nullptr);

        bool success = cell.set(42);
        assert(success);
        assert(cell.is_initialized());

        const int* value = cell.get();
        assert(value != nullptr);
        assert(*value == 42);

        // Second set should fail
        bool success2 = cell.set(100);
        assert(!success2);
        assert(*cell.get() == 42);  // Value unchanged
    }
    printf("PASS\n");
}

// Test OnceCell with multiple threads
void test_once_cell_multithreaded() {
    printf("test_once_cell_multithreaded: ");
    {
        OnceCell<std::string> cell;
        std::atomic<int> success_count{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < 10; ++i) {
            threads.emplace_back([&, i]() {
                bool success = cell.set("value-" + std::to_string(i));
                if (success) {
                    success_count++;
                }

                // Everyone should see a value
                const std::string* value = cell.get();
                assert(value != nullptr);
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(success_count == 1);  // Exactly one thread succeeded
        assert(cell.is_initialized());

        const std::string* final_value = cell.get();
        assert(final_value != nullptr);
        assert(final_value->find("value-") == 0);  // Should start with "value-"
    }
    printf("PASS\n");
}

// Test OnceCell get_or_init
void test_once_cell_get_or_init() {
    printf("test_once_cell_get_or_init: ");
    {
        OnceCell<int> cell;
        std::atomic<int> init_count{0};

        auto initializer = [&]() {
            init_count++;
            return 42;
        };

        // First call initializes
        const int& value1 = cell.get_or_init(initializer);
        assert(value1 == 42);
        assert(init_count == 1);

        // Second call returns existing value
        const int& value2 = cell.get_or_init(initializer);
        assert(value2 == 42);
        assert(init_count == 1);  // Initializer not called again
    }
    printf("PASS\n");
}

// Test OnceCell get_or_init with multiple threads
void test_once_cell_get_or_init_multithreaded() {
    printf("test_once_cell_get_or_init_multithreaded: ");
    {
        OnceCell<std::vector<int>> cell;
        std::atomic<int> init_count{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < 10; ++i) {
            threads.emplace_back([&]() {
                const std::vector<int>& vec = cell.get_or_init([&]() {
                    init_count++;
                    std::this_thread::sleep_for(10ms);
                    return std::vector<int>{1, 2, 3};
                });

                assert(vec.size() == 3);
                assert(vec[0] == 1);
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(init_count == 1);  // Exactly one initialization
    }
    printf("PASS\n");
}

// Test OnceCell get_mut
void test_once_cell_get_mut() {
    printf("test_once_cell_get_mut: ");
    {
        OnceCell<std::vector<int>> cell;

        assert(cell.get_mut() == nullptr);

        cell.set(std::vector<int>{1, 2, 3});

        std::vector<int>* vec = cell.get_mut();
        assert(vec != nullptr);
        assert(vec->size() == 3);

        vec->push_back(4);
        assert(vec->size() == 4);

        // Verify through const get
        const std::vector<int>* const_vec = cell.get();
        assert(const_vec->size() == 4);
    }
    printf("PASS\n");
}

// Test OnceCell with complex type
void test_once_cell_complex_type() {
    printf("test_once_cell_complex_type: ");
    {
        struct Config {
            std::string name;
            int version;
            std::vector<std::string> options;

            Config(std::string n, int v, std::vector<std::string> opts)
                : name(std::move(n)), version(v), options(std::move(opts)) {}
        };

        OnceCell<Config> cell;

        bool success = cell.set(Config("app", 1, {"opt1", "opt2"}));
        assert(success);

        const Config* config = cell.get();
        assert(config != nullptr);
        assert(config->name == "app");
        assert(config->version == 1);
        assert(config->options.size() == 2);
    }
    printf("PASS\n");
}

int main() {
    printf("Running Once and OnceCell tests...\n");
    printf("==================================\n");

    test_once_basic();
    test_once_multithreaded();
    test_once_global();
    test_once_cell_basic();
    test_once_cell_multithreaded();
    test_once_cell_get_or_init();
    test_once_cell_get_or_init_multithreaded();
    test_once_cell_get_mut();
    test_once_cell_complex_type();

    printf("\nAll Once and OnceCell tests passed!\n");
    return 0;
}
