// Tests for lock-free MPSC channel (Phase 2: Blocking operations)

#include <iostream>
#include <thread>
#include <vector>
#include <atomic>
#include <chrono>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

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

// Test 1: Basic blocking receive
TEST(basic_blocking_recv) {
    auto [tx, rx] = channel<int>();

    // Send from another thread after delay
    std::thread sender([tx = std::move(tx)]() mutable {
        std::this_thread::sleep_for(milliseconds(10));
        tx.send(42);
    });

    // Blocking receive should wait and succeed
    auto start = high_resolution_clock::now();
    auto result = rx.recv();
    auto duration = duration_cast<milliseconds>(high_resolution_clock::now() - start);

    ASSERT(result.is_ok(), "Receive should succeed");
    ASSERT(result.unwrap() == 42, "Value should be 42");
    ASSERT(duration.count() >= 10, "Should have blocked for at least 10ms");

    sender.join();
}

// Test 2: Blocking receive with immediate message
TEST(blocking_recv_immediate) {
    auto [tx, rx] = channel<int>();

    // Send before receive
    tx.send(100);

    // Blocking receive should return immediately
    auto start = high_resolution_clock::now();
    auto result = rx.recv();
    auto duration = duration_cast<microseconds>(high_resolution_clock::now() - start);

    ASSERT(result.is_ok(), "Receive should succeed");
    ASSERT(result.unwrap() == 100, "Value should be 100");
    ASSERT(duration.count() < 1000, "Should not block significantly (< 1ms)");
}

// Test 3: Blocking receive detects disconnection
TEST(blocking_recv_disconnected) {
    auto [tx, rx] = channel<int>();

    // Drop sender from another thread
    std::thread dropper([tx = std::move(tx)]() mutable {
        std::this_thread::sleep_for(milliseconds(10));
        // tx goes out of scope, dropping sender
    });

    // Blocking receive should detect disconnection
    auto result = rx.recv();

    ASSERT(result.is_err(), "Should detect disconnection");
    ASSERT(result.unwrap_err() == RecvError::Disconnected, "Error should be Disconnected");

    dropper.join();
}

// Test 4: Multiple blocking receives
TEST(multiple_blocking_recv) {
    auto [tx, rx] = channel<int>();

    // Send messages with delays
    std::thread sender([tx = std::move(tx)]() mutable {
        for (int i = 0; i < 5; ++i) {
            std::this_thread::sleep_for(milliseconds(5));
            tx.send(i);
        }
    });

    // Receive all messages (blocking)
    for (int i = 0; i < 5; ++i) {
        auto result = rx.recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Value should match");
    }

    sender.join();
}

// Test 5: Blocking with multiple producers
TEST(blocking_recv_multiple_producers) {
    auto [tx, rx] = channel<int>();

    const int num_producers = 3;
    const int msgs_per_producer = 10;

    std::vector<std::thread> producers;

    for (int p = 0; p < num_producers; ++p) {
        auto tx_clone = tx.clone();
        producers.emplace_back([tx_clone = std::move(tx_clone), p, msgs_per_producer]() mutable {
            for (int i = 0; i < msgs_per_producer; ++i) {
                std::this_thread::sleep_for(microseconds(100));
                tx_clone.send(p * 1000 + i);
            }
        });
    }

    // Drop original sender
    tx = Sender<int>(nullptr);

    // Receive all messages (blocking)
    int count = 0;
    while (true) {
        auto result = rx.recv();
        if (result.is_err()) {
            break;
        }
        count++;
    }

    ASSERT(count == num_producers * msgs_per_producer, "Should receive all messages");

    for (auto& t : producers) {
        t.join();
    }
}

// Test 6: Stress test - rapid blocking receives
TEST(stress_blocking_recv) {
    auto [tx, rx] = channel<int>();

    const int num_messages = 1000;

    std::thread sender([tx = std::move(tx), num_messages]() mutable {
        for (int i = 0; i < num_messages; ++i) {
            tx.send(i);
            // No delay - send as fast as possible
        }
    });

    // Receive all messages (mix of immediate and blocking)
    for (int i = 0; i < num_messages; ++i) {
        auto result = rx.recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Value should match");
    }

    sender.join();
}

// Test 7: Blocking receive with timeout simulation
TEST(blocking_recv_timeout_simulation) {
    auto [tx, rx] = channel<int>();

    // Sender waits longer than we want to wait
    std::thread sender([tx = std::move(tx)]() mutable {
        std::this_thread::sleep_for(milliseconds(100));
        tx.send(42);
    });

    // Try to receive with manual timeout check
    std::atomic<bool> received{false};
    std::thread receiver([&rx, &received]() mutable {
        auto result = rx.recv();
        if (result.is_ok()) {
            received.store(true);
        }
    });

    // Check if received within 50ms
    std::this_thread::sleep_for(milliseconds(50));
    bool got_it = received.load();

    // Should not have received yet
    ASSERT(!got_it, "Should not have received within 50ms");

    // Wait for everything to complete
    sender.join();
    receiver.join();

    // Should have received eventually
    ASSERT(received.load(), "Should eventually receive");
}

// Test 8: Wake on send (verify notification works)
TEST(wake_on_send) {
    auto [tx, rx] = channel<int>();

    std::atomic<bool> waiting{false};
    std::atomic<bool> received{false};

    // Receiver thread
    std::thread receiver([&rx, &waiting, &received]() mutable {
        waiting.store(true);
        auto result = rx.recv();  // Will block
        if (result.is_ok()) {
            received.store(true);
        }
    });

    // Wait for receiver to start waiting
    while (!waiting.load()) {
        std::this_thread::yield();
    }

    // Give receiver time to actually block
    std::this_thread::sleep_for(milliseconds(10));

    // Now send - should wake receiver
    auto start = high_resolution_clock::now();
    tx.send(42);

    // Wait for receive to complete
    receiver.join();

    auto duration = duration_cast<milliseconds>(high_resolution_clock::now() - start);

    ASSERT(received.load(), "Receiver should have received");
    ASSERT(duration.count() < 100, "Wake should be fast (< 100ms)");
}

// Test 9: Multiple messages queued before blocking recv
TEST(queued_messages_no_block) {
    auto [tx, rx] = channel<int>();

    // Queue up messages
    for (int i = 0; i < 10; ++i) {
        tx.send(i);
    }

    // Receive all - should not block since messages are ready
    auto start = high_resolution_clock::now();
    for (int i = 0; i < 10; ++i) {
        auto result = rx.recv();
        ASSERT(result.is_ok(), "Receive should succeed");
        ASSERT(result.unwrap() == i, "Value should match");
    }
    auto duration = duration_cast<milliseconds>(high_resolution_clock::now() - start);

    ASSERT(duration.count() < 10, "Should not block significantly");
}

// Test 10: Interleaved blocking and non-blocking receives
TEST(mixed_recv_types) {
    auto [tx, rx] = channel<int>();

    std::thread sender([tx = std::move(tx)]() mutable {
        for (int i = 0; i < 5; ++i) {
            std::this_thread::sleep_for(milliseconds(10));
            tx.send(i);
        }
    });

    // Mix of try_recv and recv
    for (int i = 0; i < 5; ++i) {
        if (i % 2 == 0) {
            // Blocking receive
            auto result = rx.recv();
            ASSERT(result.is_ok(), "Blocking receive should succeed");
        } else {
            // Non-blocking receive (with retries)
            while (true) {
                auto result = rx.try_recv();
                if (result.is_ok()) {
                    break;
                }
                std::this_thread::sleep_for(milliseconds(1));
            }
        }
    }

    sender.join();
}

// Test 11: Latency test - measure wake time
TEST(latency_measurement) {
    auto [tx, rx] = channel<int>();

    std::atomic<high_resolution_clock::time_point> send_time;

    std::thread receiver([&rx, &send_time]() mutable {
        auto result = rx.recv();  // Will block
        if (result.is_ok()) {
            // Measure time from send to receive
            auto recv_time = high_resolution_clock::now();
            auto latency = duration_cast<microseconds>(recv_time - send_time.load());
            std::cout << " (latency: " << latency.count() << " μs)";
        }
    });

    // Give receiver time to block
    std::this_thread::sleep_for(milliseconds(10));

    // Send and record time
    send_time.store(high_resolution_clock::now());
    tx.send(42);

    receiver.join();
}

// Test 12: Producer-consumer pattern
TEST(producer_consumer_pattern) {
    auto [tx, rx] = channel<int>();

    const int num_items = 100;
    std::atomic<int> produced{0};
    std::atomic<int> consumed{0};

    // Producer
    std::thread producer([tx = std::move(tx), &produced, num_items]() mutable {
        for (int i = 0; i < num_items; ++i) {
            tx.send(i);
            produced.fetch_add(1);
            std::this_thread::sleep_for(microseconds(10));
        }
    });

    // Consumer
    std::thread consumer([rx = std::move(rx), &consumed, num_items]() mutable {
        for (int i = 0; i < num_items; ++i) {
            auto result = rx.recv();
            if (result.is_ok()) {
                consumed.fetch_add(1);
            }
        }
    });

    producer.join();
    consumer.join();

    ASSERT(produced.load() == num_items, "Should produce all items");
    ASSERT(consumed.load() == num_items, "Should consume all items");
}

int main() {
    std::cout << "\n=== Lock-Free MPSC Channel Tests (Phase 2: Blocking) ===\n\n";

    // Run all tests
    run_test_basic_blocking_recv();
    run_test_blocking_recv_immediate();
    run_test_blocking_recv_disconnected();
    run_test_multiple_blocking_recv();
    run_test_blocking_recv_multiple_producers();
    run_test_stress_blocking_recv();
    run_test_blocking_recv_timeout_simulation();
    run_test_wake_on_send();
    run_test_queued_messages_no_block();
    run_test_mixed_recv_types();
    run_test_latency_measurement();
    run_test_producer_consumer_pattern();

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
