// Tests for rusty::Condvar
#include "../include/rusty/condvar.hpp"
#include <cassert>
#include <cstdio>
#include <thread>
#include <mutex>
#include <vector>
#include <chrono>
#include <atomic>

using namespace rusty;
using namespace std::chrono_literals;

// Test basic wait and notify_one
void test_condvar_wait_notify_one() {
    printf("test_condvar_wait_notify_one: ");
    {
        std::mutex mtx;
        Condvar cv;
        bool ready = false;

        std::thread waiter([&]() {
            std::unique_lock lock(mtx);
            cv.wait(lock, [&]{ return ready; });
            assert(ready);
        });

        std::this_thread::sleep_for(50ms);

        {
            std::unique_lock lock(mtx);
            ready = true;
        }
        cv.notify_one();

        waiter.join();
    }
    printf("PASS\n");
}

// Test notify_all
void test_condvar_notify_all() {
    printf("test_condvar_notify_all: ");
    {
        std::mutex mtx;
        Condvar cv;
        bool ready = false;
        std::atomic<int> woken{0};

        std::vector<std::thread> threads;
        for (int i = 0; i < 5; ++i) {
            threads.emplace_back([&]() {
                std::unique_lock lock(mtx);
                cv.wait(lock, [&]{ return ready; });
                woken++;
            });
        }

        std::this_thread::sleep_for(50ms);

        {
            std::unique_lock lock(mtx);
            ready = true;
        }
        cv.notify_all();

        for (auto& t : threads) {
            t.join();
        }

        assert(woken == 5);
    }
    printf("PASS\n");
}

// Test wait_for with timeout
void test_condvar_wait_for_timeout() {
    printf("test_condvar_wait_for_timeout: ");
    {
        std::mutex mtx;
        Condvar cv;

        std::unique_lock lock(mtx);
        auto start = std::chrono::steady_clock::now();
        bool notified = cv.wait_for(lock, 100ms);
        auto end = std::chrono::steady_clock::now();

        assert(!notified);  // Should timeout
        auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(end - start);
        assert(elapsed >= 100ms);
    }
    printf("PASS\n");
}

// Test wait_for with notification
void test_condvar_wait_for_notified() {
    printf("test_condvar_wait_for_notified: ");
    {
        std::mutex mtx;
        Condvar cv;
        bool notified = false;

        std::thread notifier([&]() {
            std::this_thread::sleep_for(50ms);
            {
                std::unique_lock lock(mtx);
                notified = true;
            }
            cv.notify_one();
        });

        std::unique_lock lock(mtx);
        bool result = cv.wait_for(lock, 200ms, [&]{ return notified; });

        assert(result);  // Should be notified, not timeout

        notifier.join();
    }
    printf("PASS\n");
}

// Test producer-consumer pattern
void test_condvar_producer_consumer() {
    printf("test_condvar_producer_consumer: ");
    {
        std::mutex mtx;
        Condvar cv;
        std::vector<int> queue;
        bool done = false;
        std::vector<int> consumed;

        std::thread consumer([&]() {
            while (true) {
                std::unique_lock lock(mtx);
                cv.wait(lock, [&]{ return !queue.empty() || done; });

                if (!queue.empty()) {
                    int item = queue.back();
                    queue.pop_back();
                    lock.unlock();
                    consumed.push_back(item);
                } else if (done) {
                    break;
                }
            }
        });

        std::thread producer([&]() {
            for (int i = 1; i <= 10; ++i) {
                {
                    std::unique_lock lock(mtx);
                    queue.push_back(i);
                }
                cv.notify_one();
                std::this_thread::sleep_for(10ms);
            }

            {
                std::unique_lock lock(mtx);
                done = true;
            }
            cv.notify_one();
        });

        producer.join();
        consumer.join();

        assert(consumed.size() == 10);
        assert(queue.empty());
    }
    printf("PASS\n");
}

// Test wait without predicate (manual check)
void test_condvar_wait_manual() {
    printf("test_condvar_wait_manual: ");
    {
        std::mutex mtx;
        Condvar cv;
        int value = 0;

        std::thread waiter([&]() {
            std::unique_lock lock(mtx);
            while (value < 10) {
                cv.wait(lock);
            }
            assert(value == 10);
        });

        std::this_thread::sleep_for(50ms);

        {
            std::unique_lock lock(mtx);
            value = 10;
        }
        cv.notify_one();

        waiter.join();
    }
    printf("PASS\n");
}

// Test wait_until
void test_condvar_wait_until() {
    printf("test_condvar_wait_until: ");
    {
        std::mutex mtx;
        Condvar cv;

        auto deadline = std::chrono::steady_clock::now() + 100ms;

        std::unique_lock lock(mtx);
        bool notified = cv.wait_until(lock, deadline);

        assert(!notified);  // Should timeout
        assert(std::chrono::steady_clock::now() >= deadline);
    }
    printf("PASS\n");
}

int main() {
    printf("Running Condvar tests...\n");
    printf("=======================\n");

    test_condvar_wait_notify_one();
    test_condvar_notify_all();
    test_condvar_wait_for_timeout();
    test_condvar_wait_for_notified();
    test_condvar_producer_consumer();
    test_condvar_wait_manual();
    test_condvar_wait_until();

    printf("\nAll Condvar tests passed!\n");
    return 0;
}
