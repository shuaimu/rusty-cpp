#pragma once

#include <memory>
#include <mutex>
#include <condition_variable>
#include <queue>
#include <atomic>
#include <type_traits>
#include "../option.hpp"
#include "../result.hpp"
#include "../send_trait.hpp"
#include "../send_impls.hpp"

// Multi-Producer Single-Consumer (MPSC) Channel
// Equivalent to Rust's std::sync::mpsc
//
// Guarantees:
// - Thread-safe message passing
// - Multiple senders, single receiver
// - Blocking and non-blocking operations
// - Proper cleanup when channel is closed
//
// Type Requirements (like Rust's Send trait):
// - T must be explicitly marked as Send
// - Default is NOT Send (conservative, safe)
// - Prevents accidental sending of non-thread-safe types

// @safe
namespace rusty {
namespace sync {
// @unsafe - mpsc channel uses mutable mutex for thread-safe interior mutability
namespace mpsc {

// Forward declarations
template<typename T> class Receiver;
template<typename T> class Sender;

// Unit type for Result<Unit, E> (like Rust's ())
struct Unit {};

// Send concept using our explicit opt-in system
#if __cplusplus >= 202002L  // C++20 concepts
template<typename T>
concept Send = rusty::is_send<T>::value;
#endif

// Channel errors
enum class TrySendError {
    Disconnected,  // Receiver has been dropped
};

enum class RecvError {
    Disconnected,  // All senders have been dropped
};

enum class TryRecvError {
    Empty,         // No messages available
    Disconnected,  // All senders have been dropped
};

// Shared state between Sender and Receiver
// @unsafe - Uses mutable mutex for thread-safe interior mutability
template<typename T>
class ChannelState {
private:
    friend class Sender<T>;
    friend class Receiver<T>;

    mutable std::mutex mutex_;
    std::condition_variable cv_;
    std::queue<T> queue_;
    std::atomic<size_t> sender_count_;
    bool receiver_alive_;

    // Compile-time check: T must be explicitly marked as Send
    static_assert(rusty::is_send<T>::value,
                  "Channel type T must be Send (marked explicitly). "
                  "Use 'static constexpr bool is_send = true;' in your type, "
                  "or specialize rusty::is_explicitly_send<YourType>. "
                  "Note: Rc<T> is NOT Send, use Arc<T> instead.");

public:
    ChannelState()
        : sender_count_(1), receiver_alive_(true) {}

    // Send a value (blocking)
    Result<Unit, TrySendError> send(T value) {
        std::unique_lock<std::mutex> lock(mutex_);

        if (!receiver_alive_) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }

        queue_.push(std::move(value));
        cv_.notify_one();
        return Result<Unit, TrySendError>::Ok(Unit{});
    }

    // Try to send a value (non-blocking)
    Result<Unit, TrySendError> try_send(T value) {
        std::unique_lock<std::mutex> lock(mutex_);

        if (!receiver_alive_) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }

        queue_.push(std::move(value));
        cv_.notify_one();
        return Result<Unit, TrySendError>::Ok(Unit{});
    }

    // Receive a value (blocking)
    Result<T, RecvError> recv() {
        std::unique_lock<std::mutex> lock(mutex_);

        // Wait until queue is not empty or all senders are gone
        cv_.wait(lock, [this]() {
            return !queue_.empty() || sender_count_.load() == 0;
        });

        if (!queue_.empty()) {
            T value = std::move(queue_.front());
            queue_.pop();
            return Result<T, RecvError>::Ok(std::move(value));
        }

        // Queue is empty and all senders are gone
        return Result<T, RecvError>::Err(RecvError::Disconnected);
    }

    // Try to receive a value (non-blocking)
    Result<T, TryRecvError> try_recv() {
        std::unique_lock<std::mutex> lock(mutex_);

        if (!queue_.empty()) {
            T value = std::move(queue_.front());
            queue_.pop();
            return Result<T, TryRecvError>::Ok(std::move(value));
        }

        if (sender_count_.load() == 0) {
            return Result<T, TryRecvError>::Err(TryRecvError::Disconnected);
        }

        return Result<T, TryRecvError>::Err(TryRecvError::Empty);
    }

    void increment_sender() {
        sender_count_.fetch_add(1, std::memory_order_relaxed);
    }

    void decrement_sender() {
        if (sender_count_.fetch_sub(1, std::memory_order_acq_rel) == 1) {
            // Last sender dropped, wake up receiver
            std::lock_guard<std::mutex> lock(mutex_);
            cv_.notify_one();
        }
    }

    void mark_receiver_dropped() {
        std::lock_guard<std::mutex> lock(mutex_);
        receiver_alive_ = false;
    }

    bool is_receiver_alive() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return receiver_alive_;
    }

    size_t sender_count() const {
        return sender_count_.load(std::memory_order_relaxed);
    }
};

// Sender - can be cloned for multi-producer
template<typename T>
class Sender {
private:
    std::shared_ptr<ChannelState<T>> state_;

public:
    // Constructor from shared state
    explicit Sender(std::shared_ptr<ChannelState<T>> state)
        : state_(std::move(state)) {}

    // Copy constructor - clone sender
    Sender(const Sender& other) : state_(other.state_) {
        if (state_) {
            state_->increment_sender();
        }
    }

    // Move constructor
    Sender(Sender&& other) noexcept
        : state_(std::move(other.state_)) {}

    // Copy assignment
    Sender& operator=(const Sender& other) {
        if (this != &other) {
            if (state_) {
                state_->decrement_sender();
            }
            state_ = other.state_;
            if (state_) {
                state_->increment_sender();
            }
        }
        return *this;
    }

    // Move assignment
    Sender& operator=(Sender&& other) noexcept {
        if (this != &other) {
            if (state_) {
                state_->decrement_sender();
            }
            state_ = std::move(other.state_);
        }
        return *this;
    }

    // Destructor
    ~Sender() {
        if (state_) {
            state_->decrement_sender();
        }
    }

    // Send a value (blocking)
    Result<Unit, TrySendError> send(T value) {
        if (!state_) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }
        return state_->send(std::move(value));
    }

    // Try to send a value (non-blocking)
    Result<Unit, TrySendError> try_send(T value) {
        if (!state_) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }
        return state_->try_send(std::move(value));
    }

    // Clone this sender (for multi-producer)
    Sender clone() const {
        return Sender(*this);
    }
};

// Receiver - cannot be cloned (single-consumer)
template<typename T>
class Receiver {
private:
    std::shared_ptr<ChannelState<T>> state_;

public:
    // Constructor from shared state
    explicit Receiver(std::shared_ptr<ChannelState<T>> state)
        : state_(std::move(state)) {}

    // Not copyable (single consumer)
    Receiver(const Receiver&) = delete;
    Receiver& operator=(const Receiver&) = delete;

    // Movable
    Receiver(Receiver&& other) noexcept
        : state_(std::move(other.state_)) {}

    Receiver& operator=(Receiver&& other) noexcept {
        if (this != &other) {
            if (state_) {
                state_->mark_receiver_dropped();
            }
            state_ = std::move(other.state_);
        }
        return *this;
    }

    // Destructor
    ~Receiver() {
        if (state_) {
            state_->mark_receiver_dropped();
        }
    }

    // Receive a value (blocking)
    Result<T, RecvError> recv() {
        if (!state_) {
            return Result<T, RecvError>::Err(RecvError::Disconnected);
        }
        return state_->recv();
    }

    // Try to receive a value (non-blocking)
    Result<T, TryRecvError> try_recv() {
        if (!state_) {
            return Result<T, TryRecvError>::Err(TryRecvError::Disconnected);
        }
        return state_->try_recv();
    }

    // Iterator-like interface - receive until channel is closed
    // Returns None when all senders are dropped and queue is empty
    Option<T> recv_opt() {
        auto result = recv();
        if (result.is_ok()) {
            return Some(result.unwrap());
        }
        return None;
    }
};

// Factory function to create a channel
// Requires T to be explicitly marked as Send
// This mirrors Rust's requirement: pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>)
//
// IMPORTANT: T must be Send. To mark a type as Send:
// 1. Add: static constexpr bool is_send = true; to your type
// 2. Or specialize: template<> struct rusty::is_explicitly_send<YourType> : std::true_type {};
//
// Note: Primitives (int, double, etc.) and rusty types (Box, Arc, Vec, Option) are already marked.
//       Rc<T> is NOT Send - use Arc<T> instead for thread-safe sharing.
#if __cplusplus >= 202002L  // C++20 concepts
template<Send T>
std::pair<Sender<T>, Receiver<T>> channel() {
    auto state = std::make_shared<ChannelState<T>>();
    return std::make_pair(
        Sender<T>(state),
        Receiver<T>(state)
    );
}
#else  // Pre-C++20: static_assert in ChannelState will catch violations
template<typename T>
std::pair<Sender<T>, Receiver<T>> channel() {
    // Static assert will trigger if T is not Send
    auto state = std::make_shared<ChannelState<T>>();
    return std::make_pair(
        Sender<T>(state),
        Receiver<T>(state)
    );
}
#endif

} // namespace mpsc
} // namespace sync

// Mark Sender<T> and Receiver<T> as Send if T is Send
// This mirrors Rust's behavior where channel endpoints are thread-safe
template<typename T>
struct is_send<sync::mpsc::Sender<T>> : is_send<T> {};

template<typename T>
struct is_send<sync::mpsc::Receiver<T>> : is_send<T> {};

} // namespace rusty
