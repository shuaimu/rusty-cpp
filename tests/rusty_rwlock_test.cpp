// Tests for rusty::RwLock<T>
#include "../include/rusty/rwlock.hpp"
#include <cassert>
#include <cstdio>
#include <thread>
#include <vector>
#include <chrono>
#include <atomic>

using namespace rusty;
using namespace std::chrono_literals;

// Test basic read lock
void test_rwlock_read() {
    printf("test_rwlock_read: ");
    {
        RwLock<int> lock(42);
        auto read_guard = lock.read();
        assert(*read_guard == 42);
        assert(read_guard.get() != nullptr);
    }
    printf("PASS\n");
}

// Test basic write lock
void test_rwlock_write() {
    printf("test_rwlock_write: ");
    {
        RwLock<int> lock(42);
        auto write_guard = lock.write();
        assert(*write_guard == 42);
        *write_guard = 100;
        assert(*write_guard == 100);
    }
    {
        RwLock<int> lock(42);
        auto read_guard = lock.read();
        assert(*read_guard == 42);  // Original value preserved
    }
    printf("PASS\n");
}

// Test multiple readers
void test_rwlock_multiple_readers() {
    printf("test_rwlock_multiple_readers: ");
    {
        RwLock<std::vector<int>> lock(std::vector<int>{1, 2, 3});
        std::vector<std::thread> threads;
        std::atomic<int> counter{0};

        // Create 5 reader threads
        for (int i = 0; i < 5; ++i) {
            threads.emplace_back([&lock, &counter]() {
                auto read_guard = lock.read();
                assert(read_guard->size() == 3);
                counter++;
                std::this_thread::sleep_for(10ms);
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(counter == 5);
    }
    printf("PASS\n");
}

// Test write exclusivity
void test_rwlock_write_exclusive() {
    printf("test_rwlock_write_exclusive: ");
    {
        RwLock<int> lock(0);
        std::vector<std::thread> threads;

        // Create 10 threads that increment
        for (int i = 0; i < 10; ++i) {
            threads.emplace_back([&lock]() {
                for (int j = 0; j < 100; ++j) {
                    auto write_guard = lock.write();
                    (*write_guard)++;
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        auto read_guard = lock.read();
        assert(*read_guard == 1000);  // All increments should be atomic
    }
    printf("PASS\n");
}

// Test try_read
void test_rwlock_try_read() {
    printf("test_rwlock_try_read: ");
    {
        RwLock<int> lock(42);

        auto maybe_guard = lock.try_read();
        assert(maybe_guard.is_some());
        auto read_guard = maybe_guard.unwrap();
        assert(*read_guard == 42);
    }
    printf("PASS\n");
}

// Test try_write
void test_rwlock_try_write() {
    printf("test_rwlock_try_write: ");
    {
        RwLock<int> lock(42);

        auto maybe_guard = lock.try_write();
        assert(maybe_guard.is_some());
        auto write_guard = maybe_guard.unwrap();
        *write_guard = 100;
        assert(*write_guard == 100);
    }
    printf("PASS\n");
}

// Test WriteGuard get_mut
void test_rwlock_get_mut() {
    printf("test_rwlock_get_mut: ");
    {
        RwLock<std::vector<int>> lock(std::vector<int>{1, 2, 3});
        auto write_guard = lock.write();

        std::vector<int>& vec = write_guard.get_mut();
        vec.push_back(4);
        assert(vec.size() == 4);
        assert(vec[3] == 4);
    }
    printf("PASS\n");
}

// Test WriteGuard into_inner
void test_rwlock_into_inner() {
    printf("test_rwlock_into_inner: ");
    {
        RwLock<std::vector<int>> lock(std::vector<int>{1, 2, 3});
        auto write_guard = lock.write();

        std::vector<int> vec = std::move(write_guard).into_inner();
        assert(vec.size() == 3);
        assert(vec[0] == 1);
    }
    printf("PASS\n");
}

// Test reader-writer interaction
void test_rwlock_reader_writer_interaction() {
    printf("test_rwlock_reader_writer_interaction: ");
    {
        RwLock<int> lock(0);
        std::atomic<int> read_count{0};
        std::atomic<int> write_count{0};
        std::vector<std::thread> threads;

        // 5 readers
        for (int i = 0; i < 5; ++i) {
            threads.emplace_back([&]() {
                for (int j = 0; j < 50; ++j) {
                    auto read_guard = lock.read();
                    read_count++;
                    std::this_thread::sleep_for(1ms);
                }
            });
        }

        // 2 writers
        for (int i = 0; i < 2; ++i) {
            threads.emplace_back([&]() {
                for (int j = 0; j < 50; ++j) {
                    auto write_guard = lock.write();
                    (*write_guard)++;
                    write_count++;
                    std::this_thread::sleep_for(1ms);
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        assert(read_count == 250);  // 5 threads * 50 reads
        assert(write_count == 100); // 2 threads * 50 writes

        auto final_value = lock.read();
        assert(*final_value == 100);
    }
    printf("PASS\n");
}

int main() {
    printf("Running RwLock tests...\n");
    printf("======================\n");

    test_rwlock_read();
    test_rwlock_write();
    test_rwlock_multiple_readers();
    test_rwlock_write_exclusive();
    test_rwlock_try_read();
    test_rwlock_try_write();
    test_rwlock_get_mut();
    test_rwlock_into_inner();
    test_rwlock_reader_writer_interaction();

    printf("\nAll RwLock tests passed!\n");
    return 0;
}
