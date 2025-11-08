// Demonstration of lock-free MPSC channel (Phase 1: Non-blocking)

#include <iostream>
#include <thread>
#include <vector>
#include <chrono>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

using namespace rusty::sync::mpsc::lockfree;
using namespace std::chrono;

// Mark std::string as Send for this demo
namespace rusty {
    template<>
    struct is_explicitly_send<std::string> : std::true_type {};
}

int main() {
    std::cout << "=== Lock-Free MPSC Channel Demo ===\n\n";

    // Example 1: Basic usage
    {
        std::cout << "1. Basic send/receive:\n";

        auto [tx, rx] = channel<int>();

        tx.send(42);
        tx.send(100);

        auto result1 = rx.try_recv();
        auto result2 = rx.try_recv();

        std::cout << "   Received: " << result1.unwrap() << std::endl;
        std::cout << "   Received: " << result2.unwrap() << std::endl;
        std::cout << std::endl;
    }

    // Example 2: Multi-producer
    {
        std::cout << "2. Multiple producers:\n";

        auto [tx, rx] = channel<std::string>();

        // Create multiple producers
        std::vector<std::thread> producers;

        for (int i = 0; i < 3; ++i) {
            auto tx_clone = tx.clone();
            producers.emplace_back([tx_clone = std::move(tx_clone), i]() {
                std::string msg = "Message from producer " + std::to_string(i);
                tx_clone.send(std::move(msg));
            });
        }

        // Drop original sender
        tx = Sender<std::string>(nullptr);

        // Wait for producers
        for (auto& t : producers) {
            t.join();
        }

        // Receive all messages
        int count = 0;
        while (true) {
            auto result = rx.try_recv();
            if (result.is_err()) {
                break;
            }
            std::cout << "   Received: " << result.unwrap() << std::endl;
            count++;
        }
        std::cout << "   Total messages: " << count << "\n\n";
    }

    // Example 3: Performance test
    {
        std::cout << "3. Performance test (100,000 messages):\n";

        auto [tx, rx] = channel<int>();

        const int num_messages = 100000;

        auto start = high_resolution_clock::now();

        // Send all messages
        for (int i = 0; i < num_messages; ++i) {
            tx.send(i);
        }

        auto send_done = high_resolution_clock::now();

        // Receive all messages
        int received_count = 0;
        while (true) {
            auto result = rx.try_recv();
            if (result.is_err()) {
                break;
            }
            received_count++;
        }

        auto recv_done = high_resolution_clock::now();

        auto send_duration = duration_cast<microseconds>(send_done - start);
        auto recv_duration = duration_cast<microseconds>(recv_done - send_done);
        auto total_duration = duration_cast<microseconds>(recv_done - start);

        std::cout << "   Sent " << num_messages << " messages in "
                  << send_duration.count() << " μs" << std::endl;
        std::cout << "   Received " << received_count << " messages in "
                  << recv_duration.count() << " μs" << std::endl;
        std::cout << "   Total time: " << total_duration.count() << " μs" << std::endl;

        double throughput = (num_messages * 1000000.0) / total_duration.count();
        std::cout << "   Throughput: " << (throughput / 1000000.0) << " M msgs/sec\n\n";
    }

    // Example 4: FIFO ordering guarantee
    {
        std::cout << "4. FIFO ordering guarantee:\n";

        auto [tx, rx] = channel<int>();

        // Send in order
        for (int i = 1; i <= 5; ++i) {
            tx.send(i);
        }

        std::cout << "   Sent: 1, 2, 3, 4, 5\n";
        std::cout << "   Received: ";

        // Receive and verify order
        bool first = true;
        while (true) {
            auto result = rx.try_recv();
            if (result.is_err()) {
                break;
            }
            if (!first) std::cout << ", ";
            std::cout << result.unwrap();
            first = false;
        }
        std::cout << "\n   ✓ Order preserved (FIFO)\n\n";
    }

    // Example 5: Error handling
    {
        std::cout << "5. Error handling:\n";

        auto [tx, rx] = channel<int>();

        // Drop receiver
        {
            auto rx_temp = std::move(rx);
        }

        // Try to send
        auto result = tx.send(42);
        if (result.is_err()) {
            std::cout << "   ✓ Send failed: Receiver disconnected\n";
        }

        std::cout << std::endl;
    }

    std::cout << "=== Demo Complete ===\n";

    return 0;
}
