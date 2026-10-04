#pragma once
// rusty::LocalKey<T> — the C++ lowering target for Rust's `thread_local!`.
//
// The transpiler emits
//     thread_local! { static X: T = init; }
// as
//     thread_local rusty::LocalKey<T> X{init};
// and leaves access sites alone: `X.with(|v| ...)` lowers through the
// ordinary method-call path to `X.with([&](auto&& v) { ... })`, which
// resolves against `with` below.  Mirroring std::thread::LocalKey:
//
//  * per-thread storage: the `thread_local` on the emitted variable gives
//    every thread its own LocalKey instance, dynamically initialized on that
//    thread's first ODR-use and destroyed at thread exit — the same lazy
//    init / destructor semantics Rust's LocalKey provides;
//  * closure-only access: like Rust, there is no way to get at the value
//    except through `with`, so a reference cannot accidentally outlive the
//    thread (C++ cannot enforce the no-escape part, but the shape steers
//    callers the same way).
#include <type_traits>
#include <utility>

#include "result.hpp"

namespace rusty {

// std::thread::AccessError: `try_with` on a key whose value is being (or has
// been) destroyed by thread exit.
struct AccessError {
    bool operator==(const AccessError&) const = default;
};

template <typename T>
class LocalKey {
public:
    template <typename... Args>
    explicit LocalKey(Args&&... args) : value_(std::forward<Args>(args)...) {}

    LocalKey(const LocalKey&) = delete;
    LocalKey& operator=(const LocalKey&) = delete;

    // The key's value dies with the thread. From the start of that
    // destruction on, `try_with` reports AccessError instead of handing out
    // the value (Rust's contract: code that may run during thread-local
    // teardown, such as another key's destructor, uses try_with). Like Rust's
    // own thread-local keys, the state lives in the key's storage, which the
    // thread keeps until all of its thread-locals are destroyed.
    ~LocalKey() { alive_ = false; }

    template <typename F>
    decltype(auto) with(F&& f) {
        return std::forward<F>(f)(value_);
    }

    template <typename F>
    decltype(auto) with(F&& f) const {
        return std::forward<F>(f)(value_);
    }

    template <typename F>
    auto try_with(F&& f) -> Result<std::remove_cvref_t<std::invoke_result_t<F&&, T&>>, AccessError> {
        using R = std::remove_cvref_t<std::invoke_result_t<F&&, T&>>;
        if (!alive_) {
            return Result<R, AccessError>::Err(AccessError{});
        }
        if constexpr (std::is_void_v<R>) {
            std::forward<F>(f)(value_);
            return Result<R, AccessError>::Ok();
        } else {
            return Result<R, AccessError>::Ok(std::forward<F>(f)(value_));
        }
    }

private:
    T value_;
    bool alive_ = true;
};

}  // namespace rusty
