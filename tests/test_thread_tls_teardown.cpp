#ifdef NDEBUG
#undef NDEBUG
#endif

#include <rusty/thread.hpp>
#include <atomic>
#include <cassert>
#include <chrono>
#include <condition_variable>
#include <cstdlib>
#include <mutex>
#include <optional>
#include <pthread.h>
#include <thread>

using Token = rusty::thread::detail::SharedState<rusty::thread::detail::ParkTokenShared>;
std::atomic<unsigned> completed{0};
unsigned expected_completed = 0;
bool late_static_ran = false;

void final_exit_check() { assert(late_static_ran); }
struct LateStatic {
    LateStatic() { assert(std::atexit(final_exit_check) == 0); }
    ~LateStatic() {
        assert(completed.load() == expected_completed);
        // This destructor was registered before the first current() call,
        // so it runs after the initial main-thread token cleanup.
        auto current = rusty::thread::current();
        assert(current.id() == rusty::thread::current_id());
        current.unpark();
        rusty::thread::park();
        late_static_ran = true;
        // LSan must find neither the initial nor this re-created slot leaked.
    }
} late_static;

struct Handoff {
    std::mutex mutex;
    std::condition_variable cv;
    std::optional<rusty::thread::Thread> handle;
    Token retained_token;
    bool ready = false;
    bool finish = false;
    bool destructor_parking = false;
};

struct TlsProbe {
    rusty::thread::ThreadId id = rusty::thread::current_id();
    rusty::thread::detail::ParkTokenShared* identity = nullptr;
    Handoff* handoff = nullptr;
    ~TlsProbe() {
        auto current = rusty::thread::current();
        assert(current.id() == id);
        for (int i = 0; i < 8; ++i) {
            auto token = rusty::thread::detail::current_park_token();
            if (identity) assert(token.get() == identity);
        }
        if (handoff) {
            {
                std::lock_guard lock(handoff->mutex);
                handoff->destructor_parking = true;
            }
            handoff->cv.notify_one();
            rusty::thread::park();
        } else {
            current.unpark();
            rusty::thread::park();
        }
        completed.fetch_add(1);
    }
};

thread_local std::optional<TlsProbe> probe;

void prepare(bool token_first, bool first_use_in_destructor, Handoff* handoff = nullptr) {
    if (token_first) (void)rusty::thread::current();
    // The optional's destructor is registered before its emplaced value calls
    // current(), reproducing lazy initialization inside an existing TLS slot.
    probe.emplace();
    if (!first_use_in_destructor) {
        auto current = rusty::thread::current();
        auto token = rusty::thread::detail::current_park_token();
        probe->identity = token.get();
        if (handoff) {
            probe->handoff = handoff;
            std::unique_lock lock(handoff->mutex);
            handoff->handle.emplace(std::move(current));
            handoff->retained_token = std::move(token);
            handoff->ready = true;
            handoff->cv.notify_one();
            assert(handoff->cv.wait_for(lock, std::chrono::seconds(10), [&] {
                return handoff->finish;
            }));
        }
    }
}

void external_pthread(bool token_first, bool first_use_in_destructor) {
    struct Arguments { bool token_first; bool first_use; } args{token_first, first_use_in_destructor};
    pthread_t worker;
    assert(pthread_create(&worker, nullptr, +[](void* opaque) -> void* {
        auto& args = *static_cast<Arguments*>(opaque);
        prepare(args.token_first, args.first_use);
        return nullptr;
    }, &args) == 0);
    assert(pthread_join(worker, nullptr) == 0);
}

void retained_handle_unparks_during_and_after_teardown() {
    Handoff handoff;
    std::thread worker([&] { prepare(false, false, &handoff); });
    {
        std::unique_lock lock(handoff.mutex);
        assert(handoff.cv.wait_for(lock, std::chrono::seconds(10), [&] { return handoff.ready; }));
        handoff.finish = true;
        handoff.cv.notify_one();
        assert(handoff.cv.wait_for(lock, std::chrono::seconds(10), [&] {
            return handoff.destructor_parking;
        }));
    }
    handoff.handle->unpark();
    worker.join();
    // Native cleanup released exactly its own reference. Both escaped owners
    // remain valid even though the target thread has completely exited.
    assert(handoff.retained_token->refcount.load() == 2);
    handoff.handle->unpark();
    handoff.handle.reset();
    assert(handoff.retained_token->refcount.load() == 1);
}

int main() {
    for (int i = 0; i < 32; ++i) {
        std::thread([i] { prepare(i % 2 == 0, false); }).join();
        external_pthread(i % 2 == 0, false);
        rusty::platform::threading::thread worker([i] { prepare(i % 2 == 0, false); });
        worker.join();
    }
    std::thread([] { prepare(false, true); }).join();
    external_pthread(false, true);
    retained_handle_unparks_during_and_after_teardown();
    prepare(false, false); // Normal main return exercises TLS then atexit.
    expected_completed = completed.load() + 1;
}
