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

## Other Known Limitations

### Virtual Function Calls
- Basic method calls work
- Dynamic dispatch through virtual functions not fully analyzed

### Loop Counter Variables
- Variables declared in `for(int i=...)` not tracked in variables map
- Use `int i; for(i=0; ...)` if tracking is needed

### Exception Handling
- Try/catch blocks are ignored
- Stack unwinding not modeled

---

*Last updated: December 2025*
