// This file demonstrates compile-time error when violating Send constraint
// Expected to FAIL compilation with clear error message

#include "rusty/sync/mpsc.hpp"

// Type that is NOT Send (not move-constructible)
struct NotSend {
    int value;

    NotSend(int v) : value(v) {}

    // Deleted move operations - NOT Send!
    NotSend(NotSend&&) = delete;
    NotSend& operator=(NotSend&&) = delete;
};

int main() {
    // This should cause a compile error with message:
    // "Channel type T must be move-constructible (like Rust's Send trait)"
    auto [tx, rx] = rusty::sync::mpsc::channel<NotSend>();

    return 0;
}
