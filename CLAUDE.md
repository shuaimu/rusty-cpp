# Rusty-CPP - Project Context for Claude

## Project Overview

This is a Rust-based static analyzer that applies Rust's ownership and borrowing rules to C++ code. The goal is to catch memory safety issues at compile-time without runtime overhead.

**Supported C++ Standard**: C++20 (parser configured with `-std=c++20`)

## Current State (Updated: January 2026 - Chained Method Temporaries!)

### What's Fully Implemented ✅

**Latest Features (January 2026):**
- ✅ **Chained Method Temporary Detection** - Detects dangling references from method chains (**newly implemented!**)
  - Detects `Builder().method().get_ref()` patterns where temporary dies
  - Tracks constructor calls creating temporary objects
  - Detects when method returns reference tied to `'self` lifetime
  - Reports error when temporary-tied reference escapes statement
  - Uses `@lifetime: (&'self) -> &'self` annotation to track self-referencing returns
  - Example error: `int& ref = Builder().set(42).get_value();` - Builder() is temporary
  - See `tests/test_cross_function_lifetime.rs::test_chained_method_call_dangling`

- ✅ **Loop Dangling Reference Detection** - Detects references to loop-local variables that escape iteration (**newly implemented!**)
  - Detects when loop-local variable's reference is stored in non-loop-local container
  - Tracks loop-local variables (declared inside loop body)
  - Reports error when reference to loop-local escapes via function call
  - Example error: `refs[i] = &identity(temp);` where `temp` is loop-local
  - See `tests/test_cross_function_lifetime.rs::test_loop_accumulates_dangling_refs`

**Previous Features (December 2025):**
- ✅ **rusty::move** - Rust-like move semantics for C++ references
  - `rusty::move` for values: same as std::move, transfers ownership
  - `rusty::move` for mutable references: invalidates the reference itself (Rust-like)
  - `rusty::move` for const references: compile error (use = to copy)
  - `rusty::copy` for explicit copies
  - Zero runtime overhead (identical to std::move at runtime)
  - **std::move on references forbidden in @safe code** - use rusty::move instead
  - **Reference assignment semantics**: mutable refs move (like `&mut T`), const refs copy (like `&T`)
  - See `docs/reference_semantics.md` and `include/rusty/move.hpp`

- ✅ **Function Pointer Safety** - Type-safe function pointer wrappers (**newly implemented!**)
  - `rusty::SafeFn<Sig>` - holds pointers to @safe functions, safe to call
  - `rusty::UnsafeFn<Sig>` - holds any function, requires @unsafe to call
  - `rusty::SafeMemFn<Sig>` / `rusty::UnsafeMemFn<Sig>` - for member function pointers
  - Analyzer verifies SafeFn only holds @safe function targets
  - UnsafeFn::call_unsafe() requires @unsafe context
  - Mirrors Rust's `fn()` vs `unsafe fn()` type distinction
  - See `docs/function_pointer_safety.md` and `include/rusty/fn.hpp`

- ✅ **String Literal Safety** - String literals recognized as safe (**newly implemented!**)
  - `"hello"` expressions are safe in @safe code (static lifetime)
  - Explicit `char*` / `const char*` variable declarations require @unsafe
  - Safe wrapper pattern: functions can take char* and use internal @unsafe

- ✅ **Partial Borrow Tracking** - Borrow individual struct fields independently
  - Borrow different fields mutably at the same time (`&mut p.first` and `&mut p.second`)
  - Nested field support (`o.inner.data`)
  - Double mutable borrow of same field detected
  - Mixed mutable/immutable borrow conflict detection
  - Whole-struct vs field borrow conflict detection
  - Sequential borrows in separate scopes properly cleaned up
  - See `docs/PARTIAL_MOVES_PLAN.md` for details

- ✅ **RAII Tracking Module** - Comprehensive resource lifetime tracking
  - Reference/pointer stored in container detection
  - User-defined RAII types (classes with destructors)
  - Iterator outlives container detection
  - Lambda escape analysis (refined - non-escaping ref captures allowed)
  - Member lifetime tracking (`&obj.field` borrows)
  - new/delete tracking (double-free, use-after-free)
  - 22 comprehensive tests
  - See `docs/RAII_TRACKING.md` for details

**Previous Features (January 2025):**
- ✅ **Phase 3: Conflict Detection** - Prevents multiple mutable borrows
  - Detects when the same variable is borrowed mutably twice
  - Prevents mutable borrow when immutable borrows exist
  - Allows multiple immutable borrows (Rust-style rules)
  - Enhanced complex type detection (`Option<T&>`, custom types)
  - 7 comprehensive integration tests
  - See `docs/PHASE3_COMPLETE.md` for details

- ✅ **Phase 4: Transitive Borrow Tracking** - Tracks borrow chains (**newly implemented!**)
  - Detects transitive borrows: `ref3 -> ref2 -> ref1 -> value`
  - Recursive algorithm handles chains of any depth
  - Prevents moves anywhere in the borrow chain
  - Error messages show complete borrow chain
  - 6 comprehensive integration tests
  - See `docs/PHASE4_COMPLETE.md` for details

- ✅ **@unsafe Block Support** - Fine-grained safety escapes
  - Use `// @unsafe { ... }` to mark specific code blocks as unsafe
  - **Required in two-state model** to call @unsafe functions from @safe code
  - Allows calling STL, unannotated, and explicitly @unsafe functions
  - Proper scope tracking with depth counter for nested blocks
  - All safety checks skipped inside @unsafe blocks
  - Works with both qualified (`std::`) and unqualified names

- ✅ **Full Template Support** - Complete analysis of C++ template code
  - Template free functions analyzed with generic types
  - Template class methods fully supported (all qualifiers: const, non-const, &&)
  - Multiple type parameters (T, U, etc.)
  - Move detection and borrow checking in templates
  - Analyzes template declarations (no instantiation needed!)
  - 100% test pass rate on template test suite

- ✅ **Two-State Safety System** - Safe/Unsafe distinction (December 2025)
  - **Simplified model**: Only `@safe` and `@unsafe` states (no "undeclared")
  - **Unannotated code is @unsafe by default**
  - **@safe can ONLY call @safe** - must use `@unsafe { }` block to call anything else
  - STL/external functions require `@unsafe` blocks in @safe code
  - Clean audit boundary - either code is safe or it isn't

- ✅ **Header-to-Implementation Propagation** - Annotations flow from .h to .cpp
  - Safety annotations in headers automatically apply to implementations
  - Source file annotations can override header annotations
  - Supports namespace-level safety
  - Works with class methods and free functions

**Advanced Features (Added 2025):**
- ✅ **STL Usage in @safe Code** - Requires `@unsafe` blocks
  - With two-state model, all STL functions are @unsafe by default
  - @safe functions must wrap STL calls in `@unsafe { }` blocks
  - This is the "strict" approach - no STL whitelist
  - Users can use external annotations to mark specific STL functions as safe
  - See `tests/test_std_annotations.rs` for examples

- ✅ **STL Lifetime Annotations** - Complete lifetime checking for C++ STL types
  - Vector, map, unique_ptr, shared_ptr, string, etc.
  - Iterator invalidation detection
  - Reference stability rules
  - No modification to STL headers required
  - See `include/stl_lifetimes.hpp` and `docs/stl_lifetimes.md`

- ✅ **Unified External Annotations** - Combined safety + lifetime for third-party code
  - Annotate external functions without source modification
  - Compact syntax: `func: [safety, lifetime_spec]` where safety is `safe` or `unsafe`
  - Mark audited external functions as `[safe]` to call them directly from @safe code
  - No hardcoded defaults - users control what is marked safe
  - See `include/unified_external_annotations.hpp` and `docs/unified_annotations.md`

**Core Features:**
- ✅ **Complete reference borrow checking** for C++ const and mutable references
  - Multiple immutable borrows allowed
  - Single mutable borrow enforced
  - No mixing of mutable and immutable borrows
  - Clear error messages with variable names
- ✅ **std::move detection and use-after-move checking**
  - Detects move() and std::move() calls by name matching
  - Tracks moved-from state of variables
  - Reports use-after-move errors
  - Handles both direct moves and moves in function calls
  - Works for all types including unique_ptr, std::string, std::vector
  - **Fixed (Jan 2026)**: Detects use-after-move when passing moved variables to function calls
- ✅ **Scope tracking for accurate borrow checking**
  - Tracks when `{}` blocks begin and end
  - Automatically cleans up borrows when they go out of scope
  - Eliminates false positives from sequential scopes
  - Properly handles nested scopes
- ✅ **Loop analysis with 2-iteration simulation**
  - Detects use-after-move in loops (for, while, do-while)
  - Simulates 2 iterations to catch errors on second pass
  - Properly clears loop-local borrows between iterations
  - Tracks moved state across loop iterations
  - **Fixed (Dec 2025)**: Variables declared in loop body (`Box x;`) are now correctly tracked as loop-local
- ✅ **If/else conditional analysis with path-sensitivity**
  - Parses if/else statements and conditions
  - Conservative path-sensitive analysis
  - Variable is moved only if moved in ALL paths
  - Borrows cleared when not present in all branches
  - Handles nested conditionals
- ✅ **Two-state safety annotation system** (Updated December 2025)
  - **Two states**: `@safe` and `@unsafe` only (no "undeclared" state)
  - **Single rule**: Annotations only attach to the NEXT code element
  - **Annotation suffixes**: Annotations support any suffix (`@safe-note`, `@unsafe: reason`, etc.)
  - **Unannotated code is @unsafe by default**
  - **Calling rules matrix**:
    - `@safe` → can call: @safe ✅, @unsafe ❌ (use `@unsafe { }` block)
    - `@unsafe` → can call: @safe ✅, @unsafe ✅
  - **Key insight**: Clean audit boundary - code is either safe or unsafe, no middle ground
  - **@unsafe blocks**: Required to call @unsafe functions from @safe code
    ```cpp
    // @safe
    void example() {
        // @unsafe
        {
            std::vector<int> v;  // STL is @unsafe
            v.push_back(1);
        }
    }
    ```
  - **Annotation hierarchy** (lower level overrides higher level):
    1. **Function-level**: `// @safe` before function - highest priority
    2. **Class-level**: `// @safe` before class - overrides namespace
    3. **Namespace-level**: `// @safe` before namespace - lowest priority
  - **Per-file scope**: Namespace annotations are **per-file**, not global
    - Same namespace can have `@safe` in one file, `@unsafe` in another
    - Each file's annotation only affects code in that file
    - Enables gradual migration: annotate files independently
  - **Header propagation**: Annotations in headers automatically apply to implementations
  - STL and external libraries are @unsafe by default (require @unsafe blocks)
- ✅ **Cross-file analysis with lifetime annotations**
  - Rust-like lifetime syntax in headers (`&'a`, `&'a mut`, `owned`)
  - Header parsing and caching system
  - Include path resolution (-I flags, compile_commands.json, environment variables)
- ✅ **Advanced lifetime checking**
  - Scope-based lifetime tracking
  - Dangling reference detection
  - Transitive outlives checking ('a: 'b: 'c)
  - Automatic lifetime inference for local variables
  - **Cross-function lifetime enforcement** (NEW!)
    - Detects when function returns reference tied to temporary argument
    - Uses `@lifetime` annotations to track parameter-return relationships
    - Example: `identity(42)` correctly flagged as dangling when assigned to reference
  - **Array subscript return value tracking** (NEW!)
    - Methods returning `data[i]` correctly track member lifetime
    - Fixes false positive for container `at()` methods
- ✅ **Include path support**
  - CLI flags (-I)
  - Environment variables (CPLUS_INCLUDE_PATH, CPATH, etc.)
  - compile_commands.json parsing
  - Distinguishes quoted vs angle bracket includes
- ✅ Basic project structure with modular architecture
- ✅ LibClang integration for parsing C++ AST
- ✅ IR with CallExpr and Return statements
- ✅ Z3 solver integration for constraints
- ✅ Colored diagnostic output
- ✅ **Raw pointer safety checking (Rust-like)**
  - Detects unsafe pointer operations in safe code
  - Address-of (`&x`) requires unsafe context
  - Dereference (`*ptr`) requires unsafe context
  - Type-based detection to distinguish & from *
  - References remain safe (not raw pointers)
- ✅ **Unsafe propagation checking**
  - Safe functions cannot call @unsafe functions directly
  - Must use `@unsafe { }` blocks to call @unsafe functions from @safe code
  - All unannotated functions (including STL) are @unsafe by default
  - No hardcoded STL whitelist - users control what is marked safe
  - Users can mark audited external functions as `[safe]` via external annotations
  - Proper error reporting with function names and locations
  - Comprehensive test coverage (10+ tests)
- ✅ **Standalone binary support**
  - Build with `cargo build --release`
  - Embeds library paths (no env vars needed at runtime)
  - Platform-specific RPATH configuration
- ✅ **Comprehensive test suite**: 670+ tests covering templates, variadic templates, STL annotations, C++ casts, pointer safety, move detection, reassignment-after-move, borrow checking (including conflict detection, transitive borrows, partial borrows, and field method borrows), unsafe propagation, @unsafe blocks, cross-function lifetime, lambda capture safety, RAII tracking (containers, iterators, members, new/delete), partial moves/borrows, function pointer safety, string literal tracking, STL use-after-move, and comprehensive integration tests

### What's Partially Implemented ⚠️
- ⚠️ Virtual function calls (basic method calls work)
- ⚠️ Loop counter variables declared in `for(int i=...)` not tracked in variables map

### What's Not Implemented Yet ❌

#### Critical for Modern C++
- ✅ **Smart pointer safety through move detection**
  - `unique_ptr`: use-after-move detected via std::move()
  - `shared_ptr`: use-after-move detected (explicit moves)
  - C++ compiler prevents illegal copies
  - Main safety issues are covered
  
- ❌ **Advanced smart pointer features**
  - No circular reference detection for shared_ptr
  - No weak_ptr validity checking (runtime issue)
  - Thread safety not analyzed

#### Important for Correctness

- ⚠️ **Constructor/Destructor (RAII)** - Partially implemented!
  - ✅ Object lifetime tracked via scope-based analysis
  - ✅ User-defined RAII types detected (classes with destructors)
  - ✅ Member lifetime tracking (`&obj.field` tied to `obj`)
  - ✅ Iterator/container lifetime relationships
  - ✅ Lambda escape analysis with reference captures
  - ✅ new/delete tracking (double-free, use-after-free)
  - ❌ Constructor initialization order not checked
  - See `docs/RAII_TRACKING.md` for full details

#### Nice to Have
- ✅ **Reassignment after move** (Implemented November 2025!)
  - Moved variable becomes valid after reassignment
  - `x = std::move(y); x = 42; use(x);` - x valid again and tracked
  - Works with literals, variables, and move assignments
  - 12 comprehensive tests

- ❌ **Exception handling**
  - Try/catch blocks ignored
  - Stack unwinding not modeled
  
- ✅ **Lambda capture safety** (Implemented November 2025, **refined December 2025!**)
  - Reference captures ([&], [&x]) now allowed if lambda doesn't escape
  - Reference captures that escape (returned, stored) are forbidden
  - Copy captures ([x], [=]) always allowed
  - Move captures ([y = std::move(x)]) always allowed
  - 'this' capture always forbidden (raw pointer)
  - 13 comprehensive tests + escape analysis tests
  
- ❌ **Better diagnostics**
  - No code snippets in errors
  - No fix suggestions
  - No explanation of borrowing rules
  
- ❌ **IDE integration**
  - No Language Server Protocol (LSP)
  - CLI only

## How Rust's Borrow Checker Handles Loops

Rust uses a sophisticated approach to detect use-after-move in loops:

1. **Control Flow Graph with Back Edges**: Loops have edges from end back to beginning
2. **Fixed-Point Iteration**: Analysis runs until no more state changes
3. **Three-State Tracking**: Variables are "definitely initialized", "definitely uninitialized", or "maybe initialized"
4. **Conservative Analysis**: "Maybe initialized" treated as error for moves

Example of what Rust catches:
```rust
for i in 0..2 {
    let y = x;  // ERROR: value moved here, in previous iteration of loop
}
```

To implement similar analysis in our checker:
- Detect loop back edges in CFG
- Analyze loop body twice (simulating two iterations)
- Track "maybe moved" state for variables
- Error if "maybe moved" variable is used

## Key Technical Decisions

1. **Language Choice**: Rust for memory safety and performance
2. **Parser**: LibClang for accurate C++ parsing (C++20 standard)
3. **Solver**: Z3 for lifetime constraint solving
4. **IR Design**: Ownership-aware representation with CFG
5. **Analysis Strategy**: Per-translation-unit with header annotations (no .cpp-to-.cpp needed)
6. **C++ Standard**: C++20 support with modern features (concepts, ranges, coroutines, etc.)

## Project Structure

```
src/
├── main.rs              # Entry point, CLI handling, include path resolution
├── parser/
│   ├── mod.rs          # Parse orchestration
│   ├── ast_visitor.rs  # AST traversal, function call extraction
│   ├── annotations.rs  # Lifetime annotation parsing
│   └── header_cache.rs # Header signature caching
├── ir/
│   └── mod.rs          # IR with CallExpr, Return, CFG
├── analysis/
│   ├── mod.rs              # Main analysis coordinator
│   ├── ownership.rs        # Ownership state tracking
│   ├── borrows.rs          # Basic borrow checking
│   ├── lifetimes.rs        # Original lifetime framework
│   ├── lifetime_checker.rs # Annotation-based checking
│   ├── scope_lifetime.rs   # Scope-based tracking
│   ├── lifetime_inference.rs # Automatic inference
│   ├── raii_tracking.rs    # RAII tracking (containers, iterators, members, new/delete)
│   └── lambda_capture_safety.rs # Lambda capture escape analysis
├── solver/
│   └── mod.rs          # Z3 constraint solving
└── diagnostics/
    └── mod.rs          # Error formatting
```

## Environment Setup

```bash
# macOS
export Z3_SYS_Z3_HEADER=/opt/homebrew/include/z3.h
export DYLD_LIBRARY_PATH=/opt/homebrew/Cellar/llvm/19.1.7/lib:$DYLD_LIBRARY_PATH

# Linux
export Z3_SYS_Z3_HEADER=/usr/include/z3.h
export LD_LIBRARY_PATH=/usr/lib/llvm-16/lib:$LD_LIBRARY_PATH

# Optional: Include paths via environment
export CPLUS_INCLUDE_PATH=/usr/include/c++:/usr/local/include
export CPATH=/usr/include
```

## Usage Examples

### Basic Usage
```bash
# Basic usage (C++20 standard by default)
cargo run -- file.cpp

# With include paths
cargo run -- file.cpp -I include -I /usr/local/include

# With compile_commands.json
cargo run -- file.cpp --compile-commands build/compile_commands.json

# Using environment variables
export CPLUS_INCLUDE_PATH=/project/include:/third_party/include
cargo run -- src/main.cpp
```

**Note**: The analyzer uses C++20 standard (`-std=c++20`) for parsing, supporting modern C++ features like concepts, ranges, coroutines, and three-way comparison.

### Using Rusty Structures (Recommended)
```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
void safe_example() {
    // Vector iterator invalidation detection
    rusty::Vec<int> vec = {1, 2, 3};
    auto it = vec.begin();  // Iterator borrows from vec
    vec.push_back(4);       // ERROR: Would invalidate iterator

    // Smart pointer ownership tracking
    rusty::Box<int> ptr = rusty::Box<int>::make(42);
    int& ref = *ptr;        // Reference borrows from ptr
    auto ptr2 = std::move(ptr);  // ERROR: Cannot move while borrowed
}
```

### Using External Annotations for STL (If Needed)
```cpp
#include <vector>
#include <unified_external_annotations.hpp>

// STL structures are @unsafe by default in the two-state model
// @external: {
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
// }

// @safe
void stl_example() {
    // Must use unsafe block for STL
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);  // OK in unsafe block
    }
}

// Better: Use Rusty structures instead
// @safe
void better_example() {
    rusty::Vec<int> vec = {1, 2, 3};
    vec.push_back(4);  // Safe by design, no annotations needed
}
```

### Annotating Third-Party Functions
```cpp
#include <unified_external_annotations.hpp>

// IMPORTANT: All external functions must be marked [unsafe] because RustyCpp
// doesn't analyze external code. The programmer takes responsibility for auditing.
//
// @external: {
//   third_party::process: [unsafe, (const Data& d) -> Result]
//   third_party::allocate: [unsafe, (size_t) -> owned void*]
//   sqlite3_column_text: [unsafe, (stmt* s, int col) -> const char* where s: 'a, return: 'a]
// }

// @safe
void external_example() {
    // All external functions require @unsafe blocks (no hardcoded whitelist)
    // @unsafe
    {
        const char* str = "hello";
        const char* found = strchr(str, 'e');  // OK: in unsafe block

        Data d;
        Result r = third_party::process(d);  // OK: in unsafe block
        void* buf = third_party::allocate(100);  // OK: in unsafe block
    }
}
```

### Template Support
```cpp
// Template free functions - fully supported!
template<typename T>
// @safe
T process(T x) {
    T a = std::move(x);   // Move x
    T b = std::move(x);   // ERROR: Use after move detected!
    return b;
}

// Template class methods - all qualifiers supported
template<typename T>
class Container {
    T data;
public:
    // @safe
    void bad_method() {
        T moved = std::move(data);  // ERROR: Cannot move field from non-const method
    }

    // @safe
    T ok_rvalue_method() && {
        return std::move(data);  // OK: && method can move fields
    }
};

// Multiple type parameters
template<typename T, typename U>
// @safe
void swap_types(T& a, U& b) {
    T temp = std::move(a);
    // Move analysis works with multiple type params
}
```

### Annotation Hierarchy and Per-File Scope

The safety annotation system has three levels, with lower levels overriding higher levels:

```cpp
// ============================================================================
// ANNOTATION HIERARCHY: Function > Class > Namespace
// ============================================================================

// @safe
namespace myapp {

// @unsafe - Class annotation overrides namespace
class UnsafeClass {
public:
    // @safe - Function annotation overrides class
    void safe_method() {
        int x = 42;  // This is safe despite being in unsafe class
    }

    void unsafe_method() {
        int* ptr = nullptr;  // OK - inherits unsafe from class
    }
};

// @safe - Inherits from namespace
class SafeClass {
public:
    void safe_method() {
        int x = 42;  // Safe - inherits from class
    }

    // @unsafe - Function overrides class
    void unsafe_method() {
        int* ptr = nullptr;  // OK - function is unsafe
    }
};

} // namespace myapp
```

**Annotation Suffixes** - Annotations support any suffix for documentation:
```cpp
// @safe-verified on 2025-01-17
void audited_function() { }

// @unsafe: uses raw pointers for performance
void performance_critical() { }

// @safe, reviewed by security team
void reviewed_function() { }
```

**Per-File Namespace Annotations** - Namespace annotations are file-scoped:

```cpp
// ============================================================================
// File: legacy_code.cpp
// ============================================================================
// @unsafe
namespace myapp {

void old_unsafe_code() {
    int* ptr = nullptr;
    *ptr = 42;  // OK - this file marks namespace as unsafe
}

} // namespace myapp


// ============================================================================
// File: new_code.cpp (same namespace, different file)
// ============================================================================
// @safe
namespace myapp {

void new_safe_code() {
    int x = 42;  // This file marks namespace as safe
    // Cannot use raw pointers here
}

} // namespace myapp
```

**Key Insights:**
- Each `.cpp` file can annotate the same namespace differently
- Enables **gradual migration**: migrate files one at a time
- Namespace annotations only affect code in that specific file
- Different modules of the same namespace can have different safety levels

## Lifetime Annotation Syntax

```cpp
// In header files (.h/.hpp)

// @lifetime: &'a
const int& getRef();

// @lifetime: (&'a) -> &'a
const T& identity(const T& x);

// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& selectFirst(const T& a, const T& b);

// @lifetime: owned
std::unique_ptr<T> create();

// @lifetime: &'a mut
T& getMutable();
```

## Testing Commands

```bash
# Set environment variables first (macOS)
export Z3_SYS_Z3_HEADER=/opt/homebrew/include/z3.h
export DYLD_LIBRARY_PATH=/opt/homebrew/Cellar/llvm/19.1.7/lib:$DYLD_LIBRARY_PATH

# Build the project
cargo build

# Run all tests (110+ tests)
cargo test

# Run specific test categories
cargo test lifetime   # Lifetime tests
cargo test borrow     # Borrow checking tests
cargo test safe       # Safe/unsafe annotation tests
cargo test move       # Move detection tests
cargo test template   # Template support tests
cargo test raii       # RAII tracking tests
cargo test lambda     # Lambda capture safety tests

# Run on example files
cargo run -- examples/reference_demo.cpp
cargo run -- examples/safety_annotation_demo.cpp

# Build release binary (standalone, no env vars needed)
cargo build --release
./target/release/rusty-cpp-checker file.cpp
```

## Known Issues

1. **Include Paths**: Standard library headers (like `<iostream>`) aren't found by default
2. **Limited C++ Support**: Lambdas, virtual functions, and advanced features not fully supported
3. **Left-side dereference**: `*ptr = value` not always detected (assignment target)
4. **Advanced templates**: Variadic templates, SFINAE, and partial specialization not yet supported

## Key Design Insights

### Template Analysis Without Instantiations

Our template support analyzes template **declarations** with generic types (T, U, etc.) rather than accessing template **instantiations**. This works because:

- **Borrow checking is type-independent**: Whether a variable is moved or borrowed doesn't depend on the concrete type
- **Move detection works on generic types**: `std::move(x)` is a move regardless of whether `x` is `int` or `std::string`
- **LibClang limitation**: Template instantiations are implicit entities filtered from the public API
- **Efficient**: Analyze once per template, not once per instantiation

Example:
```cpp
template<typename T>
void bad(T x) {
    T a = std::move(x);  // Move detected for any T
    T b = std::move(x);  // ERROR: Use after move (works for any T)
}
```

This approach matches how Rust's borrow checker analyzes generic code - once at the declaration, not per monomorphization.

### Why shared_ptr Doesn't Need Special Handling

Our move detection is sufficient for `shared_ptr` safety because:
- **Copying is safe** - Multiple owners are allowed by design
- **Move detection covers the risk** - `std::move(shared_ptr)` is detected
- **Reference counting is runtime** - Not a compile-time safety issue
- **Circular references** - Too complex for static analysis (Rust has same issue with `Rc<T>`)
- **Thread safety** - Outside scope of borrow checking

What we DO catch:
- ✅ Use after explicit move: `auto sp2 = std::move(sp1); *sp1;`

What we DON'T catch (and shouldn't):
- ❌ Circular references (requires whole-program analysis)
- ❌ Weak pointer validity (runtime issue)
- ❌ Data races (requires concurrency analysis)

### Why No .cpp-to-.cpp Analysis Needed

The tool correctly follows C++'s compilation model:
- Each `.cpp` file is analyzed independently
- Function signatures come from headers (with lifetime annotations)
- No need to see other `.cpp` implementations
- Matches how C++ compilers and Rust's borrow checker work

### Analysis Approach

1. **Parse all files** → Extract ALL functions from main file + headers (no filtering)
   - Includes STL, system headers, and third-party libraries
   - Functions tracked with their source file path
2. **Classify functions** → Distinguish user code from system libraries
   - **System headers**: Detected by file path patterns (`/include/c++/`, `/bits/`, `stl_`, etc.)
   - **User code**: Functions from main source file
3. **Analyze user code** → Full borrow checking, pointer safety, and lifetime validation
   - Borrow checking (ownership, moves, borrows)
   - Pointer safety (address-of, dereference in @safe code)
   - Lifetime inference and validation
4. **Track system functions** → Safety annotation checking only (no internal analysis)
   - System header functions are NOT analyzed for borrow violations
   - They ARE checked for safety annotations (safe/unsafe)
   - System functions are @unsafe by default, requiring @unsafe blocks in @safe code
5. **Validate calls** → Ensure safety rules and lifetime constraints are met
6. **Report errors** → With clear messages and locations

## Code Patterns to Follow

### Adding New Analysis
```rust
// In analysis/mod.rs or new module
pub fn check_feature(program: &IrProgram, cache: &HeaderCache) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    // Analysis logic
    Ok(errors)
}
```

### Adding Lifetime Annotations
```cpp
// In header file
// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const T& function(const T& longer, const T& shorter);
```

## Development Tips

1. **Test incrementally** - Use small C++ examples first
2. **Check parser output** - `cargo run -- file.cpp -vv` for debug
3. **Verify lifetimes** - Use `examples/test_*.cpp` for validation
4. **Run clippy** - `cargo clippy` for Rust best practices
5. **Update tests** - Add test cases for new features

## Example Output

```
Rusty-CPP
Analyzing: examples/reference_demo.cpp
✗ Found 3 violation(s):
Cannot create mutable reference to 'value': already mutably borrowed
Cannot create mutable reference to 'value': already immutably borrowed  
Use after move: variable 'x' has been moved
```

## Recent Achievements

**Latest (December 2025): Function Pointer Safety Implementation**
1. ✅ **Function Pointer Safety via Type Wrappers**
   - Created `rusty::SafeFn<Sig>` and `rusty::UnsafeFn<Sig>` wrapper types
   - Created `rusty::SafeMemFn<Sig>` and `rusty::UnsafeMemFn<Sig>` for member functions
   - Type detection in analyzer: `is_safe_fn_type()`, `is_unsafe_fn_type()`
   - SafeFn assignment checking: verifies target is @safe
   - UnsafeFn::call_unsafe() requires @unsafe context
   - 9 unit tests + 8 integration tests

2. ✅ **String Literal Safety Tracking**
   - Added `Expression::StringLiteral` variant to AST
   - String literals (`"hello"`) are safe in @safe code
   - Explicit char* variable declarations require @unsafe
   - 7 integration tests

**Files created:**
- `include/rusty/fn.hpp` - SafeFn/UnsafeFn wrapper types
- `src/analysis/function_pointer_safety.rs` - Analyzer module
- `tests/test_function_pointer_safety.rs` - Integration tests
- `docs/function_pointer_safety.md` - User documentation

**Previous (December 2025): Partial Borrow Tracking Complete**
1. ✅ **Partial Borrow Tracking** - Borrow different struct fields independently
   - Added `field_borrows: HashMap<String, HashMap<String, BorrowInfo>>` for per-field tracking
   - Added `check_field_borrow_conflicts()` function for field-level conflict detection
   - Added `check_whole_object_vs_field_borrows()` for whole/field conflict detection
   - Proper cleanup of field borrows on scope exit
   - Nested field support (`o.inner.data`)

2. ✅ **Bug Fixes**
   - Fixed single-line struct context stack issue (structs complete on one line no longer pollute context)
   - Fixed references incorrectly marked as RAII types (`std::string&` no longer has `has_destructor=true`)
   - Fixed field borrows not cleared on scope exit (sequential borrows in separate scopes now work)

3. **Test files added:**
   - `tests/raii/partial_borrow_test.cpp` - 7 tests for basic partial borrows
   - `tests/raii/partial_borrow_nested_test.cpp` - 9 tests for nested field borrows
   - `tests/raii/partial_borrow_control_flow_test.cpp` - 11 tests for control flow

**Documentation:** See `docs/PARTIAL_MOVES_PLAN.md` and `docs/RUST_COMPARISON.md`

**Previous (December 2025): RAII Tracking Implementation Complete**
1. ✅ **Phase 1: Reference/Pointer Stored in Container**
   - Detects pointers stored in containers that outlive pointees
   - Tracks `push_back`, `insert`, `emplace`, etc.
   - Container types: vector, list, deque, set, map, etc.

2. ✅ **Phase 2: User-Defined RAII Types**
   - Detects classes with user-defined destructors
   - Added `has_destructor` field to Class parsing
   - Integrated with `IrProgram.user_defined_raii_types`

3. ✅ **Phase 3: Iterator Outlives Container**
   - Tracks iterator borrows from containers
   - Detects when container dies while iterator survives
   - Tracks `begin`, `end`, `find`, etc.

4. ✅ **Phase 4: Lambda Escape Analysis (Refined)**
   - Changed from blanket ban to escape analysis
   - Reference captures allowed if lambda doesn't escape
   - Errors only for escaping lambdas with ref captures
   - 'this' capture still always forbidden

5. ✅ **Phase 5: Member Lifetime Tracking**
   - Tracks `&obj.field` borrows
   - Detects when member references outlive containing object
   - Proper cleanup when reference or object dies

6. ✅ **Phase 6: new/delete Tracking**
   - Tracks heap allocations
   - Detects double-free (delete freed pointer)
   - Detects use-after-free

7. ⏳ **Phase 7: Constructor Init Order** (Pending - LOW priority)
   - Requires parser changes for member initializer lists
   - Would detect initialization order dependencies

**Testing:** 22 RAII-specific tests, all passing
**Documentation:** See `docs/RAII_TRACKING.md`

**Previous (January 2025): Advanced Borrow Checking - Phases 3 & 4 Complete**
1. ✅ **Phase 3: Conflict Detection** (~2 hours implementation)
   - Multiple mutable borrow detection
   - Mutable/immutable borrow conflict detection
   - Multiple immutable borrows allowed (Rust-style)
   - Enhanced type detection for complex types (Option<T&>, etc.)
   - 7 comprehensive integration tests
   - Test count: 509 tests passing

2. ✅ **Phase 4: Transitive Borrow Tracking** (~1 hour implementation)
   - Recursive transitive borrow detection
   - Handles borrow chains of any depth
   - Prevents moves anywhere in chain
   - Error messages show complete chain
   - 6 comprehensive integration tests
   - Test count: 515 tests passing

3. ✅ **Integration Testing** - Comprehensive end-to-end tests
   - 7 integration tests exercising all phases together
   - Tests complex borrow graphs, scope cleanup, mixed mutability
   - **Final test count: 522 tests, all passing**

**Previous (January 2025): System Header Handling and Universal Parsing**
1. ✅ **Universal parsing** - Extract ALL functions from ALL files (main + headers)
   - Parser no longer filters by file - extracts from STL, system headers, third-party libraries
   - Functions tracked with source file path for classification
2. ✅ **System header detection** - Intelligent classification of user code vs. system libraries
   - Path-based detection: `/include/c++/`, `/bits/`, `stl_`, `/lib/gcc/`, etc.
   - Works with both absolute and relative paths
3. ✅ **Selective analysis** - System headers skipped during internal analysis
   - System header functions NOT analyzed for borrow violations
   - System header functions ARE tracked for safety annotation checking
   - System functions are @unsafe by default, requiring @unsafe blocks in @safe code
4. ✅ **Source file tracking** - Added `source_file` field to `IrFunction`
   - Analysis modules use file path to distinguish user code from system code
   - Applied across all analysis phases (borrow checking, pointer safety, lifetime inference)

**Previous (January 2025): Full Template Support Implementation**
1. ✅ **Template free functions** - Analyzes template declarations with generic types (T, U, etc.)
2. ✅ **Template class methods** - All method qualifiers supported (const, non-const, &&)
3. ✅ **Multiple type parameters** - Handles `template<typename T, typename U>`
4. ✅ **Parser bug fixes** - Fixed two similar bugs:
   - Variable arguments misclassified as function names (template-dependent lookups)
   - Field accesses misclassified as function calls (MemberRefExpr handling)
5. ✅ **100% test pass rate** - All 11 template tests passing (up from 4)
6. ✅ **Safety annotation support** - Template functions recognized in @safe annotation parser

Earlier achievements:
- ✅ **Unsafe propagation checking** - Safe functions cannot call unmarked/unsafe functions
- ✅ **Pointer safety checking** - Raw pointer operations require unsafe context
- ✅ **Type-based operator detection** - Distinguish & from * using type analysis
- ✅ Simplified @unsafe annotation to match @safe behavior
- ✅ Removed @endunsafe - both annotations now only affect next element
- ✅ Verified move detection works for all smart pointers
- ✅ Created standalone binary support with embedded library paths

## Next Priority Tasks

### High Priority
1. **Non-Lexical Lifetimes (NLL)** - Would dramatically reduce false positives
2. **Reborrowing** - Important for ergonomic reference-heavy code

### Medium Priority
3. **Better error messages** - Code snippets and fix suggestions
4. **Constructor initialization order** - Member initializer list analysis (Phase 7)
5. **Advanced template features** - Variadic templates, SFINAE, partial specialization
6. **Switch/case statements** - Common control flow

### Low Priority
7. **Two-phase borrows** - Method call patterns
8. **Loop counter variable tracking** - Variables in `for(int i=...)`
9. **Iterator invalidation from modifications** - Track `clear()`, `erase()`, etc.
10. **Circular reference detection** - Complex whole-program analysis
11. **Exception handling** - Stack unwinding
12. **Virtual function analysis** - Dynamic dispatch tracking
13. **IDE integration (LSP)** - CLI works for now

## Contact with Original Requirements

The tool achieves the core goals:
- ✅ **Standalone static analyzer** - Works independently, can build release binaries
- ✅ **Detect use-after-move** - Fully working with move() detection (including templates)
- ✅ **Detect multiple mutable borrows** - Fully working
- ✅ **Track lifetimes** - Complete with inference and validation
- ✅ **Detect unsafe pointer operations** - Rust-like pointer safety
- ✅ **Support modern C++ templates** - Template functions and classes fully analyzed
- ✅ **Provide clear error messages** - With locations and context
- ✅ **Support gradual adoption** - Per-function/namespace opt-in with @safe