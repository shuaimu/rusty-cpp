// Demo showing Rust's Send trait equivalent in C++
// The channel enforces compile-time checks that T is safe to send between threads

#include "rusty/sync/mpsc.hpp"
#include "rusty/box.hpp"
#include <iostream>
#include <thread>

// Example 1: Types that ARE Send (safe to send between threads)
struct SafeToSend {
    int value;
    SafeToSend(int v) : value(v) {}

    // Move-constructible, move-assignable, destructible
    SafeToSend(SafeToSend&&) = default;
    SafeToSend& operator=(SafeToSend&&) = default;
    ~SafeToSend() = default;
};

// Example 2: Type that is NOT Send (not move-constructible)
struct NotSend {
    int value;

    // Deleted move constructor - NOT Send!
    NotSend(NotSend&&) = delete;
    NotSend& operator=(NotSend&&) = delete;
};

// Example 3: Type with raw pointer (technically movable, but conceptually unsafe)
// NOTE: This WILL compile because raw pointers are trivially movable
// This is a known limitation - C++ can't fully replicate Rust's Send
struct HasRawPointer {
    int* ptr;
    HasRawPointer(int* p) : ptr(p) {}
};

void test_send_types() {
    std::cout << "=== Send Trait Demo ===\n\n";

    // ✅ WORKS: SafeToSend is move-constructible
    std::cout << "1. Creating channel<SafeToSend>... ";
    auto [tx1, rx1] = rusty::sync::mpsc::channel<SafeToSend>();
    tx1.send(SafeToSend(42));
    auto result1 = rx1.recv();
    if (result1.is_ok()) {
        std::cout << "✓ Received: " << result1.unwrap().value << "\n";
    }

    // ✅ WORKS: int is trivially movable
    std::cout << "2. Creating channel<int>... ";
    auto [tx2, rx2] = rusty::sync::mpsc::channel<int>();
    tx2.send(100);
    std::cout << "✓ OK\n";

    // ✅ WORKS: Box<T> is move-only (Send)
    std::cout << "3. Creating channel<Box<int>>... ";
    auto [tx3, rx3] = rusty::sync::mpsc::channel<rusty::Box<int>>();
    tx3.send(rusty::make_box<int>(200));
    std::cout << "✓ OK\n";

    // ❌ COMPILE ERROR: NotSend is not move-constructible
    // Uncomment to see the error:
    // auto [tx4, rx4] = rusty::sync::mpsc::channel<NotSend>();
    // Error: "Channel type T must be move-constructible (like Rust's Send trait)"

    std::cout << "\n=== Type Requirements ===\n";
    std::cout << "To use channel<T>, T must satisfy (like Rust's Send):\n";
    std::cout << "  ✓ Move-constructible\n";
    std::cout << "  ✓ Move-assignable\n";
    std::cout << "  ✓ Destructible\n\n";

    std::cout << "Type checks:\n";
    std::cout << "  SafeToSend:   " << (std::is_move_constructible_v<SafeToSend> ? "✓ Send" : "✗ Not Send") << "\n";
    std::cout << "  NotSend:      " << (std::is_move_constructible_v<NotSend> ? "✓ Send" : "✗ Not Send") << "\n";
    std::cout << "  int:          " << (std::is_move_constructible_v<int> ? "✓ Send" : "✗ Not Send") << "\n";
    std::cout << "  Box<int>:     " << (std::is_move_constructible_v<rusty::Box<int>> ? "✓ Send" : "✗ Not Send") << "\n";

    std::cout << "\n=== Limitations ===\n";
    std::cout << "Unlike Rust's Send trait, our C++ check cannot detect:\n";
    std::cout << "  - Raw pointers to thread-local data\n";
    std::cout << "  - Types with unsafe move semantics\n";
    std::cout << "  - Rc<T> (should not be Send, but passes our check)\n";
    std::cout << "\nFor these cases, rely on code review and testing.\n";
}

void test_comparison_with_rust() {
    std::cout << "\n=== Rust vs C++ Comparison ===\n\n";

    std::cout << "Rust:\n";
    std::cout << "  pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>)\n";
    std::cout << "  - T: Send is a marker trait\n";
    std::cout << "  - Compiler auto-implements Send for safe types\n";
    std::cout << "  - Prevents sending Rc<T> between threads\n\n";

    std::cout << "C++ (our implementation):\n";
    std::cout << "  template<Send T> // C++20 concept\n";
    std::cout << "  std::pair<Sender<T>, Receiver<T>> channel()\n";
    std::cout << "  - Checks: move-constructible, move-assignable, destructible\n";
    std::cout << "  - static_assert provides clear error messages\n";
    std::cout << "  - Cannot detect all unsafe patterns (raw pointers, Rc)\n\n";

    std::cout << "Trade-offs:\n";
    std::cout << "  ✓ Catches most common errors at compile-time\n";
    std::cout << "  ✓ Clear error messages when constraint violated\n";
    std::cout << "  ✗ Less powerful than Rust's full Send analysis\n";
    std::cout << "  ✗ Requires programmer discipline for edge cases\n";
}

int main() {
    test_send_types();
    test_comparison_with_rust();

    std::cout << "\n✓ Demo complete!\n";
    return 0;
}
