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

## Function Pointers Not Supported

### Current Status

Function pointers (both free function and member function pointers) are not currently tracked by RustyCpp.

### Why Function Pointers Could Be Safe

Unlike data pointers, function pointers have properties that make them inherently safer:

1. **Point to code, not data** - The target is compiled code in the text segment
2. **No deallocation** - Code isn't freed at runtime (except dlclose scenarios)
3. **No use-after-free** - The pointed-to code remains valid for the program's lifetime
4. **No mutation** - You can't modify code through a function pointer

Rust treats function pointers (`fn(args) -> ret`) as safe for these reasons.

### Why We Don't Support Them Yet

The challenge is **safety propagation**. A function pointer's safety depends on what it points to:

```cpp
void safe_func() { /* safe operations */ }
void unsafe_func() { int* p = nullptr; *p = 1; }

void (*fp)();  // Same type for both!

fp = &safe_func;   // Calling fp() should be safe
fp = &unsafe_func; // Calling fp() should require @unsafe
```

C++ doesn't distinguish at the type level between pointers to @safe vs @unsafe functions. Tracking this would require:
- Flow-sensitive analysis of function pointer assignments
- Annotations on function pointer types
- Conservative assumptions when the target is unknown

### Workaround

Mark code using function pointers as `@unsafe`:

```cpp
// @unsafe - uses function pointers
void dispatch(void (*handler)(Request)) {
    handler(req);  // Can't verify handler is @safe
}
```

### Future Consideration

If implemented, the rules would likely be:
- Declaring function pointers: Safe
- Assigning from @safe function: Safe
- Calling through pointer known to point to @safe function: Safe
- Calling through pointer to @unsafe function: Requires @unsafe
- Calling through pointer with unknown target: Requires @unsafe (conservative)

This mirrors Rust's distinction between `fn()` (safe) and `unsafe fn()` (unsafe to call).

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
