// Comprehensive Multi-Threaded Safety Tests for rusty::Arc<T>
//
// This test suite verifies thread-safety guarantees of Arc<T>:
// - Atomic reference counting under high contention
// - Memory ordering and synchronization
// - Concurrent clone/drop operations
// - Weak reference thread-safety
// - Immutability guarantees (data race prevention)
// - Exclusive access via get_mut()

#include "../include/rusty/arc.hpp"
#include "../include/rusty/weak.hpp"
#include "../include/rusty/mutex.hpp"
#include "../include/rusty/vec.hpp"
#include <cassert>
#include <cstdio>
#include <thread>
#include <vector>
#include <atomic>
#include <chrono>
#include <random>
#include <algorithm>

using namespace rusty;

// ============================================================================
// Test 1: Concurrent Cloning Stress Test
// ============================================================================
void test_concurrent_cloning_stress() {
    printf("test_concurrent_cloning_stress: ");

    constexpr int NUM_THREADS = 50;
    constexpr int CLONES_PER_THREAD = 1000;

    auto arc = Arc<int>::make(42);
    std::vector<std::thread> threads;
    std::atomic<int> success_count{0};

    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([arc, &success_count]() {
            for (int j = 0; j < CLONES_PER_THREAD; ++j) {
                auto local = arc.clone();
                assert(local.is_valid());
                assert(*local == 42);
                success_count.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    assert(success_count.load() == NUM_THREADS * CLONES_PER_THREAD);
    assert(arc.is_valid());
    assert(arc.strong_count() == 1);

    printf("PASS\n");
}

// ============================================================================
// Test 2: Concurrent Destruction
// ============================================================================
void test_concurrent_destruction() {
    printf("test_concurrent_destruction: ");

    constexpr int NUM_THREADS = 20;
    std::atomic<bool> all_started{false};

    for (int iteration = 0; iteration < 10; ++iteration) {
        auto arc = Arc<int>::make(iteration);
        std::vector<std::thread> threads;
        std::vector<Arc<int>> initial_clones;

        for (int i = 0; i < NUM_THREADS; ++i) {
            initial_clones.push_back(arc.clone());
        }

        assert(arc.strong_count() == NUM_THREADS + 1);

        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([clone = std::move(initial_clones[i]), &all_started]() mutable {
                while (!all_started.load(std::memory_order_acquire)) {
                    std::this_thread::yield();
                }
                assert(clone.is_valid());
            });
        }

        all_started.store(true, std::memory_order_release);

        for (auto& t : threads) {
            t.join();
        }

        all_started.store(false, std::memory_order_relaxed);
        assert(arc.strong_count() == 1);
    }

    printf("PASS\n");
}

// ============================================================================
// Test 3: High Contention Reference Counting
// ============================================================================
void test_high_contention_refcount() {
    printf("test_high_contention_refcount: ");

    constexpr int NUM_THREADS = 30;
    constexpr int ITERATIONS = 500;

    auto arc = Arc<int>::make(99);
    std::vector<std::thread> threads;

    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([arc]() {
            for (int j = 0; j < ITERATIONS; ++j) {
                auto clone1 = arc.clone();
                auto clone2 = arc.clone();
                assert(*clone1 == 99);
                assert(*clone2 == 99);
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    assert(arc.strong_count() == 1);

    printf("PASS\n");
}

// ============================================================================
// Test 4: Immutability Guarantee
// ============================================================================
void test_immutability_guarantee() {
    printf("test_immutability_guarantee: ");

    struct SharedData {
        int value;
        std::vector<int> numbers;
        SharedData(int v) : value(v), numbers{1, 2, 3, 4, 5} {}
    };

    auto arc = Arc<SharedData>::make(42);
    std::vector<std::thread> threads;
    std::atomic<int> read_count{0};

    constexpr int NUM_THREADS = 20;
    constexpr int READS_PER_THREAD = 1000;

    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([arc, &read_count]() {
            for (int j = 0; j < READS_PER_THREAD; ++j) {
                assert(arc->value == 42);
                assert(arc->numbers.size() == 5);
                read_count.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    assert(read_count.load() == NUM_THREADS * READS_PER_THREAD);
    printf("PASS\n");
}

// ============================================================================
// Test 5: get_mut() Exclusive Access
// ============================================================================
void test_get_mut_exclusivity() {
    printf("test_get_mut_exclusivity: ");

    auto arc = Arc<int>::make(100);

    int* mut_ptr = arc.get_mut();
    assert(mut_ptr != nullptr);
    *mut_ptr = 200;

    auto clone = arc.clone();
    assert(arc.get_mut() == nullptr);

    std::atomic<bool> all_checked{false};
    std::thread checker([clone, &all_checked]() mutable {
        assert(clone.get_mut() == nullptr);
        all_checked.store(true, std::memory_order_release);
    });

    assert(arc.get_mut() == nullptr);
    checker.join();

    printf("PASS\n");
}

// ============================================================================
// Test 6: Weak Reference Concurrent Upgrades
// ============================================================================
void test_weak_concurrent_upgrades() {
    printf("test_weak_concurrent_upgrades: ");

    constexpr int NUM_THREADS = 20;
    std::atomic<int> upgrade_success{0};

    {
        auto arc = Arc<int>::make(777);
        auto weak = downgrade(arc);
        std::vector<std::thread> threads;

        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([weak, &upgrade_success]() mutable {
                for (int j = 0; j < 100; ++j) {
                    auto maybe_arc = weak.upgrade();
                    if (maybe_arc.is_some()) {
                        auto strong = maybe_arc.unwrap();
                        assert(*strong == 777);
                        upgrade_success.fetch_add(1, std::memory_order_relaxed);
                    }
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(upgrade_success.load() == NUM_THREADS * 100);
    }

    printf("PASS\n");
}

// ============================================================================
// Test 7: Cross-Thread Ownership Transfer
// ============================================================================
void test_cross_thread_transfer() {
    printf("test_cross_thread_transfer: ");

    std::atomic<bool> received{false};
    std::atomic<int> final_value{0};

    auto arc = Arc<int>::make(123);

    std::thread worker([arc = std::move(arc), &received, &final_value]() mutable {
        assert(arc.is_valid());
        int* mut = arc.get_mut();
        assert(mut != nullptr);
        *mut = 456;
        final_value.store(*arc, std::memory_order_release);
        received.store(true, std::memory_order_release);
    });

    worker.join();
    assert(received.load());
    assert(final_value.load() == 456);

    printf("PASS\n");
}

// ============================================================================
// Test 8: Arc<Mutex<T>> Pattern
// ============================================================================
void test_arc_mutex_pattern() {
    printf("test_arc_mutex_pattern: ");

    constexpr int NUM_THREADS = 20;
    constexpr int INCREMENTS = 500;

    auto counter = Arc<Mutex<int>>::make(0);
    std::vector<std::thread> threads;

    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([counter]() {
            for (int j = 0; j < INCREMENTS; ++j) {
                auto guard = counter->lock();
                *guard += 1;
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    auto final_guard = counter->lock();
    assert(*final_guard == NUM_THREADS * INCREMENTS);

    printf("PASS\n");
}

// ============================================================================
// Test 9: Maximum Concurrency Stress
// ============================================================================
void test_maximum_concurrency_stress() {
    printf("test_maximum_concurrency_stress: ");

    constexpr int NUM_THREADS = 100;
    constexpr int OPERATIONS = 100;

    struct Counter {
        mutable std::atomic<uint64_t> value{0};
    };

    auto arc = Arc<Counter>::make();
    std::vector<std::thread> threads;

    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([arc]() {
            for (int j = 0; j < OPERATIONS; ++j) {
                auto c1 = arc.clone();
                auto c2 = arc.clone();
                c1->value.fetch_add(1, std::memory_order_relaxed);
                auto w = downgrade(c2);
                auto maybe = w.upgrade();
                assert(maybe.is_some());
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    assert(arc->value.load(std::memory_order_relaxed) == NUM_THREADS * OPERATIONS);
    assert(arc.strong_count() == 1);

    printf("PASS\n");
}

// ============================================================================
// Main
// ============================================================================
int main() {
    printf("========================================\n");
    printf("Arc<T> Multi-Threaded Safety Test Suite\n");
    printf("========================================\n\n");

    auto start = std::chrono::high_resolution_clock::now();

    test_concurrent_cloning_stress();
    test_concurrent_destruction();
    test_high_contention_refcount();
    test_immutability_guarantee();
    test_get_mut_exclusivity();
    test_weak_concurrent_upgrades();
    test_cross_thread_transfer();
    test_arc_mutex_pattern();
    test_maximum_concurrency_stress();

    auto end = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end - start);

    printf("\n========================================\n");
    printf("All 9 multi-threaded tests PASSED!\n");
    printf("Total time: %ld ms\n", duration.count());
    printf("========================================\n");

    return 0;
}
