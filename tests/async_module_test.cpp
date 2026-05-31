// Smoke test: imports the rusty.async module and exercises the basic
// Task<void> + Executor flow. Proves the module compiles and that the
// Executor's task storage (now vec_port::Vec instead of legacy VecLegacy)
// instantiates and runs.

#include <rusty/async.hpp>   // rusty::Task<void> (header lives outside the module)
#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdio>

import rusty.async;          // rusty::Executor

static int counter = 0;

rusty::Task<void> hello() {
    counter += 1;
    co_return;
}

int main() {
    rusty::Executor exec;
    exec.spawn(hello());
    exec.spawn(hello());
    exec.spawn(hello());
    exec.run();

    assert(counter == 3);

    std::printf("rusty.async module smoke: ALL CHECKS PASSED\n");
    return 0;
}
