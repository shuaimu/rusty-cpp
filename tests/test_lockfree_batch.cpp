// Tests for lock-free MPSC channel (Phase 4: Batch operations and optimizations)

#include <iostream>
#include <thread>
#include <vector>
#include <chrono>
#include <memory>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

// Mark unique_ptr as Send for testing
namespace rusty {
    template<typename T>
    struct is_explicitly_send<std::unique_ptr<T>> : std::true_type {};
}

using namespace rusty::sync::mpsc::lockfree;
using namespace std::chrono;

// Test counter
static int tests_passed = 0;
static int tests_failed = 0;

#define TEST(name) \
    void test_##name(); \
    void run_test_##name() { \
        std::cout << "Running test: " #name "... "; \
        try { \
            test_##name(); \
            std::cout << "PASS" << std::endl; \
            tests_passed++; \
        } catch (const std::exception& e) { \
            std::cout << "FAIL: " << e.what() << std::endl; \
            tests_failed++; \
        } \
    } \
    void test_##name()

#define ASSERT(cond, msg) \
    if (!(cond)) { \
        throw std::runtime_error(msg); \
    }

// Test 1: Basic batch send
TEST(basic_batch_send) {
    auto [tx, rx] = channel<int>();

    std::vector<int> batch = {1, 2, 3, 4, 5};
    size_t sent = tx.send_batch(batch);

    ASSERT(sent == 5, "Should send all 5 messages");

    // Receive all
    for (int i = 1; i <= 5; ++i) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Should receive message");
        ASSERT(result.unwrap() == i, "Values should match");
    }
}

// Test 2: Empty batch send
TEST(empty_batch_send) {
    auto [tx, rx] = channel<int>();

    std::vector<int> empty_batch;
    size_t sent = tx.send_batch(empty_batch);

    ASSERT(sent == 0, "Should send 0 messages for empty batch");
    ASSERT(rx.is_empty(), "Queue should be empty");
}

// Test 3: Large batch send
TEST(large_batch_send) {
    auto [tx, rx] = channel<int>();

    std::vector<int> batch;
    for (int i = 0; i < 1000; ++i) {
        batch.push_back(i);
    }

    size_t sent = tx.send_batch(batch);
    ASSERT(sent == 1000, "Should send all 1000 messages");

    // Verify all received
    for (int i = 0; i < 1000; ++i) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Should receive message");
        ASSERT(result.unwrap() == i, "Values should match");
    }
}

// Test 4: Basic batch receive
TEST(basic_batch_recv) {
    auto [tx, rx] = channel<int>();

    // Send messages individually
    for (int i = 0; i < 10; ++i) {
        tx.send(i);
    }

    // Batch receive
    auto batch = rx.recv_batch(10);

    ASSERT(batch.size() == 10, "Should receive 10 messages");
    for (size_t i = 0; i < batch.size(); ++i) {
        ASSERT(batch[i] == static_cast<int>(i), "Values should match");
    }
}

// Test 5: Batch receive with partial results
TEST(batch_recv_partial) {
    auto [tx, rx] = channel<int>();

    // Send only 5 messages
    for (int i = 0; i < 5; ++i) {
        tx.send(i);
    }

    // Try to receive 10
    auto batch = rx.recv_batch(10);

    ASSERT(batch.size() == 5, "Should receive only 5 messages (what's available)");
}

// Test 6: Batch receive on empty queue
TEST(batch_recv_empty) {
    auto [tx, rx] = channel<int>();

    auto batch = rx.recv_batch(10);

    ASSERT(batch.empty(), "Should receive empty vector");
}

// Test 7: is_empty() helper
TEST(is_empty_helper) {
    auto [tx, rx] = channel<int>();

    ASSERT(rx.is_empty(), "Initially empty");

    tx.send(42);
    ASSERT(!rx.is_empty(), "Not empty after send");

    rx.try_recv();
    ASSERT(rx.is_empty(), "Empty after receive");
}

// Test 8: is_disconnected() helper
TEST(is_disconnected_helper) {
    auto [tx, rx] = channel<int>();

    ASSERT(!rx.is_disconnected(), "Initially connected");

    // Drop sender
    {
        auto tx_temp = std::move(tx);
        // tx_temp goes out of scope
    }

    ASSERT(rx.is_disconnected(), "Disconnected after sender dropped");
}

// Test 9: Batch send performance vs individual
TEST(batch_send_performance) {
    auto [tx, rx] = channel<int>();

    const int N = 1000;

    // Prepare batch
    std::vector<int> batch;
    for (int i = 0; i < N; ++i) {
        batch.push_back(i);
    }

    // Batch send
    auto start = high_resolution_clock::now();
    tx.send_batch(batch);
    auto batch_time = duration_cast<microseconds>(high_resolution_clock::now() - start);

    // Drain
    rx.drain();

    // Individual sends
    start = high_resolution_clock::now();
    for (int i = 0; i < N; ++i) {
        tx.send(i);
    }
    auto individual_time = duration_cast<microseconds>(high_resolution_clock::now() - start);

    std::cout << " (batch: " << batch_time.count() << " μs, "
              << "individual: " << individual_time.count() << " μs)";

    // Note: Batch operations reduce contention, not necessarily latency
    // For small batches in single-producer scenarios, individual sends may be faster
    // Real benefit is in high-contention multi-producer scenarios
    // This test is informational only
}

// Test 10: Batch receive performance vs individual
TEST(batch_recv_performance) {
    auto [tx, rx] = channel<int>();

    const int N = 1000;

    // Send messages
    for (int i = 0; i < N; ++i) {
        tx.send(i);
    }

    // Batch receive
    auto start = high_resolution_clock::now();
    auto batch = rx.recv_batch(N);
    auto batch_time = duration_cast<microseconds>(high_resolution_clock::now() - start);

    ASSERT(batch.size() == N, "Should receive all messages");

    // Send again for individual test
    for (int i = 0; i < N; ++i) {
        tx.send(i);
    }

    // Individual receives
    start = high_resolution_clock::now();
    for (int i = 0; i < N; ++i) {
        rx.try_recv();
    }
    auto individual_time = duration_cast<microseconds>(high_resolution_clock::now() - start);

    std::cout << " (batch: " << batch_time.count() << " μs, "
              << "individual: " << individual_time.count() << " μs)";

    // Note: Batch receive mainly saves function call overhead
    // Performance benefit depends on message processing cost
    // This test is informational only
}

// Test 11: Concurrent batch sends
TEST(concurrent_batch_sends) {
    auto [tx, rx] = channel<int>();

    const int num_senders = 4;
    const int batch_size = 100;

    std::vector<std::thread> senders;

    for (int s = 0; s < num_senders; ++s) {
        auto tx_clone = tx.clone();
        senders.emplace_back([tx_clone = std::move(tx_clone), s, batch_size]() mutable {
            std::vector<int> batch;
            for (int i = 0; i < batch_size; ++i) {
                batch.push_back(s * 1000 + i);
            }
            tx_clone.send_batch(batch);
        });
    }

    tx = Sender<int>(nullptr);

    for (auto& t : senders) {
        t.join();
    }

    // Receive all
    int count = 0;
    while (auto result = rx.try_recv()) {
        if (result.is_ok()) {
            count++;
        } else {
            break;
        }
    }

    ASSERT(count == num_senders * batch_size, "Should receive all messages");
}

// Test 12: Mixed batch and individual operations
TEST(mixed_operations) {
    auto [tx, rx] = channel<int>();

    // Individual send
    tx.send(1);

    // Batch send
    std::vector<int> batch = {2, 3, 4};
    tx.send_batch(batch);

    // Individual send
    tx.send(5);

    // Receive all
    std::vector<int> received;
    while (auto result = rx.try_recv()) {
        if (result.is_ok()) {
            received.push_back(result.unwrap());
        } else {
            break;
        }
    }

    ASSERT(received.size() == 5, "Should receive all 5 messages");
    for (size_t i = 0; i < received.size(); ++i) {
        ASSERT(received[i] == static_cast<int>(i + 1), "Values should be in order");
    }
}

// Test 13: Batch operations with move-only types
TEST(batch_with_unique_ptr) {
    auto [tx, rx] = channel<std::unique_ptr<int>>();

    std::vector<std::unique_ptr<int>> batch;
    for (int i = 0; i < 5; ++i) {
        batch.push_back(std::make_unique<int>(i));
    }

    size_t sent = tx.send_batch(std::move(batch));
    ASSERT(sent == 5, "Should send all 5 unique_ptrs");

    auto received = rx.recv_batch(5);
    ASSERT(received.size() == 5, "Should receive all 5");

    for (size_t i = 0; i < received.size(); ++i) {
        ASSERT(*received[i] == static_cast<int>(i), "Values should match");
    }
}

// Test 14: Batch send iterator interface
TEST(batch_send_iterators) {
    auto [tx, rx] = channel<int>();

    std::vector<int> data = {10, 20, 30, 40, 50};

    // Use iterator interface
    size_t sent = tx.send_batch(data.begin(), data.end());

    ASSERT(sent == 5, "Should send all 5 messages");

    // Verify received
    for (int expected : data) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok() && result.unwrap() == expected, "Values should match");
    }
}

// Test 15: Batch receive preserves FIFO order
TEST(batch_recv_fifo) {
    auto [tx, rx] = channel<int>();

    // Send in specific order
    for (int i = 0; i < 100; ++i) {
        tx.send(i);
    }

    // Batch receive
    auto batch = rx.recv_batch(100);

    // Verify order
    for (size_t i = 0; i < batch.size(); ++i) {
        ASSERT(batch[i] == static_cast<int>(i), "FIFO order should be preserved");
    }
}

int main() {
    std::cout << "\n=== Lock-Free MPSC Channel Tests (Phase 4: Batch Operations) ===\n\n";

    // Run all tests
    run_test_basic_batch_send();
    run_test_empty_batch_send();
    run_test_large_batch_send();
    run_test_basic_batch_recv();
    run_test_batch_recv_partial();
    run_test_batch_recv_empty();
    run_test_is_empty_helper();
    run_test_is_disconnected_helper();
    run_test_batch_send_performance();
    run_test_batch_recv_performance();
    run_test_concurrent_batch_sends();
    run_test_mixed_operations();
    run_test_batch_with_unique_ptr();
    run_test_batch_send_iterators();
    run_test_batch_recv_fifo();

    // Summary
    std::cout << "\n=== Test Summary ===\n";
    std::cout << "Passed: " << tests_passed << std::endl;
    std::cout << "Failed: " << tests_failed << std::endl;

    if (tests_failed == 0) {
        std::cout << "\n✓ All tests PASSED!\n" << std::endl;
        return 0;
    } else {
        std::cout << "\n✗ Some tests FAILED!\n" << std::endl;
        return 1;
    }
}
