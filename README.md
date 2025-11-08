# rusty-cpp

Bringing Rust's safety to C++ through:

**1. Static Borrow Checker** - Compile-time ownership and lifetime analysis via `rusty-cpp-checker`.
<br>
**2. Safe Types** - `Box<T>`, `RefCell<T>`, `Vec<T>`, `HashMap<K,V>`, etc.
<br>
**3. Rust Idioms** - `Send`/`Sync` traits, RAII guards, type-state patterns, `Result<T,E>`/`Option<T>`, etc.

---

## 1. Borrow Checking and Lifetime Analysis

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

#### ⚠️ Build Requirements (IMPORTANT)

This tool requires the following native dependencies to be installed **before** building from source or installing via cargo:

- **Rust**: 1.70+ (for building the analyzer)
- **LLVM/Clang**: 14+ (for parsing C++ - required by clang-sys)
- **Z3**: 4.8+ (for constraint solving - required by z3-sys)

**Note**: These dependencies must be installed system-wide before running `cargo install rusty-cpp` or building from source. The build will fail without them.

#### Installing from crates.io

Once you have the prerequisites installed:

```bash
# macOS: Set environment variable for Z3
export Z3_SYS_Z3_HEADER=/opt/homebrew/include/z3.h

# Linux: Set environment variable for Z3
export Z3_SYS_Z3_HEADER=/usr/include/z3.h

# Install from crates.io
cargo install rusty-cpp

# The binary will be installed as 'rusty-cpp-checker'
rusty-cpp-checker --help
```

#### Building from Source

##### macOS

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

##### Linux (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install llvm-14-dev libclang-14-dev libz3-dev

# Clone and build
git clone https://github.com/shuaimu/rusty-cpp
cd rusty-cpp
cargo build --release
```

##### Windows

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

### 🛡️ Safety System

The borrow checker uses a three-state safety system with automatic header-to-implementation propagation:

#### Three Safety States

1. **`@safe`** - Functions with full borrow checking and strict calling rules
2. **`@unsafe`** - Explicitly marked unsafe functions (documented risks)
3. **Undeclared** (default) - Functions without annotations (unaudited legacy code)

#### Calling Rules Matrix

| Caller → Can Call | @safe | @unsafe | Undeclared |
|-------------------|-------|---------|------------|
| **@safe**         | ✅ Yes | ✅ Yes  | ❌ No      |
| **@unsafe**       | ✅ Yes | ✅ Yes  | ✅ Yes     |
| **Undeclared**    | ✅ Yes | ✅ Yes  | ✅ Yes     |

#### Safety Rules Explained

```cpp
// @safe
void safe_function() {
    // ✅ CAN call other @safe functions
    safe_helper();
    
    // ✅ CAN call @unsafe functions (risks are documented)
    explicitly_unsafe_func();
    
    // ❌ CANNOT call undeclared functions (must audit first!)
    // legacy_function();  // ERROR: must be marked @safe or @unsafe
    
    // ❌ CANNOT do pointer operations
    // int* ptr = &x;  // ERROR: requires unsafe context
}

// @unsafe
void unsafe_function() {
    // ✅ Can call anything and do pointer operations
    legacy_function();     // OK: can call undeclared
    safe_function();       // OK: can call safe
    another_unsafe();      // OK: can call unsafe
    int* ptr = nullptr;    // OK: pointer operations allowed
}

// No annotation - undeclared (default)
void legacy_function() {
    // Not checked by borrow checker
    // ✅ Can call anything including other undeclared functions
    another_legacy();      // OK: undeclared can call undeclared
    safe_function();       // OK: undeclared can call safe
    unsafe_function();     // OK: undeclared can call unsafe
    
    // This enables gradual migration of existing codebases
}
```

**Key Insight**: This creates an "audit ratchet" - once you mark a function as `@safe`, you must explicitly audit all its dependencies. Undeclared functions can freely call each other, allowing existing code to work without modification.

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

By default, all STL and external functions are **undeclared**, meaning safe functions cannot call them without explicit annotation. **The recommended approach is to use Rusty structures instead of STL structures in safe code:**

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

If you need to use STL structures in safe code, you must explicitly annotate them as unsafe:

```cpp
#include <vector>
#include <unified_external_annotations.hpp>

// @external: {
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
// }

// @safe
void safe_with_stl_marked_unsafe() {
    std::vector<int> vec;
    vec.push_back(42);  // OK: vec::push_back is marked as unsafe
    printf("Hello\n");  
}
```

This forces you to audit external code before using it in safe contexts. See [Complete Annotations Guide](docs/annotations.md) for comprehensive documentation on all annotation features, including safety, lifetime, external, and STL annotations.

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
error: use of moved value: `ptr1`
  --> example.cpp:6:5
   |
6  |     *ptr1 = 10;
   |     ^^^^^ value used here after move
   |
note: value moved here
  --> example.cpp:5:29
   |
5  |     rusty::Box<int> ptr2 = std::move(ptr1);
   |                             ^^^^^^^^^^^^^^
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

**For STL structures**, you must use external annotations to mark them as unsafe:

```cpp
#include <vector>
#include <unified_external_annotations.hpp>

// @external: {
//   std::vector::push_back: [unsafe, (&'a mut, T) -> void]
//   std::vector::operator[]: [unsafe, (&'a, size_t) -> &'a]
// }
```

See [Complete Annotations Guide](docs/annotations.md) for all annotation features.

#### External Function Annotations

Annotate third-party functions with both safety and lifetime information without modifying their source:

```cpp
#include <unified_external_annotations.hpp>

// @external: {
//   strchr: [safe, (const char* str, int c) -> const char* where str: 'a, return: 'a]
//   malloc: [unsafe, (size_t size) -> owned void*]
//   sqlite3_column_text: [safe, (sqlite3_stmt* stmt, int col) -> const char* where stmt: 'a, return: 'a]
// }

// @safe
void use_third_party() {
    const char* text = "hello";
    const char* found = strchr(text, 'e');  // OK: safe with lifetime checking
    // void* buf = malloc(100);  // ERROR: unsafe function in safe context
}
```

See [Complete Annotations Guide](docs/annotations.md) for comprehensive documentation on safety annotations, lifetime annotations, external annotations, and STL handling.

For detailed examples, migration guides, troubleshooting, and complete reference tables, see [Complete Annotations Guide](docs/annotations.md).

---

## 2. Safe Type Alternatives

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

These types are designed to work seamlessly with the borrow checker and enforce Rust's safety guarantees at runtime.

---

## 3. Rust Design Idioms

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
