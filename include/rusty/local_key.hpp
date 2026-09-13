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
#include <utility>

namespace rusty {

template <typename T>
class LocalKey {
public:
    template <typename... Args>
    explicit LocalKey(Args&&... args) : value_(std::forward<Args>(args)...) {}

    LocalKey(const LocalKey&) = delete;
    LocalKey& operator=(const LocalKey&) = delete;

    template <typename F>
    decltype(auto) with(F&& f) {
        return std::forward<F>(f)(value_);
    }

    template <typename F>
    decltype(auto) with(F&& f) const {
        return std::forward<F>(f)(value_);
    }

private:
    T value_;
};

}  // namespace rusty
