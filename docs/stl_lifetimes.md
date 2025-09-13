# STL Lifetime Annotations

RustyCpp now supports lifetime checking for C++ Standard Template Library (STL) types without modifying the standard library headers. This is achieved through external lifetime annotations that describe how lifetimes flow through STL methods.

## Quick Start

To enable STL lifetime checking in your code:

```cpp
#include <stl_lifetimes.hpp>  // RustyCpp STL annotations
#include <vector>
#include <map>
#include <memory>

// @safe
void example() {
    std::vector<int> vec = {1, 2, 3};
    int& ref = vec[0];     // Borrows &'vec mut
    vec.push_back(4);      // ERROR: Cannot modify vec while ref exists
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

## Common STL Patterns

### Vector Iterator Invalidation

```cpp
// @safe
void iterator_invalidation() {
    std::vector<int> vec = {1, 2, 3};
    
    auto it = vec.begin();  // it borrows &'vec mut
    vec.push_back(4);       // ERROR: Would invalidate iterator
    *it = 5;                // Would be use-after-invalidation
}
```

### Map Reference Stability

```cpp
// @safe
void map_references() {
    std::map<int, std::string> m;
    m[1] = "one";
    
    std::string& ref = m[1];  // ref borrows &'m mut
    m[3] = "three";           // OK: map references are stable
    m.erase(1);               // ERROR: Would invalidate ref
}
```

### Smart Pointer Ownership

```cpp
// @safe
void unique_ptr_ownership() {
    std::unique_ptr<int> ptr1 = std::make_unique<int>(42);
    int& ref = *ptr1;                          // ref borrows &'ptr1
    
    std::unique_ptr<int> ptr2 = std::move(ptr1); // ERROR: Cannot move while borrowed
}
```

### Raw Pointers Require Unsafe

```cpp
// @safe
void requires_unsafe() {
    std::vector<int> vec = {1, 2, 3};
    int* ptr = vec.data();  // Getting pointer is OK
    *ptr = 10;              // ERROR: Dereferencing requires @unsafe
}

// @unsafe
void with_unsafe() {
    std::vector<int> vec = {1, 2, 3};
    int* ptr = vec.data();
    *ptr = 10;              // OK in unsafe context
}
```

## Supported STL Types

### Containers
- `std::vector<T>` - Dynamic array with iterator invalidation rules
- `std::map<K,V>` - Ordered map with stable references
- `std::unordered_map<K,V>` - Hash map with invalidation on rehash
- `std::set<T>` - Ordered set with stable iterators
- `std::unordered_set<T>` - Hash set with invalidation on rehash
- `std::deque<T>` - Double-ended queue
- `std::list<T>` - Doubly-linked list with stable iterators
- `std::array<T,N>` - Fixed-size array

### Smart Pointers
- `std::unique_ptr<T>` - Unique ownership with move semantics
- `std::shared_ptr<T>` - Shared ownership with reference counting
- `std::weak_ptr<T>` - Weak references to shared objects

### Other Types
- `std::string` - String with small-string optimization
- `std::optional<T>` - Optional values
- `std::pair<T1,T2>` - Pair of values
- `std::tuple<T...>` - Tuple of values

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