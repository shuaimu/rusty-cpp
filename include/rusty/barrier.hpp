#pragma once

#include <mutex>
#include <condition_variable>
#include <cstddef>

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
    std::mutex mtx_;
    std::condition_variable cv_;
    std::size_t threshold_;
    std::size_t count_;
    std::size_t generation_;

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
        : threshold_(count), count_(count), generation_(0) {
        if (count == 0) {
            throw std::invalid_argument("Barrier count must be greater than 0");
        }
    }

    // Wait for all threads to arrive at the barrier
    // Returns a BarrierWaitResult indicating if this thread is the leader
    BarrierWaitResult wait() {
        std::unique_lock<std::mutex> lock(mtx_);
        std::size_t gen = generation_;

        if (--count_ == 0) {
            // Last thread to arrive - this is the leader
            generation_++;
            count_ = threshold_;
            cv_.notify_all();
            return BarrierWaitResult(true);
        }

        // Not the last thread - wait for the leader to release everyone
        cv_.wait(lock, [this, gen] { return gen != generation_; });
        return BarrierWaitResult(false);
    }

    // Non-copyable, non-movable
    Barrier(const Barrier&) = delete;
    Barrier& operator=(const Barrier&) = delete;
    Barrier(Barrier&&) = delete;
    Barrier& operator=(Barrier&&) = delete;

    ~Barrier() = default;
};

} // namespace rusty
