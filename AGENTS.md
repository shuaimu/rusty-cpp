# Rusty-CPP Agent Guide

This file is the repository guidance for coding agents. It is synced from
`CLAUDE.md` and should be updated whenever the project context there changes.

Last synced from `CLAUDE.md`: January 2026 context.

## Project Overview

Rusty-CPP is a Rust-based static analyzer that applies Rust-style ownership,
borrowing, lifetime, and safe/unsafe rules to C++ code. The goal is to catch
memory-safety issues at compile time without runtime overhead.

- Supported C++ standard: strict C++23.
- Parser configuration: `-std=c++23`.
- CMake configuration: `CMAKE_CXX_STANDARD 23` and `CMAKE_CXX_EXTENSIONS OFF`.
- Default toolchain: clang/LLVM. CMake auto-selects `clang++-{19,18,17,16}`
  when `CMAKE_CXX_COMPILER` and `$CXX` are unset.
- GCC can build the non-module subset, but transpiled C++20 module targets are
  clang-only.

## Current Implementation State

The analyzer has broad test coverage and many implemented safety checks. Recent
January 2026 context says 670+ tests cover templates, variadic templates, STL
annotations, C++ casts, pointer safety, move detection, reassignment after move,
borrow checking, unsafe propagation, unsafe blocks, cross-function lifetime
checks, lambda capture safety, RAII tracking, partial moves/borrows, function
pointer safety, string literal tracking, STL use-after-move, and integration
behavior.

Recently implemented features include:

- Chained method temporary detection for patterns like
  `Builder().method().get_ref()`, using `@lifetime: (&'self) -> &'self`.
- Loop dangling reference detection for references to loop-local variables that
  escape an iteration.
- `rusty::move` and `rusty::copy` semantics, including Rust-like invalidation of
  mutable references and forbidden `std::move` on references in `@safe` code.
- Function pointer safety wrappers:
  `rusty::SafeFn`, `rusty::UnsafeFn`, `rusty::SafeMemFn`,
  `rusty::UnsafeMemFn`.
- String literal safety in `@safe` code.
- Partial borrow tracking for individual and nested struct fields.
- RAII tracking for references/pointers stored in containers, user-defined RAII
  types, iterator/container lifetime relationships, lambda escape analysis,
  member lifetimes, and `new`/`delete`.
- Conflict detection, transitive borrow tracking, two-state safety checking,
  header-to-implementation annotation propagation, template analysis, STL
  lifetime annotations, unified external annotations, and raw pointer safety.

Partially implemented or known incomplete areas:

- Virtual function calls have only basic method-call support.
- Loop counter variables declared in `for (int i = ...)` are not fully tracked in
  the variables map.
- Constructor initialization order is not checked.
- Exception handling is not modeled.
- Diagnostics do not yet include code snippets, fix suggestions, or detailed
  borrowing-rule explanations.
- No IDE/LSP integration.

## Safety Model

The project uses a two-state safety system:

- `@safe`
- `@unsafe`

Unannotated code is `@unsafe` by default. `@safe` code can only call `@safe`
functions directly. Calling `@unsafe`, unannotated, STL, or external functions
from `@safe` code requires an `@unsafe` block.

Annotations only attach to the next code element and can have suffixes, such as:

```cpp
// @safe-reviewed
void audited_function();

// @unsafe: uses raw pointers for performance
void low_level_function();
```

Annotation precedence is:

1. Function-level annotations.
2. Class-level annotations.
3. Namespace-level annotations.

Namespace annotations are per-file, not global. The same namespace may be marked
safe in one file and unsafe in another, which supports gradual migration.

## Lifetime Annotation Syntax

Lifetime annotations live in headers and use Rust-like syntax:

```cpp
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

External annotations combine safety and lifetime information:

```cpp
// @external: {
//   third_party::process: [unsafe, (const Data& d) -> Result]
//   sqlite3_column_text: [unsafe, (stmt* s, int col) -> const char* where s: 'a, return: 'a]
// }
```

External functions should be marked `[unsafe]` unless they have been explicitly
audited and intentionally marked `[safe]`.

## Project Structure

```text
src/
|-- main.rs                    # CLI handling and include path resolution
|-- parser/
|   |-- mod.rs                 # Parse orchestration
|   |-- ast_visitor.rs         # AST traversal and function call extraction
|   |-- annotations.rs         # Lifetime annotation parsing
|   `-- header_cache.rs        # Header signature caching
|-- ir/
|   `-- mod.rs                 # IR with CallExpr, Return, CFG
|-- analysis/
|   |-- mod.rs                 # Main analysis coordinator
|   |-- ownership.rs           # Ownership state tracking
|   |-- borrows.rs             # Basic borrow checking
|   |-- lifetimes.rs           # Original lifetime framework
|   |-- lifetime_checker.rs    # Annotation-based checking
|   |-- scope_lifetime.rs      # Scope-based tracking
|   |-- lifetime_inference.rs  # Automatic inference
|   |-- raii_tracking.rs       # RAII tracking
|   `-- lambda_capture_safety.rs
|-- solver/
|   `-- mod.rs                 # Z3 constraint solving
`-- diagnostics/
    `-- mod.rs                 # Error formatting
```

## Development Environment

LLVM/Clang 16.0 or later is required. Libclang is auto-detected in standard
locations.

```bash
# macOS
export Z3_SYS_Z3_HEADER=/opt/homebrew/include/z3.h

# Linux
export Z3_SYS_Z3_HEADER=/usr/include/z3.h

# Optional, only for non-standard LLVM installs
export LIBCLANG_PATH=/path/to/llvm/lib
export LLVM_CONFIG_PATH=/path/to/llvm-config

# Optional include paths
export CPLUS_INCLUDE_PATH=/usr/include/c++:/usr/local/include
export CPATH=/usr/include
```

## Common Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run focused test groups
cargo test lifetime
cargo test borrow
cargo test safe
cargo test move
cargo test template
cargo test raii
cargo test lambda

# Run examples
cargo run -- examples/reference_demo.cpp
cargo run -- examples/safety_annotation_demo.cpp

# Include paths and compile database
cargo run -- file.cpp -I include -I /usr/local/include
cargo run -- file.cpp --compile-commands build/compile_commands.json

# Debug parser/analyzer behavior
cargo run -- file.cpp -vv

# Release binary
cargo build --release
./target/release/rusty-cpp-checker file.cpp

# Lints
cargo clippy
```

## Analysis Approach

The analyzer follows C++ translation-unit boundaries:

1. Parse the main file and included headers, extracting all functions.
2. Track each function's source file path.
3. Classify user code versus system headers by path patterns.
4. Analyze user code for ownership, borrow, pointer safety, and lifetimes.
5. Skip internal borrow analysis for system headers, but still enforce safety
   annotations.
6. Validate calls against safety rules and lifetime constraints.
7. Report diagnostics with source locations and context.

No `.cpp`-to-`.cpp` implementation analysis is required. Function signatures and
lifetime relationships should come from headers.

Template declarations are analyzed once with generic types instead of per
instantiation. This works because move and borrow checks are type-independent for
the relevant rules.

## Code Patterns

When adding a new analysis, follow the existing module style:

```rust
pub fn check_feature(program: &IrProgram, cache: &HeaderCache) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    // Analysis logic.
    Ok(errors)
}
```

Prefer adding small C++ examples and focused integration tests for new behavior.
Useful references include:

- `docs/reference_semantics.md`
- `docs/function_pointer_safety.md`
- `docs/RAII_TRACKING.md`
- `docs/PARTIAL_MOVES_PLAN.md`
- `docs/stl_lifetimes.md`
- `docs/unified_annotations.md`
- `docs/RUST_COMPARISON.md`

## Known Issues

- Standard library headers like `<iostream>` may not be found without include
  path configuration.
- Some advanced C++ constructs remain incomplete, including parts of virtual
  dispatch, constructor initializer order, exception handling, and advanced
  templates such as SFINAE and partial specialization.
- Assignment-target dereference detection for `*ptr = value` is not always
  complete.

## Current Priorities

High priority:

1. Reborrowing support for ergonomic reference-heavy code.

Medium priority:

1. Better error messages with snippets and fix suggestions.
2. Constructor initialization order analysis.
3. Advanced template features, including SFINAE and partial specialization.
4. Switch/case statement support.

Low priority:

1. Two-phase borrows.
2. Loop counter variable tracking for `for (int i = ...)`.
3. More iterator invalidation modeling, such as `clear()` and `erase()`.
4. Circular reference detection.
5. Exception handling.
6. Virtual function analysis.
7. IDE/LSP integration.
8. Non-lexical lifetimes.

## Agent Maintenance Notes

- Keep this file aligned with `CLAUDE.md`.
- If `CLAUDE.md` and code disagree, verify against the code before updating this
  file.
- Preserve the C++23 project standard unless the build configuration changes.
- Add or update tests when changing analyzer behavior.
