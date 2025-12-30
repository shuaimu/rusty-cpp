# C++ Reference Semantics in RustyCpp

This document describes how RustyCpp applies Rust-like ownership and borrowing semantics to C++ references, bridging the fundamental differences between C++ and Rust reference models.

## Table of Contents

1. [The Fundamental Difference](#the-fundamental-difference)
2. [Reference Assignment Semantics](#reference-assignment-semantics)
3. [rusty::move - Rust-like Move for References](#rustymove---rust-like-move-for-references)
4. [rusty::copy - Explicit Copies](#rustycopy---explicit-copies)
5. [std::move Restrictions in @safe Code](#stdmove-restrictions-in-safe-code)
6. [Summary Tables](#summary-tables)
7. [Header File Reference](#header-file-reference)

---

## The Fundamental Difference

### C++ References: Aliases

In C++, references are **aliases** - they are not first-class types. A reference is just another name for an existing object:

```cpp
int x = 42;
int& ref = x;      // ref is an alias for x
int y = ref;       // Same as: int y = x;
ref = 100;         // Same as: x = 100;
```

Key implications:
- You cannot "move" a reference - `std::move(ref)` moves the underlying object `x`
- References cannot be null or reseated
- References don't have their own identity separate from what they reference

### Rust References: First-Class Types

In Rust, references are **first-class types** with their own ownership semantics:

```rust
let x = 42;
let r1: &mut i32 = &mut x;  // r1 is a mutable reference
let r2 = r1;                 // r1 is MOVED to r2, r1 is now invalid
// println!("{}", r1);       // ERROR: borrow of moved value
println!("{}", r2);          // OK: r2 owns the reference now
```

Key implications:
- `&mut T` (mutable reference) is **not Copy** - assigning moves it
- `&T` (immutable reference) **is Copy** - assigning copies it
- References have their own lifetime and can be invalidated

### RustyCpp's Approach

RustyCpp bridges this gap by:
1. **Tracking reference variables** as first-class entities
2. **Applying Rust semantics** to reference assignments at analysis time
3. **Providing `rusty::move`** for explicit Rust-like reference moves
4. **Forbidding `std::move` on references** in `@safe` code

---

## Reference Assignment Semantics

RustyCpp automatically applies Rust-like semantics to reference assignments:

| C++ Reference Type | Rust Equivalent | Assignment Behavior |
|--------------------|-----------------|---------------------|
| `T&` (non-const) | `&mut T` | **Move** - original becomes invalid |
| `const T&` | `&T` | **Copy** - both remain valid |
| `T&&` (named) | `&mut T` | **Move** - original becomes invalid |

### Mutable References Move

When you assign a mutable reference to another, the original is invalidated:

```cpp
// @safe
void example() {
    int x = 42;
    int& r1 = x;      // r1 borrows x mutably
    int& r2 = r1;     // r1 is MOVED to r2 (Rust-like)

    int y = r2;       // OK: r2 is valid
    int z = r1;       // ERROR: r1 has been moved
}
```

This matches Rust's behavior:
```rust
fn example() {
    let mut x = 42;
    let r1: &mut i32 = &mut x;
    let r2 = r1;      // r1 moved to r2

    let y = *r2;      // OK
    let z = *r1;      // ERROR: borrow of moved value
}
```

### Const References Copy

Const references can be freely copied - all copies remain valid:

```cpp
// @safe
void example() {
    int x = 42;
    const int& r1 = x;    // r1 borrows x immutably
    const int& r2 = r1;   // r1 is COPIED to r2
    const int& r3 = r1;   // r1 can be copied multiple times

    int a = r1;           // OK: r1 is still valid
    int b = r2;           // OK: r2 is valid
    int c = r3;           // OK: r3 is valid
}
```

This matches Rust's behavior where `&T` is `Copy`:
```rust
fn example() {
    let x = 42;
    let r1: &i32 = &x;
    let r2 = r1;          // Copy, not move
    let r3 = r1;          // Can copy multiple times

    println!("{} {} {}", r1, r2, r3);  // All valid
}
```

### Chained Moves

Mutable reference chains properly track moves:

```cpp
// @safe
void example() {
    int x = 42;
    int& r1 = x;
    int& r2 = r1;     // r1 moved
    int& r3 = r2;     // r2 moved
    int& r4 = r3;     // r3 moved

    int y = r4;       // OK: only r4 is valid
    int z = r1;       // ERROR: r1 was moved
    int w = r2;       // ERROR: r2 was moved
    int v = r3;       // ERROR: r3 was moved
}
```

---

## rusty::move - Rust-like Move for References

`rusty::move` is a drop-in replacement for `std::move` that provides Rust-like semantics for reference handling.

### Include

```cpp
#include <rusty/move.hpp>
```

Or via the umbrella header:
```cpp
#include <rusty/rusty.hpp>
```

### Behavior

| Type | std::move | rusty::move |
|------|-----------|-------------|
| Value `T` | Moves the value | Moves the value (same) |
| Mutable ref `T&` | Moves underlying object | Moves the **reference** |
| Rvalue ref `T&&` | Moves underlying object | Moves the **reference** |
| Const ref `const T&` | Returns rvalue ref | **Compile error** |

### Moving Values (Same as std::move)

```cpp
#include <rusty/move.hpp>

// @safe
void example() {
    std::unique_ptr<int> p1(new int(42));
    std::unique_ptr<int> p2 = rusty::move(p1);  // p1 is moved
    // use(*p1);  // ERROR: use after move
    use(*p2);     // OK
}
```

### Moving Mutable References

```cpp
#include <rusty/move.hpp>

// @safe
void example() {
    int x = 42;
    int& r1 = x;
    int& r2 = rusty::move(r1);  // r1 is now invalid

    // int y = r1;  // ERROR: use after move
    int z = r2;     // OK: r2 is valid
}
```

### Const References: Compile Error

`rusty::move` on const references produces a compile-time error:

```cpp
// @safe
void example() {
    int x = 42;
    const int& r1 = x;
    // const int& r2 = rusty::move(r1);  // COMPILE ERROR!
}
```

Error message:
```
Cannot rusty::move a const reference (const T&).
Const references are like Rust's &T which is Copy.
Use assignment (=) to copy const references instead.
```

Use plain assignment for const refs:
```cpp
const int& r2 = r1;  // Just copy it
```

### Named Rvalue References

Named rvalue references (`T&&`) are lvalues in C++, so they follow mutable reference rules:

```cpp
// @safe
void forward(int&& rr) {
    int&& r2 = rusty::move(rr);  // rr is now invalid
    // use(rr);  // ERROR: use after move
    use(r2);     // OK
}
```

---

## rusty::copy - Explicit Copies

When moves are the norm, use `rusty::copy` to make copies explicit:

```cpp
#include <rusty/move.hpp>

// @safe
void example() {
    int x = 42;
    int y = rusty::copy(x);  // Explicit copy

    use(x);  // OK: x is still valid
    use(y);  // OK: y is a copy
}
```

This is purely for documentation/clarity - it's equivalent to:
```cpp
int y = x;  // Implicit copy
```

---

## std::move Restrictions in @safe Code

In `@safe` code, using `std::move` on references is **forbidden** because it has confusing semantics - it moves the underlying object, not the reference.

### The Problem

```cpp
void problematic() {
    std::unique_ptr<int> ptr(new int(42));
    std::unique_ptr<int>& ref = ptr;

    // std::move(ref) moves *ptr*, not ref
    // ref still looks valid but points to a moved-from object
    std::unique_ptr<int> ptr2 = std::move(ref);

    // This compiles but ref now points to moved-from ptr!
    // No compile error, but undefined behavior territory
}
```

### The Rule

| Expression | In @safe code |
|------------|---------------|
| `std::move(value)` | Allowed |
| `std::move(reference)` | **Forbidden** |
| `std::move(rvalue_ref_param)` | **Forbidden** |
| `rusty::move(value)` | Allowed |
| `rusty::move(reference)` | Allowed |

### Error Message

```
std::move on reference 'ref' at line 5:
In @safe code, std::move on references is forbidden because it moves
the underlying object, not the reference. Use rusty::move for Rust-like
reference semantics, or use @unsafe block if you need C++ behavior.
```

### Using @unsafe for C++ Semantics

If you need standard C++ behavior:

```cpp
// @safe
void example() {
    // @unsafe
    {
        std::unique_ptr<int>& ref = get_ref();
        auto ptr2 = std::move(ref);  // OK in unsafe block
    }
}
```

---

## Summary Tables

### Reference Assignment Summary

| Source Type | Target Type | Behavior | Source After |
|-------------|-------------|----------|--------------|
| `T&` | `T&` | Move | Invalid |
| `T&` | `const T&` | Move | Invalid |
| `const T&` | `const T&` | Copy | Valid |
| `T&&` (named) | `T&&` | Move | Invalid |

### Move Function Summary

| Function | On Values | On Mutable Refs | On Const Refs |
|----------|-----------|-----------------|---------------|
| `std::move` | Moves value | Moves object (confusing!) | Returns rvalue ref |
| `rusty::move` | Moves value | Moves reference | Compile error |

### Safety Summary

| Operation | In @safe | In @unsafe |
|-----------|----------|------------|
| `std::move(value)` | Allowed | Allowed |
| `std::move(reference)` | Forbidden | Allowed |
| `rusty::move(value)` | Allowed | Allowed |
| `rusty::move(reference)` | Allowed | Allowed |
| Mutable ref assignment | Auto-move | Auto-move |
| Const ref assignment | Auto-copy | Auto-copy |

---

## Header File Reference

### rusty/move.hpp

```cpp
#pragma once
#include <type_traits>
#include <utility>

namespace rusty {

// Rust-like move: works on values and mutable references
// Compile error on const references
template<typename T>
constexpr std::remove_reference_t<T>&& move(T&& t) noexcept;

// Explicit copy for clarity
template<typename T>
constexpr T copy(const T& t) noexcept(std::is_nothrow_copy_constructible_v<T>);

} // namespace rusty
```

### Runtime Behavior

Both functions have **zero runtime overhead**:
- `rusty::move` is identical to `std::move` at runtime
- `rusty::copy` is a simple copy construction
- All the safety benefits come from static analysis

---

## See Also

- [Rust's Move Semantics](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)
- [C++ Value Categories](https://en.cppreference.com/w/cpp/language/value_category)
- [RustyCpp Safety Annotations](annotations.md)
- [Borrow Checking](PHASE3_COMPLETE.md)
