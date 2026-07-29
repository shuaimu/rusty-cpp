// Regression pin: `rusty::for_in` must NEVER consume an LVALUE container.
//
// The transpiler lowers Rust `for x in expr` to `rusty::for_in(expr)` and
// leaves the value category to for_in's dispatch. When the port modules'
// by-value-self `into_iter()` lost its `&&` qualifier, for_in's
// into_iter-first preference started consuming lvalues in place:
//   * btree map: the tree is moved out while the map stays in scope —
//     second iteration yields nothing, `get()` misses, `len()` is STALE
//     (surfaced as mako's ShardManager migration checksum mismatches);
//   * vec: double free at scope end (the IntoIter frees the buffer the
//     still-live Vec also owns).
// Rust cannot express that state — a consumed container's lifetime ends at
// the `for` — so for a still-live lvalue the only faithful lowering is a
// borrowing iteration. Rvalues (`for x in std::move(v)` / temporaries)
// still take the consuming path.
import vec_port.vec;          // rusty::port::vec::Vec<T, A> — transpiled Vec.

#include "../include/rusty/rusty.hpp"
#include "../include/rusty/slice.hpp"

#include <cassert>
#include <cstdio>
#include <string>
#include <utility>

namespace {

// Minimal stand-in for a port-module container whose ONLY iteration
// surface is an unqualified consuming `into_iter()` (the transpiled
// module shape). for_in must still work for lvalues via the guarded
// last-resort branch — such call sites own the container (a translated
// `for x in local` last use).
struct ConsumeOnly {
    int remaining = 3;
    struct IntoIter {
        int left;
        rusty::Option<int> next() {
            if (left <= 0) return rusty::None;
            --left;
            return rusty::Option<int>(3 - (left + 1) + 1);
        }
    };
    IntoIter into_iter() {
        int n = remaining;
        remaining = 0;  // consumed
        return IntoIter{n};
    }
};

}  // namespace

int main() {
    int failures = 0;

    // 1. Lvalue rusty::Vec: for_in twice must yield the items BOTH times
    //    and leave the Vec intact (pre-fix: double free / empty rerun).
    {
        auto v = rusty::port::vec::Vec<std::string>::new_();
        v.push(std::string("a"));
        v.push(std::string("b"));
        int n1 = 0, n2 = 0;
        for (auto&& e : rusty::for_in(v)) { (void)e; ++n1; }
        for (auto&& e : rusty::for_in(v)) { (void)e; ++n2; }
        if (n1 != 2 || n2 != 2 || v.len() != 2) {
            std::fprintf(stderr, "FAIL lvalue Vec: n1=%d n2=%d len=%zu\n",
                         n1, n2, (size_t)v.len());
            ++failures;
        }
    }

    // 2. Rvalue rusty::Vec: the consuming path still fires and yields
    //    owned elements exactly once.
    {
        auto v = rusty::port::vec::Vec<std::string>::new_();
        v.push(std::string("x"));
        v.push(std::string("y"));
        int n = 0;
        for (auto&& e : rusty::for_in(std::move(v))) { (void)e; ++n; }
        if (n != 2) {
            std::fprintf(stderr, "FAIL rvalue Vec: n=%d\n", n);
            ++failures;
        }
    }

    // 3. Lvalue of a consume-only container (no begin/end, no iter()):
    //    the guarded fallback keeps Rust's consuming for-loop semantics.
    {
        ConsumeOnly c;
        int n = 0;
        for (auto&& e : rusty::for_in(c)) { (void)e; ++n; }
        if (n != 3) {
            std::fprintf(stderr, "FAIL consume-only lvalue: n=%d\n", n);
            ++failures;
        }
    }

    std::printf("for_in lvalue-borrow: %s (%d failures)\n",
                failures ? "FAIL" : "OK", failures);
    return failures == 0 ? 0 : 1;
}
