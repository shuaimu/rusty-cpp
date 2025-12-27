# Known Limitations

This document describes known limitations of the rusty-cpp borrow checker that may cause false positives or require workarounds.

## ~~Loop-Local Variable Move Detection~~ (Fixed December 2025)

**This issue has been fixed.** Loop-local variables are now correctly tracked and do not produce false positives.

The checker now tracks variables declared inside loop bodies via:
- `CallExpr { result: Some(var) }` - function call results
- `Move { to: var }` - move-initialization
- `Assign { lhs: var }` - assignment
- `Borrow { to: var }` - reference creation

These variables are recognized as fresh each iteration and their moved state is properly reset.

### What Works Now

```cpp
// @safe
void handle_requests(std::list<std::unique_ptr<Request>>& requests) {
    // @unsafe
    {
        while (!requests.empty()) {
            // Fresh variable each iteration - NOW WORKS CORRECTLY
            std::unique_ptr<Request> req = std::move(requests.front());
            requests.pop_front();
            process(std::move(req));  // OK - req is fresh each iteration
        }
    }
}

// Also works:
for (int i = 0; i < n; i++) {
    auto obj = create_object();
    consume(std::move(obj));  // OK - obj is fresh each iteration
}
```

## Lambda Variable Declaration (Known Limitation)

### Problem

Variables declared via lambda expressions (e.g., `auto fn = [...]`) are not tracked as loop-local variables because the lambda declaration doesn't generate a proper variable declaration statement in the IR.

### Example

```cpp
// @safe
void test() {
    // @unsafe
    {
        for (int i = 0; i < 5; i++) {
            std::unique_ptr<int> data = std::make_unique<int>(i);
            auto fn = [d = std::move(data)]() mutable { };  // fn not tracked
            dispatch(std::move(fn));  // FALSE POSITIVE: "fn was moved in first iteration"
        }
    }
}
```

### Workaround

Mark the function as `@unsafe` if using lambda moves in loops:

```cpp
// @unsafe - Lambda variable declaration not tracked in loops
void test() {
    for (int i = 0; i < 5; i++) {
        std::unique_ptr<int> data = std::make_unique<int>(i);
        auto fn = [d = std::move(data)]() mutable { };
        dispatch(std::move(fn));  // OK in unsafe function
    }
}
```

## Address-of Operator on Member Function Pointers

### Problem

The address-of operator (`&`) applied to member functions (e.g., `&MyClass::method`) is flagged as unsafe because it creates a raw pointer-to-member-function at runtime. The borrow checker cannot statically verify how this pointer will be used.

### Example

```cpp
// @safe
class MyService : public Service {
public:
    int __reg_to__(Server& svr, size_t idx) override {
        // ERROR: Address-of operator in safe context
        return svr.reg_method(RPC_ID, idx, &MyService::handler);
    }
};
```

### Solution: C++17 Non-Type Template Parameters

Use `template<auto Func>` to pass the member function pointer as a compile-time constant. This avoids the runtime address-of operator:

```cpp
// In server.hpp
template<auto Func>
int reg_method(i32 rpc_id, size_t svc_index) {
    using S = member_function_class_t<Func>;  // Extract class from Func
    // Func is now a compile-time constant, not a runtime value
    handlers_[rpc_id] = [this, svc_index] (Request req) {
        S* svc = static_cast<S*>(services_[svc_index].get());
        (svc->*Func)(std::move(req));
    };
    return 0;
}
```

### Usage After Refactoring

```cpp
// @safe - Now works!
class MyService : public Service {
public:
    int __reg_to__(Server& svr, size_t idx) override {
        // Member function pointer is now a template argument (compile-time constant)
        return svr.reg_method<&MyService::handler>(RPC_ID, idx);
    }
};
```

### Helper Trait

To extract the class type from a member function pointer, use:

```cpp
template<typename T>
struct member_function_class;

template<typename R, typename C, typename... Args>
struct member_function_class<R (C::*)(Args...)> {
    using type = C;
};

template<typename R, typename C, typename... Args>
struct member_function_class<R (C::*)(Args...) const> {
    using type = C;
};

template<auto Func>
using member_function_class_t = typename member_function_class<decltype(Func)>::type;
```

This pattern makes RPC service registration safe by eliminating the runtime address-of operator while preserving the same functionality.

## Function Pointers ✅

**Status: Implemented (December 2025)**

RustyCpp provides type-safe wrappers for function pointers:

| Type | Description |
|------|-------------|
| `rusty::SafeFn<Sig>` | Holds @safe functions, safe to call |
| `rusty::UnsafeFn<Sig>` | Holds any function, requires @unsafe to call |
| `rusty::SafeMemFn<Sig>` | Holds @safe member functions |
| `rusty::UnsafeMemFn<Sig>` | Holds any member function |

```cpp
#include <rusty/fn.hpp>

// @safe
void example() {
    rusty::SafeFn<void(int)> safe_cb = &safe_func;  // Analyzer verifies @safe
    safe_cb(42);  // OK

    rusty::UnsafeFn<void(int)> unsafe_cb = &any_func;
    // @unsafe { unsafe_cb.call_unsafe(42); }  // Requires @unsafe block
}
```

**See [function_pointer_safety.md](function_pointer_safety.md) for full documentation.**

## String Literals and `const char*` ✅

**Status: Implemented (December 2025)**

### The Problem

String literals in C++ have type `const char[N]` but **decay** to `const char*` in most contexts:

```cpp
"hello"              // Type: const char[6]
const char* p = "hello";  // Decays to const char*
```

If we treat string literals the same as raw `const char*` pointers, common patterns become unusable.

### Our Approach: Literals Safe, Explicit `char*` Unsafe

**Key distinction:**
- **String literal expressions** (`"hello"`) are **safe** - they have static lifetime and cannot dangle
- **Explicit `char*` / `const char*` types** are **unsafe** - they are raw pointers

```cpp
// @safe
void example() {
    log("hello");              // OK - string literal is safe

    const char* ptr = "hello"; // UNSAFE - explicit char* type
    log(ptr);                  // UNSAFE - using char* variable
}
```

### Why String Literals Are Safe

String literals have special properties:

| Property | Safety Implication |
|----------|-------------------|
| Static storage duration | Cannot be freed, exists for entire program |
| Stored in .rodata | Read-only memory, cannot be corrupted |
| Compile-time known | Address fixed at link time |
| Immutable | No mutation concerns |

**String literals cannot dangle.** The expression `"hello"` is inherently safe.

### Why Explicit `char*` Is Unsafe

Once you have a `const char*` variable, the checker cannot know its origin:
- Did it come from a string literal? (safe)
- Did it come from `malloc`? (could be freed)
- Did it come from a local array? (could dangle)

Rather than complex lifetime tracking, we keep it simple: **explicit `char*` requires `@unsafe`**.

### Safe API Pattern: Wrap Unsafe in Constructor

APIs that need to accept string literals should use `@unsafe` blocks internally:

```cpp
class Logger {
public:
    // @safe - external interface is safe
    void log(const char* msg) {
        // @unsafe - internal use of char* is wrapped
        {
            internal_log(msg);
        }
    }
};

// @safe
void example() {
    Logger logger;
    logger.log("hello");  // OK - literal passed to safe API
}
```

### Alternative: Template API (No Decay)

For APIs that only accept literals, use a template that accepts arrays directly:

```cpp
template<size_t N>
void log(const char (&msg)[N]) {
    // msg is const char[N], not const char*
    // No decay, so we know it's a literal or static array
}

// @safe
void example() {
    log("hello");  // OK - array passed directly, no decay

    const char* ptr = "hello";
    log(ptr);      // ERROR - won't compile, ptr is not an array
}
```

### Comparison with Rust

Rust takes a similar approach:
- `&'static str` (string literals) are safe
- `*const c_char` (raw char pointer) requires `unsafe`

```rust
let s: &'static str = "hello";  // Safe
let p: *const i8 = s.as_ptr();  // Raw pointer
unsafe { use_ptr(p); }          // Must be in unsafe block
```

### Current Status

**Implemented!** String literals are now recognized and allowed in `@safe` code. The checker:

1. Detects `Expression::StringLiteral` in the AST
2. Allows string literals to be passed to functions with `const char*` parameters
3. Flags explicit `char*` variable declarations in `@safe` code
4. Supports the safe wrapper pattern (functions with `const char*` parameters can use `@unsafe` blocks internally)

### Implementation Details

The string literal tracking is implemented in:
- `src/parser/ast_visitor.rs` - `Expression::StringLiteral` variant
- `src/analysis/pointer_safety.rs` - `is_char_pointer_type()` function
- Tests in `tests/string_literal_tests.rs` and `tests/string_literals/`

## Other Known Limitations

### Virtual Function Calls
- Basic method calls work
- Dynamic dispatch through virtual functions not fully analyzed
- **Note**: Virtual dispatch has the same fundamental issue as function pointers - the actual function called is determined at runtime

### Loop Counter Variables
- Variables declared in `for(int i=...)` not tracked in variables map
- Use `int i; for(i=0; ...)` if tracking is needed

### Exception Handling
- Try/catch blocks are ignored
- Stack unwinding not modeled

---

*Last updated: December 2025*
