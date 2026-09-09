#ifdef NDEBUG
#undef NDEBUG
#endif

#include <rusty/thread.hpp>
#include <atomic>
#include <cassert>
#include <chrono>
#include <memory>
#include <thread>

struct OwnedArgument {
    static constexpr bool is_send = true;
    std::unique_ptr<int> value;
};

void joined_move_only_values() {
    auto task = [base = std::make_unique<int>(5)](OwnedArgument argument) mutable {
        return std::make_unique<int>(*base + *argument.value);
    };
    auto worker = rusty::thread::spawn(std::move(task), OwnedArgument{std::make_unique<int>(7)});
    auto result = worker.join();
    assert(result.is_ok());
    assert(*result.unwrap() == 12);
}

void joined_void_result() {
    bool ran = false;
    auto worker = rusty::thread::spawn([&ran] { ran = true; });
    static_assert(std::is_same_v<decltype(worker), rusty::thread::JoinHandle<std::tuple<>>>);
    assert(worker.join().is_ok());
    assert(ran);
}

void scoped_argument_forwarding() {
    int original = 10;
    rusty::thread::scope([&](auto& scope) {
        // An lvalue argument is stored by value, then passed as an lvalue.
        auto copied = scope.spawn([](int& value) { return ++value; }, original);
        assert(copied.join().unwrap() == 11);
        assert(original == 10);
        // Explicit reference wrappers keep the caller's object borrowed.
        auto borrowed = scope.spawn([](int& value) { ++value; }, std::ref(original));
        assert(borrowed.join().is_ok());
        assert(original == 11);
        auto owned = scope.spawn([](std::unique_ptr<int> value) { return *value; },
                                 std::make_unique<int>(17));
        assert(owned.join().unwrap() == 17);
    });
}

namespace {
std::atomic<unsigned> completed{0};
std::atomic<unsigned> destroyed{0};

struct CompletionOwner {
    ~CompletionOwner() { destroyed.fetch_add(1, std::memory_order_release); }
};

void wait_for(const std::atomic<unsigned>& counter, unsigned target) {
    auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(10);
    while (counter.load(std::memory_order_acquire) < target) {
        assert(std::chrono::steady_clock::now() < deadline);
        std::this_thread::yield();
    }
}
}

void detached_owner_lifetimes() {
    constexpr unsigned count = 2000;
    for (unsigned i = 0; i != count; ++i) {
        auto worker = rusty::thread::spawn(
            [owner = std::make_unique<CompletionOwner>(), i] {
                // The observer may drop the handle immediately after this.
                assert(owner != nullptr);
                completed.store(i + 1, std::memory_order_release);
            });
        if (i % 2 == 0) worker.detach();
        wait_for(completed, i + 1);
        // Odd iterations exercise JoinHandle's detach-on-drop behavior.
    }
    wait_for(destroyed, count);
    assert(destroyed.load() == count);
}

int main() {
    joined_move_only_values();
    joined_void_result();
    scoped_argument_forwarding();
    detached_owner_lifetimes();
}
