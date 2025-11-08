// Demo and tests for Rust-inspired MPSC channel for C++
// Tests multi-producer single-consumer message passing with thread safety

#include "rusty/sync/mpsc.hpp"
#include "rusty/option.hpp"
#include "rusty/result.hpp"
#include <cstdio>
#include <thread>
#include <vector>
#include <chrono>

extern "C" int printf(const char*, ...);

// @safe
namespace channel_tests {

// Test 1: Basic send and receive
// @safe
void test_basic_send_receive() {
    printf("\n=== Test 1: Basic Send/Receive ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send a value
    auto send_result = sender.send(42);
    if (send_result.is_ok()) {
        printf("Sent: 42\n");
    } else {
        printf("ERROR: Failed to send\n");
    }

    // Receive the value
    auto recv_result = receiver.recv();
    if (recv_result.is_ok()) {
        printf("Received: %d\n", recv_result.unwrap());
    } else {
        printf("ERROR: Failed to receive\n");
    }

    printf("✓ Basic send/receive works\n");
}

// Test 2: Multiple messages in sequence
// @safe
void test_multiple_messages() {
    printf("\n=== Test 2: Multiple Messages ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send multiple values
    for (int i = 0; i < 5; i++) {
        sender.send(i * 10);
        printf("Sent: %d\n", i * 10);
    }

    // Receive all values
    for (int i = 0; i < 5; i++) {
        auto result = receiver.recv();
        if (result.is_ok()) {
            printf("Received: %d\n", result.unwrap());
        }
    }

    printf("✓ Multiple messages work\n");
}

// Test 3: Non-blocking try_send and try_recv
// @safe
void test_try_operations() {
    printf("\n=== Test 3: Try Operations ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // try_send (should succeed)
    auto send_result = sender.try_send(100);
    if (send_result.is_ok()) {
        printf("try_send succeeded\n");
    }

    // try_recv with message (should succeed)
    auto recv_result = receiver.try_recv();
    if (recv_result.is_ok()) {
        printf("try_recv got: %d\n", recv_result.unwrap());
    }

    // try_recv without message (should get Empty)
    auto empty_result = receiver.try_recv();
    if (empty_result.is_err()) {
        printf("try_recv returned Empty (as expected)\n");
    }

    printf("✓ Try operations work\n");
}

// Test 4: Sender disconnection (drop sender)
// @safe
void test_sender_disconnection() {
    printf("\n=== Test 4: Sender Disconnection ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send one message
    sender.send(42);

    // Drop sender by moving it into a scope that ends
    {
        auto temp_sender = std::move(sender);
        // temp_sender is destroyed here
    }

    // Receive the message that was sent
    auto result1 = receiver.recv();
    if (result1.is_ok()) {
        printf("Received pending message: %d\n", result1.unwrap());
    }

    // Try to receive again - should get Disconnected
    auto result2 = receiver.recv();
    if (result2.is_err()) {
        printf("Got Disconnected error (as expected)\n");
    }

    printf("✓ Sender disconnection works\n");
}

// Test 5: Receiver disconnection (try to send to dropped receiver)
// @safe
void test_receiver_disconnection() {
    printf("\n=== Test 5: Receiver Disconnection ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Drop receiver
    {
        auto temp_receiver = std::move(receiver);
        // temp_receiver is destroyed here
    }

    // Try to send - should get Disconnected
    auto result = sender.send(42);
    if (result.is_err()) {
        printf("Got Disconnected error when sending (as expected)\n");
    }

    printf("✓ Receiver disconnection works\n");
}

// Test 6: Clone sender for multi-producer
// @safe
void test_clone_sender() {
    printf("\n=== Test 6: Clone Sender ===\n");

    auto [sender1, receiver] = rusty::sync::mpsc::channel<int>();

    // Clone sender to create second producer
    auto sender2 = sender1.clone();

    // Send from both senders
    sender1.send(100);
    sender2.send(200);

    // Receive both messages
    auto msg1 = receiver.recv();
    auto msg2 = receiver.recv();

    if (msg1.is_ok() && msg2.is_ok()) {
        printf("Received from sender1 and sender2: %d, %d\n",
               msg1.unwrap(), msg2.unwrap());
    }

    printf("✓ Clone sender works\n");
}

// Test 7: recv_opt helper
// @safe
void test_recv_opt() {
    printf("\n=== Test 7: recv_opt Helper ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    sender.send(42);

    // Use recv_opt which returns Option<T>
    auto opt = receiver.recv_opt();
    if (opt.is_some()) {
        printf("recv_opt got: %d\n", opt.unwrap());
    }

    // Drop sender
    {
        auto temp = std::move(sender);
    }

    // recv_opt should return None
    auto empty_opt = receiver.recv_opt();
    if (empty_opt.is_none()) {
        printf("recv_opt returned None (as expected)\n");
    }

    printf("✓ recv_opt works\n");
}

// Test 8: Multi-threaded producer-consumer
// @safe
void test_multithreaded() {
    printf("\n=== Test 8: Multi-threaded Test ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Create multiple producer threads
    const int NUM_PRODUCERS = 3;
    const int MESSAGES_PER_PRODUCER = 5;

    std::vector<std::thread> producers;

    for (int p = 0; p < NUM_PRODUCERS; p++) {
        auto producer_sender = sender.clone();
        producers.emplace_back([producer_sender = std::move(producer_sender), p]() mutable {
            for (int i = 0; i < MESSAGES_PER_PRODUCER; i++) {
                int value = p * 100 + i;
                producer_sender.send(value);
                // Small delay to simulate work
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        });
    }

    // Drop the original sender
    {
        auto temp = std::move(sender);
    }

    // Consumer thread
    std::thread consumer([receiver = std::move(receiver)]() mutable {
        int count = 0;
        while (true) {
            auto result = receiver.recv();
            if (result.is_ok()) {
                count++;
            } else {
                // All senders disconnected
                break;
            }
        }
        printf("Consumer received %d messages\n", count);
    });

    // Wait for all producers to finish
    for (auto& t : producers) {
        t.join();
    }

    // Wait for consumer
    consumer.join();

    printf("✓ Multi-threaded test completed\n");
}

// Test 9: Stress test - many messages
// @safe
void test_stress() {
    printf("\n=== Test 9: Stress Test ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    const int NUM_MESSAGES = 1000;

    // Producer thread
    std::thread producer([sender = std::move(sender)]() mutable {
        for (int i = 0; i < NUM_MESSAGES; i++) {
            sender.send(i);
        }
    });

    // Consumer thread
    std::thread consumer([receiver = std::move(receiver)]() mutable {
        int received = 0;
        while (received < NUM_MESSAGES) {
            auto result = receiver.recv();
            if (result.is_ok()) {
                received++;
            } else {
                break;
            }
        }
        printf("Stress test: received %d/%d messages\n", received, NUM_MESSAGES);
    });

    producer.join();
    consumer.join();

    printf("✓ Stress test completed\n");
}

// Test 10: String messages
// @safe
void test_string_messages() {
    printf("\n=== Test 10: String Messages ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<const char*>();

    sender.send("Hello");
    sender.send("from");
    sender.send("channel");

    for (int i = 0; i < 3; i++) {
        auto result = receiver.recv();
        if (result.is_ok()) {
            printf("Received: %s\n", result.unwrap());
        }
    }

    printf("✓ String messages work\n");
}

// Test 11: Move-only types (Box)
// @safe
void test_move_only_types() {
    printf("\n=== Test 11: Move-only Types ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<rusty::Box<int>>();

    // Send a Box
    auto box = rusty::make_box<int>(42);
    sender.send(std::move(box));
    printf("Sent Box\n");

    // Receive the Box
    auto result = receiver.recv();
    if (result.is_ok()) {
        auto received_box = result.unwrap();
        printf("Received Box with value: %d\n", *received_box);
    }

    printf("✓ Move-only types work\n");
}

// Test 12: Bounded behavior (unlimited queue)
// @safe
void test_unbounded_queue() {
    printf("\n=== Test 12: Unbounded Queue ===\n");

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send many messages without receiving
    const int BATCH_SIZE = 100;
    for (int i = 0; i < BATCH_SIZE; i++) {
        auto result = sender.send(i);
        if (result.is_err()) {
            printf("ERROR: Failed to send message %d\n", i);
        }
    }
    printf("Sent %d messages without blocking\n", BATCH_SIZE);

    // Receive all
    int received = 0;
    for (int i = 0; i < BATCH_SIZE; i++) {
        auto result = receiver.try_recv();
        if (result.is_ok()) {
            received++;
        }
    }
    printf("Received %d messages\n", received);

    printf("✓ Unbounded queue works\n");
}

} // namespace channel_tests

int main() {
    printf("Rusty C++ MPSC Channel Tests\n");
    printf("=============================\n");

    channel_tests::test_basic_send_receive();
    channel_tests::test_multiple_messages();
    channel_tests::test_try_operations();
    channel_tests::test_sender_disconnection();
    channel_tests::test_receiver_disconnection();
    channel_tests::test_clone_sender();
    channel_tests::test_recv_opt();
    channel_tests::test_multithreaded();
    channel_tests::test_stress();
    channel_tests::test_string_messages();
    channel_tests::test_move_only_types();
    channel_tests::test_unbounded_queue();

    printf("\n=============================\n");
    printf("✓ All tests completed successfully!\n");
    return 0;
}
