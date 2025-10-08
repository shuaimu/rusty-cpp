#include <rusty/thread.hpp>
#include <rusty/arc.hpp>
#include <rusty/rc.hpp>
#include <rusty/mutex.hpp>
#include <rusty/vec.hpp>
#include <iostream>
#include <cassert>
#include <chrono>

using namespace rusty;

void test_basic_spawn() {
    std::cout << "Test: Basic spawn with return value... ";

    auto handle = thread::spawn([]() {
        return 42;
    });

    int result = handle.join();
    assert(result == 42);

    std::cout << "PASSED\n";
}

void test_spawn_with_arguments() {
    std::cout << "Test: Spawn with arguments... ";

    auto handle = thread::spawn(
        [](int a, int b) {
            return a + b;
        },
        20, 22
    );

    int result = handle.join();
    assert(result == 42);

    std::cout << "PASSED\n";
}

void test_spawn_void() {
    std::cout << "Test: Spawn with void return... ";

    auto flag = Arc<Mutex<bool>>::make_in_place(false);

    auto handle = thread::spawn(
        [](Arc<Mutex<bool>> flag) {
            auto guard = flag->lock();
            *guard = true;
        },
        flag
    );

    handle.join();

    auto guard = flag->lock();
    assert(*guard == true);

    std::cout << "PASSED\n";
}

void test_detach_on_drop() {
    std::cout << "Test: Detach on drop (Rust semantics)... ";

    auto flag = Arc<Mutex<bool>>::make_in_place(false);

    {
        auto handle = thread::spawn(
            [](Arc<Mutex<bool>> flag) {
                std::this_thread::sleep_for(std::chrono::milliseconds(50));
                auto guard = flag->lock();
                *guard = true;
            },
            flag
        );
        // handle dropped here without join() - thread detaches
    }

    // Thread should still be running (detached)
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    auto guard = flag->lock();
    assert(*guard == true);  // Thread completed

    std::cout << "PASSED\n";
}

void test_explicit_detach() {
    std::cout << "Test: Explicit detach... ";

    auto flag = Arc<Mutex<bool>>::make_in_place(false);

    auto handle = thread::spawn(
        [](Arc<Mutex<bool>> flag) {
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
            auto guard = flag->lock();
            *guard = true;
        },
        flag
    );

    handle.detach();  // Explicitly detach

    // Wait for thread to finish
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    auto guard = flag->lock();
    assert(*guard == true);

    std::cout << "PASSED\n";
}

void test_is_finished() {
    std::cout << "Test: is_finished()... ";

    auto handle = thread::spawn([]() {
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
        return 42;
    });

    // Should not be finished immediately
    assert(!handle.is_finished());

    // Wait a bit
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Should be finished now
    assert(handle.is_finished());

    int result = handle.join();
    assert(result == 42);

    std::cout << "PASSED\n";
}

void test_exception_propagation() {
    std::cout << "Test: Exception propagation... ";

    auto handle = thread::spawn([]() -> int {
        throw std::runtime_error("Test exception");
        return 42;
    });

    bool caught = false;
    try {
        int result = handle.join();
        assert(false);  // Should not reach here
    } catch (const std::runtime_error& e) {
        caught = true;
        assert(std::string(e.what()) == "Test exception");
    }

    assert(caught);

    std::cout << "PASSED\n";
}

void test_multiple_threads() {
    std::cout << "Test: Multiple threads... ";

    auto counter = Arc<Mutex<int>>::make_in_place(0);
    Vec<thread::JoinHandle<void>> handles;

    for (int i = 0; i < 5; ++i) {
        auto handle = thread::spawn(
            [](Arc<Mutex<int>> counter, int value) {
                auto guard = counter->lock();
                *guard += value;
            },
            counter,
            i
        );
        handles.push(std::move(handle));
    }

    for (auto& h : handles) {
        h.join();
    }

    auto result = counter->lock();
    assert(*result == 0 + 1 + 2 + 3 + 4);  // 10

    std::cout << "PASSED\n";
}

void test_scoped_threads() {
    std::cout << "Test: Scoped threads... ";

    Vec<int> data = {1, 2, 3, 4, 5};
    int sum = 0;

    thread::scope([&](auto& s) {
        for (size_t i = 0; i < data.size(); ++i) {
            s.spawn([&data, &sum, i]() {
                sum += data[i];
            });
        }
        // All threads joined automatically here
    });

    assert(sum == 15);

    std::cout << "PASSED\n";
}

void test_scoped_with_arc_mutex() {
    std::cout << "Test: Scoped threads with Arc<Mutex<T>>... ";

    auto data = Arc<Mutex<Vec<int>>>::make_in_place(Vec<int>());

    thread::scope([&data](auto& s) {
        for (int i = 0; i < 10; ++i) {
            s.spawn([&data, i]() {  // Captures &Arc<Mutex<...>>
                auto guard = data->lock();
                guard->push(i);
            });
        }
    });

    auto guard = data->lock();
    assert(guard->size() == 10);

    std::cout << "PASSED\n";
}

void test_move_handle() {
    std::cout << "Test: Move JoinHandle... ";

    auto handle1 = thread::spawn([]() { return 42; });

    // Move to vector
    Vec<thread::JoinHandle<int>> handles;
    handles.push(std::move(handle1));

    // Join from vector
    int result = handles[0].join();
    assert(result == 42);

    std::cout << "PASSED\n";
}

// This should NOT compile (commented out)
// void test_rc_not_send() {
//     Rc<int> rc = make_rc(42);
//     // ERROR: Rc<int> is not Send
//     auto handle = thread::spawn([](Rc<int> r) { return *r; }, rc);
// }

int main() {
    std::cout << "Running thread tests...\n\n";

    test_basic_spawn();
    test_spawn_with_arguments();
    test_spawn_void();
    test_detach_on_drop();
    test_explicit_detach();
    test_is_finished();
    test_exception_propagation();
    test_multiple_threads();
    test_scoped_threads();
    test_scoped_with_arc_mutex();
    test_move_handle();

    std::cout << "\nAll thread tests passed!\n";
    return 0;
}
