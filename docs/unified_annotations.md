# External Annotations for Third-Party Code

RustyCpp uses a unified external annotation system that combines safety and lifetime information for third-party functions. This provides a complete contract for external code without modifying the source.

**IMPORTANT**: By default, all STL and external functions are **undeclared**, meaning they cannot be called from `@safe` functions without explicit annotation. The recommended approach is to use Rusty structures (rusty::Vec, rusty::Box, etc.) instead of STL structures. If you must use STL, you need to annotate them as `unsafe`.

## Overview

The external annotation system provides:
- **Unified syntax**: All functions use `[safety, lifetime]` format
- **Scope-level unsafe**: Mark entire classes/namespaces as unsafe
- **Lifetime specifications**: Express complex lifetime relationships
- **Pattern matching**: Apply rules to groups of functions

## Core Syntax

### 1. Unified Function Annotations

All external functions use the same syntax combining safety and lifetime:

```cpp
// @external: {
//   function_name: [safety, lifetime_specification]
// }
```

Examples:
```cpp
// @external: {
//   strchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   malloc: [unsafe, (size_t size) -> owned void*]
//   strlen: [safe, (const char* str) -> size_t]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
// }
```

### 2. Marking Entire Scopes as Unsafe

Mark entire namespaces or classes as unsafe without annotating each function:

```cpp
// Single scope
// @external_unsafe: legacy::*
// @external_unsafe: OldCStyleAPI::*

// Multiple scopes at once
// @external_unsafe: {
//   scopes: [
//     "legacy::*",
//     "deprecated::*",
//     "vendor::internal::*"
//   ]
// }
```

This is especially useful for:
- Legacy code that hasn't been audited
- Low-level system interfaces
- Vendor libraries with unclear safety guarantees
- Deprecated APIs that should require unsafe context

## Lifetime Specifications

### Basic Lifetime Types

- **`owned`**: Transfers ownership (like Rust's move)
- **`&'a`**: Immutable reference with lifetime 'a
- **`&'a mut`**: Mutable reference with lifetime 'a
- **`'static`**: Static lifetime (lives forever)
- **`'a`**: Named lifetime parameter

### Common Patterns

#### Borrowing Pattern
Function returns a reference into its parameter:
```cpp
// @external: {
//   vector::at: [safe, (size_t index) -> T& where this: 'a, return: 'a]
// }
```

#### Ownership Transfer
Function takes or returns ownership:
```cpp
// @external: {
//   unique_ptr::release: [unsafe, () -> owned T*]
//   make_unique: [safe, (Args... args) -> owned unique_ptr<T>]
// }
```

#### String Functions
C string functions with lifetime relationships:
```cpp
// @external: {
//   strchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   strstr: [safe, (const char* str, const char* needle) -> const char* where str: 'a, return: 'a]
//   strdup: [unsafe, (const char* str) -> owned char*]
// }
```

## Real-World Examples

### C Standard Library

```cpp
// Memory management
// @external: {
//   malloc: [unsafe, (size_t size) -> owned void*]
//   calloc: [unsafe, (size_t n, size_t size) -> owned void*]
//   realloc: [unsafe, (void* ptr, size_t size) -> owned void*]
//   free: [unsafe, (void* ptr) -> void]
// }

// String operations
// @external: {
//   strlen: [safe, (const char* str) -> size_t]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
//   strcmp: [safe, (const char* s1, const char* s2) -> int]
// }

// I/O operations
// @external: {
//   fopen: [unsafe, (const char* path, const char* mode) -> owned FILE*]
//   fclose: [unsafe, (FILE* file) -> int]
//   fgets: [safe, (char* buffer, int size, FILE* file) -> char* where buffer: 'a, return: 'a]
// }
```

### SQLite3

```cpp
// @external: {
//   sqlite3_open: [unsafe, (const char* filename, sqlite3** db) -> int]
//   sqlite3_close: [unsafe, (sqlite3* db) -> int]
//   sqlite3_prepare_v2: [safe, (sqlite3* db, const char* sql, int nbyte, sqlite3_stmt** stmt, const char** tail) -> int]
//   sqlite3_column_text: [safe, (sqlite3_stmt* stmt, int col) -> const unsigned char* where stmt: 'a, return: 'a]
//   sqlite3_errmsg: [safe, (sqlite3* db) -> const char* where db: 'a, return: 'a]
//   sqlite3_finalize: [unsafe, (sqlite3_stmt* stmt) -> int]
// }
```

### C++ STL (Standard Template Library)

**Recommended**: Use Rusty structures instead of STL. If you must use STL in `@safe` code, annotate them as unsafe:

```cpp
// @external: {
//   // Vector - mark as unsafe
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::pop_back: [unsafe, (&'a mut) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::at: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::begin: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//   std::vector::end: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//   std::vector::data: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::vector::clear: [unsafe, (&'a mut) -> void]
//   std::vector::resize: [unsafe, (&'a mut, size_t) -> void]
//
//   // Map - mark as unsafe
//   std::map::operator[]: [unsafe, (&'a, const K&) -> &'a mut]
//   std::map::at: [unsafe, (&'a, const K&) -> &'a]
//   std::map::insert: [unsafe, (&'a mut, pair<K,V>) -> void]
//   std::map::erase: [unsafe, (&'a mut, const K&) -> void]
//   std::map::find: [unsafe, (&'a, const K&) -> iterator where this: 'a, return: 'a]
//
//   // Smart pointers - mark as unsafe
//   std::unique_ptr::get: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::unique_ptr::release: [unsafe, (&'a mut) -> owned *mut]
//   std::unique_ptr::reset: [unsafe, (&'a mut, *mut) -> void]
//   std::make_unique: [unsafe, template<T>(Args...) -> owned unique_ptr<T>]
//
//   std::shared_ptr::get: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::shared_ptr::reset: [unsafe, (&'a mut, *mut) -> void]
//   std::make_shared: [unsafe, template<T>(Args...) -> owned shared_ptr<T>]
//
//   // String - mark as unsafe
//   std::string::c_str: [unsafe, (&'a) -> const char* where this: 'a, return: 'a]
//   std::string::data: [unsafe, (&'a) -> const char* where this: 'a, return: 'a]
//   std::string::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::string::substr: [unsafe, (&'a, size_t, size_t) -> owned string]
// }
```

**Better approach - Use Rusty structures:**
```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
void safe_code() {
    // No annotations needed - these are safe by design
    rusty::Vec<int> vec = {1, 2, 3};
    rusty::Box<Widget> widget = rusty::Box<Widget>::make(args);
}
```

### Boost Library

```cpp
// @external: {
//   boost::lexical_cast: [safe, template<T, S>(const S& arg) -> owned T]
//   boost::format: [safe, (const string& fmt) -> owned format]
//   boost::shared_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   boost::unique_ptr::release: [unsafe, () -> owned T*]
//   boost::filesystem::exists: [safe, (const path& p) -> bool]
//   boost::filesystem::canonical: [safe, (const path& p) -> owned path]
// }
```

### JSON Libraries

```cpp
// @external: {
//   nlohmann::json::parse: [safe, (const string& s) -> owned json]
//   nlohmann::json::dump: [safe, (int indent) -> owned string]
//   nlohmann::json::operator[]: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   nlohmann::json::at: [safe, (const string& key) -> json& where this: 'a, return: 'a]
//   nlohmann::json::get: [safe, template<T>() -> owned T]
//   nlohmann::json::get_ref: [safe, template<T>() -> T& where this: 'a, return: 'a]
// }
```

## Complex Lifetime Relationships

### Outlives Constraints

Specify that one lifetime must outlive another:

```cpp
// @external_function: keep_longer {
//   safety: safe
//   lifetime: (const T& long_lived, const T& short_lived) -> const T&
//   where: long_lived: 'a, short_lived: 'b, return: 'a, 'a: 'b
// }
```

This means:
- `long_lived` has lifetime `'a`
- `short_lived` has lifetime `'b`
- Return value has lifetime `'a`
- `'a` must outlive `'b` (`'a: 'b`)

### Multiple Parameters

Functions with complex parameter relationships:

```cpp
// @external_function: merge_containers {
//   safety: safe
//   lifetime: (Container& dest, const Container& src) -> void
//   where: dest: 'a, src: 'b, 'b: 'a
// }
```

This ensures `src` must live at least as long as `dest`.

### Callback Lifetimes

Functions taking callbacks with lifetime requirements:

```cpp
// @external_function: async_operation {
//   safety: unsafe
//   lifetime: (Callback cb, void* context) -> void
//   where: cb: 'static, context: 'a, 'a: 'static
// }
```

## Usage in Safe Code

Once annotated, the functions are checked according to their contracts:

```cpp
#include <rusty/vec.hpp>

// @safe
void example() {
    // Recommended: Use Rusty structures (no annotations needed)
    rusty::Vec<int> vec = {1, 2, 3};
    int& ref = vec[0];  // ref lifetime tied to vec
    // vec.clear();  // ERROR: would invalidate ref

    // Safe function with lifetime checking
    const char* text = "Hello, world!";
    const char* found = strchr(text, 'o');  // OK: safe, found lifetime tied to text

    // Unsafe function requires unsafe context
    // void* buffer = malloc(100);  // ERROR: malloc is unsafe
}

// @safe
void use_stl_if_needed() {
    // If you must use STL, wrap in unsafe block
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};  // OK in unsafe block
        vec.push_back(4);
    }
}

// @unsafe
void unsafe_example() {
    // Unsafe functions allowed
    void* buffer = malloc(100);  // OK in unsafe
    memset(buffer, 0, 100);      // OK in unsafe
    free(buffer);                 // OK in unsafe

    // STL also works in unsafe functions
    std::vector<int> vec = {1, 2, 3};
    vec.push_back(4);
}
```

## Profiles with Unified Annotations

Create reusable profiles for libraries:

```cpp
// @external_profile: opencv {
//   annotations: {
//     cv::imread: [safe, (const string& path) -> owned Mat]
//     cv::Mat::data: [safe, () -> uchar* where this: 'a, return: 'a]
//     cv::Mat::at: [safe, (int row, int col) -> T& where this: 'a, return: 'a]
//     cv::resize: [safe, (const Mat& src, Mat& dst, Size size) -> void]
//     cv::cvtColor: [safe, (const Mat& src, Mat& dst, int code) -> void]
//   }
// }
```

## Pattern-Based Annotations

Apply annotations to groups of functions:

```cpp
// @external_pattern: borrowing_getter {
//   matches: ["*::get*", "*::at", "*::operator[]"]
//   safety: safe
//   lifetime: generic (&'self) -> &'self
// }

// @external_pattern: factory_function {
//   matches: ["*::create*", "*::make*"]
//   safety: varies
//   lifetime: generic (...) -> owned T
// }
```

## Best Practices

### 1. Start with Safety

First determine if a function is safe or unsafe:
- **Safe**: No undefined behavior possible
- **Unsafe**: Can cause memory errors, data races, etc.

### 2. Identify Ownership

Determine ownership semantics:
- Does it transfer ownership? Use `owned`
- Does it borrow? Use lifetime parameters
- Does it create relationships? Use constraints

### 3. Express Relationships

Use lifetime constraints for complex relationships:
- Return values that borrow from parameters
- Parameters that must outlive each other
- Callbacks that need specific lifetimes

### 4. Test Thoroughly

Create test cases that verify:
- Safety enforcement works
- Lifetime relationships are correct
- Edge cases are handled

## Integration Examples

### With STL Annotations

Combine with STL lifetime annotations:

```cpp
#include <stl_lifetimes.hpp>
#include <unified_external_annotations.hpp>

// @safe
void combined() {
    // STL with lifetime checking
    std::vector<int> vec = {1, 2, 3};
    int& ref = vec[0];  // STL annotation: &'vec mut
    
    // External function with unified annotation
    const char* str = "test";
    const char* ch = strchr(str, 't');  // External: str: 'a, return: 'a
}
```

### With Build Systems

CMake integration:

```cmake
target_compile_definitions(myproject PRIVATE
    RUSTYCPP_USE_UNIFIED_ANNOTATIONS=1
)
```

## Troubleshooting

### Common Issues

1. **Lifetime Too Restrictive**
   ```cpp
   // Too restrictive:
   // func: [safe, (&'a T) -> &'a U]
   
   // Better:
   // func: [safe, (&'a T) -> owned U]
   ```

2. **Missing Outlives Bounds**
   ```cpp
   // Missing constraint:
   // func: [safe, (&'a T, &'b T) -> &'a T]
   
   // With constraint:
   // func: [safe, (&'a T, &'b T) -> &'a T where 'a: 'b]
   ```

3. **Wrong Safety Classification**
   ```cpp
   // Wrong (strcpy can overflow):
   // strcpy: [safe, ...]
   
   // Correct:
   // strcpy: [unsafe, ...]
   ```

## Future Enhancements

- Automatic inference from function signatures
- IDE support with quick fixes
- Shared annotation databases
- Machine learning-based annotation suggestions
- Integration with documentation generators