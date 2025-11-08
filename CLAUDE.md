# Rusty-CPP - Project Context for Claude

## Project Overview

This is a Rust-based static analyzer that applies Rust's ownership and borrowing rules to C++ code. The goal is to catch memory safety issues at compile-time without runtime overhead.

**Supported C++ Standard**: C++20 (parser configured with `-std=c++20`)

## Current State (Updated: January 2025 - @unsafe blocks now working)

### What's Fully Implemented ✅

**Latest Features (January 2025):**
- ✅ **@unsafe Block Support** - Fine-grained safety escapes (**newly implemented!**)
  - Use `// @unsafe { ... }` to mark specific code blocks as unsafe
  - Allows calling undeclared functions within the block
  - Proper scope tracking with depth counter for nested blocks
  - All safety checks skipped inside @unsafe blocks
  - Works with both qualified (`std::`) and unqualified names
  - See `UNSAFE_BLOCK_NOT_IMPLEMENTED.md` for implementation details

- ✅ **Full Template Support** - Complete analysis of C++ template code
  - Template free functions analyzed with generic types
  - Template class methods fully supported (all qualifiers: const, non-const, &&)
  - Multiple type parameters (T, U, etc.)
  - Move detection and borrow checking in templates
  - Analyzes template declarations (no instantiation needed!)
  - 100% test pass rate on template test suite
  - See `PHASE3_COMPLETE.md` for details

- ✅ **Three-State Safety System** - Safe/Unsafe/Undeclared distinction
  - Safe functions have strict calling rules
  - Safe CAN call unsafe (documented risks)
  - Safe CANNOT call undeclared (unaudited code)
  - Creates audit ratchet effect

- ✅ **Header-to-Implementation Propagation** - Annotations flow from .h to .cpp
  - Safety annotations in headers automatically apply to implementations
  - Source file annotations can override header annotations
  - Supports namespace-level safety
  - Works with class methods and free functions

**Advanced Features (Added 2025):**
- ✅ **C++ Standard Library Annotations** - ~180+ std functions work in @safe code
  - No explicit annotations needed for common std library usage
  - Covers containers (vector, map, string), algorithms (sort, find), smart pointers, I/O
  - Operators (+, -, *, ==, [], <<, etc.) fully supported
  - Users can write natural C++ code without @unsafe blocks
  - **Note**: Cast operations (dynamic_cast, get(), etc.) correctly require @unsafe
  - See `include/std_annotation.hpp` and `STD_CASTS_ARE_UNSAFE.md`

- ✅ **STL Lifetime Annotations** - Complete lifetime checking for C++ STL types
  - Vector, map, unique_ptr, shared_ptr, string, etc.
  - Iterator invalidation detection
  - Reference stability rules
  - No modification to STL headers required
  - See `include/stl_lifetimes.hpp` and `docs/stl_lifetimes.md`

- ✅ **Unified External Annotations** - Combined safety + lifetime for third-party code
  - Annotate external functions without source modification
  - Compact syntax: `func: [safety, lifetime_spec]`
  - Pre-configured for C stdlib, POSIX, Boost, SQLite, etc.
  - Pattern-based matching and library profiles
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
  - Works for all types including unique_ptr
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
- ✅ **If/else conditional analysis with path-sensitivity**
  - Parses if/else statements and conditions
  - Conservative path-sensitive analysis
  - Variable is moved only if moved in ALL paths
  - Borrows cleared when not present in all branches
  - Handles nested conditionals
- ✅ **Three-state safety annotation system**
  - **Three states**: `@safe`, `@unsafe`, and undeclared (default)
  - **Single rule**: Annotations only attach to the NEXT code element
  - C++ files are undeclared by default (not checked, but distinct from unsafe)
  - **Calling rules matrix**:
    - `@safe` → can call: @safe ✅, @unsafe ✅, undeclared ❌
    - `@unsafe` → can call: @safe ✅, @unsafe ✅, undeclared ✅
    - `undeclared` → can call: @safe ✅, @unsafe ✅, undeclared ✅
  - **Key insight**: Undeclared functions can call other undeclared functions, enabling gradual migration
  - **Namespace-level**: `// @safe` before namespace applies to entire namespace contents
  - **Function-level**: `// @safe` before function enables checking for that function only
  - **Header propagation**: Annotations in headers automatically apply to implementations
  - STL and external libraries are undeclared by default (must be explicitly marked)
  - Creates "audit ratchet" - forces explicit safety decisions
- ✅ **Cross-file analysis with lifetime annotations**
  - Rust-like lifetime syntax in headers (`&'a`, `&'a mut`, `owned`)
  - Header parsing and caching system
  - Include path resolution (-I flags, compile_commands.json, environment variables)
- ✅ **Advanced lifetime checking**
  - Scope-based lifetime tracking
  - Dangling reference detection
  - Transitive outlives checking ('a: 'b: 'c)
  - Automatic lifetime inference for local variables
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
  - Safe functions cannot call unmarked or explicitly unsafe functions
  - Requires explicit @unsafe annotation for unsafe calls in safe context
  - Whitelisted standard library functions (printf, malloc, move, etc.)
  - Proper error reporting with function names and locations
  - Comprehensive test coverage (10+ tests)
- ✅ **Standalone binary support**
  - Build with `cargo build --release`
  - Embeds library paths (no env vars needed at runtime)
  - Platform-specific RPATH configuration
- ✅ **Comprehensive test suite**: 498 tests covering templates, variadic templates, STL annotations, C++ casts, pointer safety, move detection, borrow checking, unsafe propagation, and @unsafe blocks

### What's Partially Implemented ⚠️
- ⚠️ Reassignment after move (not tracked yet)
- ⚠️ Virtual function calls (basic method calls work)

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
  
- ❌ **Constructor/Destructor (RAII)**
  - Object lifetime not tracked
  - Destructor calls not analyzed
  - RAII patterns not understood

#### Nice to Have
- ❌ **Reassignment after move**
  - Can't track when moved variable becomes valid again
  - `x = std::move(y); x = 42;` - x valid again but not tracked

- ❌ **Exception handling**
  - Try/catch blocks ignored
  - Stack unwinding not modeled
  
- ❌ **Lambdas and closures**
  - Capture semantics not analyzed
  - Closure lifetime not tracked
  
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
│   └── lifetime_inference.rs # Automatic inference
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
export LD_LIBRARY_PATH=/usr/lib/llvm-14/lib:$LD_LIBRARY_PATH

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

// STL structures are undeclared by default and need to be annotated as unsafe
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
    // C stdlib with lifetime checking
    const char* str = "hello";
    const char* found = strchr(str, 'e');  // Lifetime tied to str (strchr is in std whitelist)

    // Third-party functions are unsafe (must be called from unsafe context)
    // @unsafe
    {
        Data d;
        Result r = third_party::process(d);  // OK: called from unsafe block
        void* buf = third_party::allocate(100);  // OK: called from unsafe block
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
   - They ARE checked for safety annotations (safe/unsafe/undeclared)
   - Allows detecting when @safe code calls undeclared system functions
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

**Latest (January 2025): System Header Handling and Universal Parsing**
1. ✅ **Universal parsing** - Extract ALL functions from ALL files (main + headers)
   - Parser no longer filters by file - extracts from STL, system headers, third-party libraries
   - Functions tracked with source file path for classification
2. ✅ **System header detection** - Intelligent classification of user code vs. system libraries
   - Path-based detection: `/include/c++/`, `/bits/`, `stl_`, `/lib/gcc/`, etc.
   - Works with both absolute and relative paths
3. ✅ **Selective analysis** - System headers skipped during internal analysis
   - System header functions NOT analyzed for borrow violations
   - System header functions ARE tracked for safety annotation checking
   - Enables detecting when @safe code calls undeclared system functions
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
1. **Constructor/Destructor tracking** - RAII patterns
2. **Reassignment tracking** - Variable becomes valid after reassignment
3. **Better error messages** - Code snippets and fix suggestions

### Medium Priority
4. **Advanced template features** - Variadic templates, SFINAE, partial specialization
5. **Switch/case statements** - Common control flow
6. **Lambda captures** - Closure lifetime tracking

### Low Priority
7. **Circular reference detection** - Complex whole-program analysis
8. **Exception handling** - Stack unwinding
9. **Virtual function analysis** - Dynamic dispatch tracking
10. **IDE integration (LSP)** - CLI works for now

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