# Partial Moves and Borrows in RustyCpp

This document describes RustyCpp's support for partial moves and partial borrows, which allows fine-grained tracking of struct field ownership.

## Overview

Like Rust, RustyCpp can track moves and borrows at the individual field level rather than treating structs as atomic units. This enables patterns where different fields of the same struct can be moved or borrowed independently.

## Partial Moves

### Basic Partial Move

Move individual fields while keeping others accessible:

```cpp
struct Pair {
    std::string first;
    std::string second;
};

// @safe
void example() {
    Pair p{"hello", "world"};

    std::string a = std::move(p.first);   // Move only p.first
    std::string b = p.second;              // OK: p.second not moved
}
```

### Use-After-Move Detection

RustyCpp tracks which fields have been moved:

```cpp
// @safe
void bad_double_move() {
    Pair p{"hello", "world"};

    std::string a = std::move(p.first);
    std::string b = std::move(p.first);  // ERROR: field 'p.first' already moved
}
```

### Whole-Struct Move After Partial Move

Cannot move the entire struct after partial move:

```cpp
// @safe
void bad_whole_after_partial() {
    Pair p{"hello", "world"};

    std::string a = std::move(p.first);  // Partial move
    Pair q = std::move(p);  // ERROR: Cannot move 'p' because partially moved
}
```

### Nested Field Tracking

RustyCpp supports arbitrarily nested field paths:

```cpp
struct Inner { std::string data; };
struct Outer { Inner inner; };

// @safe
void nested_example() {
    Outer o;
    o.inner.data = "hello";

    std::string s = std::move(o.inner.data);  // Move nested field

    // ERROR: field 'o.inner.data' already moved
    std::string t = std::move(o.inner.data);
}
```

## Partial Borrows

### Borrowing Different Fields

Like Rust, you can borrow different fields mutably at the same time:

```cpp
// @safe
void borrow_different_fields() {
    Pair p{"hello", "world"};

    std::string& r1 = p.first;   // Mutable borrow of p.first
    std::string& r2 = p.second;  // OK: p.second is separate

    r1 = "hi";
    r2 = "there";
}
```

### Conflict Detection

Same field cannot be borrowed mutably twice:

```cpp
// @safe
void bad_double_mutable_borrow() {
    Pair p{"hello", "world"};

    std::string& r1 = p.first;
    std::string& r2 = p.first;  // ERROR: field 'p.first' already mutably borrowed
}
```

### Mixed Mutable/Immutable Conflicts

Mutable and immutable borrows of the same field conflict:

```cpp
// @safe
void bad_mixed_borrow() {
    Pair p{"hello", "world"};

    const std::string& r1 = p.first;  // Immutable borrow
    std::string& r2 = p.first;        // ERROR: cannot borrow mutably while immutably borrowed
}
```

### Multiple Immutable Borrows Allowed

Multiple immutable borrows of the same field are permitted:

```cpp
// @safe
void multiple_immutable() {
    Pair p{"hello", "world"};

    const std::string& r1 = p.first;
    const std::string& r2 = p.first;  // OK: multiple immutable borrows allowed
    const std::string& r3 = p.first;  // OK
}
```

### Whole-Struct vs Field Borrow Conflicts

Cannot borrow the whole struct while fields are borrowed, and vice versa:

```cpp
// @safe
void whole_vs_field() {
    Pair p{"hello", "world"};

    std::string& r = p.first;  // Borrow field
    Pair& q = p;               // ERROR: cannot borrow 'p' while 'p.first' is borrowed
}

// @safe
void field_vs_whole() {
    Pair p{"hello", "world"};

    Pair& q = p;               // Borrow whole struct
    std::string& r = p.first;  // ERROR: cannot borrow field while 'p' is borrowed
}
```

### Nested Field Borrows

Nested fields work the same way:

```cpp
struct Inner { std::string data; int count; };
struct Outer { Inner inner; std::string name; };

// @safe
void nested_borrows() {
    Outer o;

    std::string& r1 = o.inner.data;   // Borrow nested field
    int& r2 = o.inner.count;          // OK: different field
    std::string& r3 = o.name;         // OK: different top-level field

    std::string& r4 = o.inner.data;   // ERROR: already borrowed
}
```

## Comparison with Rust

RustyCpp's partial move/borrow behavior mirrors Rust:

**Rust:**
```rust
struct Pair { a: String, b: String }

fn example() {
    let mut p = Pair { a: "".into(), b: "".into() };
    let r_a = &mut p.a;  // Borrow only p.a
    let r_b = &mut p.b;  // OK - p.b is separate
    *r_a = "hello".into();
    *r_b = "world".into();
}
```

**C++ with RustyCpp:**
```cpp
struct Pair { std::string a; std::string b; };

// @safe
void example() {
    Pair p{"", ""};
    std::string& r_a = p.a;  // Borrow only p.a
    std::string& r_b = p.b;  // OK - p.b is separate
    r_a = "hello";
    r_b = "world";
}
```

## Feature Summary

| Feature | Status |
|---------|--------|
| Move individual field (`std::move(p.first)`) | ✅ |
| Use-after-move per field | ✅ |
| Use unmoved field after partial move | ✅ |
| Whole-struct move after partial move | ✅ Error detected |
| Nested field tracking (`p.inner.data`) | ✅ |
| Nested double-move detection | ✅ |
| Borrow different fields mutably | ✅ |
| Double mutable borrow of same field | ✅ Error detected |
| Mixed mutable/immutable borrow conflict | ✅ Error detected |
| Multiple immutable borrows allowed | ✅ |
| Whole-struct vs field borrow conflict | ✅ Error detected |
| Branch merging for field state | ✅ |

## Test Files

Test coverage in `tests/raii/`:
- `partial_move_test.cpp` - Basic partial move tests
- `partial_move_detailed_test.cpp` - Detailed feature tests
- `partial_move_whole_struct_test.cpp` - Whole-struct after partial move
- `partial_move_nested_fields.cpp` - Nested struct field moves
- `partial_borrow_test.cpp` - Field-level borrow conflicts
- `partial_borrow_nested_test.cpp` - Nested field borrows
- `partial_borrow_control_flow_test.cpp` - Control flow with partial borrows

Run partial move/borrow tests:
```bash
cargo test partial
cargo run -- tests/raii/partial_*.cpp
```
