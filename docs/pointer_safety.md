# Pointer Safety in RustyCpp

RustyCpp follows Rust's model: **raw pointers are unsafe**. For safe pointer-like behavior, use `rusty::Ptr<T>` and `rusty::MutPtr<T>`.

## Design Philosophy

### Raw Pointers Are Unsafe (Like Rust)

In Rust, raw pointers (`*const T`, `*mut T`) are inherently unsafe - any dereference requires an `unsafe` block. RustyCpp adopts the same approach for C++ raw pointers:

| Operation | Raw Pointer (`T*`) | `rusty::Ptr<T>` |
|-----------|-------------------|-----------------|
| Declaration | Requires @unsafe | Safe |
| Address-of (`&x`) | Requires @unsafe | Safe via `addr_of()` |
| Dereference (`*p`) | Requires @unsafe | Safe |
| Arithmetic | Requires @unsafe | Safe via `offset()` |

### Why Make Raw Pointers Unsafe?

#### 1. C++ Pointers Have No Safety Guarantees

Raw C++ pointers can be:
- Null (undefined behavior on dereference)
- Dangling (pointing to freed memory)
- Uninitialized (garbage value)
- Out of bounds (buffer overflow)

```cpp
int* ptr;           // Uninitialized - garbage value
int* ptr = nullptr; // Null - UB on dereference
int* ptr = &x;      // x might go out of scope later
ptr[100];           // No bounds checking
```

#### 2. Clear Safety Boundary

By making raw pointers unsafe, we create a clear boundary:
- `@safe` code: No raw pointer operations
- `@unsafe` code: Full pointer access with programmer responsibility

This forces developers to either:
1. Use safe alternatives (`rusty::Ptr<T>`, references, smart pointers)
2. Explicitly mark code as `@unsafe` and take responsibility

#### 3. Gradual Migration Path

Legacy code using raw pointers can be:
1. Left as `@unsafe` (works immediately)
2. Gradually migrated to `rusty::Ptr<T>` for safety
3. Refactored to use references where possible

## Safe Pointer Types: `rusty::Ptr<T>` and `rusty::MutPtr<T>`

RustyCpp provides safe pointer wrappers that can be used in `@safe` code:

```cpp
#include <rusty/ptr.hpp>

// @safe
void example() {
    int x = 42;

    // Safe pointer creation and use
    rusty::Ptr<int> p = rusty::addr_of(x);      // Immutable pointer
    rusty::MutPtr<int> mp = rusty::addr_of_mut(x); // Mutable pointer

    int y = *p;    // Safe dereference
    *mp = 100;     // Safe write
}
```

### Why Are `Ptr<T>`/`MutPtr<T>` Safe?

These types enforce safety invariants:

1. **Non-null by construction**: Created from valid references via `addr_of()`/`addr_of_mut()`
2. **Lifetime tracked**: The analyzer tracks the source lifetime
3. **Borrow checked**: Mutable/immutable access rules enforced
4. **No implicit conversions**: Cannot accidentally create from raw pointers in @safe code

### API Reference

```cpp
namespace rusty {

// Type aliases (underlying representation)
template<typename T> using Ptr = const T*;    // Immutable pointer
template<typename T> using MutPtr = T*;       // Mutable pointer

// Safe creation from references
// @safe
template<typename T>
constexpr Ptr<T> addr_of(const T& value);

// @safe
template<typename T>
constexpr MutPtr<T> addr_of_mut(T& value);

// Safe conversion (const promotion)
// @safe
template<typename T>
constexpr Ptr<T> as_const(MutPtr<T> ptr);

// Unsafe conversion (const cast)
// @unsafe - casting away const is dangerous
template<typename T>
constexpr MutPtr<T> as_mut(Ptr<T> ptr);

// Safe pointer arithmetic
// @safe
template<typename T>
constexpr Ptr<T> offset(Ptr<T> ptr, std::ptrdiff_t count);

// @safe
template<typename T>
constexpr MutPtr<T> offset_mut(MutPtr<T> ptr, std::ptrdiff_t count);

// Null pointer constants (for @unsafe code only)
template<typename T> constexpr Ptr<T> null_ptr = nullptr;
template<typename T> constexpr MutPtr<T> null_mut_ptr = nullptr;

}
```

## Rules for Raw Pointers in @safe Code

### Rule 1: No Raw Pointer Declarations

```cpp
// @safe
void bad() {
    int* ptr;              // ERROR: Raw pointer declaration
    const int* cptr;       // ERROR: Raw pointer declaration
}
```

### Rule 2: No Address-of Operator

```cpp
// @safe
void bad() {
    int x = 42;
    int* ptr = &x;         // ERROR: Address-of creates raw pointer
}

// Use rusty::addr_of() instead
// @safe
void good() {
    int x = 42;
    rusty::Ptr<int> p = rusty::addr_of(x);  // OK
}
```

### Rule 3: No Raw Pointer Dereference

```cpp
// @safe
void bad(int* ptr) {       // ERROR: Raw pointer parameter
    int x = *ptr;          // ERROR: Raw pointer dereference
}

// Use rusty::Ptr<T> instead
// @safe
void good(rusty::Ptr<int> ptr) {  // OK: Safe pointer parameter
    int x = *ptr;                  // OK: Safe dereference
}
```

### Rule 4: No Pointer Arithmetic on Raw Pointers

```cpp
// @safe
void bad(int* arr, size_t len) {
    int* end = arr + len;  // ERROR: Pointer arithmetic
    arr[5];                // ERROR: Subscript is pointer arithmetic
}

// Use rusty::offset() instead
// @safe
void good(rusty::MutPtr<int> arr, size_t len) {
    rusty::MutPtr<int> end = rusty::offset_mut(arr, len);  // OK
}
```

## Working with Raw Pointers

### Using @unsafe Blocks

When you need raw pointer operations, use `@unsafe` blocks:

```cpp
// @safe
void process_c_array(int* data, size_t len) {
    // @unsafe
    {
        for (size_t i = 0; i < len; i++) {
            data[i] *= 2;  // OK: In @unsafe block
        }
    }
}
```

### Interfacing with C APIs

C APIs typically use raw pointers. Wrap them in @unsafe:

```cpp
// @safe
void read_file(const char* filename) {
    // @unsafe
    {
        FILE* f = fopen(filename, "r");
        if (f) {
            // ... use f ...
            fclose(f);
        }
    }
}
```

### Converting Between Safe and Raw Pointers

```cpp
// @safe
void example() {
    int x = 42;
    rusty::MutPtr<int> safe_ptr = rusty::addr_of_mut(x);

    // @unsafe
    {
        // Safe -> Raw: Implicit (Ptr<T> is just const T*)
        int* raw = safe_ptr;

        // Raw -> Safe: Must go through addr_of
        // (Cannot directly assign raw pointer to Ptr<T> in @safe)
    }
}
```

## Pointer Members in Structs

### Raw Pointer Members Require @unsafe

```cpp
// @safe
struct Bad {
    int* ptr;  // ERROR: Raw pointer member in @safe struct
};

// @unsafe
struct RawWrapper {
    int* ptr;  // OK: Struct is @unsafe
};
```

### Safe Pointer Members Are Allowed

```cpp
// @safe
struct SafeWrapper {
    rusty::Ptr<int> ptr;  // OK: Safe pointer type

    SafeWrapper(int& value) : ptr(rusty::addr_of(value)) {}
};
```

### Smart Pointer Members (Always Safe)

```cpp
// @safe
struct Container {
    std::unique_ptr<int> owned;   // OK: Smart pointer
    std::shared_ptr<Data> shared; // OK: Smart pointer
};
```

## Comparison: Old vs New Model

| Aspect | Old Model (pointer_safety.md) | New Model |
|--------|------------------------------|-----------|
| Raw pointer decl | Allowed (must init) | Requires @unsafe |
| Address-of (`&x`) | Requires @unsafe | Requires @unsafe |
| Dereference (`*p`) | Safe (if valid) | Requires @unsafe |
| nullptr init | Forbidden | Requires @unsafe |
| Ptr<T>/MutPtr<T> | Type aliases only | Safe wrappers |
| Philosophy | "Pointers as valid refs" | "Raw unsafe, wrappers safe" |

## Summary

| Context | Raw `T*` | `rusty::Ptr<T>` | Reference `T&` |
|---------|----------|-----------------|----------------|
| @safe code | ❌ Forbidden | ✅ Safe | ✅ Safe |
| @unsafe code | ✅ Allowed | ✅ Allowed | ✅ Allowed |
| Null allowed | Yes (in @unsafe) | No | No |
| Borrow checked | No | Yes | Yes |

**Key principle: In @safe code, use `rusty::Ptr<T>`/`rusty::MutPtr<T>` instead of raw pointers. Raw pointers require `@unsafe`.**
