// Unit tests for the runtime's lowering of Rust's relocating reads:
// `MaybeUninit::assume_init_read` and `NonNull::read`.
//
// Both are `ptr::read` in Rust: the value moves to the caller and the source is
// thereafter logically uninitialized — nothing drops it again. Transpiled code
// relies on that (btree's slice_remove shifts over the read slot, a dying node
// is freed without drops, a consumed IntoIter range is never revisited). So the
// lowering must END the source's lifetime; a plain copy leaves the original
// alive in a slot nobody destroys, leaking one element per read — for a
// refcounted element, one strong reference (tests/btree_port_drop_balance_test
// pins the btree-level symptom).
//
// `std::shared_ptr::use_count()` is the Rc analogue. A counted type checks
// construct/destroy balance independently of refcounting.
#include "../include/rusty/maybe_uninit.hpp"
#include "../include/rusty/ptr.hpp"

#include <cstdio>
#include <memory>
#include <new>

namespace {

int g_failures = 0;
#define CHECK(cond, ...)                                           \
    do {                                                           \
        if (!(cond)) {                                             \
            std::printf("  FAIL %s:%d: ", __FILE__, __LINE__);     \
            std::printf(__VA_ARGS__);                              \
            std::printf("\n");                                     \
            ++g_failures;                                          \
            return;                                                \
        }                                                          \
    } while (0)

struct Counted {
    static inline long constructed = 0;
    static inline long destroyed = 0;
    int v;
    explicit Counted(int x) : v(x) { ++constructed; }
    Counted(const Counted& o) : v(o.v) { ++constructed; }
    Counted(Counted&& o) noexcept : v(o.v) { ++constructed; }
    ~Counted() { ++destroyed; }
};

void maybe_uninit_read_relocates_refcounted() {
    auto owner = std::make_shared<int>(7);
    rusty::MaybeUninit<std::shared_ptr<int>> slot;
    slot.write(owner);
    CHECK(owner.use_count() == 2, "after write use_count = %ld", static_cast<long>(owner.use_count()));
    {
        auto out = slot.assume_init_read();
        CHECK(*out == 7, "read value %d", *out);
        CHECK(owner.use_count() == 2,
              "while read value alive use_count = %ld, want 2 (a copy makes 3)",
              static_cast<long>(owner.use_count()));
    }
    // The slot is logically uninitialized now; nothing will drop it.
    CHECK(owner.use_count() == 1,
          "after read value dropped use_count = %ld, want 1 (the slot kept a reference)",
          static_cast<long>(owner.use_count()));
}

void maybe_uninit_read_balances_lifetimes() {
    Counted::constructed = Counted::destroyed = 0;
    {
        rusty::MaybeUninit<Counted> slot;
        slot.write(Counted(1));
        auto out = slot.assume_init_read();
        CHECK(out.v == 1, "read value %d", out.v);
    }
    CHECK(Counted::constructed == Counted::destroyed,
          "constructed %ld != destroyed %ld", Counted::constructed, Counted::destroyed);
}

// Trivially destructible T keeps copy semantics: btree reads the same
// pointer-shaped handle more than once, which is only sound because nothing is
// destroyed.
void maybe_uninit_read_trivial_is_repeatable() {
    rusty::MaybeUninit<int> slot;
    slot.write(42);
    const int a = slot.assume_init_read();
    const int b = slot.assume_init_read();
    CHECK(a == 42 && b == 42, "repeat reads %d %d", a, b);
}

void nonnull_read_relocates_refcounted() {
    auto owner = std::make_shared<int>(9);
    alignas(std::shared_ptr<int>) unsigned char storage[sizeof(std::shared_ptr<int>)];
    auto* raw = ::new (static_cast<void*>(storage)) std::shared_ptr<int>(owner);
    auto nn = rusty::ptr::NonNull<std::shared_ptr<int>>::new_unchecked(raw);
    CHECK(owner.use_count() == 2, "after placement use_count = %ld", static_cast<long>(owner.use_count()));
    {
        auto out = nn.read();
        CHECK(*out == 9, "read value %d", *out);
        CHECK(owner.use_count() == 2,
              "while read value alive use_count = %ld, want 2 (a copy makes 3)",
              static_cast<long>(owner.use_count()));
    }
    CHECK(owner.use_count() == 1,
          "after read value dropped use_count = %ld, want 1 (the source kept a reference)",
          static_cast<long>(owner.use_count()));
}

void nonnull_read_trivial_is_repeatable() {
    int x = 5;
    auto nn = rusty::ptr::NonNull<int>::new_unchecked(&x);
    CHECK(nn.read() == 5 && nn.read() == 5 && x == 5, "repeat reads changed the source");
}

struct Case {
    const char* name;
    void (*fn)();
};

}  // namespace

int main() {
    const Case cases[] = {
        {"maybe_uninit_read_relocates_refcounted", maybe_uninit_read_relocates_refcounted},
        {"maybe_uninit_read_balances_lifetimes", maybe_uninit_read_balances_lifetimes},
        {"maybe_uninit_read_trivial_is_repeatable", maybe_uninit_read_trivial_is_repeatable},
        {"nonnull_read_relocates_refcounted", nonnull_read_relocates_refcounted},
        {"nonnull_read_trivial_is_repeatable", nonnull_read_trivial_is_repeatable},
    };
    int failed = 0;
    for (const Case& c : cases) {
        const int before = g_failures;
        c.fn();
        const bool ok = g_failures == before;
        std::printf("%s %s\n", ok ? "ok  " : "FAIL", c.name);
        if (!ok) ++failed;
    }
    if (failed != 0) {
        std::printf("relocating read: %d of %zu case(s) FAILED\n", failed, sizeof(cases) / sizeof(cases[0]));
        return 1;
    }
    std::printf("relocating read: all %zu cases passed\n", sizeof(cases) / sizeof(cases[0]));
    return 0;
}
