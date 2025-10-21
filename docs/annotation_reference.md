# RustyCpp Annotation Quick Reference

> **📘 This is a quick reference for syntax lookup. For complete documentation with examples, migration guides, and detailed explanations, see [annotations.md](annotations.md).**

Quick syntax reference for all annotation types supported by RustyCpp.

## Annotation Types Overview

RustyCpp supports three main categories of annotations:

1. **In-Place Annotations** - Written directly in your C++ source code
2. **External Annotations** - For third-party code you can't modify
3. **STL Annotations** - Pre-configured for C++ Standard Library

## In-Place Annotations (Your Code)

### Safety Annotations (Three-State System)

```cpp
// Three states: @safe, @unsafe, and undeclared (default)

// Mark next element as safe (full borrow checking + strict calling rules)
// @safe
void myFunction() {
    // Can call @safe and @unsafe functions
    // CANNOT call undeclared functions
    // Cannot do pointer operations
}

// Mark next element as unsafe (skip checking, explicitly documented)
// @unsafe
void lowLevelFunction() {
    // Can call anything
    // Can do pointer operations
}

// No annotation = undeclared (unaudited legacy code)
void legacyFunction() {
    // Not checked, can call anything
    // Safe functions CANNOT call this
}

// Apply to namespace
// @safe
namespace myapp {
    // All functions here are safe
}

// Header-to-implementation propagation
// In header: @safe void func();
// In .cpp: void func() { /* automatically safe */ }
```

### Lifetime Annotations

```cpp
// Basic syntax
// @lifetime: (parameters) -> return_type where constraints

// Examples:

// Borrowing - return has same lifetime as parameter
// @lifetime: (&'a) -> &'a
const T& borrow(const Container& c);

// Ownership transfer
// @lifetime: owned
std::unique_ptr<T> create();

// Mutable borrow
// @lifetime: (&'a mut) -> void
void modify(Data& data);

// Multiple parameters with constraints
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& selectFirst(const T& a, const T& b);

// Static lifetime
// @lifetime: () -> &'static
const Config& getGlobalConfig();
```

### Combined Annotations

```cpp
// Combine safety and lifetime
// @safe
// @lifetime: (&'a) -> &'a
const Data& process(const Data& input) {
    // Function is both safe and has lifetime checking
}
```

## External Annotations (Third-Party Code)

### Safety-Only Annotations

```cpp
// @external_safety: {
//   third_party::func1: safe
//   third_party::func2: unsafe
//   legacy::old_api: unsafe
// }
```

### Lifetime-Only Annotations

```cpp
// @external_lifetime: {
//   strchr: (const char* str, int c) -> const char* where str: 'a, return: 'a
//   strdup: (const char* str) -> owned char*
// }
```

### Unified Annotations (Safety + Lifetime)

```cpp
// Compact syntax
// @external: {
//   function_name: [safety, lifetime_spec]
// }

// Examples:
// @external: {
//   malloc: [unsafe, (size_t) -> owned void*]
//   strlen: [safe, (const char*) -> size_t]
//   strcpy: [unsafe, (char* dest, const char* src) -> char* where dest: 'a, return: 'a]
// }

// Detailed syntax
// @external_function: function_name {
//   safety: safe/unsafe
//   lifetime: (params) -> return
//   where: constraints
// }
```

### Pattern-Based Annotations

```cpp
// Whitelist patterns (mark as safe)
// @external_whitelist: {
//   patterns: ["std::*", "*::size", "*::empty"]
// }

// Blacklist patterns (mark as unsafe)
// @external_blacklist: {
//   patterns: ["*::malloc", "*::free", "*::operator new*"]
// }
```

### Library Profiles

```cpp
// Define reusable profile
// @external_profile: profile_name {
//   safe: ["pattern1", "pattern2"]
//   unsafe: ["pattern3", "pattern4"]
// }

// With annotations
// @external_profile: boost {
//   annotations: {
//     boost::format: [safe, (const string&) -> owned format]
//     boost::shared_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   }
// }
```

## STL Type Annotations

### Type-Level Lifetime Annotations

```cpp
// @type_lifetime: std::vector<T> {
//   at(size_t) -> &'self mut
//   at(size_t) const -> &'self
//   push_back(T) -> owned
//   data() -> *mut
//   begin() -> &'self mut
//   iterator: &'self mut
// }
```

### Using STL Annotations

```cpp
#include <stl_lifetimes.hpp>
#include <vector>

// @safe
void example() {
    std::vector<int> vec = {1, 2, 3};
    int& ref = vec[0];     // Tracked: &'vec mut
    vec.push_back(4);      // ERROR: would invalidate ref
}
```

## Lifetime Types Reference

| Lifetime Type | Meaning | Example |
|--------------|---------|---------|
| `owned` | Transfers ownership | `malloc() -> owned void*` |
| `&'a` | Immutable borrow with lifetime 'a | `get() -> &'a` |
| `&'a mut` | Mutable borrow with lifetime 'a | `get_mut() -> &'a mut` |
| `'static` | Lives forever | `global() -> &'static` |
| `*const` | Const raw pointer (unsafe) | `data() const -> *const` |
| `*mut` | Mutable raw pointer (unsafe) | `data() -> *mut` |
| `'a` | Named lifetime parameter | Used in constraints |

## Lifetime Constraints

| Constraint | Meaning | Example |
|-----------|---------|---------|
| `'a: 'b` | 'a outlives 'b | `where 'a: 'b` |
| `param: 'a` | Parameter has lifetime 'a | `where str: 'a` |
| `return: 'a` | Return has lifetime 'a | `where return: 'a` |
| `'a: 'b: 'c` | Transitive outlives | `where 'a: 'b, 'b: 'c` |

## Common Patterns

### 1. Borrowing Pattern
```cpp
// Return borrows from parameter
// @lifetime: (&'a) -> &'a
const T& get(const Container& c);
```

### 2. Factory Pattern
```cpp
// Creates new owned object
// @lifetime: owned
std::unique_ptr<T> make();
```

### 3. Transformation Pattern
```cpp
// Takes reference, returns owned
// @lifetime: (&'a) -> owned
std::string toString(const Data& d);
```

### 4. Selector Pattern
```cpp
// Selects one of the inputs
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& choose(const T& first, const T& second);
```

### 5. Mutator Pattern
```cpp
// Modifies in place
// @lifetime: (&'a mut) -> void
void update(Data& data);
```

## Annotation Priority

When multiple annotations could apply, priority is:

1. In-place annotations (highest priority)
2. External function-specific annotations
3. External pattern matches
4. Profile annotations
5. Default rules (lowest priority)

## File Organization

Recommended file structure:

```
project/
├── src/
│   ├── main.cpp           # Your code with in-place annotations
│   └── utils.cpp          # More code with in-place annotations
├── include/
│   ├── stl_lifetimes.hpp       # STL annotations (from RustyCpp)
│   ├── external_annotations.hpp # External function annotations
│   └── project_annotations.hpp  # Your project-specific annotations
└── third_party/
    └── library/           # Unannotated third-party code
```

## Safety Rules Matrix

### Calling Permissions

| Caller → Can Call | @safe | @unsafe | Undeclared |
|-------------------|-------|---------|------------|
| **@safe**         | ✅ Yes | ✅ Yes  | ❌ No      |
| **@unsafe**       | ✅ Yes | ✅ Yes  | ✅ Yes     |
| **Undeclared**    | ✅ Yes | ✅ Yes  | ✅ Yes     |

### Key Insights:
- **@unsafe ≠ Undeclared**: Unsafe is explicitly marked (audited), undeclared is not
- **Safe enforces auditing**: Can't call unaudited code
- **Undeclared can call undeclared**: Legacy code continues to work without modification
- **STL is undeclared by default**: Must explicitly mark before use in safe code
- **Audit ratchet**: Once marked @safe, all dependencies must be explicitly audited

### Example Scenarios

```cpp
// Scenario 1: Legacy code chains work fine
void helper() { }           // undeclared
void process() { helper(); } // undeclared calling undeclared ✅
void init() { process(); }   // undeclared calling undeclared ✅

// Scenario 2: Safe function forces auditing
// @safe
void secure_function() {
    // helper();  // ERROR: safe cannot call undeclared ❌
    // Must explicitly mark helper as @safe or @unsafe first
}

// Scenario 3: Unsafe as escape hatch
// @unsafe
void low_level_function() {
    helper();     // OK: unsafe can call undeclared ✅
    process();    // OK: unsafe can call undeclared ✅
    init();       // OK: unsafe can call undeclared ✅
}
```

## Best Practices

### 1. Gradual Adoption
- Start by marking obviously safe functions as `@safe`
- Mark dangerous functions as `@unsafe` 
- Leave legacy code undeclared initially
- Gradually audit and mark undeclared functions
- Use external annotations for dependencies

### 2. Annotation Placement
- In-place: Directly above declaration
- External: In dedicated header files
- STL: Include provided headers

### 3. Clarity Over Brevity
```cpp
// Good: Clear what's happening
// @lifetime: (&'container, size_t) -> &'container

// Less clear:
// @lifetime: (&'a, size_t) -> &'a
```

### 4. Document Complex Lifetimes
```cpp
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
// Returns first parameter, which must outlive second
const T& keepFirst(const T& long_lived, const T& short_lived);
```

## Common Errors and Solutions

### Error: "Cannot borrow as mutable while immutable borrow exists"
```cpp
// Problem:
const T& ref = container.get();
container.modify();  // ERROR

// Solution: Limit scope of borrows
{
    const T& ref = container.get();
    // use ref
}  // ref out of scope
container.modify();  // OK now
```

### Error: "Use after move"
```cpp
// Problem:
auto ptr2 = std::move(ptr1);
ptr1->method();  // ERROR

// Solution: Don't use after move
auto ptr2 = std::move(ptr1);
ptr2->method();  // Use ptr2 instead
```

### Error: "Lifetime constraint not satisfied"
```cpp
// Problem:
// @lifetime: (&'a, &'b) -> &'a
// Missing: where 'a: 'b

// Solution: Add constraint
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
```

## Tool Support

### Command Line
```bash
# Check file with annotations
rusty-cpp-checker annotated_file.cpp

# With external annotations
rusty-cpp-checker -I include file.cpp
```

### Build Integration
```cmake
# CMake
target_compile_definitions(project PRIVATE
    RUSTYCPP_ANNOTATIONS_ENABLED=1
)
```

## Debugging Annotations

### Verbose Output
```bash
# See how annotations are parsed
rusty-cpp-checker -vv file.cpp

# JSON output for tooling
rusty-cpp-checker --format json file.cpp
```

### Common Issues
1. **Not recognized**: Check exact syntax
2. **Too restrictive**: Consider `owned` instead of borrowed
3. **Missing**: Check file is included
4. **Conflicting**: In-place overrides external

## Future Extensions

Planned improvements:
- Lifetime elision (automatic inference)
- Method lifetime annotations
- Generic lifetime parameters
- Async lifetime tracking
- IDE quick fixes