# Inheritance Safety in RustyCpp

## Overview

**Core Principle:** Inheritance is `@unsafe` by default, except when inheriting from a pure `@interface`.

This mimics Rust's approach where:
- You cannot inherit from structs (data + behavior)
- You can only implement traits (pure behavior contracts)

Since C++ has no grammar for "pure interface", we introduce the `@interface` annotation.

## The `@interface` Annotation

### Basic Usage

```cpp
// @interface
class IDrawable {
public:
    virtual void draw() const = 0;
    virtual ~IDrawable() = default;
};

// @safe - OK: inheriting from @interface
class Circle : public IDrawable {
    int radius;
public:
    void draw() const override { /* ... */ }
};

// @safe - ERROR: inheriting from non-interface class
class ColoredCircle : public Circle {  // Circle is not an @interface!
    int color;
};
```

### What Qualifies as an `@interface`?

A class marked `@interface` must be validated to ensure it's a true interface:

| Requirement | Rationale |
|-------------|-----------|
| All methods are pure virtual (`= 0`) | No default implementation |
| No non-static data members | No state to slice |
| No non-virtual methods | All behavior is overridable |
| Virtual destructor (can be defaulted) | Safe polymorphic deletion |
| Can inherit from other `@interface` only | Interface composition |

```cpp
// @interface - VALID
class ISerializable {
public:
    virtual void serialize() const = 0;
    virtual void deserialize() = 0;
    virtual ~ISerializable() = default;
};

// @interface - INVALID: has data member
class IBadInterface {
    int cache;  // ERROR: interfaces cannot have data members
public:
    virtual void process() = 0;
};

// @interface - INVALID: has non-pure virtual method
class IBadInterface2 {
public:
    virtual void process() { }  // ERROR: must be pure virtual (= 0)
    virtual ~IBadInterface2() = default;
};
```

## Method Safety Annotations

Interface methods can be marked `@safe` or `@unsafe`. Implementations MUST follow the same safety contract.

**Two options for implementations:**

1. **Explicit annotation (recommended)** - Annotate the implementation method; checker validates it matches the interface
2. **Implicit inheritance** - Don't annotate; checker automatically applies the interface's safety level

```cpp
// @interface
class IProcessor {
public:
    // @safe
    virtual int process(int x) = 0;

    // @unsafe
    virtual void* allocate(size_t n) = 0;

    virtual ~IProcessor() = default;
};

// Option 1: Explicit annotations (recommended for clarity)
class ExplicitProcessor : public IProcessor {
public:
    // @safe - Explicit: checker verifies this matches interface
    int process(int x) override {
        return x * 2;
    }

    // @unsafe - Explicit: checker verifies this matches interface
    void* allocate(size_t n) override {
        return malloc(n);
    }
};

// Option 2: Implicit inheritance (less verbose)
class ImplicitProcessor : public IProcessor {
public:
    // No annotation: inherits @safe from interface
    int process(int x) override {
        return x * 2;
    }

    // No annotation: inherits @unsafe from interface
    void* allocate(size_t n) override {
        return malloc(n);
    }
};
```

### Key Rules

1. If implementation has explicit annotation, it MUST match the interface
2. If implementation has no annotation, it inherits from the interface
3. The checker validates that `@safe` implementations contain only safe code
4. Mismatch between explicit annotation and interface is an error

```cpp
// @interface
class IReader {
public:
    // @safe
    virtual int read() = 0;
    virtual ~IReader() = default;
};

// ERROR: Explicit annotation doesn't match interface
class MismatchReader : public IReader {
public:
    // @unsafe  <-- ERROR: interface requires @safe!
    int read() override { return 0; }
};
// Expected: "Method 'MismatchReader::read' annotated @unsafe but interface 'IReader' requires @safe"

// ERROR: Implicit @safe but body is unsafe
class UnsafeBodyReader : public IReader {
public:
    // No annotation: inherits @safe from interface
    int read() override {
        int* p = nullptr;
        return *p;  // ERROR: unsafe operation in @safe method
    }
};
// Expected: "Method 'UnsafeBodyReader::read' violates @safe contract from interface 'IReader'"
```

## Safety Rules Summary

### Inheritance Rules

| Scenario | Result |
|----------|--------|
| Class inherits from `@interface` | ✅ Allowed in @safe code |
| Class inherits from regular class | ❌ Error (use `@unsafe` block) |
| Class inherits from multiple `@interface`s | ✅ Allowed |
| `@interface` inherits from `@interface` | ✅ Allowed |
| `@interface` inherits from regular class | ❌ Error |

### Method Safety Contract Rules

| Implementation Annotation | Interface Annotation | Result |
|---------------------------|---------------------|--------|
| None (implicit) | `@safe` | ✅ Inherits `@safe`, body validated |
| None (implicit) | `@unsafe` | ✅ Inherits `@unsafe` |
| `@safe` (explicit) | `@safe` | ✅ Matches, body validated |
| `@unsafe` (explicit) | `@unsafe` | ✅ Matches |
| `@safe` (explicit) | `@unsafe` | ❌ Error: mismatch |
| `@unsafe` (explicit) | `@safe` | ❌ Error: mismatch |

## Comparison with Rust

| Rust | C++ with RustyCpp |
|------|-------------------|
| `trait Drawable { fn draw(&self); }` | `// @interface` + pure virtual class |
| `impl Drawable for Circle { ... }` | `class Circle : public IDrawable { ... }` |
| Cannot inherit from structs | Inheritance from non-interface is `@unsafe` |
| Trait objects: `&dyn Drawable` | Interface pointers: `IDrawable*` |
| `unsafe fn` in trait | `// @unsafe` on interface method |
| Safe trait methods must be implemented safely | `@safe` interface methods validated in implementations |
| Trait safety is part of the contract | Interface method safety propagates to implementations |

### Example Comparison

**Rust:**
```rust
trait Processor {
    fn process(&self, x: i32) -> i32;  // safe by default
    unsafe fn raw_alloc(&self, n: usize) -> *mut u8;  // unsafe
}

impl Processor for MyProcessor {
    fn process(&self, x: i32) -> i32 {
        x * 2  // Must be safe code
    }

    unsafe fn raw_alloc(&self, n: usize) -> *mut u8 {
        // Can use unsafe operations
        std::alloc::alloc(Layout::from_size_align_unchecked(n, 1))
    }
}
```

**C++ with RustyCpp:**
```cpp
// @interface
class IProcessor {
public:
    // @safe
    virtual int process(int x) = 0;  // implementations must be safe

    // @unsafe
    virtual void* rawAlloc(size_t n) = 0;  // implementations can be unsafe

    virtual ~IProcessor() = default;
};

class MyProcessor : public IProcessor {
public:
    int process(int x) override {  // inherits @safe, validated by checker
        return x * 2;
    }

    void* rawAlloc(size_t n) override {  // inherits @unsafe
        return malloc(n);
    }
};
```

## Future Extensions

1. **Default method implementations** - Allow `@interface` with default methods (like Rust trait defaults)
2. **Associated types** - Template-based associated type pattern
3. **Object safety** - Detect non-object-safe interface patterns

## References

- [Rust Traits vs Inheritance](https://users.rust-lang.org/t/rust-traits-vs-inheritance/121341)
- [C++ Interfaces (Abstract Classes)](https://en.cppreference.com/w/cpp/language/abstract_class)
- [Safe C++ Extensions Proposal](https://www.theregister.com/2024/09/16/safe_c_plusplus/)
