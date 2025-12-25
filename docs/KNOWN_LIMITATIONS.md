# Known Limitations

This document describes known limitations of the rusty-cpp borrow checker that may cause false positives or require workarounds.

## Loop-Local Variable Move Detection (False Positive)

### Problem

The borrow checker's loop analysis simulates 2 iterations to detect use-after-move errors. However, it incorrectly tracks moved state across iterations for variables declared **inside** the loop body.

In C++, a variable declared inside a loop body is a fresh variable on each iteration - it's not the same variable being reused. The checker doesn't understand this and reports false positives.

### Minimal Example

```cpp
#include <list>
#include <memory>

struct Request {
    int data;
};

void process(std::unique_ptr<Request> req);

// @safe
void handle_requests(std::list<std::unique_ptr<Request>>& requests) {
    // @unsafe
    {
        while (!requests.empty()) {
            // This creates a FRESH variable each iteration
            std::unique_ptr<Request> req = std::move(requests.front());
            requests.pop_front();

            // Use the request
            if (req->data > 0) {
                process(std::move(req));  // Move req
            }
            // req goes out of scope here
        }
    }
}
```

**Expected behavior**: No error - each iteration has its own `req` variable.

**Actual behavior**:
```
Use after move: variable 'req' has already been moved
```

The checker sees:
1. Iteration 1: `req` created, moved at `process(std::move(req))`
2. Iteration 2: `req` used at `req->data` → ERROR (thinks it's the same moved variable)

### Why This Happens

The checker's loop analysis (documented in CLAUDE.md):
- Simulates 2 iterations to catch errors on second pass
- Tracks moved state across loop iterations
- Does NOT reset moved state for variables declared inside the loop body

This is a fundamental limitation of the current scope tracking implementation.

### Workarounds

#### 1. Mark the function as @unsafe (Recommended)

If the loop pattern is correct, mark the function as `@unsafe` with a comment explaining why:

```cpp
// @unsafe - Loop-local variable move is safe but checker has false positive
void handle_requests(std::list<std::unique_ptr<Request>>& requests) {
    while (!requests.empty()) {
        std::unique_ptr<Request> req = std::move(requests.front());
        requests.pop_front();
        process(std::move(req));
    }
}
```

#### 2. Use an @unsafe block around the loop

```cpp
// @safe
void handle_requests(std::list<std::unique_ptr<Request>>& requests) {
    // @unsafe - Loop-local variable move; checker false positive
    {
        while (!requests.empty()) {
            std::unique_ptr<Request> req = std::move(requests.front());
            requests.pop_front();
            process(std::move(req));
        }
    }
}
```

**Note**: @unsafe blocks do NOT suppress use-after-move detection. The @unsafe annotation on the function itself is required.

#### 3. Refactor to avoid the pattern (Not recommended)

You could refactor to use indices or iterators, but this makes the code less idiomatic and harder to read. The workarounds above are preferred.

### Related Patterns

This limitation also affects:

1. **For-range loops with move**:
   ```cpp
   for (auto& item : container) {
       process(std::move(item));  // False positive on second iteration
   }
   ```

2. **Any loop with local variable that gets moved**:
   ```cpp
   for (int i = 0; i < n; i++) {
       auto obj = create_object();
       consume(std::move(obj));  // False positive
   }
   ```

### Future Fix

A proper fix would require the checker to:
1. Track variable declarations per-scope, not just per-function
2. Reset moved state when a variable goes out of scope
3. Recognize that loop body scope creates fresh variables each iteration

This would require changes to `src/analysis/ownership.rs` and the loop simulation logic.

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

*Last updated: December 2024*
