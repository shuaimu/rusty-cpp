# C++ Standard Library in Safe Code

## Overview

In RustyCpp's two-state safety model, all STL and external functions are **@unsafe by default**. This means `@safe` code cannot call STL functions directly.

## Why STL is Unsafe by Default

The C++ Standard Library was not designed with Rust-style borrow checking in mind. Many STL operations can:
- Invalidate iterators
- Create dangling references
- Allow data races

Rather than maintaining a whitelist of "safe" STL functions (which would be incomplete and error-prone), RustyCpp takes the conservative approach: **all external code is unsafe until explicitly marked otherwise**.

## Options for Using STL in Safe Code

### Option 1: Use Rusty Structures (Recommended)

The best approach is to use RustyCpp's safe data structures:

```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
void safe_example() {
    // ✅ Rusty structures work directly in @safe code
    rusty::Vec<int> vec = {1, 2, 3};
    vec.push_back(4);

    rusty::Box<Widget> widget = rusty::Box<Widget>::make();
}
```

### Option 2: Use @unsafe Blocks

Wrap STL usage in `@unsafe` blocks:

```cpp
#include <vector>
#include <algorithm>

// @safe
void use_stl_with_unsafe_blocks() {
    // @unsafe
    {
        std::vector<int> vec = {5, 2, 8, 1, 9};
        vec.push_back(10);
        std::sort(vec.begin(), vec.end());
    }
}
```

### Option 3: External Annotations with [safe]

If you've audited specific external functions and determined they're safe for your use case, you can mark them as `[safe]` via external annotations:

```cpp
// @external: {
//   my_safe_helper: [safe, () -> void]
// }

void my_safe_helper();  // External function you've audited

// @safe
void caller() {
    my_safe_helper();  // OK: marked [safe] via external annotation
}
```

**Note**: This approach requires careful auditing. You take responsibility for ensuring the function is actually safe.

## Examples

### Vector Operations

```cpp
#include <vector>

// @safe
void vector_example() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);
        vec.pop_back();
        int size = vec.size();
    }
}
```

### String Operations

```cpp
#include <string>

// @safe
void string_example() {
    // @unsafe
    {
        std::string s1 = "Hello";
        std::string s2 = " World";
        std::string s3 = s1 + s2;
        s3.append("!");
    }
}
```

### Smart Pointers

```cpp
#include <memory>

// @safe
void smart_pointer_example() {
    // @unsafe
    {
        auto ptr1 = std::make_unique<int>(42);
        int value1 = *ptr1;

        auto ptr2 = std::make_shared<int>(100);
        int value2 = *ptr2;
    }
}
```

### Algorithms

```cpp
#include <vector>
#include <algorithm>

// @safe
void algorithm_example() {
    // @unsafe
    {
        std::vector<int> vec = {5, 2, 8, 1, 9};
        std::sort(vec.begin(), vec.end());
        auto it = std::find(vec.begin(), vec.end(), 8);
    }
}
```

### I/O Operations

```cpp
#include <iostream>

// @safe
void io_example() {
    // @unsafe
    {
        std::cout << "Hello" << std::endl;
        std::cout << 42 << std::endl;
    }
}
```

## @unsafe Functions Can Use STL Directly

Functions marked `@unsafe` (or unannotated functions, which are `@unsafe` by default) can use STL directly without `@unsafe` blocks:

```cpp
#include <vector>
#include <algorithm>

// @unsafe
void unsafe_function() {
    // ✅ No @unsafe block needed - function is already unsafe
    std::vector<int> vec = {1, 2, 3};
    vec.push_back(4);
    std::sort(vec.begin(), vec.end());
}

// No annotation = @unsafe by default
void legacy_function() {
    // ✅ Also works - unannotated functions are @unsafe
    std::vector<int> vec = {1, 2, 3};
    vec.push_back(4);
}
```

## Comparison: Rusty vs STL

| Feature | rusty::Vec | std::vector |
|---------|-----------|-------------|
| Works in @safe | ✅ Yes | ❌ No (needs @unsafe block) |
| Iterator invalidation detected | ✅ Yes | ❌ No |
| Borrow checking | ✅ Built-in | ❌ None |
| Performance | Same | Same |

## Best Practices

1. **Prefer Rusty structures** in new `@safe` code
2. **Use @unsafe blocks** when you must use STL
3. **Keep @unsafe blocks small** - minimize the unsafe surface area
4. **Document why** you need the unsafe block
5. **Consider wrapping** frequently-used STL patterns in safe helper functions

## Migration Strategy

When migrating existing code:

1. Start by marking functions as `@safe` or `@unsafe`
2. For `@safe` functions that use STL, wrap STL usage in `@unsafe` blocks
3. Gradually replace STL with Rusty structures where practical
4. Use external annotations for audited third-party functions

## See Also

- [Safety System](../../README.md#-safety-system) - Overview of two-state safety model
- [Annotations Guide](../annotations.md) - Complete annotation reference
- [Rusty Structures](../../include/rusty/README.md) - Safe data structure documentation
