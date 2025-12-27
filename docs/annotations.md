# RustyCpp Annotations - Complete Guide

This is the comprehensive guide for all annotation features in RustyCpp. It consolidates safety annotations, lifetime annotations, external annotations, and STL handling.

**IMPORTANT**: RustyCpp uses a **two-state safety model**: code is either `@safe` or `@unsafe`. Unannotated code is `@unsafe` by default. All STL and external functions are `@unsafe`, so `@safe` code must use `@unsafe` blocks to call them, or use Rusty structures (`rusty::Vec`, `rusty::Box`, etc.) instead.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Safety System](#safety-system)
3. [Lifetime Annotations](#lifetime-annotations)
4. [Using Rusty Structures (Recommended)](#using-rusty-structures-recommended)
5. [External Annotations](#external-annotations)
6. [STL in Safe Code](#stl-in-safe-code)
7. [Complete Examples](#complete-examples)
8. [Reference Tables](#reference-tables)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Recommended: Use Rusty Structures

```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
void example() {
    // ✅ Recommended: No annotations needed
    rusty::Vec<int> vec = {1, 2, 3};
    int& ref = vec[0];
    // vec.push_back(4);  // ERROR: Cannot modify while borrowed

    rusty::Box<Widget> widget = rusty::Box<Widget>::make(args);
}
```

### If You Must Use STL

```cpp
#include <vector>

// @safe
void use_stl() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);  // OK in unsafe block
    }
}
```

### If You've Audited External Functions

```cpp
// @external: {
//   my_audited_function: [safe, () -> void]
// }

void my_audited_function();

// @safe
void caller() {
    my_audited_function();  // OK: marked [safe] via external annotation
}
```

---

## Safety System

### Two-State Safety Model

RustyCpp uses a two-state safety system:

1. **`@safe`** - Functions with full borrow checking and strict calling rules
2. **`@unsafe`** - Everything else (unannotated code is @unsafe by default)

### Calling Rules Matrix

| Caller → Can Call | @safe | @unsafe |
|-------------------|-------|---------|
| **@safe**         | ✅ Yes | ❌ No (use `@unsafe` block) |
| **@unsafe**       | ✅ Yes | ✅ Yes  |

**Key Insight**: This is a clean two-state model - code is either `@safe` or `@unsafe`. To call unsafe code from `@safe` functions, use an `@unsafe { }` block.

### Safety Annotation Syntax

```cpp
// @safe - Apply to next element only
// @safe
void safe_function() {
    // ✅ CAN call other @safe functions
    safe_helper();

    // ❌ CANNOT call @unsafe functions directly
    // unsafe_func();  // ERROR

    // ✅ CAN call @unsafe via @unsafe block
    // @unsafe
    {
        unsafe_func();           // OK: in @unsafe block
        std::vector<int> vec;    // OK: STL in @unsafe block
    }

    // ❌ CANNOT do pointer operations (outside @unsafe block)
    // int* ptr = &x;  // ERROR: requires unsafe context
}

// @unsafe - Apply to next element only (or no annotation = same)
// @unsafe
void unsafe_function() {
    // ✅ Can call anything and do pointer operations
    safe_function();       // OK
    another_unsafe();      // OK
    int* ptr = nullptr;    // OK
    std::vector<int> vec;  // OK
}

// No annotation = @unsafe by default
void legacy_function() {
    // Treated as @unsafe
    // ✅ Can call anything
    std::vector<int> vec;  // OK
}

// Apply to entire namespace
// @safe
namespace myapp {
    void func1() { }  // Automatically @safe
    void func2() { }  // Automatically @safe
}
```

### Header-to-Implementation Propagation

Safety annotations in headers automatically apply to implementations:

```cpp
// math.h
// @safe
int calculate(int a, int b);

// @unsafe
void process_raw_memory(void* ptr);

// math.cpp
#include "math.h"

int calculate(int a, int b) {
    // Automatically @safe from header
    return a + b;
}

void process_raw_memory(void* ptr) {
    // Automatically @unsafe from header
    // Pointer operations allowed
}
```

---

## Lifetime Annotations

### Basic Syntax

```cpp
// @lifetime: (parameters) -> return_type where constraints
```

### Lifetime Types

| Lifetime Type | Meaning | Example |
|--------------|---------|---------|
| `owned` | Transfers ownership | `create() -> owned` |
| `&'a` | Immutable borrow with lifetime 'a | `get() -> &'a` |
| `&'a mut` | Mutable borrow with lifetime 'a | `get_mut() -> &'a mut` |
| `'static` | Lives forever | `global() -> &'static` |
| `*const` | Const raw pointer (unsafe) | `data() const -> *const` |
| `*mut` | Mutable raw pointer (unsafe) | `data() -> *mut` |

### Common Patterns

#### 1. Borrowing Pattern

```cpp
// @lifetime: (&'a) -> &'a
const T& get(const Container& c) {
    return c.data;
}
```

#### 2. Factory Pattern

```cpp
#include <rusty/box.hpp>

// @lifetime: owned
rusty::Box<Widget> createWidget() {
    return rusty::Box<Widget>::make();
}
```

#### 3. Transformation Pattern

```cpp
// @lifetime: (&'a) -> owned
std::string toString(const Data& d) {
    std::string result;
    // Transform...
    return result;
}
```

#### 4. Selector Pattern

```cpp
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& selectFirst(const T& long_lived, const T& short_lived) {
    return long_lived;
}
```

#### 5. Mutator Pattern

```cpp
// @lifetime: (&'a mut) -> void
void modify(Data& data) {
    data.value = 42;
}
```

### Class Member Functions

```cpp
class Container {
    rusty::Vec<Item> items;

    // @lifetime: (&'a, size_t) -> &'a
    const Item& operator[](size_t index) const {
        return items[index];
    }

    // @lifetime: (&'a mut, size_t) -> &'a mut
    Item& operator[](size_t index) {
        return items[index];
    }

    // @lifetime: owned
    Item remove(size_t index) {
        Item result = std::move(items[index]);
        items.erase(items.begin() + index);
        return result;
    }
};
```

### Combining Safety and Lifetime

```cpp
// @safe
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const Buffer& processBuffer(const Buffer& input, const Config& config) {
    validateConfig(config);
    return input;  // Return has lifetime of input
}
```

---

## Using Rusty Structures (Recommended)

### Why Rusty Structures?

Rusty structures are designed for safe code and integrate seamlessly with the borrow checker:

- ✅ **Safe by design** - No annotations needed
- ✅ **Borrow checking built-in** - Enforced at compile time
- ✅ **Drop trait support** - RAII with Rust semantics
- ✅ **No external annotations** - Works out of the box

### Available Rusty Structures

| Rusty Type | STL Equivalent | Purpose |
|-----------|---------------|---------|
| `rusty::Vec<T>` | `std::vector<T>` | Dynamic array |
| `rusty::Box<T>` | `std::unique_ptr<T>` | Unique ownership |
| `rusty::Arc<T>` | `std::shared_ptr<T>` (thread-safe) | Shared ownership |
| `rusty::Rc<T>` | `std::shared_ptr<T>` (single-thread) | Shared ownership |
| `Weak<T>` | `std::weak_ptr<T>` | Weak references |
| `rusty::Cell<T>` | N/A | Interior mutability (Copy types) |
| `rusty::RefCell<T>` | N/A | Interior mutability (complex types) |

### Examples

#### Vector Operations

```cpp
#include <rusty/vec.hpp>

// @safe
void vector_example() {
    rusty::Vec<int> vec = {1, 2, 3};

    auto it = vec.begin();  // it borrows &'vec mut
    vec.push_back(4);       // ERROR: Cannot modify while borrowed
    *it = 5;                // Would be use-after-invalidation
}
```

#### Smart Pointer Ownership

```cpp
#include <rusty/box.hpp>

// @safe
void unique_ptr_example() {
    rusty::Box<int> ptr1 = rusty::Box<int>::make(42);
    int& ref = *ptr1;                    // ref borrows &'ptr1

    rusty::Box<int> ptr2 = std::move(ptr1); // ERROR: Cannot move while borrowed
}
```

#### Interior Mutability

**IMPORTANT**: The C++ `mutable` keyword is **not allowed** in `@safe` functions. Instead, use `UnsafeCell<T>` with explicit `unsafe` blocks for interior mutability.

```cpp
#include <rusty/unsafe_cell.hpp>

class Counter {
    UnsafeCell<int> count_;  // UnsafeCell for interior mutability

public:
    Counter() : count_(0) {}

    // @safe
    void increment() const {  // const method can mutate via unsafe
        // @unsafe
        {
            int* ptr = count_.get();
            (*ptr)++;
        }
    }

    // @safe
    int get_count() const {
        // @unsafe
        {
            const int* ptr = count_.get();
            return *ptr;
        }
    }
};
```

For types that are trivially copyable (like `int`, `bool`, etc.), you can use `Cell<T>` which provides a safer API:

```cpp
#include <rusty/cell.hpp>

class Counter {
    Cell<int> count_;  // Cell for trivially copyable types

public:
    Counter() : count_(0) {}

    // @safe - but requires unsafe block to access mutable state
    void increment() const {
        // @unsafe
        {
            count_.set(count_.get() + 1);
        }
    }

    // @safe
    int get_count() const {
        return count_.get();  // Read-only, no unsafe needed
    }
};
```

---

## External Annotations

External annotations allow you to annotate third-party code without modifying the source.

### Unified Syntax

```cpp
// @external: {
//   function_name: [safety, lifetime_specification]
// }
```

Where `safety` is either `safe` or `unsafe`:
- **`[safe, ...]`** - Function can be called directly from `@safe` code (you've audited it)
- **`[unsafe, ...]`** - Function requires `@unsafe` block (default for all external code)

### Marking External Functions as Safe

If you've audited an external function and determined it's safe:

```cpp
// @external: {
//   my_audited_function: [safe, () -> void]
//   another_safe_func: [safe, (int x) -> int]
// }

void my_audited_function();
int another_safe_func(int x);

// @safe
void caller() {
    my_audited_function();      // OK: marked [safe]
    int y = another_safe_func(42);  // OK: marked [safe]
}
```

### C Standard Library

By default, all external functions are `@unsafe`. You can mark them as `[unsafe]` with lifetime info for documentation, or use `@unsafe` blocks to call them:

```cpp
// @external: {
//   // Memory management
//   malloc: [unsafe, (size_t size) -> owned void*]
//   calloc: [unsafe, (size_t n, size_t size) -> owned void*]
//   free: [unsafe, (void* ptr) -> void]
//
//   // String operations (marked unsafe even if they're "safe" - external code is always unsafe)
//   strlen: [unsafe, (const char* str) -> size_t]
//   strchr: [unsafe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   strcmp: [unsafe, (const char* s1, const char* s2) -> int]
//
//   // I/O operations
//   fopen: [unsafe, (const char* path, const char* mode) -> owned FILE*]
//   fclose: [unsafe, (FILE* file) -> int]
//   fgets: [unsafe, (char* buffer, int size, FILE* file) -> char* where buffer: 'a, return: 'a]
// }
```

### SQLite3

```cpp
// @external: {
//   sqlite3_open: [unsafe, (const char* filename, sqlite3** db) -> int]
//   sqlite3_close: [unsafe, (sqlite3* db) -> int]
//   sqlite3_prepare_v2: [unsafe, (sqlite3* db, const char* sql, int nbyte, sqlite3_stmt** stmt, const char** tail) -> int]
//   sqlite3_column_text: [unsafe, (sqlite3_stmt* stmt, int col) -> const unsigned char* where stmt: 'a, return: 'a]
//   sqlite3_errmsg: [unsafe, (sqlite3* db) -> const char* where db: 'a, return: 'a]
//   sqlite3_finalize: [unsafe, (sqlite3_stmt* stmt) -> int]
// }
```

### Marking Entire Scopes as Unsafe

```cpp
// Single scope
// @external_unsafe: legacy::*

// Multiple scopes
// @external_unsafe: {
//   scopes: [
//     "legacy::*",
//     "deprecated::*",
//     "vendor::internal::*"
//   ]
// }
```

### Important: STL Internal Type Names

When annotating STL functions, you must use the **internal implementation names** that libclang reports, not the user-facing typedef names. This is because libclang parses the actual type definitions and reports the underlying type.

**Example: `std::string`**

`std::string` is actually a typedef:
```cpp
// In <string> header:
namespace std {
    typedef basic_string<char> string;
    // Or in C++11 ABI:
    namespace __cxx11 {
        typedef basic_string<char> string;
    }
}
```

When you write code using `std::string`, libclang sees the internal name `std::__cxx11::basic_string`. Therefore, external annotations must use the internal name:

```cpp
// ❌ Won't work - typedef name
// @external: {
//   std::string: [safe]
// }

// ✅ Works - internal name that libclang reports
// @external: {
//   std::__cxx11::basic_string::basic_string: [safe]
// }
```

**Common STL Internal Names:**

| User-Facing Type | Internal Name (libclang) |
|-----------------|--------------------------|
| `std::string` | `std::__cxx11::basic_string<char>` |
| `std::wstring` | `std::__cxx11::basic_string<wchar_t>` |
| `std::string_view` | `std::basic_string_view<char>` |
| `std::list<T>::front` | `std::list<...>::front` |
| `std::list<T>::pop_front` | `std::list<...>::pop_front` |

**Tip:** If you're unsure of the internal name, run the borrow checker with verbose output (`-vv`) to see what function names it reports in error messages.

---

## STL in Safe Code

**IMPORTANT**: All STL functions are **@unsafe by default**. To use STL in `@safe` code, you have two options:

1. **Use `@unsafe` blocks** (recommended for STL)
2. **Use Rusty structures instead** (best option)

### Why STL is Unsafe

STL was not designed with Rust-style borrow checking in mind. Operations can invalidate iterators, create dangling references, and allow data races. Rather than maintaining a whitelist of "safe" functions, RustyCpp treats all external code as unsafe by default.

### Complete STL Annotations

```cpp
// @external: {
//   // Vector - ALL marked as unsafe
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::pop_back: [unsafe, (&'a mut) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::at: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::begin: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//   std::vector::end: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//   std::vector::data: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::vector::clear: [unsafe, (&'a mut) -> void]
//   std::vector::resize: [unsafe, (&'a mut, size_t) -> void]
//   std::vector::front: [unsafe, (&'a) -> &'a]
//   std::vector::back: [unsafe, (&'a) -> &'a]
//
//   // Map - ALL marked as unsafe
//   std::map::operator[]: [unsafe, (&'a, const K&) -> &'a mut]
//   std::map::at: [unsafe, (&'a, const K&) -> &'a]
//   std::map::insert: [unsafe, (&'a mut, pair<K,V>) -> void]
//   std::map::erase: [unsafe, (&'a mut, const K&) -> void]
//   std::map::find: [unsafe, (&'a, const K&) -> iterator where this: 'a, return: 'a]
//   std::map::clear: [unsafe, (&'a mut) -> void]
//
//   // Smart pointers - ALL marked as unsafe
//   std::unique_ptr::get: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::unique_ptr::release: [unsafe, (&'a mut) -> owned *mut]
//   std::unique_ptr::reset: [unsafe, (&'a mut, *mut) -> void]
//   std::make_unique: [unsafe, template<T>(Args...) -> owned unique_ptr<T>]
//
//   std::shared_ptr::get: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::shared_ptr::reset: [unsafe, (&'a mut, *mut) -> void]
//   std::make_shared: [unsafe, template<T>(Args...) -> owned shared_ptr<T>]
//
//   // String - ALL marked as unsafe
//   std::string::c_str: [unsafe, (&'a) -> const char* where this: 'a, return: 'a]
//   std::string::data: [unsafe, (&'a) -> const char* where this: 'a, return: 'a]
//   std::string::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::string::substr: [unsafe, (&'a, size_t, size_t) -> owned string]
//   std::string::append: [unsafe, (&'a mut, const string&) -> void]
// }
```

### Using STL in Safe Code

Simply wrap STL usage in `@unsafe` blocks:

```cpp
#include <vector>

// @safe
void use_stl_in_safe() {
    // Must wrap in unsafe block
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);
        int x = vec[0];
    }
}

// Better: Use Rusty structures
// @safe
void use_rusty_instead() {
    rusty::Vec<int> vec = {1, 2, 3};
    vec.push_back(4);  // No unsafe block needed
    int x = vec[0];
}
```

---

## Complete Examples

### Example 1: Safe Application with Rusty Structures

```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
class DataProcessor {
    rusty::Vec<rusty::Box<Record>> records;

public:
    // @lifetime: (&'a mut, owned Record) -> void
    void add_record(Record rec) {
        records.push_back(rusty::Box<Record>::make(std::move(rec)));
    }

    // @lifetime: (&'a, size_t) -> &'a
    const Record& get_record(size_t index) const {
        return *records[index];
    }

    // @lifetime: (&'a) -> size_t
    size_t count() const {
        return records.size();
    }
};
```

### Example 2: Mixed Safe and Unsafe Code

```cpp
#include <rusty/vec.hpp>

// @safe
namespace application {
    void process_data() {
        rusty::Vec<int> data = load_data();
        // Safe processing with lifetime checking
        for (auto& item : data) {
            process_item(item);
        }
    }
}

// @unsafe
namespace low_level {
    void optimize_memory() {
        // Manual memory management allowed
        void* buffer = malloc(1024);
        memset(buffer, 0, 1024);
        free(buffer);
    }
}
```

### Example 3: Using External Libraries

```cpp
#include <unified_external_annotations.hpp>

// @external: {
//   json::parse: [unsafe, (const string& s) -> owned json]
//   json::dump: [unsafe, (const json& j) -> owned string]
// }

// @safe
void process_json() {
    // External functions are always unsafe - must use unsafe block
    // @unsafe
    {
        std::string input = "{\"key\": \"value\"}";
        auto data = json::parse(input);  // OK: called from unsafe block
        std::string output = json::dump(data);  // OK: called from unsafe block
    }
}
```

---

## Reference Tables

### Safety Annotation Reference

| Annotation | Applies To | Scope | Checking |
|-----------|-----------|-------|----------|
| `@safe` | Next element | Function/namespace | Full borrow checking |
| `@unsafe` | Next element | Function/namespace | No checking |
| None (default) | Current element | Implicit | Treated as @unsafe |

### Lifetime Constraint Reference

| Constraint | Syntax | Meaning |
|-----------|--------|---------|
| Outlives | `'a: 'b` | Lifetime 'a must outlive 'b |
| Parameter lifetime | `param: 'a` | Parameter has lifetime 'a |
| Return lifetime | `return: 'a` | Return value has lifetime 'a |
| Transitive | `'a: 'b, 'b: 'c` | 'a outlives 'b outlives 'c |

### Rusty vs STL Comparison

| Feature | rusty::Vec | std::vector | Notes |
|---------|-----------|-------------|-------|
| Safe by default | ✅ Yes | ❌ No | Rusty types work with @safe |
| Borrow checking | ✅ Built-in | ❌ Needs annotation | STL needs external annotation |
| Iterator invalidation | ✅ Detected | ⚠️ Manual | STL requires unsafe blocks |
| Use in safe code | ✅ Direct | ❌ Unsafe block | STL marked as unsafe |

---

## Best Practices

### 1. Prefer Rusty Structures

```cpp
// ✅ Good: Use Rusty structures in safe code
// @safe
void good_example() {
    rusty::Vec<int> vec = {1, 2, 3};
    rusty::Box<Widget> widget = rusty::Box<Widget>::make();
}

// ❌ Avoid: Using STL in safe code
// @safe
void avoid_this() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};  // Requires unsafe block
    }
}
```

### 2. Gradual Adoption

- Start by marking obviously safe functions as `@safe`
- Leave legacy code unannotated (it's `@unsafe` by default)
- Gradually audit functions and mark them `@safe` as you verify them
- Use `@unsafe` blocks when `@safe` code needs to call unsafe functions
- Use external annotations with `[safe]` for audited third-party functions

### 3. Clear Lifetime Documentation

```cpp
// ✅ Good: Clear what's happening
// @lifetime: (&'container, size_t) -> &'container
const T& get_element(const Container& c, size_t idx);

// ❌ Less clear: Generic lifetime names
// @lifetime: (&'a, size_t) -> &'a
const T& get_element(const Container& c, size_t idx);
```

### 4. Annotation Placement

- **In-place**: Directly above declaration
- **External**: In dedicated header files
- **STL**: Include unified_external_annotations.hpp

---

## Troubleshooting

### Common Errors

#### 1. "Cannot call unsafe function from safe code"

```cpp
// Problem:
// @safe
void my_func() {
    legacy_function();  // ERROR: unsafe function
}

// Solution 1: Use @unsafe block
// @safe
void my_func() {
    // @unsafe
    {
        legacy_function();  // OK: in unsafe block
    }
}

// Solution 2: Mark legacy_function as @safe (if it truly is safe)
// @safe
void legacy_function() { }
```

#### 2. "Cannot borrow as mutable while immutable borrow exists"

```cpp
// Problem:
rusty::Vec<int> vec = {1, 2, 3};
const int& ref = vec[0];
vec.push_back(4);  // ERROR

// Solution: Limit borrow scope
{
    const int& ref = vec[0];
    // use ref
}  // ref out of scope
vec.push_back(4);  // OK now
```

#### 3. "Use after move"

```cpp
// Problem:
rusty::Box<int> ptr1 = rusty::Box<int>::make(42);
rusty::Box<int> ptr2 = std::move(ptr1);
*ptr1 = 10;  // ERROR: use after move

// Solution: Use ptr2
*ptr2 = 10;  // OK
```

#### 4. "STL function cannot be called from safe code"

```cpp
// Problem:
// @safe
void my_func() {
    std::vector<int> vec;  // ERROR: STL is @unsafe
}

// Solution 1: Use Rusty structures (recommended)
// @safe
void my_func() {
    rusty::Vec<int> vec;  // OK
}

// Solution 2: Use @unsafe block for STL
// @safe
void my_func() {
    // @unsafe
    {
        std::vector<int> vec;  // OK in unsafe block
    }
}
```

### Debugging Tips

```bash
# Verbose output
rusty-cpp-checker -vv file.cpp

# JSON output for tooling
rusty-cpp-checker --format json file.cpp

# With include paths
rusty-cpp-checker -I include -I third_party file.cpp
```

---

## Migration Guide

### From STL to Rusty

```cpp
// Before: Using STL
#include <vector>
#include <memory>

void old_code() {
    std::vector<int> vec = {1, 2, 3};
    std::unique_ptr<Widget> widget = std::make_unique<Widget>();
}

// After: Using Rusty
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
void new_code() {
    rusty::Vec<int> vec = {1, 2, 3};
    rusty::Box<Widget> widget = rusty::Box<Widget>::make();
}
```

### From Undeclared to Safe

```cpp
// Step 1: Original undeclared code
void process() {
    helper();
}

// Step 2: Mark dependencies first
// @safe
void helper() { }

// Step 3: Mark main function
// @safe
void process() {
    helper();  // OK: both are safe
}
```

---

## Future Enhancements

Planned improvements:
- Lifetime elision (automatic inference)
- Better template support
- Generic lifetime parameters
- Async lifetime tracking
- IDE integration with quick fixes
- Shared annotation databases

---

## Summary

### Key Takeaways

1. ✅ **Use Rusty structures** (`rusty::Vec`, `rusty::Box`) for safe code
2. ⚠️ **STL is @unsafe by default** - use `@unsafe` blocks or Rusty structures
3. 🔒 **Two-state safety** - code is either `@safe` or `@unsafe`
4. 📝 **Lifetime annotations** - Express borrowing relationships
5. 🎯 **Gradual adoption** - Mark functions `@safe` as you audit them
6. 🔧 **External annotations** - Mark audited external functions as `[safe]`

### Quick Decision Tree

```
Need a container?
├─ In @safe code? → Use rusty::Vec
├─ In @safe with STL? → Use @unsafe block
└─ In @unsafe code? → Can use std::vector directly

Need a smart pointer?
├─ Unique ownership? → Use rusty::Box
├─ Shared ownership (thread-safe)? → Use rusty::Arc
├─ Shared ownership (single-thread)? → Use rusty::Rc
└─ In @unsafe code? → Can use std::unique_ptr

Calling a function from @safe?
├─ Is it @safe? → Can call directly
├─ Is it @unsafe? → Use @unsafe block
└─ Want direct call? → Mark with [safe] external annotation
```

---

For more information, see:
- [README.md](../README.md) - Project overview
- [annotation_reference.md](annotation_reference.md) - Quick reference
- Source code examples in `examples/` directory
