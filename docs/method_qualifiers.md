# Method Qualifiers and Rust-like Self Tracking

This document describes how RustyCpp enforces Rust's ownership and borrowing rules at the method level using C++ method qualifiers (`const`, non-const, and `&&`).

## Table of Contents

1. [Overview](#overview)
2. [Rust's Self Types](#rusts-self-types)
3. [C++ to Rust Mapping](#c-to-rust-mapping)
4. [Rules Enforced](#rules-enforced)
5. [Design Principles](#design-principles)
6. [Examples](#examples)
7. [Common Patterns](#common-patterns)
8. [Error Messages](#error-messages)
9. [Best Practices](#best-practices)

---

## Overview

RustyCpp enforces Rust's `self`, `&self`, and `&mut self` semantics using C++'s method qualifiers:

| C++ Method Qualifier | Rust Equivalent | Meaning |
|---------------------|-----------------|---------|
| `const`             | `&self`         | Shared immutable access |
| non-const           | `&mut self`     | Exclusive mutable access |
| `&&` (rvalue ref)   | `self`          | Consuming/ownership transfer |

**Key Insight**: The method qualifier determines what operations are allowed on the object's fields, enforcing Rust's ownership rules at compile time.

---

## Rust's Self Types

In Rust, methods can receive `self` in three forms:

### `&self` - Shared Immutable Borrow
```rust
impl MyStruct {
    fn read(&self) -> &i32 {
        &self.value  // ✅ Can read fields
        // self.value = 42;  // ❌ Cannot modify
        // std::mem::replace(&mut self.value, 0)  // ❌ Cannot move
    }
}
```
**Rules:**
- Can read fields
- Cannot modify fields
- Cannot move fields
- Can create immutable references to fields
- Cannot create mutable references to fields

### `&mut self` - Exclusive Mutable Borrow
```rust
impl MyStruct {
    fn modify(&mut self, val: i32) {
        self.value = val;  // ✅ Can modify
        let r = &mut self.value;  // ✅ Can borrow mutably
        // let x = std::mem::replace(&mut self.value, 0);  // ❌ Cannot move!
    }
}
```
**Rules:**
- Can read fields
- Can modify fields
- **Cannot move fields** (critical restriction!)
- Can create both mutable and immutable references to fields

**Why can't `&mut self` move?** Because `&mut self` is a *borrow*, not ownership. The caller still owns the object and expects it to remain valid.

### `self` - Consuming/Ownership
```rust
impl MyStruct {
    fn consume(self) -> i32 {
        self.value  // ✅ Can move - we own it
    }
}
```
**Rules:**
- Full ownership
- Can do anything (read, modify, move)
- Object is consumed and cannot be used afterward

---

## C++ to Rust Mapping

### Const Methods → `&self`

```cpp
// @safe
class Container {
    int value;
    std::unique_ptr<int> data;

public:
    int read() const {
        return value;  // ✅ OK: Reading is allowed
    }

    void modify() const {
        value = 42;  // ❌ ERROR: Cannot modify in const method
    }

    std::unique_ptr<int> take() const {
        return std::move(data);  // ❌ ERROR: Cannot move in const method
    }

    int& get_mut_ref() const {
        return value;  // ❌ ERROR: Cannot create mutable reference in const method
    }

    const int& get_ref() const {
        return value;  // ✅ OK: Immutable reference is allowed
    }
};
```

### Non-const Methods → `&mut self`

```cpp
// @safe
class Container {
    int value;
    std::unique_ptr<int> data;

public:
    void modify(int val) {
        value = val;  // ✅ OK: Can modify
    }

    int& get_mut_ref() {
        return value;  // ✅ OK: Can create mutable reference
    }

    const int& get_ref() {
        return value;  // ✅ OK: Can downgrade to immutable reference
    }

    void take() {
        auto temp = std::move(data);  // ❌ ERROR: Cannot move field from &mut self!
    }
};
```

**Key Rule**: Non-const methods can modify but **cannot move** fields. Use `&&` qualifier for consuming methods.

### Rvalue Reference Methods → `self`

```cpp
// @safe
class Container {
    std::unique_ptr<int> data;

public:
    std::unique_ptr<int> consume() && {
        return std::move(data);  // ✅ OK: && method has ownership
    }
};

// Usage:
Container c;
auto result = std::move(c).consume();  // OK: c is moved into consume()
// c.consume();  // ERROR: consume() requires rvalue
```

---

## Rules Enforced

### Field Access Rules

| Operation | `const` (`&self`) | non-const (`&mut self`) | `&&` (`self`) |
|-----------|-------------------|-------------------------|---------------|
| Read field | ✅ Yes | ✅ Yes | ✅ Yes |
| Write field | ❌ No | ✅ Yes | ✅ Yes |
| Move field | ❌ No | ❌ No | ✅ Yes |
| Borrow field immutably | ✅ Yes | ✅ Yes | ✅ Yes |
| Borrow field mutably | ❌ No | ✅ Yes | ✅ Yes |

### Field Borrow Rules

All standard Rust borrow checking rules apply to field borrows:

1. **Multiple immutable borrows allowed**:
   ```cpp
   // @safe
   class Example {
       int value;
   public:
       void read_multiple() const {
           const int& ref1 = value;  // ✅ OK
           const int& ref2 = value;  // ✅ OK: Multiple immutable borrows
       }
   };
   ```

2. **No double mutable borrow**:
   ```cpp
   // @safe
   class Example {
       int value;
   public:
       void double_mut() {
           int& ref1 = value;  // ✅ OK
           int& ref2 = value;  // ❌ ERROR: Already borrowed mutably
       }
   };
   ```

3. **No mutable + immutable conflict**:
   ```cpp
   // @safe
   class Example {
       int value;
   public:
       void mixed_borrows() {
           int& ref1 = value;        // ✅ OK
           const int& ref2 = value;  // ❌ ERROR: Already borrowed mutably
       }
   };
   ```

4. **No immutable + mutable conflict**:
   ```cpp
   // @safe
   class Example {
       int value;
   public:
       void mixed_borrows2() {
           const int& ref1 = value;  // ✅ OK
           int& ref2 = value;        // ❌ ERROR: Already borrowed immutably
       }
   };
   ```

---

## Design Principles

### 1. Method Qualifier = Permission Level

The method qualifier determines the **maximum permission** the method has over the object:

- `const` = Read-only access (shared borrow)
- non-const = Read-write access (exclusive mutable borrow)
- `&&` = Full ownership (can consume)

### 2. No Permission Escalation

A method cannot perform operations that require more permission than it has:

```cpp
// @safe
class Example {
public:
    void const_method() const {
        // Has: Read permission
        // Cannot: Write or move (would require &mut self or self)
    }

    void mut_method() {
        // Has: Read + Write permission
        // Cannot: Move (would require self ownership)
    }
};
```

### 3. Preventing Use-After-Move

The key insight: `&mut self` methods **borrow** the object; they don't own it. If they could move fields, the object would be left in a partially moved state, violating the borrow contract.

```cpp
// @safe
class Container {
    std::unique_ptr<int> data;

public:
    void bad_method() {
        auto temp = std::move(data);  // ❌ ERROR!
        // Now 'data' is empty, but caller still has Container
        // Caller expects Container to be fully valid
    }

    // Correct approach
    std::unique_ptr<int> consume() && {
        return std::move(data);  // ✅ OK: We own 'this'
        // After this call, 'this' is consumed and cannot be used
    }
};
```

### 4. Const Correctness ≠ Immutability

C++ `const` traditionally meant "I won't modify this object," but RustyCpp enforces it as Rust's `&self` - a **shared immutable borrow**:

- Traditional C++: `const` is advisory, can be cast away
- RustyCpp: `const` methods **cannot** create mutable references to fields

```cpp
// Traditional C++ might allow (via const_cast)
class Traditional {
    int value;
public:
    void bad_const() const {
        int& ref = const_cast<int&>(value);  // Bad practice
    }
};

// RustyCpp enforces at parse time
// @safe
class RustyCpp {
    int value;
public:
    void enforced_const() const {
        int& ref = value;  // ❌ ERROR: Cannot create mutable borrow in const method
    }
};
```

### 5. Explicit Ownership Transfer

To move an object or its fields, use rvalue reference qualifiers explicitly:

```cpp
// @safe
class ResourceHolder {
    std::unique_ptr<Resource> resource;

public:
    // ❌ Wrong: Trying to transfer ownership without consuming 'this'
    void release() {
        auto r = std::move(resource);  // ERROR!
    }

    // ✅ Correct: Explicit ownership transfer
    std::unique_ptr<Resource> into_resource() && {
        return std::move(resource);
    }
};

// Usage:
ResourceHolder h;
auto res = std::move(h).into_resource();  // h is consumed
```

---

## Examples

### Example 1: Builder Pattern

```cpp
// @safe
class Builder {
    std::string name;
    int value;

public:
    // Setters return &mut self (non-const reference)
    Builder& set_name(const std::string& n) {
        name = n;  // ✅ OK: Copy assignment
        return *this;
    }

    Builder& set_value(int v) {
        value = v;  // ✅ OK
        return *this;
    }

    // Build consumes the builder (self)
    Product build() && {
        return Product{std::move(name), value};  // ✅ OK: && method can move
    }
};

// Usage:
Product p = Builder{}
    .set_name("widget")
    .set_value(42)
    .build();  // Works because Builder{} is already an rvalue
```

### Example 2: Resource Management

```cpp
// @safe
class FileHandle {
    std::unique_ptr<FILE, decltype(&fclose)> file;

public:
    // Read operations - const (&self)
    std::string read_line() const {
        // ✅ OK: Reading doesn't modify the handle
        // Implementation...
    }

    // Write operations - non-const (&mut self)
    void write_line(const std::string& line) {
        // ✅ OK: Writing modifies file state
        // Implementation...
    }

    // Cannot close in non-const method
    void close() {
        file.reset();  // ❌ ERROR: Cannot move field from &mut self
    }

    // Correct: Close consumes the handle
    void close() && {
        file.reset();  // ✅ OK: && method has ownership
    }
};

// Usage:
FileHandle fh = open_file("data.txt");
fh.write_line("Hello");           // OK: &mut self
std::string line = fh.read_line(); // OK: &self
std::move(fh).close();             // OK: self consumed
// fh.read_line();                 // ERROR: fh was moved
```

### Example 3: Cache with Interior Mutability

```cpp
// @safe
class Cache {
    std::unordered_map<std::string, Value> cache;

public:
    // Lookup - const (&self)
    const Value* get(const std::string& key) const {
        auto it = cache.find(key);
        return it != cache.end() ? &it->second : nullptr;
    }

    // Insert - non-const (&mut self)
    void insert(const std::string& key, Value val) {
        cache[key] = std::move(val);  // ✅ OK: Can modify in &mut self
    }

    // Clear and return all values - consumes cache
    std::unordered_map<std::string, Value> drain() && {
        return std::move(cache);  // ✅ OK: && method can move
    }
};
```

### Example 4: Field Borrowing

```cpp
// @safe
class Container {
    int value;
    std::string name;

public:
    // Const method can create immutable references
    const int& get_value() const {
        return value;  // ✅ OK
    }

    // Const method cannot create mutable references
    int& get_value_mut() const {
        return value;  // ❌ ERROR: Cannot create mutable borrow in const method
    }

    // Non-const method can create mutable references
    int& get_value_mut() {
        return value;  // ✅ OK
    }

    // Multiple immutable borrows allowed
    void read_multiple() const {
        const int& ref1 = value;
        const std::string& ref2 = name;
        const int& ref3 = value;  // ✅ OK: Multiple immutable borrows
    }

    // Double mutable borrow not allowed
    void double_mut_borrow() {
        int& ref1 = value;
        int& ref2 = value;  // ❌ ERROR: Already borrowed mutably
    }
};
```

---

## Common Patterns

### Pattern 1: Into Methods (Consuming Conversion)

Methods that convert the object into something else should use `&&`:

```cpp
// @safe
class Optional {
    std::unique_ptr<T> value;

public:
    T unwrap() && {
        if (!value) panic();
        return std::move(*value);  // ✅ OK: && method
    }
};
```

### Pattern 2: Getters and Setters

```cpp
// @safe
class Data {
    int value;

public:
    // Getter (const)
    int get_value() const {
        return value;  // ✅ OK
    }

    // Setter (non-const)
    void set_value(int v) {
        value = v;  // ✅ OK
    }

    // Get mutable reference (non-const)
    int& value_mut() {
        return value;  // ✅ OK
    }
};
```

### Pattern 3: Swap and Replace

To replace a field's value while preserving the old value, use `&&`:

```cpp
// @safe
class Holder {
    std::unique_ptr<T> data;

public:
    // Cannot swap in &mut self
    void swap(Holder& other) {
        std::swap(data, other.data);  // ❌ ERROR: Moves fields
    }

    // Consume both and create new
    static std::pair<Holder, Holder> swap(Holder&& a, Holder&& b) {
        return {std::move(b), std::move(a)};
    }
};
```

### Pattern 4: Borrowing During Modification

```cpp
// @safe
class Buffer {
    std::vector<uint8_t> data;

public:
    // Borrow while modifying
    void process() {
        uint8_t& first = data[0];  // Borrow field element
        // data.push_back(0);       // ❌ ERROR: Cannot modify while borrowed
        first = 42;                 // ✅ OK: Can modify through borrow
    }
};
```

---

## Error Messages

### Cannot Move Field from &mut self

```cpp
// @safe
class Example {
    std::unique_ptr<T> data;
public:
    void take_data() {
        auto temp = std::move(data);
    }
};
```
**Error**: `Cannot move field 'data' from &mut self method (use && qualified method for self ownership)`

**Fix**: Use `&&` qualifier:
```cpp
// @safe
class Example {
    std::unique_ptr<T> data;
public:
    std::unique_ptr<T> take_data() && {
        return std::move(data);
    }
};
```

### Cannot Modify in Const Method

```cpp
// @safe
class Example {
    int value;
public:
    void modify() const {
        value = 42;
    }
};
```
**Error**: `Cannot modify field 'value' from const method (&self)`

**Fix**: Remove `const` qualifier:
```cpp
// @safe
class Example {
    int value;
public:
    void modify() {
        value = 42;
    }
};
```

### Cannot Create Mutable Borrow in Const Method

```cpp
// @safe
class Example {
    int value;
public:
    int& get_mut() const {
        return value;
    }
};
```
**Error**: `Cannot create mutable borrow of field 'value' in const method`

**Fix**: Remove `const` qualifier or return const reference:
```cpp
// @safe
class Example {
    int value;
public:
    int& get_mut() {
        return value;
    }

    // Or:
    const int& get() const {
        return value;
    }
};
```

### Already Borrowed Mutably

```cpp
// @safe
class Example {
    int value;
public:
    void double_mut() {
        int& ref1 = value;
        int& ref2 = value;
    }
};
```
**Error**: `Cannot borrow field 'value': already borrowed mutably`

**Fix**: Don't create overlapping mutable borrows:
```cpp
// @safe
class Example {
    int value;
public:
    void single_mut() {
        int& ref = value;
        // Use ref
    }  // ref goes out of scope before creating another
};
```

### Already Borrowed Immutably

```cpp
// @safe
class Example {
    int value;
public:
    void immut_then_mut() {
        const int& ref1 = value;
        int& ref2 = value;
    }
};
```
**Error**: `Cannot borrow field 'value' mutably: already borrowed immutably`

**Fix**: Don't mix mutable and immutable borrows:
```cpp
// @safe
class Example {
    int value;
public:
    void immut_only() {
        const int& ref1 = value;
        const int& ref2 = value;  // OK
    }
};
```

---

## Best Practices

### 1. Default to Const

Make methods `const` unless they need to modify the object:

```cpp
// @safe
// ✅ Good: Const by default
class Point {
    int x, y;

public:
    int get_x() const { return x; }

    void set_x(int val) { x = val; }  // Only this needs to be non-const
};
```

### 2. Use && for Consuming Operations

If a method consumes the object or transfers ownership, use `&&`:

```cpp
// @safe
class Example {
    Resource resource;
public:
    // ✅ Good: Clear ownership transfer
    Resource into_resource() && {
        return std::move(resource);
    }

    // ❌ Bad: Unclear if object is still valid
    Resource get_resource() {
        return std::move(resource);  // ERROR anyway!
    }
};
```

### 3. Prefer Returning by Value for Ownership

```cpp
// @safe
class Example {
    std::unique_ptr<T> data;
public:
    // ✅ Good: Clear ownership
    std::unique_ptr<T> take() && {
        return std::move(data);
    }

    // ❌ Bad: Unclear ownership
    std::unique_ptr<T>& get_mut() {
        return data;  // Returning reference to unique_ptr is risky
    }
};
```

### 4. Document Consuming Methods

```cpp
// @safe
class Builder {
    std::vector<Part> parts;
public:
    /// Consumes this builder and returns the built product.
    /// After calling this, the builder is no longer usable.
    Product build() && {
        return Product{std::move(parts)};
    }
};
```

### 5. Use Method Chaining Carefully

```cpp
// @safe
// ✅ Good: All intermediate calls are &mut self
class Builder {
    std::vector<Part> parts;
public:
    Builder& add_part(Part p) {
        parts.push_back(std::move(p));  // OK: moving parameter, not field
        return *this;
    }

    Product build() && {
        return Product{std::move(parts)};
    }
};

// Usage: Must use std::move() or create rvalue
Product p = std::move(builder).add_part(part1).build();
```

### 6. Avoid Partial Moves

```cpp
// @safe
// ❌ Bad: Partial move in &mut self
class BadContainer {
    std::unique_ptr<A> a;
    std::unique_ptr<B> b;

public:
    void reset_a() {
        a.reset();  // ERROR: Cannot move field
    }
};

// @safe
// ✅ Good: Consume entire object
class GoodContainer {
    std::unique_ptr<A> a;
    std::unique_ptr<B> b;

public:
    std::pair<std::unique_ptr<A>, std::unique_ptr<B>> take_all() && {
        return {std::move(a), std::move(b)};
    }
};
```

### 7. Leverage Rust-like Patterns

```cpp
// @safe
// ✅ Good: Rust-like Option pattern
class Option<T> {
    std::unique_ptr<T> value;

public:
    bool is_some() const { return value != nullptr; }

    const T& unwrap() const {
        if (!value) panic();
        return *value;
    }

    T unwrap() && {
        if (!value) panic();
        return std::move(*value);
    }

    T unwrap_or(T default_val) && {
        return value ? std::move(*value) : std::move(default_val);
    }
};
```

---

## Summary

RustyCpp enforces Rust's ownership rules through C++ method qualifiers:

- **`const` methods** (`&self`): Can read and create immutable borrows, cannot modify or move
- **Non-const methods** (`&mut self`): Can read, modify, and create any borrows, but **cannot move** fields
- **`&&` methods** (`self`): Full ownership, can do anything, consumes the object

This design:
- ✅ Prevents use-after-move at the field level
- ✅ Enforces clear ownership semantics
- ✅ Catches common C++ mistakes at compile time
- ✅ Aligns with modern C++ move semantics
- ✅ Provides Rust-level safety guarantees

By following these patterns, you can write C++ code that has the same safety guarantees as Rust!
