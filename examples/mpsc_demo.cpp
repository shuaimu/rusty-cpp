// Demo showing Rust-like API path: rusty::sync::mpsc::channel
// Exactly mirrors Rust's std::sync::mpsc::channel

#include "rusty/sync/mpsc.hpp"
#include <iostream>
#include <thread>

int main() {
    std::cout << "Rust-style MPSC Channel Demo\n";
    std::cout << "=============================\n\n";

    // Create a channel - exactly like Rust's std::sync::mpsc::channel()
    auto [sender, receiver] = rusty::sync::mpsc::channel<int>();

    std::cout << "Created channel with rusty::sync::mpsc::channel<int>()\n";
    std::cout << "This mirrors Rust's std::sync::mpsc::channel::<i32>()\n\n";

    // Clone sender for multi-producer - like Rust's sender.clone()
    auto sender2 = sender.clone();
    std::cout << "Cloned sender for multi-producer setup\n\n";

    // Producer thread 1
    std::thread producer1([sender = std::move(sender)]() mutable {
        for (int i = 0; i < 5; i++) {
            sender.send(i);
            std::cout << "Producer 1 sent: " << i << "\n";
        }
    });

    // Producer thread 2
    std::thread producer2([sender2 = std::move(sender2)]() mutable {
        for (int i = 100; i < 105; i++) {
            sender2.send(i);
            std::cout << "Producer 2 sent: " << i << "\n";
        }
    });

    // Consumer receives from both producers
    std::thread consumer([receiver = std::move(receiver)]() mutable {
        std::cout << "\nConsumer receiving messages:\n";
        for (int i = 0; i < 10; i++) {
            auto result = receiver.recv();
            if (result.is_ok()) {
                std::cout << "  Received: " << result.unwrap() << "\n";
            }
        }
    });

    producer1.join();
    producer2.join();
    consumer.join();

    std::cout << "\n✓ Demo complete!\n";
    std::cout << "\nAPI Comparison:\n";
    std::cout << "  Rust:  use std::sync::mpsc;\n";
    std::cout << "         let (tx, rx) = mpsc::channel();\n";
    std::cout << "  C++:   #include <rusty/sync/mpsc.hpp>\n";
    std::cout << "         auto [tx, rx] = rusty::sync::mpsc::channel<T>();\n";

    return 0;
}
