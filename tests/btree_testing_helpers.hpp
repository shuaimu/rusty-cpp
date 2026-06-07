// C++ port of rustc's btree test helpers:
// - library/alloctests/testing/crash_test.rs → CrashTestDummy, Instance, Panic
// - library/alloctests/testing/ord_chaos.rs → Cyclic3, Governor, Governed, IdBased
//
// Used by the rustc-translated btree tests in btree_tests_port_unstubbed.cpp.
//
// Differences from the Rust originals:
//   - No `catch_unwind` / panic recovery available in the codebase, so the
//     `Panic::InClone / InDrop / InQuery` variants call `std::abort()` rather
//     than unwinding. Tests that rely on observing post-panic state are NOT
//     translatable yet; tests that only need drop/clone/query counting with
//     `Panic::Never` work.
//   - Atomics use `std::atomic<size_t>` (Rust's `AtomicUsize` semantics).
//   - `Governed` and `Cyclic3` expose both `operator<` and `cmp()` so they
//     interop with whichever path the BTreeMap implementation takes.
//
// All types match Rust's logical semantics for the operations the tests
// actually exercise (Ord, Eq, Drop side effects).

#pragma once

#include <atomic>
#include <cassert>
#include <cstddef>
#include <cstdlib>
#include <string>

namespace btree_testing {

// ─────────────────────────────────────────────────────────────────────
// crash_test.rs
// ─────────────────────────────────────────────────────────────────────

enum class Panic {
    Never,
    InClone,
    InDrop,
    InQuery,
};

class Instance;

class CrashTestDummy {
public:
    size_t id;

    explicit CrashTestDummy(size_t id_) : id(id_), cloned_(0), dropped_(0), queried_(0) {}

    CrashTestDummy(const CrashTestDummy&) = delete;
    CrashTestDummy& operator=(const CrashTestDummy&) = delete;

    Instance spawn(Panic panic) const;

    size_t cloned() const { return cloned_.load(std::memory_order_seq_cst); }
    size_t dropped() const { return dropped_.load(std::memory_order_seq_cst); }
    size_t queried() const { return queried_.load(std::memory_order_seq_cst); }

private:
    friend class Instance;
    mutable std::atomic<size_t> cloned_;
    mutable std::atomic<size_t> dropped_;
    mutable std::atomic<size_t> queried_;
};

// `Instance` is the value put into the BTreeMap. Its copy ctor counts clones,
// its destructor counts drops. The pointer back to `CrashTestDummy` is the
// id-source — comparisons hash off `origin->id`.
//
// We use a pointer instead of Rust's `&'a CrashTestDummy` reference. The
// dummies must outlive every Instance that points at them — same lifetime
// rule as in Rust, just enforced by convention rather than by the borrow
// checker.
class Instance {
public:
    Instance(const CrashTestDummy* origin, Panic panic)
        : origin_(origin), panic_(panic) {}

    Instance(const Instance& other) : origin_(other.origin_), panic_(Panic::Never) {
        origin_->cloned_.fetch_add(1, std::memory_order_seq_cst);
        if (other.panic_ == Panic::InClone) {
            // Rust would panic here; we don't have catch_unwind so abort.
            std::abort();
        }
    }

    Instance(Instance&& other) noexcept : origin_(other.origin_), panic_(other.panic_) {
        other.origin_ = nullptr;
        other.panic_ = Panic::Never;
    }

    Instance& operator=(const Instance& other) {
        if (this == &other) return *this;
        origin_ = other.origin_;
        panic_ = Panic::Never;
        origin_->cloned_.fetch_add(1, std::memory_order_seq_cst);
        if (other.panic_ == Panic::InClone) {
            std::abort();
        }
        return *this;
    }

    Instance& operator=(Instance&& other) noexcept {
        if (this == &other) return *this;
        origin_ = other.origin_;
        panic_ = other.panic_;
        other.origin_ = nullptr;
        other.panic_ = Panic::Never;
        return *this;
    }

    ~Instance() {
        if (origin_ != nullptr) {
            origin_->dropped_.fetch_add(1, std::memory_order_seq_cst);
            if (panic_ == Panic::InDrop) {
                // Rust would panic; C++ in a dtor must not throw, so abort.
                std::abort();
            }
        }
    }

    size_t id() const { return origin_->id; }

    template<typename R>
    R query(R result) const {
        origin_->queried_.fetch_add(1, std::memory_order_seq_cst);
        if (panic_ == Panic::InQuery) {
            std::abort();
        }
        return result;
    }

    // Ord-style comparison the BTreeMap uses on keys.
    bool operator<(const Instance& other) const { return id() < other.id(); }
    bool operator==(const Instance& other) const { return id() == other.id(); }
    bool operator!=(const Instance& other) const { return id() != other.id(); }
    bool operator<=(const Instance& other) const { return id() <= other.id(); }
    bool operator>(const Instance& other) const { return id() > other.id(); }
    bool operator>=(const Instance& other) const { return id() >= other.id(); }

private:
    const CrashTestDummy* origin_;
    Panic panic_;
};

inline Instance CrashTestDummy::spawn(Panic panic) const {
    return Instance(this, panic);
}

// ─────────────────────────────────────────────────────────────────────
// ord_chaos.rs
// ─────────────────────────────────────────────────────────────────────

// Cyclic3: 3-state enum whose Ord violates transitivity. A<B<C<A.
// Used to feed the BTreeMap broken inputs and verify it doesn't UB even
// when its `Ord` invariants are violated.
enum class Cyclic3 { A, B, C };

inline bool cyclic3_lt(Cyclic3 a, Cyclic3 b) {
    // Less in: (A,B), (B,C), (C,A).
    switch (a) {
        case Cyclic3::A: return b == Cyclic3::B;
        case Cyclic3::B: return b == Cyclic3::C;
        case Cyclic3::C: return b == Cyclic3::A;
    }
    return false;
}

inline bool operator<(Cyclic3 a, Cyclic3 b) { return cyclic3_lt(a, b); }
inline bool operator==(Cyclic3 a, Cyclic3 b) {
    return static_cast<int>(a) == static_cast<int>(b);
}
inline bool operator!=(Cyclic3 a, Cyclic3 b) { return !(a == b); }
inline bool operator<=(Cyclic3 a, Cyclic3 b) { return a < b || a == b; }
inline bool operator>(Cyclic3 a, Cyclic3 b) { return cyclic3_lt(b, a); }
inline bool operator>=(Cyclic3 a, Cyclic3 b) { return b < a || a == b; }

// Governor: shared flip-state used by Governed<T>.
class Governor {
public:
    Governor() : flipped_(false) {}

    Governor(const Governor&) = delete;
    Governor& operator=(const Governor&) = delete;

    void flip() const { flipped_ = !flipped_; }
    bool flipped() const { return flipped_; }

private:
    mutable bool flipped_;
};

// Governed<T>: wraps a T but consults a Governor at compare time. With the
// governor un-flipped, comparisons match T's normal ordering; flipped, the
// ordering inverts. Lets tests build a BTreeMap under one order and then
// observe what happens when the order suddenly flips.
template<typename T>
struct Governed {
    T value;
    const Governor* gov;

    Governed(T v, const Governor* g) : value(std::move(v)), gov(g) {}

    bool operator<(const Governed& other) const {
        assert(gov == other.gov && "Governed values from different Governors compared");
        if (value < other.value) return !gov->flipped();
        if (other.value < value) return gov->flipped();
        return false;
    }
    bool operator==(const Governed& other) const {
        assert(gov == other.gov && "Governed values from different Governors compared");
        return value == other.value;
    }
    bool operator!=(const Governed& other) const { return !(*this == other); }
    bool operator<=(const Governed& other) const { return *this < other || *this == other; }
    bool operator>(const Governed& other) const { return other < *this; }
    bool operator>=(const Governed& other) const { return other < *this || *this == other; }
};

// IdBased: id determines order, name is opaque. Lets tests insert
// "different" values that map to the same key.
struct IdBased {
    uint32_t id;
    std::string name;

    IdBased(uint32_t id_, std::string name_) : id(id_), name(std::move(name_)) {}

    bool operator<(const IdBased& other) const { return id < other.id; }
    bool operator==(const IdBased& other) const { return id == other.id; }
    bool operator!=(const IdBased& other) const { return id != other.id; }
    bool operator<=(const IdBased& other) const { return id <= other.id; }
    bool operator>(const IdBased& other) const { return id > other.id; }
    bool operator>=(const IdBased& other) const { return id >= other.id; }
};

}  // namespace btree_testing
