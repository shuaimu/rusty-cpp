// Demonstration of Rusty synchronization primitives
// Showcases Mutex, RwLock, Condvar, Barrier, and Once

#include <iostream>
#include <thread>
#include <vector>
#include <chrono>
#include <rusty/rusty.hpp>

using namespace std::chrono_literals;

// Example 1: Mutex with MutexGuard (RAII)
void mutex_example() {
    std::cout << "\n=== Mutex Example ===" << std::endl;

    rusty::Mutex<int> counter(0);
    std::vector<std::thread> threads;

    // Spawn 10 threads that increment the counter
    for (int i = 0; i < 10; ++i) {
        threads.emplace_back([&counter]() {
            for (int j = 0; j < 1000; ++j) {
                auto guard = counter.lock();
                *guard += 1;
                // Lock automatically released when guard goes out of scope
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    auto final_value = counter.lock();
    std::cout << "Final counter value: " << *final_value << " (expected 10000)" << std::endl;
}

// Example 2: RwLock with ReadGuard and WriteGuard
void rwlock_example() {
    std::cout << "\n=== RwLock Example ===" << std::endl;

    rusty::RwLock<std::vector<int>> shared_data(std::vector<int>{1, 2, 3, 4, 5});
    std::vector<std::thread> threads;

    // Spawn 5 reader threads
    for (int i = 0; i < 5; ++i) {
        threads.emplace_back([&shared_data, i]() {
            auto read_guard = shared_data.read();
            std::cout << "Reader " << i << " sees " << read_guard->size() << " elements" << std::endl;
            std::this_thread::sleep_for(10ms);
        });
    }

    // Spawn 1 writer thread
    threads.emplace_back([&shared_data]() {
        std::this_thread::sleep_for(50ms);  // Let readers go first
        auto write_guard = shared_data.write();
        write_guard->push_back(6);
        std::cout << "Writer added element, now " << write_guard->size() << " elements" << std::endl;
    });

    for (auto& t : threads) {
        t.join();
    }
}

// Example 3: Condvar for producer-consumer
void condvar_example() {
    std::cout << "\n=== Condvar Example (Producer-Consumer) ===" << std::endl;

    std::mutex mtx;
    rusty::Condvar cv;
    std::vector<int> queue;
    bool done = false;

    // Consumer thread
    std::thread consumer([&]() {
        while (true) {
            std::unique_lock lock(mtx);
            cv.wait(lock, [&]{ return !queue.empty() || done; });

            if (!queue.empty()) {
                int item = queue.back();
                queue.pop_back();
                lock.unlock();
                std::cout << "Consumer: consumed " << item << std::endl;
            } else if (done) {
                std::cout << "Consumer: producer is done, exiting" << std::endl;
                break;
            }
        }
    });

    // Producer thread
    std::thread producer([&]() {
        for (int i = 1; i <= 5; ++i) {
            std::this_thread::sleep_for(100ms);
            {
                std::unique_lock lock(mtx);
                queue.push_back(i);
                std::cout << "Producer: produced " << i << std::endl;
            }
            cv.notify_one();
        }

        {
            std::unique_lock lock(mtx);
            done = true;
        }
        cv.notify_one();
    });

    producer.join();
    consumer.join();
}

// Example 4: Barrier for thread synchronization
void barrier_example() {
    std::cout << "\n=== Barrier Example ===" << std::endl;

    const int NUM_THREADS = 4;
    rusty::Barrier barrier(NUM_THREADS);
    std::vector<std::thread> threads;

    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([&barrier, i]() {
            std::cout << "Thread " << i << " working..." << std::endl;
            std::this_thread::sleep_for(std::chrono::milliseconds(100 * (i + 1)));

            std::cout << "Thread " << i << " reached barrier" << std::endl;
            auto result = barrier.wait();

            if (result.is_leader()) {
                std::cout << "Thread " << i << " is the LEADER!" << std::endl;
            }

            std::cout << "Thread " << i << " passed barrier" << std::endl;
        });
    }

    for (auto& t : threads) {
        t.join();
    }
}

// Example 5: Once for one-time initialization
static rusty::Once INIT;
static int* global_resource = nullptr;

void initialize_resource() {
    std::cout << "Initializing global resource..." << std::endl;
    global_resource = new int(42);
}

void once_example() {
    std::cout << "\n=== Once Example ===" << std::endl;

    std::vector<std::thread> threads;

    // Spawn 5 threads that all try to initialize
    for (int i = 0; i < 5; ++i) {
        threads.emplace_back([i]() {
            std::cout << "Thread " << i << " calling initialize..." << std::endl;
            INIT.call_once(initialize_resource);
            std::cout << "Thread " << i << " sees resource value: " << *global_resource << std::endl;
        });
    }

    for (auto& t : threads) {
        t.join();
    }

    delete global_resource;
}

// Example 6: OnceCell for lazy initialization
void once_cell_example() {
    std::cout << "\n=== OnceCell Example ===" << std::endl;

    static rusty::OnceCell<std::string> CONFIG;

    std::vector<std::thread> threads;

    for (int i = 0; i < 5; ++i) {
        threads.emplace_back([i]() {
            // Try to set the config (only first one succeeds)
            bool success = CONFIG.set("config-value-" + std::to_string(i));
            if (success) {
                std::cout << "Thread " << i << " successfully initialized CONFIG" << std::endl;
            } else {
                std::cout << "Thread " << i << " found CONFIG already initialized" << std::endl;
            }

            // Everyone can read the value
            const std::string* value = CONFIG.get();
            if (value) {
                std::cout << "Thread " << i << " reads: " << *value << std::endl;
            }
        });
    }

    for (auto& t : threads) {
        t.join();
    }
}

int main() {
    std::cout << "Rusty Synchronization Primitives Demo" << std::endl;
    std::cout << "======================================" << std::endl;

    mutex_example();
    rwlock_example();
    condvar_example();
    barrier_example();
    once_example();
    once_cell_example();

    std::cout << "\n=== All examples completed ===" << std::endl;
    return 0;
}
