// Runtime surface Lion's reactor and executor lower to (--crate-graph):
// io::Result over a type without a default constructor, LocalKey::try_with,
// RefCell::try_borrow{,_mut}, OnceLock, Option::map with a unit callback,
// Option<T&>::is_some_and, the raw-pointer dispatch helpers codegen emits
// for closure parameters of unknown type, Pin projections, Poll<()>'s two
// spellings, Mutex::into_inner and PoisonError::into_inner as a value,
// env::var, thread::available_parallelism and JoinHandle::thread;
// io::Error::from(kind) / io::Error::other, io::Result's consuming unwrap,
// Arc::ptr_eq, Pin::as_mut / as_ref, flatten over owned containers without
// `.iter()`, and a drained VecDeque converted through `rusty_from_impl`;
// a Task awaiting a hand-written pollable, `Task::Output`, Waker::will_wake.

#include "../include/rusty/arc.hpp"
#include "../include/rusty/io.hpp"
#include "../include/rusty/local_key.hpp"
#include "../include/rusty/once.hpp"
#include "../include/rusty/option.hpp"
#include "../include/rusty/ptr.hpp"
#include "../include/rusty/refcell.hpp"
#include "../include/rusty/async.hpp"
#include "../include/rusty/mutex.hpp"
#include "../include/rusty/pin.hpp"
#include "../include/rusty/process.hpp"
#include "../include/rusty/slice.hpp"
#include "../include/rusty/thread.hpp"
#include "../include/rusty/vecdeque.hpp"

#include <cassert>
#include <cstdint>
#include <cstdio>
#include <memory>
#include <string>
#include <vector>
#include <tuple>

namespace {

// A handle type: no default constructor (an fd wrapper).
struct Fd {
    explicit Fd(int raw) : raw(raw) {}
    int raw;
    bool operator==(const Fd&) const = default;
};

void test_io_result_without_default_constructor() {
    auto ok = rusty::io::Result<Fd>::ok(Fd(3));
    auto err = rusty::io::Result<Fd>::err(rusty::io::Error::new_(rusty::io::ErrorKind::Other, "boom"));
    assert(ok.is_ok());
    assert(ok.unwrap().raw == 3);
    assert(ok.expect("open").raw == 3);
    assert(err.is_err());
    assert(err.unwrap_err().to_string() == "boom");
    assert(err.unwrap_or(Fd(9)).raw == 9);
    assert(ok.ok().is_some());
    assert(err.ok().is_none());
    assert(err.err().is_some());
    auto mapped = ok.map([](Fd fd) { return fd.raw + 1; });
    assert(mapped.unwrap() == 4);
}

void test_local_key_try_with() {
    rusty::LocalKey<int> key(41);
    auto r = key.try_with([](int& v) { return v + 1; });
    assert(r.is_ok());
    assert(r.unwrap() == 42);
    int seen = 0;
    auto unit = key.try_with([&](int& v) { seen = v; });
    assert(unit.is_ok());
    assert(seen == 41);
}

void test_refcell_try_borrow() {
    rusty::RefCell<int> cell(5);
    {
        auto shared = cell.try_borrow();
        assert(shared.is_ok());
        assert(cell.try_borrow().is_ok());        // many readers
        assert(cell.try_borrow_mut().is_err());   // no writer while read
    }
    {
        auto excl = cell.try_borrow_mut();
        assert(excl.is_ok());
        assert(cell.try_borrow().is_err());       // no reader while written
        assert(cell.try_borrow_mut().is_err());
    }
    assert(cell.try_borrow_mut().is_ok());
}

rusty::OnceLock<int> GLOBAL = rusty::OnceLock<int>::new_();

void test_once_lock() {
    assert(GLOBAL.get().is_none());
    int calls = 0;
    assert(GLOBAL.get_or_init([&] { ++calls; return 7; }) == 7);
    assert(GLOBAL.get_or_init([&] { ++calls; return 8; }) == 7);
    assert(calls == 1);
    assert(GLOBAL.get().is_some());
    assert(GLOBAL.get().unwrap() == 7);
}

void test_option_map_unit_and_is_some_and() {
    int total = 0;
    rusty::Option<int> some(4);
    auto unit = some.map([&](int v) { total += v; });
    static_assert(std::is_same_v<decltype(unit), rusty::Option<std::tuple<>>>);
    assert(unit.is_some());
    assert(total == 4);
    rusty::Option<int> none(rusty::None);
    assert(none.map([&](int v) { total += v; }).is_none());
    assert(total == 4);

    int x = 6;
    rusty::Option<int&> by_ref(x);
    assert(by_ref.is_some_and([](int& v) { return v == 6; }));
    assert(!by_ref.is_some_and([](int& v) { return v == 7; }));
    by_ref.map([](int& v) { v = 9; });
    assert(x == 9);
}

void test_pointer_dispatch() {
    int value = 3;
    int* p = &value;
    int* null = nullptr;
    // `p.as_mut()` on an untyped closure parameter holding a raw pointer.
    assert(rusty::ptr::as_mut_dispatch(p).is_some());
    assert(rusty::ptr::as_mut_dispatch(null).is_none());
    assert(rusty::ptr::as_ref_dispatch(p).is_some());
    // ... or holding an Option: the receiver's own as_mut().
    rusty::Option<int> opt(1);
    assert(rusty::ptr::as_mut_dispatch(opt).is_some());
    // `x as usize` for a pointer (its address) or an integer.
    assert(rusty::ptr::detail::integer_or_address_cast<std::size_t>(p) ==
           reinterpret_cast<std::uintptr_t>(p));
    assert(rusty::ptr::detail::integer_or_address_cast<std::size_t>(std::int32_t{12}) == 12u);
}

struct Inner {
    int v = 1;
    int poll() { return ++v; }
};

struct Projected {
    Inner _0;
    // `self.map_unchecked_mut(|s| &mut s.0).poll(cx)` in a
    // `self: Pin<&mut Self>` method.
    int poll() { return rusty::pin_place::map_unchecked_mut(this, [](auto&& s) { return &s._0; }).poll(); }
    Projected& get() { return rusty::pin_place::get_unchecked_mut(this); }
};

void test_pin_place() {
    Projected p;
    assert(p.poll() == 2);
    assert(&p.get() == &p);
    Inner i;
    assert(&rusty::pin_place::get_mut(i) == &i);
}

void test_poll_unit_spellings() {
    rusty::Poll<std::tuple<>> ready = rusty::Poll<void>::ready_with();
    rusty::Poll<std::tuple<>> pending = rusty::Poll<void>::pending();
    assert(ready.is_ready());
    assert(pending.is_pending());
}

void test_mutex_into_inner_and_poison() {
    rusty::Mutex<int> m(5);
    // `m.lock().unwrap_or_else(PoisonError::into_inner)`.
    auto guard = m.lock().unwrap_or_else(rusty::sync::poison_into_inner);
    assert(*guard == 5);
    rusty::Mutex<std::string> owned(std::string("payload"));
    assert(owned.into_inner().unwrap_or_else(rusty::sync::poison_into_inner) == "payload");
}

void test_env_var() {
    ::setenv("RUSTY_RUNTIME_SURFACE_TEST", "42", 1);
    auto set = rusty::env::var("RUSTY_RUNTIME_SURFACE_TEST");
    assert(set.is_ok());
    assert(std::string_view(set.unwrap()) == "42");
    ::unsetenv("RUSTY_RUNTIME_SURFACE_TEST");
    assert(rusty::env::var("RUSTY_RUNTIME_SURFACE_TEST").is_err());
}

void test_thread_handle_and_parallelism() {
    assert(rusty::thread::available_parallelism().map([](auto n) { return n.get(); }).unwrap_or(0) >= 1);
    std::atomic<bool> go{false};
    auto handle = rusty::thread::spawn([&go]() {
        // Parks until the spawner unparks it through JoinHandle::thread().
        while (!go.load()) {
            rusty::thread::park();
        }
        return 7;
    });
    rusty::thread::Thread t = handle.thread().clone();
    assert(t.id() == handle.thread().id());
    go.store(true);
    t.unpark();
    assert(handle.join().unwrap() == 7);
}

// `io::Error::from(ErrorKind::WouldBlock)` / `io::Error::other(e)`.
void test_io_error_from_kind_and_other() {
    auto e = rusty::io::Error::from(rusty::io::ErrorKind::WouldBlock);
    assert(e.kind() == rusty::io::ErrorKind::WouldBlock);
    assert(e.to_string() == "operation would block");
    auto o = rusty::io::Error::other("boom");
    assert(o.kind() == rusty::io::ErrorKind::Other);
    assert(o.to_string() == "boom");
    struct Displayed {
        std::string to_string() const { return "displayed"; }
    };
    assert(rusty::io::Error::other(Displayed{}).to_string() == "displayed");
}

// `unwrap(self)` / `expect(self, ..)` consume the io::Result: the value moves
// out (a move-only one too), where a reference into a temporary dangled.
void test_io_result_unwrap_moves_out() {
    auto boxed = rusty::io::Result<std::unique_ptr<int>>::ok(std::make_unique<int>(5));
    std::unique_ptr<int> p = boxed.unwrap();
    assert(p && *p == 5);
    std::unique_ptr<int> q =
        rusty::io::Result<std::unique_ptr<int>>::ok(std::make_unique<int>(6)).expect("six");
    assert(q && *q == 6);
}

// `Arc::ptr_eq(&a, &b)`: same allocation, whether given values or pointers.
void test_arc_ptr_eq() {
    auto a = rusty::Arc<int>::make(1);
    auto b = a.clone();
    auto c = rusty::Arc<int>::make(1);
    assert(rusty::Arc<int>::ptr_eq(a, b));
    assert(!rusty::Arc<int>::ptr_eq(a, c));
    assert(rusty::Arc<int>::ptr_eq(&a, b));
    assert(!rusty::Arc<int>::ptr_eq(a, &c));
    assert(rusty::Arc<int>::ptr_eq(&a, &b));
}

// `pinned.as_mut()` / `as_ref()` reborrow the pinned place itself.
void test_pin_as_mut_is_the_place() {
    Inner i;
    assert(&rusty::pin_place::as_mut(i) == &i);
    assert(&rusty::pin_place::as_ref(&i) == &i);
    assert(rusty::pin_place::as_mut(i).poll() == 2);
}

// `batch.into_iter().flatten()` over `Option<VecDeque<T>>`: each inner
// container is an owned temporary (kept alive by the adapter) with no
// `.iter()` — it is iterated through its begin/end range.
void test_flatten_owned_containers_without_iter() {
    rusty::VecDeque<int> d;
    d.push_back(1);
    d.push_back(2);
    d.push_back(3);
    auto batch = rusty::Option<rusty::VecDeque<int>>(std::move(d));
    int sum = 0;
    int count = 0;
    for (auto&& x : rusty::for_in(rusty::flatten(rusty::iter(std::move(batch))))) {
        sum += x;
        ++count;
    }
    assert(count == 3 && sum == 6);
    auto none = rusty::Option<rusty::VecDeque<int>>();
    int seen = 0;
    for (auto&& x : rusty::for_in(rusty::flatten(rusty::iter(std::move(none))))) {
        seen += x;
    }
    assert(seen == 0);
}

// `impl<T> From<VecDeque<T>> for Vec<T>` reached from `.into()`: a
// consumed deque drains front to back into any `from_iter` collection.
struct Collected {
    std::vector<std::unique_ptr<int>> items;
    template <typename I>
    static Collected from_iter(I it) {
        Collected c;
        while (true) {
            auto next = it.next();
            if (next.is_none()) {
                break;
            }
            c.items.push_back(next.unwrap());
        }
        return c;
    }
};

void test_vecdeque_conversion_hook() {
    rusty::VecDeque<std::unique_ptr<int>> d;
    d.push_back(std::make_unique<int>(1));
    d.push_back(std::make_unique<int>(2));
    d.push_front(std::make_unique<int>(0));
    Collected c = rusty_from_impl(std::type_identity<Collected>{}, std::move(d));
    assert(c.items.size() == 3);
    assert(*c.items[0] == 0 && *c.items[1] == 1 && *c.items[2] == 2);
}

// `handle.await` / `sleep.await` inside an async fn: a pollable (no
// await_ready) is pinned into a Task polled with the awaiting task's
// context, consuming the operand, lvalue or prvalue; Tasks await as before.
struct CountdownFuture {
    int n;
    std::unique_ptr<int> payload;  // move-only, as a JoinHandle is
    rusty::Poll<int> poll(rusty::Context& cx) {
        if (n-- > 0) {
            cx.waker->wake_by_ref();
            return rusty::Poll<int>::pending();
        }
        return rusty::Poll<int>::ready_with(*payload);
    }
};

rusty::Task<int> inner_task() { co_return 1; }
rusty::Task<void> unit_task() { co_return; }

rusty::Task<int> awaiting_task() {
    int a = co_await inner_task();
    CountdownFuture lvalue{2, std::make_unique<int>(40)};
    int b = co_await std::move(lvalue);
    int c = co_await CountdownFuture{1, std::make_unique<int>(100)};
    co_await unit_task();
    co_return a + b + c;
}

void test_task_awaits_a_pollable() {
    static_assert(std::is_same_v<rusty::Task<int>::Output, int>);
    static_assert(std::is_same_v<rusty::Task<void>::Output, std::tuple<>>);
    assert(rusty::block_on(awaiting_task()) == 141);
}

// `Waker::will_wake`: the same Arc-built waker (or a clone) wakes the same
// task; a different one, or a callable-built one, is not known to.
struct CountingWake {
    mutable int woken = 0;
    static void wake(rusty::Arc<CountingWake> self) { self->woken += 1; }
};

void test_waker_will_wake() {
    auto a = rusty::Arc<CountingWake>::make(CountingWake{});
    auto b = rusty::Arc<CountingWake>::make(CountingWake{});
    rusty::Waker wa = rusty::Waker::from_arc(a.clone());
    rusty::Waker wa2 = wa.clone();
    rusty::Waker wb = rusty::Waker::from_arc(b.clone());
    rusty::Waker callable = rusty::Waker::from_callable([]() {});
    assert(wa.will_wake(wa2));
    assert(wa.will_wake(rusty::Waker::from_arc(a.clone())));
    assert(!wa.will_wake(wb));
    assert(!callable.will_wake(callable));
    wa.wake_by_ref();
    assert(a->woken == 1);
}

// A match arm's payload view of an owned scrutinee: mutable where the
// scrutinee is (a `self`-by-value member is then callable on the binding),
// const where it is not, and the scrutinee is never consumed.
struct Guard {
    int calls = 0;
    int consume() { return ++calls; }
};

void test_peek_unwrap_views_an_owned_payload() {
    rusty::io::Result<Guard> owned = rusty::io::Result<Guard>::ok(Guard{});
    auto&& guard = rusty::detail::peek_unwrap(owned);
    static_assert(std::is_same_v<decltype(guard), Guard&>);
    assert(guard.consume() == 1);
    assert(owned.unwrap_mut().calls == 1);
    const rusty::io::Result<Guard> fixed = rusty::io::Result<Guard>::ok(Guard{});
    auto&& viewed = rusty::detail::peek_unwrap(fixed);
    static_assert(std::is_same_v<decltype(viewed), const Guard&>);
    rusty::io::Result<Guard> failed =
        rusty::io::Result<Guard>::err(rusty::io::Error::from(rusty::io::Error::Kind::WouldBlock));
    auto&& error = rusty::detail::peek_unwrap_err(failed);
    static_assert(std::is_same_v<decltype(error), rusty::io::Error&>);
    assert(error.kind() == rusty::io::Error::Kind::WouldBlock);
    assert(failed.is_err());
    rusty::Option<Guard> some = rusty::Option<Guard>(Guard{});
    assert(rusty::detail::peek_unwrap(some).consume() == 1);
    assert(some.is_some() && some.unwrap_mut().calls == 1);
}

}  // namespace

int main() {
    test_io_result_without_default_constructor();
    test_local_key_try_with();
    test_refcell_try_borrow();
    test_once_lock();
    test_option_map_unit_and_is_some_and();
    test_pointer_dispatch();
    test_pin_place();
    test_poll_unit_spellings();
    test_mutex_into_inner_and_poison();
    test_env_var();
    test_thread_handle_and_parallelism();
    test_io_error_from_kind_and_other();
    test_io_result_unwrap_moves_out();
    test_arc_ptr_eq();
    test_pin_as_mut_is_the_place();
    test_flatten_owned_containers_without_iter();
    test_vecdeque_conversion_hook();
    test_task_awaits_a_pollable();
    test_waker_will_wake();
    test_peek_unwrap_views_an_owned_payload();
    std::printf("rusty_runtime_surface_test: all passed\n");
    return 0;
}
