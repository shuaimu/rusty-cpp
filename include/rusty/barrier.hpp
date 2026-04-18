#pragma once

#include <condition_variable>
#include <cstddef>
#include <memory>
#include <mutex>
#include <stdexcept>

namespace rusty {

// Barrier - Synchronization point for multiple threads
// Matches Rust's std::sync::Barrier behavior
//
// Usage:
//   Barrier barrier(3);  // Wait for 3 threads
//
//   // In each thread:
//   BarrierWaitResult result = barrier.wait();
//   if (result.is_leader()) {
//       // One thread (the leader) will see is_leader() == true
//       // This thread can perform cleanup or coordination tasks
//   }
//
class Barrier {
private:
    struct State {
        mutable std::mutex mtx;
        mutable std::condition_variable cv;
        std::size_t threshold;
        mutable std::size_t count;
        mutable std::size_t generation;

        explicit State(std::size_t count_)
            : threshold(count_), count(count_), generation(0) {}
    };

    // Keep state behind indirection so Barrier stays movable for tuple/vector
    // construction patterns used by transpiled tests.
    std::unique_ptr<State> state_;

public:
    // Result of a barrier wait operation
    class BarrierWaitResult {
    private:
        bool is_leader_;

        friend class Barrier;

        explicit BarrierWaitResult(bool leader) : is_leader_(leader) {}

    public:
        // Returns true if this thread is the "leader" (last to arrive)
        // The leader thread is chosen arbitrarily from the waiting threads
        bool is_leader() const { return is_leader_; }
    };

    // Constructor - specify number of threads that must call wait()
    explicit Barrier(std::size_t count)
        : state_(std::make_unique<State>(count)) {
        if (count == 0) {
            throw std::invalid_argument("Barrier count must be greater than 0");
        }
    }

    static Barrier new_(std::size_t count) { return Barrier(count); }

    // Wait for all threads to arrive at the barrier
    // Returns a BarrierWaitResult indicating if this thread is the leader
    BarrierWaitResult wait() const {
        std::unique_lock<std::mutex> lock(state_->mtx);
        std::size_t gen = state_->generation;

        if (--state_->count == 0) {
            // Last thread to arrive - this is the leader
            state_->generation++;
            state_->count = state_->threshold;
            state_->cv.notify_all();
            return BarrierWaitResult(true);
        }

        // Not the last thread - wait for the leader to release everyone
        state_->cv.wait(lock, [this, gen] { return gen != state_->generation; });
        return BarrierWaitResult(false);
    }

    // Non-copyable, movable.
    Barrier(const Barrier&) = delete;
    Barrier& operator=(const Barrier&) = delete;
    Barrier(Barrier&&) noexcept = default;
    Barrier& operator=(Barrier&&) noexcept = default;

    ~Barrier() = default;
};

} // namespace rusty
