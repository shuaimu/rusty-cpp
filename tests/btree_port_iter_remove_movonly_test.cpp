// Coverage-gap pin: btree_port's transpiled BTreeMap was only tested
// with trivially-copyable `int32_t` via insert/get/first_key_value/
// last_key_value/contains/len. None of the existing tests exercise
// `iter()`, `remove()`, or `clone()` — the code paths that drag in
// four latent bugs reported by the rrr::Alarm migration attempt:
//
//   (B1) Variant tuple-field access — Rust `r._0` lowered as direct
//        `.._0` member access on std::variant / std::tuple return
//        values inside btree_internal node-traversal helpers (same
//        shape as hashbrown_port::Iter::next() fix in ae978d9).
//
//   (B2) NodeRef temp binding — borrowed handle bound to `auto&`
//        where the originating Rust expression has a lifetimed
//        receiver, leaving a dangling/short-lived temporary.
//
//   (B3) const-correctness drift — `BTreeMap::iter()` declared
//        `const` in C++ but body calls non-const helpers (the Rust
//        `&self -> Iter` signature didn't propagate `const` through
//        the transpiler's reborrow/full_range chain).
//
//   (B4) Copy-ctor requirement on move-only T — the iter/remove
//        instantiation path drags in a branch that asks for
//        `T(const T&)` even when T is move-only.
//
// Trigger payload: `BTreeMap<int64_t, std::pair<int64_t, rusty::Function<void()>>>`
// where `rusty::Function` is move-only (copy ctor deleted; see
// include/rusty/function.hpp). The test asks the map to iter over
// the pairs, remove a key, and (later) clone — any of which should
// reproduce at least one of the four bugs.
//
// Test is wired into CMake but expected to FAIL TO COMPILE today.
// Once each bug is fixed, the assertions also check runtime behavior.
// Marking compile-failure as expected pins coverage without
// gating CI green/red.
//
// Sibling tests with only-copyable T (btree_port_module_test.cpp and
// btree_port_set_module_test.cpp) miss this entire surface — they
// only call insert/get/first_key_value/last_key_value/contains/len.

import btree_port.btree.map;

#include <rusty/alloc.hpp>
#include <rusty/function.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <utility>

namespace {

struct MoveOnlyCallable {
    int payload;
    MoveOnlyCallable() : payload(0) {}
    explicit MoveOnlyCallable(int p) : payload(p) {}
    MoveOnlyCallable(const MoveOnlyCallable&) = delete;
    MoveOnlyCallable& operator=(const MoveOnlyCallable&) = delete;
    MoveOnlyCallable(MoveOnlyCallable&&) noexcept = default;
    MoveOnlyCallable& operator=(MoveOnlyCallable&&) noexcept = default;
    void operator()() const {}
};

}  // namespace

int main() {
    // T payload: pair<id, move-only callable>. The callable mirrors the
    // rrr::Alarm migration's `rusty::Function<void()>` value — any
    // move-only T works; we use a hand-rolled one to keep the test
    // free of rusty.function's full dependency set.
    using V = std::pair<int64_t, MoveOnlyCallable>;
    using BTreeMap =
        ::rusty::port::collections::btree::map::BTreeMap<int64_t, V,
                                                        ::rusty::alloc::Global>;

    auto m = BTreeMap::new_in(::rusty::alloc::Global{});

    // Insert a few entries — exercises the non-iter path, which the
    // current smoke test already covers but with a copyable V. Doing
    // it with move-only V here exposes whether the insert/storage
    // path itself accidentally requires copy.
    m.insert(1, V{10, MoveOnlyCallable{100}});
    m.insert(2, V{20, MoveOnlyCallable{200}});
    m.insert(3, V{30, MoveOnlyCallable{300}});

    // (B3) iter() is declared const but body chain reborrow()/
    // full_range() may not be const. If transpiler regressed the
    // const-correctness, this line fails to compile.
    const BTreeMap& cref = m;
    auto it = cref.iter();
    (void)it;

    // (B1) range-for over iter — yields std::tuple<const K&, const V&>
    // (or similar). If transpiler emits `.._0` access on the tuple
    // return value inside btree_internal node helpers, this fails to
    // compile. Range-for is the canonical user-facing path so it must
    // work.
    int64_t seen = 0;
    for (const auto& kv : cref) {
        // Either binding shape — both should work post-fix.
        // structured-binding form catches `._0` emit issues by
        // forcing the compiler to project the tuple.
        const auto& k = std::get<0>(kv);
        const auto& v = std::get<1>(kv);
        (void)v;
        seen += k;
    }
    assert(seen == 1 + 2 + 3);

    // (B2) NodeRef temp binding — bind explicit iterator results to
    // `auto` (not `auto&`). If the transpiler bound a borrowed handle
    // to `auto&` somewhere in the call chain, the temp would dangle;
    // accessing the result after the chain returns surfaces UB or
    // miscompiles.
    auto explicit_iter = m.iter();
    (void)explicit_iter;

    // (B4) remove path — drags in the same code branches as iter()
    // plus extra paths. Move-only V should be supported; if the
    // template instantiation asks for `V(const V&)`, this fails to
    // compile.
    auto removed = m.remove(2);
    assert(removed.is_some());

    std::printf(
        "btree_port iter+remove+move-only-T coverage pin: ALL CHECKS PASSED\n");
    return 0;
}
