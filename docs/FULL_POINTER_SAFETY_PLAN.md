# Full Pointer Safety Plan

## Background Discussion

### The Question

We currently treat C++ pointers like Rust references and apply borrow checking rules. But Rust distinguishes between:
- **References (`&T`, `&mut T`)**: Compile-time guaranteed safe
- **Raw pointers (`*const T`, `*mut T`)**: Unsafe to use

This raises the question: Are we missing safety checks by treating pointers as safe references? What can't we check?

### Current State

**What we check today:**
- ✅ Borrow rules (multiple readers XOR single writer)
- ✅ Use-after-move detection
- ✅ Scope-based lifetime tracking
- ✅ Use-after-free (for tracked new/delete)
- ✅ Pointer arithmetic flagged as unsafe
- ✅ `nullptr` literal flagged as unsafe

**What Rust guarantees for `&T` that we don't check:**
- ❌ Non-null (references are never null)
- ❌ Points to initialized data
- ❌ Array bounds safety
- ❌ Proper alignment
- ❌ Type safety (no punning)
- ❌ Pointer provenance

### Key Insight

**None of these missing checks are fundamentally impossible.** They require additional analysis passes but are all tractable with static analysis techniques used by other tools (Clang Static Analyzer, Facebook Infer, Coverity, etc.).

## Goals

**Primary Goal**: Achieve Rust-reference-level safety guarantees for C++ pointers in `@safe` code.

**Success Criteria**: When code passes rusty-cpp with all checks enabled, pointers have the same safety guarantees as Rust references:
1. Never null when dereferenced
2. Always point to initialized, valid memory
3. Array accesses are bounds-checked
4. No type confusion through casts
5. Proper alignment maintained

## Execution Plan

### Phase 1: Null Safety

**Goal**: Ensure pointers are never null when dereferenced in `@safe` code.

**Approach**: Dataflow analysis tracking "possibly null" state.

**States for each pointer variable:**
- `NonNull` - Definitely not null (e.g., `&x`)
- `Null` - Definitely null (e.g., `nullptr`)
- `MaybeNull` - Could be either (e.g., function parameter, conditional assignment)

**Rules:**
```cpp
// @safe
void example(int* param) {           // param is MaybeNull (unknown from caller)
    int x = 42;
    int* p1 = &x;                    // p1 is NonNull (address-of)
    int* p2 = nullptr;               // ERROR: nullptr in @safe (already flagged)
    int* p3 = param;                 // p3 is MaybeNull

    *p1;                             // OK: NonNull
    *p3;                             // ERROR: Dereferencing MaybeNull pointer

    if (p3 != nullptr) {
        *p3;                         // OK: Null check narrows to NonNull
    }

    int* p4 = condition ? &x : p3;   // p4 is MaybeNull (one branch is MaybeNull)
}
```

**Implementation:**
1. Add `NullState` enum to pointer tracking in analysis
2. Propagate null state through assignments and control flow
3. At dereference, require `NonNull` state
4. Support null-check narrowing in conditionals

**Estimated complexity**: Medium (2-3 days)

---

### Phase 2: Initialization Tracking

**Goal**: Ensure pointers only reference initialized memory.

**Approach**: Definite assignment analysis (like Java/C# compilers do).

**States for each variable:**
- `Uninitialized` - Declared but not assigned
- `Initialized` - Has been assigned a value
- `MaybeUninitialized` - Assigned in some paths but not all

**Rules:**
```cpp
// @safe
void example() {
    int x;                           // x is Uninitialized
    int* p = &x;                     // OK: Taking address is fine
    int y = *p;                      // ERROR: Dereferencing pointer to uninitialized memory

    x = 42;                          // x becomes Initialized
    int z = *p;                      // OK: x is now initialized

    int a;
    if (condition) {
        a = 1;
    }
    int* pa = &a;
    int b = *pa;                     // ERROR: a is MaybeUninitialized
}
```

**Implementation:**
1. Track initialization state per variable in scope
2. For pointers, track what they point to
3. At dereference, check target's initialization state
4. Handle control flow merging conservatively

**Estimated complexity**: Medium (2-3 days)

---

### Phase 3: Array Bounds Safety

**Goal**: Prevent out-of-bounds array access.

**Approach**: Two-pronged:
1. Track array sizes where statically known
2. Provide `rusty::Span<T>` for dynamic bounds checking

**Part A: Static array bounds**

```cpp
// @safe
void example() {
    int arr[10];                     // Known size: 10
    int* p = arr;                    // p points to arr, inherits bounds [0, 10)

    arr[9];                          // OK: 9 < 10
    arr[10];                         // ERROR: Index 10 out of bounds [0, 10)

    p[5];                            // OK: 5 < 10
    p++;                             // p now has bounds [0, 9)
    p[9];                            // ERROR: 9 >= 9 (remaining bounds)

    for (int i = 0; i < 10; i++) {
        arr[i];                      // OK: Loop bounds match array bounds
    }

    for (int i = 0; i <= 10; i++) {
        arr[i];                      // ERROR: i can be 10
    }
}
```

**Part B: Dynamic bounds with Span**

```cpp
#include <rusty/span.hpp>

// @safe
void process(rusty::Span<int> data) {  // Span carries size
    for (size_t i = 0; i < data.size(); i++) {
        data[i];                     // OK: Bounds checked
    }
    data[data.size()];               // ERROR: Out of bounds
}

// @safe
void example() {
    int arr[10];
    process(rusty::Span(arr));       // Span created with size 10

    // @unsafe
    {
        int* raw = arr;
        process(rusty::Span(raw, 10)); // Manual size in unsafe
    }
}
```

**Implementation:**
1. Track array declarations and their sizes
2. Track bounds through pointer arithmetic
3. At subscript, verify index against bounds
4. Implement `rusty::Span<T>` with size tracking
5. Analyze loop bounds for common patterns

**Estimated complexity**: High (1-2 weeks)

---

### Phase 4: Type Safety (No Punning)

**Goal**: Prevent type confusion through unsafe casts.

**Approach**: Flag dangerous casts as requiring `@unsafe`.

**Rules:**
```cpp
// @safe
void example() {
    float f = 3.14f;

    // Safe casts (value conversion)
    int i = static_cast<int>(f);     // OK: Numeric conversion

    // Unsafe casts (reinterpretation)
    int* ip = reinterpret_cast<int*>(&f);  // ERROR: reinterpret_cast requires @unsafe
    int* ip2 = (int*)&f;             // ERROR: C-style cast to different pointer type requires @unsafe

    // Const cast
    const int x = 42;
    int* p = const_cast<int*>(&x);   // ERROR: const_cast requires @unsafe

    // OK: Same-type pointer casts
    int* p1 = &i;
    int* p2 = static_cast<int*>(p1); // OK: Same type
    void* vp = static_cast<void*>(p1); // OK: To void* is safe
}
```

**Casts requiring @unsafe:**
- `reinterpret_cast` (always)
- `const_cast` (always)
- C-style casts between incompatible pointer types
- `static_cast` to unrelated pointer types

**Casts allowed in @safe:**
- Numeric conversions (`int` to `float`, etc.)
- Derived-to-base pointer casts
- Pointer to `void*`
- Same-type casts

**Implementation:**
1. Detect cast expressions by type
2. Classify casts as safe or unsafe
3. Flag unsafe casts outside `@unsafe` blocks

**Estimated complexity**: Low-Medium (1-2 days)

---

### Phase 5: Pointer Provenance

**Goal**: Prevent undefined behavior from comparing/subtracting pointers to different allocations.

**Approach**: Track allocation origin for each pointer.

**Rules:**
```cpp
// @safe
void example() {
    int a, b;
    int arr[10];

    int* pa = &a;
    int* pb = &b;
    int* parr = arr;
    int* parr2 = arr + 5;

    // Same allocation - OK
    ptrdiff_t diff1 = parr2 - parr;  // OK: Same array
    bool cmp1 = parr < parr2;        // OK: Same array

    // Different allocations - ERROR
    ptrdiff_t diff2 = pa - pb;       // ERROR: Different allocations
    bool cmp2 = pa < pb;             // ERROR: Different allocations (UB in C++)

    // Equality comparison is OK (just checks address)
    bool eq = pa == pb;              // OK: Equality is well-defined
}
```

**Implementation:**
1. Assign allocation ID to each stack variable, heap allocation, array
2. Track allocation ID through pointer assignments and arithmetic
3. At pointer subtraction or relational comparison, verify same allocation
4. Allow equality/inequality comparisons (well-defined in C++)

**Estimated complexity**: Medium (2-3 days)

---

### Phase 6: Alignment Safety

**Goal**: Ensure pointers maintain proper alignment for their type.

**Approach**: Track alignment through pointer arithmetic and casts.

**Rules:**
```cpp
// @safe
void example() {
    alignas(8) char buffer[64];

    // OK: Properly aligned
    int64_t* p1 = reinterpret_cast<int64_t*>(buffer);  // (in @unsafe block)

    // ERROR: Misaligned
    int64_t* p2 = reinterpret_cast<int64_t*>(buffer + 1);  // Misaligned by 1

    // Arithmetic can break alignment
    int* arr = new int[10];
    char* cp = reinterpret_cast<char*>(arr);
    cp++;                            // Now misaligned for int
    int* ip = reinterpret_cast<int*>(cp);  // ERROR: Potentially misaligned
}
```

**Implementation:**
1. Track alignment of allocations (from type or `alignas`)
2. Track alignment offset through pointer arithmetic
3. At cast to stricter alignment, verify alignment is maintained
4. This mostly matters in `@unsafe` blocks where casts are allowed

**Estimated complexity**: Medium (2-3 days)
**Note**: Lower priority since type punning already requires `@unsafe`

---

## Implementation Order

Recommended order based on impact and complexity:

| Phase | Feature | Impact | Complexity | Dependencies |
|-------|---------|--------|------------|--------------|
| 1 | Null Safety | High | Medium | None |
| 2 | Initialization | High | Medium | None |
| 4 | Type Safety | High | Low | None |
| 5 | Provenance | Medium | Medium | None |
| 3 | Array Bounds | High | High | Span type |
| 6 | Alignment | Low | Medium | Type Safety |

**Suggested timeline:**
- Week 1: Phases 1, 2, 4 (null, init, type safety)
- Week 2-3: Phase 3 (array bounds - most complex)
- Week 4: Phases 5, 6 (provenance, alignment)

## Design Decisions Needed

### 1. Function Parameters

How do we handle pointer parameters from unknown callers?

**Option A: Pessimistic (Rust-like)**
```cpp
// @safe
void func(int* p) {
    *p;  // ERROR: p is MaybeNull, must check first
}
```

**Option B: Trust annotations**
```cpp
// @safe
void func(int* p) {  // Implicit: p is NonNull because @safe
    *p;  // OK: @safe contract implies valid pointer
}

// @safe
void func2(int* _Nullable p) {  // Explicit nullable
    *p;  // ERROR: Must check
}
```

**Recommendation**: Option B with explicit nullability annotations, similar to Clang's `_Nullable`/`_Nonnull`.

### 2. Span Integration

Should `rusty::Span<T>` be required for all array access in `@safe`?

**Option A: Required**
- All pointer subscript `p[i]` requires `@unsafe`
- Must use `Span<T>` for bounds-safe access

**Option B: Best-effort**
- Static arrays with known bounds are checked
- Unknown bounds emit warning but compile

**Recommendation**: Option B initially, with clear warnings. Option A as an optional strict mode.

### 3. Gradual Adoption

How strict should we be for existing codebases?

**Levels:**
1. **Warn**: Report issues but don't fail
2. **Error**: Fail on safety violations (default)
3. **Strict**: All pointer subscripts require Span

**Recommendation**: Make level configurable via flag.

## Success Metrics

After full implementation, rusty-cpp should catch:

1. ✅ All null pointer dereferences (with proper annotations)
2. ✅ All uses of uninitialized memory
3. ✅ All static array bounds violations
4. ✅ All type punning through casts
5. ✅ All pointer provenance violations
6. ✅ All obvious alignment issues

**Test suite targets:**
- 50+ tests for null safety
- 30+ tests for initialization
- 50+ tests for array bounds
- 20+ tests for type safety
- 20+ tests for provenance
- 10+ tests for alignment

## Conclusion

Achieving Rust-reference-level safety for C++ pointers is **feasible**. The key insight is that Rust's raw pointers are "unsafe" because the *compiler* can't verify them - but we're building a *static analyzer* that can perform more sophisticated analysis.

With these phases implemented, `@safe` C++ code with rusty-cpp will have equivalent safety guarantees to Rust references:
- Non-null (with proper annotations)
- Valid (initialized, not dangling)
- Bounds-safe (with Span or known-size arrays)
- Type-safe (no punning without @unsafe)
- Properly aligned

This is the path to making C++ pointers truly as safe as Rust references.
