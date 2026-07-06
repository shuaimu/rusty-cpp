# rusty-cpp

[![CI](https://github.com/shuaimu/rusty-cpp/actions/workflows/ci.yml/badge.svg)](https://github.com/shuaimu/rusty-cpp/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/documentation-online-blue)](http://mpaxos.com/software/rusty-cpp.html)

Bringing Rust's safety to C++ — in both directions. You can have your existing C++ *checked* like Rust, or you can *write* Rust and ship C++:

**1. Static Borrow Checker** - Compile-time ownership and lifetime analysis for C++ via `rusty-cpp-checker`.
<br>
**2. Rust-to-C++ Translation** - `rusty-cpp-transpiler` translates Rust crates into readable C++20 modules; validated by running real crates' own test suites (serde, semver, smallvec, ...) through the pipeline.
<br>
**3. Rust as an Embedded DSL** - Write individual functions in Rust *inside* your `.cpp` files; the tool keeps a generated C++ fallback in-place, so your build stays a plain C++ build.
<br>
**4. Safe Types** - `Box<T>`, `RefCell<T>`, `Vec<T>`, `HashMap<K,V>`, etc. — the same `rusty::` runtime the transpiler emits code against.
<br>
**5. Rust Idioms** - `Send`/`Sync` traits, RAII guards, type-state patterns, `Result<T,E>`/`Option<T>`, etc.

Which tool do I want?

| You have | You want | Use |
|---|---|---|
| Existing C++ | Memory-safety checking without changing the code | `rusty-cpp-checker` (§1) |
| A Rust crate (or the wish to write one) | C++20 modules that drop into a C++ build | `rusty-cpp-transpiler` (§2) |
| A C++ codebase you're migrating piecemeal | Rust for new/rewritten functions, C++ everywhere else | inline Rust DSL (§3) |
| Hand-written C++ | Rust-shaped types and idioms | `rusty::` headers (§4, §5) |

---

## 1. Borrow Checking and Lifetime Analysis

### 🎯 Vision

This project aims to catch memory safety issues at compile-time by applying Rust's proven ownership model to C++ code. It helps prevent common bugs like use-after-move, double-free, and dangling references before they reach production.

Though C++ is flexible enough to mimic Rust's idioms in many ways, implementing a borrow-checking without modifying the compiler system appears to be impossible, as analyzed in [this document](https://docs.google.com/document/d/e/2PACX-1vSt2VB1zQAJ6JDMaIA9PlmEgBxz2K5Tx6w2JqJNeYCy0gU4aoubdTxlENSKNSrQ2TXqPWcuwtXe6PlO/pub). 

We provide rusty-cpp-checker, a standalone static analyzer that enforces Rust-like ownership and borrowing rules for C++ code, bringing memory safety guarantees to existing C++ codebases without runtime overhead. rusty-cpp-checker does not bringing any new grammar into c++. Everything works through simple annoations such as adding `// @safe` enables safety checking on a function.


### Example

Here's a simple demonstration of how const reference borrowing works:

```cpp
// @safe
void demonstrate_const_ref_borrowing() {
    int value = 42;
    
    // Multiple const references are allowed (immutable borrows)
    const int& ref1 = value;  // First immutable borrow - OK
    const int& ref2 = value;  // Second immutable borrow - OK  
    const int& ref3 = value;  // Third immutable borrow - OK
    
    // All can be used simultaneously to read the value
    int sum = ref1 + ref2 + ref3;  // OK - reading through const refs
}

// @safe
void demonstrate_const_ref_violation() {
    int value = 42;
    
    const int& const_ref = value;  // Immutable borrow - OK
    int& mut_ref = value;          // ERROR: Cannot have mutable borrow when immutable exists
    
    // This would violate the guarantee that const_ref won't see unexpected changes
    mut_ref = 100;  // If allowed, const_ref would suddenly see value 100
}
```

**Analysis Output:**
```
Rusty C++ Checker
Analyzing: example.cpp
✗ Found 2 violation(s) in example.cpp:
Cannot create mutable reference to 'value': already immutably borrowed
Cannot create mutable borrow 'mut_ref': 'value' is already borrowed by 'const_ref'
```

### ✨ Features

#### Core Capabilities
- **🔄 Borrow Checking**: Enforces Rust's borrowing rules (multiple readers XOR single writer)
- **🔒 Ownership Tracking**: Ensures single ownership of resources with move semantics
- **⏳ Lifetime Analysis**: Validates that references don't outlive their data
<!-- - **🎯 Smart Pointer Support**: Special handling for `std::unique_ptr`, `std::shared_ptr`, and `std::weak_ptr` -->
<!-- - **🎨 Beautiful Diagnostics**: Clear, actionable error messages with source locations -->

#### Detected Issues
- Use-after-move violations
- Multiple mutable borrows
- Dangling references
- Lifetime constraint violations
- RAII violations
- Data races (through borrow checking)

### 📦 Installation

#### Option 1: Git Submodule (Recommended)

**The recommended way to use rusty-cpp** is to use cmake to do automatic checking at build. Also consider adding rusty-cpp as a submoduel so it is easy to track updates as rusty-cpp is rapidly evolving.
See [cmake-example-project/](cmake-example-project/) for a complete working example.


#### Option 2: Global Install Script

For system-wide installation, use our install script which detects your OS and installs all dependencies:

```bash
curl -sSL https://raw.githubusercontent.com/shuaimu/rusty-cpp/main/install.sh | bash
```

Or clone and run locally:
```bash
git clone https://github.com/shuaimu/rusty-cpp
cd rusty-cpp
./install.sh
```

**Supported platforms:** macOS (Homebrew), Ubuntu/Debian (apt), Fedora (dnf), CentOS/RHEL 8+ (dnf), Arch Linux (pacman)

#### Option 3: Manual Build

**Prerequisites** (must be installed before building):
- **Rust**: 1.70+
- **LLVM/Clang**: 16+ (for parsing C++)
- **Z3**: 4.8+ (constraint solver)

##### macOS

```bash
brew install llvm z3
git clone https://github.com/shuaimu/rusty-cpp
cd rusty-cpp
cargo build --release
```

##### Linux (Ubuntu/Debian)

```bash
sudo apt-get install llvm-16-dev libclang-16-dev clang-16 libz3-dev
git clone https://github.com/shuaimu/rusty-cpp
cd rusty-cpp
cargo build --release
```

##### Windows

```bash
# Install LLVM from https://releases.llvm.org/
set LIBCLANG_PATH=C:\Program Files\LLVM\lib
cargo build --release
```

### 🚀 Usage

#### Basic Usage

```bash
# Analyze a single file
rusty-cpp-checker path/to/file.cpp

# Analyze with verbose output
rusty-cpp-checker -vv path/to/file.cpp

# Output in JSON format (for IDE integration)
rusty-cpp-checker --format json path/to/file.cpp
```

#### Standalone Binary (No Environment Variables Required)

For release distributions, we provide a standalone binary that doesn't require setting environment variables:

```bash
# Build standalone release
./build_release.sh

# Install from distribution
cd dist/rusty-cpp-checker-*/
./install.sh

# Or use directly
./rusty-cpp-checker-standalone file.cpp
```

See [RELEASE.md](RELEASE.md) for details on building and distributing standalone binaries.

#### Environment Setup (macOS)

No environment variables required! Both Z3 and LLVM are auto-detected via pkg-config at build time.

If dependencies are in non-standard locations, you can optionally set:
```bash
export LIBCLANG_PATH=/path/to/llvm/lib
export Z3_SYS_Z3_HEADER=/path/to/z3.h
```

### 🛡️ Safety System

The borrow checker uses a two-state safety system with automatic header-to-implementation propagation:

#### Two Safety States

1. **`@safe`** - Functions with full borrow checking and strict calling rules
2. **`@unsafe`** - Everything else (unannotated code is @unsafe by default)

#### Calling Rules Matrix

| Caller → Can Call | @safe | @unsafe |
|-------------------|-------|---------|
| **@safe**         | ✅ Yes | ❌ No (use `@unsafe` block) |
| **@unsafe**       | ✅ Yes | ✅ Yes  |

#### Safety Rules Explained

```cpp
// @safe
void safe_function() {
    // ✅ CAN call other @safe functions
    safe_helper();

    // ❌ CANNOT call @unsafe functions directly
    // unsafe_func();  // ERROR: must use @unsafe block

    // ✅ CAN call @unsafe functions via @unsafe block
    // @unsafe
    {
        unsafe_func();  // OK: in @unsafe block
        std::vector<int> vec;  // OK: STL in @unsafe block
    }

    // ✅ CAN use pointers (treated like references)
    int x = 42;
    int* ptr = &x;  // OK: pointers allowed in safe code

    // ❌ CANNOT do pointer arithmetic or use nullptr
    // ptr++;              // ERROR: pointer arithmetic requires unsafe context
    // int* p = nullptr;   // ERROR: nullptr requires unsafe context
}

// @unsafe (or no annotation - same thing)
void unsafe_function() {
    // ✅ Can call anything, use nullptr, and do pointer arithmetic
    safe_function();       // OK: can call safe
    another_unsafe();      // OK: can call unsafe
    int* ptr = nullptr;    // OK: nullptr allowed
    ptr++;                 // OK: pointer arithmetic allowed
    std::vector<int> vec;  // OK: STL allowed
}
```

**Key Insight**: This is a clean two-state model - code is either `@safe` or `@unsafe`. Unannotated code is `@unsafe` by default. To call anything unsafe from `@safe` code, wrap it in an `@unsafe { }` block.

#### Header-to-Implementation Propagation

Safety annotations in headers automatically apply to implementations:

```cpp
// math.h
// @safe
int calculate(int a, int b);

// @unsafe  
void process_raw_memory(void* ptr);

// math.cpp
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

#### STL and External Libraries

By default, all STL and external functions are **@unsafe**, meaning `@safe` functions cannot call them directly. You have three options:

**Option 1 (Recommended): Use Rusty structures**

```cpp
#include <rusty/box.hpp>
#include <rusty/vec.hpp>

// @safe
void safe_with_rusty() {
    // ✅ OK: Rusty structures are designed for safe code
    rusty::Vec<int> vec;
    vec.push_back(42);  // Safe by design

    rusty::Box<Widget> widget = rusty::Box<Widget>::make(args);
}
```

**Option 2: Use @unsafe blocks for STL**

```cpp
#include <vector>

// @safe
void safe_with_stl() {
    // @unsafe
    {
        std::vector<int> vec;
        vec.push_back(42);  // OK: in @unsafe block
    }
}
```

**Option 3: Mark specific external functions as [safe] via external annotations**

If you've audited an external function and want to call it directly from `@safe` code:

```cpp
// @external: {
//   my_audited_function: [safe, () -> void]
// }

void my_audited_function();  // External function you've audited

// @safe
void caller() {
    my_audited_function();  // OK: marked [safe] via external annotation
}
```

See [Complete Annotations Guide](docs/annotations.md) for comprehensive documentation on all annotation features, including safety, lifetime, and external annotations.

### 📝 Examples

#### Example 1: Use After Move

```cpp
#include <rusty/box.hpp>

// @safe
void bad_code() {
    rusty::Box<int> ptr1 = rusty::Box<int>::make(42);
    rusty::Box<int> ptr2 = std::move(ptr1);

    *ptr1 = 10;  // ERROR: Use after move!
}
```

**Output:**
```
Rusty C++ Checker
Analyzing: example.cpp
✗ Found 1 violation(s) in example.cpp:
Use after move: cannot dereference_write (via operator*) variable 'ptr1' because it has been moved
```

#### Example 2: Multiple Mutable Borrows

```cpp
// @safe
void bad_borrow() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = value;  // ERROR: Cannot borrow as mutable twice
}
```

**Output:**
```
Rusty C++ Checker
Analyzing: example.cpp
✗ Found 3 violation(s) in example.cpp:
Cannot create mutable reference to 'value': already mutably borrowed
Cannot create mutable borrow 'ref1': 'value' is already borrowed by 'ref2'
Cannot create mutable borrow 'ref2': 'value' is already borrowed by 'ref1'
```

#### Example 3: Lifetime Violation

```cpp
// @safe
int& dangling_reference() {
    int local = 42;
    return local;  // ERROR: Returning reference to local variable
}
```

**Output:**
```
Rusty C++ Checker
Analyzing: example.cpp
✗ Found 2 violation(s) in example.cpp:
Safe function 'dangling_reference' returns a reference but has no @lifetime annotation
Cannot return reference to local variable 'local'
```

### 🏗️ Architecture

```
┌─────────────┐     ┌──────────┐     ┌────────┐
│   C++ Code  │────▶│  Parser  │────▶│   IR   │
└─────────────┘     └──────────┘     └────────┘
                          │                │
                    (libclang)              ▼
                                    ┌──────────────┐
┌─────────────┐     ┌──────────┐   │   Analysis   │
│ Diagnostics │◀────│  Solver  │◀──│   Engine     │
└─────────────┘     └──────────┘   └──────────────┘
                         │                │
                       (Z3)        (Ownership/Lifetime)
```

#### Components

- **Parser** (`src/parser/`): Uses libclang to build C++ AST
- **IR** (`src/ir/`): Ownership-aware intermediate representation
- **Analysis** (`src/analysis/`): Core borrow checking algorithms
- **Solver** (`src/solver/`): Z3-based constraint solving for lifetimes
- **Diagnostics** (`src/diagnostics/`): User-friendly error reporting

### 🆕 Advanced Features

#### Using Rusty Structures (Recommended)

RustyCpp provides safe data structures that integrate seamlessly with the borrow checker:

```cpp
#include <rusty/vec.hpp>
#include <rusty/box.hpp>

// @safe
void example() {
    rusty::Vec<int> vec = {1, 2, 3};
    int& ref = vec[0];     // Borrows &'vec mut
    vec.push_back(4);      // ERROR: Cannot modify vec while ref exists
}
```

**For STL structures**, use `@unsafe` blocks:

```cpp
#include <vector>

// @safe
void stl_example() {
    // @unsafe
    {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);  // OK: in @unsafe block
    }
}
```

See [Complete Annotations Guide](docs/annotations.md) for all annotation features.

#### External Function Annotations

Annotate third-party functions with safety and lifetime information without modifying their source.

By default, all external functions are `@unsafe`. You can:

1. **Use `@unsafe` blocks** to call them from `@safe` code
2. **Mark specific functions as `[safe]`** if you've audited them

```cpp
// @external: {
//   // Mark as [safe] if you've audited the function
//   my_audited_function: [safe, () -> void]
//
//   // Mark as [unsafe] with lifetime info for documentation
//   strchr: [unsafe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   malloc: [unsafe, (size_t size) -> owned void*]
// }

void my_audited_function();

// @safe
void example() {
    my_audited_function();  // OK: marked [safe] via external annotation

    // @unsafe
    {
        const char* text = "hello";
        const char* found = strchr(text, 'e');  // OK: in @unsafe block
    }
}
```

See [Complete Annotations Guide](docs/annotations.md) for comprehensive documentation on safety annotations, lifetime annotations, and external annotations.

---

## 2. Rust-to-C++ Translation

### 🎯 Vision

The borrow checker above retrofits Rust's rules onto C++. The transpiler takes the opposite (and stronger) route: **write actual Rust, let `rustc` be the safety authority, and translate the result into C++20 modules** that drop into an ordinary C++ build. Rust becomes a front-end language for C++ projects — you get real borrow checking, real pattern matching, and real trait-based generics, while your build system, linker, and the rest of your codebase stay C++.

The core contract is a **forward correctness guarantee**: if the Rust source compiles under `rustc`, the generated C++ compiles and produces the same results. Rust is the source of truth; the C++ output is a faithful, readable rendering — not the other way around. (C++ being more permissive than Rust, the reverse direction is explicitly not claimed.) See [docs/rusty-cpp-transpiler.md](docs/rusty-cpp-transpiler.md) for the full construct-by-construct mapping.

### Example

```rust
// point.rs
pub struct Point { pub x: i32, pub y: i32 }

impl Point {
    pub fn flip(self) -> Point {
        Point { x: self.y, y: self.x }
    }
    pub fn norm2(&self) -> i32 {
        self.x * self.x + self.y * self.y
    }
}

pub fn farthest(points: &[Point]) -> Option<&Point> {
    let mut best: Option<&Point> = None;
    for p in points {
        match best {
            Some(b) if b.norm2() >= p.norm2() => {}
            _ => best = Some(p),
        }
    }
    best
}
```

```bash
rusty-cpp-transpiler point.rs -o point.cppm -m point
```

produces a C++20 module exporting `Point`, `flip`, `norm2`, and `farthest` against the `rusty::` runtime (`rusty::Option`, `std::span` for the slice, pattern-match lowering for the `match`), importable from any C++ translation unit with `import point;`.

### Usage

```bash
# Build the transpiler (pure Rust — no LLVM/Z3 needed, unlike the checker)
cargo build --release -p rusty-cpp-transpiler

# Single file -> single C++20 module
rusty-cpp-transpiler input.rs -o output.cppm -m my_module

# Whole crate -> one module per Rust module, with cross-module imports
rusty-cpp-transpiler --crate path/to/Cargo.toml --output-dir cpp_out

# Generate a CMakeLists.txt for the emitted modules from Cargo.toml
rusty-cpp-transpiler --crate path/to/Cargo.toml --cmake path/to/Cargo.toml

# Crates using macros: expand first (requires cargo-expand)
rusty-cpp-transpiler --crate path/to/Cargo.toml --expand

# Close the loop: run the borrow checker over the transpiled output
rusty-cpp-transpiler input.rs -o output.cppm --verify
```

Compiling the output requires a clang toolchain with C++20 modules support (the test matrix builds with `clang++ -std=c++23`); the emitted code `#include`s the header-only `rusty/` runtime from this repository. Generated code can also call *into* existing C++: `use cpp::...` imports resolve against a user-supplied C++ module symbol index (`--cpp-module-index`).

### What's covered

Generics and traits (including associated types and trait objects), data-carrying enums with pattern matching, closures, iterators and adapter chains, the `?` operator, `Box`/`Rc`/`Arc`/`Cell`/`RefCell`, slices and string handling, `Formatter`-based `Debug`/`Display`, multi-module crates with cross-crate dependencies, and — increasingly — `unsafe` raw-pointer code (exercised by porting a C-style YAML parser). Rust's module tree maps onto C++20 modules plus namespaces; traits lower to free-function namespaces resolved via compile-time dispatch, so generic code stays template-friendly with no virtual-dispatch overhead.

### How it's validated: the parity matrix

Every claim above is enforced by CI-style parity testing against **real, unmodified crates from crates.io**: each crate *and its own test suite* are transpiled, compiled with clang, and executed — and the test results must match `cargo test` on the Rust side.

Currently green in the default matrix: `either`, `tap`, `cfg-if`, `take_mut`, `arrayvec`, `semver`, `bitflags`, `smallvec`, `once_cell`, `serde_bytes`, `serde_repr`, `serde_core`, `serde`, `pollster` (plus a focused local `Vec` suite). In progress: `itertools`, `hashbrown`, `indexmap`, `serde_yaml`.

```bash
# Run one crate through the parity pipeline
bash tests/transpile_tests/run_parity_matrix.sh --crate semver
```

---

## 3. Rust as an Embedded DSL in C++

For codebases that can't (or shouldn't) move whole crates at once, the transpiler supports **inline Rust blocks inside `.cpp` files**. You write a function in Rust where it lives today; the tool generates and maintains the equivalent C++ right below it. Normal builds compile the generated C++ (`RUSTYCPP_RUST=0`) — consumers of your project never need a Rust toolchain.

```cpp
#if RUSTYCPP_RUST
fn clamp_add(a: i32, b: i32, lo: i32, hi: i32) -> i32 {
    let s = a + b;
    if s < lo { lo } else if s > hi { hi } else { s }
}
#endif
/*RUSTYCPP:GEN-BEGIN id=clamp_add version=1 rust_sha256=<hex>*/
// generated C++ (read-only; regenerated from the Rust above)
/*RUSTYCPP:GEN-END id=clamp_add*/
```

```bash
# Validate marker structure and Rust-payload hashes (CI-friendly)
rusty-cpp-transpiler inline-rust --check --files src/*.cpp

# Regenerate the C++ fallback regions in place
rusty-cpp-transpiler inline-rust --rewrite --files src/*.cpp
```

The generator is deterministic, touches only the `GEN` regions, and records a `rust_sha256` of the Rust payload so CI can reject stale fallbacks. V1 deliberately accepts a conservative Rust subset (free functions, structs with named fields, inherent impls, `Option`/`Result`/`Vec`/`String`, standard control flow) and keeps each block local to its translation unit — no cross-TU declaration magic. See §12 of [docs/rusty-cpp-transpiler.md](docs/rusty-cpp-transpiler.md) for the normative grammar and subset.

---

## 4. Safe Type Alternatives

RustyCpp provides drop-in replacements for C++ standard library types with built-in safety guarantees:

### Available Types

#### Smart Pointers
- **`rusty::Box<T>`** - Single ownership pointer with move-only semantics
  ```cpp
  rusty::Box<Widget> widget = rusty::Box<Widget>::make(42);
  auto widget2 = std::move(widget);  // OK: explicit ownership transfer
  // widget.get();  // ERROR: use-after-move detected
  ```

#### Interior Mutability
- **`rusty::RefCell<T>`** - Runtime borrow checking for interior mutability
  ```cpp
  rusty::RefCell<int> cell(42);
  {
      auto ref = cell.borrow();      // Immutable borrow
      // auto mut_ref = cell.borrow_mut();  // ERROR: already borrowed
  }
  auto mut_ref = cell.borrow_mut();  // OK: previous borrow ended
  ```

#### Containers
- **`rusty::Vec<T>`** - Dynamic array with iterator invalidation detection
  ```cpp
  rusty::Vec<int> vec = {1, 2, 3};
  auto it = vec.begin();
  // vec.push_back(4);  // ERROR: would invalidate iterator
  ```

- **`rusty::HashMap<K, V>`** - Hash map with safe concurrent access patterns
- **`rusty::HashSet<T>`** - Hash set with ownership semantics
- **`rusty::Rc<T>`** - Reference counted pointer (single-threaded)
- **`rusty::Arc<T>`** - Atomic reference counted pointer (thread-safe)

#### Utility Types
- **`rusty::Option<T>`** - Explicit handling of optional values
- **`rusty::Result<T, E>`** - Explicit error handling

### Usage

Include the headers:
```cpp
#include <rusty/box.hpp>
#include <rusty/refcell.hpp>
#include <rusty/vec.hpp>
#include <rusty/hashmap.hpp>
```

These types are designed to work seamlessly with the borrow checker and enforce Rust's safety guarantees at runtime. They are also the runtime the transpiler (§2) emits code against — hand-written `rusty::` C++ and transpiled Rust share one type system, so the two styles mix freely in a codebase.

---

## 5. Rust Design Idioms

RustyCpp implements key Rust design patterns for safer concurrent programming:

### Thread Safety Traits

#### Send Trait (Explicit Opt-In System)

RustyCpp implements Rust's `Send` trait using an **explicit opt-in** system that prevents accidental data races at compile-time:

```cpp
#include <rusty/sync/mpsc.hpp>

// ✅ Primitives are pre-marked as Send
auto [tx, rx] = rusty::sync::mpsc::channel<int>();

// ✅ Rusty types are Send if their content is Send
auto [tx, rx] = rusty::sync::mpsc::channel<rusty::Arc<int>>();

// ❌ Rc is NOT Send (non-atomic reference counting)
auto [tx, rx] = rusty::sync::mpsc::channel<rusty::Rc<int>>();  // Compile error!

// ❌ Unmarked user types are NOT Send (must explicitly mark)
struct MyType { int value; };
auto [tx, rx] = rusty::sync::mpsc::channel<MyType>();  // Compile error!
```

**How to mark your types as Send:**

```cpp
// Method 1: Static marker (recommended for your types)
class ThreadSafe {
public:
    static constexpr bool is_send = true;  // Explicitly mark as Send
    // ... your thread-safe implementation
};

// Method 2: External specialization (for third-party types)
namespace rusty {
    template<>
    struct is_explicitly_send<ThirdPartyType> : std::true_type {};
}
```

**Key Features:**
- **Safe by default**: Types are NOT Send unless explicitly marked
- **Compositional safety**: `struct { Rc<T> }` is automatically rejected (no Send marker)
- **Clear errors**: Compiler tells you exactly how to fix the issue
- **No deep analysis needed**: Simple marker check at compile-time

**Example - Compositional Safety:**

```cpp
// Without marker, this is NOT Send (safe!)
struct ContainsRc {
    rusty::Rc<int> data;  // Non-thread-safe
};

auto [tx, rx] = channel<ContainsRc>();  // ✗ Compile error!
// Error: ContainsRc must be Send (marked explicitly)

// Arc is thread-safe, so use it instead
struct ThreadSafeVersion {
    static constexpr bool is_send = true;
    rusty::Arc<int> data;  // Thread-safe
};

auto [tx, rx] = channel<ThreadSafeVersion>();  // ✓ Works!
```

#### MPSC Channel (Multi-Producer Single-Consumer)

Thread-safe message passing channel, identical to Rust's `std::sync::mpsc`:

```cpp
#include <rusty/sync/mpsc_lockfree.hpp>  // Lock-free (recommended)
// or
#include <rusty/sync/mpsc.hpp>  // Mutex-based
#include <thread>

using namespace rusty::sync::mpsc::lockfree;  // or ::mutex

void example() {
    // Create channel
    auto [tx, rx] = channel<int>();

    // Clone sender for multiple producers
    auto tx2 = tx.clone();

    // Producer threads
    std::thread t1([tx = std::move(tx)]() mutable {
        tx.send(42);
    });

    std::thread t2([tx2 = std::move(tx2)]() mutable {
        tx2.send(100);
    });

    // Consumer receives from both
    for (int i = 0; i < 2; i++) {
        auto result = rx.recv();
        if (result.is_ok()) {
            int value = result.unwrap();
            std::cout << "Received: " << value << "\n";
        }
    }

    t1.join();
    t2.join();
}
```

**Two Implementations Available:**

1. **Lock-Free** (`mpsc_lockfree.hpp`) - **Recommended**
   - 28 M msg/s throughput, 3.3 μs p50 latency
   - Batch operations (4x faster under contention)
   - Wait-free consumer, lock-free producers
   - See [User Guide](docs/mpsc_lockfree_user_guide.md) and [Developer Guide](docs/mpsc_lockfree_developer_guide.md)

2. **Mutex-Based** (`mpsc.hpp`)
   - Simple, straightforward implementation
   - Lower throughput but easier to understand
   - Good for low-frequency communication

**Common Features:**
- Blocking operations: `send()`, `recv()`
- Non-blocking operations: `try_send()`, `try_recv()`
- Disconnection detection
- Type-safe with Send constraint
- Rust-compatible API

#### Sync Trait

Compile-time marker for types safe to share references between threads:

```cpp
template<typename T>
concept Sync = /* implementation */;
```

### RAII Guards

Scope-based resource management following Rust's guard pattern:

```cpp
// MutexGuard automatically unlocks on scope exit
auto guard = mutex.lock();
// ... use protected data ...
// Guard destroyed here, mutex automatically unlocked
```

### Type-State Patterns

Encode state machines in the type system:

```cpp
struct Unopened {};
struct Opened {};

template<typename State>
class File {
    // Only available when Opened
    std::string read_line() requires std::same_as<State, Opened>;
};

File<Unopened> file("data.txt");
File<Opened> opened = file.open();  // State transition via move
std::string line = opened.read_line();  // OK
```

### Error Handling

Rust-style `Result` and `Option` types:

```cpp
rusty::Result<int, std::string> parse_number(const std::string& str);
rusty::Option<int> find_value(const std::string& key);

// Pattern matching style
auto result = parse_number("42");
if (result.is_ok()) {
    int value = result.unwrap();
} else {
    std::string error = result.unwrap_err();
}
```

---

## Tips in writing rusty c++
Writing C++ that is easier to debug by adopting principles from Rust.

### Being Explicit

Explicitness is one of Rust's core philosophies. It helps prevent errors that arise from overlooking hidden or implicit code behaviors.

#### No computation in constructors/destructors
Constructors should be limited to initializing member variables and establishing the object's memory layout—nothing more. For additional initialization steps, create a separate `Init()` function. When member variables require initialization, handle this in the `Init()` function rather than in the constructor.

Similarly, if you need computation in a destructor (such as setting flags or stopping threads), implement a `Destroy()` function that must be explicitly called before destruction.

#### Composition over inheritance
Avoid inheritance whenever possible.

When polymorphism is necessary, limit inheritance to a single layer: an abstract base class and its implementation class. The abstract class should contain no member variables, and all its member functions should be pure virtual (declared with `= 0`). The implementation class should be marked as `final` to prevent further inheritance.

#### Use move and disallow copy assignment/constructor
Except for primitive types, prefer using `move` instead of copy operations. There are multiple ways to disallow copy constructors; our convention is to inherit from the `boost::noncopyable` class:

```cpp
class X: private boost::noncopyable {};
```

If copy from an object is necessary, implement move constructor and a `Clone` function:
```cpp
Object obj1 = move(obj2.Clone()); // move can be omitted because it is already a right value. 
```

### Memory Safety, Pointers, and References

#### No raw pointers
Avoid using raw pointers except when required by system calls, in which case wrap them in a dedicated class.

<!-- #### Ownership model: unique_ptr and shared_ptr

Program with the ownership model, where each object is owned by another object or function throughout its lifetime.

To transfer ownership, wrap objects in `unique_ptr`.

Avoid shared ownership whenever possible. While this can be challenging, it's achievable in most cases. If shared ownership is truly necessary, consider using `shared_ptr`, but be aware that it incurs a non-negligible performance cost due to atomic reference counting operations (similar to Rust's `Arc` rather than `Rc`).
 -->

#### Use POD types  
Try to use [POD](https://en.cppreference.com/w/cpp/named_req/PODType) types if possible. POD means "plain old data". A class is POD if:  
* No user-defined copy assignment
* No virtual functions
* No destructor 

### Incrementally Migrate to Rust (C++/Rust Interop)
Some languages (like D, Zig, and Swift) offer seamless integration with C++. This makes it easier to adopt these languages in existing C++ projects, as you can simply write new code in the chosen language and interact with existing C++ code without friction.

Rust does not support this level of integration natively (perhaps intentionally, to avoid becoming a secondary option in the C++ ecosystem), as discussed [here](https://internals.rust-lang.org/t/true-c-interop-built-into-the-language/19175/5). The conventional route is FFI-based bindings through the cxx/autocxx crates.

This project takes a different route around the problem: instead of linking Rust and C++ object code across an FFI boundary, **translate the Rust into C++** (§2, §3). New code is written and checked as Rust, but what enters the build is ordinary C++20 — it interoperates with the rest of the codebase the way any C++ does, with no binding layer, no dual toolchain for consumers, and no ABI seam. Whole crates can come over via `rusty-cpp-transpiler --crate`, and single functions can migrate in place via the inline Rust DSL.

### Closely related projects to watch

Two projects that (attempt to) implement borrow checking in C++ at compile time are [Circle C++](https://github.com/seanbaxter/circle), and [Crubit](https://github.com/google/crubit). 

<!-- ### TODO 

* Investigate microsoft [proxy](https://github.com/microsoft/proxy). It looks like a promising approach to add polymorphism to POD types. But can it be integrated with cxx/autocxx?  
* Investigate autocxx. It provides an interesting feature to implement a C++ subclass in Rust. Can it do the reverse (implement a Rust trait in C++)?
* Multi threading? 
* Make the RefCell implementation has the same memory layout and APIs as the Rust standard library. Then integrate it into autocxx. -->

---

**⚠️ Note**: This is an experimental tool. Use it at your own discretion.  
