#include <rusty/mutex.hpp>
#include <rusty/arc.hpp>
#include <rusty/thread.hpp>
#include <rusty/vec.hpp>
#include <iostream>
#include <cassert>

using namespace rusty;

void test_basic_locking() {
    std::cout << "Test: Basic locking... ";

    Mutex<int> m(42);

    {
        auto guard = m.lock();
        assert(*guard == 42);
        *guard = 100;
    }

    {
        auto guard = m.lock();
        assert(*guard == 100);
    }

    std::cout << "PASSED\n";
}

void test_try_lock() {
    std::cout << "Test: Try lock... ";

    Mutex<int> m(42);

    {
        auto guard1 = m.lock();
        assert(*guard1 == 42);

        // Try to acquire while already locked
        auto guard2 = m.try_lock();
        assert(guard2.is_none());  // Should fail (using rusty::Option API)
    }

    // After first guard released, should succeed
    auto guard3 = m.try_lock().unwrap();  // Using unwrap() now!
    assert(*guard3 == 42);

    std::cout << "PASSED\n";
}

void test_move_guard() {
    std::cout << "Test: Move guard... ";

    Mutex<int> m(42);

    auto guard1 = m.lock();
    *guard1 = 100;

    // Move guard
    auto guard2 = std::move(guard1);
    assert(*guard2 == 100);

    std::cout << "PASSED\n";
}

void test_arrow_operator() {
    std::cout << "Test: Arrow operator... ";

    struct Data {
        int x;
        int y;
    };

    Mutex<Data> m(Data{10, 20});

    {
        auto guard = m.lock();
        assert(guard->x == 10);
        assert(guard->y == 20);

        guard->x = 30;
        guard->y = 40;
    }

    auto guard = m.lock();
    assert(guard->x == 30);
    assert(guard->y == 40);

    std::cout << "PASSED\n";
}

void test_thread_safety() {
    std::cout << "Test: Thread safety (10 threads, 1000 increments each)... ";

    auto counter = Arc<Mutex<int>>::make_in_place(0);
    Vec<thread::JoinHandle<void>> handles;

    for (int i = 0; i < 10; ++i) {
        auto handle = thread::spawn(
            [](Arc<Mutex<int>> counter) {
                for (int j = 0; j < 1000; ++j) {
                    auto guard = counter->lock();
                    *guard += 1;
                }
            },
            counter
        );
        handles.push(std::move(handle));
    }

    // Join all threads
    for (auto& h : handles) {
        h.join();
    }

    auto final_value = counter->lock();
    assert(*final_value == 10000);

    std::cout << "PASSED (final value: " << *final_value << ")\n";
}

void test_scoped_threads_with_mutex() {
    std::cout << "Test: Scoped threads with shared Mutex... ";

    Mutex<int> counter(0);

    thread::scope([&counter](auto& s) {
        for (int i = 0; i < 10; ++i) {
            s.spawn([&counter]() {
                for (int j = 0; j < 100; ++j) {
                    auto guard = counter.lock();
                    *guard += 1;
                }
            });
        }
    });

    auto result = counter.lock();
    assert(*result == 1000);

    std::cout << "PASSED (final value: " << *result << ")\n";
}

void test_const_mutex() {
    std::cout << "Test: Const Mutex... ";

    const Mutex<int> m(42);

    // Can still lock const Mutex (interior mutability)
    auto guard = m.lock();
    assert(*guard == 42);

    std::cout << "PASSED\n";
}

void test_new_api() {
    std::cout << "Test: New API (unwrap, expect, into_inner, get_mut)... ";

    Mutex<int> m(100);

    // Test unwrap() - cleaner than has_value() + **
    {
        auto guard = m.try_lock().unwrap();
        assert(*guard == 100);
    }

    // Test expect() with custom message
    {
        auto guard = m.try_lock().expect("Should succeed when unlocked");
        assert(*guard == 100);
    }

    // Test get_mut() - Rust-like naming
    {
        auto guard = m.lock();
        guard.get_mut() = 200;
        assert(*guard == 200);
    }

    // Test into_inner() - consumes guard and extracts value
    {
        auto guard = m.lock();
        int value = std::move(guard).into_inner();
        assert(value == 200);
        // guard is now moved-from, lock released
    }

    // Test is_none() with rusty::Option
    {
        auto guard1 = m.lock();
        auto maybe_guard2 = m.try_lock();
        assert(maybe_guard2.is_none());  // Can't lock twice
    }

    std::cout << "PASSED\n";
}

int main() {
    std::cout << "Running Mutex tests...\n\n";

    test_basic_locking();
    test_try_lock();
    test_move_guard();
    test_arrow_operator();
    test_thread_safety();
    test_scoped_threads_with_mutex();
    test_const_mutex();
    test_new_api();

    std::cout << "\nAll Mutex tests passed!\n";
    return 0;
}
