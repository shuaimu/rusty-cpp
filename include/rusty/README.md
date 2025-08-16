# Rusty C++ Library

A collection of Rust-inspired safe types for C++ with proper lifetime annotations.

## Overview

This library provides C++ implementations of popular Rust types that work with the Rusty C++ Checker to ensure memory safety at compile time. All types follow Rust's ownership and borrowing principles.

## Types

### Box<T> - Single Ownership Smart Pointer
```cpp
#include "rusty/box.hpp"

// Create a Box
auto box = rusty::make_box<int>(42);

// Move ownership
auto box2 = std::move(box);  // box is now invalid

// Automatic cleanup when Box goes out of scope
```

**Guarantees:**
- Single ownership
- No copying allowed
- Automatic deallocation
- Null state after move

### Arc<T> - Atomic Reference Counting
```cpp
#include "rusty/arc.hpp"

// Create an Arc
auto arc1 = rusty::make_arc<int>(100);

// Clone increases ref count
auto arc2 = arc1.clone();

// Thread-safe reference counting
// Immutable access only
```

**Guarantees:**
- Thread-safe shared ownership
- Automatic cleanup when last Arc is dropped
- Immutable access only (use Mutex for mutation)

### Rc<T> - Reference Counting (Single-threaded)
```cpp
#include "rusty/rc.hpp"

// Create an Rc
auto rc1 = rusty::make_rc<int>(50);

// Clone increases ref count
auto rc2 = rc1.clone();

// Single-threaded only (use Arc for multi-threading)
```

**Guarantees:**
- Shared ownership within a thread
- Lower overhead than Arc
- Not thread-safe

### Vec<T> - Dynamic Array
```cpp
#include "rusty/vec.hpp"

// Create a Vec
rusty::Vec<int> vec;
vec.push(10);
vec.push(20);

// Access elements
int first = vec[0];

// Pop from back
int last = vec.pop();

// Move semantics
auto vec2 = std::move(vec);  // vec is now invalid
```

**Guarantees:**
- Single ownership of container
- Elements are owned by Vec
- Automatic resizing
- No copying of Vec allowed

### Option<T> - Nullable Values
```cpp
#include "rusty/option.hpp"

// Create Some and None
auto some = rusty::Some(42);
auto none = rusty::Option<int>(rusty::None);

// Check and unwrap
if (some.is_some()) {
    int val = some.unwrap();
}

// Map over Option
auto doubled = some.map([](int x) { return x * 2; });

// Unwrap with default
int val = none.unwrap_or(0);
```

**Guarantees:**
- Explicit null handling
- No null pointer dereferencing
- Type-safe absence representation

### Result<T, E> - Error Handling
```cpp
#include "rusty/result.hpp"

// Function that can fail
rusty::Result<int, const char*> divide(int a, int b) {
    if (b == 0) {
        return rusty::Err<int, const char*>("Division by zero");
    }
    return rusty::Ok<int, const char*>(a / b);
}

// Check and handle
auto result = divide(10, 2);
if (result.is_ok()) {
    int val = result.unwrap();
} else {
    const char* err = result.unwrap_err();
}

// Chain operations
auto chained = divide(100, 5)
    .and_then([](int x) { return divide(x, 2); });
```

**Guarantees:**
- Explicit error handling
- No exceptions unless explicitly unwrapped
- Composable error propagation

## Lifetime Annotations

All types include lifetime annotations that work with the Rusty C++ Checker:

```cpp
// Functions returning references have lifetime annotations
// @lifetime: (&'a) -> &'a
const T& Box<T>::operator*() const;

// Functions transferring ownership
// @lifetime: owned
Box<T> make_box(T value);

// Functions with lifetime constraints
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& select_first(const T& a, const T& b);
```

## Safety Guarantees

When used with the Rusty C++ Checker, these types provide:

1. **Use-after-move detection**
   ```cpp
   auto box1 = rusty::make_box<int>(42);
   auto box2 = std::move(box1);
   int val = *box1;  // ERROR: use after move
   ```

2. **Borrow checking**
   ```cpp
   auto box = rusty::make_box<int>(42);
   const int& ref1 = *box;  // Immutable borrow
   int& ref2 = *box;         // ERROR: can't have mutable with immutable
   ```

3. **Lifetime safety**
   ```cpp
   const int& get_ref() {
       auto box = rusty::make_box<int>(42);
       return *box;  // ERROR: returning reference to local
   }
   ```

4. **Thread safety**
   ```cpp
   rusty::Arc<int> arc = make_arc<int>(42);
   // Can safely share across threads
   
   rusty::Rc<int> rc = make_rc<int>(42);
   // Single-threaded only
   ```

## Usage

Include all types:
```cpp
#include "rusty/rusty.hpp"
```

Or include individual headers:
```cpp
#include "rusty/box.hpp"
#include "rusty/vec.hpp"
// etc.
```

## Comparison with std::

| Rusty Type | std:: Equivalent | Key Differences |
|------------|------------------|-----------------|
| Box<T> | unique_ptr<T> | Lifetime annotations, stricter move semantics |
| Arc<T> | shared_ptr<T> | Immutable by default, explicit cloning |
| Rc<T> | shared_ptr<T> | Single-threaded, lower overhead |
| Vec<T> | vector<T> | No copying allowed, owned elements |
| Option<T> | optional<T> | Explicit None handling, map/unwrap methods |
| Result<T,E> | expected<T,E> | Method chaining, monadic operations |

## Design Principles

1. **Zero-cost abstractions** - No runtime overhead compared to manual management
2. **Explicit ownership** - Clear who owns what
3. **Move-by-default** - Copying must be explicit
4. **Type safety** - Compile-time guarantees
5. **Composability** - Types work well together

## Examples

See `examples/rusty_types_demo.cpp` for comprehensive examples.

## Requirements

- C++11 or later
- Rusty C++ Checker for compile-time safety verification

## License

Same as the Rusty C++ Checker project.