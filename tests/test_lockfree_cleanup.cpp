// Tests for lock-free MPSC channel (Phase 3: Memory cleanup and reclamation)

#define RUSTY_MPSC_TRACK_ALLOCATIONS  // Enable memory tracking

#include <iostream>
#include <thread>
#include <vector>
#include <atomic>
#include <memory>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

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

// Tracking allocations for a type with destructor
struct TrackedValue {
    int value;
    static std::atomic<int> instances;

    TrackedValue(int v = 0) : value(v) {
        instances.fetch_add(1);
    }

    TrackedValue(const TrackedValue& other) : value(other.value) {
        instances.fetch_add(1);
    }

    TrackedValue(TrackedValue&& other) noexcept : value(other.value) {
        instances.fetch_add(1);
    }

    ~TrackedValue() {
        instances.fetch_sub(1);
    }

    TrackedValue& operator=(const TrackedValue&) = default;
    TrackedValue& operator=(TrackedValue&&) = default;

    // Mark as Send
    static constexpr bool is_send = true;
};

std::atomic<int> TrackedValue::instances{0};

// Test 1: Basic cleanup - no messages
TEST(cleanup_empty_channel) {
    {
        auto [tx, rx] = channel<int>();
        // Channel created and immediately dropped
    }
    // If we get here without crash, cleanup worked
}

// Test 2: Cleanup with pending messages
TEST(cleanup_with_pending_messages) {
    TrackedValue::instances.store(0);

    {
        auto [tx, rx] = channel<TrackedValue>();

        // Send messages
        for (int i = 0; i < 10; ++i) {
            tx.send(TrackedValue(i));
        }

        // Drop without receiving
    }

    // All instances should be destroyed
    ASSERT(TrackedValue::instances.load() == 0,
           "All TrackedValue instances should be destroyed");
}

// Test 3: Receiver drops first
TEST(receiver_drops_first) {
    auto [tx, rx] = channel<int>();

    // Send some messages
    for (int i = 0; i < 5; ++i) {
        tx.send(i);
    }

    // Drop receiver
    {
        auto rx_temp = std::move(rx);
        // rx_temp goes out of scope
    }

    // Try to send - should fail
    auto result = tx.send(100);
    ASSERT(result.is_err(), "Send should fail after receiver dropped");
}

// Test 4: Sender drops first
TEST(sender_drops_first) {
    auto [tx, rx] = channel<int>();

    // Send messages
    tx.send(1);
    tx.send(2);

    // Drop sender
    {
        auto tx_temp = std::move(tx);
        // tx_temp goes out of scope
    }

    // Receive existing messages
    auto r1 = rx.try_recv();
    auto r2 = rx.try_recv();

    ASSERT(r1.is_ok() && r1.unwrap() == 1, "Should receive first message");
    ASSERT(r2.is_ok() && r2.unwrap() == 2, "Should receive second message");

    // Try to receive again - should fail with disconnected
    auto r3 = rx.try_recv();
    ASSERT(r3.is_err() && r3.unwrap_err() == TryRecvError::Disconnected,
           "Should be disconnected");
}

// Test 5: Multiple senders, all drop
TEST(multiple_senders_all_drop) {
    auto [tx, rx] = channel<int>();

    std::vector<Sender<int>> senders;
    for (int i = 0; i < 5; ++i) {
        senders.push_back(tx.clone());
    }

    // Send from each sender
    for (size_t i = 0; i < senders.size(); ++i) {
        senders[i].send(static_cast<int>(i));
    }

    // Drop original sender
    {
        auto tx_temp = std::move(tx);
    }

    // Drop all cloned senders
    senders.clear();

    // Receive all messages
    for (int i = 0; i < 5; ++i) {
        auto result = rx.try_recv();
        ASSERT(result.is_ok(), "Should receive message");
    }

    // Should be disconnected now
    auto result = rx.try_recv();
    ASSERT(result.is_err() && result.unwrap_err() == TryRecvError::Disconnected,
           "Should be disconnected after all senders dropped");
}

// Test 6: Drain functionality
TEST(drain_messages) {
    auto [tx, rx] = channel<int>();

    // Send 100 messages
    for (int i = 0; i < 100; ++i) {
        tx.send(i);
    }

    // Drain all messages
    size_t drained = rx.drain();

    ASSERT(drained == 100, "Should drain exactly 100 messages");

    // Channel should be empty
    auto result = rx.try_recv();
    ASSERT(result.is_err() && result.unwrap_err() == TryRecvError::Empty,
           "Channel should be empty after drain");
}

// Test 7: Approximate length
TEST(approximate_length) {
    auto [tx, rx] = channel<int>();

    // Empty channel
    size_t len1 = rx.approximate_len();
    ASSERT(len1 == 0, "Empty channel should have length 0");

    // Send messages
    for (int i = 0; i < 10; ++i) {
        tx.send(i);
    }

    size_t len2 = rx.approximate_len();
    ASSERT(len2 == 10, "Channel should have 10 messages");

    // Receive some
    rx.try_recv();
    rx.try_recv();

    size_t len3 = rx.approximate_len();
    ASSERT(len3 == 8, "Channel should have 8 messages after receiving 2");

    // Drain rest
    rx.drain();

    size_t len4 = rx.approximate_len();
    ASSERT(len4 == 0, "Channel should be empty after drain");
}

// Test 8: Memory statistics (with tracking enabled)
TEST(memory_statistics) {
    auto [tx, rx] = channel<int>();

    // Check initial stats
    auto stats1 = rx.memory_stats();
    ASSERT(stats1.nodes_allocated == 1, "Should have dummy node");
    ASSERT(stats1.nodes_deallocated == 0, "No nodes deallocated yet");
    ASSERT(stats1.nodes_live == 1, "One node live");

    // Send messages
    for (int i = 0; i < 10; ++i) {
        tx.send(i);
    }

    auto stats2 = rx.memory_stats();
    ASSERT(stats2.nodes_allocated == 11, "Should have 11 nodes (dummy + 10 messages)");
    ASSERT(stats2.nodes_deallocated == 0, "No nodes deallocated yet");
    ASSERT(stats2.nodes_live == 11, "11 nodes live");

    // Receive all
    for (int i = 0; i < 10; ++i) {
        rx.try_recv();
    }

    auto stats3 = rx.memory_stats();
    ASSERT(stats3.nodes_allocated == 11, "Still 11 allocated");
    ASSERT(stats3.nodes_deallocated == 10, "10 nodes deallocated");
    ASSERT(stats3.nodes_live == 1, "Only dummy node live");

    // When channel drops, dummy should be deallocated
}

// Test 9: Large message cleanup
TEST(large_message_cleanup) {
    TrackedValue::instances.store(0);

    {
        auto [tx, rx] = channel<TrackedValue>();

        // Send many messages
        for (int i = 0; i < 1000; ++i) {
            tx.send(TrackedValue(i));
        }

        // Receive some
        for (int i = 0; i < 500; ++i) {
            rx.try_recv();
        }

        // Leave 500 in queue when dropped
    }

    // All should be cleaned up
    ASSERT(TrackedValue::instances.load() == 0,
           "All instances should be destroyed");
}

// Test 10: Concurrent cleanup
TEST(concurrent_cleanup) {
    TrackedValue::instances.store(0);

    {
        auto [tx, rx] = channel<TrackedValue>();

        // Multiple producers sending
        std::vector<std::thread> producers;
        for (int p = 0; p < 3; ++p) {
            auto tx_clone = tx.clone();
            producers.emplace_back([tx_clone = std::move(tx_clone), p]() mutable {
                for (int i = 0; i < 100; ++i) {
                    tx_clone.send(TrackedValue(p * 1000 + i));
                }
            });
        }

        // Consumer receiving some
        std::thread consumer([rx = std::move(rx)]() mutable {
            for (int i = 0; i < 150; ++i) {
                rx.try_recv();
            }
            // Leave rest in queue
        });

        tx = Sender<TrackedValue>(nullptr);  // Drop original

        for (auto& t : producers) {
            t.join();
        }
        consumer.join();

        // Channel state still exists with remaining messages
    }

    // Everything should be cleaned up
    ASSERT(TrackedValue::instances.load() == 0,
           "All instances should be destroyed after concurrent operations");
}

// Test 11: Drain during concurrent sends
TEST(drain_during_sends) {
    auto [tx, rx] = channel<int>();

    std::atomic<bool> keep_sending{true};

    // Producer thread
    std::thread producer([tx = std::move(tx), &keep_sending]() mutable {
        int i = 0;
        while (keep_sending.load()) {
            tx.send(i++);
            std::this_thread::sleep_for(std::chrono::microseconds(10));
        }
    });

    // Let producer send some messages
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Drain while producer is still sending
    size_t drained1 = rx.drain();

    // Wait a bit more
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Drain again
    size_t drained2 = rx.drain();

    // Stop producer
    keep_sending.store(false);
    producer.join();

    // Final drain
    size_t drained3 = rx.drain();

    std::cout << " (drained: " << drained1 << " + " << drained2 << " + " << drained3 << ")";

    ASSERT(drained1 + drained2 + drained3 > 0, "Should have drained some messages");
}

// Test 12: No memory leaks (verified by destructor)
TEST(no_memory_leaks) {
    // Create and destroy many channels
    for (int i = 0; i < 10; ++i) {
        auto [tx, rx] = channel<TrackedValue>();

        for (int j = 0; j < 100; ++j) {
            tx.send(TrackedValue(j));
        }

        // Receive some, leave some
        for (int j = 0; j < 50; ++j) {
            rx.try_recv();
        }

        // Drop channel
    }

    ASSERT(TrackedValue::instances.load() == 0,
           "No instances should remain after all channels dropped");
}

// Test 13: Move semantics don't leak
TEST(move_semantics_no_leak) {
    TrackedValue::instances.store(0);

    {
        auto [tx, rx] = channel<TrackedValue>();

        tx.send(TrackedValue(1));

        // Move sender
        auto tx2 = std::move(tx);

        // Move receiver
        auto rx2 = std::move(rx);

        auto result = rx2.try_recv();
        ASSERT(result.is_ok(), "Should receive from moved receiver");
    }

    ASSERT(TrackedValue::instances.load() == 0,
           "No instances should leak through moves");
}

int main() {
    std::cout << "\n=== Lock-Free MPSC Channel Tests (Phase 3: Cleanup) ===\n";
    std::cout << "Memory tracking: ENABLED\n\n";

    // Run all tests
    run_test_cleanup_empty_channel();
    run_test_cleanup_with_pending_messages();
    run_test_receiver_drops_first();
    run_test_sender_drops_first();
    run_test_multiple_senders_all_drop();
    run_test_drain_messages();
    run_test_approximate_length();
    run_test_memory_statistics();
    run_test_large_message_cleanup();
    run_test_concurrent_cleanup();
    run_test_drain_during_sends();
    run_test_no_memory_leaks();
    run_test_move_semantics_no_leak();

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
