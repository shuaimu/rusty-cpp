#pragma once

#include <atomic>
#include <memory>
#include <type_traits>
#include <thread>
#include <chrono>
#include <iostream>
#include <vector>
#include "../option.hpp"
#include "../result.hpp"
#include "../send_trait.hpp"
#include "../send_impls.hpp"

// Platform-specific CPU pause instruction
#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || defined(_M_IX86)
    #include <immintrin.h>
    #define CPU_RELAX() _mm_pause()
#elif defined(__aarch64__) || defined(__arm__)
    #define CPU_RELAX() __asm__ __volatile__("yield" ::: "memory")
#else
    #define CPU_RELAX() std::this_thread::yield()
#endif

// Lock-Free Multi-Producer Single-Consumer (MPSC) Channel
//
// Implementation based on lock-free linked list queue optimized for MPSC:
// - Multiple producers atomically append to tail
// - Single consumer reads from head (no contention!)
// - No ABA problem (single consumer owns popped nodes)
// - Cache-line aligned to prevent false sharing
//
// Memory Ordering:
// - Producers use release/acquire on tail operations
// - Consumer uses acquire on next pointer loads
// - Forms happens-before relationship for memory safety
//
// Type Requirements:
// - T must be explicitly marked as Send (same as mutex version)

// @safe
namespace rusty {
namespace sync {
namespace mpsc {
namespace lockfree {

// Forward declarations
template<typename T> class Receiver;
template<typename T> class Sender;

// Unit type for Result<Unit, E>
struct Unit {};

// Channel errors (same as mutex version)
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

// Exponential backoff helper for spinning
class Backoff {
private:
    int step_;
    static constexpr int MAX_SPIN_STEP = 6;  // 2^6 = 64 iterations max before yield

public:
    Backoff() : step_(0) {}

    // Spin with exponential backoff
    void spin() {
        if (step_ <= MAX_SPIN_STEP) {
            // Exponential backoff: 1, 2, 4, 8, 16, 32, 64 iterations
            for (int i = 0; i < (1 << step_); ++i) {
                CPU_RELAX();
            }
            step_++;
        } else {
            // After max spins, yield to scheduler
            std::this_thread::yield();
        }
    }

    // Check if we've exceeded spin limit
    bool is_completed() const {
        return step_ > MAX_SPIN_STEP + 10;  // Give up after some yields
    }

    void reset() {
        step_ = 0;
    }
};

// Node in the lock-free linked list
// Each node contains one message (T) and an atomic pointer to next node
template<typename T>
struct Node {
    T data;
    std::atomic<Node<T>*> next;

    // Constructor for node with data
    explicit Node(T&& value)
        : data(std::move(value))
        , next(nullptr) {}

    // Constructor for dummy node (no data)
    Node()
        : data()  // Default construct T
        , next(nullptr) {}
};

// Lock-Free Channel State
//
// Structure:
//   dummy -> node1 -> node2 -> node3 -> nullptr
//   ^                           ^
//   head_                       tail_ (atomic)
//
// Invariants:
// - tail_ always points to the last node
// - head_ points to a dummy node (or the last consumed node)
// - head_->next is the first unconsumed message (or nullptr if empty)
// - Only consumer touches head_ (no atomics needed!)
// - All producers append to tail_ (atomic exchange)
//
template<typename T>
class LockFreeChannelState {
private:
    friend class Sender<T>;
    friend class Receiver<T>;

    // Producer side - cache line aligned to prevent false sharing
    alignas(64) std::atomic<Node<T>*> tail_;

    // Consumer side - cache line aligned
    alignas(64) Node<T>* head_;

    // Synchronization state
    alignas(64) std::atomic<size_t> sender_count_;
    std::atomic<bool> receiver_alive_;

    // Blocking/waking mechanism
    // Incremented by producers when sending, waited on by consumer
    alignas(64) std::atomic<uint32_t> signal_;

    // Memory statistics (optional, for debugging)
    #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
    alignas(64) std::atomic<size_t> nodes_allocated_;
    alignas(64) std::atomic<size_t> nodes_deallocated_;
    #endif

    // Compile-time check: T must be Send
    static_assert(rusty::is_send<T>::value,
                  "Channel type T must be Send (marked explicitly). "
                  "Use 'static constexpr bool is_send = true;' in your type, "
                  "or specialize rusty::is_explicitly_send<YourType>. "
                  "Note: Rc<T> is NOT Send, use Arc<T> instead.");

public:
    LockFreeChannelState()
        : sender_count_(1)  // Initial sender
        , receiver_alive_(true)
        , signal_(0)
    {
        #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
        nodes_allocated_.store(0, std::memory_order_relaxed);
        nodes_deallocated_.store(0, std::memory_order_relaxed);
        #endif

        // Create dummy node to simplify queue logic
        // This allows us to distinguish empty queue from uninitialized queue
        Node<T>* dummy = new Node<T>();
        head_ = dummy;
        tail_.store(dummy, std::memory_order_relaxed);

        #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
        nodes_allocated_.fetch_add(1, std::memory_order_relaxed);
        #endif
    }

    ~LockFreeChannelState() {
        // Drain remaining messages (calling their destructors)
        while (try_pop().is_some()) {
            // Just drop the values
        }

        // Delete remaining nodes (including dummy)
        // At this point, all senders and receiver have been dropped,
        // so no concurrent access is possible
        Node<T>* curr = head_;
        while (curr != nullptr) {
            Node<T>* next = curr->next.load(std::memory_order_relaxed);
            delete curr;
            #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
            nodes_deallocated_.fetch_add(1, std::memory_order_relaxed);
            #endif
            curr = next;
        }

        #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
        // Verify no leaks (all allocated nodes should be deallocated)
        size_t allocated = nodes_allocated_.load(std::memory_order_relaxed);
        size_t deallocated = nodes_deallocated_.load(std::memory_order_relaxed);
        if (allocated != deallocated) {
            std::cerr << "WARNING: Memory leak detected in MPSC channel!\n";
            std::cerr << "  Allocated: " << allocated << "\n";
            std::cerr << "  Deallocated: " << deallocated << "\n";
            std::cerr << "  Leaked: " << (allocated - deallocated) << " nodes\n";
        }
        #endif
    }

    // Try to send a value (non-blocking)
    //
    // Algorithm:
    // 1. Allocate new node with value
    // 2. Atomically swap tail_ with new node
    // 3. Link old tail to new node (publishes to consumer)
    // 4. Notify receiver (for blocking recv)
    //
    // Memory ordering:
    // - exchange uses acq_rel: synchronizes with other producers
    // - next.store uses release: publishes node data to consumer
    // - signal increment uses release: makes data visible before wake
    //
    // Returns: Ok if sent, Err if receiver dropped
    Result<Unit, TrySendError> try_send(T value) {
        // Check if receiver is alive
        if (!receiver_alive_.load(std::memory_order_acquire)) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }

        // Allocate new node
        Node<T>* new_node = new Node<T>(std::move(value));
        new_node->next.store(nullptr, std::memory_order_relaxed);

        #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
        nodes_allocated_.fetch_add(1, std::memory_order_relaxed);
        #endif

        // Atomically append to tail
        // acq_rel: ensures we see all previous tail updates (acquire)
        //          and our update is visible to future operations (release)
        Node<T>* old_tail = tail_.exchange(new_node, std::memory_order_acq_rel);

        // Link old tail to new node
        // release: publishes the node data to consumer
        // The consumer will use acquire load to observe this
        old_tail->next.store(new_node, std::memory_order_release);

        // Notify receiver (increment signal for wake-up)
        // release: ensures message is visible before receiver wakes
        signal_.fetch_add(1, std::memory_order_release);

        // Wake up receiver if it's waiting
        #if __cplusplus >= 202002L
            signal_.notify_one();
        #endif

        return Result<Unit, TrySendError>::Ok(Unit{});
    }

    // Try to receive a value (non-blocking)
    //
    // Algorithm:
    // 1. Load head->next (first unconsumed message)
    // 2. If nullptr, queue is empty
    // 3. Otherwise, extract data and advance head
    // 4. Delete old head node
    //
    // Memory ordering:
    // - next.load uses acquire: observes published node data
    //
    // Note: No atomics needed on head_ itself (single consumer!)
    //
    // Returns: Some(value) if available, None if empty
    Option<T> try_pop() {
        Node<T>* head = head_;

        // Load next pointer with acquire semantics
        // This synchronizes with the producer's release store
        Node<T>* next = head->next.load(std::memory_order_acquire);

        if (next == nullptr) {
            // Queue is empty
            return None;
        }

        // Extract value from next node
        // Use move semantics to avoid copy
        T value = std::move(next->data);

        // Advance head (no atomic needed - single consumer!)
        head_ = next;

        // Delete old head (now safe - we own it)
        delete head;

        #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
        nodes_deallocated_.fetch_add(1, std::memory_order_relaxed);
        #endif

        return Some(std::move(value));
    }

    // Try to receive with error information
    Result<T, TryRecvError> try_recv() {
        auto result = try_pop();

        if (result.is_some()) {
            return Result<T, TryRecvError>::Ok(result.unwrap());
        }

        // Queue is empty - check if disconnected
        if (sender_count_.load(std::memory_order_acquire) == 0) {
            return Result<T, TryRecvError>::Err(TryRecvError::Disconnected);
        }

        return Result<T, TryRecvError>::Err(TryRecvError::Empty);
    }

    // Blocking receive with hybrid wait strategy
    //
    // Strategy:
    // 1. Phase 1: Optimistic spin (fast path for available messages)
    // 2. Phase 2: Exponential backoff (reduce CPU usage)
    // 3. Phase 3: Wait on atomic (C++20) or sleep (C++17)
    //
    // Returns: Ok(value) if received, Err if disconnected
    Result<T, RecvError> blocking_recv() {
        Backoff backoff;

        // Phase 1 & 2: Spin with exponential backoff
        while (!backoff.is_completed()) {
            // Try to pop a message
            auto result = try_pop();
            if (result.is_some()) {
                return Result<T, RecvError>::Ok(result.unwrap());
            }

            // Check if all senders are gone
            if (sender_count_.load(std::memory_order_acquire) == 0) {
                return Result<T, RecvError>::Err(RecvError::Disconnected);
            }

            // Backoff: spin or yield
            backoff.spin();
        }

        // Phase 3: Wait on atomic signal
        #if __cplusplus >= 202002L
            // C++20: Use atomic::wait for efficient blocking
            while (true) {
                // Load current signal value
                uint32_t old_signal = signal_.load(std::memory_order_acquire);

                // Try to pop again before waiting
                auto result = try_pop();
                if (result.is_some()) {
                    return Result<T, RecvError>::Ok(result.unwrap());
                }

                // Check if disconnected
                if (sender_count_.load(std::memory_order_acquire) == 0) {
                    return Result<T, RecvError>::Err(RecvError::Disconnected);
                }

                // Wait for signal to change (spurious wakeups are OK)
                signal_.wait(old_signal, std::memory_order_acquire);
            }
        #else
            // C++17 fallback: Sleep between checks
            while (true) {
                // Try to pop
                auto result = try_pop();
                if (result.is_some()) {
                    return Result<T, RecvError>::Ok(result.unwrap());
                }

                // Check if disconnected
                if (sender_count_.load(std::memory_order_acquire) == 0) {
                    return Result<T, RecvError>::Err(RecvError::Disconnected);
                }

                // Sleep briefly (1 microsecond)
                std::this_thread::sleep_for(std::chrono::microseconds(1));
            }
        #endif
    }

    // Increment sender count (called on Sender clone)
    void increment_sender() {
        sender_count_.fetch_add(1, std::memory_order_relaxed);
    }

    // Decrement sender count (called on Sender drop)
    // Returns true if this was the last sender
    bool decrement_sender() {
        size_t prev = sender_count_.fetch_sub(1, std::memory_order_acq_rel);
        bool was_last = (prev == 1);

        if (was_last) {
            // Last sender dropped - wake up receiver so it can detect disconnection
            signal_.fetch_add(1, std::memory_order_release);
            #if __cplusplus >= 202002L
                signal_.notify_one();
            #endif
        }

        return was_last;
    }

    // Mark receiver as dropped
    void mark_receiver_dropped() {
        receiver_alive_.store(false, std::memory_order_release);
    }

    // Check if receiver is alive
    bool is_receiver_alive() const {
        return receiver_alive_.load(std::memory_order_acquire);
    }

    // Get sender count (for debugging/testing)
    size_t sender_count() const {
        return sender_count_.load(std::memory_order_relaxed);
    }

    // Drain all remaining messages
    // Returns the number of messages drained
    // Useful for explicit cleanup or when you want to discard pending messages
    size_t drain() {
        size_t count = 0;
        while (try_pop().is_some()) {
            count++;
        }
        return count;
    }

    // Count approximate number of messages in queue
    // Note: This walks the entire linked list, O(n) operation
    // Only use for debugging/testing, not in hot paths
    size_t approximate_len() const {
        size_t count = 0;
        Node<T>* curr = head_;
        Node<T>* next = curr->next.load(std::memory_order_acquire);

        while (next != nullptr) {
            count++;
            curr = next;
            next = curr->next.load(std::memory_order_acquire);
        }

        return count;
    }

    // Check if queue is empty
    // Note: Result may be stale immediately due to concurrent sends
    bool is_empty() const {
        Node<T>* next = head_->next.load(std::memory_order_acquire);
        return next == nullptr;
    }

    // Check if all senders have disconnected
    bool is_disconnected() const {
        return sender_count_.load(std::memory_order_acquire) == 0;
    }

    // Batch send operation - send multiple messages with single atomic tail swap
    // Reduces contention when sending many messages at once
    // Returns: Number of messages successfully sent
    //
    // Performance: Much faster than N individual sends
    // - Individual sends: N atomic exchanges
    // - Batch send: 1 atomic exchange + (N-1) relaxed stores
    template<typename Iterator>
    size_t batch_send(Iterator begin, Iterator end) {
        if (begin == end) {
            return 0;  // Nothing to send
        }

        // Check if receiver is alive
        if (!receiver_alive_.load(std::memory_order_acquire)) {
            return 0;  // All failed
        }

        // Build linked chain of nodes
        Node<T>* first = new Node<T>(std::move(*begin));
        first->next.store(nullptr, std::memory_order_relaxed);

        #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
        nodes_allocated_.fetch_add(1, std::memory_order_relaxed);
        #endif

        Node<T>* last = first;
        size_t count = 1;

        ++begin;
        for (auto it = begin; it != end; ++it) {
            Node<T>* node = new Node<T>(std::move(*it));
            node->next.store(nullptr, std::memory_order_relaxed);

            #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
            nodes_allocated_.fetch_add(1, std::memory_order_relaxed);
            #endif

            // Link to chain (relaxed - not published yet)
            last->next.store(node, std::memory_order_relaxed);
            last = node;
            count++;
        }

        // Single atomic operation to append entire chain
        Node<T>* old_tail = tail_.exchange(last, std::memory_order_acq_rel);

        // Publish chain to consumer
        old_tail->next.store(first, std::memory_order_release);

        // Notify receiver (single notification for batch)
        signal_.fetch_add(count, std::memory_order_release);
        #if __cplusplus >= 202002L
            signal_.notify_one();
        #endif

        return count;
    }

    // Batch receive - receive up to N messages in one call
    // Useful for reducing overhead when processing many messages
    // Returns: Vector of received values (may be less than max_count)
    //
    // Performance: Faster than N individual receives
    // - Reduces function call overhead
    // - Better cache locality
    // - Amortizes atomic operations
    std::vector<T> batch_recv(size_t max_count) {
        std::vector<T> results;
        results.reserve(max_count);

        for (size_t i = 0; i < max_count; ++i) {
            auto value = try_pop();
            if (value.is_none()) {
                break;  // Queue empty
            }
            results.push_back(value.unwrap());
        }

        return results;
    }

    #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
    // Get memory statistics (only available with RUSTY_MPSC_TRACK_ALLOCATIONS)
    struct MemoryStats {
        size_t nodes_allocated;
        size_t nodes_deallocated;
        size_t nodes_live;
    };

    MemoryStats memory_stats() const {
        size_t allocated = nodes_allocated_.load(std::memory_order_relaxed);
        size_t deallocated = nodes_deallocated_.load(std::memory_order_relaxed);
        return MemoryStats{
            allocated,
            deallocated,
            allocated - deallocated
        };
    }
    #endif
};

// Sender - can be cloned for multi-producer
template<typename T>
class Sender {
private:
    std::shared_ptr<LockFreeChannelState<T>> state_;

public:
    // Constructor from shared state
    explicit Sender(std::shared_ptr<LockFreeChannelState<T>> state)
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

    // Send a value (non-blocking for now - blocking will be added in Phase 2)
    Result<Unit, TrySendError> send(T value) const {
        if (!state_) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }
        return state_->try_send(std::move(value));
    }

    // Try to send a value (non-blocking)
    Result<Unit, TrySendError> try_send(T value) const {
        if (!state_) {
            return Result<Unit, TrySendError>::Err(TrySendError::Disconnected);
        }
        return state_->try_send(std::move(value));
    }

    // Clone this sender (for multi-producer)
    Sender clone() const {
        return Sender(*this);
    }

    // Batch send - send multiple messages with single atomic operation
    // Much faster than individual sends when sending many messages
    //
    // Example:
    //   std::vector<int> batch = {1, 2, 3, 4, 5};
    //   size_t sent = tx.send_batch(batch.begin(), batch.end());
    //
    // Performance: 1 atomic exchange instead of N
    template<typename Iterator>
    size_t send_batch(Iterator begin, Iterator end) const {
        if (!state_) {
            return 0;
        }
        return state_->batch_send(begin, end);
    }

    // Convenience overload for containers
    template<typename Container>
    size_t send_batch(Container&& container) const {
        return send_batch(container.begin(), container.end());
    }
};

// Receiver - cannot be cloned (single-consumer)
template<typename T>
class Receiver {
private:
    std::shared_ptr<LockFreeChannelState<T>> state_;

public:
    // Constructor from shared state
    explicit Receiver(std::shared_ptr<LockFreeChannelState<T>> state)
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

    // Try to receive a value (non-blocking)
    Result<T, TryRecvError> try_recv() {
        if (!state_) {
            return Result<T, TryRecvError>::Err(TryRecvError::Disconnected);
        }
        return state_->try_recv();
    }

    // Blocking receive - waits for message or disconnection
    //
    // This method blocks until:
    // - A message is available (returns Ok(value))
    // - All senders are dropped (returns Err(Disconnected))
    //
    // Waiting strategy:
    // 1. Spin briefly (low latency for ready messages)
    // 2. Exponential backoff (reduce CPU usage)
    // 3. Wait on atomic signal (efficient blocking)
    Result<T, RecvError> recv() {
        if (!state_) {
            return Result<T, RecvError>::Err(RecvError::Disconnected);
        }

        return state_->blocking_recv();
    }

    // Iterator-like interface
    Option<T> recv_opt() {
        auto result = try_recv();
        if (result.is_ok()) {
            return Some(result.unwrap());
        }
        return None;
    }

    // Drain all remaining messages from the channel
    // Returns the number of messages drained
    // Useful for explicit cleanup or discarding pending messages
    //
    // Example:
    //   size_t discarded = rx.drain();
    //   std::cout << "Discarded " << discarded << " messages\n";
    size_t drain() {
        if (!state_) {
            return 0;
        }
        return state_->drain();
    }

    // Get approximate number of messages in queue
    // Note: This is O(n) and walks the entire list
    // Only use for debugging/testing, not in hot paths
    //
    // Note: Result may be stale immediately after returning
    // due to concurrent sends
    size_t approximate_len() const {
        if (!state_) {
            return 0;
        }
        return state_->approximate_len();
    }

    // Check if queue is empty
    // Note: Result may be stale immediately due to concurrent sends
    bool is_empty() const {
        if (!state_) {
            return true;
        }
        return state_->is_empty();
    }

    // Check if all senders have disconnected
    bool is_disconnected() const {
        if (!state_) {
            return true;
        }
        return state_->is_disconnected();
    }

    // Batch receive - receive up to max_count messages
    // Returns vector of received messages (may be less than max_count)
    //
    // Performance: Faster than N individual receives
    // - Reduces function call overhead
    // - Better cache locality
    //
    // Example:
    //   auto messages = rx.recv_batch(100);  // Receive up to 100
    //   for (const auto& msg : messages) {
    //       process(msg);
    //   }
    std::vector<T> recv_batch(size_t max_count) {
        if (!state_) {
            return std::vector<T>();
        }
        return state_->batch_recv(max_count);
    }

    #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
    // Get memory statistics (only available with RUSTY_MPSC_TRACK_ALLOCATIONS)
    // Useful for detecting memory leaks during development/testing
    auto memory_stats() const {
        using MemoryStats = typename LockFreeChannelState<T>::MemoryStats;
        if (!state_) {
            return MemoryStats{0, 0, 0};
        }
        return state_->memory_stats();
    }
    #endif
};

// Factory function to create a lock-free channel
// Requires T to be Send (same as mutex version)
#if __cplusplus >= 202002L  // C++20 concepts
template<Send T>
std::pair<Sender<T>, Receiver<T>> channel() {
    auto state = std::make_shared<LockFreeChannelState<T>>();
    return std::make_pair(
        Sender<T>(state),
        Receiver<T>(state)
    );
}
#else  // Pre-C++20: static_assert in LockFreeChannelState will catch violations
template<typename T>
std::pair<Sender<T>, Receiver<T>> channel() {
    auto state = std::make_shared<LockFreeChannelState<T>>();
    return std::make_pair(
        Sender<T>(state),
        Receiver<T>(state)
    );
}
#endif

} // namespace lockfree
} // namespace mpsc
} // namespace sync
} // namespace rusty
