#pragma once

#include "config.hpp"

#include <chrono>
#include <cstddef>
#include <cstring>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <shared_mutex>
#include <stdexcept>
#include <thread>
#include <type_traits>
#include <utility>

#if defined(RUSTY_PLATFORM_BACKEND_POSIX)
#  include <errno.h>
#  include <pthread.h>
#  include <sched.h>
#  include <time.h>
#endif

namespace rusty::platform::threading {

#if !defined(RUSTY_PLATFORM_BACKEND_POSIX)

using mutex = std::mutex;
using shared_mutex = std::shared_mutex;

using try_to_lock_t = std::try_to_lock_t;
inline constexpr try_to_lock_t try_to_lock = std::try_to_lock;

using once_flag = std::once_flag;
template<typename F, typename... Args>
inline void call_once(once_flag& flag, F&& f, Args&&... args) {
    std::call_once(flag, std::forward<F>(f), std::forward<Args>(args)...);
}

template<typename M>
using unique_lock = std::unique_lock<M>;

template<typename M>
using shared_lock = std::shared_lock<M>;

template<typename M>
using lock_guard = std::lock_guard<M>;

template<typename... Ms>
using scoped_lock = std::scoped_lock<Ms...>;

using condition_variable = std::condition_variable;
using cv_status = std::cv_status;

using thread_id = std::thread::id;

class thread {
public:
    thread() noexcept = default;

    template<typename F, typename... Args>
    explicit thread(F&& f, Args&&... args)
        : inner_(std::forward<F>(f), std::forward<Args>(args)...) {}

    ~thread() = default;

    thread(const thread&) = delete;
    thread& operator=(const thread&) = delete;
    thread(thread&&) noexcept = default;
    thread& operator=(thread&&) noexcept = default;

    bool joinable() const noexcept {
        return inner_.joinable();
    }

    void join() {
        inner_.join();
    }

    void detach() {
        inner_.detach();
    }

    thread_id get_id() const noexcept {
        return inner_.get_id();
    }

private:
    std::thread inner_{};
};

inline thread_id current_thread_id() {
    return std::this_thread::get_id();
}

inline bool thread_id_equal(thread_id lhs, thread_id rhs) {
    return lhs == rhs;
}

inline bool thread_id_less(thread_id lhs, thread_id rhs) {
    return lhs < rhs;
}

inline std::size_t thread_id_hash(thread_id id) {
    return std::hash<thread_id>{}(id);
}

inline void yield() {
    std::this_thread::yield();
}

template<typename Rep, typename Period>
inline void sleep_for(const std::chrono::duration<Rep, Period>& duration) {
    std::this_thread::sleep_for(duration);
}

#else

struct try_to_lock_t {};
inline constexpr try_to_lock_t try_to_lock{};

class mutex {
public:
    mutex() {
        pthread_mutex_init(&native_, nullptr);
    }

    ~mutex() {
        pthread_mutex_destroy(&native_);
    }

    mutex(const mutex&) = delete;
    mutex& operator=(const mutex&) = delete;

    mutex(mutex&&) = delete;
    mutex& operator=(mutex&&) = delete;

    void lock() {
        pthread_mutex_lock(&native_);
    }

    bool try_lock() {
        return pthread_mutex_trylock(&native_) == 0;
    }

    void unlock() {
        pthread_mutex_unlock(&native_);
    }

    pthread_mutex_t* native_handle() {
        return &native_;
    }

private:
    pthread_mutex_t native_{};
};

class shared_mutex {
public:
    shared_mutex() {
        pthread_rwlock_init(&native_, nullptr);
    }

    ~shared_mutex() {
        pthread_rwlock_destroy(&native_);
    }

    shared_mutex(const shared_mutex&) = delete;
    shared_mutex& operator=(const shared_mutex&) = delete;

    shared_mutex(shared_mutex&&) = delete;
    shared_mutex& operator=(shared_mutex&&) = delete;

    void lock() {
        pthread_rwlock_wrlock(&native_);
    }

    bool try_lock() {
        return pthread_rwlock_trywrlock(&native_) == 0;
    }

    void unlock() {
        pthread_rwlock_unlock(&native_);
    }

    void lock_shared() {
        pthread_rwlock_rdlock(&native_);
    }

    bool try_lock_shared() {
        return pthread_rwlock_tryrdlock(&native_) == 0;
    }

    void unlock_shared() {
        pthread_rwlock_unlock(&native_);
    }

private:
    pthread_rwlock_t native_{};
};

class once_flag {
public:
    once_flag() = default;
    once_flag(const once_flag&) = delete;
    once_flag& operator=(const once_flag&) = delete;

private:
    mutex guard_;
    bool called_ = false;
    template<typename F, typename... Args>
    friend void call_once(once_flag& flag, F&& f, Args&&... args);
};

template<typename F, typename... Args>
inline void call_once(once_flag& flag, F&& f, Args&&... args) {
    flag.guard_.lock();
    if (flag.called_) {
        flag.guard_.unlock();
        return;
    }
    try {
        std::invoke(std::forward<F>(f), std::forward<Args>(args)...);
        flag.called_ = true;
        flag.guard_.unlock();
    } catch (...) {
        flag.guard_.unlock();
        throw;
    }
}

template<typename M>
class unique_lock {
public:
    unique_lock() noexcept = default;

    explicit unique_lock(M& m) : mtx_(&m), owns_(false) {
        lock();
    }

    unique_lock(M& m, try_to_lock_t) : mtx_(&m), owns_(m.try_lock()) {}

    ~unique_lock() {
        if (owns_ && mtx_ != nullptr) {
            mtx_->unlock();
        }
    }

    unique_lock(const unique_lock&) = delete;
    unique_lock& operator=(const unique_lock&) = delete;

    unique_lock(unique_lock&& other) noexcept
        : mtx_(other.mtx_), owns_(other.owns_) {
        other.mtx_ = nullptr;
        other.owns_ = false;
    }

    unique_lock& operator=(unique_lock&& other) noexcept {
        if (this == &other) {
            return *this;
        }
        if (owns_ && mtx_ != nullptr) {
            mtx_->unlock();
        }
        mtx_ = other.mtx_;
        owns_ = other.owns_;
        other.mtx_ = nullptr;
        other.owns_ = false;
        return *this;
    }

    void lock() {
        if (mtx_ != nullptr && !owns_) {
            mtx_->lock();
            owns_ = true;
        }
    }

    bool try_lock() {
        if (mtx_ == nullptr || owns_) {
            return false;
        }
        owns_ = mtx_->try_lock();
        return owns_;
    }

    void unlock() {
        if (mtx_ != nullptr && owns_) {
            mtx_->unlock();
            owns_ = false;
        }
    }

    bool owns_lock() const noexcept {
        return owns_;
    }

    explicit operator bool() const noexcept {
        return owns_;
    }

    M* mutex() const noexcept {
        return mtx_;
    }

private:
    M* mtx_ = nullptr;
    bool owns_ = false;
};

template<typename M>
class shared_lock {
public:
    shared_lock() noexcept = default;

    explicit shared_lock(M& m) : mtx_(&m), owns_(false) {
        lock();
    }

    shared_lock(M& m, try_to_lock_t) : mtx_(&m), owns_(m.try_lock_shared()) {}

    ~shared_lock() {
        if (owns_ && mtx_ != nullptr) {
            mtx_->unlock_shared();
        }
    }

    shared_lock(const shared_lock&) = delete;
    shared_lock& operator=(const shared_lock&) = delete;

    shared_lock(shared_lock&& other) noexcept
        : mtx_(other.mtx_), owns_(other.owns_) {
        other.mtx_ = nullptr;
        other.owns_ = false;
    }

    shared_lock& operator=(shared_lock&& other) noexcept {
        if (this == &other) {
            return *this;
        }
        if (owns_ && mtx_ != nullptr) {
            mtx_->unlock_shared();
        }
        mtx_ = other.mtx_;
        owns_ = other.owns_;
        other.mtx_ = nullptr;
        other.owns_ = false;
        return *this;
    }

    void lock() {
        if (mtx_ != nullptr && !owns_) {
            mtx_->lock_shared();
            owns_ = true;
        }
    }

    bool try_lock() {
        if (mtx_ == nullptr || owns_) {
            return false;
        }
        owns_ = mtx_->try_lock_shared();
        return owns_;
    }

    void unlock() {
        if (mtx_ != nullptr && owns_) {
            mtx_->unlock_shared();
            owns_ = false;
        }
    }

    bool owns_lock() const noexcept {
        return owns_;
    }

private:
    M* mtx_ = nullptr;
    bool owns_ = false;
};

template<typename M>
class lock_guard {
public:
    explicit lock_guard(M& m) : mtx_(m) {
        mtx_.lock();
    }

    ~lock_guard() {
        mtx_.unlock();
    }

    lock_guard(const lock_guard&) = delete;
    lock_guard& operator=(const lock_guard&) = delete;

private:
    M& mtx_;
};

template<typename... Ms>
using scoped_lock = std::scoped_lock<Ms...>;

enum class cv_status {
    no_timeout,
    timeout,
};

using thread_id = pthread_t;

namespace detail {
inline std::size_t hash_bytes(const void* data, std::size_t bytes) noexcept {
    const auto* p = static_cast<const unsigned char*>(data);
    std::size_t h = 1469598103934665603ULL;
    for (std::size_t i = 0; i < bytes; ++i) {
        h ^= static_cast<std::size_t>(p[i]);
        h *= 1099511628211ULL;
    }
    return h;
}
} // namespace detail

class thread {
public:
    thread() noexcept = default;

    template<typename F, typename... Args>
    explicit thread(F&& f, Args&&... args) {
        auto task = std::make_unique<task_impl<std::decay_t<F>, std::decay_t<Args>...>>(
            std::forward<F>(f), std::forward<Args>(args)...);
        const int rc = pthread_create(&id_, nullptr, &thread::entry, task.get());
        if (rc != 0) {
            throw std::runtime_error("pthread_create failed");
        }
        (void)task.release();
        joinable_ = true;
    }

    ~thread() {
        if (joinable_) {
            pthread_detach(id_);
        }
    }

    thread(const thread&) = delete;
    thread& operator=(const thread&) = delete;

    thread(thread&& other) noexcept : id_(other.id_), joinable_(other.joinable_) {
        other.joinable_ = false;
    }

    thread& operator=(thread&& other) noexcept {
        if (this == &other) {
            return *this;
        }
        if (joinable_) {
            pthread_detach(id_);
        }
        id_ = other.id_;
        joinable_ = other.joinable_;
        other.joinable_ = false;
        return *this;
    }

    bool joinable() const noexcept {
        return joinable_;
    }

    void join() {
        if (!joinable_) {
            return;
        }
        pthread_join(id_, nullptr);
        joinable_ = false;
    }

    void detach() {
        if (!joinable_) {
            return;
        }
        pthread_detach(id_);
        joinable_ = false;
    }

    thread_id get_id() const noexcept {
        return id_;
    }

private:
    struct task_base {
        virtual ~task_base() = default;
        virtual void run() = 0;
    };

    template<typename F, typename... Args>
    struct task_impl final : task_base {
        explicit task_impl(F&& f, Args&&... args)
            : fn_(std::bind(std::forward<F>(f), std::forward<Args>(args)...)) {}

        void run() override {
            fn_();
        }

        std::function<void()> fn_;
    };

    static void* entry(void* arg) noexcept {
        std::unique_ptr<task_base> task(static_cast<task_base*>(arg));
        try {
            task->run();
        } catch (...) {
            std::terminate();
        }
        return nullptr;
    }

    pthread_t id_{};
    bool joinable_ = false;
};

namespace detail {
inline timespec realtime_after(std::chrono::nanoseconds delta) {
    if (delta.count() < 0) {
        delta = std::chrono::nanoseconds(0);
    }

    timespec ts{};
    clock_gettime(CLOCK_REALTIME, &ts);

    auto sec = std::chrono::seconds(ts.tv_sec);
    auto nsec = std::chrono::nanoseconds(ts.tv_nsec);
    auto total = sec + nsec + delta;

    auto out_sec = std::chrono::duration_cast<std::chrono::seconds>(total);
    auto out_nsec = std::chrono::duration_cast<std::chrono::nanoseconds>(total - out_sec);

    timespec out{};
    out.tv_sec = static_cast<time_t>(out_sec.count());
    out.tv_nsec = static_cast<long>(out_nsec.count());
    return out;
}
} // namespace detail

class condition_variable {
public:
    condition_variable() {
        pthread_cond_init(&native_, nullptr);
    }

    ~condition_variable() {
        pthread_cond_destroy(&native_);
    }

    condition_variable(const condition_variable&) = delete;
    condition_variable& operator=(const condition_variable&) = delete;

    void notify_one() {
        pthread_cond_signal(&native_);
    }

    void notify_all() {
        pthread_cond_broadcast(&native_);
    }

    void wait(unique_lock<mutex>& lock) {
        if (!lock.owns_lock()) {
            lock.lock();
        }
        pthread_cond_wait(&native_, lock.mutex()->native_handle());
    }

    template<typename Predicate>
    void wait(unique_lock<mutex>& lock, Predicate pred) {
        while (!pred()) {
            wait(lock);
        }
    }

    template<typename Rep, typename Period>
    cv_status wait_for(
        unique_lock<mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration) {
        if (!lock.owns_lock()) {
            lock.lock();
        }
        const auto abs = detail::realtime_after(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
        const int rc = pthread_cond_timedwait(&native_, lock.mutex()->native_handle(), &abs);
        if (rc == ETIMEDOUT) {
            return cv_status::timeout;
        }
        return cv_status::no_timeout;
    }

    template<typename Rep, typename Period, typename Predicate>
    bool wait_for(
        unique_lock<mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred) {
        auto deadline = std::chrono::steady_clock::now() + duration;
        while (!pred()) {
            auto now = std::chrono::steady_clock::now();
            if (now >= deadline) {
                return pred();
            }
            auto remaining = deadline - now;
            if (wait_for(lock, remaining) == cv_status::timeout && !pred()) {
                return false;
            }
        }
        return true;
    }

    template<typename Clock, typename Duration>
    cv_status wait_until(
        unique_lock<mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time) {
        auto now = Clock::now();
        if (now >= timeout_time) {
            return cv_status::timeout;
        }
        return wait_for(lock, timeout_time - now);
    }

    template<typename Clock, typename Duration, typename Predicate>
    bool wait_until(
        unique_lock<mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time,
        Predicate pred) {
        while (!pred()) {
            if (wait_until(lock, timeout_time) == cv_status::timeout && !pred()) {
                return false;
            }
        }
        return true;
    }

private:
    pthread_cond_t native_{};
};

inline thread_id current_thread_id() {
    return pthread_self();
}

inline bool thread_id_equal(thread_id lhs, thread_id rhs) {
    return pthread_equal(lhs, rhs) != 0;
}

inline bool thread_id_less(thread_id lhs, thread_id rhs) {
    return detail::hash_bytes(&lhs, sizeof(lhs)) < detail::hash_bytes(&rhs, sizeof(rhs));
}

inline std::size_t thread_id_hash(thread_id id) {
    return detail::hash_bytes(&id, sizeof(id));
}

inline void yield() {
    sched_yield();
}

template<typename Rep, typename Period>
inline void sleep_for(const std::chrono::duration<Rep, Period>& duration) {
    auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(duration);
    if (ns.count() <= 0) {
        return;
    }

    timespec req{};
    req.tv_sec = static_cast<time_t>(
        std::chrono::duration_cast<std::chrono::seconds>(ns).count());
    req.tv_nsec = static_cast<long>(
        (ns - std::chrono::seconds(req.tv_sec)).count());

    while (nanosleep(&req, &req) == -1 && errno == EINTR) {
        // Retry until sleep completes.
    }
}

#endif

} // namespace rusty::platform::threading
