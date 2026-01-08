# RustyCpp Design Document & User Manual

A Rust-style borrow checker for C++ code.

---

## Table of Contents

- [Part I: Introduction](#part-i-introduction)
  - [1. Overview](#1-overview)
  - [2. Getting Started](#2-getting-started)
- [Part II: Core Concepts](#part-ii-core-concepts)
  - [3. Ownership Model](#3-ownership-model)
  - [4. Borrowing Rules](#4-borrowing-rules)
  - [5. Lifetimes](#5-lifetimes)
- [Part III: Safety Annotations](#part-iii-safety-annotations)
  - [6. The Safety Annotation System](#6-the-safety-annotation-system)
  - [7. What Gets Checked](#7-what-gets-checked)
- [Part IV: The Rusty Type Library](#part-iv-the-rusty-type-library)
  - [8. Smart Pointers](#8-smart-pointers)
  - [9. Interior Mutability](#9-interior-mutability)
  - [10. Optional & Error Types](#10-optional--error-types)
  - [11. Function Pointers](#11-function-pointers)
  - [12. Move Semantics](#12-move-semantics)
- [Part V: Analysis Features](#part-v-analysis-features)
  - [13. Borrow Checking](#13-borrow-checking)
  - [14. Move Analysis](#14-move-analysis)
  - [15. RAII & Container Safety](#15-raii--container-safety)
  - [16. Const Propagation](#16-const-propagation)
- [Part VI: Integration](#part-vi-integration)
  - [17. Build System Integration](#17-build-system-integration)
  - [18. Include Path Configuration](#18-include-path-configuration)
  - [19. Gradual Adoption Strategy](#19-gradual-adoption-strategy)
- [Part VII: Reference](#part-vii-reference)
  - [20. Annotation Reference](#20-annotation-reference)
  - [21. Error Messages](#21-error-messages)
  - [22. Limitations & Known Issues](#22-limitations--known-issues)
- [Part VIII: Advanced Topics](#part-viii-advanced-topics)
  - [23. Relocation vs Move Semantics](#23-relocation-vs-move-semantics)
  - [24. Thread Safety Model](#24-thread-safety-model)
  - [25. Initialization Analysis](#25-initialization-analysis)
  - [26. Lifetime Elision Rules](#26-lifetime-elision-rules)
  - [27. Pattern Matching & Sum Types](#27-pattern-matching--sum-types)
  - [28. Comparison with Safe C++ Proposal](#28-comparison-with-safe-c-proposal)
- [Part IX: Future Roadmap](#part-ix-future-roadmap)
  - [29. Planned Features](#29-planned-features)
  - [30. FAQ & Troubleshooting](#30-faq--troubleshooting)

---

# Part I: Introduction

## 1. Overview

### What is RustyCpp?

RustyCpp is a static analyzer that applies Rust's ownership and borrowing rules to C++ code. It catches memory safety issues at compile-time without runtime overhead.

**Key Features:**
- Detects use-after-move errors
- Prevents double mutable borrows
- Tracks reference lifetimes
- Catches dangling pointer/reference bugs
- Zero runtime cost (pure static analysis)

### Motivation: Memory Safety in C++

C++ provides powerful low-level control but lacks compile-time memory safety guarantees. After 15+ years of systems programming in C++, the most troubling failures remain memory-related: segmentation faults, memory corruptions, dangling pointers, and use-after-free bugs. These issues cause sleepless nights and can take months to diagnose.

Common memory safety bugs include:

- **Use-after-free**: Accessing memory after it's been deallocated
- **Double-free**: Freeing the same memory twice
- **Dangling references**: References to destroyed objects
- **Iterator invalidation**: Using iterators after container modification
- **Data races**: Concurrent mutable access to shared data

Rust solves these problems through its ownership system, enforced at compile time. However, rewriting existing C++ codebases in Rust is often impractical. RustyCpp brings Rust's safety guarantees to C++ through static analysis and opt-in safety annotations—without requiring you to leave the C++ ecosystem.

### Why Not Other Approaches?

Several approaches to C++ memory safety have been tried:

**Interop with Rust**: While some hope for seamless C++/Rust interoperability (like C++ has with C), deep integration between the two languages remains unlikely in the near term due to fundamental differences in their memory models.

**Macro-based solutions**: Google engineers explored using C++'s macro system to track borrows at compile time. This approach proved [impossible due to C++ language limitations](https://docs.google.com/document/d/e/2PACX-1vSt2VB1zQAJ6JDMaIA9PlmEgBxz2K5Tx6w2JqJNeYCy0gU4aoubdTxlENSKNSrQ2TXqPWcuwtXe6PlO/pub).

**Circle C++ / Safe C++**: The [Circle compiler](https://github.com/seanbaxter/circle) implements Rust-style borrow checking with new C++ syntax. While technically impressive, it requires a closed-source compiler and introduces breaking syntax changes. The [Safe C++ proposal](https://safecpp.org/draft.html) takes a similar approach but awaits standardization.

**RustyCpp's approach**: Rather than modifying the language or compiler, RustyCpp is a static analyzer that works with standard C++. It uses comment-based annotations (`@safe`, `@unsafe`) that are invisible to compilers, enabling gradual adoption without breaking existing code.

### How It Works (High-Level Architecture)

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   C++ Source    │────▶│   LibClang      │────▶│   RustyCpp IR   │
│   with @safe    │     │   Parser        │     │                 │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                         │
                        ┌─────────────────┐              │
                        │   Violations    │◀─────────────┤
                        │   Report        │              │
                        └─────────────────┘     ┌────────▼────────┐
                                                │   Analysis      │
                                                │   - Borrow Check│
                                                │   - Move Check  │
                                                │   - Lifetime    │
                                                └─────────────────┘
```

1. **Parse**: LibClang parses C++ source into an AST
2. **Transform**: AST is converted to RustyCpp's IR (Intermediate Representation)
3. **Analyze**: Multiple analysis passes check for violations
4. **Report**: Clear error messages with locations

### Comparison with Other Tools

| Tool | Type | When | Overhead | Coverage |
|------|------|------|----------|----------|
| **RustyCpp** | Static | Compile-time | None | Opt-in (@safe) |
| AddressSanitizer | Dynamic | Runtime | 2x slowdown | All executed paths |
| Valgrind | Dynamic | Runtime | 10-50x slowdown | All executed paths |
| Clang Static Analyzer | Static | Compile-time | None | Heuristic-based |

RustyCpp differs by using Rust's proven ownership model rather than heuristics, and by requiring explicit opt-in through annotations.

---

## 2. Getting Started

### Installation & Build Requirements

**Prerequisites:**
- Rust 1.70+ (for building the checker)
- LLVM/Clang 16+ (for LibClang)
- Z3 Solver (for constraint solving)

**Build from source:**
```bash
# Clone the repository
git clone <repository-url>
cd rusty-cpp

# Set environment variables
# macOS:
export Z3_SYS_Z3_HEADER=/opt/homebrew/include/z3.h

# Linux:
export Z3_SYS_Z3_HEADER=/usr/include/z3.h

# Build release binary
cargo build --release

# Binary is at: target/release/rusty-cpp-checker
```

### Quick Start Example

Create a file `example.cpp`:

```cpp
#include <rusty/box.hpp>

// @safe
void good_example() {
    auto ptr = rusty::Box<int>::make(42);
    int value = *ptr;  // OK: ptr is valid
}

// @safe
void bad_example() {
    auto ptr = rusty::Box<int>::make(42);
    auto ptr2 = std::move(ptr);  // ptr is moved
    int value = *ptr;  // ERROR: use after move
}
```

### Running the Checker

```bash
# Basic usage
./rusty-cpp-checker example.cpp

# With include paths
./rusty-cpp-checker example.cpp -I include -I /usr/local/include

# With compile_commands.json
./rusty-cpp-checker example.cpp --compile-commands build/compile_commands.json
```

### Understanding Output

```
Rusty C++ Checker
Analyzing: example.cpp
Auto-detected 7 C++ include path(s)
✗ Found 1 violation(s) in example.cpp:
In function 'bad_example': Use after move: variable 'ptr' was moved at line 12 and used at line 13
```

The output includes:
- Function name where the violation occurred
- Type of violation
- Line numbers for both the problematic operation and related context

---

# Part II: Core Concepts

## 3. Ownership Model

### Single Ownership Principle

In RustyCpp, every value has exactly one owner at any given time. When the owner goes out of scope, the value is destroyed.

```cpp
// @safe
void ownership_example() {
    auto box = rusty::Box<int>::make(42);  // box owns the int
    // box is the sole owner
}  // box goes out of scope, int is destroyed
```

### The Fundamental Difference: C++ vs Rust References

Understanding how RustyCpp bridges C++ and Rust reference models is key to using the tool effectively.

#### C++ References: Aliases

In C++, references are **aliases** - they are not first-class types. A reference is just another name for an existing object:

```cpp
int x = 42;
int& ref = x;      // ref is an alias for x
int y = ref;       // Same as: int y = x;
ref = 100;         // Same as: x = 100;
```

Key implications:
- You cannot "move" a reference - `std::move(ref)` moves the underlying object `x`
- References cannot be null or reseated
- References don't have their own identity separate from what they reference

#### Rust References: First-Class Types

In Rust, references are **first-class types** with their own ownership semantics:

```rust
let x = 42;
let r1: &mut i32 = &mut x;  // r1 is a mutable reference
let r2 = r1;                 // r1 is MOVED to r2, r1 is now invalid
// println!("{}", r1);       // ERROR: borrow of moved value
println!("{}", r2);          // OK: r2 owns the reference now
```

Key implications:
- `&mut T` (mutable reference) is **not Copy** - assigning moves it
- `&T` (immutable reference) **is Copy** - assigning copies it
- References have their own lifetime and can be invalidated

#### RustyCpp's Bridge

RustyCpp bridges this gap by:
1. **Tracking reference variables** as first-class entities
2. **Applying Rust semantics** to reference assignments at analysis time
3. **Providing `rusty::move`** for explicit Rust-like reference moves
4. **Forbidding `std::move` on references** in `@safe` code

### Move Semantics

Ownership can be transferred through moves:

```cpp
// @safe
void move_example() {
    auto box1 = rusty::Box<int>::make(42);
    auto box2 = std::move(box1);  // Ownership transferred to box2
    // box1 is now in a moved-from state
}
```

#### Reference Assignment Semantics

RustyCpp automatically applies Rust-like semantics to reference assignments:

| C++ Reference Type | Rust Equivalent | Assignment Behavior |
|--------------------|-----------------|---------------------|
| `T&` (non-const) | `&mut T` | **Move** - original becomes invalid |
| `const T&` | `&T` | **Copy** - both remain valid |
| `T&&` (named) | `&mut T` | **Move** - original becomes invalid |

**Mutable references move:**
```cpp
// @safe
void mutable_ref_moves() {
    int x = 42;
    int& r1 = x;      // r1 borrows x mutably
    int& r2 = r1;     // r1 is MOVED to r2 (Rust-like)

    int y = r2;       // OK: r2 is valid
    int z = r1;       // ERROR: r1 has been moved
}
```

**Const references copy:**
```cpp
// @safe
void const_ref_copies() {
    int x = 42;
    const int& r1 = x;    // r1 borrows x immutably
    const int& r2 = r1;   // r1 is COPIED to r2
    const int& r3 = r1;   // r1 can be copied multiple times

    int a = r1;           // OK: all are still valid
    int b = r2;           // OK
    int c = r3;           // OK
}
```

#### `std::move` vs `rusty::move`

| Type | `std::move` | `rusty::move` |
|------|-----------|-------------|
| Value `T` | Moves the value | Moves the value (same) |
| Mutable ref `T&` | Moves underlying object | Moves the **reference** |
| Rvalue ref `T&&` | Moves underlying object | Moves the **reference** |
| Const ref `const T&` | Returns rvalue ref | **Compile error** |

`rusty::move` provides Rust-like semantics for references:

```cpp
// @safe
void rusty_move_example() {
    int x = 42;
    int& ref = x;

    // rusty::move on a mutable reference invalidates it
    int& ref2 = rusty::move(ref);
    // ref is now invalid, ref2 is the active borrow
}
```

#### `std::move` Restrictions in @safe Code

In `@safe` code, using `std::move` on references is **forbidden** because it has confusing semantics - it moves the underlying object, not the reference:

```cpp
// The problem:
void problematic() {
    std::unique_ptr<int> ptr(new int(42));
    std::unique_ptr<int>& ref = ptr;

    // std::move(ref) moves *ptr*, not ref
    // ref still looks valid but points to a moved-from object
    std::unique_ptr<int> ptr2 = std::move(ref);

    // No compile error, but undefined behavior territory
}
```

| Expression | In @safe code |
|------------|---------------|
| `std::move(value)` | Allowed |
| `std::move(reference)` | **Forbidden** |
| `rusty::move(value)` | Allowed |
| `rusty::move(reference)` | Allowed |

### Use-After-Move Detection

RustyCpp detects when a moved-from variable is used:

```cpp
// @safe
void use_after_move() {
    auto ptr = rusty::Box<int>::make(42);
    auto ptr2 = std::move(ptr);

    *ptr;  // ERROR: Use after move
}
```

### Reassignment Recovery

Unlike true relocation, moved variables can become valid again through reassignment:

```cpp
// @safe
void recovery() {
    auto box = rusty::Box<int>::make(42);
    auto box2 = std::move(box);  // box is "moved"

    box = rusty::Box<int>::make(100);  // box is valid again
    *box;  // OK
}
```

---

## 4. Borrowing Rules

### Immutable vs Mutable Borrows

- **Immutable borrow** (`const T&`): Read-only access, multiple allowed
- **Mutable borrow** (`T&`): Read-write access, only one allowed

```cpp
// @safe
void borrow_example() {
    int x = 42;

    // Multiple immutable borrows: OK
    const int& r1 = x;
    const int& r2 = x;

    // Single mutable borrow: OK
    int& m1 = x;

    // But not both at once...
}
```

### The Exclusivity Rule

You can have either:
- **One or more** immutable borrows (`&T`), OR
- **Exactly one** mutable borrow (`&mut T`)

But never both simultaneously:

```cpp
// @safe
void exclusivity_violation() {
    int x = 42;
    int& mut_ref = x;      // Mutable borrow
    const int& ref = x;    // ERROR: Cannot borrow immutably while mutably borrowed
}
```

### Partial Borrows (Struct Fields)

Like Rust, RustyCpp can track borrows at the individual field level rather than treating structs as atomic units. Different fields of a struct can be borrowed independently:

```cpp
struct Point {
    int x;
    int y;
};

// @safe
void partial_borrow() {
    Point p{1, 2};
    int& x_ref = p.x;  // Borrow p.x
    int& y_ref = p.y;  // OK: p.y is independent

    int& x_ref2 = p.x;  // ERROR: p.x already borrowed
}
```

#### Conflict Detection

Same field cannot be borrowed mutably twice:

```cpp
// @safe
void bad_double_mutable_borrow() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    std::string& r1 = p.first;
    std::string& r2 = p.first;  // ERROR: field 'p.first' already mutably borrowed
}
```

#### Mixed Mutable/Immutable Conflicts

Mutable and immutable borrows of the same field conflict:

```cpp
// @safe
void bad_mixed_borrow() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    const std::string& r1 = p.first;  // Immutable borrow
    std::string& r2 = p.first;        // ERROR: cannot borrow mutably while immutably borrowed
}
```

#### Multiple Immutable Borrows Allowed

Multiple immutable borrows of the same field are permitted:

```cpp
// @safe
void multiple_immutable() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    const std::string& r1 = p.first;
    const std::string& r2 = p.first;  // OK: multiple immutable borrows allowed
    const std::string& r3 = p.first;  // OK
}
```

#### Whole-Struct vs Field Borrow Conflicts

Cannot borrow the whole struct while fields are borrowed, and vice versa:

```cpp
// @safe
void whole_vs_field() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    std::string& r = p.first;  // Borrow field
    Pair& q = p;               // ERROR: cannot borrow 'p' while 'p.first' is borrowed
}

// @safe
void field_vs_whole() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    Pair& q = p;               // Borrow whole struct
    std::string& r = p.first;  // ERROR: cannot borrow field while 'p' is borrowed
}
```

#### Nested Field Borrows

RustyCpp supports arbitrarily nested field paths:

```cpp
struct Inner { std::string data; int count; };
struct Outer { Inner inner; std::string name; };

// @safe
void nested_borrows() {
    Outer o;

    std::string& r1 = o.inner.data;   // Borrow nested field
    int& r2 = o.inner.count;          // OK: different field
    std::string& r3 = o.name;         // OK: different top-level field

    std::string& r4 = o.inner.data;   // ERROR: already borrowed
}
```

### Partial Moves

RustyCpp also tracks moves at the individual field level:

```cpp
// @safe
void partial_move_example() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    std::string a = std::move(p.first);   // Move only p.first
    std::string b = p.second;              // OK: p.second not moved

    std::string c = std::move(p.first);   // ERROR: field 'p.first' already moved
}
```

#### Whole-Struct Move After Partial Move

Cannot move the entire struct after a partial move:

```cpp
// @safe
void bad_whole_after_partial() {
    struct Pair { std::string first; std::string second; };
    Pair p{"hello", "world"};

    std::string a = std::move(p.first);  // Partial move
    Pair q = std::move(p);  // ERROR: Cannot move 'p' because partially moved
}
```

### Transitive Borrows

Borrows form chains that are tracked recursively:

```cpp
// @safe
void transitive_borrow() {
    int x = 42;
    int& ref1 = x;      // ref1 borrows x
    int& ref2 = ref1;   // ref2 borrows ref1 (transitively borrows x)

    x = 100;  // ERROR: Cannot modify x while borrowed through ref1 -> ref2
}
```

RustyCpp can handle borrow chains of any depth:

```cpp
// @safe
void long_chain() {
    int x = 42;
    int& r1 = x;
    int& r2 = r1;     // r1 moved to r2
    int& r3 = r2;     // r2 moved to r3
    int& r4 = r3;     // r3 moved to r4

    int y = r4;       // OK: only r4 is valid
    int z = r1;       // ERROR: r1 was moved
}
```

Error messages show the complete borrow chain:

```
ERROR: Cannot move 'x' because it is borrowed
  Borrow chain: ref3 -> ref2 -> ref1 -> x
```

---

## 5. Lifetimes

### Scope-Based Lifetime Tracking

Every variable has a lifetime determined by its scope:

```cpp
// @safe
void lifetime_example() {
    int* ptr;
    {
        int x = 42;
        ptr = &x;  // ptr borrows x
    }  // x's lifetime ends here

    *ptr;  // ERROR: Dangling pointer - x is out of scope
}
```

### Dangling Reference Detection

RustyCpp catches references that outlive their referents:

```cpp
// @safe
int& dangling_reference() {
    int x = 42;
    return x;  // ERROR: Returning reference to local variable
}
```

### Lifetime Annotations

For complex lifetime relationships, use `@lifetime` annotations:

```cpp
// @lifetime: (&'a, &'b) -> &'a
// Returns a reference with the same lifetime as the first parameter
const int& first(const int& a, const int& b) {
    return a;
}

// @lifetime: (&'a) -> &'a
// The returned reference lives as long as the input
const std::string& identity(const std::string& s) {
    return s;
}
```

### Cross-Function Lifetime Checking

The checker validates that lifetime annotations are respected:

```cpp
// @safe
void cross_function_example() {
    const int& ref = identity(42);  // ERROR: 42 is a temporary
    // identity returns &'a where 'a is the input's lifetime
    // But 42's lifetime ends at the semicolon
}
```

---

# Part III: Safety Annotations

## 6. The Safety Annotation System

### Two-State Model: `@safe` vs `@unsafe`

RustyCpp uses a simple two-state model:

| State | Meaning | Checked? |
|-------|---------|----------|
| `@safe` | Code follows Rust's safety rules | Yes |
| `@unsafe` | Code does unsafe things intentionally | No |
| (none) | Third-party/legacy code | No |

```cpp
// @safe
void checked_function() {
    // All borrow rules enforced here
}

// @unsafe
void unchecked_function() {
    // No checking - you're on your own
}

void legacy_function() {
    // No annotation = not checked (assumed third-party)
}
```

### Calling Rules Matrix

| Caller → Can Call | @safe | @unsafe |
|-------------------|-------|---------|
| **@safe**         | ✅ Yes | ❌ No (use `@unsafe` block) |
| **@unsafe**       | ✅ Yes | ✅ Yes  |

**Key Insight**: This is a clean two-state model. To call unsafe code from `@safe` functions, use an `@unsafe { }` block.

### Annotation Syntax

Annotations attach to the **next** code element only:

```cpp
// @safe - Apply to next element only
// @safe
void safe_function() {
    // ✅ CAN call other @safe functions
    safe_helper();

    // ❌ CANNOT call @unsafe functions directly
    // unsafe_func();  // ERROR

    // ✅ CAN call @unsafe via @unsafe block
    // @unsafe
    {
        unsafe_func();           // OK: in @unsafe block
        std::vector<int> vec;    // OK: STL in @unsafe block
    }

    // ❌ CANNOT do pointer operations (outside @unsafe block)
    // int* ptr = &x;  // ERROR: requires unsafe context
}

// @unsafe - Apply to next element only (or no annotation = same)
// @unsafe
void unsafe_function() {
    // ✅ Can call anything and do pointer operations
    safe_function();       // OK
    another_unsafe();      // OK
    int* ptr = nullptr;    // OK
    std::vector<int> vec;  // OK
}

// No annotation = @unsafe by default
void legacy_function() {
    // Treated as @unsafe
    // ✅ Can call anything
    std::vector<int> vec;  // OK
}
```

### Annotation Suffixes

Annotations support any suffix for documentation purposes:

```cpp
// @safe-verified on 2025-01-17
void audited_function() { }

// @unsafe: uses raw pointers for performance
void performance_critical() { }

// @safe, reviewed by security team
void reviewed_function() { }
```

### Annotation Hierarchy

Annotations cascade from outer to inner scopes:

```
Namespace → Class → Function
```

Inner annotations override outer ones:

```cpp
// @safe
namespace myapp {
    // All functions in this namespace are @safe by default

    void safe_by_inheritance() { }  // @safe (from namespace)

    // @unsafe
    void explicitly_unsafe() { }     // @unsafe (overrides namespace)

    // @unsafe
    class LegacyWrapper {
        void unsafe_method() { }     // @unsafe (from class)

        // @safe
        void safe_method() { }       // @safe (overrides class)
    };
}
```

### Header-to-Implementation Propagation

Safety annotations in headers automatically apply to implementations:

```cpp
// === math.h ===
// @safe
int calculate(int a, int b);

// @unsafe
void process_raw_memory(void* ptr);

// === math.cpp ===
#include "math.h"

int calculate(int a, int b) {
    // Automatically @safe from header
    return a + b;
}

void process_raw_memory(void* ptr) {
    // Automatically @unsafe from header
    // Pointer operations allowed
}
```

### `@unsafe` Blocks for Escape Hatches

Within a `@safe` function, use `@unsafe` blocks for specific unsafe operations:

```cpp
// @safe
void mostly_safe() {
    int x = 42;

    // @unsafe
    {
        // This block can call unsafe functions, use raw pointers, etc.
        legacy_c_function(&x);
        std::vector<int> vec;  // STL is @unsafe
        vec.push_back(x);
    }

    // Back to safe code
    int y = x + 1;
}
```

**Note:** The function is still checked as `@safe`. The `@unsafe` block only allows calling unsafe functions within it.

### Per-File Namespace Scope

Namespace annotations are **per-file**, not global:

```cpp
// === file1.cpp ===
// @safe
namespace myapp {
    void func1() { }  // @safe
}

// === file2.cpp ===
// @unsafe
namespace myapp {
    void func2() { }  // @unsafe - different file, different annotation
}
```

This enables gradual migration: annotate files independently.

### STL and External Code

All STL and external functions are `@unsafe` by default. To use them in `@safe` code:

**Option 1: Use `@unsafe` blocks**
```cpp
// @safe
void use_stl() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);  // OK in unsafe block
    }
}
```

**Option 2: Use Rusty structures (recommended)**
```cpp
// @safe
void use_rusty() {
    rusty::Vec<int> vec = {1, 2, 3};
    vec.push_back(4);  // No unsafe block needed
}
```

**Option 3: External annotations for audited functions**
```cpp
// @external: {
//   my_audited_function: [safe, () -> void]
// }

void my_audited_function();

// @safe
void caller() {
    my_audited_function();  // OK: marked [safe] via external annotation
}
```

---

## 7. What Gets Checked

### Summary Table

| Code Type | Borrow Checking | Move Checking | Lifetime Checking |
|-----------|-----------------|---------------|-------------------|
| `@safe` functions | Yes | Yes | Yes |
| `@unsafe` functions | No | No | No |
| Unannotated functions | No | No | No |
| `@unsafe` blocks in `@safe` | Yes* | Yes* | Yes* |

*`@unsafe` blocks inside `@safe` functions are still checked because the function itself is `@safe`.

### `@safe` Functions: Full Analysis

All analysis passes run on `@safe` code:

```cpp
// @safe
void fully_checked() {
    auto box = rusty::Box<int>::make(42);
    auto box2 = std::move(box);
    *box;  // ERROR: Use after move (detected)
}
```

### `@unsafe` Functions: Skipped

`@unsafe` functions are completely skipped:

```cpp
// @unsafe
void not_checked() {
    auto box = rusty::Box<int>::make(42);
    auto box2 = std::move(box);
    *box;  // No error reported - function is @unsafe
}
```

### Unannotated Code: Skipped

Third-party headers and legacy code without annotations are not checked:

```cpp
#include <yaml-cpp/yaml.h>  // No annotations

// @safe
void my_function() {
    YAML::Node node;  // yaml-cpp internals not checked
    node["key"] = "value";  // Only my_function is checked
}
```

---

# Part IV: The Rusty Type Library

## 8. Smart Pointers

### `rusty::Box<T>` (Unique Ownership)

Equivalent to Rust's `Box<T>` or C++'s `std::unique_ptr<T>`:

```cpp
#include <rusty/box.hpp>

// @safe
void box_example() {
    // Create a boxed value
    auto box = rusty::Box<int>::make(42);

    // Dereference
    int value = *box;

    // Move ownership
    auto box2 = std::move(box);

    // box is now invalid
}
```

### `rusty::Arc<T>` (Thread-Safe Shared Ownership)

Equivalent to Rust's `Arc<T>` or C++'s `std::shared_ptr<T>` with atomic reference counting:

```cpp
#include <rusty/arc.hpp>

// @safe
void arc_example() {
    auto arc1 = rusty::Arc<int>::make(42);
    auto arc2 = arc1;  // Clones the Arc (increments ref count)

    // Both arc1 and arc2 point to the same data
    // Data is freed when last Arc is destroyed
}
```

### `rusty::Rc<T>` (Single-Thread Shared Ownership)

Equivalent to Rust's `Rc<T>` - like `Arc` but not thread-safe (faster):

```cpp
#include <rusty/rc.hpp>

// @safe
void rc_example() {
    auto rc1 = rusty::Rc<int>::make(42);
    auto rc2 = rc1;  // Clones the Rc

    // NOT thread-safe - use Arc for multithreaded code
}
```

### `rusty::Ptr<T>` / `rusty::MutPtr<T>` (Raw Pointer Aliases)

Type aliases for raw pointers with Rust-like naming:

```cpp
#include <rusty/ptr.hpp>

// @unsafe
void ptr_example() {
    int x = 42;
    rusty::Ptr<int> p = &x;        // const int*
    rusty::MutPtr<int> mp = &x;    // int*
}
```

---

## 9. Interior Mutability

Interior mutability allows mutation through shared references (`const&`), enabling patterns that would otherwise require mutable references.

### When to Use Which Type

| Type | Thread Safe | Borrow Checking | Best For |
|------|-------------|-----------------|----------|
| `Cell<T>` | ❌ No | None | Small `Copy` types (int, bool, enum) |
| `RefCell<T>` | ❌ No | Runtime | Complex types needing borrow guards |
| `UnsafeCell<T>` | ❌ No | None | Building custom abstractions |

### `rusty::Cell<T>` (Copy Types)

Allows mutation through shared references for `Copy` types. Zero overhead - just stores the value directly.

```cpp
#include <rusty/cell.hpp>

// @safe
void cell_example() {
    rusty::Cell<int> cell(42);

    int value = cell.get();    // Returns a copy
    cell.set(100);             // Mutates through shared ref

    int old = cell.replace(200);  // Replace and get old value
}
```

**Constraints:** `T` must be trivially copyable (`std::is_trivially_copyable_v<T>`).

**Complete Cell API:**

| Method | Description |
|--------|-------------|
| `get()` | Returns a copy of the value |
| `set(val)` | Sets a new value |
| `replace(val)` | Sets new value, returns old |
| `swap(other_cell)` | Swaps values with another Cell |
| `take()` | Returns value, leaves default in place |
| `update(f)` | Applies function `f` to value in-place |
| `get_mut()` | Returns raw pointer (@unsafe) |

```cpp
// @safe
void cell_advanced() {
    rusty::Cell<int> cell(42);

    // Update in-place with function
    cell.update([](int x) { return x * 2; });  // cell is now 84

    // Swap two cells
    rusty::Cell<int> other(10);
    cell.swap(other);  // cell=10, other=84

    // Take value (for default-constructible types)
    int taken = cell.take();  // cell is now 0
}
```

### `rusty::RefCell<T>` (Runtime Borrow Checking)

Allows borrowing with runtime checks. Returns RAII guards (`Ref<T>`, `RefMut<T>`) that track borrows.

```cpp
#include <rusty/refcell.hpp>

// @safe
void refcell_example() {
    rusty::RefCell<std::string> cell("hello");

    {
        auto borrow = cell.borrow();      // Immutable borrow (Ref<T>)
        std::cout << *borrow << std::endl;
    }  // borrow released here

    {
        auto mut_borrow = cell.borrow_mut();  // Mutable borrow (RefMut<T>)
        *mut_borrow = "world";
    }  // mut_borrow released here

    // Panics at runtime if borrow rules violated:
    // auto b1 = cell.borrow_mut();
    // auto b2 = cell.borrow_mut();  // PANIC: already mutably borrowed
}
```

**RefCell Borrow Rules:**
- Multiple `borrow()` calls allowed simultaneously
- Only one `borrow_mut()` allowed at a time
- Cannot have `borrow()` and `borrow_mut()` simultaneously
- Violations throw `std::runtime_error`

**Complete RefCell API:**

| Method | Returns | Description |
|--------|---------|-------------|
| `borrow()` | `Ref<T>` | Immutable borrow guard |
| `borrow_mut()` | `RefMut<T>` | Mutable borrow guard |
| `can_borrow()` | `bool` | Check if immutable borrow possible |
| `can_borrow_mut()` | `bool` | Check if mutable borrow possible |
| `replace(val)` | `T` | Replace value (must not be borrowed) |
| `swap(other)` | `void` | Swap with another RefCell |
| `take()` | `T` | Take value, leave default |
| `get()` | `T` | Get copy (for copyable types) |

**Borrow Guards:**

```cpp
// Ref<T> - immutable borrow guard
{
    rusty::Ref<std::string> guard = cell.borrow();
    const std::string& value = *guard;  // operator*
    size_t len = guard->length();       // operator->
    std::string copy = guard.clone();   // explicit copy
}  // guard destroyed, borrow released

// RefMut<T> - mutable borrow guard
{
    rusty::RefMut<std::string> guard = cell.borrow_mut();
    std::string& value = *guard;  // mutable reference
    guard->append(" world");      // modify through ->
}  // guard destroyed, borrow released
```

### `rusty::UnsafeCell<T>` (Primitive)

The building block for interior mutability. No safety guarantees - you must ensure correctness manually.

```cpp
#include <rusty/unsafe_cell.hpp>

// @unsafe
void unsafe_cell_example() {
    rusty::UnsafeCell<int> cell(42);

    int* ptr = cell.get();  // @unsafe - returns raw pointer
    *ptr = 100;

    // Caller must ensure no data races or aliasing violations
}
```

**UnsafeCell Methods:**

| Method | Safety | Description |
|--------|--------|-------------|
| `get()` | @unsafe | Returns `T*` to inner value |
| `get_const()` | @safe | Returns `const T*` for reading |
| `get_mut()` | @safe | Returns `T&` (requires non-const method) |
| `as_mut_unchecked()` | @unsafe | Returns `T&` through shared access |
| `as_ref_unchecked()` | @unsafe | Returns `const T&` through shared access |
| `replace(val)` | @unsafe | Replace value, return old |

**Note:** UnsafeCell is typically used to build other abstractions like Cell and RefCell. Most code should use those higher-level types instead.

### Using Interior Mutability in Classes

```cpp
class Counter {
    rusty::Cell<int> count;  // Cell for trivially copyable types

public:
    Counter() : count(0) {}

    // @safe - Can mutate through const reference
    void increment() const {
        count.set(count.get() + 1);
    }

    // @safe - Read-only access
    int get() const {
        return count.get();
    }
};

class Cache {
    rusty::RefCell<std::map<int, std::string>> data;

public:
    Cache() : data() {}

    // @safe - Insert with mutable borrow
    void insert(int key, std::string value) const {
        auto guard = data.borrow_mut();
        (*guard)[key] = std::move(value);
    }

    // @safe - Lookup with immutable borrow
    bool contains(int key) const {
        auto guard = data.borrow();
        return guard->count(key) > 0;
    }
};
```

---

## 10. Optional & Error Types

### `rusty::Option<T>`

Represents an optional value:

```cpp
#include <rusty/option.hpp>

// @safe
void option_example() {
    rusty::Option<int> some = rusty::Some(42);
    rusty::Option<int> none = rusty::None;

    if (some.is_some()) {
        int value = some.unwrap();  // Panics if None
    }

    int value = some.unwrap_or(0);  // Returns 0 if None

    // Pattern matching style
    some.match(
        [](int v) { std::cout << "Got: " << v << std::endl; },
        []() { std::cout << "Got nothing" << std::endl; }
    );
}
```

### `rusty::Result<T, E>`

Represents either success or error:

```cpp
#include <rusty/result.hpp>

// @safe
rusty::Result<int, std::string> divide(int a, int b) {
    if (b == 0) {
        return rusty::Err<std::string>("division by zero");
    }
    return rusty::Ok(a / b);
}

// @safe
void result_example() {
    auto result = divide(10, 2);

    if (result.is_ok()) {
        int value = result.unwrap();
    }

    int value = result.unwrap_or(0);

    // Propagate errors
    // auto value = result?;  // Not available in C++, use unwrap_or/match
}
```

---

## 11. Function Pointers

### `rusty::SafeFn<Sig>` / `rusty::UnsafeFn<Sig>`

Type-safe function pointer wrappers:

```cpp
#include <rusty/fn.hpp>

// @safe
int safe_add(int a, int b) { return a + b; }

// @unsafe
int unsafe_add(int a, int b) { return a + b; }

// @safe
void fn_example() {
    // SafeFn can only hold @safe functions
    rusty::SafeFn<int(int, int)> safe_fn = &safe_add;
    int result = safe_fn(1, 2);  // OK: safe to call

    // UnsafeFn can hold any function
    rusty::UnsafeFn<int(int, int)> unsafe_fn = &unsafe_add;
    // unsafe_fn(1, 2);  // ERROR: requires @unsafe context

    // @unsafe
    {
        int result = unsafe_fn.call_unsafe(1, 2);  // OK in @unsafe block
    }
}
```

### `rusty::SafeMemFn<Sig>` / `rusty::UnsafeMemFn<Sig>`

For member function pointers:

```cpp
class Calculator {
public:
    // @safe
    int add(int a, int b) { return a + b; }
};

// @safe
void mem_fn_example() {
    rusty::SafeMemFn<int(Calculator::*)(int, int)> mem_fn = &Calculator::add;

    Calculator calc;
    int result = (calc.*mem_fn)(1, 2);
}
```

---

## 12. Move Semantics

### `rusty::move()` vs `std::move()`

```cpp
#include <rusty/move.hpp>

// @safe
void move_comparison() {
    int x = 42;
    int& ref = x;

    // std::move on reference: just casts to rvalue (reference still valid)
    // rusty::move on reference: invalidates the reference (Rust semantics)

    int& ref2 = rusty::move(ref);  // ref is now invalid
    // ref;  // ERROR: use after move
}
```

### `rusty::copy()` for Explicit Copies

When you want to be explicit about copying:

```cpp
// @safe
void copy_example() {
    int x = 42;
    int y = rusty::copy(x);  // Explicit copy

    // x is still valid
}
```

### Reference Assignment Semantics

| Operation | Mutable Ref (`T&`) | Const Ref (`const T&`) |
|-----------|-------------------|------------------------|
| `ref2 = ref1` | Move (ref1 invalid) | Copy (both valid) |
| `rusty::move(ref)` | Invalidates ref | Compile error |
| `rusty::copy(ref)` | Creates copy | Creates copy |

---

# Part V: Analysis Features

## 13. Borrow Checking

### Double Mutable Borrow Detection

```cpp
// @safe
void double_mutable() {
    int x = 42;
    int& ref1 = x;
    int& ref2 = x;  // ERROR: Cannot borrow 'x' as mutable more than once
}
```

### Mixed Mutable/Immutable Conflicts

```cpp
// @safe
void mixed_borrow() {
    int x = 42;
    int& mut_ref = x;
    const int& const_ref = x;  // ERROR: Cannot borrow 'x' as immutable
                                // because it is also borrowed as mutable
}
```

### Scope-Based Cleanup

Borrows are released when they go out of scope:

```cpp
// @safe
void scope_cleanup() {
    int x = 42;
    {
        int& ref = x;  // Borrow starts
    }  // Borrow ends here

    int& ref2 = x;  // OK: previous borrow is out of scope
}
```

---

## 14. Move Analysis

### Use-After-Move Detection

```cpp
// @safe
void use_after_move() {
    auto box = rusty::Box<int>::make(42);
    auto box2 = std::move(box);

    *box;  // ERROR: Use after move: 'box' was moved
}
```

### Loop Analysis (2-Iteration Simulation)

The checker simulates 2 loop iterations to catch move errors:

```cpp
// @safe
void loop_move() {
    auto box = rusty::Box<int>::make(42);

    for (int i = 0; i < 2; i++) {
        auto temp = std::move(box);  // ERROR: Use after move on second iteration
    }
}
```

### Reassignment Recovery

A moved variable becomes valid again after reassignment:

```cpp
// @safe
void reassignment() {
    auto box = rusty::Box<int>::make(42);
    auto box2 = std::move(box);  // box is moved

    box = rusty::Box<int>::make(100);  // box is valid again
    *box;  // OK
}
```

---

## 15. RAII & Container Safety

RAII tracking extends RustyCpp's borrow checking to handle C++-specific patterns where object lifetimes and resource ownership can lead to dangling references, use-after-free, and other memory safety issues.

### Reference/Pointer Stored in Container

Detects when a pointer or reference is stored in a container that outlives the pointee:

```cpp
// @safe
void bad_pointer_in_container() {
    std::vector<int*> vec;
    {
        int x = 42;
        vec.push_back(&x);  // Store pointer to x
    }  // x destroyed here

    // ERROR: vec[0] is now a dangling pointer
    *vec[0] = 10;
}
```

**Detected operations:**
- `push_back`, `push_front`, `insert`, `emplace`, `emplace_back`, `emplace_front`, `assign` with pointer/reference arguments
- Container types: `vector`, `list`, `deque`, `set`, `map`, `unordered_*`, `array`, `span`

### Iterator Outlives Container

Detects when an iterator survives longer than its source container:

```cpp
// @safe
void bad_iterator_outlives_container() {
    std::vector<int>::iterator it;
    {
        std::vector<int> v = {1, 2, 3};
        it = v.begin();  // it borrows from v
    }  // v destroyed here

    // ERROR: it is now invalid
    int val = *it;
}
```

**Detected iterator-returning methods:**
- `begin`, `end`, `cbegin`, `cend`, `rbegin`, `rend`, `find`, `lower_bound`, `upper_bound`

### Iterator Invalidation (Modification During Iteration)

```cpp
// @safe
void iterator_invalidation() {
    std::vector<int> vec = {1, 2, 3};
    auto it = vec.begin();

    vec.push_back(4);  // May invalidate iterators (reallocation)

    *it;  // ERROR: Iterator may be invalidated
}
```

### User-Defined RAII Types

RustyCpp recognizes classes with user-defined destructors as RAII types, enabling proper lifetime tracking:

```cpp
class FileHandle {
    FILE* f;
public:
    FileHandle(const char* path) : f(fopen(path, "r")) {}
    ~FileHandle() { if (f) fclose(f); }  // User-defined destructor
};

// RustyCpp now tracks FileHandle as an RAII type
```

### Member Lifetime Tracking

Detects when references to object members outlive the containing object:

```cpp
// @safe
void bad_member_reference() {
    const std::string* ptr;
    {
        struct Wrapper { std::string data; };
        Wrapper w;
        w.data = "hello";
        ptr = &w.data;  // ptr references w.data
    }  // w destroyed, w.data destroyed

    // ERROR: ptr is now dangling
    std::cout << *ptr;
}
```

### new/delete Tracking

Detects double-free and use-after-free with raw heap allocations:

```cpp
// @safe
void bad_double_free() {
    int* ptr = new int(42);
    delete ptr;
    delete ptr;  // ERROR: double free
}

// @safe
void bad_use_after_free() {
    int* ptr = new int(42);
    delete ptr;
    *ptr = 10;  // ERROR: use after free
}
```

### Lambda Capture Escape Analysis

RustyCpp uses escape analysis to allow safe reference captures while catching dangerous ones:

```cpp
// @safe
std::function<int()> bad_lambda_escape() {
    int x = 42;
    auto lambda = [&x]() { return x; };  // Captures x by reference
    return lambda;  // ERROR: lambda escapes, x will be destroyed
}

// @safe
void good_lambda_local_use() {
    int x = 42;
    auto lambda = [&x]() { return x; };  // OK: lambda doesn't escape
    int result = lambda();  // Used locally, x still alive
}
```

**Capture rules:**

| Capture Type | Status |
|--------------|--------|
| Reference (`[&]`, `[&x]`) in non-escaping lambda | Allowed |
| Reference captures that escape (returned, stored) | Forbidden |
| Copy captures (`[x]`, `[=]`) | Always allowed |
| Move captures (`[y = std::move(x)]`) | Always allowed |
| `this` capture | Always forbidden (raw pointer) |

### Return Reference to Local

```cpp
// @safe
const std::string& bad_return_ref() {
    std::string local = "hello";
    return local;  // ERROR: returning reference to local variable
}
```

### Comparison with Rust

| Feature | Rust | RustyCpp |
|---------|------|----------|
| Move detection | ✅ | ✅ |
| Use-after-move | ✅ | ✅ |
| Iterator outlives container | ✅ | ✅ |
| Reference in container | ✅ | ✅ |
| Lambda escape analysis | ✅ | ✅ |
| User-defined RAII | N/A | ✅ (C++ specific) |
| Non-Lexical Lifetimes (NLL) | ✅ | ❌ (may cause false positives) |

---

## 16. Const Propagation

### Pointer Member Mutations

The checker tracks const-correctness through pointer members:

```cpp
class Container {
    int* data;
public:
    void mutate() const {
        *data = 42;  // Mutating through pointer in const method
    }
};

// @safe
void const_propagation() {
    Container c;
    const Container& ref = c;
    ref.mutate();  // ERROR: Mutating through const reference
}
```

### Interior Mutability Handling

`Cell` and similar types are handled specially:

```cpp
class Wrapper {
    rusty::Cell<int> value;
public:
    // @safe
    void set(int v) const {
        value.set(v);  // OK: Cell provides interior mutability
    }
};
```

---

# Part VI: Integration

## 17. Build System Integration

### CMake Integration

```cmake
# Find the checker binary
find_program(RUSTY_CPP_CHECKER rusty-cpp-checker)

# Add a custom target to run the checker
add_custom_target(borrow_check
    COMMAND ${RUSTY_CPP_CHECKER}
        ${CMAKE_CURRENT_SOURCE_DIR}/src/main.cpp
        -I ${CMAKE_CURRENT_SOURCE_DIR}/include
    COMMENT "Running RustyCpp borrow checker"
)

# Or integrate with compile_commands.json
add_custom_target(borrow_check_all
    COMMAND ${RUSTY_CPP_CHECKER}
        --compile-commands ${CMAKE_BINARY_DIR}/compile_commands.json
        ${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp
    COMMENT "Running RustyCpp on all sources"
)
```

### Makefile Integration

```makefile
RUSTY_CPP_CHECKER := rusty-cpp-checker
INCLUDE_DIRS := -I include -I third_party/include

borrow_check: $(SOURCES)
	$(RUSTY_CPP_CHECKER) $(SOURCES) $(INCLUDE_DIRS)

.PHONY: borrow_check
```

### compile_commands.json Support

RustyCpp can read compiler flags from `compile_commands.json`:

```bash
# Generate compile_commands.json with CMake
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..

# Run checker with compile commands
./rusty-cpp-checker src/main.cpp --compile-commands build/compile_commands.json
```

---

## 18. Include Path Configuration

### CLI Flags

```bash
./rusty-cpp-checker file.cpp -I include -I /usr/local/include
```

### Environment Variables

```bash
export CPLUS_INCLUDE_PATH=/project/include:/third_party/include
export CPATH=/usr/include

./rusty-cpp-checker file.cpp
```

### Auto-Detection

RustyCpp automatically detects common include paths:
- `/usr/include`
- `/usr/local/include`
- System C++ headers (`/include/c++/`)
- LLVM/Clang headers

---

## 19. Gradual Adoption Strategy

### File-by-File Migration

Start by annotating one file at a time:

```cpp
// === legacy_code.cpp ===
// No annotations - not checked

// === new_code.cpp ===
// @safe
namespace myapp {
    // This file is checked
}
```

### Namespace-Level Annotations

Annotate entire namespaces:

```cpp
// @safe
namespace myapp::core {
    // All functions here are @safe
}

// @unsafe
namespace myapp::legacy {
    // All functions here are @unsafe (not checked)
}
```

### Mixing Safe and Unsafe Code

Use `@unsafe` blocks to call into legacy code:

```cpp
// @safe
void safe_wrapper() {
    // @unsafe
    {
        legacy_function();  // Call to unannotated code
    }

    // Continue with safe code
}
```

---

# Part VII: Reference

## 20. Annotation Reference

### Complete Syntax

```cpp
// Function annotation (attaches to next function)
// @safe
void safe_function() { }

// @unsafe
void unsafe_function() { }

// Class annotation (all methods inherit)
// @safe
class SafeClass {
    void method();  // @safe
};

// Namespace annotation (all contents inherit)
// @safe
namespace safe_ns {
    void func();  // @safe
}

// Block annotation (within @safe function)
// @safe
void func() {
    // @unsafe
    {
        // Unsafe code here
    }
}
```

### `@lifetime` Specification

```cpp
// Return reference with same lifetime as input
// @lifetime: (&'a) -> &'a
const T& identity(const T& x);

// Return reference with lifetime of first parameter
// @lifetime: (&'a, &'b) -> &'a
const T& first(const T& a, const T& b);

// Multiple constraints
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& select(const T& a, const T& b);

// Mutable reference
// @lifetime: (&'a mut) -> &'a mut
T& get_mut(T& x);

// Owned return (no lifetime)
// @lifetime: (&'a) -> owned
T clone(const T& x);
```

### External Annotations for Third-Party Code

For code you can't modify, use external annotation files:

```cpp
// @external: {
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
//   my_lib::safe_func: [safe, (&'a) -> &'a]
// }
```

---

## 21. Error Messages

### Common Violations and Fixes

| Error | Cause | Fix |
|-------|-------|-----|
| "Use after move" | Using variable after `std::move` | Don't use moved variables, or reassign first |
| "Cannot borrow as mutable more than once" | Multiple `&mut` | Use separate scopes or restructure |
| "Cannot borrow as immutable while mutably borrowed" | `&` and `&mut` conflict | Release mutable borrow first |
| "Returning reference to local variable" | Dangling reference | Return by value or use parameter lifetime |
| "Iterator may be invalidated" | Container modified during iteration | Copy or use indices |

### Understanding Borrow Chains

```
ERROR: Cannot move 'x' because it is borrowed
  Borrow chain: ref3 -> ref2 -> ref1 -> x

  ref1 borrows x at line 5
  ref2 borrows ref1 at line 6
  ref3 borrows ref2 at line 7

  Cannot move x while this chain exists
```

---

## 22. Limitations & Known Issues

### What's Not Checked

- **Thread safety**: No data race detection
- **Exception safety**: Stack unwinding not modeled
- **Virtual functions**: Limited dynamic dispatch analysis
- **Complex templates**: SFINAE, partial specialization limitations

### False Positives

Some patterns may trigger false positives:
- Complex control flow
- Template metaprogramming
- Macro-heavy code

Use `@unsafe` to suppress false positives when necessary.

### Template Limitations

- Variadic templates: Partial support
- SFINAE: Not fully analyzed
- Concepts (C++20): Basic support

---

# Part VIII: Advanced Topics

## 23. Relocation vs Move Semantics

### The Problem with C++ Move Semantics

C++11 move semantics leave the source object in a "valid but unspecified state":

```cpp
std::vector<int> v1 = {1, 2, 3};
std::vector<int> v2 = std::move(v1);
// v1 is now "valid but unspecified" - maybe empty, maybe not
v1.push_back(4);  // Legal! v1 is still a valid vector
```

This creates issues:
- Moved-from objects can still be used (accidentally)
- Destructors still run on moved-from objects
- No compile-time enforcement of "don't use after move"

### True Relocation (Safe C++ Approach)

The Safe C++ proposal introduces `rel` for true relocation:

```cpp
// Safe C++ syntax (not RustyCpp)
std2::vector<int> v1 = {1, 2, 3};
std2::vector<int> v2 = rel v1;  // v1 is now UNINITIALIZED
v1.push_back(4);  // COMPILE ERROR: v1 is uninitialized
```

After relocation:
- Source becomes truly uninitialized (not valid-but-unspecified)
- Compiler tracks initialization state
- Use of uninitialized variable is a compile error

### RustyCpp's Approach

RustyCpp uses **static analysis** on top of standard C++ move semantics:

```cpp
// @safe
void example() {
    auto v1 = std::vector<int>{1, 2, 3};
    auto v2 = std::move(v1);  // RustyCpp marks v1 as "moved"
    v1.push_back(4);  // ERROR: Use after move (detected by analyzer)
}
```

**Key differences from Safe C++:**

| Aspect | Safe C++ (`rel`) | RustyCpp |
|--------|------------------|----------|
| Mechanism | Language feature | Static analysis |
| Moved-from state | Truly uninitialized | Valid-but-unspecified (C++ semantics) |
| Enforcement | Compiler built-in | External checker |
| Runtime behavior | No destructor call | Destructor still runs |
| Recovery | Must reinitialize | Reassignment makes valid again |

### Reassignment Recovery

Unlike true relocation, RustyCpp allows recovery through reassignment:

```cpp
// @safe
void recovery() {
    auto box = rusty::Box<int>::make(42);
    auto box2 = std::move(box);  // box is "moved"

    box = rusty::Box<int>::make(100);  // box is valid again
    *box;  // OK
}
```

This matches C++ semantics where moved-from objects can be reused.

---

## 24. Thread Safety Model

### Current Status: Not Implemented

RustyCpp currently does **not** check for thread safety issues:

- No data race detection
- No enforcement of Send/Sync traits
- No mutex lock ordering analysis

### What Safe C++ Proposes

Safe C++ introduces Rust-like thread safety through traits:

**`send` trait**: Type can be transferred to another thread
```cpp
// Safe C++ syntax
template<typename T> requires send<T>
void spawn_thread(T data);
```

**`sync` trait**: Type can be shared between threads (via `&T`)
```cpp
// Safe C++ syntax
template<typename T> requires sync<T>
void share_between_threads(const T& data);
```

**Mutex integration**:
```cpp
// Safe C++ syntax
std2::mutex<std2::vector<int>> shared_vec;
{
    auto guard = shared_vec.lock();  // Returns mutable borrow
    guard->push_back(42);
}  // Lock released, borrow ends
```

### RustyCpp's Position

We intentionally omit thread safety checking because:

1. **Complexity**: Requires whole-program analysis
2. **C++ ecosystem**: Existing code uses various threading models
3. **Runtime nature**: Many thread safety issues are inherently dynamic
4. **Tooling overlap**: Tools like ThreadSanitizer handle this well

**Recommendation**: Use ThreadSanitizer or similar tools for thread safety:
```bash
clang++ -fsanitize=thread -g source.cpp
```

### Future Consideration

We may add basic thread safety annotations in the future:

```cpp
// Hypothetical future syntax
// @thread_safe
class Counter {
    std::atomic<int> value;  // OK: atomic is thread-safe
};

// @not_thread_safe
class UnsafeCounter {
    int value;  // Would warn if shared across threads
};
```

---

## 25. Initialization Analysis

### What We Check

RustyCpp performs basic initialization tracking:

```cpp
// @safe
void init_example() {
    int x;       // Uninitialized
    int y = x;   // ERROR: Use of uninitialized variable

    int z;
    z = 42;      // Now initialized
    int w = z;   // OK
}
```

### Scope-Based Tracking

Initialization state is tracked through control flow:

```cpp
// @safe
void conditional_init() {
    int x;
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
    use(x);  // OK: x initialized on all paths
}

// @safe
void partial_init() {
    int x;
    if (condition) {
        x = 1;
    }
    use(x);  // ERROR: x may be uninitialized
}
```

### Limitations

Current initialization analysis does not cover:

- **Member initialization order**: Constructor initializer list ordering not checked
- **Delayed initialization**: Objects initialized after declaration in complex patterns
- **Placement new**: Memory reuse patterns

### Comparison with Safe C++

| Feature | Safe C++ | RustyCpp |
|---------|----------|----------|
| Uninitialized locals | Compile error | Static analysis warning |
| Relocation tracking | Built-in | Via move analysis |
| Member init order | Checked | Not checked |
| Placement new | Tracked | Not tracked |

---

## 26. Lifetime Elision Rules

### What is Lifetime Elision?

When lifetime annotations are omitted, the compiler/analyzer infers them based on common patterns. This reduces boilerplate while maintaining safety.

### RustyCpp's Elision Rules

**Rule 1: Single reference parameter**

If there's exactly one reference parameter, the return lifetime matches it:

```cpp
// No annotation needed - inferred as: (&'a) -> &'a
const std::string& first_char(const std::string& s);

// Equivalent to:
// @lifetime: (&'a) -> &'a
const std::string& first_char(const std::string& s);
```

**Rule 2: Multiple parameters, one is `this`**

For methods, return lifetime matches `this`:

```cpp
class Container {
    // Inferred as: (&'self) -> &'self
    const Item& get() const;

    // Inferred as: (&'self mut) -> &'self mut
    Item& get_mut();
};
```

**Rule 3: Multiple reference parameters**

Explicit annotation required:

```cpp
// ERROR: Ambiguous - which parameter's lifetime?
const T& select(const T& a, const T& b);

// Must specify:
// @lifetime: (&'a, &'b) -> &'a
const T& select(const T& a, const T& b);
```

### When to Write Explicit Annotations

Always write explicit `@lifetime` when:

1. Multiple reference parameters exist
2. Return lifetime differs from the obvious choice
3. Complex lifetime relationships exist (outlives constraints)
4. Documenting API contracts for library code

```cpp
// Complex case: return lifetime tied to first param only
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& longer_lived(const T& a, const T& b);

// Owned return (no lifetime relationship)
// @lifetime: (&'a) -> owned
std::string to_string(const MyClass& obj);
```

---

## 27. Pattern Matching & Sum Types

### The Limitation of C++ Optional/Variant

C++ `std::optional` and `std::variant` have safety issues:

```cpp
std::optional<int> opt;
int value = *opt;  // UB: accessing empty optional

std::variant<int, std::string> var = 42;
std::string& s = std::get<std::string>(var);  // Throws: wrong type
```

### What Safe C++ Proposes: Choice Types

Safe C++ introduces first-class sum types with pattern matching:

```cpp
// Safe C++ syntax (not RustyCpp)
choice Result<T, E> {
    Ok(T),
    Err(E),
};

Result<int, std::string> divide(int a, int b) {
    if (b == 0) return .Err("division by zero");
    return .Ok(a / b);
}

void use_result() {
    auto result = divide(10, 2);

    match (result) {
        .Ok(value) => std::cout << "Got: " << value;
        .Err(msg)  => std::cout << "Error: " << msg;
    };  // Exhaustiveness checked at compile time
}
```

**Key features:**
- Exhaustiveness checking (must handle all cases)
- No way to access wrong variant
- Pattern binding extracts values safely

### RustyCpp's Approach: Wrapper Types

We provide `rusty::Option<T>` and `rusty::Result<T, E>` with safer APIs:

```cpp
#include <rusty/option.hpp>
#include <rusty/result.hpp>

// @safe
void safe_optional() {
    rusty::Option<int> opt = rusty::None;

    // Safe access patterns
    if (opt.is_some()) {
        int value = opt.unwrap();  // OK: checked first
    }

    int value = opt.unwrap_or(0);  // Safe: provides default

    // Callback-based matching
    opt.match(
        [](int v) { std::cout << "Got: " << v; },
        []() { std::cout << "None"; }
    );
}
```

### Comparison

| Feature | Safe C++ Choice | rusty::Option/Result |
|---------|-----------------|---------------------|
| Exhaustiveness | Compile-time enforced | Runtime (match callbacks) |
| Pattern syntax | Language built-in | Library methods |
| Zero-cost | Yes | Mostly (some virtual calls) |
| Type safety | Complete | Partial (unwrap can panic) |

### Best Practices with rusty::Option

```cpp
// AVOID: Unchecked unwrap
int bad(rusty::Option<int> opt) {
    return opt.unwrap();  // May panic!
}

// PREFER: Check first or use safe alternatives
int good1(rusty::Option<int> opt) {
    return opt.unwrap_or(0);
}

int good2(rusty::Option<int> opt) {
    if (opt.is_some()) {
        return opt.unwrap();
    }
    return 0;
}

// BEST: Use match for exhaustive handling
int best(rusty::Option<int> opt) {
    int result = 0;
    opt.match(
        [&](int v) { result = v; },
        [&]() { result = 0; }
    );
    return result;
}
```

---

## 28. Comparison with Safe C++ Proposal

### Overview

The [Safe C++ Proposal](https://safecpp.org/draft.html) is a comprehensive language extension proposal. RustyCpp is a static analyzer that works with standard C++.

### Feature Comparison

| Feature | Safe C++ Proposal | RustyCpp |
|---------|-------------------|----------|
| **Approach** | Language extension | Static analyzer |
| **Syntax** | New keywords (`safe`, `rel`, `^`) | Comment annotations (`@safe`) |
| **Borrow types** | `T^` (mutable), `const T^` (shared) | Standard C++ references |
| **Relocation** | `rel` keyword | `std::move` + analysis |
| **Sum types** | `choice` + `match` | Library types + callbacks |
| **Thread safety** | `send`/`sync` traits | Not implemented |
| **Lifetime params** | First-class syntax | Comment annotations |
| **Runtime checks** | Panic on bounds, etc. | Pure static analysis |
| **Compiler support** | Requires compiler changes | Works with any C++20 compiler |
| **Adoption** | Rewrite with new syntax | Gradual annotation |

### Philosophical Differences

**Safe C++**: "Change the language to make safety the default"
- New syntax for safe constructs
- Unsafe operations require explicit `unsafe` blocks
- Breaking changes to semantics (relocation vs move)

**RustyCpp**: "Add safety checking to existing C++"
- Works with standard C++ syntax
- Opt-in via annotations
- Non-breaking (analyzer is optional)

### When to Use Which

**Use Safe C++ (when available) if:**
- Starting a new project from scratch
- Team is willing to learn new syntax
- Maximum safety guarantees needed
- Can wait for compiler support

**Use RustyCpp if:**
- Working with existing C++ codebase
- Need safety checking today
- Gradual migration is important
- Cannot change compiler

### Interoperability Vision

In the future, RustyCpp could potentially:
- Recognize Safe C++ syntax when it becomes available
- Provide migration tooling from annotated C++ to Safe C++
- Serve as a stepping stone toward full Safe C++ adoption

---

# Part IX: Future Roadmap

## 29. Planned Features

### Short Term (Next 6 Months)

**Enhanced Lifetime Analysis**
- More sophisticated elision rules
- Better cross-function tracking
- Improved error messages with suggestions

**Template Improvements**
- Better variadic template support
- SFINAE-aware analysis
- Concept constraint checking

**IDE Integration**
- Language Server Protocol (LSP) support
- Real-time error highlighting
- Quick-fix suggestions

### Medium Term (6-12 Months)

**Thread Safety (Basic)**
- Annotations for thread-safe types
- Detection of obvious data races
- Mutex guard lifetime tracking

**Unsafe Type Qualifier**
- Mark types as inherently unsafe
- Propagate unsafety through templates
- Better integration with legacy code

**Custom Annotations**
- User-defined safety annotations
- Plugin system for custom checks
- Project-specific rules

### Long Term (12+ Months)

**Safe C++ Compatibility**
- Recognize Safe C++ syntax
- Migration tooling
- Hybrid analysis mode

**Advanced Analysis**
- Whole-program analysis mode
- Inter-procedural optimization
- Incremental checking

**Ecosystem**
- Pre-analyzed annotations for popular libraries
- Community annotation sharing
- CI/CD integration guides

### Non-Goals

Some features we intentionally won't implement:

- **Full thread safety**: Use ThreadSanitizer instead
- **Exception flow analysis**: Too complex, limited benefit
- **Runtime instrumentation**: We're purely static
- **Automatic code fixes**: Too risky for safety-critical code

---

## 30. FAQ & Troubleshooting

### Q: Why isn't my function being checked?

A: Make sure it has a `@safe` annotation:
```cpp
// @safe  <-- Need this
void my_function() { }
```

### Q: How do I suppress a false positive?

A: Use `@unsafe` block or function annotation:
```cpp
// @safe
void func() {
    // @unsafe
    {
        // False positive code here
    }
}
```

### Q: Why are third-party headers causing errors?

A: Make sure third-party headers are detected as external. Check:
1. Include paths are correct
2. Headers don't have `@safe` annotations

### Q: How do I check only specific files?

A: Pass specific files to the checker:
```bash
./rusty-cpp-checker src/safe_module.cpp src/core.cpp
```

### Q: Can I use this with existing codebases?

A: Yes! Use gradual adoption:
1. Start with new files marked `@safe`
2. Wrap calls to legacy code in `@unsafe` blocks
3. Gradually migrate more code as you gain confidence

---

## Appendix: Quick Reference Card

```
ANNOTATIONS
-----------
// @safe       Function/class/namespace is checked
// @unsafe     Function/class/namespace is NOT checked
// @unsafe { } Block within @safe function for unsafe ops

LIFETIME SYNTAX
---------------
// @lifetime: (&'a) -> &'a           Same lifetime as input
// @lifetime: (&'a, &'b) -> &'a      First param's lifetime
// @lifetime: (&'a mut) -> &'a mut   Mutable reference
// @lifetime: -> owned               No lifetime (owned return)

RUSTY TYPES
-----------
rusty::Box<T>        Unique ownership (like unique_ptr)
rusty::Arc<T>        Thread-safe shared (like shared_ptr + atomic)
rusty::Rc<T>         Single-thread shared
rusty::Cell<T>       Interior mutability for Copy types
rusty::RefCell<T>    Interior mutability with runtime checks
rusty::Option<T>     Optional value
rusty::Result<T,E>   Success or error

BORROW RULES
------------
Multiple &T  : OK
Single &mut T: OK
&T + &mut T  : ERROR
Move + use   : ERROR
```

---

*Document version: 1.4*
*Last updated: January 2026*
*Updated: Part IV Section 9 (Interior Mutability) with complete Cell/RefCell/UnsafeCell APIs*
*Updated: Part III (Safety Annotations) with calling rules, annotation syntax, header propagation, STL handling*
*Updated: Part II (Core Concepts) with detailed reference semantics, partial borrows, partial moves*
*Updated: Part V Section 15 (RAII & Container Safety) with comprehensive RAII tracking details*
