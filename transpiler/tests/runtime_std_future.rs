use std::path::Path;
use std::process::Command;

fn check_runtime(sanitize: bool) {
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "c++".into());
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("future.cpp");
    let binary = temp.path().join("future");
    std::fs::write(&source, SOURCE).unwrap();
    let mut command = Command::new(compiler);
    command.args(["-std=c++20", "-pthread", "-g", "-O1", "-I"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&source).arg("-o").arg(&binary);
    if sanitize {
        command.args(["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]);
    }
    let compiled = command.output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let output = Command::new(binary)
        .env("ASAN_OPTIONS", "detect_stack_use_after_return=1:detect_leaks=1")
        .env("UBSAN_OPTIONS", "halt_on_error=1")
        .output().unwrap();
    assert!(output.status.success(), "status {}\n{}", output.status, String::from_utf8_lossy(&output.stderr));
}

#[test]
fn standard_future_runtime_owns_pending_payloads_and_wake_contexts() {
    check_runtime(false);
}

#[test]
fn standard_future_runtime_lifetimes_under_sanitizers() {
    check_runtime(true);
}

const SOURCE: &str = r#"
#include <rusty/async.hpp>
#include <rusty/arc.hpp>
#include <cassert>
#include <optional>
#include <memory>

struct Value {
    static inline int live = 0;
    int number;
    Value() = delete;
    explicit Value(int n) : number(n) { ++live; }
    Value(Value&& v) noexcept : number(v.number) { ++live; v.number = -1; }
    Value& operator=(Value&&) = delete;
    Value(const Value&) = delete;
    ~Value() { --live; }
};

rusty::Waker retained;
struct Suspend {
    rusty::Context* saved = nullptr;
    bool await_ready() const { return false; }
    template<class P> void await_suspend(std::coroutine_handle<P> h) {
        saved = h.promise().current_ctx;
        retained = saved->waker->clone();
        saved->waker->wake_by_ref();
    }
    void await_resume() { saved->waker->wake_by_ref(); }
    ~Suspend() { if (saved) saved->waker->wake_by_ref(); }
};

rusty::Task<Value> child() {
    co_await Suspend{};
    co_return Value{41};
}
rusty::Task<Value> parent() {
    auto value = co_await child();
    assert(rusty::current_context() != nullptr);
    co_return Value{value.number + 1};
}
rusty::Task<void> void_child() { co_await Suspend{}; }
rusty::Task<void> void_parent() { co_await void_child(); }

struct ConcreteFuture {
    const ConcreteFuture* first = nullptr;
    rusty::Poll<Value> poll(rusty::Context& cx) {
        if (!first) {
            first = this;
            retained = cx.waker->clone();
            return rusty::Poll<Value>::pending();
        }
        assert(first == this);
        return rusty::Poll<Value>::ready_with(Value{13});
    }
};

struct WakeTarget {
    static inline int consumed = 0;
    static inline int borrowed = 0;
    static inline int destroyed = 0;
    static void wake(rusty::Arc<WakeTarget> owner) {
        assert(owner.strong_count() >= 1);
        ++consumed;
    }
    static void wake_by_ref(const rusty::Arc<WakeTarget>& owner) {
        assert(owner.strong_count() >= 1);
        ++borrowed;
    }
    ~WakeTarget() { ++destroyed; }
};
struct DefaultWake {
    static inline int calls = 0;
    static void wake(rusty::Arc<DefaultWake> owner) {
        assert(owner.strong_count() >= 1);
        ++calls;
    }
};

int main() {
    {
        auto pending = rusty::Poll<Value>::pending();
        assert(Value::live == 0 && pending.is_pending());
        auto ready = rusty::Poll<Value>::ready_with(Value{7});
        assert(Value::live == 1);
        ready.as_mut().unwrap().number = 8;
        pending = std::move(ready);
        assert(pending.as_ref().unwrap().number == 8);
        auto value = pending.unwrap();
        assert(value.number == 8 && pending.is_pending());
        pending = rusty::Poll<Value>::pending();
    }
    assert(Value::live == 0);
    int first = 0, second = 0;
    {
        auto task = parent();
        {
            rusty::Waker w{[&] { ++first; }};
            rusty::Context cx{&w};
            assert(task.poll(cx).is_pending());
        }
        assert(first == 1 && Value::live == 0);
        rusty::Waker w{[&] { ++second; }};
        rusty::Context cx{&w};
        auto value = task.poll(cx);
        assert(value.is_ready() && value.value.number == 42);
        assert(first == 1 && second == 2);
        assert(rusty::current_context() == nullptr);
    }
    assert(Value::live == 0);
    retained.wake_by_ref();
    assert(first == 2);
    {
        std::optional<rusty::Task<Value>> task;
        task.emplace(parent());
        {
            rusty::Waker w{[&] { ++first; }};
            rusty::Context cx{&w};
            assert(task->poll(cx).is_pending());
        }
        task.reset(); // Suspended awaiter still borrows its promise-owned context.
        assert(first == 4);
    }
    retained.wake_by_ref();
    assert(first == 5);
    {
        auto task = void_parent();
        rusty::Waker w{[] {}};
        rusty::Context cx{&w};
        assert(task.poll(cx).is_pending());
        assert(task.poll(cx).is_ready());
    }
    {
        auto task = rusty::future::pin<Value>(ConcreteFuture{});
        auto moved = std::move(task);
        rusty::Waker w{[] {}};
        rusty::Context cx{&w};
        assert(moved.poll(cx).is_pending());
        assert(moved.poll(cx).unwrap().number == 13);
        auto identity = rusty::future::pin(child());
        assert(identity.poll(cx).is_pending());
        assert(identity.poll(cx).unwrap().number == 41);
    }
    assert(Value::live == 0);
    {
        auto owner = rusty::Arc<WakeTarget>::make();
        auto waker = rusty::Waker::from_arc(owner.clone());
        assert(owner.strong_count() == 2);
        auto copy = waker.clone();
        assert(owner.strong_count() == 3);
        copy.wake_by_ref();
        assert(owner.strong_count() == 3 && WakeTarget::borrowed == 1);
        copy.wake();
        assert(owner.strong_count() == 2 && WakeTarget::consumed == 1);
        waker.wake_by_ref();
        assert(WakeTarget::borrowed == 2);
    }
    assert(WakeTarget::destroyed == 1);
    {
        auto owner = rusty::Arc<DefaultWake>::make();
        auto waker = rusty::Waker::from_arc(owner.clone());
        waker.wake_by_ref();
        assert(owner.strong_count() == 2 && DefaultWake::calls == 1);
        waker.wake();
        assert(owner.strong_count() == 1 && DefaultWake::calls == 2);
    }
}
"#;
