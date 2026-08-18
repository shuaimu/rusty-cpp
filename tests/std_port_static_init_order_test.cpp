// Static-initialization-ORDER guard for the transpiled hashbrown table.
//
// Regression test for the `RawTableInner::NEW` / `Tag::EMPTY` class of bug:
// a Rust associated `const` whose type is the enclosing struct used to lower to
// a namespace-scope *dynamically* initialized inline variable
//
//     inline const RawTableInner RawTableInner::NEW = RawTableInner::new_();
//
// living in the `hashbrown` module's purview. A NON-module translation unit that
// populates a hashbrown-backed container from its OWN static initializer runs
// before that module initializer, so it clones an all-zero `NEW`, gets
// `ctrl == nullptr`, and null-derefs on the first resize. mako saw this as eight
// SEGFAULTing test binaries while everything compiled clean; the trigger there
// was `src/deptran/procedure.cc`'s `__cxx_global_var_init` reaching a
// hashbrown-backed registry through an ordinary function declaration.
//
// The fix lowers such consts as function-local-static accessors (`Owner::NAME()`
// — the C++11 magic-static rule makes them initialize on first use, so ordering
// cannot matter). This file is the runtime proof.
//
// SHAPE MATTERS, do not "simplify" this to one TU: when the TU that runs the
// pre-main initializer ALSO says `import std_port;`, clang orders that module's
// initializer ahead of the TU's own variable initializers and the bug is hidden.
// The hazard needs the initializing TU to reach the container INDIRECTLY, so the
// container work lives in std_port_static_init_order_registry.cpp and this
// driver only sees function declarations. This TU is also listed FIRST on the
// link line so its constructors run first.
//
// Keep the work big enough to force several table resizes — the crash is in
// `resize_inner`/`full_buckets_indices` reading the control bytes, not in
// construction.

#include <cstddef>
#include <cstdio>
#include <cstdlib>

// NOT assert(): these targets compile with -DNDEBUG. fflush before abort or the
// message is lost when stdout is piped.
#define CHECK(cond)                                                            \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);        \
            std::fflush(stdout);                                               \
            std::abort();                                                      \
        }                                                                      \
    } while (0)

// Defined in std_port_static_init_order_registry.cpp (the TU that imports the
// module). No module import here — see the note above.
void repro_register(int key, int value);
std::size_t repro_len();
std::size_t repro_seen_len();
bool repro_lookup(int key, int* out);
bool repro_seen_contains(int key);

static constexpr int kCount = 512;

static int g_registered = 0;

// Namespace-scope object whose CONSTRUCTOR does the container work: this runs
// before main, and — critically — potentially before the hashbrown module's own
// dynamic initialization.
struct Registrar {
    Registrar() {
        for (int i = 0; i < kCount; ++i) {
            repro_register(i, i * 3 + 1);
            ++g_registered;
        }
    }
};

static Registrar g_registrar;

int main() {
    std::printf("std_port_static_init_order_test\n");
    CHECK(g_registered == kCount);
    CHECK(repro_len() == static_cast<std::size_t>(kCount));
    CHECK(repro_seen_len() == static_cast<std::size_t>(kCount));

    // Every key inserted pre-main must be findable post-main with the right
    // value: a zeroed control block does not only crash, it can also silently
    // mis-place entries (all-zero tags read as "full, tag 0" rather than EMPTY).
    for (int i = 0; i < kCount; ++i) {
        int value = 0;
        CHECK(repro_lookup(i, &value));
        CHECK(value == i * 3 + 1);
        CHECK(repro_seen_contains(i));
    }
    int sink = 0;
    CHECK(!repro_lookup(kCount + 1, &sink));
    CHECK(!repro_seen_contains(kCount + 1));

    std::printf("  pre-main insertion of %d entries ok\n", kCount);
    std::printf("std_port_static_init_order_test PASSED\n");
    return 0;
}
