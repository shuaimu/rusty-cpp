# RustyCpp vs Rust Borrow Checker Comparison

This document provides a comprehensive comparison between RustyCpp's analysis capabilities and Rust's borrow checker, highlighting what's implemented, what's missing, and the impact of each gap.

## Overview

RustyCpp aims to bring Rust-style memory safety checking to C++ code. While it implements many core borrow checking features, there are fundamental differences due to C++'s more permissive memory model and the challenges of static analysis on an existing language.

## Feature Comparison Matrix

### Core Borrow Checking

| Feature | Rust | RustyCpp | Status |
|---------|------|----------|--------|
| Use-after-move detection | ✅ | ✅ | **Complete** |
| Multiple mutable borrow prevention | ✅ | ✅ | **Complete** |
| Mutable + immutable borrow conflict | ✅ | ✅ | **Complete** |
| Scope-based lifetime tracking | ✅ | ✅ | **Complete** |
| Transitive borrow chains | ✅ | ✅ | **Complete** |
| Return reference to local | ✅ | ✅ | **Complete** |
| Reassignment after move | ✅ | ✅ | **Complete** |

### RAII and Lifetime Tracking

| Feature | Rust | RustyCpp | Status |
|---------|------|----------|--------|
| Scope-based drop timing | ✅ | ✅ | **Complete** |
| Iterator outlives container | ✅ | ✅ | **Complete** |
| Member lifetime (`&obj.field`) | ✅ | ✅ | **Complete** |
| Reference stored in container | ✅ | ✅ | **Complete** |
| User-defined RAII types | ✅ | ✅ | **Complete** |
| new/delete tracking | N/A | ✅ | **Complete** |
| Double-free detection | N/A | ✅ | **Complete** |
| Constructor init order | ✅ | ❌ | Missing |

### Lambda/Closure Safety

| Feature | Rust | RustyCpp | Status |
|---------|------|----------|--------|
| Capture safety (move vs borrow) | ✅ | ✅ | **Complete** |
| Escape analysis | ✅ | ✅ | **Complete** |
| 'this' capture detection | N/A | ✅ | **Complete** |

### Thread Safety (Runtime Library)

| Feature | Rust | RustyCpp | Status |
|---------|------|----------|--------|
| Send trait | ✅ | ✅ | **Complete** (template-based) |
| Sync trait | ✅ | ✅ | **Complete** (template-based) |
| Arc<T> Send+Sync rules | ✅ | ✅ | **Complete** |
| Rc<T> not Send/Sync | ✅ | ✅ | **Complete** |
| Lock-free MPSC channel | ✅ | ✅ | **Complete** |
| Custom type Send marking | ✅ | ✅ | **Complete** (RUSTY_MARK_SEND) |
| Static data race detection | ✅ | ❌ | **Missing** |

### Advanced Features

| Feature | Rust | RustyCpp | Status |
|---------|------|----------|--------|
| Non-lexical lifetimes (NLL) | ✅ | ❌ | **Missing** |
| Partial moves | ✅ | ✅ | **Complete** (including nested fields) |
| Partial borrows | ✅ | ✅ | **Complete** (December 2025) |
| Reborrowing | ✅ | ❌ | **Missing** |
| Two-phase borrows | ✅ | ❌ | **Missing** |
| Send/Sync (thread safety) | ✅ | ⚠️ | **Partial** (runtime) |
| Variance | ✅ | ❌ | **Missing** |
| Pin | ✅ | ❌ | **Missing** |

---

## Detailed Gap Analysis

### 1. Non-Lexical Lifetimes (NLL)

**Impact: HIGH** - Major source of false positives

**What Rust Does:**
NLL allows borrows to end at their last use, not at the end of their lexical scope.

```rust
// Rust allows this with NLL
fn example() {
    let mut x = 5;
    let y = &x;           // immutable borrow starts
    println!("{}", y);    // last use of y - borrow ends HERE
    x = 6;                // OK in Rust - y's borrow already ended
}
```

**What RustyCpp Does:**
RustyCpp uses lexical (scope-based) lifetimes. The borrow of `y` lasts until the end of its scope.

```cpp
// RustyCpp would reject this
// @safe
void example() {
    int x = 5;
    const int& y = x;     // immutable borrow
    std::cout << y;       // use y
    x = 6;                // ERROR: x still borrowed by y (false positive)
}
```

**Workaround:**
Use explicit scopes to limit borrow lifetime:
```cpp
// @safe
void example() {
    int x = 5;
    {
        const int& y = x;
        std::cout << y;
    }  // y's borrow ends here
    x = 6;  // OK now
}
```

**Implementation Complexity:** High - requires dataflow analysis to track last use points.

---

### 2. Partial Moves and Partial Borrows ✅ COMPLETE

**Impact: LOW** - Now fully implemented (December 2025)

**What Rust Does:**
Rust tracks moves and borrows at the field level:

```rust
struct Pair { a: String, b: String }

fn example() {
    let p = Pair {
        a: String::from("hello"),
        b: String::from("world")
    };

    let x = p.a;          // Move only p.a
    let y = p.b;          // OK - p.b wasn't moved
    // println!("{}", p.a); // ERROR - p.a was moved
    // let z = p;          // ERROR - p is partially moved
}

fn borrow_example() {
    let mut p = Pair { a: "".into(), b: "".into() };
    let r_a = &mut p.a;  // Borrow only p.a
    let r_b = &mut p.b;  // OK - p.b is separate field
    *r_a = "modified".into();
    *r_b = "also modified".into();
}
```

**What RustyCpp Does:**
RustyCpp has **complete partial move and partial borrow support**:

```cpp
// @safe
void move_example() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);   // ✅ Tracked: p.first moved
    std::string y = std::move(p.second);  // ✅ OK - p.second not moved yet
    std::string z = std::move(p.first);   // ❌ ERROR: p.first already moved
}

// @safe
void borrow_example() {
    Pair p;
    std::string& r1 = p.first;   // ✅ Mutable borrow of p.first
    std::string& r2 = p.second;  // ✅ OK - p.second is separate field
    r1 = "modified";
    r2 = "also modified";
}
```

**What Works (All Complete):**
- ✅ Moving individual fields (`std::move(p.first)`)
- ✅ Use-after-move detection per field
- ✅ Using unmoved fields after partial move
- ✅ Preventing whole-struct move after partial move
- ✅ **Nested field tracking** (`p.inner.data`)
- ✅ Nested double-move detection
- ✅ Whole-struct move after nested field move
- ✅ Field-level operations in IR (`MoveField`, `UseField`, `BorrowField`)
- ✅ **Partial borrow tracking** (borrowing different fields simultaneously)
- ✅ Borrow different fields mutably at the same time
- ✅ Whole struct vs field borrow conflict detection
- ✅ Sequential borrows in separate scopes (proper cleanup)
- ✅ Double mutable borrow of same field detection
- ✅ Mixed mutable/immutable borrow conflict detection

**Implementation:** Complete for both moves and borrows. See `docs/PARTIAL_MOVES_PLAN.md` for details.

---

### 3. Reborrowing

**Impact: MEDIUM** - Important for ergonomic code

**What Rust Does:**
Rust implicitly reborrows mutable references when passed to functions:

```rust
fn take_ref(r: &mut i32) {
    *r += 1;
}

fn example() {
    let mut x = 5;
    let r = &mut x;

    take_ref(r);  // Implicit reborrow - r is NOT moved
    take_ref(r);  // OK - r still valid

    *r = 10;      // OK - r still valid
}
```

**What RustyCpp Does:**
May treat passing a reference as consuming it:

```cpp
// @safe
void take_ref(int& r) { r += 1; }

void example() {
    int x = 5;
    int& r = x;

    take_ref(r);  // Does this "consume" r?
    take_ref(r);  // May be flagged as error
}
```

**Implementation Complexity:** Medium - requires understanding reference semantics in function calls.

---

### 4. Two-Phase Borrows

**Impact: LOW** - Specific to method call patterns

**What Rust Does:**
Rust allows borrowing `self` for a method while also borrowing arguments:

```rust
fn example() {
    let mut v = vec![1, 2, 3];
    v.push(v.len());  // Two-phase borrow:
                      // 1. &mut v for push()
                      // 2. &v for len()
}
```

**What RustyCpp Does:**
May reject this as conflicting borrows.

**Implementation Complexity:** Medium - requires special handling of method receivers.

---

### 5. Thread Safety (Send/Sync)

**Impact: HIGH** - Critical for concurrent code

**What Rust Does:**
Rust's type system prevents data races at compile time:

```rust
// Send: Type can be transferred to another thread
// Sync: Type can be shared between threads via &T

// Rc<T> is neither Send nor Sync
let rc = Rc::new(5);
// std::thread::spawn(move || { ... rc ... });  // COMPILE ERROR

// Arc<T> is both Send and Sync
let arc = Arc::new(5);
std::thread::spawn(move || { println!("{}", arc); });  // OK
```

**What RustyCpp Does:**
RustyCpp provides **runtime trait-based** Send/Sync support via C++ templates:

```cpp
#include <rusty/traits.hpp>
#include <rusty/sync/mpsc_lockfree.hpp>

// Primitives are automatically Send + Sync
static_assert(rusty::is_send<int>::value);
static_assert(rusty::is_sync<int>::value);

// Rc<T> is NOT Send (compile-time error if used with channels)
static_assert(!rusty::is_send<rusty::Rc<int>>::value);

// Arc<T> is Send + Sync if T is Send + Sync
static_assert(rusty::is_send<rusty::Arc<int>>::value);
static_assert(rusty::is_sync<rusty::Arc<int>>::value);

// Lock-free MPSC channel enforces Send at compile time
auto [tx, rx] = rusty::sync::mpsc::lockfree::channel<int>();  // OK
// auto [tx2, rx2] = channel<rusty::Rc<int>>();  // COMPILE ERROR - Rc not Send

// Mark custom types as Send
struct MyData { int x; };
RUSTY_MARK_SEND(MyData);
```

**Key differences from Rust:**
- **Runtime library approach**: Uses C++ template metaprogramming, not static analysis
- **Opt-in for custom types**: User must mark types as Send via `RUSTY_MARK_SEND()`
- **No automatic inference**: Static analyzer doesn't check thread safety in arbitrary code
- **Channel enforcement**: Send trait enforced when using rusty MPSC channels

**What's still missing:**
- Static analysis of `std::thread` spawns with non-Send types
- Detection of data races in raw C++ code
- Automatic Send/Sync inference for user types

**Implementation Complexity:** The runtime trait system is complete. Compile-time static analysis would be very high complexity.

---

### 6. Variance and Lifetime Subtyping

**Impact: LOW** - Advanced lifetime patterns

**What Rust Does:**
Rust has sophisticated lifetime variance rules:

```rust
// Covariant: &'a T can be used where &'b T expected if 'a: 'b (longer to shorter)
// Contravariant: For function parameters
// Invariant: &'a mut T (cannot vary at all)

fn example<'a, 'b: 'a>(long: &'b str, short: &'a str) -> &'a str {
    if long.len() > short.len() { long } else { short }
}
```

**What RustyCpp Does:**
Basic lifetime annotations without variance analysis.

**Implementation Complexity:** High - requires sophisticated type system integration.

---

### 7. Pin and Self-Referential Structs

**Impact: LOW** - Specific to async and self-referential patterns

**What Rust Does:**
`Pin<T>` prevents moving pinned data, enabling safe self-referential structs:

```rust
use std::pin::Pin;

struct SelfRef {
    data: String,
    ptr: *const String,  // Points to data
}

// Pin prevents moving, keeping ptr valid
let pinned: Pin<Box<SelfRef>> = Box::pin(SelfRef::new());
```

**What RustyCpp Does:**
No Pin support. Self-referential structs are not specially handled.

**Implementation Complexity:** Medium - requires understanding move semantics deeply.

---

## Summary: Priority for Implementation

| Feature | Impact | Complexity | Priority |
|---------|--------|------------|----------|
| **Non-Lexical Lifetimes** | High | High | **1 - Critical** |
| **Reborrowing** | Medium | Medium | **2 - Useful** |
| **Thread Safety Static Analysis** | High | Very High | 3 - Has runtime support |
| **Two-Phase Borrows** | Low | Medium | 4 - Nice to have |
| **Constructor Init Order** | Low | Medium | 5 - Nice to have |
| **Variance** | Low | High | 6 - Advanced |
| **Pin** | Low | Medium | 7 - Specialized |
| ~~Partial Moves~~ | ~~Medium~~ | ~~Low~~ | ✅ **Complete** |
| ~~Partial Borrows~~ | ~~Medium~~ | ~~Medium~~ | ✅ **Complete** |

**Note on Thread Safety:** RustyCpp already has runtime Send/Sync trait support via C++ templates (`include/rusty/traits.hpp`). The channel implementation (`mpsc_lockfree.hpp`) enforces Send at compile time. Static analysis of arbitrary C++ thread code would be valuable but is very complex.

## Recommendations

### For Users

1. **Use explicit scopes** to work around lack of NLL
2. **Partial moves/borrows work** - you can borrow/move different fields independently
3. **Use `@unsafe` blocks** for patterns RustyCpp can't verify (like reborrowing)
4. **Don't rely on RustyCpp for thread safety** - use other tools or manual review

### For Contributors

1. **NLL is the highest-value improvement** - would dramatically reduce false positives
2. **Reborrowing** would improve ergonomics for reference-heavy code
3. **Thread safety** would be valuable but is a major undertaking
4. **Document workarounds** for missing features

## Related Documentation

- [RAII_TRACKING.md](RAII_TRACKING.md) - RAII tracking implementation details
- [../CLAUDE.md](../CLAUDE.md) - Full project documentation
- [annotation_reference.md](annotation_reference.md) - Safety annotation syntax
