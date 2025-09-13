# Rusty-CPP - Project Context for Claude

## Project Overview

This is a Rust-based static analyzer that applies Rust's ownership and borrowing rules to C++ code. The goal is to catch memory safety issues at compile-time without runtime overhead.

## Current State (Updated: Added unsafe propagation checking)

### What's Fully Implemented ✅

**New Advanced Features (Added 2025):**
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
- ✅ **Unified safe/unsafe annotation system for gradual adoption**
  - **Single rule**: Both `@safe` and `@unsafe` only attach to the NEXT code element
  - C++ files are unsafe by default (no checking) for backward compatibility
  - **Namespace-level**: `// @safe` before namespace applies to entire namespace contents
  - **Function-level**: `// @safe` before function enables checking for that function only
  - **No file-level annotation**: To make whole file safe, wrap code in namespace
  - `// @unsafe` works identically to `@safe` - only affects next element
  - No `@endunsafe` - unsafe regions are not supported
  - Fine-grained control over which code is checked
  - Allows gradual migration of existing codebases
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
- ✅ **Comprehensive test suite**: 100+ tests (including pointer safety, move detection, borrow checking, unsafe propagation)

### What's Partially Implemented ⚠️
- ⚠️ Control flow (basic blocks work, loops/conditionals limited)
- ⚠️ Reassignment after move (not tracked yet)
- ⚠️ Method calls (basic support, no virtual functions)

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
  - No member function calls (reset, release, get)
  - Thread safety not analyzed
  
- ❌ **Templates** 
  - Template declarations ignored
  - No instantiation tracking
  - Generic code goes unchecked

#### Important for Correctness
  
- ❌ **Constructor/Destructor (RAII)**
  - Object lifetime not tracked
  - Destructor calls not analyzed
  - RAII patterns not understood

#### Nice to Have
- ❌ **Reassignment after move**
  - Can't track when moved variable becomes valid again
  - `x = std::move(y); x = 42;` - x valid again but not tracked
  
- ❌ **Method calls**
  - Only free functions work
  - No `this` pointer tracking
  - Virtual functions not supported
  
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
2. **Parser**: LibClang for accurate C++ parsing
3. **Solver**: Z3 for lifetime constraint solving
4. **IR Design**: Ownership-aware representation with CFG
5. **Analysis Strategy**: Per-translation-unit with header annotations (no .cpp-to-.cpp needed)

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
# Basic usage
cargo run -- file.cpp

# With include paths
cargo run -- file.cpp -I include -I /usr/local/include

# With compile_commands.json
cargo run -- file.cpp --compile-commands build/compile_commands.json

# Using environment variables
export CPLUS_INCLUDE_PATH=/project/include:/third_party/include
cargo run -- src/main.cpp
```

### Using STL Lifetime Annotations
```cpp
#include <stl_lifetimes.hpp>
#include <vector>
#include <memory>

// @safe
void stl_example() {
    // Vector iterator invalidation detection
    std::vector<int> vec = {1, 2, 3};
    auto it = vec.begin();  // Iterator borrows from vec
    vec.push_back(4);       // ERROR: Would invalidate iterator
    
    // Smart pointer ownership tracking
    std::unique_ptr<int> ptr = std::make_unique<int>(42);
    int& ref = *ptr;        // Reference borrows from ptr
    auto ptr2 = std::move(ptr);  // ERROR: Cannot move while borrowed
}
```

### Using External Annotations
```cpp
#include <unified_external_annotations.hpp>

// Annotate third-party functions
// @external: {
//   third_party::process: [safe, (const Data& d) -> Result]
//   third_party::allocate: [unsafe, (size_t) -> owned void*]
//   sqlite3_column_text: [safe, (stmt* s, int col) -> const char* where s: 'a, return: 'a]
// }

// @safe
void external_example() {
    // C stdlib with lifetime checking
    const char* str = "hello";
    const char* found = strchr(str, 'e');  // Lifetime tied to str
    
    // Third-party functions checked
    Data d;
    Result r = third_party::process(d);  // OK: safe function
    // void* buf = third_party::allocate(100);  // ERROR: unsafe in safe context
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

# Run all tests (70+ tests)
cargo test

# Run specific test categories
cargo test lifetime   # Lifetime tests
cargo test borrow     # Borrow checking tests  
cargo test safe       # Safe/unsafe annotation tests
cargo test move       # Move detection tests

# Run on example files
cargo run -- examples/reference_demo.cpp
cargo run -- examples/safety_annotation_demo.cpp

# Build release binary (standalone, no env vars needed)
cargo build --release
./target/release/rusty-cpp-checker file.cpp
```

## Known Issues

1. **Include Paths**: Standard library headers (like `<iostream>`) aren't found by default
2. **Template Syntax**: Can't parse `std::unique_ptr<T>` or other templates
3. **Limited C++ Support**: Lambdas, virtual functions, and advanced features not supported
4. **No Method Calls**: Can't parse `.get()`, `->operator`, etc.
5. **Left-side dereference**: `*ptr = value` not always detected (assignment target)

## Key Design Insights

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

1. **Parse headers** → Extract lifetime-annotated signatures
2. **Analyze .cpp** → Check implementation against contracts
3. **Validate calls** → Ensure lifetime constraints are met
4. **Report errors** → With clear messages and locations

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

Latest session successfully implemented:
1. ✅ **Unsafe propagation checking** - Safe functions cannot call unmarked/unsafe functions
2. ✅ **Pointer safety checking** - Raw pointer operations require unsafe context
3. ✅ **Type-based operator detection** - Distinguish & from * using type analysis
4. ✅ **Comprehensive test coverage** - Added 20+ new tests for safety features
5. ✅ **Clarified shared_ptr handling** - Move detection is sufficient

Previous achievements:
- ✅ Simplified @unsafe annotation to match @safe behavior
- ✅ Removed @endunsafe - both annotations now only affect next element
- ✅ Verified move detection works for all smart pointers
- ✅ Created standalone binary support with embedded library paths

## Next Priority Tasks

### High Priority
1. **Template parsing** - Required for `std::unique_ptr<T>` and modern C++
2. **Method calls and member access** - For `.get()`, `.release()`, `->operator`
3. **Constructor/Destructor tracking** - RAII patterns

### Medium Priority  
4. **Reassignment tracking** - Variable becomes valid after reassignment
5. **Better error messages** - Code snippets and fix suggestions
6. **Switch/case statements** - Common control flow

### Low Priority
7. **Circular reference detection** - Complex whole-program analysis
8. **Lambda captures** - Complex lifetime tracking
9. **Exception handling** - Stack unwinding
10. **IDE integration (LSP)** - CLI works for now

## Contact with Original Requirements

The tool achieves the core goals:
- ✅ **Standalone static analyzer** - Works independently, can build release binaries
- ✅ **Detect use-after-move** - Fully working with move() detection
- ✅ **Detect multiple mutable borrows** - Fully working
- ✅ **Track lifetimes** - Complete with inference and validation
- ✅ **Detect unsafe pointer operations** - Rust-like pointer safety
- ✅ **Provide clear error messages** - With locations and context
- ✅ **Support gradual adoption** - Per-function/namespace opt-in with @safe