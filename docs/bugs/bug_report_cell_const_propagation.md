# Bug Report: Const Propagation Incorrectly Flags Cell::set() Calls

## Summary

The const propagation checker incorrectly flags calls to `Cell::set()` as violations, even though `Cell::set()` is a `@safe` method that internally handles the unsafe operation with an `@unsafe` block.

## Expected Behavior

When a `@safe` function contains an internal `@unsafe` block that handles the unsafe operation, callers should NOT need to wrap calls in `@unsafe` blocks. The internal `@unsafe` should be sufficient.

## Actual Behavior

The const propagation checker reports violations when calling `Cell::set()` from a const method, despite `Cell::set()` having an internal `@unsafe` block.

## Minimal Reproduction

```cpp
// test_cell_const_propagation.cpp
#include <rusty/cell.hpp>

// @safe
class Counter {
private:
    rusty::Cell<int> count_{0};

public:
    // @safe - This should NOT trigger a violation
    // Cell::set() is @safe with internal @unsafe block
    void increment() const {
        count_.set(count_.get() + 1);
    }

    // @safe
    int get() const {
        return count_.get();
    }
};

int main() {
    Counter c;
    c.increment();  // Should work fine
    return c.get();
}
```

## Error Message

```
In function 'Counter::increment': Const propagation violation at line X:
calling non-const method 'rusty::Cell::set' through const object 'this'.
In @safe code, const propagates through pointer members.
```

## Analysis

Looking at `Cell::set()` in `cell.hpp`:

```cpp
// @safe - Set the value
void set(T val) const {
    // @unsafe
    { *value.get() = val; }
}
```

The method is:
1. Marked `@safe` at the function level
2. Has an internal `@unsafe` block that handles the actual mutation via `UnsafeCell`

The const propagation checker is treating `Cell::set()` as if it's an unsafe operation that requires `@unsafe` at the call site. But this defeats the purpose of having safe interior mutability wrappers like `Cell`.

## Root Cause

The const propagation checker sees:
1. `increment()` is a `const` method
2. It calls `set()` which mutates internal state
3. It reports a violation

The checker is not recognizing that:
- `Cell::set()` is explicitly marked `@safe`
- The unsafe mutation is already handled internally with `@unsafe` block
- `Cell` is an interior mutability primitive - calling its `@safe` methods from const contexts is the whole point

## Proposed Fix

The const propagation checker should:
1. Check if the called method is marked `@safe`
2. If yes, trust that any unsafe operations are handled internally
3. Only report violations for calls to methods that are NOT `@safe` and perform mutation

Alternatively, the checker could have a whitelist of known interior mutability types (`Cell`, `RefCell`, `Mutex`, `Condvar`, etc.) whose mutating methods are allowed in const contexts.

## Workaround

Currently, users must wrap every `Cell::set()` call in an `@unsafe` block:

```cpp
void increment() const {
    // @unsafe
    { count_.set(count_.get() + 1); }
}
```

This is verbose and defeats the ergonomic benefit of having `Cell` as a safe wrapper.

## Impact

This bug makes `Cell` (and likely `RefCell`, `Mutex`, etc.) impractical to use, as every mutation requires an explicit `@unsafe` block at the call site.
