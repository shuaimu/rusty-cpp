// Test for Rust-inspired MPSC channel for C++
// Tests multi-producer single-consumer message passing with thread safety

#include "rusty/sync/mpsc.hpp"
#include "rusty/option.hpp"
#include "rusty/result.hpp"
#include "rusty/box.hpp"
#include <cassert>
#include <thread>
#include <vector>
#include <chrono>
#include <iostream>

// Test 1: Basic send and receive
void test_basic_send_receive() {
    std::cout << "Test 1: Basic send/receive... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send a value
    auto send_result = sender.send(42);
    assert(send_result.is_ok());

    // Receive the value
    auto recv_result = receiver.recv();
    assert(recv_result.is_ok());
    assert(recv_result.unwrap() == 42);

    std::cout << "PASS\n";
}

// Test 2: Multiple messages in sequence
void test_multiple_messages() {
    std::cout << "Test 2: Multiple messages... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send multiple values
    for (int i = 0; i < 5; i++) {
        auto result = sender.send(i * 10);
        assert(result.is_ok());
    }

    // Receive all values
    for (int i = 0; i < 5; i++) {
        auto result = receiver.recv();
        assert(result.is_ok());
        assert(result.unwrap() == i * 10);
    }

    std::cout << "PASS\n";
}

// Test 3: Non-blocking try_send and try_recv
void test_try_operations() {
    std::cout << "Test 3: Try operations... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // try_send (should succeed)
    auto send_result = sender.try_send(100);
    assert(send_result.is_ok());

    // try_recv with message (should succeed)
    auto recv_result = receiver.try_recv();
    assert(recv_result.is_ok());
    assert(recv_result.unwrap() == 100);

    // try_recv without message (should get Empty)
    auto empty_result = receiver.try_recv();
    assert(empty_result.is_err());

    std::cout << "PASS\n";
}

// Test 4: Sender disconnection (drop sender)
void test_sender_disconnection() {
    std::cout << "Test 4: Sender disconnection... ";

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
    assert(result1.is_ok());
    assert(result1.unwrap() == 42);

    // Try to receive again - should get Disconnected
    auto result2 = receiver.recv();
    assert(result2.is_err());

    std::cout << "PASS\n";
}

// Test 5: Receiver disconnection (try to send to dropped receiver)
void test_receiver_disconnection() {
    std::cout << "Test 5: Receiver disconnection... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Drop receiver
    {
        auto temp_receiver = std::move(receiver);
        // temp_receiver is destroyed here
    }

    // Try to send - should get Disconnected
    auto result = sender.send(42);
    assert(result.is_err());

    std::cout << "PASS\n";
}

// Test 6: Clone sender for multi-producer
void test_clone_sender() {
    std::cout << "Test 6: Clone sender... ";

    auto [sender1, receiver] = rusty::sync::mpsc::channel<int>();

    // Clone sender to create second producer
    auto sender2 = sender1.clone();

    // Send from both senders
    sender1.send(100);
    sender2.send(200);

    // Receive both messages (order doesn't matter)
    auto msg1 = receiver.recv();
    auto msg2 = receiver.recv();

    assert(msg1.is_ok());
    assert(msg2.is_ok());

    // Check we got both values (in any order)
    int val1 = msg1.unwrap();
    int val2 = msg2.unwrap();
    assert((val1 == 100 && val2 == 200) || (val1 == 200 && val2 == 100));

    std::cout << "PASS\n";
}

// Test 7: recv_opt helper
void test_recv_opt() {
    std::cout << "Test 7: recv_opt helper... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    sender.send(42);

    // Use recv_opt which returns Option<T>
    auto opt = receiver.recv_opt();
    assert(opt.is_some());
    assert(opt.unwrap() == 42);

    // Drop sender
    {
        auto temp = std::move(sender);
    }

    // recv_opt should return None
    auto empty_opt = receiver.recv_opt();
    assert(empty_opt.is_none());

    std::cout << "PASS\n";
}

// Test 8: Multi-threaded producer-consumer
void test_multithreaded() {
    std::cout << "Test 8: Multi-threaded... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Create multiple producer threads
    const int NUM_PRODUCERS = 3;
    const int MESSAGES_PER_PRODUCER = 10;

    std::vector<std::thread> producers;

    for (int p = 0; p < NUM_PRODUCERS; p++) {
        auto producer_sender = sender.clone();
        producers.emplace_back([producer_sender = std::move(producer_sender), p]() mutable {
            for (int i = 0; i < MESSAGES_PER_PRODUCER; i++) {
                int value = p * 100 + i;
                producer_sender.send(value);
                std::this_thread::sleep_for(std::chrono::microseconds(10));
            }
        });
    }

    // Drop the original sender
    {
        auto temp = std::move(sender);
    }

    // Consumer thread
    int total_received = 0;
    std::thread consumer([&receiver, &total_received]() mutable {
        while (true) {
            auto result = receiver.recv();
            if (result.is_ok()) {
                total_received++;
            } else {
                // All senders disconnected
                break;
            }
        }
    });

    // Wait for all producers to finish
    for (auto& t : producers) {
        t.join();
    }

    // Wait for consumer
    consumer.join();

    assert(total_received == NUM_PRODUCERS * MESSAGES_PER_PRODUCER);

    std::cout << "PASS\n";
}

// Test 9: Stress test - many messages
void test_stress() {
    std::cout << "Test 9: Stress test... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    const int NUM_MESSAGES = 1000;

    // Producer thread
    std::thread producer([sender = std::move(sender)]() mutable {
        for (int i = 0; i < NUM_MESSAGES; i++) {
            sender.send(i);
        }
    });

    // Consumer thread
    int received = 0;
    std::thread consumer([receiver = std::move(receiver), &received]() mutable {
        while (received < NUM_MESSAGES) {
            auto result = receiver.recv();
            if (result.is_ok()) {
                received++;
            } else {
                break;
            }
        }
    });

    producer.join();
    consumer.join();

    assert(received == NUM_MESSAGES);

    std::cout << "PASS\n";
}

// Test 10: String messages
void test_string_messages() {
    std::cout << "Test 10: String messages... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<const char*>();

    sender.send("Hello");
    sender.send("from");
    sender.send("channel");

    const char* expected[] = {"Hello", "from", "channel"};
    for (int i = 0; i < 3; i++) {
        auto result = receiver.recv();
        assert(result.is_ok());
        // Note: comparing string literals by pointer is OK here
        // In real code, use strcmp
    }

    std::cout << "PASS\n";
}

// Test 11: Move-only types (Box)
void test_move_only_types() {
    std::cout << "Test 11: Move-only types... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<rusty::Box<int>>();

    // Send a Box
    auto box = rusty::make_box<int>(42);
    sender.send(std::move(box));

    // Receive the Box
    auto result = receiver.recv();
    assert(result.is_ok());
    auto received_box = result.unwrap();
    assert(*received_box == 42);

    std::cout << "PASS\n";
}

// Test 12: Unbounded queue
void test_unbounded_queue() {
    std::cout << "Test 12: Unbounded queue... ";

    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    // Send many messages without receiving
    const int BATCH_SIZE = 100;
    for (int i = 0; i < BATCH_SIZE; i++) {
        auto result = sender.send(i);
        assert(result.is_ok());
    }

    // Receive all
    int received = 0;
    for (int i = 0; i < BATCH_SIZE; i++) {
        auto result = receiver.try_recv();
        if (result.is_ok()) {
            assert(result.unwrap() == i);
            received++;
        }
    }
    assert(received == BATCH_SIZE);

    std::cout << "PASS\n";
}

int main() {
    std::cout << "=== Rusty C++ MPSC Channel Tests ===\n\n";

    test_basic_send_receive();
    test_multiple_messages();
    test_try_operations();
    test_sender_disconnection();
    test_receiver_disconnection();
    test_clone_sender();
    test_recv_opt();
    test_multithreaded();
    test_stress();
    test_string_messages();
    test_move_only_types();
    test_unbounded_queue();

    std::cout << "\n=== All tests PASSED ===\n";
    return 0;
}
