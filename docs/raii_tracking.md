# RAII Tracking in RustyCpp

This document describes the RAII (Resource Acquisition Is Initialization) tracking capabilities in RustyCpp, which helps detect memory safety issues related to object lifetimes and resource management.

## Overview

RAII tracking extends RustyCpp's borrow checking to handle C++-specific patterns where object lifetimes and resource ownership can lead to dangling references, use-after-free, and other memory safety issues.

## Supported Features

### 1. Reference/Pointer Stored in Container

Detects when a pointer or reference is stored in a container that outlives the pointee.

```cpp
// @safe
void bad_pointer_in_container() {
    std::vector<int*> vec;
    {
        int x = 42;
        vec.push_back(&x);  // Store pointer to x
    }  // x destroyed here

    // ERROR: vec[0] is now a dangling pointer
    *vec[0] = 10;
}
```

**What's detected:**
- `push_back`, `push_front`, `insert`, `emplace`, `emplace_back`, `emplace_front`, `assign` with pointer/reference arguments
- Container types: `vector`, `list`, `deque`, `set`, `map`, `unordered_*`, `array`, `span`

### 2. User-Defined RAII Types

Recognizes classes with user-defined destructors as RAII types, enabling proper lifetime tracking.

```cpp
class FileHandle {
    FILE* f;
public:
    FileHandle(const char* path) : f(fopen(path, "r")) {}
    ~FileHandle() { if (f) fclose(f); }  // User-defined destructor
};

// RustyCpp now tracks FileHandle as an RAII type
```

**What's detected:**
- Classes with explicit destructors (`~ClassName()`)
- Destructor presence propagated to IR for lifetime analysis

### 3. Iterator Outlives Container

Detects when an iterator survives longer than its source container.

```cpp
// @safe
void bad_iterator_outlives_container() {
    std::vector<int>::iterator it;
    {
        std::vector<int> v = {1, 2, 3};
        it = v.begin();  // it borrows from v
    }  // v destroyed here

    // ERROR: it is now invalid
    int val = *it;
}
```

**What's detected:**
- Iterator-returning methods: `begin`, `end`, `cbegin`, `cend`, `rbegin`, `rend`, `find`, `lower_bound`, `upper_bound`
- Iterator types: `*::iterator`, `*::const_iterator`, `*::reverse_iterator`

### 4. Lambda Escape Analysis

Detects when lambdas with reference captures escape their scope, potentially creating dangling references.

```cpp
// @safe
std::function<int()> bad_lambda_escape() {
    int x = 42;
    auto lambda = [&x]() { return x; };  // Captures x by reference
    return lambda;  // ERROR: lambda escapes, x will be destroyed
}

// @safe
void good_lambda_local_use() {
    int x = 42;
    auto lambda = [&x]() { return x; };  // OK: lambda doesn't escape
    int result = lambda();  // Used locally, x still alive
}
```

**What's detected:**
- Lambdas returned from functions
- Lambdas stored in containers via `push_back`, `insert`, etc.
- `this` capture is always forbidden (raw pointer)

**What's allowed:**
- Reference captures in non-escaping lambdas
- Copy captures (`[x]`, `[=]`) always allowed
- Move captures (`[x = std::move(y)]`) always allowed

### 5. Member Lifetime Tracking

Detects when references to object members outlive the containing object.

```cpp
// @safe
void bad_member_reference() {
    const std::string* ptr;
    {
        struct Wrapper { std::string data; };
        Wrapper w;
        w.data = "hello";
        ptr = &w.data;  // ptr references w.data
    }  // w destroyed, w.data destroyed

    // ERROR: ptr is now dangling
    std::cout << *ptr;
}
```

**What's detected:**
- `&obj.field` expressions where the reference outlives `obj`
- Field borrows tracked through `BorrowField` IR statements

### 6. new/delete Tracking

Detects double-free and use-after-free with raw heap allocations.

```cpp
// @safe
void bad_double_free() {
    int* ptr = new int(42);
    delete ptr;
    delete ptr;  // ERROR: double free
}

// @safe
void bad_use_after_free() {
    int* ptr = new int(42);
    delete ptr;
    *ptr = 10;  // ERROR: use after free
}
```

**What's detected:**
- `operator new` / `operator delete` calls
- Double-free (delete already-freed pointer)
- Use-after-free (access freed memory)

## Not Yet Implemented

### 7. Constructor Initialization Order

**Status:** Not Implemented (LOW priority)

Would detect when member initializers reference uninitialized members.

```cpp
class Bad {
    const int& ref;  // Declared first
    int value;       // Declared second

    // ref uses value before it's initialized!
    Bad() : ref(value), value(42) {}
};
```

**Why not implemented:**
- Requires parser changes to extract member initializer lists
- Relatively rare bug pattern

## Comparison with Rust

### What RustyCpp Has

| Feature | Rust | RustyCpp | Notes |
|---------|------|----------|-------|
| Move detection | ✅ | ✅ | `std::move()` tracked |
| Use-after-move | ✅ | ✅ | Full detection |
| Multiple immutable borrows | ✅ | ✅ | Allowed |
| Single mutable borrow | ✅ | ✅ | Enforced |
| Mutable + immutable conflict | ✅ | ✅ | Detected |
| Transitive borrow tracking | ✅ | ✅ | Chain detection |
| Scope-based borrow cleanup | ✅ | ✅ | Working |
| Loop iteration analysis | ✅ | ✅ | 2-iteration simulation |
| Conditional path analysis | ✅ | ✅ | Path-sensitive |
| Reassignment after move | ✅ | ✅ | Variable valid again |
| Return reference to local | ✅ | ✅ | Detected |
| Return reference to temp | ✅ | ✅ | Detected |
| Lifetime annotations | ✅ | ✅ | `@lifetime` syntax |
| Iterator outlives container | ✅ | ✅ | Detected |
| Reference in container | ✅ | ✅ | Detected |
| User-defined RAII | N/A | ✅ | C++ specific |
| Lambda escape analysis | ✅ | ✅ | Refined implementation |

### What RustyCpp Is Missing

| Feature | Rust | RustyCpp | Notes |
|---------|------|----------|-------|
| Non-Lexical Lifetimes (NLL) | ✅ | ❌ | Would reduce false positives |
| Constructor init order | N/A | ❌ | C++ specific, low priority |

### Key Architectural Differences

1. **Rust's NLL (Non-Lexical Lifetimes):** Borrows end at last use, not scope end. RustyCpp uses scope-based tracking, which can produce false positives.

2. **Rust's Drop Check:** Rust understands exactly when destructors run and validates no references outlive their referents through drop analysis. RustyCpp has `ImplicitDrop` but limited semantic understanding.

3. **Rust's Ownership System:** Built into the type system. RustyCpp retrofits ownership tracking onto C++, which has different semantics (copy by default, explicit move).

4. **Type System Integration:** Rust's borrow checker is integrated with the type system. RustyCpp is a separate static analyzer that can't modify C++ compilation.

## Usage

RAII tracking is automatically enabled when analyzing `@safe` functions:

```cpp
// @safe
void my_function() {
    // RAII issues will be detected here
}
```

Run the checker:
```bash
cargo run -- your_file.cpp
```

Example output:
```
✗ Found 2 violation(s):
Dangling pointer in container: 'vec' stored pointer to 'x' which goes out of scope
Iterator outlives container: 'it' borrows from 'vec' which goes out of scope
```

## Test Coverage

The RAII tracking module has comprehensive test coverage:

- **22 unit/integration tests** in `tests/raii_integration_tests.rs`
- **Test files** in `tests/raii/`:
  - `return_ref_to_local.cpp`
  - `iterator_outlives_container.cpp`
  - `user_defined_raii.cpp`
  - `double_free.cpp`
  - `lambda_capture_escape.cpp`

Run RAII-specific tests:
```bash
cargo test raii
```

## Limitations

1. **No whole-program analysis**: Each translation unit is analyzed independently
2. **Conservative assumptions**: Some safe patterns may be flagged as errors
3. **Template instantiation**: Templates analyzed at declaration, not instantiation
4. **External code**: Functions from external libraries are not analyzed internally
5. **Constructor init order**: Not yet implemented

## Future Work

- Implement Constructor Initialization Order checking
- Improve escape analysis for lambdas passed to functions
- Better tracking of iterator invalidation from container modifications (not just destruction)
- Support for custom container types via annotations
- Non-Lexical Lifetimes (NLL) for reduced false positives
