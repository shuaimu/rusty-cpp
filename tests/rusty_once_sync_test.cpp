// rusty::Once / OnceCell / OnceLock: every reader reaches the initializer's
// writes through the cell's own acquire/release atomic, never only through
// std::call_once's internal handoff.
//
// Built twice. The plain build checks the behaviour (initializers run once,
// completed cells are read without re-entering, `set` after initialization is
// a no-op). The ThreadSanitizer build (rusty_once_sync_tsan_test, libc++)
// fails on any race report. libc++'s call_once checks completion with an
// inline acquire load, but the matching release store is made inside the
// uninstrumented shared library, so a thread that called get_or_init AFTER
// another had initialized the cell, with nothing else ordering the two,
// read the value with no happens-before edge TSan could see: SRPC's TSan
// battery failed that way on Lion's Instant::now() start time. (A thread
// that WAITED inside call_once is ordered through the pthread mutex libc++
// waits on, which TSan does see.)
#include "../include/rusty/once.hpp"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <thread>
#include <vector>

#define CHECK(cond)                                                              \
    do {                                                                         \
        if (!(cond)) {                                                           \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__, __LINE__, \
                         #cond);                                                 \
            std::abort();                                                        \
        }                                                                        \
    } while (0)

namespace {

using namespace std::chrono_literals;

// One thread initializes; another, started with it and ordered after it only
// by the cell, reads after the initializer finished (Lion's Instant::now()
// shape: the first `now()` on one thread, the next on another).
void test_once_lock_read_after_initialization() {
    rusty::OnceLock<std::string> start;
    std::atomic<int> runs{0};
    std::string seen[2];
    auto init = [&runs] {
        runs.fetch_add(1, std::memory_order_relaxed);
        std::this_thread::sleep_for(20ms);
        return std::string("lion-start");
    };
    std::thread first([&] { seen[0] = start.get_or_init(init); });
    std::thread late([&] {
        std::this_thread::sleep_for(80ms);
        seen[1] = start.get_or_init(init);
    });
    first.join();
    late.join();
    CHECK(runs.load() == 1);
    CHECK(seen[0] == "lion-start" && seen[1] == "lion-start");
    CHECK(start.get().is_some() && start.get().unwrap() == "lion-start");
}

// A reader that arrives while the initializer runs waits for it.
void test_once_lock_waiter() {
    rusty::OnceLock<std::string> start;
    std::atomic<int> runs{0};
    std::string seen[2];
    auto init = [&runs] {
        runs.fetch_add(1, std::memory_order_relaxed);
        std::this_thread::sleep_for(40ms);
        return std::string("lion-start");
    };
    std::thread first([&] { seen[0] = start.get_or_init(init); });
    std::thread waiter([&] {
        std::this_thread::sleep_for(10ms);
        seen[1] = start.get_or_init(init);
    });
    first.join();
    waiter.join();
    CHECK(runs.load() == 1);
    CHECK(seen[0] == "lion-start" && seen[1] == "lion-start");
}

// Once publishing a plain (non-atomic) global, as `INIT.call_once(|| ..)`
// guarding a static does.
int g_config = 0;

void test_once_publishes_its_effects() {
    rusty::Once init;
    CHECK(!init.is_completed());
    std::atomic<int> runs{0};
    int seen[3] = {0, 0, 0};
    auto body = [&runs] {
        runs.fetch_add(1, std::memory_order_relaxed);
        std::this_thread::sleep_for(20ms);
        g_config = 42;
    };
    std::thread first([&] { init.call_once(body); seen[0] = g_config; });
    std::thread late([&] {
        std::this_thread::sleep_for(80ms);
        init.call_once(body);
        seen[2] = g_config;
    });
    first.join();
    late.join();
    std::thread waiter([&] { init.call_once(body); seen[1] = g_config; });
    waiter.join();
    CHECK(init.is_completed());
    CHECK(runs.load() == 1);
    for (int v : seen) {
        CHECK(v == 42);
    }
}

// set() racing set(): exactly one wins; get() and get_or_init() afterwards see
// the winner without running anything.
void test_once_cell_set_race() {
    rusty::OnceCell<std::vector<int>> cell;
    std::atomic<int> wins{0};
    std::vector<std::thread> threads;
    for (int i = 0; i < 8; ++i) {
        threads.emplace_back([&cell, &wins, i] {
            if (cell.set(std::vector<int>(4, i))) {
                wins.fetch_add(1, std::memory_order_relaxed);
            }
            const std::vector<int>* v = cell.get();
            CHECK(v != nullptr && v->size() == 4);
            CHECK((*v)[0] == (*v)[3]);
        });
    }
    for (auto& t : threads) {
        t.join();
    }
    CHECK(wins.load() == 1);
    bool ran = false;
    const auto& v = cell.get_or_init([&ran] {
        ran = true;
        return std::vector<int>{};
    });
    CHECK(!ran && v.size() == 4);
    CHECK(!cell.set(std::vector<int>{1}));
}

}  // namespace

int main() {
    test_once_lock_read_after_initialization();
    test_once_lock_waiter();
    test_once_publishes_its_effects();
    test_once_cell_set_race();
    std::printf("rusty_once_sync_test: all passed\n");
    return 0;
}
