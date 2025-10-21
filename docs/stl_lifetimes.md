# STL Lifetime Annotations

**IMPORTANT: For safe code, it is recommended to use Rusty structures (rusty::Vec, rusty::Box, etc.) instead of STL structures.**

STL structures are **undeclared by default**, meaning they cannot be called from `@safe` functions without explicit annotation. This document explains how to annotate STL types if you need to use them.

## Recommended Approach: Use Rusty Structures

For safe code, use RustyCpp's built-in data structures:

```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>
#include <rusty/rc.hpp>

// @safe
void example() {
    rusty::Vec<int> vec = {1, 2, 3};
    int& ref = vec[0];     // Borrows &'vec mut
    vec.push_back(4);      // ERROR: Cannot modify vec while ref exists
}
```

## Annotating STL Types (When Required)

If you must use STL structures in your codebase, you need to explicitly annotate them as **unsafe** using external annotations:

```cpp
#include <vector>
#include <unified_external_annotations.hpp>

// @external: {
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::begin: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
// }

// @safe
void use_stl() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);  // OK: in unsafe block
    }
}
```

## How It Works

### Type-Level Annotations

The `stl_lifetimes.hpp` header contains special comments that RustyCpp recognizes:

```cpp
// @type_lifetime: std::vector<T> {
//   operator[](size_t) -> &'self mut      // Returns mutable reference to self
//   operator[](size_t) const -> &'self    // Returns const reference to self
//   push_back(T) -> owned                 // Takes ownership, no borrow
//   data() -> *mut                        // Returns raw pointer (unsafe)
//   begin() -> &'self mut                 // Iterator borrows mutably
// }
```

### Lifetime Types

- `&'self` - Immutable reference with object's lifetime
- `&'self mut` - Mutable reference with object's lifetime
- `*const` - Const raw pointer (requires unsafe)
- `*mut` - Mutable raw pointer (requires unsafe)
- `owned` - Ownership transfer, no borrowing

## Common Patterns with Rusty Structures

### Vector Iterator Invalidation

```cpp
#include <rusty/vec.hpp>

// @safe
void iterator_invalidation() {
    rusty::Vec<int> vec = {1, 2, 3};

    auto it = vec.begin();  // it borrows &'vec mut
    vec.push_back(4);       // ERROR: Would invalidate iterator
    *it = 5;                // Would be use-after-invalidation
}
```

### Smart Pointer Ownership

```cpp
#include <rusty/box.hpp>

// @safe
void unique_ptr_ownership() {
    rusty::Box<int> ptr1 = rusty::Box<int>::make(42);
    int& ref = *ptr1;                    // ref borrows &'ptr1

    rusty::Box<int> ptr2 = std::move(ptr1); // ERROR: Cannot move while borrowed
}
```

## Using STL with External Annotations

If you need to use STL types, annotate them as unsafe:

### Vector with External Annotations

```cpp
#include <vector>
#include <unified_external_annotations.hpp>

// @external: {
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::begin: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//   std::vector::data: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
// }

// @safe
void requires_unsafe() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        int* ptr = vec.data();  // OK in unsafe block
        *ptr = 10;              // OK in unsafe context
    }
}
```

### Map with External Annotations

```cpp
#include <map>
#include <unified_external_annotations.hpp>

// @external: {
//   std::map::operator[]: [unsafe, (&'a, const K&) -> &'a mut]
//   std::map::erase: [unsafe, (&'a mut, const K&) -> void]
// }

// @safe
void map_references() {
    // @unsafe
    {
        std::map<int, std::string> m;
        m[1] = "one";

        std::string& ref = m[1];  // ref borrows &'m mut
        m[3] = "three";           // OK: map references are stable
        m.erase(1);               // ERROR: Would invalidate ref
    }
}
```

## STL Types Requiring External Annotations

**Note**: All these types must be annotated as `unsafe` to use in `@safe` code. The preferred approach is to use Rusty equivalents instead.

### Containers (Use rusty::Vec, rusty::Map instead)
- `std::vector<T>` - Dynamic array (use `rusty::Vec<T>`)
- `std::map<K,V>` - Ordered map
- `std::unordered_map<K,V>` - Hash map
- `std::set<T>` - Ordered set
- `std::unordered_set<T>` - Hash set
- `std::deque<T>` - Double-ended queue
- `std::list<T>` - Doubly-linked list
- `std::array<T,N>` - Fixed-size array

### Smart Pointers (Use rusty::Box, rusty::Arc, rusty::Rc instead)
- `std::unique_ptr<T>` - Unique ownership (use `rusty::Box<T>`)
- `std::shared_ptr<T>` - Shared ownership (use `rusty::Arc<T>` or `rusty::Rc<T>`)
- `std::weak_ptr<T>` - Weak references (use custom `Weak<T>`)

### Other Types
- `std::string` - String with small-string optimization
- `std::optional<T>` - Optional values
- `std::pair<T1,T2>` - Pair of values
- `std::tuple<T...>` - Tuple of values

### Example External Annotations for Common STL Types

```cpp
// @external: {
//   // Vector
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   std::vector::begin: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//   std::vector::end: [unsafe, (&'a) -> iterator where this: 'a, return: 'a]
//
//   // Map
//   std::map::operator[]: [unsafe, (&'a, const K&) -> &'a mut]
//   std::map::at: [unsafe, (&'a, const K&) -> &'a]
//   std::map::insert: [unsafe, (&'a mut, pair<K,V>) -> void]
//
//   // Smart pointers
//   std::unique_ptr::get: [unsafe, (&'a) -> *mut where this: 'a, return: 'a]
//   std::unique_ptr::release: [unsafe, (&'a mut) -> owned *mut]
//   std::make_unique: [unsafe, (Args...) -> owned unique_ptr<T>]
// }
```

## Lifetime Rules

### Iterator Invalidation

Operations that invalidate iterators/references:
- **vector**: `push_back`, `insert`, `erase`, `resize`, `reserve`
- **deque**: `push_front`, `push_back` (middle iterators stable)
- **unordered_map/set**: `insert`, `erase` (on rehash)

Operations that preserve iterators/references:
- **map/set**: All operations except erasing the element
- **list**: All operations except erasing the element

### Reference Lifetime

1. References returned by methods have the container's lifetime
2. Cannot modify container structure while references exist
3. Can have multiple immutable references
4. Can have only one mutable reference

### Move Semantics

1. After `std::move()`, the source object is in moved-from state
2. Cannot use moved-from objects (except assignment/destruction)
3. References to moved objects become invalid

## Writing Custom Annotations

You can add lifetime annotations for your own types:

```cpp
// @type_lifetime: MyContainer<T> {
//   get(size_t) -> &'self mut
//   get(size_t) const -> &'self
//   add(T) -> owned
//   iterator: &'self
// }
class MyContainer {
    // Implementation
};
```

## Integration with Existing Code

### Gradual Adoption

1. Include `stl_lifetimes.hpp` in files you want to check
2. Mark functions or namespaces with `@safe`
3. Fix lifetime errors incrementally
4. Use `@unsafe` for low-level code that needs raw pointers

### Mixing Safe and Unsafe

```cpp
// @safe
namespace application {
    void process_data() {
        std::vector<int> data = load_data();
        // ... safe processing with lifetime checking
    }
}

// @unsafe
namespace low_level {
    void optimize_memory() {
        int* raw = allocate_buffer();
        // ... manual memory management
        deallocate_buffer(raw);
    }
}
```

## Limitations

1. **Template Instantiation**: Full template support is limited
2. **Type Inference**: Simplified type inference for auto variables
3. **Custom Allocators**: Not tracked
4. **Thread Safety**: Not analyzed (data races not detected)

## Best Practices

1. **Prefer References**: Use references over raw pointers
2. **Avoid data()**: Use iterators or operator[] instead
3. **RAII**: Let destructors handle cleanup
4. **Smart Pointers**: Use unique_ptr/shared_ptr over raw pointers
5. **Const Correctness**: Use const methods when not modifying

## Error Messages

RustyCpp provides clear error messages for lifetime violations:

```
error: Cannot modify 'vec' while reference 'ref' exists
  --> example.cpp:5:5
   |
 4 |     int& ref = vec[0];
   |     --- 'ref' borrows 'vec' here
 5 |     vec.push_back(4);
   |     ^^^^^^^^^^^^^^^^ cannot modify while borrowed
```

## Future Improvements

- Full template instantiation support
- Custom allocator tracking
- Thread safety analysis
- IDE integration with quick fixes
- Performance optimizations for large codebases