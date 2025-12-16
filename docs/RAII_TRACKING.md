# RAII Tracking in RustyCpp

This document describes the RAII (Resource Acquisition Is Initialization) tracking capabilities in RustyCpp, which helps detect memory safety issues related to object lifetimes and resource management.

## Overview

RAII tracking extends RustyCpp's borrow checking to handle C++-specific patterns where object lifetimes and resource ownership can lead to dangling references, use-after-free, and other memory safety issues.

## Supported Features

### 1. Reference/Pointer Stored in Container

**Status:** ✅ Fully Implemented

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

**Status:** ✅ Fully Implemented

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

**Status:** ✅ Fully Implemented

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

**Status:** ✅ Fully Implemented (Refined)

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

**Status:** ✅ Fully Implemented

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

**Status:** ✅ Fully Implemented

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

**Status:** ❌ Not Implemented (LOW priority)

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
- LOW priority in the implementation plan

## Architecture

### Key Components

1. **`RaiiTracker`** (`src/analysis/raii_tracking.rs`)
   - Main tracker coordinating all RAII-related analysis
   - Tracks container borrows, iterator borrows, member borrows, lambda captures, heap allocations
   - Scope-aware: detects issues at scope boundaries

2. **Tracking Structures:**
   - `ContainerBorrow`: Tracks pointers stored in containers
   - `IteratorBorrow`: Tracks iterators and their source containers
   - `MemberBorrow`: Tracks references to object fields
   - `LambdaCapture`: Tracks lambda reference captures and escape status
   - `HeapAllocation`: Tracks new/delete operations

3. **Integration Points:**
   - `IrStatement::BorrowField`: Generated when `&obj.field` is seen
   - `IrStatement::LambdaCapture`: Generated for lambda capture lists
   - `check_raii_issues()`: Main entry point called during analysis

### Scope-Based Detection

The tracker uses scope levels to detect lifetime violations:

```
Scope 0 (function level)
├── Scope 1 (block)
│   └── Scope 2 (nested block)
```

When exiting a scope:
1. Check for container borrows where pointee dies but container survives
2. Check for iterators that outlive their containers
3. Check for member references that outlive their objects
4. Check for escaped lambdas with reference captures to dying variables
5. Clean up tracking data for the dying scope

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
cargo test --test raii_integration_tests
```

## Limitations

1. **No whole-program analysis**: Each translation unit is analyzed independently
2. **Conservative assumptions**: Some safe patterns may be flagged as errors
3. **Template instantiation**: Templates analyzed at declaration, not instantiation
4. **External code**: Functions from external libraries are not analyzed internally
5. **Constructor init order**: Not yet implemented

## Future Work

- Implement Phase 7 (Constructor Initialization Order)
- Improve escape analysis for lambdas passed to functions
- Better tracking of iterator invalidation from container modifications (not just destruction)
- Support for custom container types via annotations
