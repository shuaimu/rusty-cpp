// Demonstration of lock-free MPSC channel with blocking recv (Phase 2)

#include <iostream>
#include <thread>
#include <chrono>
#include <vector>
#include <string>
#include <atomic>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

// Mark std::string as Send for this demo
namespace rusty {
    template<>
    struct is_explicitly_send<std::string> : std::true_type {};
}

using namespace rusty::sync::mpsc::lockfree;
using namespace std::chrono;

int main() {
    std::cout << "=== Lock-Free MPSC Channel - Blocking Demo ===\n\n";

    // Example 1: Basic blocking receive
    {
        std::cout << "1. Basic blocking receive:\n";

        auto [tx, rx] = channel<int>();

        // Send from another thread after delay
        std::thread sender([tx = std::move(tx)]() mutable {
            std::cout << "   [Sender] Waiting 100ms before sending...\n";
            std::this_thread::sleep_for(milliseconds(100));
            std::cout << "   [Sender] Sending message\n";
            tx.send(42);
        });

        std::cout << "   [Receiver] Calling recv() - will block...\n";
        auto start = high_resolution_clock::now();
        auto result = rx.recv();
        auto duration = duration_cast<milliseconds>(high_resolution_clock::now() - start);

        std::cout << "   [Receiver] Received: " << result.unwrap() << std::endl;
        std::cout << "   Blocked for: " << duration.count() << " ms\n\n";

        sender.join();
    }

    // Example 2: Immediate return (message already available)
    {
        std::cout << "2. Immediate return (message ready):\n";

        auto [tx, rx] = channel<int>();

        // Send before recv
        tx.send(100);
        std::cout << "   Message already in queue\n";

        auto start = high_resolution_clock::now();
        auto result = rx.recv();
        auto duration = duration_cast<microseconds>(high_resolution_clock::now() - start);

        std::cout << "   Received: " << result.unwrap() << std::endl;
        std::cout << "   Latency: " << duration.count() << " μs (fast path!)\n\n";
    }

    // Example 3: Producer-consumer pattern
    {
        std::cout << "3. Producer-consumer pattern:\n";

        auto [tx, rx] = channel<std::string>();

        // Producer thread
        std::thread producer([tx = std::move(tx)]() mutable {
            std::vector<std::string> tasks = {"Task 1", "Task 2", "Task 3", "Task 4", "Task 5"};

            for (const auto& task : tasks) {
                std::this_thread::sleep_for(milliseconds(50));
                std::cout << "   [Producer] Produced: " << task << std::endl;
                tx.send(task);
            }

            std::cout << "   [Producer] Done\n";
        });

        // Consumer thread
        std::thread consumer([rx = std::move(rx)]() mutable {
            int count = 0;
            while (true) {
                auto result = rx.recv();
                if (result.is_err()) {
                    break;  // Producer disconnected
                }

                std::string task = result.unwrap();
                std::cout << "   [Consumer] Consumed: " << task << std::endl;
                count++;

                // Simulate processing
                std::this_thread::sleep_for(milliseconds(30));
            }

            std::cout << "   [Consumer] Done, processed " << count << " tasks\n";
        });

        producer.join();
        consumer.join();

        std::cout << std::endl;
    }

    // Example 4: Multiple producers, blocking consumer
    {
        std::cout << "4. Multiple producers, blocking consumer:\n";

        auto [tx, rx] = channel<int>();

        // Start 3 producers
        std::vector<std::thread> producers;
        for (int i = 0; i < 3; ++i) {
            auto tx_clone = tx.clone();
            producers.emplace_back([tx_clone = std::move(tx_clone), i]() mutable {
                for (int j = 0; j < 3; ++j) {
                    std::this_thread::sleep_for(milliseconds(30));
                    int value = i * 10 + j;
                    tx_clone.send(value);
                    std::cout << "   [Producer " << i << "] Sent: " << value << std::endl;
                }
            });
        }

        // Drop original sender
        tx = Sender<int>(nullptr);

        // Consumer receives all (blocking)
        std::thread consumer([rx = std::move(rx)]() mutable {
            std::vector<int> received;
            while (true) {
                auto result = rx.recv();
                if (result.is_err()) {
                    break;
                }
                received.push_back(result.unwrap());
            }

            std::cout << "   [Consumer] Received " << received.size() << " messages\n";
        });

        for (auto& t : producers) {
            t.join();
        }
        consumer.join();

        std::cout << std::endl;
    }

    // Example 5: Latency benchmark
    {
        std::cout << "5. Latency benchmark (1000 messages):\n";

        auto [tx, rx] = channel<int>();

        std::atomic<int> ready_count{0};

        // Sender
        std::thread sender([tx = std::move(tx), &ready_count]() mutable {
            ready_count.store(1);
            for (int i = 0; i < 1000; ++i) {
                tx.send(i);
            }
        });

        // Receiver
        std::thread receiver([rx = std::move(rx), &ready_count]() mutable {
            // Wait for sender to be ready
            while (ready_count.load() == 0) {
                std::this_thread::yield();
            }

            auto start = high_resolution_clock::now();

            for (int i = 0; i < 1000; ++i) {
                auto result = rx.recv();
                if (result.is_err()) {
                    break;
                }
            }

            auto duration = duration_cast<microseconds>(high_resolution_clock::now() - start);
            double avg_latency = duration.count() / 1000.0;

            std::cout << "   Total time: " << duration.count() << " μs\n";
            std::cout << "   Average latency: " << avg_latency << " μs per message\n";
            std::cout << "   Throughput: " << (1000.0 / duration.count() * 1000000.0 / 1000000.0)
                      << " M msgs/sec\n";
        });

        sender.join();
        receiver.join();

        std::cout << std::endl;
    }

    // Example 6: Graceful shutdown
    {
        std::cout << "6. Graceful shutdown:\n";

        auto [tx, rx] = channel<int>();

        // Consumer waiting
        std::thread consumer([rx = std::move(rx)]() mutable {
            std::cout << "   [Consumer] Waiting for messages...\n";

            while (true) {
                auto result = rx.recv();
                if (result.is_err()) {
                    std::cout << "   [Consumer] All senders disconnected, shutting down\n";
                    break;
                }
                std::cout << "   [Consumer] Received: " << result.unwrap() << std::endl;
            }
        });

        // Send a few messages
        for (int i = 0; i < 3; ++i) {
            std::this_thread::sleep_for(milliseconds(20));
            tx.send(i);
        }

        // Drop sender to signal shutdown
        std::this_thread::sleep_for(milliseconds(20));
        std::cout << "   [Main] Dropping sender...\n";
        tx = Sender<int>(nullptr);

        consumer.join();
        std::cout << std::endl;
    }

    std::cout << "=== Demo Complete ===\n";

    return 0;
}
