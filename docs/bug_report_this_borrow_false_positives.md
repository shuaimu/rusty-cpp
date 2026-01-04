# Bug Report: False Positives in Method Call Borrow Checking

## Summary

Recent changes in commit `86aa04a` ("Enforce borrow rules uniformly for pointers and references") introduced stricter borrow checking that generates false positives for common C++ patterns involving method calls on `this` and return value tracking.

## Update (After commit 4804911)

Commit `4804911` ("Fix false positives in method call borrow checking") addressed some issues:
- ✅ FIXED: "Cannot borrow from 'this': variable is not alive in current scope"
- ⚠️ PARTIALLY FIXED: Field borrow conflicts (smart method mutation heuristic added)
- ✅ FIXED: "Cannot return 'value' because it has been moved" (fixed in subsequent commit - see below)
- ❌ STILL PRESENT: Some field borrow conflicts with method calls on borrowed fields

## Update (After "Fix return value moved false positive" commit)

The "Cannot return 'value' because it has been moved" false positive has been fixed in two commits.

### Fix 1: Method calls (commit 277261a)

**Root cause**: The `extract_return_source` function in `src/ir/mod.rs` was incorrectly returning the first argument of a function call as the "source" variable for return statements. For method calls like `opt.unwrap()`, this meant returning `opt` as the source, even though `opt` is consumed by `unwrap()` and the actual return value is the RESULT of `unwrap()`, not `opt` itself.

**Fix**: The function now distinguishes between:
1. **Method calls** (function name contains `::`): Return `None` because the result is a new value, not the receiver
2. **Constructor calls** (no `::`): Return the first argument as source for dangling reference detection

This correctly handles:
- `return opt.unwrap()->id;` - No false positive, `opt` is consumed but we're returning the result
- `return Holder{x};` - Still detects dangling ref if `x` is local

### Fix 2: Move arguments in function calls (current fix)

**Root cause**: The `extract_return_source` function had a loop that checked for Move expressions inside function call arguments:
```rust
for arg in args.iter() {
    if let Expression::Move { .. } = arg {
        return extract_return_source(arg, statements);
    }
}
```

This incorrectly tracked the moved variable as the return source. For `return UnsafeCell<T>(std::move(value));`:
1. The Move in the argument would be detected
2. `value` would be returned as the source
3. Since `value` was already marked as Moved (from processing the function call args), the error triggered

**Fix**: Removed this loop. For `return Constructor(std::move(x))`:
- The return value is the Constructor result, NOT `x`
- `x` is consumed by the constructor and we're returning a NEW value
- The Move is already tracked separately when processing function call arguments

## Affected Version

Original: Commit `86aa04a Enforce borrow rules uniformly for pointers and references`
Current: Commit `4804911 Fix false positives in method call borrow checking`

## Remaining Symptoms

After all fixes, the following patterns remain:

1. ✅ **FIXED: "Cannot return 'value' because it has been moved"**
   - Both Example 1 and Example 2 are now fixed
   - The fix removes incorrect tracking of moved variables in function call arguments

2. **"Cannot call method on 'this.epochs_': field is mutably borrowed by ei"**
   - Still occurs in some cases despite the smart method heuristic
   - Need to improve method call conflict detection

3. **"Cannot move out of 'rhs' because it is behind a reference"**
   - Need to investigate if this is a false positive or legitimate

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
