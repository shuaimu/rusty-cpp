# Mixed Pointer-Reference Borrow Checking Plan

## Problem Statement

The current implementation skips borrow conflict checking entirely in @unsafe blocks. This means:
- References and pointers can conflict without detection
- A mutable reference to `x` followed by a pointer to `x` goes undetected
- The Rust-style borrow rules are not enforced uniformly

## Design Rationale

The @unsafe annotation should allow operations that the analyzer can't prove safe (like raw pointer creation), but should NOT disable fundamental borrow checking rules. The borrow checker's job is to prevent:
- Data races (multiple mutable access)
- Aliasing violations (mutable + immutable access)

These rules apply regardless of whether code is @safe or @unsafe.

## Current Behavior

In `src/analysis/mod.rs`, the `IrStatement::Borrow` handler does:

```rust
if ownership_tracker.is_in_unsafe_block() {
    // Still record the borrow for consistency
    ownership_tracker.add_borrow(from.clone(), to.clone(), kind.clone());
    ownership_tracker.mark_as_reference(to.clone(), *kind == BorrowKind::Mutable);
    return;  // <-- Skips borrow conflict checking!
}
```

This causes borrow conflicts between pointers and references to go undetected in @unsafe contexts.

## Solution

Modify the borrow handling to:
1. Check for borrow conflicts EVEN in @unsafe blocks
2. Only skip the "pointer creation requires unsafe" check
3. Apply Rust-style borrow rules uniformly:
   - Multiple immutable borrows allowed
   - Only one mutable borrow allowed
   - Cannot mix mutable and immutable borrows

## Implementation Changes

### src/analysis/mod.rs

Change the `IrStatement::Borrow` handler to:
1. Check borrow conflicts FIRST (before the unsafe block check)
2. Still record the borrow
3. The unsafe block check should only affect whether pointer operations are allowed, which is already handled in `pointer_safety.rs`

### Expected Behavior After Fix

| Scenario | Expected |
|----------|----------|
| `int& ref = x; int& ref2 = x;` | Error: double mutable borrow |
| `int& ref = x; int* ptr = &x;` | Error: pointer conflicts with mutable reference |
| `const int& ref = x; const int* ptr = &x;` | OK: multiple immutable borrows |
| `int* p = &x; int& ref = x;` | Error: reference conflicts with mutable pointer |

## Test Cases

1. Mutable reference then mutable pointer - should fail
2. Mutable pointer then mutable reference - should fail
3. Immutable reference then immutable pointer - should pass
4. Mutable reference then immutable pointer - should fail
5. Immutable reference then mutable pointer - should fail
