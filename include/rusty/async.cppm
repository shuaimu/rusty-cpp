// rusty.async module — `Executor` only.
//
// `Executor`'s task storage uses vec_port::Vec (transpiled rustc Vec). Headers
// cannot `import` C++20 modules, hence Executor is split out from async.hpp
// into this module unit. The header (rusty/async.hpp) still defines Poll,
// Waker, Context, Task, block_on — types that the transpiled vec_port /
// btree_port / hashbrown_port preludes reference as `rusty::Poll<T>` and
// `rusty::Context`. Keeping those in a header means we don't break the
// transpiled libraries' header-mode consumption.
//
// Consumer pattern:
//     import rusty.async;          // brings in rusty::Executor
//     #include <rusty/async.hpp>   // brings in rusty::Task<void> etc.
// (Importing the umbrella `rusty` module pulls both in transitively.)

module;

#include <queue>
#include <utility>
#include <rusty/async.hpp>    // Task<void> in module purview signatures
#include <rusty/alloc.hpp>    // rusty::alloc::Global

export module rusty.async;

import vec_port.vec;

export namespace rusty {

// ── Executor: event loop ───────────────────────────────────────
// MIGRATION: tasks_ was `rusty::VecLegacy<Task<void>>`. Now uses
// vec_port::Vec (the transpiled rustc Vec, exported flat at global scope
// as `::Vec<T, A>`). API surface: new_in (no parameterless new_ for now —
// vec_port requires an allocator instance), push, len, operator[].
class Executor {
public:
    Executor() : tasks_(::Vec<Task<void>, ::rusty::alloc::Global>::new_in(::rusty::alloc::Global{})) {}

    void spawn(Task<void> task) {
        tasks_.push(std::move(task));
        ready_queue_.push(tasks_.len() - 1);
    }

    void run() {
        while (!ready_queue_.empty()) {
            auto idx = ready_queue_.front();
            ready_queue_.pop();

            Waker waker{[this, idx]() { ready_queue_.push(idx); }};
            Context cx{&waker};

            // vec_port::Vec only exposes a const operator[]. For mutable
            // access, go through as_mut_ptr() — Task<void>::poll is non-const.
            auto result = tasks_.as_mut_ptr()[idx].poll(cx);
            // If Pending, waker will re-enqueue when IO fires
            // If Ready, task is done
        }
    }

private:
    ::Vec<Task<void>, ::rusty::alloc::Global> tasks_;
    std::queue<size_t> ready_queue_;
};

} // export namespace rusty
