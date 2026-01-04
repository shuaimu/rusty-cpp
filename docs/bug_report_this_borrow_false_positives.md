# Bug Report: False Positives in Method Call Borrow Checking

## Summary

Recent changes in commit `86aa04a` ("Enforce borrow rules uniformly for pointers and references") introduced stricter borrow checking that generates false positives for common C++ patterns involving method calls on `this` and return value tracking.

## Affected Version

Commit: `86aa04a Enforce borrow rules uniformly for pointers and references`

## Symptoms

The borrow checker reports the following errors across multiple files in real-world C++ code:

1. **"Cannot return 'value' because it has been moved"**
   - Occurs in functions that return values
   - False positive: the value is not actually moved before return

2. **"Cannot borrow from 'this': variable is not alive in current scope"**
   - Occurs frequently when calling methods on object members
   - False positive: `this` is clearly alive during method execution

3. **"Cannot modify field 'm_pNode' in const method"**
   - Occurs with interior mutability patterns
   - False positive: code uses valid interior mutability (rusty::Cell, etc.)

4. **"Cannot call method on 'this.epochs_': field is borrowed by ei"**
   - Occurs with field access patterns
   - False positive: sequential field accesses that don't actually conflict

## Reproduction

These errors occur when running the borrow checker on the Janus/Mako codebase:

```bash
# In janus project directory
cmake -DENABLE_BORROW_CHECKING=ON ..
make borrow_check_rrr
```

Files affected include:
- `src/rrr/reactor/event.cc`
- `src/rrr/reactor/coroutine.cc`
- `src/rrr/misc/marshal.cpp`
- `src/rrr/rpc/server.cpp`
- `src/deptran/*.cc` (multiple files)

## Expected Behavior

The borrow checker should not report errors for:
1. Valid return statements where the value hasn't been moved
2. Method calls on `this` during object lifetime
3. Interior mutability patterns using `rusty::Cell` and similar
4. Sequential field accesses that don't actually conflict

## Workaround

Temporarily disable borrow checking for affected files by using explicit empty file lists in CMakeLists.txt instead of glob patterns.

## Impact

This issue blocks the "Make rrr code rusty-cpp safe" task in the Janus/Mako project, as the borrow checker cannot be used to validate code safety.

## Additional Context

The code being checked:
- Uses rusty-cpp types (Cell, RefCell, Arc, Rc, etc.)
- Has safety annotations (@safe, @unsafe)
- Follows Rust naming conventions

The errors appear to be related to how the checker:
1. Tracks `this` pointer lifetime in method contexts
2. Handles return value move semantics
3. Analyzes field borrowing through method calls
