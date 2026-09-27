// Runtime surface Lion's reactor and executor lower to (--crate-graph):
// io::Result over a type without a default constructor, LocalKey::try_with,
// RefCell::try_borrow{,_mut}, OnceLock, Option::map with a unit callback,
// Option<T&>::is_some_and, the raw-pointer dispatch helpers codegen emits
// for closure parameters of unknown type, Pin projections, Poll<()>'s two
// spellings, Mutex::into_inner and PoisonError::into_inner as a value,
// env::var, thread::available_parallelism and JoinHandle::thread.

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
#include "../include/rusty/thread.hpp"

#include <cassert>
#include <cstdint>
#include <cstdio>
#include <string>
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
    std::printf("rusty_runtime_surface_test: all passed\n");
    return 0;
}
