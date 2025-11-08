// Tests for rusty::Barrier
#include "../include/rusty/barrier.hpp"
#include <cassert>
#include <cstdio>
#include <thread>
#include <vector>
#include <chrono>
#include <atomic>
#include <algorithm>

using namespace rusty;
using namespace std::chrono_literals;

// Test basic barrier synchronization
void test_barrier_basic() {
    printf("test_barrier_basic: ");
    {
        const int NUM_THREADS = 4;
        Barrier barrier(NUM_THREADS);
        std::atomic<int> counter{0};
        std::atomic<int> before_barrier{0};
        std::atomic<int> after_barrier{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([&, i]() {
                // Phase 1: increment before barrier
                before_barrier++;
                std::this_thread::sleep_for(std::chrono::milliseconds(10 * (i + 1)));

                // Wait at barrier
                barrier.wait();

                // Phase 2: increment after barrier (all should reach here together)
                after_barrier++;
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(before_barrier == NUM_THREADS);
        assert(after_barrier == NUM_THREADS);
    }
    printf("PASS\n");
}

// Test leader selection
void test_barrier_leader() {
    printf("test_barrier_leader: ");
    {
        const int NUM_THREADS = 5;
        Barrier barrier(NUM_THREADS);
        std::atomic<int> leader_count{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([&]() {
                auto result = barrier.wait();
                if (result.is_leader()) {
                    leader_count++;
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(leader_count == 1);  // Exactly one leader
    }
    printf("PASS\n");
}

// Test multiple barrier uses
void test_barrier_reuse() {
    printf("test_barrier_reuse: ");
    {
        const int NUM_THREADS = 3;
        const int NUM_ITERATIONS = 5;
        Barrier barrier(NUM_THREADS);
        std::atomic<int> phase{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([&, i]() {
                for (int iter = 0; iter < NUM_ITERATIONS; ++iter) {
                    // Do some work
                    std::this_thread::sleep_for(10ms);

                    // Wait at barrier
                    auto result = barrier.wait();

                    // Leader increments phase
                    if (result.is_leader()) {
                        phase++;
                    }
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(phase == NUM_ITERATIONS);
    }
    printf("PASS\n");
}

// Test barrier with different arrival times
void test_barrier_staggered_arrival() {
    printf("test_barrier_staggered_arrival: ");
    {
        const int NUM_THREADS = 4;
        Barrier barrier(NUM_THREADS);
        std::vector<std::chrono::steady_clock::time_point> arrival_times(NUM_THREADS);
        std::vector<std::chrono::steady_clock::time_point> release_times(NUM_THREADS);

        std::vector<std::thread> threads;
        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([&, i]() {
                // Stagger arrival times
                std::this_thread::sleep_for(std::chrono::milliseconds(50 * i));
                arrival_times[i] = std::chrono::steady_clock::now();

                // Wait at barrier
                barrier.wait();

                release_times[i] = std::chrono::steady_clock::now();
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        // All threads should be released at approximately the same time
        auto max_release = *std::max_element(release_times.begin(), release_times.end());
        auto min_release = *std::min_element(release_times.begin(), release_times.end());
        auto release_spread = std::chrono::duration_cast<std::chrono::milliseconds>(
            max_release - min_release);

        // Release spread should be very small (< 100ms)
        assert(release_spread < 100ms);
    }
    printf("PASS\n");
}

// Test barrier with work phases
void test_barrier_work_phases() {
    printf("test_barrier_work_phases: ");
    {
        const int NUM_THREADS = 3;
        Barrier barrier(NUM_THREADS);
        std::vector<int> shared_data(NUM_THREADS, 0);

        std::vector<std::thread> threads;
        for (int i = 0; i < NUM_THREADS; ++i) {
            threads.emplace_back([&, i]() {
                // Phase 1: Each thread writes its own index
                shared_data[i] = i + 1;
                barrier.wait();

                // Phase 2: All threads can now safely read all data
                int sum = 0;
                for (int val : shared_data) {
                    sum += val;
                }
                assert(sum == 6);  // 1 + 2 + 3 = 6

                barrier.wait();

                // Phase 3: Leader doubles all values
                auto result = barrier.wait();
                if (result.is_leader()) {
                    for (int& val : shared_data) {
                        val *= 2;
                    }
                }

                barrier.wait();

                // Phase 4: Everyone verifies doubled values
                int sum2 = 0;
                for (int val : shared_data) {
                    sum2 += val;
                }
                assert(sum2 == 12);  // 2 + 4 + 6 = 12
            });
        }

        for (auto& t : threads) {
            t.join();
        }
    }
    printf("PASS\n");
}

int main() {
    printf("Running Barrier tests...\n");
    printf("=======================\n");

    test_barrier_basic();
    test_barrier_leader();
    test_barrier_reuse();
    test_barrier_staggered_arrival();
    test_barrier_work_phases();

    printf("\nAll Barrier tests passed!\n");
    return 0;
}
