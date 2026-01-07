# Pointer Safety in RustyCpp

RustyCpp treats raw pointers in `@safe` code similarly to how Rust treats references: they must always be valid and non-null at initialization, but can be dereferenced safely.

## Design Philosophy

### Why Not Follow Rust's "Pointers Are Unsafe" Model?

In Rust, raw pointers (`*const T`, `*mut T`) are inherently unsafe - any dereference requires an `unsafe` block. RustyCpp takes a different approach for C++. Here's why:

#### 1. C++ References Are Not First-Class Citizens

Rust's references (`&T`, `&mut T`) are powerful and flexible. C++ references have significant limitations:

```cpp
// Cannot rebind C++ references
int a = 1, b = 2;
int& ref = a;
ref = b;      // This assigns b's value to a, doesn't rebind!

// Cannot have reference members without constructor initialization
struct Bad {
    int& ref;  // Must be initialized in every constructor
};

// Templates with references need special treatment
template<typename T>
struct Container {
    T value;  // What if T = int&? Complex semantics!
};
```

Because C++ references can't do everything Rust references can, C++ programmers often use pointers where Rust would use references.

#### 2. Fundamental C++ Types Rely on Pointers

Many basic C++ operations produce pointers:

```cpp
// String literals decay to pointers
const char* str = "hello";  // Automatic decay

// C-strings are pointer-based
void process(const char* s);  // Standard C string API

// Arrays decay to pointers
int arr[10];
int* p = arr;  // Automatic decay

// Many C APIs use pointers
FILE* f = fopen("file.txt", "r");
```

If we marked all pointer operations as unsafe, these common patterns would require `@unsafe` blocks everywhere:

```cpp
// This would be unusable if pointers were always unsafe:
// @safe
void greet(const char* name) {  // How do you even call this safely?
    printf("Hello, %s\n", name);  // Pointer dereference = unsafe?
}
```

#### 3. Legacy Code Compatibility

Billions of lines of C/C++ code use pointers extensively. Making pointers inherently unsafe would mean:
- Most existing functions become `@unsafe`
- Gradual migration becomes impractical
- The `@safe` subset of C++ would be too restrictive

Our goal is to make refactoring existing C++ codebases feasible, not to create a theoretical ideal that's unusable in practice.

### Our Approach: Pointers as "Must-Be-Valid References"

Instead of "pointers are unsafe", we say "pointers must be valid":

| Aspect | Rust Raw Pointers | RustyCpp Pointers in @safe |
|--------|-------------------|---------------------------|
| Creation | Safe | Requires valid source |
| Dereference | Unsafe | Safe (validity guaranteed) |
| Null allowed | Yes | No |
| Philosophy | Defer safety to use | Prove safety at creation |

In `@safe` code, pointers are treated like Rust references:
- They must be initialized to a valid, non-null value
- They can be dereferenced safely (no null check required)
- The burden of ensuring validity is at initialization/assignment time, not at use time

This mirrors Rust's `&T` model - you prove safety once when creating the reference, then use it freely.

### Why Not Check at Dereference?

Checking for null at every dereference would:
1. Require runtime checks (performance cost)
2. Not prevent the real issue (creating invalid pointers)
3. Not match Rust's reference model

Instead, we ensure pointers are valid when created, making all subsequent uses safe.

## Rules for Raw Pointers in @safe Code

### Rule 1: No Uninitialized Pointers

```cpp
// @safe
void bad() {
    int* ptr;  // ERROR: Uninitialized pointer
}

// @safe
void good(int* valid_ptr) {
    int* ptr = valid_ptr;  // OK: Initialized from parameter
}
```

### Rule 2: No nullptr Initialization

```cpp
// @safe
void bad() {
    int* ptr = nullptr;  // ERROR: Cannot initialize to nullptr
}
```

### Rule 3: No nullptr Assignment

```cpp
// @safe
void bad(int* ptr) {
    ptr = nullptr;  // ERROR: Cannot assign nullptr
}
```

### Rule 4: Address-of Requires @unsafe

Taking an address creates a pointer from a reference, which requires explicit `@unsafe`:

```cpp
// @safe
void example() {
    int x = 42;

    // @unsafe
    {
        int* ptr = &x;  // OK: In @unsafe block
    }
}
```

## Pointer Members in Structs/Classes

### Basic Rule

In `@safe` structs, pointer members must be properly initialized:

```cpp
// @safe
struct Bad {
    int* ptr;  // ERROR: No initialization, could be garbage
};

// @safe
struct AlsoBad {
    int* ptr = nullptr;  // ERROR: Cannot initialize to nullptr
};
```

### Constructor-Initialized Pointer Members

RustyCpp allows `@safe` structs with pointer members IF there's no way to create an instance with an uninitialized or null pointer.

#### Pattern 1: Deleted Default Constructor

```cpp
// @safe - ALLOWED
struct SafeWrapper {
    int* ptr;

    SafeWrapper() = delete;  // No default construction

    // @unsafe - address-of requires unsafe
    SafeWrapper(int& value) : ptr(&value) {}  // Always non-null
};
```

#### Pattern 2: Only Parameterized Constructors

```cpp
// @safe - ALLOWED
struct AnotherSafe {
    int* ptr;

    // Only constructor requires a valid pointer
    // @unsafe
    SafeWrapper(int* p) : ptr(p) {}

    // No default constructor exists (suppressed by user-defined ctor)
};
```

#### Pattern 3: Multiple Constructors (ALL Must Initialize)

```cpp
// @safe - ALLOWED
struct MultiCtor {
    int* ptr;

    MultiCtor() = delete;

    // @unsafe
    MultiCtor(int* p) : ptr(p) {}

    // @unsafe
    MultiCtor(int& ref) : ptr(&ref) {}
};
```

### Invalid Patterns

#### Default Constructor Doesn't Initialize

```cpp
// @safe
struct Bad {
    int* ptr;

    Bad() {}  // ERROR: Default ctor doesn't initialize ptr

    // @unsafe
    Bad(int* p) : ptr(p) {}  // This one is fine, but Bad() isn't
};
```

#### One Constructor Doesn't Initialize

```cpp
// @safe
struct Bad {
    int* ptr;

    MultiCtor() = delete;

    // @unsafe
    Bad(int* p) : ptr(p) {}  // OK

    // @safe
    Bad(int x) {}  // ERROR: Doesn't initialize ptr
};
```

#### Initializer List Uses nullptr

```cpp
// @safe
struct Bad {
    int* ptr;

    Bad() = delete;

    // @safe
    Bad(int x) : ptr(nullptr) {}  // ERROR: Initializing to nullptr
};
```

## Working with Pointers Safely

### Using @unsafe Blocks

When you need raw pointer operations, use `@unsafe` blocks:

```cpp
// @safe
void process_data(int* data, size_t len) {
    // @unsafe
    {
        for (size_t i = 0; i < len; i++) {
            data[i] *= 2;  // Pointer arithmetic in @unsafe block
        }
    }
}
```

### Smart Pointer Alternative

Smart pointers are always allowed and provide additional safety:

```cpp
// @safe
struct Container {
    std::unique_ptr<int> ptr;  // OK: Smart pointer, not raw
    std::shared_ptr<Data> data;  // OK: Smart pointer
};
```

### Reference Members (Preferred)

Reference members are preferred over pointer members when possible:

```cpp
// @safe
struct Container {
    int& ref;  // OK: References must be bound at construction

    Container(int& r) : ref(r) {}
};
```

## Design Rationale

### Why Allow Constructor-Initialized Pointers?

Many valid C++ patterns require pointer members:
- Intrusive data structures
- Observer patterns
- Performance-critical code avoiding smart pointer overhead

By allowing pointer members when properly initialized, we support these patterns while maintaining safety.

### Why Require ALL Constructors to Initialize?

If any constructor can leave a pointer uninitialized, users could accidentally create unsafe instances:

```cpp
struct Wrapper {
    int* ptr;
    Wrapper() = delete;
    Wrapper(int* p) : ptr(p) {}
    Wrapper(int x) {}  // Oops! ptr is garbage
};

Wrapper w(42);  // Looks safe, but w.ptr is garbage!
```

### Why Check Initializer Lists for nullptr?

Initializing to nullptr defeats the purpose of having a non-null pointer:

```cpp
struct Wrapper {
    int* ptr;
    Wrapper(int x) : ptr(nullptr) {}  // "Initialized" but still null!
};
```

## Summary

| Context | nullptr Init | Uninitialized | Address-of |
|---------|-------------|---------------|------------|
| @safe local var | ERROR | ERROR | Requires @unsafe |
| @safe struct member | ERROR | ERROR (unless all ctors init) | N/A |
| @unsafe | OK | OK | OK |

The key principle: **In @safe code, pointers behave like references - always valid, no null checks needed.**
