# Function Pointer Safety

RustyCpp provides type-safe wrappers for function pointers that encode safety information in the type system.

## The Problem

C++ doesn't distinguish at the type level between pointers to safe vs unsafe functions:

```cpp
void safe_func() { /* safe operations */ }
void unsafe_func() { int* p = nullptr; *p = 1; }

void (*fp)();  // Same type for both!

fp = &safe_func;   // Calling fp() should be safe
fp = &unsafe_func; // Calling fp() should require @unsafe
```

## Solution: SafeFn and UnsafeFn

Include `<rusty/fn.hpp>` to use type-safe function pointer wrappers:

```cpp
#include <rusty/fn.hpp>
```

### SafeFn - Safe Function Pointers

`SafeFn<Signature>` holds a pointer to a `@safe` function. The analyzer verifies at assignment time that the target is `@safe`.

```cpp
// @safe
void process(int x);

// @safe
void example() {
    rusty::SafeFn<void(int)> callback = &process;  // OK - process is @safe
    callback(42);  // OK - calling SafeFn is always safe
}
```

### UnsafeFn - Unsafe Function Pointers

`UnsafeFn<Signature>` can hold any function pointer. Calling requires an `@unsafe` block.

```cpp
// @unsafe
void dangerous(int x) { *(int*)x = 0; }

// @safe
void example() {
    rusty::UnsafeFn<void(int)> callback = &dangerous;  // OK
    // callback(42);  // ERROR - UnsafeFn has no operator()

    // @unsafe
    {
        callback.call_unsafe(42);  // OK in @unsafe block
    }
}
```

### Type Safety

The type system prevents mixing safe and unsafe:

```cpp
rusty::SafeFn<void(int)> safe = &safe_func;
rusty::UnsafeFn<void(int)> unsafe = &dangerous;

// safe = unsafe;   // ERROR - different types (compile error)
unsafe = safe;      // OK - @safe function can be stored in UnsafeFn
```

## Member Function Pointers

For member functions, use `SafeMemFn` and `UnsafeMemFn`:

```cpp
class Widget {
public:
    // @safe
    void handle(int x);

    // @unsafe
    void dangerous(int x);
};

// @safe
void example() {
    rusty::SafeMemFn<void (Widget::*)(int)> safe_mf = &Widget::handle;
    rusty::UnsafeMemFn<void (Widget::*)(int)> unsafe_mf = &Widget::dangerous;

    Widget w;
    safe_mf(w, 42);  // OK - safe to call

    // @unsafe
    {
        unsafe_mf.call_unsafe(w, 42);  // OK in @unsafe block
    }
}
```

Const member functions are also supported:

```cpp
rusty::SafeMemFn<int (Widget::*)() const> getter = &Widget::getValue;
```

## Virtual Dispatch

Member function pointers respect C++ virtual dispatch:

```cpp
class Base {
public:
    virtual void process(int x);
};

class Derived : public Base {
public:
    void process(int x) override;
};

void (Base::*mfp)(int) = &Base::process;
Derived d;
(d.*mfp)(42);  // Calls Derived::process via vtable!
```

### Safety with `@interface`

For safe virtual dispatch, use `@interface` classes. RustyCpp's inheritance safety system ensures all implementations match the interface's safety contract:

```cpp
// @interface
class IProcessor {
public:
    // @safe - ALL implementations must be @safe
    virtual int process(int x) = 0;
};

class Impl : public IProcessor {
public:
    // @safe (inherited from interface)
    int process(int x) override {
        return x * 2;  // Only safe operations allowed
    }
};

// @safe
void example() {
    // SAFE: All implementations of IProcessor::process are guaranteed @safe
    rusty::SafeMemFn<int (IProcessor::*)(int)> mf = &IProcessor::process;

    Impl impl;
    mf(impl, 42);  // Virtual dispatch is safe
}
```

| Scenario | Safe? |
|----------|-------|
| `SafeMemFn` on `@interface` method | ✅ Yes - all implementations match contract |
| `SafeMemFn` on non-interface virtual | ⚠️ Use with caution |
| Inheritance from non-interface | ⚠️ `@unsafe` by default |

See [inheritance_safety.md](inheritance_safety.md) for details on `@interface`.

## Comparison with Rust

| Rust | RustyCpp |
|------|----------|
| `fn(i32) -> i32` (safe) | `rusty::SafeFn<int(int)>` |
| `unsafe fn(i32) -> i32` | `rusty::UnsafeFn<int(int)>` |
| Calling safe fn | `f(42)` |
| Calling unsafe fn | `unsafe { f(42) }` |

## API Reference

### SafeFn<Ret(Args...)>

| Method | Description |
|--------|-------------|
| `SafeFn()` | Default constructor (null) |
| `SafeFn(Ret (*fn)(Args...))` | Construct from function pointer (analyzer checks @safe) |
| `Ret operator()(Args...)` | Call the function (safe) |
| `explicit operator bool()` | Check if non-null |
| `pointer get()` | Get underlying raw pointer |

### UnsafeFn<Ret(Args...)>

| Method | Description |
|--------|-------------|
| `UnsafeFn()` | Default constructor (null) |
| `UnsafeFn(Ret (*fn)(Args...))` | Construct from any function pointer |
| `Ret call_unsafe(Args...)` | Call the function (requires @unsafe) |
| `explicit operator bool()` | Check if non-null |
| `pointer get()` | Get underlying raw pointer |

### SafeMemFn<Ret (Class::*)(Args...)>

| Method | Description |
|--------|-------------|
| `SafeMemFn(pointer fn)` | Construct from member function pointer |
| `Ret operator()(Class&, Args...)` | Call on object reference |
| `Ret operator()(Class*, Args...)` | Call on object pointer |

### UnsafeMemFn<Ret (Class::*)(Args...)>

| Method | Description |
|--------|-------------|
| `UnsafeMemFn(pointer fn)` | Construct from member function pointer |
| `Ret call_unsafe(Class&, Args...)` | Call on object reference (requires @unsafe) |
| `Ret call_unsafe(Class*, Args...)` | Call on object pointer (requires @unsafe) |

## Summary

| Type | Assignment | Call |
|------|------------|------|
| `SafeFn<Sig>` | Analyzer verifies target is @safe | Always safe |
| `UnsafeFn<Sig>` | Any function allowed | Requires @unsafe |
| `SafeMemFn<Sig>` | Analyzer verifies target is @safe | Always safe |
| `UnsafeMemFn<Sig>` | Any member function allowed | Requires @unsafe |
| Raw `(*)` pointer | Any function | Requires @unsafe |

---

*See `include/rusty/fn.hpp` for the complete implementation.*
