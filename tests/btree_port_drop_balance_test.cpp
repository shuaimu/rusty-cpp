// Drop-balance regression test for the transpiled btree_port.
//
// Every value a BTreeMap / BTreeSet takes ownership of must be destroyed
// exactly once, and a value handed back to the caller (remove, pop, take,
// into_iter, extract_if, ...) must leave nothing behind in the tree.
//
// The defect this pins: rustc's btree extracts elements from node slots with
// `MaybeUninit::assume_init_read` (a bitwise read: the slot is thereafter
// logically uninitialized and never dropped). The C++ runtime lowered that to a
// COPY for copy-constructible T, so every extraction cloned the element and
// left the original alive in a slot the tree had already forgotten: the slot is
// then either overwritten by the slice shift in `slice_remove`, or sits past
// the node's new `len`, and nothing ever destroys it. For a refcounted element
// (`rusty::Rc`, `std::shared_ptr`) that is one strong reference leaked per
// removal — SRPC's reactor saw `Rc<Fiber>::strong_count` stuck at 3 where 2 is
// expected after `fibers_.remove`. Removal is not the only path: node splits
// (`split_leaf_data`, i.e. plain inserts), rebalancing (`bulk_steal_*`),
// `into_iter`, `extract_if` and `append` all extract through the same read.
//
// Two independent instruments, so a fix cannot satisfy one by accident:
//   * `Tracked`       — counts constructions vs destructions, and per id how
//                       many non-moved-from objects still hold it. After a
//                       value leaves the tree and the caller drops it, its
//                       holder count must be 0; when every container is gone,
//                       constructed == destroyed.
//   * `std::shared_ptr` — `use_count()` is the Rc analogue of the SRPC report:
//                       it must return to 1 (only the test's own handle) the
//                       moment the tree gives its reference back.
//
// This TU is built with -DNDEBUG (btree_port_* wiring), so every check is an
// explicit CHECK, never assert. Under ASan/LSan a leak here also shows up as
// unfreed shared_ptr control blocks.

import btree_port.btree.map;
import btree_port.btree.set;

#include <rusty/alloc.hpp>
#include <array>
#include <cstdint>
#include <cstdio>
#include <memory>
#include <tuple>
#include <utility>
#include <vector>

namespace {

template <typename K, typename V>
using BTreeMap = ::btree_port::btree::map::BTreeMap<K, V, ::rusty::alloc::Global>;
template <typename T>
using BTreeSet = ::btree_port::btree::set::BTreeSet<T, ::rusty::alloc::Global>;

template <typename K, typename V>
BTreeMap<K, V> make_map() { return BTreeMap<K, V>::new_in(::rusty::alloc::Global{}); }
template <typename T>
BTreeSet<T> make_set() { return BTreeSet<T>::new_in(::rusty::alloc::Global{}); }

// 300 entries: with CAPACITY = 11 this is a height-2 tree, so inserts split
// leaves and internals, and removals hit internal KVs, steals and merges.
constexpr int kN = 300;

int g_failures = 0;
#define CHECK(cond, ...)                                                        \
    do {                                                                        \
        if (!(cond)) {                                                          \
            std::printf("  FAIL %s:%d: ", __FILE__, __LINE__);                  \
            std::printf(__VA_ARGS__);                                           \
            std::printf("\n");                                                  \
            ++g_failures;                                                       \
            return;                                                             \
        }                                                                       \
    } while (0)

// A value (and key) type whose every construction and destruction is counted.
// `id == -1` marks a moved-from husk, which holds nothing.
struct Tracked {
    static inline long constructed = 0;
    static inline long destroyed = 0;
    static inline std::array<int, kN + 1> holders{};

    int id;

    static void reset() {
        constructed = 0;
        destroyed = 0;
        holders.fill(0);
    }
    static long live() { return constructed - destroyed; }

    explicit Tracked(int i) : id(i) { ++constructed; ++holders[i]; }
    Tracked(const Tracked& o) : id(o.id) { ++constructed; if (id >= 0) ++holders[id]; }
    Tracked(Tracked&& o) noexcept : id(o.id) { ++constructed; o.id = -1; }
    Tracked& operator=(const Tracked& o) {
        if (this != &o) {
            if (id >= 0) --holders[id];
            id = o.id;
            if (id >= 0) ++holders[id];
        }
        return *this;
    }
    Tracked& operator=(Tracked&& o) noexcept {
        if (this != &o) {
            if (id >= 0) --holders[id];
            id = o.id;
            o.id = -1;
        }
        return *this;
    }
    ~Tracked() { ++destroyed; if (id >= 0) --holders[id]; }

    bool operator==(const Tracked& o) const { return id == o.id; }
    bool operator!=(const Tracked& o) const { return id != o.id; }
    bool operator<(const Tracked& o) const { return id < o.id; }
    bool operator<=(const Tracked& o) const { return id <= o.id; }
    bool operator>(const Tracked& o) const { return id > o.id; }
    bool operator>=(const Tracked& o) const { return id >= o.id; }
};

// Every id in [lo, hi) with `pred(id)` must be held exactly `want` times.
template <typename P>
int first_bad_holder(int lo, int hi, P pred, int want) {
    for (int i = lo; i < hi; ++i) {
        if (pred(i) && Tracked::holders[i] != want) return i;
    }
    return -1;
}
constexpr auto kAll = [](int) { return true; };

BTreeMap<int, Tracked> filled_map(int n) {
    auto m = make_map<int, Tracked>();
    for (int i = 0; i < n; ++i) m.insert(i, Tracked(i));
    return m;
}

// ── map value paths ─────────────────────────────────────────────────────────

// Single-leaf tree: `remove` goes straight through `remove_leaf_kv` ->
// `slice_remove`, no rebalancing.
void map_remove_single_leaf() {
    Tracked::reset();
    {
        auto m = filled_map(8);
        for (int i = 0; i < 8; ++i) {
            auto r = m.remove(i);
            CHECK(r.is_some() && r.unwrap().id == i, "remove(%d) did not return it", i);
        }
        CHECK(m.is_empty(), "map not empty");
        int bad = first_bad_holder(0, 8, kAll, 0);
        CHECK(bad < 0, "id %d still held %d time(s) after its remove", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// Height-2 tree: evens then odds, so removals hit internal KVs
// (`remove_internal_kv` -> `replace_kv`), `bulk_steal_left/right` and merges.
void map_remove_multi_level() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        for (int i = 0; i < kN; i += 2) {
            auto r = m.remove(i);
            CHECK(r.is_some() && r.unwrap().id == i, "remove(%d) did not return it", i);
        }
        int bad = first_bad_holder(0, kN, [](int i) { return i % 2 == 0; }, 0);
        CHECK(bad < 0, "removed id %d still held %d time(s)", bad, Tracked::holders[bad]);
        bad = first_bad_holder(0, kN, [](int i) { return i % 2 == 1; }, 1);
        CHECK(bad < 0, "kept id %d held %d time(s), want 1", bad, Tracked::holders[bad]);
        for (int i = 1; i < kN; i += 2) {
            CHECK(m.remove(i).is_some(), "remove(%d) missing", i);
        }
        CHECK(m.is_empty(), "map not empty after draining");
        bad = first_bad_holder(0, kN, kAll, 0);
        CHECK(bad < 0, "id %d still held %d time(s) after drain", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// Inserts alone: every node split moves the middle KV out with
// `split_leaf_data`. Dropping the map must then release every element once.
void map_insert_splits_then_drop() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        int bad = first_bad_holder(0, kN, kAll, 1);
        CHECK(bad < 0, "after inserts id %d held %d time(s), want 1", bad, Tracked::holders[bad]);
    }
    int bad = first_bad_holder(0, kN, kAll, 0);
    CHECK(bad < 0, "after drop id %d still held %d time(s)", bad, Tracked::holders[bad]);
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

void map_pop_first_last() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        for (int i = 0; i < kN / 2; ++i) {
            auto f = m.pop_first();
            CHECK(f.is_some(), "pop_first #%d empty", i);
            auto l = m.pop_last();
            CHECK(l.is_some(), "pop_last #%d empty", i);
        }
        CHECK(m.is_empty(), "map not empty");
        int bad = first_bad_holder(0, kN, kAll, 0);
        CHECK(bad < 0, "id %d still held %d time(s) after pop", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

void map_entry_remove() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        for (int i = 0; i < kN; i += 3) {
            auto e = m.first_entry();
            CHECK(e.is_some(), "first_entry empty at %d", i);
            auto kv = std::move(e).unwrap().remove_entry();
            (void)kv;
        }
        int bad = first_bad_holder(0, kN / 3, kAll, 0);
        CHECK(bad < 0, "entry-removed id %d still held %d time(s)", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// `into_iter` hands elements out through `into_key_val`; the rest are dropped
// in place by the IntoIter destructor (`drop_key_val`).
void map_into_iter_partial() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        auto it = m.into_iter();
        for (int i = 0; i < kN / 3; ++i) {
            auto kv = it.next();
            CHECK(kv.is_some(), "into_iter ended early at %d", i);
        }
        int bad = first_bad_holder(0, kN / 3, kAll, 0);
        CHECK(bad < 0, "yielded id %d still held %d time(s)", bad, Tracked::holders[bad]);
    }
    int bad = first_bad_holder(0, kN, kAll, 0);
    CHECK(bad < 0, "after IntoIter drop id %d still held %d time(s)", bad, Tracked::holders[bad]);
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

void map_retain() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        m.retain([](auto&& k, auto&&) { return k % 2 == 1; });
        CHECK(m.len() == static_cast<size_t>(kN / 2), "retain kept %zu", m.len());
        int bad = first_bad_holder(0, kN, [](int i) { return i % 2 == 0; }, 0);
        CHECK(bad < 0, "retain-dropped id %d still held %d time(s)", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// `append` drains both maps through IntoIter + `bulk_push`.
void map_append() {
    Tracked::reset();
    {
        auto a = make_map<int, Tracked>();
        auto b = make_map<int, Tracked>();
        for (int i = 0; i < kN; ++i) {
            if (i % 2 == 0) a.insert(i, Tracked(i));
            else b.insert(i, Tracked(i));
        }
        a.append(b);
        CHECK(a.len() == static_cast<size_t>(kN) && b.is_empty(), "append lengths %zu/%zu", a.len(), b.len());
        int bad = first_bad_holder(0, kN, kAll, 1);
        CHECK(bad < 0, "after append id %d held %d time(s), want 1", bad, Tracked::holders[bad]);
    }
    int bad = first_bad_holder(0, kN, kAll, 0);
    CHECK(bad < 0, "after drop id %d still held %d time(s)", bad, Tracked::holders[bad]);
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

void map_split_off_and_clear() {
    Tracked::reset();
    {
        auto m = filled_map(kN);
        auto right = m.split_off(kN / 2);
        CHECK(m.len() + right.len() == static_cast<size_t>(kN), "split_off lost entries");
        int bad = first_bad_holder(0, kN, kAll, 1);
        CHECK(bad < 0, "after split_off id %d held %d time(s), want 1", bad, Tracked::holders[bad]);
        m.clear();
        bad = first_bad_holder(0, kN / 2, kAll, 0);
        CHECK(bad < 0, "after clear id %d still held %d time(s)", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// Keys travel through the same slots as values.
void map_tracked_keys() {
    Tracked::reset();
    {
        auto m = make_map<Tracked, int>();
        for (int i = 0; i < kN; ++i) m.insert(Tracked(i), i);
        for (int i = 0; i < kN; i += 2) {
            CHECK(m.remove(Tracked(i)).is_some(), "remove key %d missing", i);
        }
        int bad = first_bad_holder(0, kN, [](int i) { return i % 2 == 0; }, 0);
        CHECK(bad < 0, "removed key %d still held %d time(s)", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// ── set paths ───────────────────────────────────────────────────────────────

void set_remove_take_pop() {
    Tracked::reset();
    {
        auto s = make_set<Tracked>();
        for (int i = 0; i < kN; ++i) s.insert(Tracked(i));
        for (int i = 0; i < kN; i += 3) {
            CHECK(s.remove(Tracked(i)), "set remove %d missing", i);
        }
        for (int i = 1; i < kN; i += 3) {
            auto t = s.take(Tracked(i));
            CHECK(t.is_some() && t.unwrap().id == i, "set take %d", i);
        }
        auto p = s.pop_first();
        CHECK(p.is_some() && p.unwrap().id == 2, "set pop_first");
        int bad = first_bad_holder(0, kN, [](int i) { return i % 3 != 2 || i == 2; }, 0);
        CHECK(bad < 0, "set-removed %d still held %d time(s)", bad, Tracked::holders[bad]);
    }
    CHECK(Tracked::live() == 0, "%ld Tracked object(s) never destroyed", Tracked::live());
}

// ── refcount mirror of the SRPC report (Rc<Fiber> in a registry) ────────────

void map_shared_ptr_use_count() {
    std::vector<std::shared_ptr<int>> owners;
    for (int i = 0; i < kN; ++i) owners.push_back(std::make_shared<int>(i));
    {
        auto m = make_map<int, std::shared_ptr<int>>();
        for (int i = 0; i < kN; ++i) m.insert(i, owners[i]);
        for (int i = 0; i < kN; ++i) {
            CHECK(owners[i].use_count() == 2, "use_count(%d) = %ld after insert, want 2",
                  i, static_cast<long>(owners[i].use_count()));
        }
        for (int i = 0; i < kN; ++i) {
            auto r = m.remove(i);
            CHECK(r.is_some(), "remove(%d) missing", i);
            r = rusty::Option<std::shared_ptr<int>>{rusty::None};
            CHECK(owners[i].use_count() == 1,
                  "use_count(%d) = %ld after remove, want 1 (the tree kept a reference)",
                  i, static_cast<long>(owners[i].use_count()));
        }
    }
    for (int i = 0; i < kN; ++i) {
        CHECK(owners[i].use_count() == 1, "use_count(%d) = %ld after map drop, want 1",
              i, static_cast<long>(owners[i].use_count()));
    }
}

struct Case {
    const char* name;
    void (*fn)();
};

}  // namespace

int main() {
    const Case cases[] = {
        {"map_remove_single_leaf", map_remove_single_leaf},
        {"map_remove_multi_level", map_remove_multi_level},
        {"map_insert_splits_then_drop", map_insert_splits_then_drop},
        {"map_pop_first_last", map_pop_first_last},
        {"map_entry_remove", map_entry_remove},
        {"map_into_iter_partial", map_into_iter_partial},
        {"map_retain", map_retain},
        {"map_append", map_append},
        {"map_split_off_and_clear", map_split_off_and_clear},
        {"map_tracked_keys", map_tracked_keys},
        {"set_remove_take_pop", set_remove_take_pop},
        {"map_shared_ptr_use_count", map_shared_ptr_use_count},
    };
    int failed_cases = 0;
    for (const Case& c : cases) {
        const int before = g_failures;
        c.fn();
        const bool ok = g_failures == before;
        std::printf("%s %s\n", ok ? "ok  " : "FAIL", c.name);
        if (!ok) ++failed_cases;
    }
    if (failed_cases != 0) {
        std::printf("btree_port drop balance: %d of %zu case(s) FAILED\n", failed_cases,
                    sizeof(cases) / sizeof(cases[0]));
        return 1;
    }
    std::printf("btree_port drop balance: all %zu cases passed\n", sizeof(cases) / sizeof(cases[0]));
    return 0;
}
