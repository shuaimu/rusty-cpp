// Tests for lock-free MPSC channel (Phase 1: Non-blocking operations)

#include <iostream>
#include <thread>
#include <vector>
#include <atomic>
#include <chrono>
#include <algorithm>
#include <memory>
#include <string>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

// Mark std::string and std::unique_ptr as Send for testing
namespace rusty {
    template<>
    struct is_explicitly_send<std::string> : std::true_type {};

    template<typename T>
    struct is_explicitly_send<std::unique_ptr<T>> : std::true_type {};
}

using namespace rusty::sync::mpsc::lockfree;

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

// Test 1: Basic send and receive
TEST(basic_send_recv) {
    auto [tx, rx] = channel<int>();

    // Send a value
    auto send_result = tx.send(42);
    ASSERT(send_result.is_ok(), "Send should succeed");

    // Receive the value
    auto recv_result = rx.try_recv();
    ASSERT(recv_result.is_ok(), "Receive should succeed");
    ASSERT(recv_result.unwrap() == 42, "Received value should be 42");
}

// Test 2: Try receive on empty channel
TEST(try_recv_empty) {
    auto [tx, rx] = channel<int>();

    // Try to receive from empty channel
    auto result = rx.try_recv();
    ASSERT(result.is_err(), "Receive should fail on empty channel");
    ASSERT(result.unwrap_err() == TryRecvError::Empty, "Error should be Empty");
}

// Test 3: Multiple messages in sequence
TEST(multiple_messages) {
    auto [tx, rx] = channel<int>();

    // Send multiple values
    for (int i = 0; i < 10; ++i) {
        auto result = tx.send(i);
        ASSERT(result.is_ok(), "Send should succeed");
    }

    // Receive all values
    for (int i = 0; i < 10; ++i) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Values should match");
    }

    // Channel should be empty now
    auto result = rx.try_recv();
    ASSERT(result.is_err(), "Channel should be empty");
}

// Test 4: Send after receiver dropped
TEST(send_after_receiver_dropped) {
    auto [tx, rx] = channel<int>();

    // Drop receiver
    {
        auto rx_temp = std::move(rx);
        // rx_temp goes out of scope
    }

    // Try to send - should fail
    auto result = tx.send(42);
    ASSERT(result.is_err(), "Send should fail after receiver dropped");
    ASSERT(result.unwrap_err() == TrySendError::Disconnected,
           "Error should be Disconnected");
}

// Test 5: Receive after sender dropped
TEST(recv_after_sender_dropped) {
    auto [tx, rx] = channel<int>();

    // Send a message
    tx.send(42);

    // Drop sender
    {
        auto tx_temp = std::move(tx);
        // tx_temp goes out of scope
    }

    // Receive the existing message - should succeed
    auto result1 = rx.try_recv();
    ASSERT(result1.is_ok(), "Should receive existing message");
    ASSERT(result1.unwrap() == 42, "Value should be 42");

    // Try to receive again - should fail with Disconnected
    auto result2 = rx.try_recv();
    ASSERT(result2.is_err(), "Should fail after sender dropped");
    ASSERT(result2.unwrap_err() == TryRecvError::Disconnected,
           "Error should be Disconnected");
}

// Test 6: Clone sender (multi-producer)
TEST(clone_sender) {
    auto [tx, rx] = channel<int>();

    // Clone sender
    auto tx2 = tx.clone();

    // Send from both senders
    tx.send(1);
    tx2.send(2);

    // Receive both messages
    auto result1 = rx.try_recv();
    auto result2 = rx.try_recv();

    ASSERT(result1.is_ok() && result2.is_ok(), "Both receives should succeed");

    // Values should be 1 and 2 (order guaranteed: FIFO)
    int val1 = result1.unwrap();
    int val2 = result2.unwrap();
    ASSERT(val1 == 1, "First value should be 1");
    ASSERT(val2 == 2, "Second value should be 2");
}

// Test 7: Multiple producers, single consumer (concurrent)
TEST(multiple_producers_single_consumer) {
    auto [tx, rx] = channel<int>();

    const int num_producers = 4;
    const int msgs_per_producer = 100;

    std::vector<std::thread> producers;

    // Start multiple producers
    for (int p = 0; p < num_producers; ++p) {
        auto tx_clone = tx.clone();
        producers.emplace_back([tx_clone = std::move(tx_clone), p, msgs_per_producer]() {
            for (int i = 0; i < msgs_per_producer; ++i) {
                int value = p * 1000 + i;
                tx_clone.send(value);
            }
        });
    }

    // Drop original sender
    tx = Sender<int>(nullptr);

    // Wait for all producers to finish
    for (auto& t : producers) {
        t.join();
    }

    // Receive all messages
    std::vector<int> received;
    while (auto result = rx.try_recv()) {
        if (result.is_ok()) {
            received.push_back(result.unwrap());
        } else {
            break;
        }
    }

    // Check we received all messages
    ASSERT(received.size() == num_producers * msgs_per_producer,
           "Should receive all messages");

    // Verify no duplicates
    std::sort(received.begin(), received.end());
    for (size_t i = 1; i < received.size(); ++i) {
        ASSERT(received[i] != received[i-1], "Should have no duplicates");
    }
}

// Test 8: Stress test - many messages
TEST(stress_test) {
    auto [tx, rx] = channel<int>();

    const int num_messages = 10000;

    // Send many messages
    for (int i = 0; i < num_messages; ++i) {
        auto result = tx.send(i);
        ASSERT(result.is_ok(), "Send should succeed");
    }

    // Receive all messages
    for (int i = 0; i < num_messages; ++i) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Value should match");
    }

    // Channel should be empty
    auto result = rx.try_recv();
    ASSERT(result.is_err(), "Channel should be empty");
}

// Test 9: Non-primitive types (std::string)
TEST(string_messages) {
    auto [tx, rx] = channel<std::string>();

    // Send strings
    tx.send(std::string("hello"));
    tx.send(std::string("world"));

    // Receive strings
    auto result1 = rx.try_recv();
    auto result2 = rx.try_recv();

    ASSERT(result1.is_ok() && result2.is_ok(), "Receives should succeed");
    ASSERT(result1.unwrap() == "hello", "First string should match");
    ASSERT(result2.unwrap() == "world", "Second string should match");
}

// Test 10: Move-only types (unique_ptr)
TEST(move_only_types) {
    auto [tx, rx] = channel<std::unique_ptr<int>>();

    // Send unique_ptr
    auto ptr = std::make_unique<int>(42);
    tx.send(std::move(ptr));

    // Receive unique_ptr
    auto result = rx.try_recv();
    ASSERT(result.is_ok(), "Receive should succeed");

    auto received_ptr = result.unwrap();
    ASSERT(received_ptr != nullptr, "Pointer should not be null");
    ASSERT(*received_ptr == 42, "Value should be 42");
}

// Test 11: FIFO ordering
TEST(fifo_ordering) {
    auto [tx, rx] = channel<int>();

    // Send in order
    for (int i = 0; i < 100; ++i) {
        tx.send(i);
    }

    // Receive and verify order
    for (int i = 0; i < 100; ++i) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Order should be preserved (FIFO)");
    }
}

// Test 12: Interleaved send/receive
TEST(interleaved_send_recv) {
    auto [tx, rx] = channel<int>();

    for (int i = 0; i < 100; ++i) {
        // Send
        tx.send(i);

        // Immediately receive
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Value should match");
    }

    // Channel should be empty
    auto result = rx.try_recv();
    ASSERT(result.is_err(), "Channel should be empty");
}

// Test 13: High contention (many producers sending simultaneously)
TEST(high_contention) {
    auto [tx, rx] = channel<int>();

    const int num_producers = 10;
    const int msgs_per_producer = 1000;

    std::atomic<int> ready_count{0};
    std::atomic<bool> start{false};

    std::vector<std::thread> producers;

    // Start all producers
    for (int p = 0; p < num_producers; ++p) {
        auto tx_clone = tx.clone();
        producers.emplace_back([tx_clone = std::move(tx_clone), p,
                                msgs_per_producer, &ready_count, &start]() {
            ready_count.fetch_add(1);

            // Wait for all threads to be ready
            while (!start.load()) {
                std::this_thread::yield();
            }

            // Send messages as fast as possible
            for (int i = 0; i < msgs_per_producer; ++i) {
                tx_clone.send(p * 10000 + i);
            }
        });
    }

    // Wait for all threads to be ready
    while (ready_count.load() < num_producers) {
        std::this_thread::yield();
    }

    // Start all threads simultaneously
    start.store(true);

    // Drop original sender
    tx = Sender<int>(nullptr);

    // Wait for completion
    for (auto& t : producers) {
        t.join();
    }

    // Verify we received all messages
    int count = 0;
    while (true) {
        auto result = rx.try_recv();
        if (result.is_err()) {
            break;
        }
        count++;
    }

    ASSERT(count == num_producers * msgs_per_producer,
           "Should receive all messages under high contention");
}

// Test 14: recv_opt() interface
TEST(recv_opt_interface) {
    auto [tx, rx] = channel<int>();

    tx.send(42);

    // Use recv_opt
    auto opt = rx.recv_opt();
    ASSERT(opt.is_some(), "Should return Some");
    ASSERT(opt.unwrap() == 42, "Value should be 42");

    // Empty channel
    auto opt2 = rx.recv_opt();
    ASSERT(opt2.is_none(), "Should return None for empty channel");
}

int main() {
    std::cout << "\n=== Lock-Free MPSC Channel Tests (Phase 1) ===\n\n";

    // Run all tests
    run_test_basic_send_recv();
    run_test_try_recv_empty();
    run_test_multiple_messages();
    run_test_send_after_receiver_dropped();
    run_test_recv_after_sender_dropped();
    run_test_clone_sender();
    run_test_multiple_producers_single_consumer();
    run_test_stress_test();
    run_test_string_messages();
    run_test_move_only_types();
    run_test_fifo_ordering();
    run_test_interleaved_send_recv();
    run_test_high_contention();
    run_test_recv_opt_interface();

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
