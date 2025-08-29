# rusty-cpp: Making C++ Rusty

This project aims to make C++ safer and more reliable by adopting Rust's proven safety principles, especially its borrow-checking system.  We provide a static analyzer that enforces Rust-like ownership and borrowing rules through compile-time analysis. 
<!-- 2. **Style Guide** - Best practices and utilities for writing safer C++ code with Rust-like patterns -->

---

## Borrow checking and lifetime analysis

### 🎯 Vision

This project aims to catch memory safety issues at compile-time by applying Rust's proven ownership model to C++ code. It helps prevent common bugs like use-after-move, double-free, and dangling references before they reach production.

Though C++ is flexible enough to mimic Rust's idioms in many ways, implementing a borrow-checking without modifying the compiler system appears to be impossible, as analyzed in [document](https://docs.google.com/document/d/e/2PACX-1vSt2VB1zQAJ6JDMaIA9PlmEgBxz2K5Tx6w2JqJNeYCy0gU4aoubdTxlENSKNSrQ2TXqPWcuwtXe6PlO/pub). 

We provide rusty-cpp-checker, a standalone static analyzer that enforces Rust-like ownership and borrowing rules for C++ code, bringing memory safety guarantees to existing C++ codebases without runtime overhead. rusty-cpp-checker does not bringing any new grammar into c++. Everything works through simple annoations such as adding `// @safe` enables safety checking on a function.

Note: two projects that (attempt to) implement borrow checking in C++ at compile time are [Circle C++](https://www.circle-lang.org/site/index.html) and [Crubit](https://github.com/google/crubit). As of 2025, Circle is not open sourced, and its design introduces aggressive modifications, such as the ref pointer ^. Crubit is not yet usable on this feature.

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
error: cannot borrow `value` as mutable because it is also borrowed as immutable
  --> example.cpp:6:5
   |
5  |     const int& const_ref = value;  // Immutable borrow - OK
   |                            ----- immutable borrow occurs here
6  |     int& mut_ref = value;          // ERROR
   |          ^^^^^^^ mutable borrow occurs here
```

<!-- The next best alternative is runtime checking, similar to Rust's `RefCell` smart pointer. 
This repository includes a C++ implementation of this concept (see `ref_cell.h`).
Consider using it instead of `shared_ptr` when appropriate.

A standalone static analyzer that enforces Rust-like ownership and borrowing rules for C++ code, bringing memory safety guarantees to existing C++ codebases without runtime overhead. -->



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

#### Prerequisites

- **Rust**: 1.70+ (for building the analyzer)
- **LLVM/Clang**: 14+ (for parsing C++)
- **Z3**: 4.8+ (for constraint solving)

#### macOS

```bash
# Install dependencies
brew install llvm z3

# Clone the repository
git clone https://github.com/shuaimu/rusty-cpp
cd rusty-cpp

# Build the project
cargo build --release

# Run tests
./run_tests.sh

# Add to PATH (optional)
export PATH="$PATH:$(pwd)/target/release"
```

**Note**: The project includes a `.cargo/config.toml` file that automatically sets the required environment variables for Z3. If you encounter build issues, you may need to adjust the paths in this file based on your system configuration.

#### Linux (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install llvm-14-dev libclang-14-dev libz3-dev

# Clone and build
git clone https://github.com/shuaimu/rusty-cpp
cd rusty-cpp
cargo build --release
```

#### Windows

```bash
# Install LLVM from https://releases.llvm.org/
# Install Z3 from https://github.com/Z3Prover/z3/releases
# Set environment variables:
set LIBCLANG_PATH=C:\Program Files\LLVM\lib
set Z3_SYS_Z3_HEADER=C:\z3\include\z3.h

# Build
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

For convenience, add these to your shell profile:

```bash
# ~/.zshrc or ~/.bashrc
export Z3_SYS_Z3_HEADER=/opt/homebrew/opt/z3/include/z3.h
export DYLD_LIBRARY_PATH=/opt/homebrew/opt/llvm/lib:$DYLD_LIBRARY_PATH
```

### 🛡️ Safety Annotations

The borrow checker uses a unified annotation system for gradual adoption in existing codebases:

#### Unified Rule
`@safe` and `@unsafe` annotations attach to the **next** code element (namespace, function, or first statement).

```cpp
// Example 1: Namespace-level safety
// @safe
namespace myapp {
    void func() { /* checked */ }
}

// Example 2: Function-level safety
// @safe
void checked_function() { /* checked */ }

void unchecked_function() { /* not checked - default is unsafe */ }

// Example 3: First-element rule
// @safe
int global = 42;  // Makes entire file safe

// Example 4: Unsafe blocks within safe functions
// @safe
void mixed_safety() {
    int value = 42;
    int& ref1 = value;
    
    // @unsafe
    {
        int& ref2 = value;  // Not checked in unsafe block
    }
    // @endunsafe
}
```

#### Default Behavior
- Files are **unsafe by default** (no checking) for backward compatibility
- Use `@safe` to opt into borrow checking
- Use `@unsafe` to explicitly disable checking

### 📝 Examples

#### Example 1: Use After Move

```cpp
#include <memory>

void bad_code() {
    std::unique_ptr<int> ptr1 = std::make_unique<int>(42);
    std::unique_ptr<int> ptr2 = std::move(ptr1);
    
    *ptr1 = 10;  // ERROR: Use after move!
}
```

**Output:**
```
error: use of moved value: `ptr1`
  --> example.cpp:6:5
   |
6  |     *ptr1 = 10;
   |     ^^^^^ value used here after move
   |
note: value moved here
  --> example.cpp:5:34
   |
5  |     std::unique_ptr<int> ptr2 = std::move(ptr1);
   |                                  ^^^^^^^^^^^^^^
```

#### Example 2: Multiple Mutable Borrows

```cpp
void bad_borrow() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = value;  // ERROR: Cannot borrow as mutable twice
}
```

#### Example 3: Lifetime Violation

```cpp
int& dangling_reference() {
    int local = 42;
    return local;  // ERROR: Returning reference to local variable
}
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

Unfortunately, Rust does not support this level of integration (perhaps intentionally to avoid becoming a secondary option in the C++ ecosystem), as discussed [here](https://internals.rust-lang.org/t/true-c-interop-built-into-the-language/19175/5).
Currently, the best approach for C++/Rust interoperability is through the cxx/autocxx crates.
This interoperability is implemented as a semi-automated process based on C FFIs (Foreign Function Interfaces) that both C++ and Rust support.
However, if your C++ code follows the guidelines in this document, particularly if all types are POD, the interoperability experience can approach the seamless integration offered by other languages (though this remains to be verified).

<!-- ### TODO 

* Investigate microsoft [proxy](https://github.com/microsoft/proxy). It looks like a promising approach to add polymorphism to POD types. But can it be integrated with cxx/autocxx?  
* Investigate autocxx. It provides an interesting feature to implement a C++ subclass in Rust. Can it do the reverse (implement a Rust trait in C++)?
* Multi threading? 
* Make the RefCell implementation has the same memory layout and APIs as the Rust standard library. Then integrate it into autocxx. -->

---

**⚠️ Note**: This is an experimental tool. Use it at your own discretion.  
