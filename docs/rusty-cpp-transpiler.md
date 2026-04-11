# Rust-to-C++ Transpilation: Feasibility Analysis

## Core Principle: Forward Correctness Guarantee

**If valid Rust compiles, the transpiled C++ must compile and produce the same result.** This is the fundamental guarantee of the transpiler. If a Rust program passes `rustc` (type checking, borrow checking, lifetime checking), then the generated C++ code is guaranteed to compile and behave identically at runtime.

**The reverse is explicitly not guaranteed.** If transpiled C++ happens to compile and run, that does not imply the original Rust source was valid. C++ is more permissive than Rust — it accepts programs that Rust would reject (use-after-move, data races, dangling references, etc.). The transpiler is a one-way correctness bridge: Rust's safety guarantees flow forward into the C++ output, but C++'s permissiveness does not flow backward to validate Rust source.

In other words: Rust is the source of truth for correctness. The transpiler preserves semantics, not the other way around.

---

## Executive Summary

C++ is *almost* a superset of Rust in terms of expressible semantics — both are systems languages with value semantics, deterministic destruction, zero-cost abstractions, and compile-time generics. This makes Rust-to-C++ transpilation broadly feasible, with most language constructs having direct or near-direct mappings. The hard parts are the **trait system** (Rust's core abstraction), **enums with data** (algebraic data types), **pattern matching**, and **lifetime annotations** (which have no C++ equivalent but can be erased in the output). This document maps every major Rust construct to its C++ equivalent, flags the non-obvious cases, and proposes solutions.

---

## 1. Direct Mappings (Straightforward)

These Rust constructs map 1:1 or nearly 1:1 to C++.

### 1.1 Primitive Types

| Rust | C++ |
|------|-----|
| `i8, i16, i32, i64, i128` | `int8_t, int16_t, int32_t, int64_t, __int128` |
| `u8, u16, u32, u64, u128` | `uint8_t, uint16_t, uint32_t, uint64_t, unsigned __int128` |
| `f32, f64` | `float, double` |
| `bool` | `bool` |
| `char` (Unicode scalar) | `char32_t` (note: Rust `char` is 4 bytes, not 1) |
| `usize, isize` | `size_t, ptrdiff_t` |
| `()` (unit) | `void` (return) / empty struct (value context) |
| `!` (never) | `[[noreturn]]` (on functions) |

### 1.2 Variables and Mutability

```rust
let x = 5;           // immutable binding
let mut y = 10;      // mutable binding
const Z: i32 = 42;   // compile-time constant
static S: i32 = 99;  // static variable
```

```cpp
const auto x = 5;         // const by default
auto y = 10;               // mutable
constexpr int32_t Z = 42;  // compile-time constant
static int32_t S = 99;     // static variable (note: not thread-safe init in general)
```

**Key insight**: Rust defaults to immutable, C++ defaults to mutable. The transpiler should emit `const` for all non-`mut` bindings.

### 1.3 Functions

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b   // implicit return
}
```

```cpp
int32_t add(int32_t a, int32_t b) {
    return a + b;  // explicit return needed
}
```

**Note**: Rust's expression-based returns (`a + b` without semicolon) need to be converted to explicit `return` statements. This is a straightforward AST transformation — identify the tail expression of each block.

### 1.4 Control Flow

| Rust | C++ |
|------|-----|
| `if / else if / else` | `if / else if / else` |
| `loop { }` | `while (true) { }` |
| `while cond { }` | `while (cond) { }` |
| `for x in iter` | `for (auto& x : iter)` (range-based) |
| `break` / `continue` | `break` / `continue` |
| `break value` (from loop) | Requires variable + break (see §3.5) |
| `return` | `return` |

### 1.5 References and Borrowing

```rust
fn read(x: &i32) -> i32 { *x }
fn write(x: &mut i32) { *x = 42; }
```

```cpp
int32_t read(const int32_t& x) { return x; }  // no deref needed
void write(int32_t& x) { x = 42; }
```

#### The Rebinding Problem

C++ references **cannot be rebound** — this is a critical semantic mismatch. Rust references behave more like non-null pointers:

```rust
let x = 5;
let y = 10;
let mut r = &x;  // r refers to x
r = &y;           // r now refers to y — REBINDING
```

```cpp
// WRONG — C++ reference version:
int& r = x;
r = y;   // assigns y's value into x, does NOT rebind r!

// CORRECT — use pointer:
const int* r = &x;
r = &y;  // rebinds r to point at y
```

#### Transpilation Strategy: Static Analysis of Rebinding

The transpiler should analyze whether a reference binding is ever reassigned. If not, it can safely emit a C++ reference (more idiomatic, zero overhead). If rebinding occurs, it must fall back to a pointer.

**Step 1**: For each `let mut r: &T` binding, scan all subsequent assignments to `r` in the same scope.

**Step 2**: Choose output based on the result:

```rust
// Case 1: No rebinding — emit C++ reference
let mut r = &x;
println!("{}", r);  // r is never reassigned
```
```cpp
const int& r = x;            // safe: never rebound
std::println("{}", r);
```

```rust
// Case 2: Rebinding detected — emit pointer
let mut r = &x;
r = &y;              // rebinding!
println!("{}", *r);
```
```cpp
const int* r = &x;           // must use pointer
r = &y;                       // rebinding works
std::println("{}", *r);
```

**Decision table**:

| Rust | Rebound? | C++ output |
|------|----------|------------|
| `let r: &T = ...` | N/A (immutable binding) | `const T& r = ...` |
| `let r: &mut T = ...` | N/A (immutable binding) | `T& r = ...` |
| `let mut r: &T = ...` | No | `const T& r = ...` |
| `let mut r: &T = ...` | Yes | `const T* r = &...` |
| `let mut r: &mut T = ...` | No | `T& r = ...` |
| `let mut r: &mut T = ...` | Yes | `T* r = &...` |
| `&T` (function param) | No (typical) | `const T&` |
| `&mut T` (function param) | No (typical) | `T&` |
| `*const T` | — | `const T*` |
| `*mut T` | — | `T*` |

This produces the most idiomatic C++ output — references when possible, pointers only when necessary. The analysis is trivial since Rust's scoping rules make it a simple scan of assignments within the binding's scope.

**Key insight**: Rust's `&T` is a shared (immutable) reference and `&mut T` is an exclusive (mutable) reference. The borrow checker rules are erased in the C++ output (they were enforced at the Rust level). Auto-deref (`*r` in Rust when using pointers) also needs adjustment: C++ references auto-deref, C++ pointers need explicit `*` or `->`.

### 1.6 Structs

```rust
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self { Point { x, y } }
    fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
```

```cpp
struct Point {
    double x;
    double y;

    static Point new_(double x, double y) { return Point{x, y}; }
    double distance(const Point& other) const {
        return std::sqrt(std::pow(x - other.x, 2) + std::pow(y - other.y, 2));
    }
};
```

| Rust method receiver | C++ equivalent |
|----------------------|----------------|
| `&self` | `const` method |
| `&mut self` | non-const method |
| `self` (by value) | method taking `*this` by value (C++23 explicit object) or free function |
| `Self` (associated fn) | `static` method |

### 1.7 Ownership and Move Semantics

```rust
let a = String::from("hello");
let b = a;   // a is moved, no longer usable
```

```cpp
auto a = std::string("hello");
auto b = std::move(a);  // explicit std::move needed
// a is in valid-but-unspecified state
```

**Key difference**: Rust moves are implicit and destructive (source becomes inaccessible). C++ moves are explicit (`std::move`) and the source remains in a valid-but-unspecified state. The transpiler should insert `std::move` wherever Rust does a move, and the borrow-checker guarantees (enforced on the Rust side) ensure the moved-from value is never accessed.

### 1.8 Smart Pointers

The transpiler maps directly to rusty-cpp wrappers (in `include/rusty/`), which mirror Rust's API surface and are analyzable by the rusty-cpp checker. This closes the loop: Rust → transpile → C++ → verify with rusty-cpp.

| Rust | C++ |
|------|-----|
| `Box<T>` | `rusty::Box<T>` |
| `Rc<T>` | `rusty::Rc<T>` |
| `Arc<T>` | `rusty::Arc<T>` |
| `Weak<T>` (from Rc/Arc) | `rusty::Weak<T>` |
| `Cell<T>` | `rusty::Cell<T>` |
| `RefCell<T>` | `rusty::RefCell<T>` |
| `UnsafeCell<T>` | `rusty::UnsafeCell<T>` |
| `MaybeUninit<T>` | `rusty::MaybeUninit<T>` |

### 1.9 Strings

| Rust | C++ |
|------|-----|
| `String` | `rusty::String` |
| `&str` | `rusty::str` / `std::string_view` |

### 1.10 Collections

| Rust | C++ |
|------|-----|
| `Vec<T>` | `rusty::Vec<T>` |
| `HashMap<K,V>` | `rusty::HashMap<K,V>` |
| `BTreeMap<K,V>` | `rusty::BTreeMap<K,V>` |
| `HashSet<T>` | `rusty::HashSet<T>` |
| `BTreeSet<T>` | `rusty::BTreeSet<T>` |
| `VecDeque<T>` | `rusty::VecDeque<T>` |

### 1.11 Error Handling

| Rust | C++ |
|------|-----|
| `Option<T>` | `rusty::Option<T>` |
| `Result<T, E>` | `rusty::Result<T, E>` |
| `panic!()` | `std::abort()` or `throw` (configurable) |
| `unwrap()` | `.unwrap()` |
| `?` operator | See §3.4 |

### 1.12 Concurrency Primitives

| Rust | C++ |
|------|-----|
| `Mutex<T>` | `rusty::Mutex<T>` |
| `RwLock<T>` | `rusty::RwLock<T>` |
| `Condvar` | `rusty::Condvar` |
| `Barrier` | `rusty::Barrier` |
| `Once` | `rusty::Once` |
| `thread::spawn` | `rusty::thread::spawn` |

### 1.13 Function Pointers

| Rust | C++ |
|------|-----|
| `fn(A) -> B` | `rusty::SafeFn<B(A)>` |
| `unsafe fn(A) -> B` | `rusty::UnsafeFn<B(A)>` |
| `Fn(A) -> B` | `std::function<B(A)>` |
| `FnMut(A) -> B` | `std::function<B(A)>` |
| `FnOnce(A) -> B` | `std::move_only_function<B(A)>` (C++23) |

---

## 2. Near-Direct Mappings (Minor Adjustments)

### 2.1 Closures / Lambdas

```rust
let add = |a: i32, b: i32| -> i32 { a + b };
let capture_ref = |x: &i32| *x + 1;    // borrows
let capture_move = move || println!("{}", name);  // moves name in
```

```cpp
auto add = [](int32_t a, int32_t b) -> int32_t { return a + b; };
auto capture_ref = [&x](const int32_t& x) { return x + 1; };  // captures by ref
auto capture_move = [name = std::move(name)]() { std::println("{}", name); };
```

**Mapping rules**:
- Default Rust closures (borrow environment) → `[&]` capture
- `move` closures → `[var = std::move(var), ...]` capture
- `Fn` trait bound → `const` lambda (or `std::function<Sig>`)
- `FnMut` trait bound → mutable lambda
- `FnOnce` trait bound → movable lambda (C++23 `std::move_only_function`)

### 2.2 Tuples

```rust
let pair: (i32, String) = (42, String::from("hello"));
let (a, b) = pair;  // destructuring
```

```cpp
auto pair = std::tuple<int32_t, std::string>(42, std::string("hello"));
auto [a, b] = std::move(pair);  // structured bindings (C++17)
```

### 2.3 Arrays and Slices

| Rust | C++ |
|------|-----|
| `[T; N]` (array) | `std::array<T, N>` |
| `&[T]` (slice) | `std::span<const T>` (C++20) |
| `&mut [T]` | `std::span<T>` |

### 2.4 Type Aliases

```rust
type Result<T> = std::result::Result<T, MyError>;
```

```cpp
template<typename T>
using Result = std::expected<T, MyError>;
```

### 2.5 Modules → C++20 Modules (Recommended) or Namespaces

#### The Header/Source Problem

Traditional C++ requires splitting code into headers (`.h` — declarations) and source files (`.cpp` — definitions). This creates complexity that doesn't exist in Rust:

- Every public type/function needs a declaration in a header and a definition in a source file
- Include guards (`#pragma once` / `#ifndef`) needed to prevent double inclusion
- Circular dependencies between headers require forward declarations
- Templates must be defined in headers (not source files)
- ODR (One Definition Rule) violations are easy to introduce
- Build times suffer from repeated header parsing

Transpiling Rust's single-file modules into header/source pairs would be a significant source of complexity.

#### Solution: C++20 Modules

C++20 modules bypass the header/source split entirely and map almost 1:1 to Rust's module system:

```rust
// src/graphics/mod.rs
pub mod shapes;           // public submodule
mod internal;             // private submodule

pub fn draw() { }
pub(crate) fn helper() { }
fn private_fn() { }
```

```cpp
// graphics.cppm (C++20 module interface unit)
export module graphics;
export import graphics.shapes;   // public submodule re-export
import graphics.internal;        // private submodule (not exported)

export void draw() { }           // pub → export
void helper() { }                // pub(crate) → module-visible, not exported
static void private_fn() { }     // private → static or in anonymous namespace
```

**Mapping table**:

| Rust | C++20 Modules |
|------|---------------|
| `mod foo;` | `import foo;` |
| `pub mod foo;` | `export import foo;` |
| `use crate::foo::Bar;` | `import foo;` (then use `foo::Bar`) |
| `pub fn` | `export void fn()` |
| `fn` (private) | `void fn()` (not exported) |
| `pub(crate) fn` | `void fn()` (module-visible, not exported) |
| `pub(super)` | No direct equivalent (not exported, parent imports) |
| `pub struct` | `export struct` |
| `pub use foo::Bar;` | `export using foo::Bar;` or `export import foo;` |

**Why C++20 modules are ideal for transpilation**:

1. **No header/source split** — one `.cppm` file per Rust module, definitions and declarations together
2. **`export` = `pub`** — direct visibility mapping
3. **No include guards** — modules are imported, not textually included
4. **No circular dependency issues** — module imports are not textual
5. **No ODR problems** — each entity has exactly one owning module
6. **Templates work** — template definitions live in the module interface, no header needed
7. **Faster builds** — modules are compiled once, not re-parsed per translation unit

**Crate → module mapping**:

```
my_crate/                    →  my_crate.cppm (primary module interface)
├── src/lib.rs               →  export module my_crate;
├── src/foo.rs               →  my_crate.foo.cppm
├── src/bar/mod.rs           →  my_crate.bar.cppm
└── src/bar/baz.rs           →  my_crate.bar.baz.cppm
```

**Compiler support (as of 2026)**: GCC 14+, Clang 17+, and MSVC 19.34+ all support C++20 modules. CMake 3.28+ has `import std` support. Module support is production-ready for new projects.

#### Practical Module Emission Rules (from Real-Crate Parity)

The direct mapping above is necessary but not sufficient. Real crates force a few extra invariants.

1. Use a **global module fragment** before `export module` when including headers:

```cpp
module;
#include <variant>
#include <tuple>
#include <utility>
#include <rusty/rusty.hpp>

export module either;
```

Without this shape, standard library declarations can conflict with named-module rules on some toolchains.

2. Only emit `export` at top-level module scope.

```cpp
// Wrong (invalid C++20 module syntax)
namespace inner {
    export struct Foo { int x; };
}

// Correct
namespace inner {
    struct Foo { int x; };
}
```

3. Treat module-linkage-sensitive re-exports as Rust-only comments in module mode.

```rust
pub mod iterator { pub struct IterEither<L, R>(L, R); }
pub use iterator::IterEither;
```

```cpp
namespace iterator {
    template<class L, class R>
    struct IterEither { /* ... */ };
}
// Rust-only re-export skipped in module mode: using iterator::IterEither;
```

4. Keep forward declarations constrained and alias-safe.

```cpp
// Good: simple forward declaration for declaration-order resilience
void extend_panic();

// Avoid broad alias-dependent forward declarations that can break order:
// Option3 foo();   // if Option3 is declared later via alias, this is fragile
```

5. Merge `impl` methods into owning type declarations before emitting inline module bodies.
   This avoids invalid free-function fallbacks for methods that should live on the type.

### 2.6 Impl Blocks

Rust splits methods across multiple `impl` blocks. In C++, all methods must be declared in the class body. The transpiler must **merge all `impl` blocks** for a type into a single class definition.

```rust
struct Foo { x: i32 }
impl Foo {
    fn new(x: i32) -> Self { Foo { x } }
}
impl Foo {  // second impl block
    fn get(&self) -> i32 { self.x }
}
```

```cpp
struct Foo {
    int32_t x;
    static Foo new_(int32_t x) { return Foo{x}; }
    int32_t get() const { return x; }
};
```

Practical rule from real expanded crates: method de-duplication must be keyed by **emitted C++ signature**, not raw Rust impl tokens. Different Rust impl bounds can collapse to the same C++ signature after path/trait lowering.

---

## 3. Non-Trivial Mappings (Require Design Decisions)

### 3.1 Enums with Data (Algebraic Data Types) ⚠️

Rust enums are tagged unions with pattern matching. They map to `std::variant` + `std::visit`.

```rust
enum Shape {
    Circle(f64),                    // radius
    Rectangle { w: f64, h: f64 },  // named fields
    None,                           // unit variant
}
```

```cpp
struct Circle { double radius; };
struct Rectangle { double w; double h; };
struct None_ {};

using Shape = std::variant<Circle, Rectangle, None_>;
```

Each variant becomes its own struct, and the enum becomes a `using` alias for `std::variant<...>`. This is type-safe, zero-overhead, and preserves value semantics. For recursive enums (like linked lists or ASTs), use `rusty::Box<T>` for the recursive case.

#### Generic Enum Rule: Always Carry Template Parameters

For generic enums, variant structs and the variant alias must carry the same template parameter list.

```rust
enum Either<L, R> {
    Left(L),
    Right(R),
}
```

```cpp
template<class L, class R>
struct Either_Left { L _0; };

template<class L, class R>
struct Either_Right { R _0; };

template<class L, class R>
using Either = std::variant<Either_Left<L, R>, Either_Right<L, R>>;
```

If you omit `<L, R>` in either the variant struct or alias, template deduction and pattern matching quickly fail downstream.

#### Constructor Lowering Rule: Use Expected Type Context

Rust often writes constructor calls without explicit type args:

```rust
let a: Either<i32, i32> = Left(1);
let mut b: Either<i32, i32> = Left(2);
b = Right(3);
```

C++ often needs specialization at emission sites:

```cpp
Either<int32_t, int32_t> a = Left<int32_t, int32_t>(1);
Either<int32_t, int32_t> b = Left<int32_t, int32_t>(2);
b = Right<int32_t, int32_t>(3);
```

So the transpiler should thread expected-type hints through:

1. typed `let` initializers,
2. assignments to typed locals,
3. return expressions in typed functions,
4. value-producing match arms.

#### `Self::Variant` and Qualified Variant Paths

Patterns and constructor paths may appear as `Self::Left`, `crate::Left`, `self::Right`, `super::Left`.
Lowering should normalize to the owning enum context before emitting C++ variant types/constructors.

```rust
impl<L, R> Either<L, R> {
    fn flip(self) -> Either<R, L> {
        match self {
            Self::Left(l) => Either::Right(l),
            Self::Right(r) => Either::Left(r),
        }
    }
}
```

```cpp
template<class L, class R>
Either<R, L> flip(Either<L, R> self) {
    return std::visit(overloaded{
        [&](const Either_Left<L, R>& _v) -> Either<R, L> {
            return Either<R, L>(Right<R, L>(_v._0));
        },
        [&](const Either_Right<L, R>& _v) -> Either<R, L> {
            return Either<R, L>(Left<R, L>(_v._0));
        },
    }, self);
}
```

### 3.2 Traits → Microsoft Proxy ⚠️⚠️

Rust traits map to [Proxy](https://github.com/ngcpp/proxy) facades. Proxy provides non-invasive, value-semantic type erasure — the closest C++ equivalent to Rust's trait system. Types just need the right methods; no inheritance required.

| Rust trait usage | C++ (Proxy) mapping |
|------------------|---------------------|
| `trait Foo { fn bar(&self); }` | `PRO_DEF_MEM_DISPATCH` + `pro::facade_builder` |
| `dyn Trait` | `pro::proxy<Facade>` |
| `Box<dyn Trait>` | `pro::proxy<Facade>` (owns the value) |
| `&dyn Trait` | `pro::proxy_view<Facade>` (non-owning) |
| `impl Trait for Type` | Just add the methods to the struct |
| `T: Trait` (generic bound) | `pro::proxy<Facade>` or concept constraint |
| `T: A + B` (multiple bounds) | Combine conventions in one facade |
| Associated types (`type Item`) | Template type alias in facade |
| Supertraits (`trait B: A`) | Facade inheriting conventions from another facade |
| Operator traits (`Add`, `Index`) | C++ operator overloading (direct) |
| Marker traits (`Send`, `Sync`) | `static_assert` or concept constraints |

#### 3.2.1 Trait Definition and Dynamic Dispatch

```rust
trait Animal {
    fn speak(&self) -> String;
}

fn make_noise(animal: &dyn Animal) {
    println!("{}", animal.speak());
}
```

```cpp
#include <proxy/proxy.h>

// Trait definition → facade
PRO_DEF_MEM_DISPATCH(MemSpeak, speak);

struct AnimalFacade : pro::facade_builder
    ::add_convention<MemSpeak, std::string() const>
    ::build {};

// &dyn Animal → pro::proxy_view
void make_noise(pro::proxy_view<AnimalFacade> animal) {
    std::println("{}", animal.invoke<MemSpeak>());
}

// Any type with speak() satisfies the trait — no inheritance!
struct Dog {
    std::string speak() const { return "Woof"; }
};

void example() {
    Dog dog;
    make_noise(pro::make_proxy_view<AnimalFacade>(dog));  // &dyn Animal

    auto boxed = pro::make_proxy<AnimalFacade>(Dog{});    // Box<dyn Animal>
    make_noise(boxed);
}
```

**Why Proxy is the right choice**:
- **Non-invasive**: Types don't need to inherit anything — just have the right methods (exactly like Rust traits)
- **Value semantics**: `pro::proxy` owns the value, like `Box<dyn Trait>`
- **Small buffer optimization**: Avoids heap allocation for small types
- **No vtable inheritance**: Dispatch tables are per-facade, not per-type
- **C++20 compatible**: Works with standard compilers (GCC 14+, Clang 17+, MSVC 19.34+)

#### 3.2.2 Trait Implementation

```rust
impl Animal for Dog {
    fn speak(&self) -> String { "Woof".to_string() }
}
impl Animal for Cat {
    fn speak(&self) -> String { "Meow".to_string() }
}
```

```cpp
// Just define the methods — Proxy resolves them automatically
struct Dog {
    std::string speak() const { return "Woof"; }
};
struct Cat {
    std::string speak() const { return "Meow"; }
};
// Both Dog and Cat automatically satisfy AnimalFacade
```

#### 3.2.3 Multiple Trait Bounds

```rust
fn process(item: &(dyn Display + Debug)) { ... }
```

```cpp
PRO_DEF_MEM_DISPATCH(MemDisplay, display);
PRO_DEF_MEM_DISPATCH(MemDebug, debug);

struct DisplayDebugFacade : pro::facade_builder
    ::add_convention<MemDisplay, std::string() const>
    ::add_convention<MemDebug, std::string() const>
    ::build {};

void process(pro::proxy_view<DisplayDebugFacade> item) { ... }
```

#### 3.2.4 Default Trait Methods

```rust
trait Greet {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        format!("Hello, {}!", self.name())
    }
}
```

```cpp
PRO_DEF_MEM_DISPATCH(MemName, name);

struct GreetFacade : pro::facade_builder
    ::add_convention<MemName, std::string() const>
    ::build {};

// Default method as free function
std::string greet(pro::proxy_view<GreetFacade> self) {
    return std::format("Hello, {}!", self.invoke<MemName>());
}
```

#### 3.2.5 Operator Traits

Rust's operator traits (`Add`, `Sub`, `Index`, `Deref`, etc.) map directly to C++ operator overloading — no Proxy needed:

```rust
impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}
```

```cpp
Point operator+(Point lhs, const Point& rhs) {
    return Point{ lhs.x + rhs.x, lhs.y + rhs.y };
}
```

#### 3.2.6 UFCS in Rust and C++ Lowering

**UFCS (Universal Function Call Syntax)** in Rust is the `Trait::method(receiver, ...)` form.
It is common in expanded macro output and trait-heavy code.

```rust
use std::io::Read;

fn fill(cursor: &mut Cursor<&[u8]>, buf: &mut [u8]) -> usize {
    Read::read(cursor, buf).unwrap()
}
```

Textbook lowering rule:

1. Detect call shape `TraitPath::method(&receiver, args...)` or `TraitPath::method(&mut receiver, args...)`.
2. Rewrite to receiver-call form: `receiver.method(args...)`.
3. Normalize non-receiver UFCS arguments in method position (`&arg`/`&mut arg` → `arg`) only for this rewrite.
4. Guard against false positives:
   - do not rewrite free functions that just happen to be namespaced,
   - do not rewrite constructor-like calls such as `Type::new(...)`.
5. Normalize receiver category before emitting call syntax:
   - value/reference receiver -> `obj.method(...)`
   - pointer receiver -> `ptr->method(...)`
   - raw pointer helper cases -> lower to runtime helper (`rusty::ptr_*`) instead of member call.

```cpp
size_t fill(Cursor<span<const uint8_t>>& cursor, span<uint8_t> buf) {
    return cursor.read(buf).unwrap();
}
```

Receiver-shape sanity example:

```rust
std::ptr::add(ptr, 0).write(x);
```

```cpp
// avoid invalid pointer-member call emission
rusty::ptr_write(rusty::ptr_add(ptr, 0), x);
```

#### 3.2.7 Extension-Trait Method Lowering

Rust extension traits often appear as method calls on types that do not physically declare those methods.
In C++, this is typically represented via free-function lowering.

```rust
use tap::Tap;

fn f(x: i32) -> i32 {
    x.tap(|v| println!("{v}"))
}
```

```cpp
int32_t f(int32_t x) {
    return tap(x, [&](auto v) { std::println("{}", v); });
}
```

This keeps behavior without requiring intrusive class modification.

#### 3.2.8 Module-Mode Trait Facade Fallback

In module-expanded output, some external trait facade symbols may be unavailable.
When that happens, emitting unresolved `pro::proxy<...Facade>` symbols is worse than softening.

Practical fallback policy:

1. keep local trait/facade lowering where symbols are known and emitted,
2. soften unresolved/external trait-only surfaces to placeholder-safe forms in module mode,
3. emit explicit Rust-only comments when a trait item is intentionally not materialized.

### 3.3 Pattern Matching ⚠️

```rust
match shape {
    Shape::Circle(r) => println!("Circle with radius {}", r),
    Shape::Rectangle { w, h } => println!("Rect {}x{}", w, h),
    Shape::None => println!("Nothing"),
}
```

#### Using `std::visit`:

```cpp
std::visit(overloaded{
    [](const Circle& c) { std::println("Circle with radius {}", c.radius); },
    [](const Rectangle& r) { std::println("Rect {}x{}", r.w, r.h); },
    [](const None_&) { std::println("Nothing"); },
}, shape);
```

Where `overloaded` is the standard helper:
```cpp
template<class... Ts> struct overloaded : Ts... { using Ts::operator()...; };
```

#### Pattern matching on other types:

| Rust pattern | C++ equivalent |
|-------------|----------------|
| `match x { 1 => ..., 2 => ... }` | `switch (x) { case 1: ...; case 2: ...; }` |
| `if let Some(v) = opt` | `if (opt.has_value()) { auto v = *opt; ... }` |
| `let (a, b) = tuple` | `auto [a, b] = tuple;` |
| `let Point { x, y } = p` | `auto [x, y] = p;` (needs structured bindings) |
| Guard: `x if x > 0` | if-else chain |

**Note**: C++26 proposes `inspect` for proper pattern matching (P2688). If targeting future standards, this becomes much cleaner.

#### Match-as-Expression vs Match-as-Statement

Rust uses `match` in both statement and expression positions:

```rust
let v = match e {
    Either::Left(x) => x + 1,
    Either::Right(y) => y - 1,
};
```

A robust C++ lowering for value position is an IIFE:

```cpp
auto v = [&]() -> int32_t {
    return std::visit(overloaded{
        [&](const Either_Left<int32_t, int32_t>& _v) -> int32_t { return _v._0 + 1; },
        [&](const Either_Right<int32_t, int32_t>& _v) -> int32_t { return _v._0 - 1; },
    }, e);
}();
```

This avoids fallthrough/missing-return issues and gives each arm an explicit return type.

#### Nested Pattern Binding Rule

Nested tuple/struct patterns should emit explicit binding statements rather than relying on ad-hoc lambda parameter shapes.

```rust
match value {
    Either::Left((a, b)) => a + b,
    Either::Right((c, d)) => c - d,
}
```

```cpp
std::visit(overloaded{
    [&](const Either_Left<std::tuple<int32_t, int32_t>, std::tuple<int32_t, int32_t>>& _v) {
        auto&& _t = _v._0;
        auto a = std::get<0>(_t);
        auto b = std::get<1>(_t);
        return a + b;
    },
    [&](const Either_Right<std::tuple<int32_t, int32_t>, std::tuple<int32_t, int32_t>>& _v) {
        auto&& _t = _v._0;
        auto c = std::get<0>(_t);
        auto d = std::get<1>(_t);
        return c - d;
    },
}, value);
```

#### `Result`/`Option` Try-Style Match Lowering

Certain match shapes are semantically try-like and clearer as control-flow lowering:

```rust
let x = match res {
    Ok(v) => v,
    Err(e) => return Err(e),
};
```

```cpp
if (res.is_err()) {
    return rusty::Result<T, E>::Err(res.unwrap_err());
}
auto x = res.unwrap();
```

This strategy removes many malformed generated shapes (`return return`, invalid variant constructor context, etc.).

### 3.4 The `?` Operator ⚠️

```rust
fn read_file(path: &str) -> Result<String, io::Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.to_uppercase())
}
```

No direct C++ equivalent. Options:

#### Option A: Macro-based (Recommended)

```cpp
#define TRY(expr) \
    ({ auto _r = (expr); \
       if (!_r.has_value()) return std::unexpected(_r.error()); \
       std::move(_r.value()); })

// Usage:
std::expected<std::string, std::io::Error> read_file(std::string_view path) {
    auto content = TRY(fs::read_to_string(path));
    return to_uppercase(content);
}
```

Note: This uses GCC/Clang statement expressions. For portable code, use a different pattern.

#### Option B: Monadic chaining (C++23)

```cpp
std::expected<std::string, Error> read_file(std::string_view path) {
    return fs::read_to_string(path)
        .transform([](auto& s) { return to_uppercase(s); });
}
```

#### Option C: Exceptions (if panic strategy = exceptions)

Map `Result` to exceptions: `Err(e)` throws, `Ok(v)` returns `v`. The `?` operator is implicit. This is the simplest transpilation but changes error handling semantics.

#### Practical `?` Lowering Matrix Used by the Transpiler

In practice, one macro is not enough. The emitted form must depend on sync/async context and `Result` vs `Option` return semantics.

| Rust context | Emitted family |
|---|---|
| sync function returning `Result<..., E>` | `RUSTY_TRY(expr)` |
| sync function returning `Option<T>` | `RUSTY_TRY_OPT(expr)` |
| async function returning `Result<..., E>` | `RUSTY_CO_TRY(expr)` |
| async function returning `Option<T>` | `RUSTY_CO_TRY_OPT(expr)` |

```rust
fn next_token(it: &mut Iter) -> Option<Token> {
    let t = it.next()?;
    Some(t.normalize())
}
```

```cpp
rusty::Option<Token> next_token(Iter& it) {
    auto t = RUSTY_TRY_OPT(it.next());
    return rusty::Option<Token>::Some(t.normalize());
}
```

```rust
async fn fetch_len(c: &Client) -> Result<usize, Error> {
    let body = c.get().await?;
    Ok(body.len())
}
```

```cpp
rusty::Task<rusty::Result<size_t, Error>> fetch_len(const Client& c) {
    auto body = RUSTY_CO_TRY(co_await c.get());
    co_return rusty::Result<size_t, Error>::ok(body.len());
}
```

### 3.5 `break` with Value from `loop`

```rust
let result = loop {
    if condition {
        break 42;
    }
};
```

```cpp
auto result = [&]() -> int32_t {
    while (true) {
        if (condition) {
            return 42;
        }
    }
}();
```

Wrap the loop in an immediately-invoked lambda that uses `return` instead of `break value`.

#### `while let` Lowering Pattern

Rust:

```rust
while let Some(x) = iter.next() {
    consume(x);
}
```

C++ lowering that preserves semantics without bool-context traps:

```cpp
while (true) {
    auto _whilelet = iter.next();
    if (!_whilelet.is_some()) {
        break;
    }
    auto x = _whilelet.unwrap();
    consume(x);
}
```

This shape avoids invalid codegen such as using non-bool sentinel paths directly as loop conditions.

### 3.6 Lifetimes

Rust lifetimes have **no runtime representation** — they are purely compile-time constraints. In the C++ output, lifetimes are simply erased. The safety guarantees were already enforced by the Rust compiler.

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

```cpp
// Lifetimes erased — the Rust compiler already verified correctness
std::string_view longest(std::string_view x, std::string_view y) {
    return x.size() > y.size() ? x : y;
}
```

If the transpiled C++ is then fed back into rusty-cpp for analysis, lifetime annotations can be emitted as comments:
```cpp
// @lifetime: (&'a, &'a) -> &'a
std::string_view longest(std::string_view x, std::string_view y);
```

### 3.7 Unsafe Code → rusty-cpp @unsafe Annotations

Rust's `unsafe` marks code regions where the programmer takes responsibility for safety invariants. In the transpiled C++, we preserve these boundaries using rusty-cpp's `@unsafe` annotation system, closing the loop for analyzer verification.

#### Unsafe Blocks

```rust
fn safe_wrapper() {
    let x = 42;
    unsafe {
        let ptr = &x as *const i32;
        let val = *ptr;
    }
}
```

```cpp
void safe_wrapper() {
    const auto x = 42;
    // @unsafe
    {
        const auto ptr = static_cast<const int32_t*>(&x);
        const auto val = *ptr;
    }
}
```

The `unsafe { }` block becomes a `// @unsafe` annotated block. The rusty-cpp analyzer will skip safety checks inside `@unsafe` blocks, matching Rust's semantics.

#### Unsafe Functions

```rust
unsafe fn dangerous(ptr: *mut i32) {
    *ptr = 42;
}
```

```cpp
// @unsafe
void dangerous(int32_t* ptr) {
    *ptr = 42;
}
```

`unsafe fn` becomes a `// @unsafe` annotated function. In rusty-cpp's two-state model, calling this function from `@safe` code requires an `@unsafe { }` block.

#### Raw Pointers

Raw pointer operations pass through naturally — C++ pointers are inherently unsafe:

| Rust | C++ |
|------|-----|
| `*const T` | `const T*` |
| `*mut T` | `T*` |
| `ptr as *const T` | `static_cast<const T*>(ptr)` |
| `*ptr` (deref) | `*ptr` |
| `&x as *const T` | `static_cast<const T*>(&x)` |

Practical pointer-method lowering is also needed for real crates:

```rust
ptr.add(i).write(value);
std::ptr::copy_nonoverlapping(src, dst, n);
```

```cpp
rusty::ptr_write(rusty::ptr_add(ptr, i), value);
rusty::ptr_copy_nonoverlapping(src, dst, n);
```

This keeps generated C++ valid when Rust raw-pointer method shapes do not map directly to well-typed C++ member calls.

#### Design Decision

The transpiler emits `// @unsafe` (not just `// unsafe`) so that the rusty-cpp analyzer can enforce safety boundaries on the transpiled output. This means:

1. Transpiled safe code is checked by the analyzer (borrow rules, pointer safety)
2. Transpiled unsafe blocks are skipped by the analyzer (programmer responsibility)
3. The safety boundary is preserved across the transpilation — Rust's `unsafe` maps exactly to rusty-cpp's `@unsafe`

This is consistent with the forward correctness guarantee: if Rust's borrow checker approved the safe code, the transpiled C++ should also pass the rusty-cpp analyzer's checks.

### 3.8 Async/Await → Pollable State Machine on C++20 Coroutines

Rust's async model is a lazy, poll-based state machine. C++20 coroutines provide the state machine generation, but default to eager execution. The transpiler builds Rust's poll model on top of C++20 coroutines by customizing the `promise_type`.

#### Core Types

```cpp
#include <coroutine>
#include <functional>

// Poll<T> — Rust's Poll enum
template<typename T>
struct Poll {
    rusty::Option<T> value;  // None = Pending, Some = Ready

    static Poll ready(T v) { return Poll{rusty::Option<T>::some(std::move(v))}; }
    static Poll pending()  { return Poll{rusty::Option<T>::none()}; }
    bool is_ready() const  { return value.is_some(); }
};

// Waker — notification callback for IO readiness
struct Waker {
    std::function<void()> wake_fn;
    void wake() { if (wake_fn) wake_fn(); }
};

struct Context {
    Waker* waker;
};
```

#### Task<T> — The Lazy Coroutine Future

```cpp
template<typename T>
class Task {
public:
    struct promise_type {
        rusty::Option<T> result;
        Context* current_ctx = nullptr;

        Task get_return_object() {
            return Task{std::coroutine_handle<promise_type>::from_promise(*this)};
        }

        // KEY: suspend_always makes it LAZY — nothing runs until poll()
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }

        void return_value(T value) {
            result = rusty::Option<T>::some(std::move(value));
        }
        void unhandled_exception() { std::terminate(); }
    };

    // poll() — drives the state machine one step (like Rust's Future::poll)
    Poll<T> poll(Context& cx) {
        if (handle_.done()) {
            return Poll<T>::ready(std::move(*handle_.promise().result));
        }
        handle_.promise().current_ctx = &cx;
        handle_.resume();  // runs until next co_await or co_return
        if (handle_.done()) {
            return Poll<T>::ready(std::move(*handle_.promise().result));
        }
        return Poll<T>::pending();
    }

    ~Task() { if (handle_) handle_.destroy(); }
    Task(Task&& o) : handle_(std::exchange(o.handle_, nullptr)) {}

private:
    Task(std::coroutine_handle<promise_type> h) : handle_(h) {}
    std::coroutine_handle<promise_type> handle_;
};
```

#### Executor — The Event Loop (like tokio)

```cpp
class Executor {
    std::vector<std::function<Poll<void>(Context&)>> tasks;
    std::queue<size_t> ready_queue;

public:
    void spawn(Task<void> task) {
        tasks.push_back([t = std::move(task)](Context& cx) mutable {
            return t.poll(cx);
        });
        ready_queue.push(tasks.size() - 1);
    }

    void run() {
        while (!ready_queue.empty()) {
            auto idx = ready_queue.front();
            ready_queue.pop();

            Waker waker{[this, idx]() { ready_queue.push(idx); }};
            Context cx{&waker};

            auto result = tasks[idx](cx);
            // Pending → waker will re-enqueue when IO fires
            // Ready → task is done
        }
    }
};
```

#### Transpilation Example

```rust
async fn fetch(url: &str) -> Result<String, Error> {
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}
```

```cpp
Task<rusty::Result<rusty::String, Error>> fetch(std::string_view url) {
    auto response = co_await client.get(url).send();
    if (response.is_err()) co_return rusty::Result<rusty::String, Error>::err(response.unwrap_err());
    auto body = co_await response.unwrap().text();
    if (body.is_err()) co_return rusty::Result<rusty::String, Error>::err(body.unwrap_err());
    co_return rusty::Result<rusty::String, Error>::ok(body.unwrap());
}
```

#### How It Works

```
┌──────────────────────────────────────────────────┐
│  Executor                                        │
│                                                  │
│  loop:                                           │
│    pick task from ready_queue                    │
│    call task.poll(context)                       │
│      │                                           │
│      ├─ Ready(T) → task done                     │
│      │                                           │
│      └─ Pending → waker will re-enqueue          │
│           │        when IO/timer fires           │
│           ▼                                      │
│    ┌─────────────────┐                           │
│    │ C++20 coroutine │ (compiler-generated        │
│    │                 │  state machine)            │
│    │ co_await inner  │──► inner.poll(cx)          │
│    │   Pending?      │   suspend, return Pending  │
│    │   Ready?        │   continue to next state   │
│    │                 │                           │
│    │ co_return val   │──► return Ready(val)        │
│    └─────────────────┘                           │
└──────────────────────────────────────────────────┘
```

**Key design decisions**:
- **`initial_suspend()` → `suspend_always`** — makes coroutines lazy, matching Rust semantics
- **`poll()` wraps `handle_.resume()`** — one step of the state machine per call
- **`Waker`** — callback mechanism so IO subsystems can notify the executor
- **C++20 generates the state machine** — `co_await`/`co_return` map directly to Rust's `.await`
- **Executor is a library, not language** — same as Rust (tokio is a library too)

### 3.9 Derive Macros

```rust
#[derive(Debug, Clone, PartialEq, Hash)]
struct Point { x: f64, y: f64 }
```

| Derive | C++ equivalent |
|--------|---------------|
| `Debug` | `operator<<` or `std::format` specialization |
| `Clone` | Copy constructor (+ explicit `.clone()` method) |
| `Copy` | Trivially copyable (default for POD types) |
| `PartialEq` / `Eq` | `operator==` (C++20: `= default`) |
| `PartialOrd` / `Ord` | `operator<=>` (C++20: `= default`) |
| `Hash` | `std::hash<T>` specialization |
| `Default` | Default constructor |
| `Serialize` / `Deserialize` | External lib (nlohmann/json, etc.) |

```cpp
struct Point {
    double x;
    double y;

    auto operator<=>(const Point&) const = default;  // gives ==, <, >, etc.
    // Debug: implement format or operator<<
    friend std::ostream& operator<<(std::ostream& os, const Point& p) {
        return os << "Point { x: " << p.x << ", y: " << p.y << " }";
    }
};

template<>
struct std::hash<Point> {
    size_t operator()(const Point& p) const {
        return std::hash<double>{}(p.x) ^ (std::hash<double>{}(p.y) << 1);
    }
};
```

### 3.10 Procedural / Declarative Macros

Rust macros operate on token trees and are Turing-complete. There is no general C++ equivalent.

| Macro type | Strategy |
|-----------|----------|
| Simple `macro_rules!` (text substitution) | C preprocessor macros |
| Complex `macro_rules!` (pattern matching) | Expand at transpile time |
| Procedural macros (derive, attribute) | Generate code at transpile time |
| `println!`, `format!` | `std::println`, `std::format` (C++23) |
| `vec![1, 2, 3]` | `std::vector<int>{1, 2, 3}` (initializer list) |

**Recommendation**: Expand all macros before transpilation (using `rustc`'s macro expansion output or `cargo expand`), then transpile the expanded code.

### 3.11 Generics / Templates

```rust
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

```cpp
template<std::totally_ordered T>
T max(T a, T b) {
    return a > b ? std::move(a) : std::move(b);
}
```

| Rust | C++ |
|------|-----|
| `<T>` | `template<typename T>` |
| `T: Bound` | `Concept T` or `requires` clause |
| `T: A + B` | `requires (A<T> && B<T>)` |
| `where T: Bound` | `requires` clause |
| `impl<T> Struct<T>` | Template class method definitions |
| `T: 'static` | No equivalent (lifetime erased) |
| Monomorphization | Same — C++ templates are monomorphized |

### 3.12 Name and Path Rewriting Cookbook

Real crates rely on many Rust path families that are not valid C++ namespaces.
A practical transpiler needs explicit rewriting rules instead of ad-hoc fallback text.

#### 3.12.1 Import Rewriting

```rust
use std::io::{self, Read, SeekFrom};
use core::cmp::Ordering;
use alloc::collections::BTreeMap;
```

```cpp
namespace io = rusty::io;                 // keep `io::...` call sites valid
// Rust-only: using std::io::Read;        // trait import, no C++ symbol emitted
using rusty::io::SeekFrom;
using rusty::cmp::Ordering;
using rusty::BTreeMap;
```

Rules:

1. preserve runtime-relevant symbols as C++-valid mappings,
2. skip trait-only imports as Rust-only comments,
3. rewrite `core::`/`alloc::` consistently (not piecemeal),
4. never emit unresolved `using std::foo::Bar` just because Rust path exists.

#### 3.12.2 Runtime Path Lowering

Representative examples:

| Rust path | Lowered path |
|---|---|
| `core::intrinsics::unreachable` | `rusty::intrinsics::unreachable` |
| `core::panicking::panic_fmt` | `rusty::panicking::panic_fmt` |
| `std::str::Utf8Error` | `rusty::str_runtime::Utf8Error` |
| `std::char::from_u32` | `rusty::char_runtime::from_u32` |
| `std::io::Result<T>` | `rusty::io::Result<T>` |

The lowering table should be centralized; do not distribute these rewrites across unrelated emit paths.

#### 3.12.3 Associated/Omitted-Template Recovery

Rust often omits template args where type context is obvious:

```rust
let m = MaybeUninit::uninit();
let e = CapacityError::new(());  // local alias with default parameter
```

C++ emission must recover owner/template context:

```cpp
auto m = rusty::MaybeUninit<T>::uninit();
auto e = CapacityError<>::new_(());
```

Without this recovery, codegen drifts into unresolved `Type::member` or wrong-template diagnostics.

The same rule applies to owner constructors on generic runtime types:

```rust
let s = ArrayString::new();
let m = HashMap::new();
```

```cpp
// owner args must be recovered from expected/local context
auto s = ArrayString<32>::new_();
auto m = rusty::HashMap<K, V>::new_();
```

Canonical constraint:

- recover omitted owner args through expected-type/scope inference,
- do not apply blanket global rewrites that force template args where they are not required.

### 3.13 Transparent C++ Module Imports as Rust Modules (`use cpp::...`) ⚠️

Because the transpiler target is module-based C++, C++ modules should be imported in Rust grammar as if they were Rust modules.

This is **not** C ABI FFI (`extern "C"`). This is direct source-level C++ module interop resolved by the C++ compiler.

For this profile, the design is **no-bridge by default**: do not generate typed wrapper layers; emit direct C++ calls and let the C++ compiler resolve overloads/conversions.

#### Rust Surface (Proposed)

```rust
use cpp::std as cpp_std;

unsafe {
    let hi: i32 = 20;
    let lo: i32 = 10;
    let m: i32 = cpp_std::max(lo, hi);
}
```

Rules:

1. `cpp::` is a reserved import root for foreign C++ modules.
2. `use cpp::a::b` is treated as importing C++ module path `a.b`.
3. No Rust-side `extern "C++"` declaration blocks or `#[cpp(...)]` attributes for this path.
4. Name remapping is handled on the C++ side (module-exported aliases/shims), not by Rust attributes.
5. Alias imported C++ modules when helpful (for example `use cpp::std as cpp_std`) to keep Rust and C++ module intent explicit.

#### Module and Symbol Resolution

The transpiler resolves `use cpp::...` imports through a C++ module symbol index produced from module interface units.

The index must provide enough metadata to validate symbol existence and emit calls:

- module path to C++ namespace mapping,
- exported function names/callable sets,
- callable type shapes needed by emission diagnostics.

MVP sidecar format (`version = 1`) is supported in both JSON and TOML with this shape:

- `modules.<module_path>.namespace` (optional)
- `modules.<module_path>.symbols.<symbol_name>.kind` (optional)
- `modules.<module_path>.symbols.<symbol_name>.callable_signatures[]` (optional)

Example TOML:

```toml
version = 1

[modules.std]
namespace = "std"

[modules.std.symbols.max]
kind = "function"
callable_signatures = ["int(int,int)"]
```

CLI configuration:

- pass one or more `--cpp-module-index <path>` flags in single-file, crate, or parity flows.
- index files are merged deterministically; conflicting duplicate module/symbol definitions are rejected.
- when `use cpp::...` imports are present and no non-empty index is configured, transpilation fails immediately.

If a `cpp::` import or referenced symbol cannot be resolved, transpilation fails with an explicit import/symbol error.

#### MVP Support Limits (Enforced)

Current enforced MVP surface is intentionally narrow:

- supported:
  - free/static function calls through imported bindings (`binding::symbol(...)`),
  - module constants in value position (`binding::CONSTANT`).
- unsupported (fail-fast with explicit `TODO(leaf22.7)` diagnostics):
  - member-function import syntax (`binding::Type::method(...)` and other multi-segment member-like paths),
  - template-only exports without indexed callable signatures (no resolvable call shape),
  - macro imports/usage through `cpp::` bindings (`binding::name!(...)`).

Non-call function symbol usage in value position (for example function-pointer-like usage) is also rejected in MVP mode; only module constants are allowed in non-call positions.

#### Direct-Call Lowering Rule

For each resolved C++ module call:

1. emit C++ module imports for referenced `cpp::` modules,
2. lower Rust values to canonical emitted C++ types (same lowering table used by normal Rust emission),
3. emit direct qualified C++ calls (no generated bridge wrappers),
4. let C++ compiler perform overload resolution/implicit conversion checks and produce final call diagnostics.

#### Borrow/Move Semantics at the Boundary

| Rust signature shape at call site | Emitted C++ argument shape | Boundary behavior |
|---|---|---|
| `T` | `T` | caller passes moved value when Rust semantics require move |
| `&T` | `const T&` | shared borrow |
| `&mut T` | `T&` | exclusive mutable borrow |
| `&str` | `std::string_view` | non-owning view |

For Rust-owned runtime types (`rusty::String`, `rusty::Vec<T>`, `rusty::HashMap<K,V>`, etc.), direct interop is allowed only when the C++ side consumes the same lowered type family.

#### Generated C++ Shape

```cpp
import std;

int hi = 20;
int lo = 10;
auto m = std::max(lo, hi);
```

Transpiled Rust call sites lower directly to target C++ module symbols, preserving existing Rust lowering and ownership rules.

#### Safety Contract

- C++ imported calls are foreign/unsafe by default: callees may violate Rust aliasing/lifetime expectations.
- Calls require `unsafe` context (or explicit Rust-side safe wrapper APIs that document invariants).
- No automatic lifetime extension is introduced by the transpiler.

#### Accepted Tradeoff (No-Bridge Mode)

- overload choice and implicit conversion behavior are delegated to C++ compiler rules,
- diagnostics surface primarily at direct call sites in C++ compile stage,
- behavior can shift when target C++ module exports or visible overload sets change.

#### Rejected Patterns

- no automatic C ABI thunk generation for this surface (that is a separate `extern "C"` path),
- no global text substitution of unresolved `foo::bar` paths into C++ calls,
- no generated bridge wrappers in module-only no-bridge profile,
- no Rust-side attribute-driven symbol remapping for `cpp::` imports.

---

## 4. Semantic Gaps and Challenges

### 4.1 Exhaustiveness Checking

Rust's `match` is exhaustive — the compiler ensures all variants are handled. `std::visit` on `std::variant` provides this at compile time. `switch` on integers does not. The transpiler should prefer `std::visit` for enum matches.

### 4.2 Move Semantics Differences

| Aspect | Rust | C++ |
|--------|------|-----|
| Default | Move | Copy |
| After move | Inaccessible (compile error) | Valid-but-unspecified |
| Implicit move | Yes (last use) | No (need `std::move`) |
| Destructive move | Yes | No (destructor still runs) |

The transpiler must:
1. Insert `std::move()` at every Rust move point
2. Trust that Rust's borrow checker has verified no use-after-move
3. Optionally use `[[clang::trivial_abi]]` for destructive move optimization

### 4.3 Visibility / Access Control

| Rust | C++ |
|------|-----|
| Private (default) | Private (default in class), Public (default in struct) |
| `pub` | `public:` |
| `pub(crate)` | No equivalent (internal linkage? `friend`? anonymous namespace?) |
| `pub(super)` | No equivalent |

**Strategy**: Use `public`/`private` in structs/classes. For module-level visibility, rely on separate compilation units and header organization. `pub(crate)` can be approximated with comments or `[[deprecated("internal")]]`.

### 4.4 Orphan Rule / Coherence

Rust prevents implementing external traits for external types (orphan rule). C++ has no such restriction — you can specialize templates and overload operators for any type. The transpiler doesn't need to enforce this; it's a Rust-side concern.

### 4.5 No Null

Rust has no null — `Option<T>` is used instead. The transpiler maps all `Option` usage to `rusty::Option<T>`, which preserves Rust's semantics (no implicit null, explicit `Some`/`None`, `.unwrap()`, `.map()`, etc.).

### 4.6 Iterators

Rust iterators are lazy, composable, and zero-cost. C++ ranges (C++20) provide similar functionality.

```rust
let sum: i32 = vec.iter()
    .filter(|x| **x > 0)
    .map(|x| x * 2)
    .sum();
```

```cpp
auto sum = vec
    | std::views::filter([](int x) { return x > 0; })
    | std::views::transform([](int x) { return x * 2; })
    | std::ranges::fold_left(0, std::plus{});  // C++23
```

In practice, transpiled crates frequently need an explicit iterator runtime bridge instead of assuming every value is a native C++ range.

#### Iterator Lowering Rules Used in Practice

1. `.iter()` / `.iter_mut()` calls should lower to shared iterator helpers when direct container APIs are unavailable.

```rust
let it = values.iter();
```

```cpp
auto it = rusty::iter(values);
```

2. Rust slice iterator type paths should map to concrete runtime iterator wrappers.

```rust
type I<'a, T> = std::slice::Iter<'a, T>;
```

```cpp
template<class T>
using I = rusty::slice_iter::Iter<T>;
```

3. `.collect()` should be expected-type-aware in generated C++.

```rust
let out: Vec<_> = self.iter().cloned().collect();
```

```cpp
auto out = rusty::Vec<T>::from_iter(self.iter().cloned());
```

4. `for x in expr` should bridge through an iterator adapter for non-range iterator objects.

```rust
for x in iter_obj {
    consume(x);
}
```

```cpp
for (auto&& x : rusty::into_iter_range(iter_obj)) {
    consume(x);
}
```

This avoids hard dependency on `begin/end` members on every lowered iterator type.

5. Adapter chains should lower to shared adapter surfaces when iterator-like receivers do not define Rust-style members.

```rust
iter.by_ref().take(n).map(f).rev().enumerate()
```

```cpp
rusty::enumerate(
    rusty::rev(
        rusty::map(
            rusty::take(iter, n),
            f)));
```

Use helper-chain lowering over direct member calls when the receiver is an adapter value rather than a native C++ range class with those members.

6. `.collect()` must not emit `Target::from_iter(...)` for non-owning view targets.

```rust
let out: &[u8] = iter.into_iter().collect();
```

```cpp
const std::span<const uint8_t> out = rusty::collect_range(rusty::iter(iter));
```

Treat `std::span<...>` / `std::string_view`-family targets as view surfaces and route through `rusty::collect_range(...)` instead of emitting unavailable `view::from_iter(...)` members.

7. Iterator-adapter receiver evidence must include callable-return and qualified-call shapes, but preserve `Option::map` semantics on `next()` payloads.

```rust
Flags::iter(&value).map(|f| f.bits()).collect::<Vec<_>>();
inherent(&value).map(|f| f.bits()).collect::<Vec<_>>();
it.next().map(|x| x + 1); // Option::map, not iterator adapter map
```

```cpp
rusty::collect_range(rusty::map(value.iter(), ...));
rusty::collect_range(rusty::map(inherent(value), ...));
it.next().map(...);
```

This keeps adapter-chain lowering robust for call-return iterator sources while avoiding false-positive rewrites of optional-like map surfaces.

### 4.7 Trait Objects with Multiple Traits

```rust
fn process(item: &(dyn Display + Debug)) { ... }
```

Proxy handles this naturally by combining conventions in a single facade (see §3.2.3):

```cpp
struct DisplayDebugFacade : pro::facade_builder
    ::add_convention<MemDisplay, std::string() const>
    ::add_convention<MemDebug, std::string() const>
    ::build {};
```

### 4.8 `impl Trait` in Return Position (Existential Types)

```rust
fn make_iter() -> impl Iterator<Item = i32> {
    (0..10).filter(|x| x % 2 == 0)
}
```

Both `impl Trait` and `dyn Trait` returns map uniformly to `pro::proxy<Facade>`. This trades static dispatch for a simpler, uniform transpilation rule — one mapping for all trait-typed returns.

```cpp
PRO_DEF_MEM_DISPATCH(MemNext, next);

struct IteratorFacade : pro::facade_builder
    ::add_convention<MemNext, rusty::Option<int32_t>()>
    ::build {};

pro::proxy<IteratorFacade> make_iter() {
    return pro::make_proxy<IteratorFacade>(/* ... */);
}
```

| Rust | C++ |
|------|-----|
| `-> impl Trait` | `-> pro::proxy<Facade>` |
| `-> Box<dyn Trait>` | `-> pro::proxy<Facade>` |
| `-> &dyn Trait` | `-> pro::proxy_view<Facade>` |

In module-expanded builds, when a required external facade symbol is not emitted/available, use a guarded fallback strategy (placeholder-safe type + explicit Rust-only marker) rather than emitting unresolved proxy symbols that break whole-module compilation.

### 4.9 Systematic Translation Workflow

The canonical solutions should live with their language topics (Sections 2-4), not in a detached implementation log.

When a new transpilation failure appears, follow this procedure:

1. Capture the first deterministic hard error.
2. Identify the language feature family involved.
3. Apply the canonical lowering rule from the corresponding section.
4. Add focused fixture-agnostic regression tests.
5. Re-run parity and verify that the deterministic head moved.

Cross-reference map:

| Failure family | Primary section |
|---|---|
| Omitted owner template args (`Type::new_()` arity issues) | §3.12.3 |
| UFCS/receiver-shape call mismatches | §3.2.6 |
| Iterator adapter surface gaps (`rev/enumerate/map/take`) | §4.6 |
| Match expression/return-shape fallout | §3.3 |
| Path/import/runtime namespace mismatches | §3.12.1-§3.12.2 |
| Move/ownership ctor payload mismatches | §4.2 and §3.1 |

Section 10 remains status/frontier tracking only.

---

## 5. Complete Feature Matrix

| Rust Feature | C++ Mapping (rusty-cpp preferred) | Difficulty | Notes |
|-------------|-----------------------------------|------------|-------|
| Primitive types | Fixed-width integers | Easy | Direct mapping |
| `let` / `let mut` | `const auto` / `auto` | Easy | Flip default mutability |
| Functions | Functions | Easy | Add explicit `return` |
| Structs | Structs/classes | Easy | Merge impl blocks |
| Enums (C-like) | `enum class` | Easy | Direct |
| Enums (with data) | `std::variant` | Medium | See §3.1 |
| Traits | Microsoft Proxy facades | Medium | See §3.2 |
| Pattern matching | `std::visit` / switch | Medium | See §3.3 |
| `?` operator | Macro / monadic | Medium | See §3.4 |
| Closures | Lambdas | Easy | Capture mode mapping |
| Generics | Templates + concepts | Medium | Bounds → concepts |
| Lifetimes | Erased | Easy | No runtime effect |
| Ownership/moves | `rusty::move` / `std::move` | Medium | Insert at move points |
| `Box<T>` | `rusty::Box<T>` | Easy | Direct API match |
| `Rc<T>` / `Arc<T>` | `rusty::Rc<T>` / `rusty::Arc<T>` | Easy | Direct API match |
| `Vec<T>` | `rusty::Vec<T>` | Easy | Direct API match |
| `HashMap` / `HashSet` | `rusty::HashMap` / `rusty::HashSet` | Easy | Direct API match |
| `Option<T>` | `rusty::Option<T>` | Easy | Direct API match |
| `Result<T,E>` | `rusty::Result<T,E>` | Easy | Direct API match |
| `String` / `&str` | `rusty::String` / `std::string_view` | Easy | Direct API match |
| `Mutex<T>` / `RwLock<T>` | `rusty::Mutex<T>` / `rusty::RwLock<T>` | Easy | Data-protecting model |
| `Cell<T>` / `RefCell<T>` | `rusty::Cell<T>` / `rusty::RefCell<T>` | Easy | Runtime borrow checks |
| `fn()` / `unsafe fn()` | `rusty::SafeFn` / `rusty::UnsafeFn` | Easy | Safety-typed wrappers |
| `async`/`await` | Coroutines | Hard | No standard executor |
| Macros | Expand before transpile | Medium | Use `cargo expand` |
| Modules | C++20 modules (`.cppm`) | Easy | `pub` → `export`, see §2.5 |
| Derive macros | Code generation | Medium | Per-derive mapping |
| Unsafe blocks | Raw code | Easy | Just emit the code |
| FFI (`extern "C"`) | `extern "C"` | Easy | Direct mapping |
| C++ module interop (`use cpp::...`) | Direct native C++ module calls (no bridge wrappers) | Medium | See §3.13 |

---

## 6. Proposed Architecture

```
                    ┌─────────────────┐
                    │   Rust Source    │
                    │   (.rs files)   │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  cargo expand   │
                    │ (macro expand)  │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   syn / rustc   │
                    │  (parse AST)    │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
     ┌────────▼───────┐ ┌───▼────┐ ┌───────▼──────┐
     │ Type Resolution│ │ Trait  │ │  Lifetime    │
     │ & Inference    │ │ Mapper │ │  Erasure     │
     └────────┬───────┘ └───┬────┘ └───────┬──────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼────────┐
                    │  C++ Code Gen   │
                    │  (emit .cppm)   │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   parity-test   │
                    │  stage A→E      │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  C++20 Output   │
                    │  + artifacts    │
                    └─────────────────┘
```

### Key Components:

1. **Macro Expander**: Use `cargo expand` to flatten all macros before processing
2. **AST Parser**: Use `syn` crate to parse Rust into a typed AST
3. **Type Resolution**: Resolve all types, infer where needed, map Rust types → C++ types
4. **Trait Mapper**: Convert trait definitions to Proxy facades (`PRO_DEF_MEM_DISPATCH` + `pro::facade_builder`)
5. **Lifetime Eraser**: Strip all lifetime annotations (they have no runtime effect)
6. **Code Generator**: Emit idiomatic C++20 module code (`.cppm`) with guarded runtime helpers
7. **Parity Harness**: Execute Stage A-E (baseline, expand/transpile, build-shape checks, compile, run)
8. **Matrix Runner**: Run deterministic first-failure parity across target crate set

---

## 7. Existing Work and Related Projects

| Project | Approach | Status |
|---------|----------|--------|
| **C2Rust** | C → Rust transpiler (inverse direction) | Mature, Mozilla-backed |
| **Crubit** | Google's C++/Rust interop tool | Active, focused on FFI bindings |
| **cxx** | Rust/C++ safe interop bridge | Mature, bidirectional FFI |
| **autocxx** | Automated C++ binding generation | Active |
| **cbindgen** | Rust → C/C++ header generation | Mature, for FFI headers only |

No production-quality **Rust→C++** transpiler exists today. The closest conceptual work is **C2Rust** (which goes the other direction) and academic papers on transpilation between systems languages.

---

## 8. Recommended Strategy

### Phase 1: Core Language (MVP)
- Primitive types, functions, structs, basic enums
- `let`/`let mut` → `const auto`/`auto`
- References → `const T&` / `T&`
- `Vec`, `String`, `HashMap` → STL equivalents
- Simple `match` → `switch` or `std::visit`
- Lifetime erasure

### Phase 2: Traits and Generics
- Trait definitions → C++20 concepts
- All trait usages → Microsoft Proxy facades
- Generic functions → constrained templates
- Derive macros → generated operator overloads

### Phase 3: Advanced Features
- Async/await → C++20 coroutines
- Complex pattern matching → `std::visit` with guards
- Macro expansion via `cargo expand`
- Module system → C++20 modules

### Phase 4: Ecosystem Integration
- Standard library mapping (full `std::` equivalents)
- Build system integration (Cargo.toml → CMakeLists.txt)
- Test transpilation (`#[test]` → Google Test or Catch2)
- Documentation comments (`///` → Doxygen `///`)

### Delivery Discipline (Required)

For each feature family, use the same loop:

1. Add/adjust lowering rule.
2. Add focused regression tests for that lowering.
3. Re-run parity on the first failing target.
4. Advance only when the deterministic first blocker family moves.

This prevents chasing cascaded diagnostics and keeps changes auditable.

### Recommended Operational Commands

Single crate:

```bash
cargo run -p rusty-cpp-transpiler -- parity-test \
  --manifest-path tests/transpile_tests/either/Cargo.toml \
  --stop-after run
```

Matrix:

```bash
tests/transpile_tests/run_parity_matrix.sh
```

---

## 9. Conclusion

Rust-to-C++ transpilation is **feasible** for the vast majority of Rust code. The language constructs map well because both languages share the same computational model (value semantics, deterministic destruction, zero-cost abstractions, monomorphized generics).

The **three hardest problems** are:
1. **Traits → C++**: Mapped uniformly to Microsoft Proxy facades (non-invasive, value-semantic)
2. **Enums with data → `std::variant`**: Works but pattern matching is verbose
3. **The `?` operator**: Requires a macro or monadic chaining

The **easiest wins** are:
- All primitive types, functions, structs, and control flow map directly
- Lifetimes simply disappear (they have no runtime semantics)
- Move semantics map to `std::move` (with the borrow checker guaranteeing safety on the Rust side)
- Smart pointers and collections map directly to rusty-cpp counterparts (`Box`→`rusty::Box`, `Vec`→`rusty::Vec`, etc.)
- Modules map to C++20 modules (`pub` → `export`, `mod` → `import`)

This approach effectively lets Rust serve as a **safe DSL for C++** — write in Rust with full safety guarantees, transpile to C++ for deployment in C++ codebases. The generated C++ can then be analyzed by rusty-cpp to verify that the safety properties are maintained.

---

## 10. Implementation Status and Integrated Design Notes

This section replaces the previous leaf-by-leaf chronological log with a thematic integration of the same work.
The goal is to keep the document aligned to Sections 1-9 (language model and design), while still preserving the practical implementation outcomes from parity work.

Date baseline for this status snapshot: **April 5, 2026**.

### 10.1 Crosswalk: Where the Former Appended Content Now Lives

| Source intent (old post-§9 appendices) | Integrated location in this doc | What is now captured |
|---|---|---|
| Whole-crate transpilation planning and orchestration | §10.2, §10.3 | Current pipeline (`parity-test`), crate/module orchestration, stage model |
| Real-crate gap fixing (`either`, expanded tests) | §10.4.1-§10.4.7, §10.5.1 | Import/path lowering, constructor/type-context recovery, pattern lowering, runtime shims |
| Phase 18/20 leaf progress | §10.4, §10.5, §10.6 | Consolidated by technical theme instead of timeline |
| Seven-crate matrix harness and CI behavior | §10.3.3, §10.5.2, §10.7 | Deterministic first-failure diagnostics, matrix execution contract |
| "Wrong approaches" checklist | §11 | Grouped design constraints tied to architecture decisions |

#### 10.1.1 Former Section-Range Migration Map

| Former range (chronological log) | New integrated location | Primary theme |
|---|---|---|
| old §10.1-§10.6 | §10.2, §10.3 | whole-crate model and execution stages |
| old §10.7-§10.10 | §10.4.1-§10.4.3 | real-crate gap classes (imports, constructors, match lowering) |
| old §10.11-§10.32 | §10.3.2, §10.4.1-§10.4.5 | early Phase 18 blocker collapse and parity harness bootstrap |
| old §10.33-§10.54 | §10.4.2-§10.4.7 | mid/late Phase 18 semantic and emission hardening |
| old §10.11.27-§10.11.52 | §10.4.2-§10.4.6, §10.5.1 | expanded-test runnable parity (constructor/match/io/runtime families) |
| old §10.11.53-§10.11.69 | §10.3.3, §10.5.2, §10.7 | matrix harness infrastructure, deterministic diagnostics, CI wiring |
| old §10.11.70-§10.11.78 | §10.4.1-§10.4.4, §10.5.2 | `tap`/`cfg-if`/`take_mut`/`semver`/`bitflags` deterministic families |
| old §10.11.79-§10.100 | §10.4.6, §10.5.2, §10.6 | `arrayvec` Stage D frontier progression and current active leaf chain |
| old §11.1-§11.96 | §11.1-§11.9 | grouped anti-pattern constraints and enforcement rules |

### 10.2 Whole-Crate Transpilation Model (Current)

The transpiler operates as a source-to-source compiler from Rust crate inputs to C++20 modules with a parity-first validation workflow.

#### 10.2.1 Core Contract

1. Rust is the semantic source of truth.
2. Generated C++ must compile and preserve behavior for supported language/runtime surfaces.
3. Every non-trivial lowering change is verified by targeted tests plus parity reruns.

#### 10.2.2 Current Operational Workflow

```bash
cargo run -p rusty-cpp-transpiler -- parity-test \
  --manifest-path <crate>/Cargo.toml \
  --stop-after run \
  --work-dir <work>
```

`parity-test` stage model (logical):

- Stage A: Rust baseline (`cargo test` or crate-appropriate baseline probe)
- Stage B: Expansion/transpile (`cargo expand` as needed + Rust→C++ emission)
- Stage C: Build graph/materialization checks
- Stage D: C++ compile
- Stage E: C++ run/behavior check

This stage model is used both for single-crate parity and matrix execution.

#### 10.2.3 Whole-Crate Output Shape

- Rust crate/module tree maps to C++20 module units (`*.cppm`)
- crate root maps to `export module <crate>`
- nested modules map to `crate.submodule` module names
- generated build integration is CMake-oriented

### 10.3 Implementation Architecture (Integrated)

The architecture from §6 is implemented with parity-driven hardening in the following areas.

#### 10.3.1 Front-End and Type/Path Mapping

- Expanded support for Rust path families used in real crates:
  - `std::`, `core::`, `alloc::` import/path normalization
  - Rust-only imports converted to comments when no valid C++ symbol exists
- Type/path lowering coverage includes (non-exhaustive):
  - `Option`, `Result`, `Poll`, `Context`
  - `fmt` surfaces (`Formatter`, `Result`, `Error`, `Arguments`, `debug_list`, `write_fmt`)
  - `pin`, `path`, `ffi`, `any`, `cmp`, `str`, `char`, `io`, `collections`, `mem`, `ptr`

#### 10.3.2 Mid-End Lowering

- Expected-type propagation was extended from typed `let` into assignment, return, and match-arm contexts.
- Constructor lowering was hardened for variant constructors, `Some/Ok/Err`, and omitted-template associated calls.
- UFCS and trait-method call rewrites were narrowed to auditable patterns to avoid over-rewrites.

#### 10.3.3 Back-End Emission and Module Safety

- C++ emission now includes deterministic method de-dup keyed by emitted C++ signature, not raw Rust signature text.
- Module-mode export/re-export handling avoids invalid nested exports and linkage-unsafe re-exports.
- Forward-declaration strategy is constrained to avoid alias-order regressions and unresolved dependent signatures.

#### 10.3.4 Runtime Fallback Layer

Fallback helper surfaces were expanded only when needed by emitted code, including:

- `rusty::fmt` helpers (`Formatter` surfaces including `debug_list` and `write_fmt`)
- `rusty::panicking` helpers (`panic_fmt`, assertion helpers)
- `rusty::intrinsics` helpers (`unreachable`, `discriminant_value` paths)
- iterator/range/slice helper surfaces (`iter`, `slice_iter`, collect/range support)
- pointer/memory helper surfaces (`copy`, `copy_nonoverlapping`, pointer `.add/.offset` handling)
- `ManuallyDrop`, `MaybeUninit`, saturating integer helpers

### 10.4 Integrated Technical Outcomes by Topic

This section reorganizes the former phase/leaf appendices by language/design topic.

#### 10.4.1 Imports, Namespaces, and Path Lowering

Integrated outcomes:

- Rust-only imports are no longer emitted as hard C++ `using` declarations.
- `std::io` import family is lowered to runtime-safe aliases/mappings.
- `core::` / `alloc::` paths are normalized with C++-valid mappings.
- `std/alloc::boxed` and `std/alloc::rc` use-import families are lowered to valid runtime/type surfaces (`rusty::Box`, `rusty::boxed::*`, `rusty::Rc`, `rusty::Weak`) or explicit Rust-only import markers where no concrete C++ symbol should be emitted.
- `std/core::hint::*` and `std/core::ops::*` use-import families are now consistently marked Rust-only so expanded outputs do not emit invalid C++ namespace imports (`std::hint`, `std::ops`).
- `std/core::alloc::{LayoutErr, LayoutError}` and `std/core::mem::align_of` import families now lower to concrete `rusty::*` runtime surfaces.
- fragile unresolved `using ::Type` patterns were replaced with guarded lowering.
- module-scope import ordering and alias-safety constraints were added to avoid declaration-order fallout.

Directly supports:

- §2.5 Modules
- §3.2 Traits (import-driven dispatch surfaces)
- §4.3 Visibility and access model

#### 10.4.2 ADTs, Constructors, and Expected-Type Recovery

Integrated outcomes:

- Generic enum variant wrappers and constructor emission are stabilized.
- Constructor calls (`Left/Right`, `Ok/Err`, `Some`) now use expected-type context where needed.
- nested/qualified constructor paths (`crate::`, `self::`, `super::`) are lowered consistently.
- untyped local initialization/reassignment around variant constructors now avoids deducing wrong concrete variant struct types.
- generic function-call argument expected-type recovery now specializes declared argument types with call-site template substitutions (explicit turbofish and inferred fn-path substitutions), preserving associated-type payload coercions.
- tuple/option constructor coercion now uses typed constructor forms only in associated-type contexts that require it (`std::tuple<...>{...}` / `std::make_optional<...>(...)`); default emission remains `std::make_tuple(...)` / untyped `std::make_optional(...)` outside those contexts.
- owner-template recovery now handles `SmallVec` explicit and omitted owner forms with nested infer placeholders (`[_; N]`-style), expected-type hints, and local usage hints so constructor/call lowering does not leak omitted-template C++ forms.

Directly supports:

- §3.1 Enums with data
- §3.3 Pattern matching
- §4.2 Move and expression-shape differences

#### 10.4.3 Pattern Matching and Visitor Lowering

Integrated outcomes:

- `std::visit(overloaded{...})` helper emission is deterministic and scoped.
- match-as-expression and match-as-statement lowering is return-context aware.
- tuple and nested tuple pattern binding emission is recursive and explicit.
- `Result`/`Option` pattern shapes use runtime conditional lowering where variant-visit lowering is not valid.
- malformed `return return` and missing-return families were removed via context-aware emission.

Directly supports:

- §3.3 Pattern matching
- §3.5 `break` with value / expression contexts
- §4.1 Exhaustiveness-related codegen safety

#### 10.4.4 Traits, UFCS, and Method-Shape Normalization

Integrated outcomes:

- UFCS detection/rewrite is implemented for supported call shapes with strict guards.
- non-receiver arg normalization for UFCS-rewritten calls is scoped (no global reference stripping).
- module-mode trait facade/proxy emissions are guarded where facade symbols are unavailable.
- extension-trait and blanket-impl call-shape lowering is handled through explicit rewrites, not text patching.

Directly supports:

- §3.2 Traits
- §4.7 Multi-trait object behavior
- §4.8 `impl Trait` return handling in practical codegen

#### 10.4.5 Control Flow, Return Shapes, and Destructors

Integrated outcomes:

- return emission now depends on scope return requirements.
- `while let` and related control-flow lowering avoids invalid bool-context expressions.
- destructor-tail expression emission no longer produces invalid value-return statements in `Drop`/destructor-like contexts.
- closure-body return-context handling was hardened to avoid regressions.
- expression-block IIFE lowering now reuses shared statement/local emission paths (`emit_stmt`/`emit_local`) so local shadowing semantics in `{ let x = x; ... }` value-position blocks stay aligned with normal block lowering.
- `if`-expression IIFE branch-tail lowering now propagates expected type through block-tail statement emission, so constructor/associated-call specialization remains context-correct inside typed branch expressions.

Directly supports:

- §1.4 Control flow
- §3.5 loop/break-value adaptation
- §3.7 unsafe/control-flow edge behavior

#### 10.4.6 Iterator, Slice, Range, and IO Surfaces

Integrated outcomes:

- `.iter()` / `.iter_mut()` lowering is mapped to shared runtime iterator helpers.
- `slice::Iter` / `slice::IterMut` type paths map to runtime iterator wrappers.
- iterator adapter type-path surfaces now normalize rooted/imported variants (`alloc`/`core`/`std`/`crate` and imported single-segment aliases) to shared `decltype(...)` forms for `iter::*`, `intersperse::*`, `ziptuple::Zip`, and vec/deque `IntoIter` families.
- range/slice/index shapes (`range*`, collect, buffer arg lowering, `map/fold` frontier) are handled incrementally with parity checks.
- io and string/char path families (`from_utf8*`, `encode_utf8`, boundary checks, formatter/debug chains) are lowered to runtime-safe targets.
- `MaybeUninit` reference-typed storage access is hardened to avoid pointer-to-reference emission shapes (pointer aliases via `std::add_pointer_t` and laundered storage access).
- mixed optional-like surfaces in iterator/test-shape lowering now normalize `std::optional` receiver methods (`is_some`/`is_none`/`unwrap` → `has_value`/`!has_value`/`value`) while preserving runtime `Option` surfaces.
- pointer-helper cast lowering now preserves reference payload address semantics even when pointer target types are emitted as alias forms (`std::add_pointer_t<...>`): reference-like cast sources are emitted as `&expr` before pointer-typed adaptation in generic cast/read paths.
- runtime compatibility surfaces now include `rusty::mem::align_of<T>()`, `rusty::alloc::LayoutErr`, `Layout::from_size_align(...)`, `rusty::alloc::realloc(...)`, and `rusty::vec_extend_from_slice(...)` so expanded crate code no longer relies on missing memory/allocation/vector helper APIs.

Directly supports:

- §4.6 Iterators
- §3.3 pattern-driven iterator/match code
- §3.4 `?` in iterator-heavy contexts

#### 10.4.7 Module Emission, Ordering, and De-duplication

Integrated outcomes:

- inline module impl collection/merge is scoped and deterministic.
- duplicate methods are resolved by emitted C++ signature shape.
- forward declarations are emitted with guards to avoid alias-dependent type-order failures.
- forward declaration passes now select enum declaration surface by emitted shape (`enum class` for C-like enums, wrapper `struct` for recursive/impl-backed data enums, and alias-compatible variant forward declarations for alias-emitted data enums) and apply dependency-aware module ordering without delayable-namespace deferral, so sibling/nested namespace type surfaces are available before dependent alias/function signatures.
- split-module deferred emission now suppresses nested function bodies recursively in first pass and emits those bodies once in deferred pass, preventing early incomplete-type uses and duplicate nested function definitions in split module trees.
- `#[cfg(test)]` filtering and wrapper discovery were hardened for parity-test paths.
- deterministic module naming and work-dir artifact isolation are enforced for matrix reruns.

Directly supports:

- §2.5 Modules
- §2.6 Impl blocks
- §6 Architecture (deterministic build artifacts)

### 10.5 Parity Program Status

#### 10.5.1 Phase 18 (`either`) Consolidated Outcome

Outcome: The former long Phase 18 sequence is complete at the feature level and is now represented by the thematic integration in §10.4.

Key result:

- parity work moved from syntax/bootstrap blockers to semantic-shape correctness (constructors, match lowering, import/path/runtime surfaces), with repeated reprobe cycles until the first deterministic blocker family advanced.

#### 10.5.2 Phase 20 (Seven-Crate Matrix) Consolidated Outcome

Target matrix:

- `either`, `tap`, `cfg-if`, `take_mut`, `arrayvec`, `semver`, `bitflags`

Current observed matrix frontier:

- latest full matrix run (`/tmp/rusty-parity-matrix-10-5-40-10o-1775915467`) now passes all seven crates (`pass=7`, `fail=0`).
- latest verification rerun after lazy `if let` TRY-type-probe hardening (`/tmp/rusty-parity-matrix-iflet-try-decltype-1775920200`) also passes all seven crates (`pass=7`, `fail=0`).
- focused `bitflags` repro after Leaf 10.5.40.10 (`/tmp/rusty-parity-matrix-10-5-40-10n-1775915403`) passes (`pass=1`, `fail=0`).
- the prior `bitflags` Stage E semantic/parsing/fmt frontier is removed by shared transpiler fixes in Leaf 10.5.40.10.
- the prior `bitflags` Stage D `decltype((RUSTY_TRY_INTO(...)))` template-argument regression is removed by shared lazy if-let storage typing hardening (Leaf 10.5.40.11).

Crate-focused progress integrated from former appendices:

- `either`: parity-control crate and expanded-test correctness hardening
- `tap`: literal/method-shape and extension-trait lowering fixes
- `cfg-if`: baseline resiliency and alias/import typing fixes
- `take_mut`: type/lifetime order, ptr/mem path lowering, and template/context fixes
- `semver`: import/re-export lowering and expanded build-shape fixes
- `bitflags`: Stage D+E parity chain through Leaf 10.5.40.10 is closed; helper/parsing/format semantic parity now passes in matrix runs
- `arrayvec`: remaining deterministic frontier, advanced through many Stage D blocker families

### 10.6 Active Frontier and Next Work

The `bitflags` Stage D→E active frontier from the 10.5.40 leaf chain is now closed.

Current status snapshot:

1. Focused `bitflags` parity repro passes: `/tmp/rusty-parity-matrix-10-5-40-10n-1775915403/bitflags/{baseline.txt,build.log,run.log,matrix.log}`.
2. Full seven-crate matrix passes: `/tmp/rusty-parity-matrix-10-5-40-10o-1775915467/{either,tap,cfg-if,take_mut,arrayvec,semver,bitflags}/...` (`pass=7`, `fail=0`).
3. Full seven-crate matrix verification rerun also passes after Leaf 10.5.40.11 hardening: `/tmp/rusty-parity-matrix-iflet-try-decltype-1775920200/{either,tap,cfg-if,take_mut,arrayvec,semver,bitflags}/...` (`pass=7`, `fail=0`).
4. Next active work should follow the top unfinished TODO leaf after 10.5.40.11 closure.
5. Expanded ten-crate matrix snapshot (2026-04-11) is `pass=7`, `fail=3` with deterministic failing set `{smallvec, itertools, once_cell}`; canonical artifacts: `/tmp/rusty-parity-matrix-priority-20260411/{smallvec,itertools,once_cell}/{baseline.txt,build.log,run.log,matrix.log}`.
6. `smallvec` focused repro after `Leaf 5.1.2` (`/tmp/rusty-parity-matrix-5-1-2-20260411/smallvec/...`) collapses the prior unresolved `std::boxed`/`std::rc` and omitted-owner `SmallVec` template-arity family; first deterministic Stage D head now moves to incomplete-type/type-ordering fallout (`invalid use of incomplete type 'SmallVec<...>'`).
7. `itertools` focused repro after `Leaf 5.1.3` (`/tmp/rusty-parity-matrix-5-1-3-20260411g/itertools/...`) collapses the prior early adapter/type-order compile-head cluster (`VecDequeIntoIter`/`VecIntoIter`, early `::intersperse::Intersperse`, related namespace ordering fallout) from the first deterministic slot; new first Stage D head now starts at `merge_join` associated-type alias lowering (`MergeJoinBy = MergeBy<I, J, MergeFuncLR<F, T>>` with unbound `T`, `runner.cpp:1170`), followed by downstream `ziptuple::Zip`/`EitherOrBoth` runtime-surface fallout.
8. Full ten-crate matrix repro after `Leaf 5.1.4` (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10x-20260411h --keep-work-dirs`) recorded deterministic first failing crate `semver`; canonical artifacts: `/tmp/rusty-parity-matrix-10x-20260411h/semver/{baseline.txt,build.log,run.log,matrix.log}`.
9. Focused `semver` repro after `Leaf 5.1.5` now passes (`total=1`, `pass=1`, `fail=0`): `/tmp/rusty-parity-matrix-5-1-5-20260411b/semver/{baseline.txt,build.log,run.log,matrix.log}`; the prior declaration-surface head (`struct ErrorKind;` colliding with `using ErrorKind = std::variant<...>`) is collapsed by enum-forward-declaration shape alignment.
10. Full ten-crate matrix repro after `Leaf 5.1.6` (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-5-1-6-20260411a --keep-work-dirs`) advances deterministic first failure to `smallvec` (`total=8`, `pass=7`, `fail=1` before remaining crates), with canonical artifacts at `/tmp/rusty-parity-matrix-5-1-6-20260411a/smallvec/{baseline.txt,build.log,matrix.log}`.
11. New deterministic Stage D head begins at `runner.cpp:1065` in `smallvec`: incomplete-type/declaration-order fallout on `SmallVec<std::array<PanicOnDoubleDrop, 0>>` (`has initializer but incomplete type` and immediate nested-name-specifier incomplete-type errors), followed by downstream `catch_unwind` call-shape fallout.
12. Focused `smallvec` repro after `Leaf 5.1.7` (`/tmp/rusty-parity-matrix-5-1-7-20260411a/smallvec/...`) collapses the prior `runner.cpp:1065` incomplete-type/declaration-order family from the first deterministic slot.
13. Focused `smallvec` repro after `Leaf 5.1.8` (`/tmp/rusty-parity-matrix-5-1-8-20260411b/smallvec/...`) collapses the prior post-5.1.7 namespace/import/runtime-surface head (`std::hint`, `std::ops`, `LayoutErr`, `mem::align_of`, `Layout::from_size_align`, invalid `rusty::Vec::extend_from_slice` static path) from the first deterministic slot.
14. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1253` (`CollectionAllocErr::CapacityOverflow` enum-surface mismatch), followed by downstream `inline` identifier emission and associated-type/name-resolution fallout.
15. Focused `smallvec` repro after `Leaf 5.1.9` (`/tmp/rusty-parity-matrix-5-1-9-20260411a/smallvec/...`) collapses the prior post-5.1.8 enum-surface/identifier head (`CollectionAllocErr::CapacityOverflow` path-pattern mismatch at `runner.cpp:1253`, then `inline` keyword-identifier signature fallout) from the first deterministic slot.
16. Focused `smallvec` repro after `Leaf 5.1.10` (`/tmp/rusty-parity-matrix-5-1-10-20260411b/smallvec/...`) collapses the prior post-5.1.9 associated-type projection/constructor-shape first head (`runner.cpp:2128`, `SmallVec::IntoIter<A>` + immediate `MaybeUninit`/`SmallVecData_from_inline` fallout) from the first deterministic slot.
17. Focused `smallvec` repro after `Leaf 5.1.11` (`/tmp/rusty-parity-matrix-5-1-11-20260411b/smallvec/...`) collapses the prior post-5.1.10 owner-template/bound-type specialization first head (`NonNull::new_` unspecialized, imported `Included`/`Excluded` unresolved, and downstream unresolved `SmallVec<T>` leakage in `Drain<A>` construction) from the first deterministic slot.
18. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1616` with uninitialized local declaration surface (`auto new_alloc;` in `try_grow`), followed by method-path/type-resolution fallout (`usize::checked_next_power_of_two` and adjacent reserve-path diagnostics).
19. Focused `smallvec` repro after `Leaf 5.1.12` (`/tmp/rusty-parity-matrix-5-1-12-20260411c/smallvec/...`) collapses the prior post-5.1.11 local-declaration/method-path first head: untyped delayed-init locals now recover concrete hints (including associated-call expected-type fallback) and lower via `std::optional<T>` delayed-init storage instead of invalid `auto new_alloc;`; `usize::checked_next_power_of_two` now lowers through shared runtime helpers (`rusty::checked_next_power_of_two` / `rusty::checked_next_power_of_two_usize`).
20. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1761` (`template declaration cannot appear at block scope`), followed by malformed control-flow lowering around `runner.cpp:1827` (`return break ...` surface).
21. Focused `smallvec` repro after `Leaf 5.1.13` (`/tmp/rusty-parity-matrix-5-1-13-20260411c/smallvec/...`) collapses the prior post-5.1.12 block-scope template/control-flow first head (`runner.cpp:1761` template-at-block-scope + `runner.cpp:1827` `return break` malformed expression lowering).
22. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1789` (`rusty::range<size_t>` has no `.contains(...)`), followed by downstream template/path fallout (`rusty::Vec::from_raw_parts` used without template args and unresolved `mem::swap` namespace member).
23. Focused `smallvec` repro after `Leaf 5.1.14` (`/tmp/rusty-parity-matrix-5-1-14-20260411c/smallvec/...`) collapses the prior post-5.1.13 range/member/path first head (`runner.cpp:1789` `.contains(...)` on `rusty::range<size_t>`, plus adjacent unspecialized `Vec::from_raw_parts` and `mem::swap` path fallout).
24. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2004` (`SetLenOnDrop` incomplete-type nested-name call shape), followed by downstream declaration-order/runtime-surface fallout (`runner.cpp:2072` same `SetLenOnDrop::new_` family and `runner.cpp:2177` missing `Formatter::debug_tuple` surface).
25. Focused `smallvec` repro after `Leaf 5.1.15` (`/tmp/rusty-parity-matrix-5-1-15-20260411a/smallvec/...`) collapses the prior post-5.1.14 local-type-ordering/formatter-surface first head (`runner.cpp:2004`/`runner.cpp:2072` incomplete `SetLenOnDrop` associated-call ordering and `runner.cpp:2177`/`runner.cpp:2247` missing `Formatter::debug_tuple` surface).
26. Focused `smallvec` repro after `Leaf 5.1.16` (`/tmp/rusty-parity-matrix-5-1-16-20260411-171943/smallvec/...`) collapses the prior post-5.1.15 len-guard/callable-shape first head (`runner.cpp:1345` `*this->len` reference-deref mismatch, `runner.cpp:1354` `map(ConstNonNull)` callable shape, and `runner.cpp:2288` `move(for_each)` callable-resolution fallout).
27. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1371` (`no type named 'Item' in 'std::array<size_t, 0>'`), followed by adjacent `A::Item`-surface fallout across `SmallVec<std::array<size_t,0>>` API members and downstream equality/test-shape mismatches.
28. Focused `smallvec` repro after `Leaf 5.1.17` (`/tmp/rusty-parity-matrix-5-1-17-20260411a/smallvec/...`) collapses the prior post-5.1.16 associated-type owner-shape first head (`runner.cpp:1371` `A::Item` on `std::array` instantiations) by lowering dependent `Item` projections through shared runtime `associated_item_t` aliasing with array-like fallback support.
29. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2418` (`no match for operator==` between `SmallVec<std::array<size_t,0>>` and `std::array<int,1>`), followed by downstream string-literal ownership/equality-shape fallout.
30. Next active item is `Leaf 5.1.18` to collapse this post-5.1.17 equality/literal-surface family; guardrail check against §11 remains satisfied (shared AST/runtime-surface-aware fixes only, no crate-specific ad-hoc scripts or generated-output text patching).

Historical active-work chain (retained for traceability):

Active work items:

1. `Leaf 4.15.4.3.3.3.3.3.9.3.1` is complete.
   - `Ok/Err` constructor lowering now uses move-aware expected-type emission for local by-value payloads, so move-only payloads lower with `std::move(...)` where required.
   - iterator-like `.by_ref()` lowering now preserves iterator adapter chains by lowering to the receiver expression.
   - iterator-like `.take(...)` lowering now maps to shared runtime `rusty::take(...)`; iterator-like `.map(...)` lowering now accepts iterator-chain receivers (not only direct `.iter*()` forms).
   - runtime `include/rusty/slice.hpp` includes a shared `take` next-adapter integrated with existing `for_in`/`map`/`fold` option-like iterator adaptation.
2. `Leaf 4.15.4.3.3.3.3.3.9.3.2` is complete.
   - full matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-9-3-2-1775431204 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-9-3-2-1775431204/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
3. `Leaf 4.15.4.3.3.3.3.3.10.1` is complete.
   - generic runtime/transpiler hardening removed the prior deterministic Stage D lead diagnostics for `rusty::MaybeUninit<const T&>` reference-storage pointer shape and mixed optional-interface fallout (`std::optional` receiving `.is_some()`).
   - `arrayvec` reprobe artifact: `/tmp/rusty-parity-matrix-10-1b-1775434421/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
4. `Leaf 4.15.4.3.3.3.3.3.10.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10-2-1775435353 --keep-work-dirs`) now fails earlier at `take_mut` Stage D (`total=4`, `pass=3`, `fail=1`), with canonical artifacts at `/tmp/rusty-parity-matrix-10-2-1775435353/take_mut/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard errors now begin with scoped `take_or_recover` pointer-cast lowering shape (`static_cast<std::add_pointer_t<T>>(mut_ref)`), yielding invalid `T`→`T*` casts in pointer helper calls.
5. `Leaf 4.15.4.3.3.3.3.3.11.1` is complete.
   - generic cast lowering now treats AST pointer targets (`syn::Type::Ptr`) as pointer-typed even when rendered C++ type text is alias-based (`std::add_pointer_t<...>`), preserving address-of emission for reference-like sources.
   - focused transpiler regressions (`leaf41543333333111`) cover both direct generic casts and `std::ptr::read(...)` cast paths and assert the `static_cast<std::add_pointer_t<T>>(&...)` shape.
   - `take_mut` single-crate reprobe after 11.1 (`tests/transpile_tests/run_parity_matrix.sh --crate take_mut --work-root /tmp/rusty-parity-matrix-11-1-1775436329 --keep-work-dirs`) now passes.
6. `Leaf 4.15.4.3.3.3.3.3.11.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-11-2-1775437753 --keep-work-dirs`) now fails first at `arrayvec` Stage D (`total=5`, `pass=4`, `fail=1`), with canonical artifacts at `/tmp/rusty-parity-matrix-11-2-1775437753/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard errors now begin with fixed-capacity constructor shape mismatch (`ArrayVec::<T, N>::from(rusty::array_repeat(..., N))`), where emitted repeat shape is `std::vector<T>` while `from` expects `std::array<T, N>`.
7. `Leaf 4.15.4.3.3.3.3.3.12.1` is complete.
   - implemented context-gated fixed-array repeat lowering for fixed-capacity constructor contexts, including `ArrayVec::from/try_from([val; N])` when owner generics are explicit, inferred, or recovered from expected/scope hints.
   - fixed-array repeat materialization now uses a non-capturing lambda form (`[](auto _seed){...}(<expr>)`) to stay valid in non-local contexts; dynamic repeat lowering remains unchanged outside explicit fixed-array contexts (aligned with §11.3 no-blanket-rewrite rule).
   - focused regressions (`leaf41543333333121`) cover explicit-owner and omitted-owner `ArrayVec` repeat constructors plus non-capturing fixed-array lambda shape and dynamic-repeat preservation.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-12-1-fix-1775439781 --keep-work-dirs`) removed the prior first deterministic head (`ArrayVec::<T,N>::from(rusty::array_repeat(...))` mismatch); canonical artifacts at `/tmp/rusty-parity-matrix-12-1-fix-1775439781/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
8. `Leaf 4.15.4.3.3.3.3.3.12.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-12-2-1775440731 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-12-2-1775440731/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard errors begin with iterator-adapter method-shape gaps on iterator-like receivers (`arrayvec::Drain<...>.rev()` and `rusty::slice_iter::Iter<...>.enumerate()` emitted as missing members), followed by downstream `Result` visit/call-shape cascades.
9. `Leaf 4.15.4.3.3.3.3.3.13.1` is complete.
   - transpiler lowering now rewrites iterator-like `.rev()`/`.enumerate()` calls to shared runtime adapters (`rusty::rev(...)` / `rusty::enumerate(...)`) under `is_iterator_like_receiver_expr` gating; non-iterator methods with the same names are preserved unchanged (keeps §11.3 no-blanket-rewrite discipline).
   - runtime `include/rusty/slice.hpp` now provides shared next-adapter surfaces for `rev` and `enumerate` with option-like iterator constraints and `next_back()` enforcement for reverse iteration.
   - focused transpiler/runtime regressions were added (`leaf41543333333131` + `tests/rusty_array_test.cpp` adapter-shape coverage).
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-13-1-1775441890 --keep-work-dirs`) removed the prior first deterministic head (`Drain<...>.rev()` / `Iter<...>.enumerate()` missing members); canonical artifacts at `/tmp/rusty-parity-matrix-13-1-1775441890/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
10. `Leaf 4.15.4.3.3.3.3.3.13.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-13-2-1775442875 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-13-2-1775442875/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains iterator mutability/constness fallout in `test_retain`: `assignment of read-only location` at `runner.cpp:3007` on `*elt = i` in the `rusty::enumerate(rusty::iter(v))` loop, followed by existing downstream `Result` visit/call-shape and constructor/trait-surface cascades.
11. `Leaf 4.15.4.3.3.3.3.3.14.1` is complete.
   - transpiler method lowering now preserves iterator mutability intent: `.iter()` lowers to `rusty::iter(...)`, while `.iter_mut()` lowers to `rusty::iter_mut(...)` (no conflation).
   - runtime `include/rusty/slice.hpp` now provides `rusty::iter_mut(...)` with mutable-first adaptation order (`iter_mut()`, then `as_mut_slice()`, then `deref_mut()`, then mutable `data()/size()` / mutable iterator fallback), preserving §11.3 no-blanket-rewrite discipline by targeting only mutable iterator surfaces.
   - focused regressions were added (`leaf41543333333141` + runtime probe coverage in `tests/rusty_array_test.cpp`) to assert mutability-preserving lowering and writable element access through `rusty::enumerate(rusty::iter_mut(...))`.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-14-1b-1775444041 --keep-work-dirs`) removed the prior first deterministic head (`test_retain` read-only assignment on `*elt = i` from iterator mutability loss); canonical artifacts at `/tmp/rusty-parity-matrix-14-1b-1775444041/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now begins with `Result` visit/call-shape mismatch in `test_insert` (`std::visit(..., rusty::Result<...>)`).
12. `Leaf 4.15.4.3.3.3.3.3.14.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-14-2-1775444973 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-14-2-1775444973/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains `Result` visit/call-shape mismatch in `test_insert`: `std::visit(..., rusty::Result<std::tuple<>, errors::CapacityError<int>>)` at `runner.cpp:3204` / `3219`, followed by existing downstream constructor/trait/string/template-surface cascades.
13. `Leaf 4.15.4.3.3.3.3.3.15.1` is complete.
   - shared pattern-binding lowering in `transpiler/src/codegen.rs` now handles nested struct payload patterns (`Pat::Struct`) inside tuple-variant matches, including `{ .. }` and field-binding forms.
   - this keeps `Result`-shaped statement/expression matches on runtime helper dispatch (`is_err`/`unwrap_err`, `is_ok`/`unwrap`) instead of falling back to `std::visit` for nested payload shape.
   - focused transpiler regressions were added (`leaf41543333333151`) covering statement and expression runtime dispatch for `Err(CapacityError { .. })` plus nested field binding extraction from unwrapped payloads.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-15-1b-1775446136 --keep-work-dirs`) removed the prior deterministic first hard head (`std::visit(..., rusty::Result<...>)` mismatch); canonical artifacts at `/tmp/rusty-parity-matrix-15-1b-1775446136/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at unqualified Result constructor emission in `test_into_inner_1` (`Err` not declared at `runner.cpp:3236`), followed by downstream string/constructor/template-surface diagnostics.
14. `Leaf 4.15.4.3.3.3.3.3.15.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-15-2-1775447107 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-15-2-1775447107/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at unqualified `Result` constructor emission in `test_into_inner_1` (`Err` not declared at `runner.cpp:3236`; generated shape `auto _m1_tmp = Err(std::move(u));`), followed by downstream string-conversion/constructor/template-surface cascades.
15. `Leaf 4.15.4.3.3.3.3.3.16.1` is complete.
   - shared tuple/assertion binding scaffolding in `transpiler/src/codegen.rs` now hardens unresolved `Result` constructor emission by deriving constructor context from tuple peers (`using _ResultCtorCtx = std::remove_cvref_t<decltype((peer))>`) and emitting `_ResultCtorCtx::Ok/Err(...)` instead of bare `Ok/Err`.
   - focused transpiler regressions were added (`leaf41543333333161`) covering both `Err(id(u))` and `Ok(id(u))` unresolved-payload tuple-match assertions, asserting context-qualified emission and absence of bare constructor calls.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-16-1-1775448430 --keep-work-dirs`) removed the prior deterministic first hard head (`Err` not declared from `auto _m1_tmp = Err(...)`); canonical artifacts at `/tmp/rusty-parity-matrix-16-1-1775448430/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts with ownership/copy fallout in `test_into_inner_1` (`use of deleted function` at `runner.cpp:3236`) from `_ResultCtorCtx::Err(std::move(u))` where `u` is emitted as `const auto u = v.clone();`.
16. `Leaf 4.15.4.3.3.3.3.3.16.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-16-2-1775449410 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-16-2-1775449410/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains ownership/copy fallout in `test_into_inner_1`: `use of deleted function` at `runner.cpp:3236` from `_ResultCtorCtx::Err(std::move(u))` where `u` is emitted as `const auto u = v.clone();`, followed by downstream string-conversion/constructor/template-surface cascades.
17. `Leaf 4.15.4.3.3.3.3.3.17.1` is complete.
   - constructor payload lowering for context-qualified `Result` constructor scaffolding now tracks local constness in block scope and avoids forcing `std::move(...)` only for const-local payload path args.
   - consuming-use pre-scan now treats bare `Ok(...)`/`Err(...)` payload locals as consuming contexts, so those locals are emitted non-const and remain move-constructible for `_ResultCtorCtx::Ok/Err(...)`.
   - focused transpiler regressions (`leaf41543333333171`) assert both `Err` and `Ok` tuple-match assertion paths emit non-const payload locals and `_ResultCtorCtx::{Err,Ok}(std::move(u))` constructor calls.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-17-1b-1775450609 --keep-work-dirs`) removed the prior deterministic first hard head (`use of deleted function` at `runner.cpp:3236` from `_ResultCtorCtx::Err(std::move(u))` with `const auto u = ...`); canonical artifacts at `/tmp/rusty-parity-matrix-17-1b-1775450609/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3243`: `no match for operator==` on `rusty::Result<...>` equality in assertion scaffolding.
18. `Leaf 4.15.4.3.3.3.3.3.17.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-17-2-1775451587 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-17-2-1775451587/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:3243`: `no match for operator==` on `rusty::Result<std::array<int, 2>, arrayvec::ArrayVec<int, 2>>` equality in assertion scaffolding, followed by downstream string-conversion/array-comparison/template/runtime-surface cascades.
19. `Leaf 4.15.4.3.3.3.3.3.18.1` is complete.
   - runtime parity surface now includes `operator==` / `operator!=` for `rusty::Result<T, E>` and `rusty::Result<void, E>` in `include/rusty/result.hpp`, comparing variant first and then payload equality for matching variants.
   - focused regressions were added in `tests/rusty_result_test.cpp` for `Result` equality semantics (`Ok` vs `Ok`, `Err` vs `Err`, variant mismatch, and void-specialization comparisons).
   - focused transpiler regression (`leaf41543333333181`) asserts Result tuple-match assertion shapes keep direct `*left_val == *right_val` comparisons and context-typed `Result::Ok(...)` constructor emission (no bare `Ok(...)` in tuple scaffolding).
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-18-1-1775452758 --keep-work-dirs`) removed the prior deterministic first hard head (`no match for operator==` on `rusty::Result<...>` at `runner.cpp:3243`); canonical artifacts at `/tmp/rusty-parity-matrix-18-1-1775452758/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3258`: `request for member 'into'` on string literals (`("a").into()`), followed by downstream array/string comparison and template/runtime-surface cascades.
20. `Leaf 4.15.4.3.3.3.3.3.18.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-18-2-1775453841 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-18-2-1775453841/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains string-literal conversion surface mismatch in `test_into_inner_2`: `request for member 'into'` at `runner.cpp:3258` (`("a").into()` and siblings), followed by downstream array/string comparison (`std::array<rusty::String, 4>` vs `std::array<const char*, 4>` at `runner.cpp:3272`) and existing template/runtime-surface cascades.
21. `Leaf 4.15.4.3.3.3.3.3.19.1` is complete.
   - transpiler `.into()` lowering now applies shape-gated conversion emission for literal/primitive receivers in typed contexts:
     - string-like receivers lower to valid conversion surfaces (`rusty::String::from(...)`, `std::string(...)`, `std::string_view(...)`) rather than Rust trait-style member calls.
     - scalar-like receivers lower to typed `static_cast<target>(...)` surfaces when target type is scalar-like.
     - non-primitive receivers are preserved unchanged (no blanket rewrite).
   - method-arg expected-type inference for receiver-gated methods (`push/insert/set`) now allows concrete receiver-driven substitution when declared argument type is an uppercase generic placeholder (`T`-style), enabling `.into()` lowering in generic method call contexts.
   - focused regressions were added (`leaf41543333333191`) covering string-literal typed `.into()`, scalar typed `.into()`, and non-primitive `.into()` non-rewrite behavior.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-19-1-1775455372 --keep-work-dirs`) removed the prior deterministic first hard head (`("a").into()` member-call failure at `runner.cpp:3258`); canonical artifacts at `/tmp/rusty-parity-matrix-19-1-1775455372/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3272`: `no match for operator==` between `std::array<rusty::String, 4>` and `std::array<const char*, 4>`.
22. `Leaf 4.15.4.3.3.3.3.3.19.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-19-2-1775456311 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-19-2-1775456311/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains array/string equality surface mismatch in `test_into_inner_2`: `no match for operator==` at `runner.cpp:3272` between `std::array<rusty::String, 4>` and `std::array<const char*, 4>`, followed by downstream template/runtime-surface cascades.
23. `Leaf 4.15.4.3.3.3.3.3.20.1` is complete.
   - runtime equality surface now includes constrained mixed-element `std::array` comparison in `include/rusty/array.hpp` for differing element types with one-direction element comparability (`l == r` or `r == l`), while preserving standard same-type `std::array` equality behavior.
   - focused runtime regression coverage was added in `tests/rusty_array_test.cpp` (`test_array_cross_element_equality_shape`) for `std::array<rusty::String, N>` vs `std::array<const char*, N>` equality/inequality and both operand orders.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-20-1-1775457435 --keep-work-dirs`) removed the prior deterministic first hard head (`no match for operator==` at `runner.cpp:3272` between `std::array<rusty::String, 4>` and `std::array<const char*, 4>`); canonical artifacts at `/tmp/rusty-parity-matrix-20-1-1775457435/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3342`: omitted-template owner shape `ArrayVec<auto, 8>::new_()` (`wrong number of template arguments`), followed by downstream method/template/runtime-surface cascades.
24. `Leaf 4.15.4.3.3.3.3.3.20.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-20-2-1775458371 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-20-2-1775458371/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:3342`: omitted-template owner constructor shape `ArrayVec<auto, 8>::new_()` (`wrong number of template arguments (1, should be 2)`), followed by downstream method/template/runtime-surface cascades (`to_vec` missing, omitted template args for `ArrayString`/`HashMap`, unresolved `RUSTY_TRY`/`Ok`, and `parse`-surface fallout).
25. `Leaf 4.15.4.3.3.3.3.3.21.1` is complete.
   - transpiler local-placeholder owner recovery was hardened in `transpiler/src/codegen.rs` for omitted-template constructor shapes:
     - method-call hint collection now recognizes `write` / `write_all` / `write_fmt` receiver contexts for candidate locals and seeds element-type recovery for constructor owner placeholders.
     - simple-local receiver extraction now peels reference wrappers (`&expr` / `&mut expr`) before identifier resolution so receiver shapes like `(&mut v).write_fmt(...)` participate in inference.
   - focused transpiler regressions were added (`leaf41543333333211`) asserting `ArrayVec::<_, 8>::new()` recovers `ArrayVec<uint8_t, 8>::new_()` from both `write(...)` and `write_fmt(...)` receiver contexts (no `ArrayVec<auto, 8>::new_()` emission).
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-21-1-1775459126 --keep-work-dirs`) removed the prior deterministic first hard head (`ArrayVec<auto, 8>::new_()` at `runner.cpp:3342`); canonical artifacts at `/tmp/rusty-parity-matrix-21-1-1775459126/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3343`: pointer/member-access shape mismatch on `(&v).write_fmt(...)` (`request for member 'write_fmt' in '& v'`), followed by downstream method/template/runtime-surface cascades.
26. `Leaf 4.15.4.3.3.3.3.3.21.2` is complete.
   - full seven-crate rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-21-2-1775460145 --keep-work-dirs`) remains `pass=4`, `fail=1` with first failure at `arrayvec` Stage D.
   - canonical artifacts: `/tmp/rusty-parity-matrix-21-2-1775460145/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:3343`: reference-wrapped receiver method-call access mismatch in `test_write` (`(&v).write_fmt(...)` emitted as pointer-plus-`.` member access), followed by downstream method/template/runtime-surface cascades (`write` element-shape mismatch, `to_vec` missing, omitted template args for `ArrayVec`/`ArrayString`/`HashMap`, unresolved `RUSTY_TRY`/`Ok`, and `parse`-surface fallout).
27. `Leaf 4.15.4.3.3.3.3.3.22.1` is complete.
   - method-call receiver lowering now selects member access surface from lowered receiver shape (pointer vs value) instead of fixed `.` emission:
     - added receiver pointer-shape detection for reference-wrapped receivers and existing pointer-like receivers.
     - centralized receiver member-call emission so generic/default method-call lowering and `map_err` callable lowering use consistent `.`/`->` selection.
     - updated optional-like and `assume_init` member-call surfaces to respect receiver pointer/value shape.
   - focused transpiler regressions were added (`leaf41543333333221`) asserting reference-wrapped receivers emit `->` and non-pointer receivers keep `.`.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-22-1-1775461263 --keep-work-dirs`) removed the prior deterministic first hard head (`(&v).write_fmt(...)` pointer-plus-`.` mismatch at `runner.cpp:3343`); canonical artifacts at `/tmp/rusty-parity-matrix-22-1-1775461263/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3343`: method-surface mismatch (`ArrayVec<uint8_t, 8>` has no `write_fmt` member), followed by downstream method/template/runtime-surface cascades (`write` element-shape mismatch, `to_vec` missing, omitted template args for `ArrayVec`/`ArrayString`/`HashMap`, unresolved `RUSTY_TRY`/`Ok`, and `parse`-surface fallout).
28. `Leaf 4.15.4.3.3.3.3.3.23.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-23-2-1775464818 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-23-2-1775464818/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:3362`: `cannot convert span<const int, ...> to span<const unsigned char, ...>` in `v.write(rusty::slice_full(rusty::array_repeat(9, 16)))`.
29. `Leaf 4.15.4.3.3.3.3.3.24.1` is complete.
   - implemented shape-gated byte-write expected-type propagation in `transpiler/src/codegen.rs` for IO buffer-argument lowering (`write` / `write_all`), including `slice_full(array_repeat(...))` forms.
   - added focused transpiler regressions (`leaf41543333333241`) for byte-context `write`/`write_all` seed typing and non-byte control behavior.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-24-1c-1775466747 --keep-work-dirs`) removed the prior deterministic first hard head at `runner.cpp:3362` (`span<const int>` to `span<const unsigned char>` write-arg mismatch); canonical artifacts at `/tmp/rusty-parity-matrix-24-1c-1775466747/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:535`: `request for member 'write'` on pointer receiver (`rusty::ptr::add(...)`), followed by downstream `MaybeUninit` pointer/type-shape fallout.
30. `Leaf 4.15.4.3.3.3.3.3.24.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-24-2-1775470862 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-24-2-1775470862/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:535`: `request for member 'write'` in `rusty::ptr::add(ptr, 0)` (non-class pointer receiver) in `char_::encode_utf8`, followed by downstream `ArrayVec::to_vec`/omitted-template/`RUSTY_TRY`/`parse` fallout.
31. `Leaf 4.15.4.3.3.3.3.3.25.1` is complete.
   - implemented shape-gated pointer write-call hardening in `transpiler/src/codegen.rs`:
     - expanded pointer-valued receiver detection for `add`/`offset` call-path families (`rusty::ptr`, `ptr`, and `core/std::ptr::{mut_ptr,const_ptr}` forms),
     - prevented IO buffer-call normalization from capturing raw-pointer receiver writes,
     - kept non-pointer `write` calls on standard member-call lowering.
   - added focused regressions (`leaf41543333333251`) covering UFCS pointer-add receivers, non-pointer write control behavior, and pointer-write behavior under competing IO write hints.
   - expanded function-path mapping in `transpiler/src/types.rs` for pointer UFCS arithmetic helpers (`core/std/ptr::mut_ptr::{add,offset}` and `core/std/ptr::const_ptr::{add,offset}` to `rusty::ptr::{add,offset}`).
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-25-1b-1775476125 --keep-work-dirs`) removed the prior deterministic first hard head at `runner.cpp:535` (`rusty::ptr::add(...).write(...)` member-call pointer mismatch); canonical artifacts at `/tmp/rusty-parity-matrix-25-1b-1775476125/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3408`: `ArrayVec<rusty::Vec<int>, 4>` has no `to_vec`, followed by downstream omitted-template/`clone_from` pointer-arg/`ArrayString`/`HashMap`/`RUSTY_TRY`/`parse` fallout.
32. `Leaf 4.15.4.3.3.3.3.3.25.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-25-2-1775480817 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-25-2-1775480817/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:3408`: `ArrayVec<rusty::Vec<int>, 4>` has no member `to_vec` in `array_clone_from`, followed by downstream omitted-template and call-shape fallout (`ArrayVec<auto,4>` arity, `clone_from(&v)` pointer arg mismatch, `ArrayString`/`HashMap` missing template args, `RUSTY_TRY`/`Ok`, `parse`, and related type/runtime-surface diagnostics).
33. `Leaf 4.15.4.3.3.3.3.3.26.1` is complete.
   - implemented shape-gated `ArrayVec` call-surface fixes in `transpiler/src/codegen.rs`:
     - `.to_vec()` on `ArrayVec`/slice-like receiver shapes now lowers to `rusty::to_vec(receiver)` (non-`ArrayVec` `to_vec` methods remain unchanged),
     - `clone_from` now uses reference-style argument fallback when method-signature pass-style metadata is unavailable, avoiding pointer-arg emission (`clone_from(&src)` -> `clone_from(src)`),
     - local placeholder hint recovery now accepts `clone_from` source shapes (including reuse of earlier in-block placeholder hints) so omitted owner placeholders recover concrete `ArrayVec` element types.
   - added runtime helper support in `include/rusty/array.hpp`:
     - `rusty::to_vec(const Container&)` uses `slice_full` surfaces and clone-aware element forwarding (`.clone()` when available) to support non-copy element types.
   - added focused regressions:
     - transpiler tests (`leaf41543333333261`) for `to_vec` helper lowering, non-ArrayVec control behavior, and `clone_from`-driven omitted-owner recovery,
     - runtime regression in `tests/rusty_array_test.cpp` for `rusty::to_vec` slice-surface behavior.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-26-1c-1775484823 --keep-work-dirs`) removed the prior deterministic first hard head family in `array_clone_from` (`to_vec` missing + `ArrayVec<auto,4>` + `clone_from(&v)` mismatch); canonical artifacts at `/tmp/rusty-parity-matrix-26-1c-1775484823/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3480`: `ArrayString` used without template arguments, followed by downstream template/runtime-surface diagnostics (`HashMap` omitted args, `RUSTY_TRY`/`Ok`, parse-surface and related fallout).
34. `Leaf 4.15.4.3.3.3.3.3.26.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-26-2-1775487958 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-26-2-1775487958/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:3480`: omitted-template owner constructor surface `ArrayString::new_()` (`ArrayString` used without template arguments), followed by downstream omitted-template/runtime-surface diagnostics (`HashMap::new_()` missing template args, repeated `ArrayString` omitted args, unresolved `RUSTY_TRY`/`Ok`, and `parse`-surface fallout on C-string receivers).
35. `Leaf 4.15.4.3.3.3.3.3.27.1` is complete.
   - implemented shape-gated omitted-owner recovery for `ArrayString`/`HashMap` associated constructor surfaces in `transpiler/src/codegen.rs`:
     - expanded owner template recovery for explicit and omitted owner args using expected-type + local usage hints,
     - extended placeholder/local-binding inference to recover `ArrayString` const capacity and `HashMap<K, V>` key/value args from nearby usage (for example `insert`),
     - lowered recovered `rusty::HashMap<...>::new_()` calls to constructor form (`rusty::HashMap<...>()`) so emitted code matches runtime `HashMap` API.
   - added focused regressions (`leaf415433333333271`) for:
     - `ArrayString::<CAP>::new()` explicit-owner const-generic preservation,
     - omitted-owner `HashMap::new()` recovery + constructor-form lowering from insert usage context.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-1b-1775495221 --keep-work-dirs`) removed the prior deterministic omitted-template head family (`ArrayString::new_()` / `HashMap::new_()` missing owner args); canonical artifacts: `/tmp/rusty-parity-matrix-27-1b-1775495221/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3514`: `rusty::HashMap<ArrayString<16>, int>` key-hash/moveability surface failures (`std::hash` and move-ctor constraints for key type), followed by downstream runtime-surface/type-shape fallout.
   - guardrail check against wrong-approach checklist (§11): kept fixes root-cause-first and shape-gated; no blanket associated-call rewrites were introduced.
36. `Leaf 4.15.4.3.3.3.3.3.27.3.1` is complete.
   - implemented generic runtime HashMap indexing/lookup hardening in `include/rusty/hashmap.hpp`:
     - added lookup-only `operator[]` (missing key throws) to match Rust index read semantics (no implicit insertion),
     - added heterogeneous borrowed-key lookup overloads (`get/get_mut/remove/contains_key/operator[]`) with shape-gated key comparability (`KeyEqual` when available, else `lhs == rhs` / `rhs == lhs`).
   - added focused fixture-agnostic runtime regressions in `tests/rusty_hashmap_test.cpp` for lookup-only index semantics and heterogeneous borrowed-key lookup on `HashMap<rusty::String, int>`.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-3-1-1775506238 --keep-work-dirs`) removed the prior first deterministic head at `runner.cpp:3518` (`map[text]` index shape mismatch on `HashMap<ArrayString<16>, int>`).
37. `Leaf 4.15.4.3.3.3.3.3.27.3.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-3-2-1775506272 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-3-2-1775506272/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3556`: invalid `ArrayString<2>` to `std::string_view` conversion shape, followed by downstream string/runtime/template fallout (`Ok` path resolution, `parse` on C-string, omitted-template `ArrayString::from_byte_string`, and string/span equality-shape errors).
38. `Leaf 4.15.4.3.3.3.3.3.27.4.1` is complete.
   - implemented generic transpiler hardening for the first deterministic 27.3.2 string-surface head family in `transpiler/src/codegen.rs`:
     - `ArrayString`/string-like values now coerce to `std::string_view` in expected `&str` contexts (including full-range `[..]` lowering under string-view expectation),
     - no-turbofish `.parse()` lowering now consumes expected target type and emits numeric `rusty::str_runtime::parse<T>(...)` or non-numeric `T::from_str(...)` call-shapes,
     - omitted-owner `ArrayString::from_byte_string(...)` calls now recover const-capacity generic args from byte-string/array argument shape,
     - expected `rusty::String` argument contexts now coerce string literals through `rusty::String::from(...)`,
     - constructor-context recovery now handles closure bodies with `?` + `Ok/Err` by deriving result constructor hints from nearby try-result type context (avoids bare `Ok(...)`/`Err(...)` emission).
   - added focused fixture-agnostic transpiler regressions (`leaf4154333333332741`) covering each new shape.
   - guardrail check against wrong-approach checklist (§11): changes stay shape-gated and context-driven; no crate-specific scripts and no blanket call-site rewrites were introduced.
39. `Leaf 4.15.4.3.3.3.3.3.27.4.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-4-2-1775508053 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-4-2-1775508053/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3558`: rvalue address-taking in assertion tuple lowering (`auto _m0 = &std::string_view(tmut)`), followed by adjacent closure result-constructor hint fallout at `runner.cpp:3577` (self-referential `t_shadow1` in `Result<...>::Ok(...)` type synthesis), and then downstream type/runtime-surface diagnostics.
   - guardrail check against wrong-approach checklist (§11): kept deterministic first-head discipline and recorded frontier movement from canonical matrix artifacts before any broader rewrites.
40. `Leaf 4.15.4.3.3.3.3.3.27.5.1` is complete.
   - implemented generic transpiler hardening in `transpiler/src/codegen.rs` for the first deterministic 27.4.2 head family:
     - tuple binding/reference match lowering now treats coerced/non-lvalue reference targets as non-addressable and materializes them into `_m*_tmp` temporaries before taking addresses (preventing `&std::string_view(...)` rvalue-address emission),
     - closure emission now binds closure parameter names in nested emission scope so payload/hint expressions resolve to closure locals instead of outer shadow bindings,
     - local-initializer constructor-hint recovery now temporarily hides the in-progress local binding while scanning initializer expressions, preventing self-referential `decltype`/constructor-context synthesis.
   - added focused fixture-agnostic transpiler regressions (`leaf4154333333332751`) covering:
     - tuple string-view coercion reference materialization (no `&std::string_view(...)` emission),
     - closure `Ok(...)` constructor context recovery without self-referential shadow bindings.
   - guardrail check against wrong-approach checklist (§11): fixes are shape-gated in shared lowering paths and avoid crate-specific scripts or callsite-only rewrites.
41. `Leaf 4.15.4.3.3.3.3.3.27.5.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-5-2-1775509389 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-5-2-1775509389/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:3940` in `test_pop_at`: function-item binding lowered as `const auto s = rusty::String::from;` and fails C++ deduction (`unable to deduce const auto from rusty::String::from`), followed by adjacent unresolved Rust-path/default-surface fallout (`alloc::vec::from_elem` at `runner.cpp:4043`, `Default::default_`/`std::net` at `runner.cpp:4066-4068`) and downstream container-shape/type/runtime diagnostics.
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline and recorded canonical artifacts before opening the next fix leaf.
42. `Leaf 4.15.4.3.3.3.3.3.27.6.1` is complete.
   - implemented generic callable/default/path-surface hardening across transpiler/runtime surfaces:
     - local initializer function-item path values now lower to callable wrappers (generic forwarding lambdas), removing invalid direct value binding of associated methods (`const auto s = rusty::String::from;`),
     - zero-arg trait-path `Default::default()` now lowers contextually to `rusty::default_value<T>()` when expected type is known,
     - `alloc::vec::from_elem` now maps to `rusty::array_repeat`,
     - `std::net` import/type surfaces now lower without unresolved Rust namespace emission (`use std::net;` is Rust-only; `std::net::TcpStream` maps to `rusty::net::TcpStream`).
   - runtime additions:
     - `include/rusty/rusty.hpp`: `rusty::default_value<T>()` helper that prefers `T::default_()` and otherwise value-initializes,
     - `include/rusty/net.hpp`: minimal `rusty::net::TcpStream` compatibility surface.
   - added focused fixture-agnostic regressions in `transpiler/src/codegen.rs` (`leaf4154333333332761` tests) and `transpiler/src/types.rs` mapping tests.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf4154333333332761 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): fixes remain shape-gated in shared lowering/mapping paths; no crate-specific scripts and no blanket namespace rewrites were introduced.
43. `Leaf 4.15.4.3.3.3.3.3.27.6.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-6-2-1775510882 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-6-2-1775510882/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:4052` in `test_sizes`: assertion tuple compare emits `operator==` between `std::vector<unsigned char>` and `std::span<const unsigned char>`, which has no viable overload.
   - adjacent deterministic fallout in the same family appears at `runner.cpp:4130` and `runner.cpp:4186` (`std::span<Z>` compared with `std::vector<Z>`), indicating a shared container/slice equality-shape gap.
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline and recorded canonical artifacts/failure-family evidence before opening the next implementation leaf.
44. `Leaf 4.15.4.3.3.3.3.3.27.7.1` is complete.
   - implemented generic runtime container/slice equality hardening in `include/rusty/array.hpp`:
     - added bidirectional `std::vector`↔`std::span` equality overloads used by transpiled assertion tuple compare scaffolding,
     - kept shape-gated element comparison (`lhs == rhs` or `rhs == lhs`) and added empty marker-like element fallback when explicit equality operators are absent.
   - added focused fixture-agnostic runtime regression in `tests/rusty_array_test.cpp` (`test_vector_span_equality_helper_shape`) covering:
     - `uint8_t` vector/span equality in both directions,
     - custom comparable element equality,
     - empty marker-like element equality fallback and size mismatch behavior.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-7-1c-1775511708 --keep-work-dirs`) removed the prior deterministic first hard error family at `runner.cpp:4052/4130/4186` (`std::vector`↔`std::span` assertion equality mismatch).
   - new deterministic first hard error now starts at `runner.cpp:1636`: `ArrayString::try_from(std::string_view)` auto-return deduction conflict (`Result<std::tuple<>, _>` vs `Result<ArrayString<16>, _>`), with adjacent fallout at `runner.cpp:4239`, `runner.cpp:4262`, and `runner.cpp:4293+`.
   - verification:
     - `ctest --test-dir build-tests --output-on-failure -R rusty_array_test`
     - `ctest --test-dir build-tests --output-on-failure`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): fix is runtime-shared and shape-gated, with no crate-specific rewrites and no blanket transpiler callsite rewiring.
45. `Leaf 4.15.4.3.3.3.3.3.27.7.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-7-2-1775512081 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-7-2-1775512081/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:1636`: `ArrayString::try_from(std::string_view)` auto-return deduction conflict (`Result<std::tuple<>, _>` vs `Result<ArrayString<16>, _>`), with adjacent fallout at `runner.cpp:4239` (`std::string_view` construction from tuple), `runner.cpp:4262` (`ArrayVec<std::tuple<>, usize::MAX>` shape), and `runner.cpp:4293+` (`constexpr`/template-surface diagnostics).
   - verification:
     - `cargo test -p rusty-cpp-transpiler --test parity_matrix_harness`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline and recorded canonical full-matrix artifacts before opening the next implementation leaf; no crate-specific rewrites were introduced.
46. `Leaf 4.15.4.3.3.3.3.3.27.8.1` is complete.
   - implemented generic typed-Result `?` propagation in shared runtime/transpiler paths:
     - `include/rusty/try.hpp`: added `RUSTY_TRY_INTO(expr, ReturnResultType)` and `RUSTY_CO_TRY_INTO(expr, ReturnResultType)` so `?` can propagate `Err(E)` into explicit `Result<U, E>` return shapes.
     - `transpiler/src/codegen.rs`: `Expr::Try` lowering now emits typed try macros in known `Result` return contexts, keeps `RUSTY_*_TRY_OPT` behavior for `Option`, and preserves legacy `RUSTY_TRY`/`RUSTY_CO_TRY` fallback when return type hints are unavailable.
   - added fixture-agnostic regressions (`leaf415433333333281`) covering:
     - sync/async `Result<(), E>` `?` propagation uses typed try macros,
     - `try_from`-like `Result<Self, E>` body keeps `Self` in return typing (no `std::tuple<>` collapse).
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-8-1-1775513113 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:1636` (`ArrayString::try_from(std::string_view)` auto-return mismatch between `Result<std::tuple<>, _>` and `Result<ArrayString<16>, _>`).
   - new deterministic first hard error now starts at `runner.cpp:4239` (`std::string_view` construction from `const ArrayString<16>&`), followed by adjacent fallout at `runner.cpp:4262` (`ArrayVec<std::tuple<>, usize::MAX>` shape) and `runner.cpp:4293+` (`constexpr`/template-shape diagnostics).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-8-1-1775513113/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `ctest --test-dir build-tests --output-on-failure`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-8-1-1775513113 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, used shared shape-gated fixes only, and introduced no crate-specific scripts or blanket rewrites.
47. `Leaf 4.15.4.3.3.3.3.3.27.8.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-8-2-1775513306 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-8-2-1775513306/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:4239`: `std::string_view` construction from `const ArrayString<16>&` in `test_try_from_argument`, followed by adjacent fallout at `runner.cpp:4262` (`ArrayVec<std::tuple<>, usize::MAX>` shape) and `runner.cpp:4293+` (`constexpr`/template-shape diagnostics).
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
48. `Leaf 4.15.4.3.3.3.3.3.27.9.1` is complete.
   - implemented generic string-view coercion hardening across shared runtime/transpiler surfaces:
     - `include/rusty/rusty.hpp`: added `rusty::to_string_view(...)` helper that prefers `.as_str()` when available and otherwise falls back to direct `std::string_view(...)` construction.
     - `transpiler/src/codegen.rs`: unresolved local-path `std::string_view` coercions in expected-type lowering now emit `rusty::to_string_view(local)` instead of forcing `std::string_view(local)`.
   - added fixture-agnostic regression (`test_leaf4154333333332791_untyped_arraystring_path_uses_string_view_helper`) validating tuple-assertion-style coercion for untyped local `ArrayString` bindings.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-9-1-1775514182 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:4239` (`std::string_view(const ArrayString<16>&)` mismatch in `test_try_from_argument`).
   - new deterministic first hard error now starts from the max-capacity tuple family rooted at `runner.cpp:4262` (`ArrayVec<std::tuple<>, usize::MAX>` shape; first compiler diagnostic reported via `/usr/include/c++/14/array:61`), followed by adjacent fallout at `runner.cpp:4293+` (`constexpr`/template-shape diagnostics).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-9-1-1775514182/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `ctest --test-dir build-tests --output-on-failure`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-9-1-1775514182 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline and used shared shape-gated fixes only (no crate-specific rewrites/scripts).
49. `Leaf 4.15.4.3.3.3.3.3.27.9.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-9-2-1775514496 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-9-2-1775514496/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts from the max-capacity tuple family rooted at `runner.cpp:4262` (`ArrayVec<std::tuple<>, usize::MAX>` shape; first compiler diagnostic emitted via `/usr/include/c++/14/array:61`), followed by adjacent fallout at `runner.cpp:4293+` (`constexpr`/template-shape diagnostics).
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
50. `Leaf 4.15.4.3.3.3.3.3.27.10.1` is complete.
   - plan/scope check: fix remains under the small-change budget (<1000 LOC), so no additional leaf decomposition was required.
   - implemented generic max-capacity array materialization hardening across shared runtime/transpiler surfaces:
     - `include/rusty/rusty.hpp`: added `rusty::sanitize_array_capacity<N>()`, mapping only `N == std::numeric_limits<size_t>::max()` to a safe compile-time placeholder (`1`) for array type materialization.
     - `transpiler/src/codegen.rs`: array type emission now routes risky max-capacity/path-like const-generic capacities through `rusty::sanitize_array_capacity<...>()`, preventing emission of invalid `std::array<..., SIZE_MAX>` instantiations while preserving normal const-generic capacities.
   - added fixture-agnostic transpiler regressions:
     - `test_leaf41543333333327101_array_const_generic_capacity_uses_sanitizer`
     - `test_leaf41543333333327101_array_usize_max_capacity_uses_sanitizer`
     - updated existing const-generic struct emission assertion (`test_leaf4154_const_generic_template_preserved_in_struct_and_type_use`) to the sanitized array-capacity shape.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-10-1-1775515473 --keep-work-dirs`) removed the prior deterministic first hard head rooted at `runner.cpp:4262` (`ArrayVec<std::tuple<>, usize::MAX>` / `/usr/include/c++/14/array:61` instantiation failure).
   - new deterministic first hard error now starts at `runner.cpp:4293` (`constexpr` literal-type/copy-template fallout family), with canonical artifacts at `/tmp/rusty-parity-matrix-27-10-1-1775515473/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `ctest --test-dir build-tests --output-on-failure`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-10-1-1775515473 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): changes are shared and shape-gated in core mapping/runtime logic, deterministic first-head discipline is preserved, and no crate-specific rewrites/scripts were introduced.
51. `Leaf 4.15.4.3.3.3.3.3.27.10.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-10-2-1775515714 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-10-2-1775515714/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:4293`: `constexpr ArrayVec<rusty::Vec<uint8_t>, 10> OF_U8 = ArrayVec<...>::new_const();` fails because `ArrayVec<rusty::Vec<uint8_t>, 10>` is non-literal / non-copyable in this surface, with adjacent fallout at `runner.cpp:4294+` and neighboring `ArrayString::new_const`/template-surface diagnostics.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-10-2-1775515714 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
52. `Leaf 4.15.4.3.3.3.3.3.27.11.1` is complete.
   - plan/scope check: fix stays under the small-change budget (<1000 LOC), so no extra leaf decomposition was needed.
   - implemented generic local const-constructor materialization hardening in `transpiler/src/codegen.rs`:
     - added shape-gated local-const detection for zero-arg `new_const()` constructor calls inside block scope,
     - such local consts now emit as factory-form locals (`const auto NAME = []() -> Ty { return Ty::new_const(); };`) and path uses lower to `NAME()` to materialize fresh values per use.
   - added fixture-agnostic transpiler regressions:
     - `test_leaf41543333333327111_local_new_const_uses_factory_materialization`
     - `test_leaf41543333333327111_local_scalar_const_stays_constexpr`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-11-1-1775516598 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:4293` (`constexpr ArrayVec<rusty::Vec<uint8_t>, 10> OF_U8 = ...::new_const()` non-literal/copy fallout).
   - new deterministic first hard error now starts at `runner.cpp:4317` (`cannot convert Vec<int> to Vec<unsigned char>` in `test_arrayvec_const_constructible`), with canonical artifacts at `/tmp/rusty-parity-matrix-27-11-1-1775516598/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-11-1-1775516598 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, used shared AST-aware shape-gated lowering (no text patching / no blanket rewrites), and introduced no crate-specific scripts.
53. `Leaf 4.15.4.3.3.3.3.3.27.11.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-11-2-1775516802 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-11-2-1775516802/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:4317`: `cannot convert Vec<int> to Vec<unsigned char>` in `test_arrayvec_const_constructible` (`var.push(into_vec(box_new(std::array{3,5,8})))` payload element-shape mismatch), followed by adjacent fallout at `runner.cpp:4375+` (ArrayString/char assertion equality shape) and downstream type/runtime diagnostics.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-11-2-1775516802 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
54. `Leaf 4.15.4.3.3.3.3.3.27.12.1` is complete.
   - plan/scope check: fix stays under the small-change budget (<1000 LOC), so no extra leaf decomposition was needed.
   - implemented generic boxed-array/vector payload coercion hardening in `transpiler/src/codegen.rs`:
     - added shape-gated `into_vec(box_new(...))` specialization when expected type is `Vec<u8>`,
     - for boxed array/repeat payloads in that context, emit byte-compatible element shape (`static_cast<uint8_t>(...)`) before vector conversion.
   - added fixture-agnostic transpiler regressions:
     - `test_leaf41543333333327121_into_vec_box_new_u8_context_coerces_array_elements`
     - `test_leaf41543333333327121_into_vec_box_new_non_u8_context_unchanged`
     - `test_leaf41543333333327121_into_vec_box_new_u8_context_coerces_repeat_seed`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-12-1-1775517396 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:4317` (`Vec<int>` vs `Vec<uint8_t>` push payload mismatch in `test_arrayvec_const_constructible`).
   - new deterministic first hard error now starts at `runner.cpp:4375`: `no match for operator==` between `ArrayString<10>` and `const char` in `test_arraystring_const_constructible`, with canonical artifacts at `/tmp/rusty-parity-matrix-27-12-1-1775517396/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-12-1-1775517396 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): kept changes shared and shape-gated in AST-aware lowering; no crate-specific scripts and no blanket numeric literal rewrites were introduced.
55. `Leaf 4.15.4.3.3.3.3.3.27.12.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-12-2-1775517571 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-12-2-1775517571/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:4375`: `no match for operator==` between `ArrayString<10>` and `const char` in `test_arraystring_const_constructible` assertion tuple shape (`&var` vs `&*"hello"`), followed by adjacent downstream type/runtime diagnostics.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-12-2-1775517571 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
56. `Leaf 4.15.4.3.3.3.3.3.27.13.1` is complete.
   - plan/scope check: fix stayed under the small-change budget (<1000 LOC), so no additional leaf decomposition was required.
   - implemented generic assertion tuple string-literal deref coercion hardening in `transpiler/src/codegen.rs`:
     - tuple binding reference targets shaped as `*"..."` now lower to materialized `std::string_view("...")` temporaries before address-taking in tuple scaffolding,
     - this keeps downstream `*left_val == *right_val` comparisons string-like and removes scalar `const char` comparison fallout from `&*"hello"` RHS shapes.
   - added fixture-agnostic transpiler regressions:
     - `test_leaf41543333333327131_tuple_assertion_string_literal_deref_rhs_materializes_string_view_temp`
     - `test_leaf41543333333327131_tuple_assertion_non_string_deref_keeps_pointer_shape`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-13-1-1775518197 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:4375` (`ArrayString<10>` vs `const char` assertion tuple mismatch), with generated `runner.cpp` now emitting `_m1_tmp = std::string_view("hello")` in `test_arraystring_const_constructible`.
   - new deterministic first hard error now starts at `runner.cpp:1060`: `use of deleted function arrayvec::ArrayVec<rusty::Vec<int>, 3>::ArrayVec(const ...)`, with canonical artifacts at `/tmp/rusty-parity-matrix-27-13-1-1775518197/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-13-1-1775518197 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): kept the fix shared and shape-gated in AST-aware tuple lowering, with no crate-specific scripts and no assertion callsite special-casing.
57. `Leaf 4.15.4.3.3.3.3.3.27.13.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-13-2-1775518425 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-13-2-1775518425/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - new deterministic first hard error now starts at `runner.cpp:1060`: `use of deleted function arrayvec::ArrayVec<rusty::Vec<int>, 3>::ArrayVec(const ...)` in `ArrayVec::into_iter()` return lowering (`IntoIter<T, CAP>(0, (*this))`), followed by adjacent move/copy-surface diagnostics.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-13-2-1775518425 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
58. `Leaf 4.15.4.3.3.3.3.3.27.14.1` is complete.
   - plan/scope check: fix stayed under the small-change budget (<1000 LOC), so no additional leaf decomposition was required.
   - implemented generic consuming-`self` move hardening in `transpiler/src/codegen.rs`:
     - move insertion now treats path `self` as movable only under by-value receiver scope (`fn foo(self)`), while keeping `&self`/`&mut self` unchanged,
     - struct-literal lowering now uses move-aware field emission in both constructor-ordered and designated-field paths, so consuming `self` field payloads are not emitted as lvalue copies.
   - added fixture-agnostic transpiler regressions:
     - `test_leaf41543333333327141_consuming_self_constructor_call_moves_this`
     - `test_leaf41543333333327141_consuming_self_struct_literal_moves_this_field`
     - `test_leaf41543333333327141_borrowed_self_argument_is_not_moved`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-14-1b-1775519130 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:1060` (`IntoIter<T, CAP>(0, (*this))` deleted-copy fallback); generated `runner.cpp` now emits `IntoIter<T, CAP>(0, std::move((*this)))`.
   - new deterministic first hard error now starts at `runner.cpp:838`: `cannot convert rusty::MaybeUninit<rusty::Vec<int>>* to rusty::Vec<int>*` in `ArrayVec::get_unchecked_ptr` return via `rusty::ptr::add(rusty::as_mut_ptr((*this)), ...)`, with canonical artifacts at `/tmp/rusty-parity-matrix-27-14-1b-1775519130/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-14-1b-1775519130 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): kept changes shared and receiver-shape-gated in AST-aware lowering, avoided crate-specific scripts, and avoided one-off callsite rewrites.
59. `Leaf 4.15.4.3.3.3.3.3.27.14.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-14-2-1775519325 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-14-2-1775519325/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:838`: `cannot convert rusty::MaybeUninit<...>* to std::add_pointer_t<T>` in `ArrayVec::get_unchecked_ptr` via `rusty::ptr::add(rusty::as_mut_ptr((*this)), ...)`; prior `runner.cpp:1060` consuming-self constructor head remains collapsed.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-14-2-1775519325 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
60. `Leaf 4.15.4.3.3.3.3.3.27.15.1` is complete.
   - plan/scope check: fix stayed under the small-change budget (<1000 LOC), so no additional leaf decomposition was required.
   - implemented expected-pointer-aware raw-pointer lowering in `transpiler/src/codegen.rs`:
     - added `expected_raw_pointer_cpp_type(...)` to detect concrete expected raw-pointer context and avoid placeholder-based casts,
     - `as_ptr`/`as_mut_ptr` helper lowering now adapts pointee shape to expected pointer type for contexts that require payload pointer form (`T*`) instead of storage pointer form (`MaybeUninit<T>*`),
     - pointer arithmetic lowering (`add`/`offset`, method and function call surfaces) now propagates pointer expected-type context into receiver emission.
   - added fixture-agnostic transpiler regressions:
     - `test_leaf41543333333327151_as_mut_ptr_chain_add_adapts_expected_pointer_pointee`
     - `test_leaf41543333333327151_as_mut_ptr_argument_adapts_to_expected_pointer_shape`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327151 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): kept changes shared and type-context-gated in AST-aware lowering, with no crate-specific scripts or fixture-specific rewrites.
61. `Leaf 4.15.4.3.3.3.3.3.27.15.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-15-2-1775520042 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-15-2-1775520042/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1022`: `ArrayVec::as_mut_slice` could not convert `span<rusty::MaybeUninit<T>>` to `span<T>` through `return ArrayVecImpl::as_mut_slice((*this));`, followed by adjacent `MaybeUninit` payload/slice fallout.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-15-2-1775520042 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
62. `Leaf 4.15.4.3.3.3.3.3.27.16.1` is complete.
   - plan/scope check: fix stayed under the small-change budget (<1000 LOC), so no additional leaf decomposition was required.
   - implemented shared runtime payload-pointer adaptation in `include/rusty/array.hpp`:
     - `rusty::as_ptr`/`rusty::as_mut_ptr` now adapt `MaybeUninit<T>*` storage pointers to payload-pointer shapes (`T*`/`const T*`) through shared detail helpers where payload-facing context is available,
     - adaptation applies to both member-pointer and `.data()` helper branches so direct storage arrays and wrapper containers follow the same payload pointer contract.
   - added focused runtime regressions in `tests/rusty_array_test.cpp`:
     - `test_maybe_uninit_array_payload_pointer_adaptation_shape`
     - `test_container_item_pointer_adaptation_shape`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-16-1-1775520565 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:1022` (`span<MaybeUninit<T>>` to `span<T>` mismatch in `ArrayVec::as_mut_slice`) and adjacent `Option<T>(MaybeUninit<T>)` fallout.
   - new deterministic first hard error now starts at `runner.cpp:1123`: `no matching function for call to ArrayVec<int, 2>::extend_from_iter(...)` (template parameter `CHECK` could not be deduced), with canonical artifacts at `/tmp/rusty-parity-matrix-27-16-1-1775520565/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `ctest --test-dir build-tests -R rusty_array_test --output-on-failure`
     - `ctest --test-dir build-tests --output-on-failure`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-16-1-1775520565 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): kept fix shared and context-gated in runtime helper surfaces, avoided crate-specific scripts, and preserved deterministic first-head capture.
63. `Leaf 4.15.4.3.3.3.3.3.27.16.2` is complete.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-16-2-1775520852 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-16-2-1775520852/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1123`: `no matching function for call to ArrayVec<int, 2>::extend_from_iter(...)` because template parameter `CHECK` cannot be deduced from `extend_from_iter(rusty::iter(slice_shadow1).cloned())`, followed by adjacent iterator/template-deduction fallout.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-16-2-1775520852 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline, recorded canonical matrix artifacts before opening the next implementation leaf, and introduced no crate-specific rewrites.
64. `Leaf 4.15.4.3.3.3.3.3.27.17.1` is complete.
   - implemented generic transpiler hardening in `transpiler/src/codegen.rs`:
     - const generic defaults are now preserved in emitted template parameter lists (for example `template<typename I, bool CHECK = false>`),
     - method-call turbofish const args are now preserved (including `::<_, true/false>`), with infer type placeholders lowered to `std::remove_cvref_t<decltype((arg))>` rather than being dropped.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327171_method_const_generic_default_is_preserved`
     - `test_leaf41543333333327171_free_fn_const_generic_default_is_preserved`
     - `test_leaf41543333333327171_method_turbofish_const_args_are_preserved`
     - `test_leaf41543333333327171_nonself_method_turbofish_const_args_are_preserved`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-17-1-1775521581 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:1123` (`extend_from_iter` template-parameter deduction failure).
   - new deterministic first hard error now starts at `runner.cpp:1043`: invalid `static_cast` from `const std::array<int, 3>*` to `const std::array<rusty::MaybeUninit<int>, 3>*`, with canonical artifacts at `/tmp/rusty-parity-matrix-27-17-1-1775521581/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327171 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-17-1-1775521581 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fix remains generic and fixture-agnostic (no crate-specific scripts), inference/template adaptation is context-gated, and deterministic first-head artifact capture was preserved.
65. `Leaf 4.15.4.3.3.3.3.3.27.17.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-17-2-1775521860 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-17-2-1775521860/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1043`: invalid `static_cast` from `const std::array<int, 3>*` to `const std::array<rusty::MaybeUninit<int>, 3>*` in the `ArrayVec::from` storage-copy path; adjacent fallout includes move-only copy-constructor failures through `rusty::mem` helper calls.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-17-2-1775521860 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
66. `Leaf 4.15.4.3.3.3.3.3.27.18.1` is complete.
   - plan/scope check: transpiler-only implementation stayed well under the <1000 LOC threshold and required no additional decomposition.
   - implemented generic transpiler hardening in `transpiler/src/codegen.rs`:
     - pointer-to-pointer Rust `as` casts now emit `reinterpret_cast` instead of invalid cross-type `static_cast`,
     - raw-pointer copy-method surfaces (`copy_to_nonoverlapping` / `copy_to` / `copy_from_nonoverlapping` / `copy_from`) now lower to `rusty::ptr` runtime helpers,
     - `drop(...)` callsite scanning now marks immutable locals as consumed so transpiled locals remain movable (`auto`) for move-only payloads.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327181_pointer_to_pointer_cast_uses_reinterpret_cast`
     - `test_leaf41543333333327181_drop_marks_immutable_local_as_consumed`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-18-1-1775522528 --keep-work-dirs`) removed the prior deterministic first hard error at `runner.cpp:1043` (invalid `std::array<T,N>*` → `std::array<MaybeUninit<T>,N>*` cast) and adjacent move-only `rusty::mem::drop` copy-constructor failures.
   - new deterministic first hard error now starts at `runner.cpp:967`: `std::visit` overload mismatch in `ArrayVec::drain` bound lowering (`Bound<usize>` visitor arms emitted against `Bound<int>` variant payload), with canonical artifacts at `/tmp/rusty-parity-matrix-27-18-1-1775522528/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327181 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-18-1-1775522528 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): kept fixes generic and context-scoped, avoided crate-specific scripts/one-off rewrites, and preserved deterministic first-head capture before advancing.
67. `Leaf 4.15.4.3.3.3.3.3.27.18.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-18-2-1775522678 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-18-2-1775522678/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:967`: `std::visit` overload mismatch in `ArrayVec::drain` bound lowering (`Bound<usize>` visitor arms emitted against `Bound<int>` variant payload), with adjacent pointer-call/lambda-signature fallout.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-18-2-1775522678 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
68. `Leaf 4.15.4.3.3.3.3.3.27.19.1` is complete.
   - plan/scope check: transpiler/runtime-only implementation remained under the <1000 LOC guardrail and required no additional TODO decomposition.
   - implemented generic transpiler/runtime hardening:
     - `transpiler/src/codegen.rs`: runtime `Bound` visit-arm parameter typing now recovers from `std::variant_alternative_t<idx, std::remove_reference_t<decltype(_m)>>` when pattern template args are implicit, removing hardcoded `Bound<size_t>` assumptions.
     - `transpiler/src/codegen.rs`: statement-side `std::visit` lowering now binds scrutinee as `_m` in a local scope so the same variant-type recovery path is available for arm emission.
     - `transpiler/src/codegen.rs`: raw-pointer local inference now covers `as_ptr`/`as_mut_ptr` method+call forms, pointer add/offset/sub call shapes (including `core::ptr::*`/`std::ptr::*`), and `unsafe { ... }` wrapped initializers.
     - `transpiler/src/codegen.rs`: raw-pointer method lowering now includes `sub` alongside `add`/`offset` for pointer locals and chained raw-pointer expressions.
     - `include/rusty/ptr.hpp`: added shared runtime helper overloads `rusty::ptr::sub(const T*, Count)` / `rusty::ptr::sub(T*, Count)`.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327191_bound_match_visit_uses_variant_alternative_type_recovery`
     - `test_leaf41543333333327191_local_raw_pointer_add_sub_calls_lower_to_runtime_helpers`
     - `test_leaf41543333333327191_std_ptr_add_local_receiver_sub_lowers_to_runtime_helper`
     - `test_leaf41543333333327191_unsafe_local_pointer_add_sub_lowers_to_runtime_helpers`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-19-1d-1775524379 --keep-work-dirs`) removed the prior deterministic heads at `runner.cpp:967` (`Bound<size_t>` vs `Bound<int>` `std::visit` mismatch) and `runner.cpp:1304` (`ptr.add(...)` member-call on raw pointer).
   - new deterministic first hard error now starts at `runner.cpp:918` (`return rusty::ptr::drop_in_place(cur)` void-value misuse in `retain` lambda), followed by adjacent `SafeFn` argument-shape mismatches at `runner.cpp:933/938`; canonical artifacts are at `/tmp/rusty-parity-matrix-27-19-1d-1775524379/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327191 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-matrix-27-19-1d-1775524379 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes remain shared and shape-gated in core lowering/runtime paths, with no crate-specific scripts or fixture-only rewrites.
69. `Leaf 4.15.4.3.3.3.3.3.27.19.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-19-2-1775524575 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-19-2-1775524575/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:918`: `return rusty::ptr::drop_in_place(cur)` void-value misuse in the `retain` backshift lambda, with adjacent callable-surface mismatches at `runner.cpp:933/938`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-19-2-1775524575 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
70. `Leaf 4.15.4.3.3.3.3.3.27.20.1` is complete.
   - plan/scope check: transpiler-only implementation remained under the <1000 LOC guardrail and required no additional decomposition.
   - implemented generic transpiler hardening in `transpiler/src/codegen.rs`:
     - control-flow statement emission now scopes tail-return behavior, preventing statement-position `unsafe`/block/if/match lowering from inheriting enclosing value-return scope and emitting invalid `return <void_expr>;` forms;
     - nested function-item lowering now records local function argument pass-style/expected-type metadata before emission, so local callable invocations map Rust `&` / `&mut` arguments to C++ reference-shaped call surfaces when signatures require references.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_unsafe_block_statement_in_value_return_scope_does_not_emit_void_return`
     - `test_leaf41543333333327201_nested_fn_mut_ref_args_are_passed_by_reference`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-20-1-arrayvec --keep-work-dirs`) removed the deterministic retain/backshift head family at `runner.cpp:918` (`return rusty::ptr::drop_in_place(cur)` void-return misuse) and adjacent callable-surface mismatches at `runner.cpp:933/938`.
   - new deterministic first hard error now starts at `runner.cpp:857`: `rusty::ptr::copy` call-shape mismatch in `ArrayVec::try_insert` (`const auto* p` causes destination `ptr::offset(p, 1)` to remain `const T*`), with canonical artifacts at `/tmp/rusty-parity-27-20-1-arrayvec/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_unsafe_block_statement_in_value_return_scope_does_not_emit_void_return`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327201_nested_fn_mut_ref_args_are_passed_by_reference`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-20-1-arrayvec --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes are shared/context-gated at AST emission points, avoid crate-specific scripts, and preserve deterministic first-head artifact discipline.
71. `Leaf 4.15.4.3.3.3.3.3.27.20.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-20-2-1775525451 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-20-2-1775525451/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:857`: `rusty::ptr::copy` call-shape mismatch in `ArrayVec::try_insert` (`const auto* p = get_unchecked_ptr(...)` feeds `ptr::offset(p, 1)` as a const destination), with adjacent fallout at `runner.cpp:1137`, `runner.cpp:1408`, and `runner.cpp:1588`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-20-2-1775525451 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
72. `Leaf 4.15.4.3.3.3.3.3.27.21.1` is complete.
   - plan/scope check: transpiler-only implementation remained under the <1000 LOC guardrail and required no additional decomposition.
   - implemented generic transpiler hardening in `transpiler/src/codegen.rs`:
     - immutable local binding emission now preserves mutable raw-pointer pointee shape (`*mut`) by emitting const pointer bindings (`T* const` / `auto* const`) instead of pointer-to-const (`const T*` / `const auto*`);
     - applied in both `Pat::Ident` and `Pat::Type` local-lowering paths so typed and inferred pointer locals share the same mutability behavior.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327211_typed_mut_ptr_local_keeps_writable_pointee_shape`
     - `test_leaf41543333333327211_inferred_mut_ptr_local_keeps_writable_pointee_shape`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-21-1-1775526113 --keep-work-dirs`) removed the deterministic `try_insert` head at `runner.cpp:857` (const destination pointer shape in `rusty::ptr::copy`).
   - new deterministic first hard error now starts at `runner.cpp:1137`: `std::span<rusty::Vec<int>>` has no member `clone_from_slice`, with canonical artifacts at `/tmp/rusty-parity-27-21-1-1775526113/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327211_typed_mut_ptr_local_keeps_writable_pointee_shape`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327211_inferred_mut_ptr_local_keeps_writable_pointee_shape`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-21-1-1775526113 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fix remains shared/context-gated in AST emission, avoids crate-specific scripts, and preserves deterministic first-head artifact capture.
73. `Leaf 4.15.4.3.3.3.3.3.27.21.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-21-2-1775526278 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-21-2-1775526278/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1137`: `std::span<rusty::Vec<int>>` has no member `clone_from_slice`, with adjacent fallout at `runner.cpp:1408` and `runner.cpp:1588`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-21-2-1775526278 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
74. `Leaf 4.15.4.3.3.3.3.3.27.22.1` is complete.
   - plan/scope check: transpiler/runtime implementation remained a focused change set well under the <1000 LOC guardrail; no additional decomposition was required.
   - implemented generic transpiler/runtime hardening:
     - `transpiler/src/codegen.rs`: added receiver-shape-gated lowering for method-call `clone_from_slice` so slice/span-like receivers dispatch through `rusty::clone_from_slice(...)` instead of invalid member calls on `std::span`.
     - `transpiler/src/codegen.rs`: added slice/span receiver-shape detection for this lowering (slice range indexing, typed slice/span receivers, and raw slice-constructor expressions) while preserving non-slice user methods as member calls.
     - `include/rusty/array.hpp`: added shared runtime helper `rusty::clone_from_slice(std::span<...>, std::span<...>)` with Rust-like size check and clone-aware element assignment (`elem.clone()` when available, assignment fallback otherwise).
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327221_slice_clone_from_slice_dispatches_to_runtime_helper`
     - `test_leaf41543333333327221_non_slice_clone_from_slice_stays_member_call`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-22-1-1775527001 --keep-work-dirs`) removed the deterministic head at `runner.cpp:1137` (`std::span<rusty::Vec<int>>` missing `clone_from_slice`).
   - new deterministic first hard error now starts at `runner.cpp:1408` (designator-order mismatch for `ArrayString<CAP>::new_` aggregate initialization), with adjacent fallout at `runner.cpp:1588` (`&*"..."` address-of-rvalue/string-view compare shape) and `include/rusty/array.hpp:309` (`len(const char*)` no matching `std::size`) in the same build.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327221_slice_clone_from_slice_dispatches_to_runtime_helper -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327221_non_slice_clone_from_slice_stays_member_call -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-22-1-1775527001 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fix is shared and receiver-shape-gated in core lowering/runtime paths, avoids crate-specific scripts and blanket rewrites, and preserves deterministic first-head artifact discipline.
75. `Leaf 4.15.4.3.3.3.3.3.27.22.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-22-2-1775527215 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-22-2-1775527215/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1408`: designator order mismatch for `array_string::ArrayString<CAP>::len_field` in aggregate initialization, with adjacent fallout at `runner.cpp:1588` (`&*"..."` address-of-rvalue / string-view comparison shape).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-22-2-1775527215 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
76. `Leaf 4.15.4.3.3.3.3.3.27.23.1` is complete.
   - plan/scope check: transpiler-only implementation remained under the <1000 LOC guardrail and required no additional decomposition.
   - implemented generic transpiler hardening in `transpiler/src/codegen.rs`:
     - struct-literal lowering for named aggregates now emits designated fields in declaration order (using recorded struct field-order metadata) rather than source-expression order, preserving C++ designated-initializer ordering requirements;
     - existing drop-sensitive full-field constructor lowering remains unchanged, so move/drop semantics for those shapes are not broadened by this fix.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327231_struct_literal_designators_follow_decl_order`
     - `test_leaf41543333333327231_arraystring_like_literal_designators_follow_decl_order`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-23-1-1775527657 --keep-work-dirs`) removed the deterministic head at `runner.cpp:1408` (aggregate designator order mismatch for `ArrayString<CAP>::new_`).
   - new deterministic first hard error now starts at `runner.cpp:1588`: `&*"..."` address-of-rvalue and `std::string_view*` vs `std::string_view` compare-shape mismatch, with adjacent fallout at `include/rusty/array.hpp:309` (`len(const char*)` unresolved `std::size`) and downstream string/constexpr-capacity errors.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327231_struct_literal_designators_follow_decl_order -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327231_arraystring_like_literal_designators_follow_decl_order -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-23-1-1775527657 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fix is shared and AST-context-gated, avoids crate-specific scripts or text-rewrite shortcuts, and preserves deterministic first-head artifact discipline.
77. `Leaf 4.15.4.3.3.3.3.3.27.23.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-23-2-1775527809 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-23-2-1775527809/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1588`: `&*"..."` address-of-rvalue and `std::string_view*` vs `std::string_view` comparison mismatch, with adjacent fallout at `include/rusty/array.hpp:309` (`len(const char*)` no matching `std::size`) and downstream string/capacity constexpr errors.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-23-2-1775527809 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and added no crate-specific scripts/rewrite shortcuts.
78. `Leaf 4.15.4.3.3.3.3.3.27.24.1` is complete.
   - plan/scope check: transpiler-only implementation remained under the <1000 LOC guardrail and required no additional decomposition.
   - implemented generic reborrow collapse hardening in `transpiler/src/codegen.rs`: `&*` collapse now recurses through nested unary-deref operands in reference-expression lowering, removing address-of-rvalue artifacts for string-like `&**self` comparison paths while preserving raw-pointer-sensitive behavior through existing guards.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327241_nested_self_deref_reborrow_drops_address_of_artifact`
     - `test_leaf41543333333327241_raw_pointer_reborrow_of_deref_is_not_collapsed`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-24-1-1775528337 --keep-work-dirs`) removed the deterministic head at `runner.cpp:1588` (`&*"..."` / `std::string_view*` compare mismatch).
   - new deterministic first hard error now starts at `include/rusty/array.hpp:309` (`len(const char*)` calls `std::size` on raw C-string), with adjacent fallout at `runner.cpp:1617` (`std::string_view(ArrayString<4>)` conversion shape) and downstream capacity/constexpr errors.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327241_nested_self_deref_reborrow_drops_address_of_artifact -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327241_raw_pointer_reborrow_of_deref_is_not_collapsed -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-24-1-1775528337 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fix is shared and AST-context-gated in core reference/unary lowering, avoids crate-specific scripts and post-generation rewrites, and preserves deterministic first-head artifact discipline.
79. `Leaf 4.15.4.3.3.3.3.3.27.24.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-24-2-1775529801 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-24-2-1775529801/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `include/rusty/array.hpp:309`: `len(const char*)` attempts `std::size` on `const char*`, with adjacent fallout at `runner.cpp:1617` (`std::string_view(ArrayString<4>)` conversion shape) and downstream capacity/constexpr errors.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-24-2-1775529801 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
80. `Leaf 4.15.4.3.3.3.3.3.27.25.1` is complete.
   - plan/scope check: transpiler/runtime implementation remained under the <1000 LOC guardrail and required no additional decomposition.
   - implemented generic runtime C-string length fallback in `include/rusty/array.hpp`: added `rusty::len(const char*)` and `rusty::len(char*)` overloads (null-safe `std::strlen`) so transpiled `&str` pointer-like surfaces no longer hit the generic `std::size(container)` path.
   - implemented generic string-view coercion hardening in `transpiler/src/codegen.rs`: expected-`std::string_view` lowering now routes `&Self`-typed local path expressions through `rusty::to_string_view(...)`, preserving `.as_str()` preference for string-like owners and eliminating invalid `std::string_view(rhs)` shapes.
   - added focused regression in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327251_self_typed_path_expected_str_uses_string_view_helper`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-25-1-1775531207 --keep-work-dirs`) removed deterministic heads at `include/rusty/array.hpp:309` and adjacent `runner.cpp:1617`; generated site now lowers to `try_push_str(rusty::to_string_view(rhs))`.
   - new deterministic first hard error now starts at `runner.cpp:710`: `MakeMaybeUninit` constexpr/materialization family (`usize::MAX` array-capacity fallout), with adjacent errors at `runner.cpp:711` and downstream `MaybeUninit` constexpr/zeroed surfaces.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327251_self_typed_path_expected_str_uses_string_view_helper -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-25-1-1775531207 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes are shared runtime/transpiler changes, AST/type-context-gated, and avoid crate-specific scripts or post-generation rewrites.
81. `Leaf 4.15.4.3.3.3.3.3.27.25.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-25-2-1775531752 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-25-2-1775531752/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:710`: `MakeMaybeUninit<T>::VALUE` uses non-`constexpr` `MaybeUninit<T>::uninit()`, with adjacent fallout at `runner.cpp:711` (array materialization/`usize::MAX` capacity conversion) and instantiation roots at `/usr/include/c++/14/array:61`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-25-2-1775531752 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
82. `Leaf 4.15.4.3.3.3.3.3.27.26.1` is complete.
   - plan/scope check: transpiler-only implementation remained under the <1000 LOC guardrail and required no additional decomposition.
   - implemented generic impl-associated-const lowering hardening in `transpiler/src/codegen.rs`: impl const items mapped to `rusty::MaybeUninit<...>` surfaces now emit `static inline const` instead of `static constexpr`, avoiding invalid constexpr requirements for `MaybeUninit` initialization/copy paths while preserving `static constexpr` for other const families.
   - implemented generic fixed-array repeat sanitization in `transpiler/src/codegen.rs`: repeat-lambda materialization now applies `rusty::sanitize_array_capacity<...>()` for path/max-capacity lengths in both direct fixed-array expected contexts and recovered ArrayVec owner-capacity paths, preventing unsanitized `std::array<..., N>` materialization for `usize::MAX`-like capacities.
   - updated focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf415432_local_assoc_const_template_args_recovered_with_name_mismatch` now asserts sanitized inner repeat capacity (`sanitize_array_capacity<N>()`).
     - `test_leaf4154333333361_impl_const_maybeuninit_uninit_uses_expected_owner_type` now asserts `static inline const rusty::MaybeUninit<T> VALUE = rusty::MaybeUninit<T>::uninit();`.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-26-1-1775532325 --keep-work-dirs`) removed deterministic heads at `runner.cpp:710/711` (`MakeMaybeUninit` constexpr/materialization + unsanitized-capacity fallout).
   - new deterministic first hard error now starts at `runner.cpp:1469`: missing `MaybeUninit<std::array<...>>::zeroed`, with adjacent fallout at `runner.cpp:1073` (`raw_ptr_add` deduction) and downstream iterator/visit return-shape families.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf415432_local_assoc_const_template_args_recovered_with_name_mismatch -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf4154333333361_impl_const_maybeuninit_uninit_uses_expected_owner_type -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-26-1-1775532325 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes are shared transpiler changes, AST/type-context-gated, and avoid crate-specific scripts or post-generation rewrites.
83. `Leaf 4.15.4.3.3.3.3.3.27.26.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no code changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-26-2-1775532846 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-26-2-1775532846/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1469`: missing `rusty::MaybeUninit<std::array<...>>::zeroed`, with adjacent fallout at `runner.cpp:1073` (`raw_ptr_add` deduction) and downstream iterator/variant return-shape families.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-26-2-1775532846 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
84. `Leaf 4.15.4.3.3.3.3.3.27.27.1` is complete.
   - plan/scope check: runtime+transpiler fix leaf stayed under the <1000 LOC threshold and required no additional decomposition.
   - implemented shared runtime `MaybeUninit::zeroed()` in `include/rusty/maybe_uninit.hpp`, and added transpiler regression `test_leaf41543333333327271_zeroed_uses_expected_type_for_maybe_uninit_receiver` to keep owner-type recovery for `MaybeUninit::zeroed().assume_init()` in array-backed contexts.
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-27-1-1775530696 --keep-work-dirs`) removed the deterministic `runner.cpp:1469` missing-`zeroed` head.
   - new deterministic first hard error now starts at `runner.cpp:1073` (`raw_ptr_add` template deduction failure), with adjacent fallout at `runner.cpp:1078` (`into_iter` member assumption on iterator-like adapters/ranges).
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327271_zeroed_uses_expected_type_for_maybe_uninit_receiver -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-27-1-1775530696 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fix remains shared and context-gated in core runtime/transpiler surfaces, with no crate-specific scripts or post-generation rewrites.
85. `Leaf 4.15.4.3.3.3.3.3.27.27.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-27-2-1775530872 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-27-2-1775530872/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1073`: `raw_ptr_add` template-argument deduction failure (`raw_ptr_add(int*, size_t)` call shape), with adjacent fallout at `runner.cpp:1078` (`into_iter` member assumption on iterator-like adapters/ranges) and downstream range-bound visit/return-shape families.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-27-2-1775530872 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
86. `Leaf 4.15.4.3.3.3.3.3.27.28.1` is complete.
   - plan/scope check: transpiler/runtime implementation remained focused and under the <1000 LOC threshold, so no additional decomposition was required.
   - implemented generic free-function pointer-call template-argument recovery in `transpiler/src/codegen.rs`: collected function type-generic metadata and applied shape-gated template arg recovery for pointer-typed helper calls so `raw_ptr_add` call sites emit explicit args when deduction would otherwise fail.
   - implemented shared iterator-adaptation normalization in runtime headers:
     - `include/rusty/slice.hpp`: added move-preserving `.into_iter()` to `map_next_iter`, `enumerate_next_iter`, `rev_next_iter`, and `take_next_iter`.
     - `include/rusty/array.hpp`: added `.into_iter()` to `range`, `range_inclusive`, and `range_from`, plus Rust-style `range_inclusive::next()` / `count()` helpers for transpiled iterator surfaces.
   - updated/added fixture-agnostic regressions in `transpiler/src/codegen.rs`:
     - `test_leaf41543333333327151_as_mut_ptr_argument_adapts_to_expected_pointer_shape`
     - `test_leaf41543333333327281_nongeneric_pointer_call_does_not_gain_template_args`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-28-1-1775533001 --keep-work-dirs`) removed the deterministic `runner.cpp:1073` (`raw_ptr_add` deduction) and adjacent `runner.cpp:1078` (`into_iter`) head family.
   - new deterministic first hard error now starts at `runner.cpp:1104` (raw pointer `ptr.write(...)` member-call shape), with adjacent fallout at `runner.cpp:1080/1081` (`std::optional` emitted with Rust `is_some`/`unwrap` surface) and downstream variant/copy cascades.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327151_as_mut_ptr_argument_adapts_to_expected_pointer_shape -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327281_nongeneric_pointer_call_does_not_gain_template_args -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-28-1-1775533001 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes remain shared and AST/type-context-gated, avoid crate-specific scripts/post-generation rewrites, and preserve deterministic first-head artifact capture.
87. `Leaf 4.15.4.3.3.3.3.3.27.28.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-28-2-1775532180 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-28-2-1775532180/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:1104`: raw pointer value emitted as `ptr.write(...)` member call, with adjacent fallout at `runner.cpp:1080/1081` (`std::optional` emitted with Rust `is_some`/`unwrap` surface) and downstream variant/copy/lifetime cascades.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-28-2-1775532180 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
88. `Leaf 4.15.4.3.3.3.3.3.27.29.1` is complete.
   - plan/scope check: transpiler-only implementation stayed under the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - extended local-type recovery from free-function return metadata so untyped locals initialized from pointer helpers keep raw-pointer type context (which routes `ptr.write(...)` through existing pointer helper lowering to `rusty::ptr::write(...)`).
     - hardened `if let` / `while let` option-pattern lowering with a tri-state surface:
       - known `std::optional` → `has_value()` / `value()`
       - known `rusty::Option` → `is_some()` / `unwrap()`
       - unresolved generic optional-like flows → `rusty::detail::option_has_value(...)` / `rusty::detail::option_take_value(...)`
     - added helper-based rvalue-safe binding in statement-level `if let` when using `option_take_value(...)` to avoid non-const lvalue-reference binding failures on temporary `next()` results.
   - added/updated fixture-agnostic regressions:
     - `test_leaf41543333333327291_local_pointer_helper_write_lowers_to_runtime_helper`
     - `test_leaf41543333333327291_next_optional_like_methods_lower_to_optional_surface`
     - `test_leaf41543333333327291_next_optional_like_methods_lower_for_type_param_iterator_locals`
     - `test_leaf41543333333327291_while_let_iter_next_uses_optional_surface_for_type_param_locals`
     - `test_leaf41543333333327291_if_let_expr_iter_next_uses_optional_surface_for_type_param_locals`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-29-1-1775534183 --keep-work-dirs`) removed the prior adjacent `runner.cpp:1080/1081` optional-surface first-family diagnostics.
   - new deterministic first hard error now starts at `/home/shuai/git/rusty-cpp/include/rusty/ptr.hpp:119` (move-only copy fallback in pointer read/write family), with canonical artifacts at `/tmp/rusty-parity-27-29-1-1775534183/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327291 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-29-1-1775534183 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes remain shared and context-gated, avoid crate-specific scripts/post-generation rewrites, and preserve deterministic first-head artifact capture.
89. `Leaf 4.15.4.3.3.3.3.3.27.29.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-29-2-1775534455 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-29-2-1775534455/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `/home/shuai/git/rusty-cpp/include/rusty/ptr.hpp:119` (`rusty::ptr::read` returns by copy for a move-only payload, surfaced via `ArrayVecImpl::pop`), with immediate adjacent fallout at `/home/shuai/git/rusty-cpp/include/rusty/mem.hpp:86` (`rusty::mem::replace` copy-assignment surface on move-only `ArrayVec<...Bump...>`), followed by downstream dependent variant/slice/template diagnostics.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-29-2-1775534455 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
90. `Leaf 4.15.4.3.3.3.3.3.27.30.1` is complete.
   - plan/scope check: shared runtime + regression-test implementation stayed under the <1000 LOC threshold and required no further decomposition.
   - implemented shared runtime move-only transfer fixes:
     - `include/rusty/ptr.hpp`: `rusty::ptr::read(const T*)` now models Rust-like move-out semantics via `std::move(*const_cast<T*>(src))` instead of copy fallback.
     - `include/rusty/mem.hpp`: `rusty::mem::replace(T&, U&&)` now performs move-out + destroy + placement reconstruction, removing copy/move-assignment requirements on destination payload types.
   - added fixture-agnostic runtime regressions in `transpiler/tests/runtime_move_semantics.rs`:
     - `test_ptr_read_const_pointer_supports_move_only_payloads`
     - `test_mem_replace_supports_non_assignable_move_only_payloads`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-30-1-20260407-001026 --keep-work-dirs`) removed the deterministic 27.29.2 runtime head family (`ptr::read` copy + `mem::replace` copy-assignment).
   - new deterministic first hard error now starts at `runner.cpp:968` (`std::visit` return-type mismatch across range-bound alternatives), with immediate adjacent fallout at `runner.cpp:973` (slice pointer shape mismatch), and additional downstream move-only/runtime surfaces at `/home/shuai/git/rusty-cpp/include/rusty/result.hpp:72` + `/home/shuai/git/rusty-cpp/include/rusty/ptr.hpp:132`.
   - canonical artifacts: `/tmp/rusty-parity-27-30-1-20260407-001026/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-30-1-20260407-001026 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes remained in shared runtime surfaces, avoided crate-specific scripts/post-generation rewrites, and preserved deterministic first-head artifact capture.
91. `Leaf 4.15.4.3.3.3.3.3.27.30.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-30-2-20260407-001457 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-30-2-20260407-001457/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `runner.cpp:968`: `ArrayVec::drain` bound-visitor `std::visit` alternatives do not unify to one return type; immediate adjacent fallout appears at `runner.cpp:973` where generated `const auto* range_slice` assumes pointer shape while `rusty::slice((*this), start, end)` yields a span/slice value.
   - downstream dependent families remain (for example `/home/shuai/git/rusty-cpp/include/rusty/result.hpp:72` default-construction of non-default-constructible `Err` payload and `/home/shuai/git/rusty-cpp/include/rusty/ptr.hpp:132` move-assignment requirement in `ptr::write`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-30-2-20260407-001457 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
92. `Leaf 4.15.4.3.3.3.3.3.27.31.1` is complete.
   - plan/scope check: transpiler-only implementation with focused regressions stayed under the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - untyped `start_bound()` / `end_bound()` match-expression lowering now forces `std::visit<size_t>(...)` return shape to prevent mixed integral alternative return mismatches in bound visitors.
     - typed raw-pointer locals initialized from slice-range references now materialize local slice backing storage first, then bind pointer locals to backing address (avoids direct span-to-pointer assignment assumptions while preserving pointer call-sites).
   - added/updated fixture-agnostic regressions:
     - `test_leaf415433333333311_bound_match_without_expected_type_forces_size_t_visit_return`
     - `test_leaf41543333331_typed_raw_pointer_local_does_not_emit_duplicate_const` (extended to assert backing storage materialization + pointer binding shape)
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-31-1-20260407-002720 --keep-work-dirs`) removed the deterministic `runner.cpp:968`/`runner.cpp:973` drain-family head.
   - new deterministic first hard error now starts at `/home/shuai/git/rusty-cpp/include/rusty/result.hpp:72` (`Result::Err` default-constructs non-default-constructible `E` payload), with immediate adjacent fallout at `runner.cpp:1013` (move-only array copy surface) and `/home/shuai/git/rusty-cpp/include/rusty/ptr.hpp:132` (`ptr::write` assignment requirement on move-only payloads).
   - canonical artifacts: `/tmp/rusty-parity-27-31-1-20260407-002720/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf415433333333311_bound_match_without_expected_type_forces_size_t_visit_return -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333331_typed_raw_pointer_local_does_not_emit_duplicate_const -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-31-1-20260407-002720 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes are shared and type-gated in core lowering paths, avoid crate-specific scripts/post-generation rewrites, and preserve deterministic first-head artifact capture.
93. `Leaf 4.15.4.3.3.3.3.3.27.31.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-31-2-verify-20260407-010640 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-31-2-verify-20260407-010640/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error now starts at `/home/shuai/git/rusty-cpp/include/rusty/result.hpp:72`: `Result::Err` still default-constructs non-default-constructible move-only payloads (surfacing as `no matching function for call to 'arrayvec::ArrayVec<int, 2>::ArrayVec()'`), with immediate adjacent fallout at `runner.cpp:1013` (move-only array copy surface) and `/home/shuai/git/rusty-cpp/include/rusty/ptr.hpp:132` (`ptr::write` assignment requirement on move-only payloads).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-31-2-verify-20260407-010640 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
94. `Leaf 4.15.4.3.3.3.3.3.27.32.1` is complete.
   - plan/scope check: shared runtime + transpiler + regression updates stayed below the <1000 LOC threshold and required no further decomposition.
   - implemented shared runtime fixes:
     - `include/rusty/result.hpp`: `Result<T, E>::Ok/Err` now construct active storage directly via an uninitialized constructor-tag path instead of routing through `Result()`, removing implicit `E()` default-construction requirements for `Err` payloads.
     - `include/rusty/ptr.hpp`: `rusty::ptr::write(T*, U&&)` now uses `std::construct_at` write semantics instead of assignment, removing move-assignment requirements for move-only/non-assignable payloads.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`: immutable-local consumption analysis now treats by-value `return x;`, by-value `break x`, and tail value expressions as consuming surfaces so move-out locals are not emitted as `const auto`.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf415433333333321_tail_return_consumes_local_binding`
     - `codegen::tests::test_leaf415433333333321_explicit_return_consumes_local_binding`
     - `runtime_move_semantics::test_ptr_write_supports_non_assignable_move_only_payloads`
     - `runtime_move_semantics::test_result_err_supports_non_default_constructible_error_payloads`
   - single-crate reprobe (`tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-32-1-20260407-013012 --keep-work-dirs`) removed the deterministic `result.hpp:72` + `runner.cpp:1013` + `ptr.hpp:132` move-only family.
   - new deterministic first hard error now starts at `runner.cpp:1342` (`raw_ptr_add` assumes wrapper-pointer `.cast` surface while receiving raw pointer pointees), with immediate adjacent fallout at `runner.cpp:1327` (scope-exit guard lambda invocation/value-category mismatch against `auto&` first-parameter expectations).
   - canonical artifacts: `/tmp/rusty-parity-27-32-1-20260407-013012/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf415433333333321_tail_return_consumes_local_binding -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf415433333333321_explicit_return_consumes_local_binding -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-32-1-20260407-013012 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): fixes remained in shared runtime/transpiler surfaces, avoided crate-specific scripts/post-generation rewrites, and preserved deterministic first-head artifact capture.
95. `Leaf 4.15.4.3.3.3.3.3.27.32.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-32-2-20260407-013518 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-32-2-20260407-013518/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:1342`: `raw_ptr_add` still assumes wrapper-pointer `.cast<uint8_t>()` surface while receiving raw pointer pointees; immediate adjacent fallout remains at `runner.cpp:1327` where scope-exit guard callback invocation shape cannot bind the lambda’s first `auto&` parameter from the emitted `&this->data` argument.
   - downstream dependent families remain (for example `runner.cpp:1593` string-view `.hash` surface mismatch).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-32-2-20260407-013518 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
96. `Leaf 4.15.4.3.3.3.3.3.27.33.1` is complete.
   - plan/scope check: shared transpiler-only updates (raw-pointer lowering/inference + closure parameter lowering) stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler changes in `transpiler/src/codegen.rs`:
     - added raw-pointer method result inference for `.cast::<T>()` and `.wrapping_add/.wrapping_sub(...)` chains.
     - lowered raw-pointer `.cast::<T>()` to mutability-aware `reinterpret_cast<...>(...)` and `.wrapping_add/.wrapping_sub(...)` to `rusty::ptr::add/sub(...)`.
     - adjusted closure `&pattern` parameter lowering to use forwarding params plus deref-prelude bindings, preventing scope-exit callback call-shape mismatches.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327331_closure_ref_pattern_expr_body_uses_deref_prelude`
     - `codegen::tests::test_leaf41543333333327331_raw_ptr_cast_wrapping_add_cast_chain_lowers_generically`
     - `codegen::tests::test_leaf41543333333327331_const_raw_ptr_cast_wrapping_add_preserves_constness`
     - updated `codegen::tests::test_leaf41543333332_closure_ref_pattern_param_emits_single_auto_ref`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327331 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333332_closure_ref_pattern_param_emits_single_auto_ref -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-33-1-20260407-005858 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.32.2 head family is collapsed: `runner.cpp:1342` now emits raw-pointer-safe cast/add lowering and scope-exit callback invocation at `runner.cpp:1328` no longer fails to bind lambda parameters.
   - new deterministic first hard error starts at `runner.cpp:1594` (`std::string_view` has no member `hash`), with downstream dependent families in `slice.hpp`/span equality diagnostics.
   - canonical artifacts: `/tmp/rusty-parity-27-33-1-20260407-005858/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed in shared transpiler lowering/type-inference surfaces, added fixture-agnostic regressions, and avoided crate-specific rewrites/scripts.
97. `Leaf 4.15.4.3.3.3.3.3.27.33.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-33-2-20260407-010206 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-33-2-20260407-010206/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error is now `runner.cpp:1594`: `std::string_view` hash-call mismatch (`(*(*this)).hash(h)` has no `.hash` surface on `std::string_view`), with immediate adjacent fallout at `/home/shuai/git/rusty-cpp/include/rusty/slice.hpp:81` (`ClonedIter::next` copy-constructs move-only `rusty::Vec<int>`) and downstream dependent span-equality payload-shape diagnostics (for example `/usr/include/c++/14/bits/stl_algobase.h:1196`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-33-2-20260407-010206 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
98. `Leaf 4.15.4.3.3.3.3.3.27.34.1` is complete.
   - plan/scope check: shared transpiler/runtime updates (hash-call lowering + fallback helper semantics + slice cloned-iterator clone semantics) stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared fixes:
     - `transpiler/src/codegen.rs`: lowered `.hash(state)` method-call surfaces to `rusty::hash::hash(receiver, state)` so string-view-backed receivers do not emit invalid `.hash(...)` member calls.
     - `transpiler/src/codegen.rs`: upgraded runtime fallback `rusty::hash::hash` helper from no-op to generic dispatch (`value.hash(state)` when available, otherwise `std::hash` and byte-hash combine fallback).
     - `include/rusty/slice.hpp`: `slice_iter::Iter::ClonedIter` now uses `clone()` when available (copy fallback only for copy-constructible values), removing move-only cloneable payload copy-constructor requirements.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327341_string_backed_hash_method_lowers_to_runtime_helper`
     - `runtime_move_semantics::test_slice_cloned_iter_supports_move_only_cloneable_payloads`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327341_string_backed_hash_method_lowers_to_runtime_helper -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics test_slice_cloned_iter_supports_move_only_cloneable_payloads -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-34-1-20260407-011235 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.33.2 head family is collapsed: `runner.cpp:1594` string-view `.hash` member-call and adjacent `slice.hpp:81` cloned-iterator copy head are gone.
   - new deterministic first hard error starts at `runner.cpp:3444` (span equality payload-shape mismatch; `std::span<rusty::Vec<int>>` compared against `std::span<const rusty::Vec<rusty::Vec<int>>>`), with dependent comparator diagnostics rooted at `/usr/include/c++/14/bits/stl_algobase.h:1196`.
   - canonical artifacts: `/tmp/rusty-parity-27-34-1-20260407-011235/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed in shared transpiler/runtime surfaces with fixture-agnostic regressions, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
99. `Leaf 4.15.4.3.3.3.3.3.27.34.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-34-2-20260407-011501 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-34-2-20260407-011501/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:3444` (span assertion equality payload-shape mismatch between `std::span<rusty::Vec<int>>` and `std::span<const rusty::Vec<rusty::Vec<int>>>`), with immediate comparator hard diagnostic at `/usr/include/c++/14/bits/stl_algobase.h:1196` and downstream dependent mismatch at the same equality surface (`rusty::Vec<unsigned char>` vs `rusty::Vec<int>`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-34-2-20260407-011501 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
100. `Leaf 4.15.4.3.3.3.3.3.27.35.1` is complete.
   - plan/scope check: shared runtime-pointer adaptation plus focused regression coverage stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared runtime fix in `include/rusty/array.hpp`:
     - `rusty::as_ptr(const T&)` and `rusty::as_mut_ptr(T&)` now prefer pointer-valued `.begin()` fallback when `.as_ptr()`/`.data()` are unavailable, so `slice_full` over container-like wrappers materializes element-pointer spans instead of wrapper-address spans.
   - added fixture-agnostic regression:
     - `runtime_move_semantics::test_slice_full_vec_of_vec_uses_element_pointer_not_container_pointer`
   - verification:
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics test_slice_full_vec_of_vec_uses_element_pointer_not_container_pointer -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-35-1-20260407-012351 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.34.2 head family is collapsed: `runner.cpp:3444` span payload-shape mismatch is gone.
   - new deterministic first hard error now starts at `runner.cpp:4350`, with immediate comparator failure at `/usr/include/c++/14/bits/stl_algobase.h:1196` (`rusty::Vec<unsigned char>` vs `rusty::Vec<int>` assertion payload mismatch).
   - canonical artifacts: `/tmp/rusty-parity-27-35-1-20260407-012351/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed in shared runtime surfaces with fixture-agnostic coverage, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
101. `Leaf 4.15.4.3.3.3.3.3.27.35.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-35-2-20260407-012749 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-35-2-20260407-012749/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error is now `runner.cpp:4350`: assertion tuple equality compares `std::span<rusty::Vec<unsigned char>>` with `std::array<rusty::Vec<int>, 1>` payloads, with immediate comparator hard diagnostic at `/usr/include/c++/14/bits/stl_algobase.h:1196` (`operator==` mismatch between `rusty::Vec<unsigned char>` and `rusty::Vec<int>`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-35-2-20260407-012749 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
102. `Leaf 4.15.4.3.3.3.3.3.27.36.1` is complete.
   - plan/scope check: shared transpiler-only tuple expected-type propagation/inference plus focused regression coverage stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - binding-tuple assertion lowering now derives per-element expected types from peer tuple expressions for array/repeat RHS values when global tuple expected type is unresolved.
     - iterator item-type inference now handles slice/index/call surfaces (including `slice_full(...)`) for peer-context element recovery, allowing tuple assertion RHS array literals to inherit `Vec<u8>` element context.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327361_tuple_assertion_rhs_into_vec_box_new_uses_peer_u8_hint`
     - `codegen::tests::test_leaf41543333333327361_tuple_assertion_rhs_into_vec_box_new_non_u8_peer_unchanged`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327361 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-36-1-20260407-014027 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.35.2 head family is collapsed: `runner.cpp:4350` tuple assertion mismatch (`Vec<uint8_t>` vs `Vec<int>`) is gone.
   - new deterministic first hard error now starts at `runner.cpp:4145` (`static_cast<auto>(Z{})` invalid in `array_repeat` emission), with immediate adjacent repeats at `runner.cpp:4201` and `runner.cpp:4220`.
   - canonical artifacts: `/tmp/rusty-parity-27-36-1-20260407-014027/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed in shared transpiler inference/lowering surfaces with fixture-agnostic regressions, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
103. `Leaf 4.15.4.3.3.3.3.3.27.36.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-36-2-20260407-014322 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-36-2-20260407-014322/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first hard error remains at `runner.cpp:4145`: `rusty::array_repeat(static_cast<auto>(Z{}), 5)` emits invalid `static_cast<auto>` for non-primitive repeat seed values in assertion scaffolding, with immediate adjacent repeats at `runner.cpp:4201` and `runner.cpp:4220`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-36-2-20260407-014322 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
104. `Leaf 4.15.4.3.3.3.3.3.27.37.1` is complete.
   - plan/scope check: transpiler-only repeat-seed cast gating plus focused fixture-agnostic regressions stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - added repeat-seed cast gating helpers so repeat-seed `static_cast<...>` is emitted only for scalar primitive C++ targets, and skipped for `auto`/TODO/non-primitive targets.
     - applied the shared cast helper to `emit_repeat_expr_with_element_hint`, `emit_repeat_expr_with_fixed_array_hint`, and `try_emit_arrayvec_from_repeat_with_fixed_array_arg`.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327371_repeat_seed_u8_slice_hint_preserves_uint8_cast`
     - `codegen::tests::test_leaf41543333333327371_repeat_seed_nonprimitive_slice_hint_avoids_cast`
     - `codegen::tests::test_leaf41543333333327371_repeat_seed_inferred_slice_hint_avoids_auto_cast`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327371 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-37-1-20260407-015432 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.36.2 compile-head family is collapsed: Stage D build now passes and `runner.cpp` no longer emits `static_cast<auto>(...)` repeat-seed casts.
   - new deterministic first failure shifts to Stage E runtime: `array_clone_from` fails with `Called unwrap on None` (captured from `/tmp/rusty-parity-27-37-1-20260407-015432/arrayvec/run.log`).
   - canonical artifacts: `/tmp/rusty-parity-27-37-1-20260407-015432/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed in shared transpiler coercion/lowering surfaces with fixture-agnostic regressions, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
105. `Leaf 4.15.4.3.3.3.3.3.27.37.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-37-2-20260407-020850 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-37-2-20260407-020850/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first failure has shifted to Stage E runtime: `array_clone_from FAILED: Called unwrap on None` (`run.log:4`), rooted at `runner.cpp:3433` (`u.clone_from(v)`), with active clone path at `runner.cpp:1157-1164` (`ArrayVec::clone_from` using `rusty::clone_from_slice` + `slice_from` + `extend_from_slice`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-37-2-20260407-020850 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
106. `Leaf 4.15.4.3.3.3.3.3.27.38.1` is complete.
   - plan/scope check: transpiler-only statement `if let` single-evaluation lowering plus focused fixture-agnostic regressions stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - `emit_if_let` now single-evaluates side-effectful scrutinees via C++17 if-init storage (`if (auto&& _iflet_scrutinee = ...; cond)`), eliminating duplicate scrutinee evaluation in statement `if let` lowering.
     - `emit_if_let_body` now supports optional if-init emission while preserving explicit `as_mut` binding shape (`auto& val = *...`) in Option/Result reference surfaces.
     - path/field scrutinee lowering remains unchanged unless side-effectful shape detection requires storage, preserving no-blanket-rewrite discipline.
   - added fixture-agnostic regressions:
     - updated `codegen::tests::test_leaf41543333333327291_if_let_expr_iter_next_uses_optional_surface_for_type_param_locals`
     - added `codegen::tests::test_leaf41543333333327381_else_if_let_iter_next_uses_single_eval_storage`
     - updated `codegen::tests::test_leaf411_result_as_mut_if_let_binds_referenced_inner_value`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327291_if_let_expr_iter_next_uses_optional_surface_for_type_param_locals -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327381_else_if_let_iter_next_uses_single_eval_storage -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-38-1-20260407-023010 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.37.2 runtime head family is collapsed: `array_clone_from` now passes.
   - new deterministic first failure shifts to Stage E hard runtime abort after `char_test_encode_utf8 PASSED`; next scheduled test is `char_test_encode_utf8_oob` (`runner.cpp:4616`), with panic/abort surface in test body around `runner.cpp:661-665` (`matches!` assertion checks over `encode_utf8(...)`).
   - canonical artifacts: `/tmp/rusty-parity-27-38-1-20260407-023010/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed in shared AST-aware lowering surfaces with fixture-agnostic regressions, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
107. `Leaf 4.15.4.3.3.3.3.3.27.38.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-38-2b-20260407-025507 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-38-2b-20260407-025507/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first failure remains Stage E runtime: output still stops immediately after `char_test_encode_utf8 PASSED`; next scheduled runner test is `char_test_encode_utf8_oob` (`runner.cpp:4616`), which calls `char_::test_encode_utf8_oob` (`runner.cpp:636`) where active assertion/panic surfaces are at `runner.cpp:661-665` (`matches!` checks over `encode_utf8(...)`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-38-2b-20260407-025507 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
108. `Leaf 4.15.4.3.3.3.3.3.27.39.1` is complete.
   - plan/scope check: runtime-only `for_in` lifetime fix plus focused fixture-agnostic regression stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared runtime fix in `include/rusty/slice.hpp`:
     - added `detail::preserve_for_in_range` so `rusty::for_in` preserves begin/end-capable rvalue ranges by value while retaining lvalue reference behavior.
     - updated `for_in` branch ordering to prefer begin/end range iteration before `iter(...)` adaptation, preventing temporary-container lifetime loss (for example `rusty::for_in(rusty::zip(...))`).
   - added fixture-agnostic regression:
     - `runtime_move_semantics::test_for_in_zip_temporary_preserves_rvalue_storage_lifetime`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_for_in_zip_temporary_preserves_rvalue_storage_lifetime -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-39-1-20260407-032937 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.38.2 runtime head family is collapsed: Stage E now proceeds through `char_test_encode_utf8_oob PASSED`.
   - new deterministic first failure shifts to Stage E runner semantics: `deny_max_capacity_arrayvec_value FAILED: ArrayVec: largest supported capacity is u32::MAX` (`run.log:6`), rooted at panic-expected test body `runner.cpp:4290-4297` (libtest metadata skipped) with fail accounting in runner dispatch at `runner.cpp:4619-4621`.
   - canonical artifacts: `/tmp/rusty-parity-27-39-1-20260407-032937/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed in shared runtime iteration/lifetime surfaces with fixture-agnostic regression coverage, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
109. `Leaf 4.15.4.3.3.3.3.3.27.39.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-39-2-20260407-034412 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-39-2-20260407-034412/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first failure remains Stage E runtime/runner semantics: `deny_max_capacity_arrayvec_value FAILED: ArrayVec: largest supported capacity is u32::MAX` (`run.log:6`), rooted at panic-expected test body `runner.cpp:4290-4297` with fail-accounting catch branch at `runner.cpp:4619-4621`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-39-2-20260407-034412 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
110. `Leaf 4.15.4.3.3.3.3.3.27.40.1` is complete.
   - plan/scope check: shared metadata threading and parity-runner classification changes stayed well below the <1000 LOC guardrail and did not require further decomposition.
   - implemented shared transpiler/parity-runner fixes (no crate-specific scripts):
     - `transpiler/src/codegen.rs` now extracts libtest `should_panic` state from skipped `test::TestDescAndFn` metadata consts and emits wrapper metadata comments carrying marker + panic expectation.
     - `transpiler/src/main.rs` now parses wrapper metadata into structured runner entries and adds isolated single-test execution (`--rusty-single-test`) so panic-expected tests are classified by process outcome (`non-zero => expected panic pass`, `zero => expected panic fail`) even when panic paths abort instead of throwing.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327401_libtest_wrapper_metadata_marks_should_panic`
     - `tests::test_collect_rusty_test_entries_from_cppm_reads_should_panic_metadata`
     - `parity_test_verification::test_stop_after_run_treats_should_panic_tests_as_expected_passes`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327401_libtest_wrapper_metadata_marks_should_panic -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_collect_rusty_test_entries_from_cppm_reads_should_panic_metadata -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_stop_after_run_treats_should_panic_tests_as_expected_passes -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): fix stayed in shared transpiler/parity-runner surfaces, remained metadata/shape-gated, and introduced no crate-specific rewrites/scripts.
111. `Leaf 4.15.4.3.3.3.3.3.27.40.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-40-2-rerun-20260407-042145 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-40-2-rerun-20260407-042145/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first failure shifts to a new Stage E runtime abort family: after `test_arraystring_const_constructible PASSED` (`run.log:7`), execution aborts with `ArrayVec: largest supported capacity is u32::MAX` (`run.log:9`) and `Aborted` (`run.log:10`), entering `test_arraystring_zero_filled_has_some_sanity_checks` from runner dispatch (`runner.cpp:4749`) and hitting the capacity guard panic path in `ArrayString::zero_filled()` (`runner.cpp:1486`).
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-40-2-rerun-20260407-042145 --keep-work-dirs`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
112. `Leaf 4.15.4.3.3.3.3.3.27.41.1` is complete.
   - plan/scope check: runtime helper + literal lowering hardening with focused regressions stayed well below the <1000 LOC threshold and did not require further decomposition.
   - implemented shared runtime/transpiler fixes (no crate-specific scripts):
     - `include/rusty/rusty.hpp`: `rusty::to_string_view` now prefers deref-style string surfaces before `.as_str()` to avoid recursive `as_str() -> to_string_view` loops in generated string-like wrappers.
     - `transpiler/src/codegen.rs`: embedded-NUL Rust string literals now lower to sized `std::string_view("...", N)` so Rust `&str` byte-length semantics are preserved in assertion/equality paths.
   - added fixture-agnostic regressions:
     - `runtime_move_semantics::test_to_string_view_prefers_deref_over_recursive_as_str`
     - `codegen::tests::test_leaf41543333333327411_embedded_nul_string_literal_uses_sized_string_view`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_to_string_view_prefers_deref_over_recursive_as_str -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327411_embedded_nul_string_literal_uses_sized_string_view -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-41-1b-20260407-050904 --keep-work-dirs`
   - single-crate reprobe confirms the deterministic 27.40.2 head family is collapsed: Stage E now proceeds through `test_arraystring_zero_filled_has_some_sanity_checks PASSED`.
   - new deterministic first failure shifts to Stage E runtime abort at `test_compact_size`: `run.log` reaches `test_capacity_left PASSED` (`run.log:10`) before abort, and single-wrapper repro (`./runner --rusty-single-test rusty_test_test_compact_size`) aborts with stack at `test_compact_size` (`runner.cpp:2774-2797`).
   - canonical artifacts: `/tmp/rusty-parity-27-41-1b-20260407-050904/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed in shared runtime/transpiler surfaces, were shape-gated, and introduced no crate-specific rewrites/scripts.
113. `Leaf 4.15.4.3.3.3.3.3.27.41.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-41-2-20260407-052350 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-41-2-20260407-052350/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first failure head remains the new Stage E runtime abort family: execution reaches `test_capacity_left PASSED` (`run.log:10`) and then aborts with `ArrayVec: largest supported capacity is u32::MAX` (`run.log:12`) / `Aborted` (`run.log:13`) before `test_compact_size` can be reported by the main runner (`runner.cpp:4758` dispatch).
   - single-wrapper repro from canonical artifacts confirms the same head: `./runner --rusty-single-test rusty_test_test_compact_size` exits with `134` (abort), with active test body at `runner.cpp:2776-2797` and capacity-guard panic sites at `runner.cpp:810`, `runner.cpp:1425`, and `runner.cpp:1486`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-41-2-20260407-052350 --keep-work-dirs`
     - `cd /tmp/rusty-parity-matrix-27-41-2-20260407-052350/arrayvec && ./runner --rusty-single-test rusty_test_test_compact_size`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
114. `Leaf 4.15.4.3.3.3.3.3.27.42.1` is complete.
   - plan/scope check: runtime/transpiler updates stayed well below the <1000 LOC threshold and did not require further decomposition.
   - implemented shared runtime/transpiler fixes (no crate-specific scripts):
     - `include/rusty/mem.hpp`: added shared forgotten-address runtime APIs (`mark_forgotten_address` / `consume_forgotten_address`) to avoid per-instance drop-skip layout fields, and updated `rusty::mem::size_of<T>()` to use Rust-layout sizing for fixed-capacity transpiled containers exposing `CAPACITY` + `len_field` + `xs` (`sizeof(len_field) + tuple_size(xs) * sizeof(element)`), restoring `CAP=0` parity.
     - `transpiler/src/codegen.rs`: removed emitted `rusty_forget_flag_` member storage and rewired generated Drop/move glue to runtime APIs (`other.rusty_mark_forgotten()`, destructor guard via `rusty::mem::consume_forgotten_address(this)`, `rusty_mark_forgotten()` helper via `rusty::mem::mark_forgotten_address(this)`).
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf4154_drop_trait_impl_emits_destructor`
     - `codegen::tests::test_leaf4154_drop_struct_literal_uses_constructor_call`
     - `runtime_move_semantics::test_mem_size_of_uses_rust_layout_for_arrayvec_like_zero_capacity_storage`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf4154_drop_trait_impl_emits_destructor -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf4154_drop_struct_literal_uses_constructor_call -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_mem_size_of_uses_rust_layout_for_arrayvec_like_zero_capacity_storage -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-42-1c-20260407-081541 --keep-work-dirs`
   - single-crate repro confirms the deterministic 27.41.2 head family is collapsed: Stage E now proceeds through `test_compact_size PASSED` and `test_default PASSED`.
   - new deterministic first failure shifts to Stage E runtime assertion abort in `test_drain`: `run.log` now ends after `test_default PASSED` (`run.log:12`), and single-wrapper repro (`./runner --rusty-single-test rusty_test_test_drain`) aborts with stack in the drain assertion path (`runner.cpp:2813-2841`, `assert_failed` at `runner.cpp:2829`/adjacent assertion block).
   - canonical artifacts: `/tmp/rusty-parity-27-42-1c-20260407-081541/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed in shared runtime/transpiler surfaces, were shape-gated, and introduced no crate-specific rewrites/scripts.
115. `Leaf 4.15.4.3.3.3.3.3.27.42.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - full seven-crate matrix rerun (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-42-2-rerun-20260407-095210 --keep-work-dirs`) remains deterministic with first failing crate `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-42-2-rerun-20260407-095210/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - deterministic first failure head remains the Stage E runtime abort family: execution reaches `test_default PASSED` (`run.log:12`) and then aborts with `ArrayVec: largest supported capacity is u32::MAX` (`run.log:14`) / `Aborted` (`run.log:15`) before `test_drain` can be reported by the main runner (`runner.cpp:4760` dispatch).
   - single-wrapper repro from canonical artifacts confirms the same head: `./runner --rusty-single-test rusty_test_test_drain` exits with `134` (abort), with active test body at `runner.cpp:2813-2841` and assertion abort site at `runner.cpp:2829`.
   - verification:
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-42-2-rerun-20260407-095210 --keep-work-dirs`
     - `cd /tmp/rusty-parity-matrix-27-42-2-rerun-20260407-095210/arrayvec && ./runner --rusty-single-test rusty_test_test_drain`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
116. `Leaf 4.15.4.3.3.3.3.3.27.43.1` is complete.
   - plan/scope check: targeted transpiler-only implementation stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fix (no crate-specific scripts):
     - `transpiler/src/codegen.rs`: added expected-cast skip guard for local initializers inferred as fallback `*mut u8` from `as_mut_ptr(...)` when pointee inference is unavailable. This preserves element-pointer shape in drain-tail copy lowering and avoids emitting `reinterpret_cast<uint8_t*>(rusty::as_mut_ptr(...))` that corrupts pointer-step semantics.
   - added fixture-agnostic regression:
     - `codegen::tests::test_leaf41543333333327431_drain_tail_copy_keeps_element_pointer_shape`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333333327431_drain_tail_copy_keeps_element_pointer_shape -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-43-1b-20260407-120322 --keep-work-dirs`
   - single-crate repro confirms the deterministic 27.42.2 head family is collapsed: Stage E now proceeds through `test_drain PASSED`, `test_drain_oob PASSED (expected panic)`, and `test_drain_range_inclusive PASSED`.
   - new deterministic first failure shifts later in Stage E: after `test_drain_range_inclusive_oob PASSED (expected panic)`, run aborts with `ArrayVec: largest supported capacity is u32::MAX` and `slice range out of bounds` messages (`run.log:18-21`).
   - canonical artifacts: `/tmp/rusty-parity-27-43-1b-20260407-120322/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): kept the fix in shared transpiler surfaces, used shape-gated logic, and introduced no crate-specific rewrites/scripts.
117. `Leaf 4.15.4.3.3.3.3.3.27.43.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - re-ran full seven-crate matrix (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-43-2b-20260407-080242 --keep-work-dirs`) after 27.43.1: `either`, `tap`, `cfg-if`, and `take_mut` pass; first blocking crate remains `arrayvec` at Stage E.
   - deterministic first failure head has shifted to a Stage E non-terminating runtime family in `char_test_encode_utf8`:
     - runner dispatch proceeds through `allow_max_capacity_arrayvec_type` and `array_clone_from`, then next scheduled test is `rusty_test_char_test_encode_utf8` (`runner.cpp:4724`).
     - active generated loop in that test body uses `rusty::range_inclusive(0, static_cast<uint32_t>(std::numeric_limits<char32_t>::max()))` (`runner.cpp:582`), producing the new blocking runtime surface.
   - timeout-scoped canonical repro from the same artifact:
     - `timeout 60s stdbuf -oL -eL ./runner` exits `124`; `run-timeout.log` contains only:
       - `allow_max_capacity_arrayvec_type PASSED`
       - `array_clone_from PASSED`
     - single-wrapper probes:
       - `rusty_test_allow_max_capacity_arrayvec_type` → `EXIT_CODE=0`
       - `rusty_test_array_clone_from` → `EXIT_CODE=0`
       - `rusty_test_char_test_encode_utf8` → `EXIT_CODE=124`
       - `rusty_test_char_test_encode_utf8_oob` → `EXIT_CODE=0`
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-43-2b-20260407-080242/arrayvec/{baseline.txt,build.log,matrix.log,runner.cpp,run-timeout.log,*.single.log}`.
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
118. `Leaf 4.15.4.3.3.3.3.3.27.44.1` is complete.
   - plan/scope check: targeted transpiler-only implementation stayed well below the <1000 LOC threshold and required no further decomposition.
   - implemented shared transpiler fix (no crate-specific scripts):
     - `transpiler/src/codegen.rs`: `char::MAX`/`std::char::MAX`/`core::char::MAX` lowering now emits Rust Unicode scalar upper bound (`static_cast<char32_t>(0x10FFFF)`) instead of `std::numeric_limits<char32_t>::max()`, restoring Rust-parity loop bounds for char-range surfaces.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327441_std_char_max_uses_unicode_scalar_upper_bound`
     - `codegen::tests::test_leaf41543333333327441_char_max_range_does_not_use_char32_storage_max`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327441 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-44-1-20260407-081914 --keep-work-dirs`
   - single-crate repro confirms the deterministic 27.43.2 head family is collapsed: Stage E now proceeds through `char_test_encode_utf8 PASSED` and `char_test_encode_utf8_oob PASSED`.
   - new deterministic first failure shifts later in Stage E (same downstream family previously observed): after `test_drain_range_inclusive_oob PASSED (expected panic)`, run aborts with `ArrayVec: largest supported capacity is u32::MAX` and `slice range out of bounds` messages.
   - canonical artifacts: `/tmp/rusty-parity-27-44-1-20260407-081914/arrayvec/{baseline.txt,build.log,run.log,matrix.log}`.
   - guardrail check against wrong-approach checklist (§11): kept the fix in shared transpiler surfaces, used shape-gated logic, and introduced no crate-specific rewrites/scripts.
119. `Leaf 4.15.4.3.3.3.3.3.27.44.2` is complete.
   - plan/scope check: rerun/documentation-only leaf with no implementation changes; work stayed well below the <1000 LOC threshold and required no further decomposition.
   - re-ran full seven-crate matrix (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-44-2-20260407-082206 --keep-work-dirs`) after 27.44.1: deterministic first failure remains `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - deterministic first failure head remains the new Stage E runtime abort family: execution reaches `test_drain_range_inclusive_oob PASSED (expected panic)` (`run.log:16`) and then aborts with `ArrayVec: largest supported capacity is u32::MAX` (`run.log:18`) / `Aborted` (`run.log:19`) before `test_drop` can be reported by the main runner (`runner.cpp:4778` dispatch).
   - single-wrapper repro from canonical artifacts confirms the same head:
     - `./runner --rusty-single-test rusty_test_test_drop` exits with `134` (abort)
     - `./runner --rusty-single-test rusty_test_test_drop_in_insert` exits with `134` (abort)
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-44-2-20260407-082206/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp,rusty_test_test_drop.log,rusty_test_test_drop_in_insert.log}`.
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow and introduced no crate-specific rewrites/scripts.
120. `Leaf 4.15.4.3.3.3.3.3.27.45.1` is complete.
   - plan/scope check: targeted transpiler/runtime updates stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes (no crate-specific scripts):
     - `transpiler/src/codegen.rs`: local `Drop` impl merging for local structs, outer `current_struct` restoration across nested local-type emission, and merge-scope restriction to inherent + `Drop` local impls.
     - `transpiler/src/codegen.rs`: drop-enabled move constructors now propagate forgotten-state across chained moves; drop trait destructors are emitted as `noexcept(false)` so panic paths can unwind/catch.
     - `transpiler/src/codegen.rs`: `as_ptr/as_mut_ptr` on `ManuallyDrop` receivers now dispatch through wrapped values (`(*holder).as_ptr()`), fixing `into_inner_unchecked`/drop-path corruption.
     - `include/rusty/mem.hpp`: forgotten-address tracking switched to per-address refcounts; `rusty::mem::drop` no longer enforces a terminate-on-unwind path.
     - `include/rusty/vec.hpp`: move assignment/destructor now allow unwinding (`~Vec() noexcept(false)`), preserving `catch_unwind(drop(vec))` behavior.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf41543333333327451_local_drop_impl_merges_into_local_struct`
     - `codegen::tests::test_leaf41543333333327451_local_non_drop_trait_impl_is_skipped`
     - `codegen::tests::test_leaf41543333333327451_local_drop_unit_struct_has_default_ctor`
     - `codegen::tests::test_leaf41543333333327451_local_impl_keeps_outer_self_context`
     - `codegen::tests::test_leaf41543333333327451_manually_drop_as_ptr_dispatches_to_inner_receiver`
     - `runtime_move_semantics` regressions for forgotten-address refcount and panic-catch drop paths.
   - verification:
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate arrayvec --work-root /tmp/rusty-parity-27-45-1j-20260407-094516 --keep-work-dirs`
   - single-crate repro confirms the 27.44.2 head family is collapsed: Stage E now reports `test_drop PASSED` and `test_drop_in_insert PASSED` (and proceeds through `test_pop_at PASSED`) before the next deterministic failure.
   - canonical artifacts: `/tmp/rusty-parity-27-45-1j-20260407-094516/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes remained in shared transpiler/runtime surfaces and introduced no crate-specific rewrites/scripts.
121. `Leaf 4.15.4.3.3.3.3.3.27.45.2` is complete.
   - plan/scope check: rerun/documentation-focused work plus required regression repair from verification stayed well below the <1000 LOC threshold and required no further decomposition.
   - required transpiler-suite verification exposed one deterministic regression in associated-call expected-type specialization; fixed in shared transpiler logic (`transpiler/src/codegen.rs`) by gating mapped-method reuse on mapped-owner match with expected owner type, preventing invalid member-call rewrites like `rusty::mem::ManuallyDrop<T>::manually_drop_new(...)`.
   - added/updated fixture-agnostic regression:
     - `codegen::tests::test_leaf41543333332_std_mem_manually_drop_new_path_remapped`
   - re-ran full seven-crate matrix (`tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-45-2-20260407-1 --keep-work-dirs`) after 27.45.1: deterministic first failing crate remains `arrayvec` (`total=5`, `pass=4`, `fail=1`).
   - deterministic first failure head has moved to the Stage E `test_retain` family:
     - `run.log` reaches `test_pop_at PASSED` (`run.log:36`) and then aborts (`run.log:38-46`) before another pass/fail marker.
     - next scheduled wrapper in generated dispatch is `rusty_test_test_retain` (`runner.cpp:4840`).
     - single-wrapper repro exits with `134` (abort), and `gdb` batch backtrace points to `rusty::panicking::assert_failed` from `test_retain()`, with active assertion surface at `runner.cpp:3133-3136`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-27-45-2-20260407-1/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-27-45-2-20260407-1 --keep-work-dirs`
     - `cd /tmp/rusty-parity-matrix-27-45-2-20260407-1/arrayvec && timeout 30s ./runner --rusty-single-test rusty_test_test_retain`
     - `cd /tmp/rusty-parity-matrix-27-45-2-20260407-1/arrayvec && gdb -q ./runner -ex 'set args --rusty-single-test rusty_test_test_retain' -ex run -ex bt -ex quit`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head + canonical-artifact workflow, kept fixes in shared transpiler/runtime surfaces, and introduced no crate-specific rewrites/scripts.
122. `Leaf 4.15.4.4.7` is complete.
   - plan/scope check: shared transpiler-only updates plus regression coverage stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes in `transpiler/src/codegen.rs` (no crate-specific scripts):
     - added `std/core::fmt` use-import rewrites so concrete runtime surfaces are not dropped as Rust-only (`std::fmt` namespace alias and `Alignment`/`Formatter`/`Result`/`Arguments`/`Error` mappings).
     - hardened runtime fallback formatter surface: `rusty::fmt::Result` now maps to `rusty::Result<std::tuple<>, rusty::fmt::Error>` with Result-shaped `write_fmt`/`write_char`/`write_str`/debug helper returns.
     - extended `Ok`/`Err` lowering so expected `rusty::fmt::Result` contexts emit qualified constructors (`rusty::fmt::Result::Ok(...)` / `Err(...)`) instead of bare `Ok(...)`.
     - added generic switch-match tuple-literal harmonization: unsuffixed integer literals in tuple arms are cast via peer-expression type hints to avoid inconsistent lambda return tuple deduction.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf415447_fmt_import_rewrites_keep_concrete_runtime_surfaces`
     - `codegen::tests::test_leaf415447_fmt_result_ok_lowering_uses_fmt_result_ctor_surface`
     - `codegen::tests::test_leaf415447_switch_match_tuple_casts_unsuffixed_int_literals_from_peer_type`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf415447 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-4-15-4-4-7-semver-20260407-1 --keep-work-dirs`
   - single-crate semver repro confirms the prior formatter/import head family is collapsed (`Alignment` import now emitted at `runner.cpp:481`; tuple-literal cast harmonization in `display::pad` at `runner.cpp:492`; `rusty::fmt::Result::Ok(std::make_tuple())` emitted at `runner.cpp:501`).
   - new deterministic first Stage D head moves to incomplete-type ordering in `eval` helpers, with first compile blocker at `runner.cpp:621` (`invalid use of incomplete type` for `VersionReq` and related `Version`/`Comparator` surfaces).
   - canonical artifacts: `/tmp/rusty-parity-4-15-4-4-7-semver-20260407-1/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed AST-aware and shape-gated in shared transpiler/runtime surfaces; no crate-specific rewrites/scripts were introduced.
123. `Leaf 4.15.4.4.8` is complete.
   - plan/scope check: targeted transpiler-only ordering changes plus focused regression coverage stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs` (no crate-specific scripts):
     - `order_items_for_emission` now delays function-only inline namespaces (modules containing only `use`/`fn`/nested function-only modules) until after sibling non-module items, ensuring function bodies are emitted after complete sibling type definitions.
     - when inline-module dependency sorting is cyclic/incomplete, fallback now keeps original module order while still applying delayable-module logic (instead of returning early and skipping delay).
     - added shape-gated helpers `module_is_delayable_function_namespace` and `module_contains_fn_items`.
   - added fixture-agnostic regression:
     - `codegen::tests::test_leaf415448_function_only_inline_module_emits_after_sibling_type_definitions`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf415433333335_inline_module_emission_orders_local_use_dependencies_first -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf415448_function_only_inline_module_emits_after_sibling_type_definitions -- --nocapture`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-4-15-4-4-8-semver-20260407-3 --keep-work-dirs`
   - single-crate semver repro confirms the prior `eval` incomplete-type ordering head is collapsed:
     - `eval` function-body namespace now emits after sibling type definitions (`struct Version` at `runner.cpp:1110`; `namespace eval` body at `runner.cpp:1612`, with only forward declarations at `runner.cpp:426`).
     - previous first blockers (`invalid use of incomplete type` from `eval::*` around `runner.cpp:621+`) are absent.
   - new deterministic first Stage D head moved to identifier pointer/memory lowering surfaces:
     - first compile blocker at `runner.cpp:654`: invalid pointer cast shape in `identifier::Identifier::empty`.
     - adjacent deterministic failures at `runner.cpp:664` (`copy_nonoverlapping` `char*` vs `uint8_t*` mismatch and unresolved `mem::transmute`) and `runner.cpp:668/673/694` (`NonNull` equality/cast surface).
   - canonical artifacts: `/tmp/rusty-parity-4-15-4-4-8-semver-20260407-3/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST-shape-gated in emission-ordering logic, avoided crate-specific rewrites/scripts, and preserved deterministic first-head artifact capture.
124. `Leaf 4.15.4.4.9` is complete.
   - plan/scope check: shared transpiler/runtime updates plus focused regression coverage stayed below the <1000 LOC threshold and required no further decomposition.
   - implemented shared fixes (no crate-specific scripts):
     - `transpiler/src/codegen.rs`:
       - unary `!` lowering now emits bitwise `~` for integer-like operands (while retaining logical `!` for bool).
       - integer↔pointer cast lowering now bridges via `std::uintptr_t` to avoid invalid `static_cast` pointer/integer forms.
       - function-local Rust `const` items keep scalar numeric/boolean/floating forms `constexpr` and lower non-scalar forms to `const`, preventing non-constexpr pointer-cast initialization failures in local constant contexts without regressing scalar constexpr emission.
       - repeat expressions with `mem::size_of::<T>()` lengths now materialize fixed arrays so transmute/source-shape flows stay typed.
       - runtime fallback helper surface now includes `rusty::panicking::unreachable_display`.
     - `transpiler/src/types.rs`: added `core::panicking::unreachable_display` → `rusty::panicking::unreachable_display` function-path mapping.
     - `include/rusty/ptr.hpp`: added `NonNull` equality operators and heterogenous `copy`/`copy_nonoverlapping` overloads (equal-element-size constrained) for `char*`/`uint8_t*` interop.
     - `include/rusty/mem.hpp`: added generic equal-size `rusty::mem::transmute<From, To>` byte reinterpretation surface.
   - added fixture-agnostic regressions:
     - `codegen::tests::test_leaf415449_unary_not_integer_uses_bitwise_operator`
     - `codegen::tests::test_leaf415449_unary_not_bool_stays_logical_operator`
     - `codegen::tests::test_leaf415449_integer_to_pointer_cast_uses_uintptr_bridge`
     - `codegen::tests::test_leaf415449_pointer_to_integer_cast_uses_uintptr_bridge`
     - `codegen::tests::test_leaf415449_function_local_const_item_uses_const_storage`
     - `codegen::tests::test_leaf415449_repeat_size_of_len_prefers_fixed_array_materialization`
     - `runtime_move_semantics::{test_ptr_copy_nonoverlapping_supports_char_to_u8_surface,test_ptr_nonnull_supports_equality_comparison,test_mem_transmute_supports_equal_size_byte_reinterpretation}`
     - `types::tests::test_function_path_mapping` updated for `core::panicking::unreachable_display`.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf415449 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler types::tests::test_function_path_mapping -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-4-15-4-4-9-semver-20260407-2 --keep-work-dirs`
   - single-crate semver repro confirms the prior identifier pointer/memory blockers are collapsed:
     - old first blocker (`invalid static_cast` from `bool` to `uint8_t*`) is absent.
     - old `copy_nonoverlapping` `char*`/`uint8_t*` mismatch and missing `mem::transmute` surface are absent as first blockers.
     - old `NonNull` equality/cast head is absent as first blocker.
   - new deterministic first Stage D head moved to `identifier::new_unchecked` control-flow/lambda-return lowering:
     - first compile blocker: inconsistent lambda return type (`Identifier` vs `void`) at `runner.cpp:666`.
     - adjacent follow-ons in the same family: `is_null` on raw pointer and `rotate_right`/`wrapping_sub` intrinsic-method emission at `runner.cpp:708/748/760`.
   - canonical artifacts: `/tmp/rusty-parity-4-15-4-4-9-semver-20260407-2/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared + AST/context-gated and introduced no crate-specific rewrites/scripts.
125. `Leaf 10.2.1` is complete.
   - plan/scope check: the fix stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs` (no crate-specific scripts):
     - added shape-gated for-loop iterable self-shadow detection: when loop-pattern binding names overlap the iterable root path name (for example `for lhs in lhs`), codegen now stabilizes iterable evaluation before introducing loop bindings.
     - added scoped synthetic temp reservation for loop lowering (`_for_iter`/suffix fallback) so generated temp names do not collide with existing local/parameter C++ names.
     - lowered self-shadowing loops to a two-step shape (`auto&& _for_iter = rusty::for_in(...); for (auto&& lhs : _for_iter) { ... }`) while keeping non-shadowing loop lowering unchanged.
   - added focused regressions in `transpiler/src/codegen.rs`:
     - `test_leaf1021_for_loop_iterable_self_shadowing_uses_stable_iter_temp`
     - `test_leaf1021_for_loop_borrowed_iterable_self_shadowing_uses_stable_iter_temp`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1021 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): change is shape-gated and local to for-loop lowering; no blanket for-loop rewrite and no crate-specific patching was introduced.
126. `Leaf 10.2.3` is complete.
   - plan/scope check: parity reprobe + documentation updates stayed well below the <1000 LOC threshold and required no additional decomposition.
   - verification run:
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-2-3-1775846232 --keep-work-dirs`
   - deterministic semver Stage D head after `10.2.1/10.2.2`:
     - first compile blocker starts at `runner.cpp:1060` (`Prerelease::cmp`): generated `auto&& _for_iter = rusty::for_in(lhs); for (auto&& lhs : _for_iter)` fails with `begin/end` not declared in range-for.
     - immediate adjacent fallout in the same block starts at `runner.cpp:1061` (`rhs_shadow1` use-before-deduction from nested match shadowing).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-2-3-1775846232/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): this leaf remained deterministic-first and evidence-capture only; no crate-specific rewrite or blanket lowering was introduced.
127. `Leaf 10.2.4` and `Leaf 10.2.5` are complete.
   - plan/scope check: implementation + focused regression updates stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs` (no crate-specific scripts):
     - self-shadowing `for` loops now stabilize the iterable source expression only, then keep range-for over `rusty::for_in(...)` directly.
     - non-borrowed self-shadowing shape now emits `auto _for_iter = <iterable>; for (auto&& pat : rusty::for_in(_for_iter))`.
     - borrowed self-shadowing shape now emits `auto&& _for_iter = <iterable>; for (auto&& pat : rusty::for_in(rusty::iter(_for_iter)))`.
     - removed prior `for (auto&& pat : _for_iter)` over `rusty::for_in(...)` temp result.
   - focused regressions:
     - updated `test_leaf1021_for_loop_iterable_self_shadowing_uses_stable_iter_temp`
     - updated `test_leaf1021_for_loop_borrowed_iterable_self_shadowing_uses_stable_iter_temp`
     - added `test_leaf1024_self_shadowing_next_iterable_stabilizes_source_before_for_in`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf102 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): changes are shape-gated to self-shadowing loop cases, avoid blanket loop rewrites, and remain shared transpiler behavior.
128. `Leaf 10.2.6` is complete.
   - verification run:
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-2-4-1775846704 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - prior first head at `runner.cpp:1060` (`begin/end` missing on `_for_iter` range-for) is removed.
     - new first deterministic head starts at `runner.cpp:1061` in `Prerelease::cmp`: nested try-style match binding self-reference (`rhs_shadow1` use-before-deduction).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-2-4-1775846704/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): kept deterministic-first workflow with canonical artifacts and no crate-specific patching.
129. `Leaf 10.5.1` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - added mapping-aware try-style pattern binding collection (`collect_pattern_binding_stmts_with_cpp_name_map`) that returns emitted binding statements plus Rust-name→C++-name mapping.
     - `runtime_try_pattern_details` now returns `(condition, binding_stmts, rust_to_cpp_map, unwrap_method)` and uses mapped C++ names for `Pat::Ident` / tuple / struct payload bindings.
     - this aligns try-style binding declarations with name resolution used by emitted arm bodies in shadowing scenarios.
   - focused regressions:
     - `test_leaf1051_try_style_runtime_ident_binding_uses_shadowed_cpp_name`
     - `test_leaf1051_try_style_runtime_tuple_binding_uses_shadowed_cpp_names`
     - `test_leaf1051_try_style_runtime_struct_binding_uses_shadowed_cpp_names`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1051 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): change stays shared and AST-shape-gated, avoids crate-specific rewrites, and does not rely on post-generation text patching.
130. `Leaf 10.5.2` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - added temporary try-style binding-scope emission helpers for arm-body expression emission (`emit_expr_with_try_style_binding_scope`) and return-arm emission (`emit_return_expr_with_variant_ctx_and_try_style_binding_scope`).
     - applied these helpers in both try-style match lowerings:
       - `emit_try_style_runtime_match_expr`
       - `emit_try_style_either_match_expr`
     - updated try-style either payload binding collection to use mapping-aware bindings (`collect_pattern_binding_stmts_with_cpp_name_map`) so emitted payload declarations and arm-body name resolution remain aligned under shadowing.
   - focused regressions:
     - `test_leaf1052_try_style_either_shadowed_payload_bindings_scope_arm_bodies`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1052 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf1051 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): fix stays shared and AST-scope-gated, avoids crate-specific rewrites, and avoids post-generation text patching.
131. `Leaf 10.5.3` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - hardened local shadow initializer emission in `emit_local` (`Pat::Ident`) so previous same-scope Rust-name→C++-name mappings are preserved while temporarily hiding in-progress shadow locals.
     - hardened shadow-name allocation in `allocate_local_cpp_name` so nested-scope candidates do not reuse the same C++ shadow name as outer same-Rust-name bindings.
     - this removes nested `let rhs = match rhs.next() { ... }` self-reference/use-before-deduction shapes in generated C++ try-style lowering.
   - focused regressions:
     - `test_leaf1053_try_style_runtime_next_shadow_same_scope_uses_outer_iterator_binding`
     - `test_leaf1053_try_style_runtime_next_shadow_loop_scope_avoids_self_reference_head`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf105 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): fix stays shared and scope/shape-gated in AST-aware lowering, with no crate-specific rewrites or post-generation text patching.
132. `Leaf 10.5.4` is complete.
   - plan/scope check: parity repro + deterministic-head analysis + docs updates stayed well below the <1000 LOC threshold and required no additional decomposition.
   - verification run:
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-4-1775849157 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - prior first head family in `Prerelease::cmp` (`rhs_shadow1` self-reference/use-before-deduction from nested try-style shadowing) is removed.
     - new first deterministic head starts at `runner.cpp:1064`: `std::basic_string_view<char>` has no `.bytes()` in `lhs.bytes().all(...)` / `rhs_shadow2.bytes().all(...)`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-4-1775849157/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): maintained deterministic first-head discipline and recorded canonical artifacts before opening the next implementation leaf; no crate-specific rewrite scripts were introduced.
133. `Leaf 11.2.1` is complete.
   - plan/scope check: implementation + focused regressions + parity repro stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler detection in `transpiler/src/codegen.rs`:
     - recursively collects struct/data-enum items across the full module tree for by-value cycle analysis.
     - detects SCCs in the by-value dependency graph while excluding indirection edges (`Box`/`Rc`/`Arc`/`Weak`/`NonNull`/`Pin`) and reference/raw-pointer edges from cycle triggering.
     - emits deterministic unsupported diagnostics in generated output preamble:
       - `// UNSUPPORTED: unsupported by-value circular type dependency in scope <crate>: [...]`
   - focused regressions:
     - `test_leaf1121_by_value_cycle_emits_unsupported_diagnostic`
     - `test_leaf1121_cross_module_by_value_cycle_emits_diagnostic`
     - `test_leaf1121_indirection_cycle_does_not_emit_by_value_cycle_diagnostic`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1121 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-11-2-1-1775849924 --keep-work-dirs`
   - semver repro note:
     - current deterministic Stage D head remains `runner.cpp:1064` (`std::basic_string_view<char>` missing `.bytes()`).
     - this repro did not trigger by-value SCC diagnostics in current expanded semver outputs.
   - canonical artifacts: `/tmp/rusty-parity-matrix-11-2-1-1775849924/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix is shared and AST-shape-gated, avoids crate-specific scripts/rewrites, and keeps deterministic-first parity evidence.
134. `Leaf 11.2.2` is complete.
   - plan/scope check: implementation + focused regression fixture stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler hardening in `transpiler/src/codegen.rs`:
     - by-value SCC diagnostics now include a deterministic cycle path string in addition to sorted type names.
     - cycle paths are selected deterministically via name-ordered traversal inside the SCC (for example `A -> B -> C -> A`).
   - focused regressions:
     - `test_leaf1122_by_value_cycle_diagnostic_includes_cycle_path_and_type_names`
     - existing `leaf1121` cycle diagnostics tests remain green with path-aware output.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf112 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): kept the fix shared and deterministic in AST-aware dependency analysis; no crate-specific rewrites/scripts were introduced.
135. `Leaf 11.2.3` is complete.
   - plan/scope check: this was a design-only documentation leaf and stayed well below the <1000 LOC threshold.
   - added architecture design note in `§11.9.1` for opt-in by-value SCC cycle breaking:
     - explicit opt-in activation contract (default remains diagnostic-only).
     - deterministic edge-selection and rewrite boundaries for SCC cycle breaking.
     - safety/compatibility constraints, artifact expectations, and non-goals before implementation.
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf112 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): added explicit anti-pattern constraints so cycle breaking cannot silently become default behavior.
136. `Leaf 11.2.4` is complete.
   - plan/scope check: implementation stayed well under the <1000 LOC target and was delivered as an opt-in prototype (no default semantic rewrite changes).
   - implemented option plumbing in `transpiler/src/transpile.rs` and `transpiler/src/main.rs`:
     - added `TranspileOptions { by_value_cycle_breaking_prototype: bool }`.
     - added option-aware entry points (`transpile_with_type_map_and_extension_hints_and_options`, `transpile_full_with_options`).
     - wired CLI/runtime flag `--by-value-cycle-breaking-prototype` for single-file/crate flows and `parity-test`.
   - implemented deterministic prototype planning diagnostics in `transpiler/src/codegen.rs`:
     - default mode remains unchanged (`// UNSUPPORTED: ...` only).
     - opt-in mode emits `// PROTOTYPE: ...` diagnostics listing deterministic selected feedback edges (`owner.field -> target`) and cycle path.
     - prototype remains diagnostic-only (no by-value field rewrite lowering yet).
   - focused regressions:
     - `test_leaf1124_default_mode_does_not_emit_cycle_breaking_prototype_diagnostics`
     - `test_leaf1124_opt_in_mode_emits_deterministic_cycle_breaking_feedback_edge`
     - `test_leaf1124_opt_in_mode_can_select_multiple_feedback_edges_for_same_pair`
     - `test_transpile_options_toggle_by_value_cycle_breaking_prototype_diagnostics`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf112 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler transpile_options_toggle -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): kept default behavior diagnostic-only and made edge selection deterministic under explicit opt-in only.
137. `Leaf 13.1` is complete.
   - plan/scope check: implemented as a focused metadata-only pre-scan enhancement and stayed well below the <1000 LOC target.
   - implemented callable-bound metadata capture for extension methods in `transpiler/src/codegen.rs`:
     - added callable metadata model tracking callable trait kind (`Fn`/`FnMut`/`FnOnce`) and argument pass intent (`Value`, `SharedRef`, `MutRef`, `Pointer`).
     - extension-trait method pre-scan now records per-parameter callable-bound metadata from generic/type bounds and where clauses (for example `F: FnOnce(&mut Self) -> R`).
     - conflicting duplicate callable bounds for the same type parameter are dropped deterministically (no guessed merge).
   - focused regressions:
     - `test_leaf131_collects_callable_bound_metadata_for_extension_method_where_clause`
     - `test_leaf131_collects_callable_bound_metadata_for_fn_families_and_ref_shapes`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf131 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): this leaf only adds AST-aware metadata collection and does not apply blanket call-site rewrites.
138. `Leaf 13.2` is complete.
   - plan/scope check: implemented as a focused call-argument lowering change and stayed well below the <1000 LOC target.
   - implemented callable-bound pass-intent application in extension free-function bodies (`transpiler/src/codegen.rs`):
     - added a scoped callable-bound metadata context while emitting extension free-function blocks.
     - call-argument emission now checks callable-bound arg pass intent for callable params (for example `f`).
     - for callable bounds that expect borrowed args (`Fn(&...)` / `FnMut(&mut ...)` / `FnOnce(&mut ...)`), explicit borrow arguments are preserved as borrow-shaped call arguments (`f(&self_)`, `f(&val)`) instead of falling back to by-value stripped forms.
   - focused regressions:
     - updated `test_leaf4154_extension_trait_preserves_explicit_mut_borrow_for_callable_arg` (now asserts `f(&self_)`).
     - added `test_leaf132_extension_trait_callable_bound_preserves_borrow_shape_for_inner_binding` (asserts `f(&val)` for `tap_err`-style inner binding).
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf13 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf4154_extension_trait_preserves_explicit_mut_borrow_for_callable_arg -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): fix is scoped to recognized callable-bound extension-method call sites; no blanket reference rewrite across all call expressions.
139. `Leaf 13.3` is complete.
   - plan/scope check: implemented as focused regression coverage in `transpiler/src/codegen.rs` and stayed well below the <1000 LOC target.
   - added tap-family regression coverage for dereferencing callback bodies:
     - `test_leaf133_tap_call_shape_keeps_deref_closure_param`
     - `test_leaf133_tap_err_call_shape_keeps_deref_closure_param`
     - `test_leaf133_tap_some_call_shape_keeps_deref_closure_param`
   - assertions verify:
     - extension-method calls are rewritten to `rusty::tap(...)` / `rusty::tap_err(...)` / `rusty::tap_some(...)` (no lingering `.tap*(` call forms),
     - closure bodies keep dereference behavior (`*v`, `*error`, `*value`),
     - callback invocations in extension free-function bodies keep borrow-shaped args (`f(&self_)`, `f(&val)`).
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf133 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): this leaf is regression-only and validates shape-gated behavior; no new blanket rewrite path was introduced.
140. `Leaf 13.4` is complete.
   - plan/scope check: executed as parity evidence capture only (no transpiler code-path changes), staying well below the <1000 LOC target.
   - re-ran tap parity matrix:
     - `tests/transpile_tests/run_parity_matrix.sh --crate tap --work-root /tmp/rusty-parity-matrix-13-4-1775853247 --keep-work-dirs`
   - deterministic Stage D delta:
     - prior deterministic tap Stage D head (`invalid type argument of unary '*' (have 'int')` from generated `rusty::tap(10, [&](auto&& v) { return foo += *v; })`) is no longer present.
     - current tap parity pipeline passes Stage D and Stage E (`Build: PASS`, run: `2 passed, 0 failed`).
   - canonical artifacts:
     - `/tmp/rusty-parity-matrix-13-4-1775853247/tap/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - guardrail check against wrong-approach checklist (§11): maintained deterministic-first evidence workflow and recorded artifact-backed frontier movement before opening a new implementation leaf.
141. `Leaf 11.2` was decomposed into follow-up subleaves (`11.2.5` to `11.2.8`) so remaining architecture work stays in <1000 LOC slices.
   - decomposition rationale:
     - `11.2.5`: diagnostics-only eligibility classification for deterministic rewrite planning.
     - `11.2.6`: declaration-site rewrite for directly rewritable feedback edges only.
     - `11.2.7`: constructor/initializer propagation for rewritten edges.
     - `11.2.8`: parity-facing validation and closure reassessment for `Leaf 11.2`.
142. `Leaf 11.2.5` is complete.
   - plan/scope check: implementation + focused regression stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared prototype diagnostics hardening in `transpiler/src/codegen.rs`:
     - added feedback-edge rewrite eligibility classification (`DirectFieldType` / `NonDirectFieldType`) while collecting by-value graph field edges.
     - direct top-level field-type edges are marked rewrite-eligible; nested/container-shaped edges are marked rewrite-ineligible for this phase.
     - opt-in prototype diagnostics now include deterministic eligible/ineligible edge sets (with ineligibility reason text) in addition to selected feedback edges.
   - focused regressions:
     - `test_leaf1125_opt_in_mode_reports_feedback_edge_rewrite_eligibility`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf112 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): this leaf is metadata/diagnostic-only, deterministic, and avoids blanket or crate-specific rewrite behavior.
143. `Leaf 11.2.6` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared opt-in declaration rewrite in `transpiler/src/codegen.rs`:
     - added deterministic rewrite-plan capture from selected by-value feedback edges, filtered to rewrite-eligible (`DirectFieldType`) edges only.
     - rewrote selected declaration sites to `rusty::Box<...>` for named/tuple struct fields and data-enum variant struct fields under prototype opt-in mode.
     - preserved non-direct/ineligible edges as diagnostics-only and intentionally left constructor/initializer propagation for `Leaf 11.2.7`.
     - updated prototype diagnostic banner wording from `diagnostic-only prototype` to `prototype mode` to reflect declaration rewrite activation.
   - focused regressions:
     - `test_leaf1126_default_mode_does_not_rewrite_cycle_field_declaration`
     - `test_leaf1126_opt_in_mode_rewrites_selected_direct_cycle_field_declaration`
     - `test_leaf1126_opt_in_mode_only_rewrites_direct_edge_declarations`
     - `test_leaf1126_opt_in_mode_rewrites_direct_enum_variant_field_declaration`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf112 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): rewrite is opt-in, deterministic, AST-aware, and shape-gated; no crate-specific or post-generation text patching was introduced.
144. `Leaf 11.2.7` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared constructor/initializer propagation in `transpiler/src/codegen.rs`:
     - added a deterministic field-initializer wrapper path keyed by rewrite-plan metadata so rewritten edges initialize with `rusty::Box::make(...)`.
     - updated drop-generated struct constructors to initialize rewritten by-value fields with `rusty::Box::make(std::move(...))` while preserving unchanged behavior for non-rewritten fields.
     - updated data-enum variant constructor helper bodies (named and tuple variants) so rewritten field payloads are wrapped with `rusty::Box::make(...)`.
     - updated struct-literal emission in both designated and positional-constructor paths to wrap rewritten field initializers with `rusty::Box::make(...)`.
   - focused regressions:
     - `test_leaf1127_opt_in_mode_drop_constructor_initializes_rewritten_field_with_box_make`
     - `test_leaf1127_opt_in_mode_struct_literal_designated_field_initialization_wraps_with_box_make`
     - `test_leaf1127_opt_in_mode_struct_literal_positional_constructor_initialization_wraps_with_box_make`
     - `test_leaf1127_opt_in_mode_enum_variant_constructor_helper_wraps_rewritten_field_with_box_make`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf112 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11): changes remain opt-in, deterministic, AST-aware, and field-shape-gated; no crate-specific rewrites or post-generation text patching were introduced.
145. `Leaf 11.2.8` is complete.
   - plan/scope check: this leaf was parity-validation/documentation only, stayed well below the <1000 LOC target, and required no additional decomposition.
   - verification runs:
     - `cargo run -p rusty-cpp-transpiler -- parity-test --manifest-path /home/shuai/git/rusty-cpp/tests/transpile_tests/semver/Cargo.toml --stop-after run --work-dir /tmp/rusty-parity-11-2-8-semver-optin-20260410 --keep-work-dir --by-value-cycle-breaking-prototype`
     - `tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-11-2-8-default-matrix-20260410 --keep-work-dirs`
     - explicit opt-in matrix probe over `either,tap,cfg-if,take_mut,arrayvec,semver,bitflags` with `--by-value-cycle-breaking-prototype` and per-crate work dirs under `/tmp/rusty-parity-11-2-8-optin-matrix-20260410`.
   - deterministic parity results:
     - semver opt-in single-crate parity still fails first at `runner.cpp:1064` (`std::basic_string_view<char>` has no `.bytes()` in `lhs.bytes().all(...)` / `rhs_shadow2.bytes().all(...)`).
     - default matrix and opt-in matrix both stop at the same first failing crate/head (`semver`, `runner.cpp:1064`) with identical summary counts (`total=6 pass=5 fail=1`).
     - semver opt-in generated outputs still show no by-value SCC cycle diagnostics/rewrite markers in this expanded set (no `// PROTOTYPE`/`// UNSUPPORTED` cycle lines and no `rusty::Box::make(...)` cycle-lowering markers in `semver.cppm`).
   - canonical artifacts:
     - semver opt-in single-crate parity: `/tmp/rusty-parity-11-2-8-semver-optin-20260410/{baseline.txt,build.log,run.log,runner.cpp,targets/...}`
     - default matrix first-failure crate: `/tmp/rusty-parity-11-2-8-default-matrix-20260410/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
     - opt-in matrix first-failure crate: `/tmp/rusty-parity-11-2-8-optin-matrix-20260410/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - closure reassessment:
     - `Leaf 11.2` is complete for the planned architecture/prototype scope (`11.2.1`-`11.2.8`).
     - remaining semver parity blockers are currently outside the by-value cycle-breaking family and continue under separate deterministic Stage D families.
   - guardrail check against wrong-approach checklist (§11): validation remained deterministic, artifact-backed, and opt-in-only; no crate-specific rewrite shortcuts were introduced.
146. `Leaf 22.1` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented shared `cpp::` reserved-root import classification in `transpiler/src/codegen.rs`:
     - added reserved-root detection in `emit_use` so `cpp` is not treated as an external crate import root.
     - added `CppModuleUseImport` + `classify_cpp_module_use_import(...)` so flattened `use` paths are classified as foreign C++ module imports when they target `cpp::...`.
     - added dedicated `CodeGen` symbol-resolution tracking for classified `cpp::` imports:
       - `cpp_module_import_bindings` (`binding_name -> module_path`),
       - `cpp_module_import_paths` (ordered unique module paths).
     - updated `emit_use` to emit deterministic foreign-module marker comments for `cpp::` imports instead of normal C++ `using` lowering.
   - focused regressions:
     - `test_leaf221_use_cpp_import_is_classified_as_foreign_module_import`
     - `test_leaf221_use_cpp_alias_import_records_alias_binding`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf221 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): this leaf stayed parser/classification-scoped, introduced no bridge wrappers, and avoided global text substitution of unresolved paths.
147. `Leaf 22.2` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented shared C++ module symbol-index loading and fail-fast plumbing:
     - added stable sidecar model + loader in `transpiler/src/transpile.rs` (`version = 1`, `modules` map with optional `namespace`, per-symbol `kind` and `callable_signatures`) with JSON/TOML parsing.
     - added deterministic multi-file merge with explicit conflict diagnostics for duplicate module/symbol definitions.
     - normalized module keys to canonical `::` path form (`a.b` and `a::b` accepted).
     - added transpile-stage fail-fast check: when `use cpp::...` imports are present and no non-empty symbol index is configured, transpilation now errors before code generation.
     - added CLI support in `transpiler/src/main.rs`:
       - top-level `--cpp-module-index <path>` for single-file and `--crate` flows,
       - parity subcommand `--cpp-module-index <path>`,
       - all wired through shared `TranspileOptions`.
   - focused regressions:
     - `transpile::tests::test_load_cpp_module_symbol_index_json`
     - `transpile::tests::test_load_cpp_module_symbol_index_toml`
     - `transpile::tests::test_cpp_module_import_requires_symbol_index`
     - `transpile::tests::test_cpp_module_import_with_symbol_index_is_allowed`
     - `tests/e2e_basic.rs::test_cli_cpp_module_index_flag_single_file`
     - `tests/e2e_basic.rs::test_crate_mode_cpp_import_requires_symbol_index`
     - `tests/e2e_basic.rs::test_crate_mode_cpp_import_with_symbol_index_succeeds`
   - verification:
     - `cargo test -p rusty-cpp-transpiler cpp_module -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): this leaf remained loader/configuration-only; no bridge-wrapper generation, no call-lowering shortcuts, and no global path text substitution were introduced.
148. `Leaf 22.3` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented deterministic C++ module import emission in `transpiler/src/codegen.rs`:
     - `emit_file` now initializes prologue text via `emit_cpp_module_import_prologue()` so `use cpp::...` imports lower into emitted C++20 `import ...;` lines.
     - added `emit_cpp_module_import_prologue()` to map collected `cpp_module_import_paths` into C++ module names, sort, de-duplicate, and emit one import per module.
     - added `cpp_module_path_to_import_name(...)` helper to convert canonical `a::b` paths to C++ module import names (`a.b`).
   - focused regressions:
     - `test_leaf223_cpp_module_imports_emit_deduped_sorted_cxx_imports`
     - `test_leaf223_cpp_and_rust_imports_coexist`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf223 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): this leaf is import-emission scoped, deterministic, AST-driven, and introduces no bridge wrappers or generated-text patching.
149. `Leaf 22.4` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented direct `cpp::` call-path lowering in `transpiler/src/codegen.rs`:
     - added `rewrite_cpp_import_bound_expr_path(...)` to rewrite expression paths rooted at `cpp::` import bindings to direct qualified C++ paths.
     - integrated that rewrite into `emit_expr_path_to_string(...)`, so aliased imported call paths (for example `cpp_std::max(...)`) lower to native calls (`std::max(...)`) without bridge wrappers.
     - preserved existing canonical call argument/return lowering by reusing the existing call emission pipeline (`emit_call_expr_to_string`) rather than adding interop-only adapters.
   - focused regressions:
     - `test_leaf224_cpp_alias_call_lowers_to_direct_cpp_call_path`
     - `test_leaf224_cpp_nested_module_binding_lowers_to_qualified_cpp_call_path`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf224 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): lowering is AST-aware and scope-gated to classified `cpp` bindings; no bridge-wrapper generation, no blanket/global text substitutions, and no crate-specific shortcuts were introduced.
150. `Leaf 22.5` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented safety-boundary enforcement for `cpp::` imported foreign calls in `transpiler/src/transpile.rs`:
     - added an AST visitor (`CppForeignCallSafetyVisitor`) that tracks `use cpp::...` bindings across module/block scopes and identifies foreign call expressions through those bindings.
     - visitor tracks unsafe context (`unsafe fn` and `unsafe { ... }`) and emits deterministic diagnostics when foreign calls occur in safe context.
     - `transpile_full_with_options` now fails fast with aggregated call-site diagnostics when safe-context foreign C++ calls are detected.
   - focused regressions:
     - `transpile::tests::test_cpp_module_foreign_call_requires_unsafe_context`
     - `transpile::tests::test_cpp_module_foreign_call_in_unsafe_context_is_allowed`
   - verification:
     - `cargo test -p rusty-cpp-transpiler cpp_module -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): enforcement is AST-aware and scope-gated with deterministic diagnostics; no bridge wrappers, no global generated-text substitutions, and no crate-specific shortcuts were introduced.
151. `Leaf 22.6` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented deterministic transpile-stage resolution diagnostics for `cpp::` imported calls in `transpiler/src/transpile.rs`:
     - added `CppForeignCallResolutionVisitor` and integrated it into `transpile_full_with_options` as a fail-fast validation pass before unsafe-boundary checking.
     - visitor tracks lexical `use cpp::...` bindings and validates `binding::symbol(...)` call sites against the configured C++ module symbol index.
     - emits deterministic diagnostics for unresolved module paths, unresolved symbols within resolved modules, and callable-family mismatch when call arity cannot be matched to indexed signatures.
     - diagnostics include module path, symbol name, call/context metadata, and configured index source path(s).
   - propagated index-source diagnostics context through options wiring:
     - added `TranspileOptions::cpp_module_symbol_index_sources`.
     - updated both top-level CLI and parity transpile-option construction in `transpiler/src/main.rs`.
   - focused regressions:
     - `transpile::tests::test_cpp_module_call_errors_when_module_path_missing_from_index`
     - `transpile::tests::test_cpp_module_call_errors_when_symbol_missing_from_index_module`
     - `transpile::tests::test_cpp_module_call_errors_when_signature_family_does_not_match_call_shape`
     - updated `transpile::tests::test_cpp_module_foreign_call_requires_unsafe_context` and `transpile::tests::test_cpp_module_foreign_call_in_unsafe_context_is_allowed` index fixtures so safety checks remain the tested behavior after resolution validation is introduced.
   - verification:
     - `cargo test -p rusty-cpp-transpiler cpp_module -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): implementation is AST-aware and deterministic in shared transpile validation; no bridge wrappers, no blanket/global text rewrites, and no crate-specific shortcuts were introduced.
152. `Leaf 22.7` is complete.
   - plan/scope check: implementation + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - implemented enforced MVP support limits for `cpp::` imports in `transpiler/src/transpile.rs`:
     - extended `CppForeignCallResolutionVisitor` to validate both call and non-call `cpp::` symbol access.
     - supported MVP surfaces are now explicitly enforced as:
       - free/static function calls (`binding::symbol(...)`),
       - module constants in value position (`binding::CONSTANT`).
     - added deterministic fail-fast `TODO(leaf22.7)` diagnostics for unsupported surfaces:
       - member-function import syntax (`binding::Type::method(...)` / multi-segment member-like paths),
       - template-only exports without indexed callable signatures,
       - `cpp::` macro imports/usage (`binding::name!(...)`).
     - unresolved module/symbol/call-family diagnostics remain in place and keep configured index-source attribution.
   - focused regressions:
     - `transpile::tests::test_cpp_module_constant_value_access_is_allowed`
     - `transpile::tests::test_cpp_module_constant_access_errors_when_symbol_missing_from_index_module`
     - `transpile::tests::test_cpp_module_call_errors_for_member_function_import_syntax`
     - `transpile::tests::test_cpp_module_call_errors_for_template_only_export_without_call_shape`
     - `transpile::tests::test_cpp_module_macro_usage_errors_as_unsupported_surface`
   - verification:
     - `cargo test -p rusty-cpp-transpiler cpp_module -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): implementation is AST-aware and shape-gated by surface kind (call/value/macro), and introduces no bridge wrappers, no blanket/global text rewriting, and no crate-specific shortcuts.
153. `Leaf 22.8` was decomposed into sub-leaves (`22.8.1`-`22.8.3`) to keep each execution step below the <1000 LOC target while preserving deterministic integration coverage scope.
154. `Leaf 22.8.1` is complete.
   - plan/scope check: fixture + parity verification updates stayed well below the <1000 LOC target and required no additional decomposition.
   - added dedicated integration fixture assets under `tests/transpile_tests/cpp_module_interop/`:
     - fixture Rust crate (`Cargo.toml`, `src/lib.rs`) uses `use cpp::std as cpp_std;` and `use cpp::custom::math as cpp_math;` and exercises both supported MVP surfaces (free/static calls and module constants).
     - committed symbol index sidecar (`cpp_module_index.toml`) with `std::max`, `custom::math::add_one`, and `custom::math::DEFAULT_BIAS`.
     - committed tiny custom C++ module fixture (`cpp_modules/custom.math.cppm`) that imports `std` and exports `DEFAULT_BIAS` and `add_one`.
   - added parity transpile-stage integration regressions in `transpiler/tests/parity_test_verification.rs`:
     - `test_cpp_module_interop_stop_after_transpile_emits_module_imports_and_direct_calls`
     - `test_cpp_module_interop_stop_after_transpile_requires_symbol_index`
   - regression assertions cover:
     - generated `.cppm` emits expected C++20 imports (`import std;`, `import custom.math;`),
     - direct call lowering (`std::max(...)`, `custom::math::add_one(...)`) and constant lowering (`custom::math::DEFAULT_BIAS`),
     - expected parity Stage C missing-index diagnostics when `--cpp-module-index` is omitted.
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_cpp_module_interop_stop_after_transpile -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): validation stays in shared parity/transpile flows with fixture-agnostic assertions; no generated-output patching, no bridge wrappers, and no global text rewrites were introduced.
155. `Leaf 22.8.2` is complete.
   - plan/scope check: parity dry-run reporting + regression updates stayed well below the <1000 LOC target and required no additional decomposition.
   - updated shared parity harness dry-run behavior in `transpiler/src/main.rs`:
     - Stage C dry-run now reports deterministic transpile actions per discovered target (instead of depending on expanded-source population).
     - added explicit `cpp` index-shape reporting in dry-run Stage C lines:
       - configured index path list (`cpp index: <path...>`),
       - missing-index invocation shape (`cpp index: <none>`).
   - added focused dry-run regressions for the `cpp_module_interop` fixture in `transpiler/tests/parity_test_verification.rs`:
     - `test_cpp_module_interop_dry_run_transpile_reports_indexed_stage_shapes`
     - `test_cpp_module_interop_dry_run_transpile_reports_missing_index_shape`
   - regression assertions cover deterministic Stage B/Stage C dry-run reporting, interop-target discovery, configured-vs-missing index invocation shape, and `--stop-after transpile` boundary (no Stage D execution).
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_cpp_module_interop_dry_run_transpile -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): changes remain in shared parity harness flow with fixture-agnostic assertions; no crate-specific behavior forks, no generated-output patching, and no bridge-wrapper shortcuts were introduced.
156. `Leaf 22.8.3` is complete.
   - plan/scope check: compile-harness script + CI wiring + focused regressions stayed well below the <1000 LOC target and required no additional decomposition.
   - added compile-stage interop harness script: `tests/transpile_tests/run_cpp_module_interop_compile.sh`.
     - drives parity Stage C (`--stop-after transpile`) for the committed `cpp_module_interop` fixture with required symbol index.
     - probes compiler support for `import std;` (tries `g++` then `clang++`) and deterministically returns `SKIP` when unsupported, preventing flaky false failures on hosts without module-ready standard library support.
     - when supported, compiles both fixture custom module (`custom.math.cppm`) and generated transpiled module (`cpp_module_interop.cppm`) and records deterministic diagnostics (`transpile.log`, `build.log`, module paths) on failure.
   - added CI coverage in `.github/workflows/ci.yml`:
     - new `cpp-module-interop-compile` job (after `build-and-test`) runs:
       - `./tests/transpile_tests/run_cpp_module_interop_compile.sh --work-dir "${RUNNER_TEMP}/rusty-cpp-module-interop"`
     - failure-only artifact upload captures `${{ runner.temp }}/rusty-cpp-module-interop/**`.
   - added focused regressions in `transpiler/tests/parity_matrix_harness.rs`:
     - `test_cpp_module_interop_compile_script_dry_run_reports_expected_commands`
     - `test_ci_workflow_defines_cpp_module_interop_compile_job`
     - `test_ci_workflow_uploads_cpp_module_interop_artifacts_on_failure`
   - verification:
     - `bash tests/transpile_tests/run_cpp_module_interop_compile.sh --dry-run`
     - `bash tests/transpile_tests/run_cpp_module_interop_compile.sh --work-dir "$(mktemp -d)"`
     - `cargo test -p rusty-cpp-transpiler --test parity_matrix_harness -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
   - guardrail check against wrong-approach checklist (§11 and §3.13): interop compile coverage remains shared harness/CI orchestration and fixture-agnostic workflow assertions; no bridge-wrapper path, no crate-specific generated-text patching, and no global text substitutions were introduced.
157. `Leaf 22.8` and `Phase 22` are now complete (`22.8.1`-`22.8.3` all done): transpile-stage + dry-run + compile-stage coverage exists for the `cpp::` interop MVP path.
158. `Leaf 10.5.5` is complete.
   - plan/scope check: shared transpiler/runtime updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes:
     - `transpiler/src/codegen.rs`:
       - lowered string-like `.bytes()` to `rusty::as_bytes(...)`,
       - lowered iterator-like `.all(...)` to `rusty::all(...)`,
       - updated local method/item inference so `bytes`/`as_bytes` feed iterator item type `u8` and `.all(...)` infers `bool`.
     - `include/rusty/slice.hpp`: added `rusty::all(range, pred)` helper over `for_in(...)`.
   - focused regressions:
     - `transpiler/src/codegen.rs`: `test_leaf2114_str_bytes_all_lowers_to_runtime_iter_helper`
     - `tests/rusty_array_test.cpp`: `test_all_iterator_helper_shape`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf2114 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `ctest --test-dir build-tests --output-on-failure -R rusty_array_test`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-21-14-1b-1775860634 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1064` (`std::string_view` missing `.bytes()` / `.all(...)` chain) is collapsed.
     - new first deterministic head remains at `runner.cpp:1064` but is now a `std::visit` argument-shape mismatch (`bool, bool`) in the same `Prerelease::cmp` branch family.
   - canonical artifacts: `/tmp/rusty-parity-matrix-21-14-1b-1775860634/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and shape-gated in AST-aware lowering/runtime surfaces; no crate-specific rewrites/scripts and no blanket callsite rewrites were introduced.
159. `Leaf 10.5.6` is complete.
   - plan/scope check: shared transpiler-only lowering updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes in `transpiler/src/codegen.rs`:
     - added shape-gated tuple-value match lowering (`emit_match_expr_tuple_value_conditions`) for non-variant tuple scrutinees (literal/path/wild/ident tuple-element pattern families), avoiding invalid `std::visit` usage on scalar tuples.
     - added tuple-pattern support gating (`tuple_match_can_lower_as_value_conditions`) so tuple value matches lower via deterministic condition chains while tuple-variant visit lowering remains available for non-value pattern families.
     - hardened tuple-value arm emission to avoid duplicated `return return ...` forms when arm-body lowering already emits `return`.
   - focused regressions:
     - `transpiler/src/codegen.rs`: `test_leaf1056_tuple_bool_match_uses_value_conditions_not_visit`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1056 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf2114 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-6b-1775863888 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1064` (`std::visit(..., bool, bool)` from tuple `bytes().all(...)` flags) is collapsed.
     - new first deterministic head remains at `runner.cpp:1064`, now in `Prerelease::cmp` ordering/lambda-return family (`cmp(...).then_with(...)` on non-chainable `Ordering`, plus adjacent lambda return-shape mismatch `Ordering` vs `void`).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-6b-1775863888/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST-shape-gated, with no blanket callsite rewrites, no crate-specific scripts, and no generated-text patching.
160. `Leaf 10.5.7` is complete.
   - plan/scope check: shared transpiler/runtime-fallback updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes in `transpiler/src/codegen.rs`:
     - added shape-gated `Ordering::then_with` lowering (`try_emit_ordering_then_with_call`) to `rusty::cmp::then_with(receiver, callback)` for Ordering-typed receiver families (`cmp(...)`, inferred Ordering receivers, and chained `.then_with(...).then_with(...)`).
     - hardened tuple-value match fallback for non-void contexts to emit terminal `rusty::intrinsics::unreachable();` statement form instead of invalid `return void` shapes.
     - added runtime fallback helper surface for `rusty::cmp::then_with(Ordering, F&&)` in `runtime_path_fallback_helpers_text()`.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf1057_ordering_then_with_lowers_to_runtime_helper`
       - `test_leaf1057_ordering_then_with_lowers_for_cmp_call_receiver_shape`
       - `test_leaf1057_ordering_then_with_chain_lowers_to_runtime_helper_calls`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1057 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf1056 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf2114 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-7c-1775862052 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1064` (`cmp(...).then_with(...)` + lambda return-shape mismatch) is collapsed.
     - new deterministic head moved to `include/rusty/slice.hpp:514` (`rusty::enumerate` deduction recursion) with adjacent omitted-template `rusty::Vec` fallout at `runner.cpp:1267`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-7c-1775862052/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST-shape-gated, with no crate-specific rewrites/scripts and no generated-text patching.
161. `Leaf 10.5.8` is complete.
   - plan/scope check: shared transpiler/runtime updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes:
     - `include/rusty/vec.hpp`: added `data()`/`const data()` accessors so `rusty::iter(vec)` uses the slice-style `data()/size()` path and no longer recurses through `rusty::enumerate(iter(vec))`.
     - `include/rusty/array.hpp`: hardened `rusty::collect_range` to support generic C++ ranges (`begin/end`), `into_iter()`, and Option-like `next()` iterator surfaces.
     - `transpiler/src/codegen.rs`:
       - added function-call expected-argument placeholder hint augmentation for block-local generic placeholders (Vec-focused shape), enabling `Vec::new_()` specialization from callee signatures.
       - added impl-struct field fallback local type recovery for same-name placeholder constructor locals (e.g., `comparators` in `impl VersionReq`).
       - lowered `Vec::from_iter(...)` to `rusty::collect_range(...)` in both generic call path and expected-type associated-call path.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf1058_vec_new_placeholder_uses_function_arg_expected_type_hint`
       - `test_leaf1058_vec_new_placeholder_uses_impl_field_name_fallback`
       - `test_vec_from_iter_mapping`
       - `test_vec_from_iter_with_turbofish`
       - `test_vec_from_iter_with_expected_type_uses_collect_range`
     - `tests/rusty_array_test.cpp`:
       - `test_collect_range_iterator_adapter_shape`
       - `test_iter_vec_enumerate_adapter_shape`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf1058 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_vec_from_iter -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `ctest --test-dir build-tests --output-on-failure -R rusty_array_test`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-8b-1775863750 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `include/rusty/slice.hpp:514` (`rusty::enumerate` recursion) + adjacent omitted-template Vec family (`runner.cpp:1267`) is collapsed.
     - new deterministic head starts at `runner.cpp:1278` (`VersionReq::STAR` deleted-copy path), with immediate adjacent fallout at `runner.cpp:1280` (`ch` unresolved) and `runner.cpp:1290` (`Vec::set_len` missing runtime surface).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-8b-1775863750/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and shape-gated in core transpiler/runtime surfaces, with no crate-specific scripts and no generated-text patching.
162. `Leaf 10.5.9` is complete.
   - plan/scope check: shared transpiler/runtime updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes in `transpiler/src/codegen.rs`:
     - hardened if-let tuple payload binding for `Some((...))` / `Ok((...))` / `Err((...))` by using pattern-driven binding statement emission (`collect_pattern_binding_stmts_with_cpp_name_map`) and scoped Rust-name → C++-name overlays in then-arm body emission.
     - changed associated-const by-value lowering from invalid const-move shapes to `rusty::clone(Type::CONST)` in value-path contexts; retained no blanket multi-segment move insertion.
   - implemented shared runtime support in `include/rusty/vec.hpp`:
     - added unsafe-style `Vec::set_len(size_t)` surface (`assert(new_len <= capacity_)`) for transpiled `unsafe { vec.set_len(len) }` flows.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_ok_variant_with_struct_const_uses_clone_not_move`
       - `test_returning_struct_const_uses_clone_not_move`
       - `test_if_let_some_tuple_payload_binds_nested_tuple_names`
     - `tests/rusty_vec_test.cpp`:
       - `test_vec_set_len`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_ok_variant_with_struct_const_uses_clone_not_move -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_returning_struct_const_uses_clone_not_move -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_if_let_some_tuple_payload_binds_nested_tuple_names -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `ctest --test-dir build-tests --output-on-failure -R rusty_vec_test`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-9b-1775864919 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1278` / `1280` / `1290` (`VersionReq::STAR` deleted-copy, missing `ch` tuple payload binding, and missing `Vec::set_len`) is collapsed.
     - new first deterministic head starts at `runner.cpp:1656` (`std::visit` applied to `rusty::Option<rusty::cmp::Ordering>` in `Version::operator<=>`), with adjacent dependent lambda-return fallout at `runner.cpp:1858`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-9b-1775864919/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/runtime-surface-gated, with no crate-specific scripts and no generated-text patching.
163. `Leaf 10.5.10` is complete.
   - plan/scope check: shared transpiler-only lowering updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - hardened runtime Option/Result tuple-struct match lowering (`emit_runtime_match_expr`) to support payload value-pattern families (for example `Some(Ordering::Equal)`, `Err(0)`) without falling back to invalid `std::visit` on runtime `rusty::Option`/`rusty::Result`.
     - when payload binding statement extraction is not applicable, runtime match lowering now reuses value-condition emission (`tuple_pattern_elem_value_condition`) and composes payload condition + guard condition in shared `is_some`/`is_err` dispatch blocks.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10510_runtime_option_payload_path_pattern_uses_runtime_match_not_visit`
       - `test_leaf10510_runtime_result_payload_literal_pattern_uses_runtime_match_not_visit`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10510 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-10-1775865623 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1656` (`std::visit` on runtime `Option<Ordering>` in `Version::operator<=>`) is removed.
     - new deterministic head starts at `runner.cpp:1858` (`/* TODO: if-expression */` in `matches_caret` lambda return-shape path), with adjacent later runtime Option `std::visit` fallout at `runner.cpp:2056`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-10-1775865623/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST-shape-gated in runtime-match lowering, with no crate-specific rewrites/scripts and no generated-text patching.
164. `Leaf 10.5.11` is complete.
   - plan/scope check: shared transpiler-only lowering updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - expanded try-style runtime match return-arm detection to support nested return-flow arm-body shapes (`if ... { return ... } else { return ... }`), not only direct `return` tails.
     - added nested return-flow statement emission for try-style runtime match lowering so return arms with nested `if`/`block` shapes lower as concrete return-flow statements instead of `/* TODO: if-expression */` placeholders.
   - focused regression:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10511_runtime_option_return_arm_if_expr_lowers_without_todo`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10511 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-11-1775866015 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1858` (`/* TODO: if-expression */` return-shape in `matches_caret`) is removed.
     - new deterministic head starts at `runner.cpp:1907` (invalid `static_cast` from `identifier::Identifier` to `uintptr_t` in `identifier::inline_len`), with adjacent later runtime Option `std::visit` fallout at `runner.cpp:2056`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-11-1775866015/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST-shape-gated in try-style/runtime-match lowering, with no crate-specific rewrites/scripts and no generated-text patching.
165. `Leaf 10.5.12` is complete.
   - plan/scope check: shared transpiler-only local-shadow/cast-lowering hardening + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - added scoped in-progress local-initializer tracking so local type/reference lookup skips the just-declared shadow binding while emitting its initializer (`let repr = ... repr ...` resolves `repr` to the outer binding in initializer context).
     - hardened `lookup_local_binding_type` to bypass only the innermost same-name local entry during that initializer emission, restoring outer/parameter reference type visibility for cast lowering.
     - preserved move semantics for inferred-typed unannotated local initializers by using expected-type emission + move insertion (`emit_expr_to_string_with_expected_and_move_if_needed`) so shadowed parameter move semantics remain correct.
   - focused regression:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10512_shadowed_param_pointer_cast_uses_outer_reference_binding_in_initializer`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10512 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41543333332_local_binding_shadowing_param_is_renamed -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-12-1775866809 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1907` (`identifier::inline_len` invalid cast-chain lowering from shadowed `repr` parameter to pointer) is removed.
     - new deterministic head starts at `runner.cpp:1930` (`identifier::decode_len` `/* TODO: complex pattern binding */` fallout causing missing `first`/`second` bindings and `decode_len_cold` call-shape breakage), with adjacent later runtime Option `std::visit` fallout at `runner.cpp:2056`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-12-1775866809/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST/context-gated in local-shadow + cast lowering, with no crate-specific rewrites/scripts and no generated-text patching.
166. `Leaf 10.5.13` is complete.
   - plan/scope check: shared transpiler-only pattern-lowering + block-emission ordering updates with focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - added local slice-pattern destructuring support for `let [a, b] = expr;` in block locals (`emit_local`, `register_local_binding_pattern`, `emit_pat_to_string`) so these patterns lower as structured bindings instead of complex-pattern TODO fallbacks.
     - hoisted nested block-local function item emission (`Stmt::Item(Item::Fn)`) ahead of non-function statements in each block, preserving Rust item visibility semantics for same-block call sites where the nested item appears later in lexical order.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10513_local_slice_binding_lowers_without_todo`
       - `test_leaf10513_nested_local_fn_call_before_item_definition_is_hoisted`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10513 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-13-1775867215 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1930` (`decode_len` missing `[first, second]` binding + nested `decode_len_cold` call-order breakage) is removed.
     - new deterministic head starts at `runner.cpp:1977` in `parse::numeric_identifier` (`while let Some(&digit)` lowering fallout emitting `while (rusty::intrinsics::unreachable())` with missing `digit` binding), with adjacent later runtime Option `std::visit` fallout at `runner.cpp:2056`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-13-1775867215/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/context-gated in local pattern + block emission lowering, with no crate-specific rewrites/scripts and no generated-text patching.
167. `Leaf 10.5.14` is complete.
   - plan/scope check: shared transpiler-only `while let` lowering updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - added `while_let_condition_parts(...)` and switched `emit_while_let(...)` to condition+binding planning that supports reference payload patterns (`while let Some(&digit) = ...`) instead of falling back to boolean `ExprLet` lowering.
     - preserved optional-surface behavior for iterator/optional-like scrutinees and existing `while let Some(v) = iter.next()` lowering (`option_has_value`/`option_take_value` helper path when needed).
     - added mapped local binding-scope emission for `while let` payload bindings so non-simple patterns emit stable C++ bindings in loop bodies.
   - focused regression:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10514_while_let_option_ref_payload_binds_without_unreachable_condition`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10514 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf41543333333327291 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-14-1775869000 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1977` (`while let Some(&digit)` lowering emitted `while (rusty::intrinsics::unreachable())` with missing `digit` binding) is removed.
     - new deterministic head starts at `runner.cpp:1989` in `parse::numeric_identifier` (`checked_add(int&, uint64_t)` type-shape mismatch), with adjacent downstream runtime Option `std::visit` fallback still present later at `runner.cpp:2060`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-14-1775869000/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST/type-context-gated in core `while let` lowering, with no crate-specific rewrites/scripts and no generated-text patching.
168. `Leaf 10.5.15` is complete.
   - plan/scope check: shared transpiler-only checked-arithmetic lowering update + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fix in `transpiler/src/codegen.rs`:
     - checked method-call lowering (`checked_add`/`checked_sub`/`checked_mul`/`checked_div`) now normalizes RHS argument type to receiver value type (`std::remove_cvref_t<decltype((receiver))>`) before calling shared runtime helpers (`rusty::checked_*`), preventing mixed-width RHS expressions from breaking template deduction.
   - focused regression:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10515_checked_add_rhs_is_normalized_to_receiver_type`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10515 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf4154412 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-15-1775869800 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:1989` (`checked_add(int&, uint64_t)` mismatch in `parse::numeric_identifier`) is removed.
     - new deterministic head starts at `runner.cpp:2060` (`std::visit` emitted over runtime `rusty::Option<const uint8_t&>` in `parse::identifier`), with adjacent comparator/local-deduction fallback errors later at `runner.cpp:2090+`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-15-1775869800/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and type-context-gated in core checked-arithmetic lowering, with no crate-specific rewrites/scripts and no generated-text patching.
169. `Leaf 10.5.16` is complete.
   - plan/scope check: shared transpiler-only control-flow/match-lowering updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - added value-shape gating for forced tail match expression lowering (`match_expr_is_value_like` + fallthrough analysis helpers), so loop-tail `match` blocks with unit-like arms are emitted via statement control-flow lowering instead of `return <match-iife>;`.
     - extended runtime statement match lowering (`try_emit_runtime_match_stmt`) to support tuple payload value conditions and top-level OR runtime patterns through deterministic matcher-state synthesis (`_m_or_match*`) on shared `is_some`/`unwrap` surfaces.
     - switched runtime statement lowering to a two-pass plan-then-emit flow so unsupported runtime patterns fail cleanly without partial output corruption.
     - extended tuple payload value-condition lowering (`tuple_pattern_elem_value_condition`) for range payloads (`Pat::Range`) and recursive wrapper payload patterns (`Pat::Reference`/`Pat::Type`/`Pat::Paren`), including range cases inside OR payloads.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10516_runtime_option_payload_range_pattern_uses_runtime_match_not_visit`
       - `test_leaf10516_tail_loop_runtime_option_or_pattern_uses_statement_lowering`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10516 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler --test either_parity_harness test_either_parity_harness_stop_after_run_passes_as_control_crate -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-16-1775877400 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:2060` (`std::visit` emitted over runtime `rusty::Option<const uint8_t&>` in `parse::identifier`) is removed.
     - new deterministic head starts at `runner.cpp:2094` (`std::string_view` has no `split_at` in `parse::identifier` boundary return path), with adjacent comparator/local-deduction fallback errors later at `runner.cpp:2129+`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-16-1775877400/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/control-flow-shape-gated in core match lowering, with no crate-specific rewrites/scripts and no generated-text patching.
170. `Leaf 10.5.17` is complete.
   - plan/scope check: shared transpiler/runtime helper updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler/runtime fixes:
     - `transpiler/src/codegen.rs`:
       - added shape-gated lowering for `split_at` on known string-like receivers to `rusty::split_at(receiver, idx)` so `std::string_view` call sites no longer emit invalid `.split_at(...)` member calls.
       - added method-result type inference for `split_at` to `(&str, &str)` in local-binding inference paths to keep destructuring/type-context lowering stable.
     - `include/rusty/string.hpp`:
       - added shared `rusty::split_at(std::string_view, size_t)` helper returning `std::tuple<std::string_view, std::string_view>`.
       - helper enforces Rust-like bounds and UTF-8 boundary checks (continuation-byte split offsets are rejected) and the header now includes `<cstdint>` explicitly for `uint8_t` helper surfaces.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10517_str_split_at_lowers_to_runtime_helper`
       - `test_leaf10517_non_string_split_at_method_is_not_rewritten`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10517 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-17-1775870140 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:2094` (`std::string_view` missing `split_at` in `parse::identifier`) is removed.
     - new deterministic head starts at `runner.cpp:2129` (`parse::comparator` emits `use of 'op' before deduction of 'auto'`), with adjacent structured-binding/void-deduction fallout at `runner.cpp:2133+`.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-17-1775870140/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and receiver-shape/type-gated in core method lowering/runtime helpers, with no crate-specific rewrites/scripts and no generated-text patching.
171. `Leaf 10.5.18` is complete.
   - plan/scope check: shared transpiler-only local-binding/control-flow lowering updates + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - hardened local binding allocation to avoid collisions with visible function names in scope (top-level and module-qualified), including tuple/ident binding paths; this removes self-colliding forms like `auto [op, text] = op(...)`.
     - extended recursive if-assignment lowering for nested `else if` branches in if-let statement-block mode so return/`?` branches stay in statement lowering and no longer fall back to `/* TODO: if-expression */` in this family.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10518_tuple_binding_shadowing_function_name_is_renamed`
       - `test_leaf10518_ident_binding_shadowing_function_name_is_renamed`
       - `test_leaf10518_if_let_nested_else_if_with_return_and_try_lowers_without_todo`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10518 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-18-1775870799 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:2129` (`parse::comparator` local destructuring/function-call name collision `use of 'op' before deduction of 'auto'`) is removed.
     - new deterministic head starts at `runner.cpp:2180` in `parse::comparator` (`patch_shadow1` lowered as `std::nullopt_t` and then used via `.is_some()`), with adjacent fallout at `runner.cpp:2193` (`.is_some()` repeat), `runner.cpp:2200` (const assignment), and `runner.cpp:2203` (stale `text_shadow12` binding use).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-18-1775870799/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/control-flow-shape-gated in core local-binding + if-let lowering paths, with no crate-specific rewrites/scripts and no generated-text patching.
172. `Leaf 10.5.19` is complete.
   - plan/scope check: shared transpiler-only lowering/type-inference hardening + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - hardened if-let tuple statement-block lowering to use per-element inferred tuple expected types when seeding else/default tuple values, so `None` tuple elements lower into typed `Option` surfaces instead of `std::nullopt_t` auto-deduction traps.
     - added a fallback `?` payload inference path for local tuple-env updates so `let (...) = foo()?;` contributes element types when direct initializer inference misses the payload.
     - preserved reference element shape when binding `if let` condition patterns into inference env (avoids degrading `&str` to `str` and stabilizes tuple-branch merge typing).
     - restored tuple peer Result-constructor context emission stability by preserving peer-context lowering when expected type context is also available.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10519_if_let_tuple_result_assigns_multistmt_tail_value`
       - `test_leaf10519_single_if_result_temp_is_mutable_in_statement_lowering`
       - `test_leaf10519_if_let_tuple_result_seed_is_option_typed_not_nullopt_tuple`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10519 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf41543333333161 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-19b-1775873154 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family member at `runner.cpp:2150` (`_iflet_result2` assignment into `std::tuple<std::nullopt_t, ...>`) is removed.
     - new deterministic head starts at `runner.cpp:2172` (`_iflet_result3` in adjacent patch branch still deduces `std::nullopt_t`), with adjacent fallout at `runner.cpp:2180/2193` (`.is_some()` on nullopt_t), `runner.cpp:2203` (stale `text_shadow12` binding), and `runner.cpp:2209+` return-shape cascade.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-19b-1775873154/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/type-context-gated in core if-let/type-inference lowering, with no crate-specific rewrites/scripts and no generated-text patching.
173. `Leaf 10.5.20` is complete.
   - plan/scope check: shared transpiler-only control-flow/type-inference hardening + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - hardened tuple-result if-expression inference to tolerate diverging branch forms (for example `else if ... { return Err(...); }`) by merging non-diverging tuple evidence with explicit block-tail divergence checks, so statement-lowered if-let tuple temps keep typed `Option` payloads.
     - added transient local-scope handling for statement-lowered if/if-let/if-assign branches to prevent branch-local binding leakage into outer post-if statements.
     - fixed local-shadow initializer handling for same-Rust-name outer bindings in statement-lowering scopes so shadow initializers resolve to the outer binding (avoid `let text = &text[1..]` self-reference emission).
     - ensured early-return statement-lowered local-init path records the finalized Rust-name → C++-name mapping for subsequent statements in the enclosing scope.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10520_if_let_tuple_result_with_else_if_return_is_option_typed`
       - `test_leaf10520_statement_lowered_if_shadow_binding_does_not_leak`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10520 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-20c-1775874276 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:2172` (`_iflet_result3` nullopt tuple seed), plus adjacent fallout at `runner.cpp:2180/2193` (`.is_some()` on nullopt_t) and `runner.cpp:2203` (stale text binding), is removed.
     - new deterministic head starts at `runner.cpp:2209` (`version_req` error-arm lambda still lowers through `/* TODO: if-expression */`, yielding tuple-vs-result return-shape mismatch), with adjacent fallout at `runner.cpp:2218/2223` (void placeholder propagation).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-20c-1775874276/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/control-flow/type-context-gated in core statement-lowering/inference paths, with no crate-specific rewrites/scripts and no generated-text patching.
174. `Leaf 10.5.21` is complete.
   - plan/scope check: shared transpiler-only try-style/control-flow/type-inference hardening + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - extended try-style runtime return-flow detection/emission to support multi-statement block arms with tail return expressions, including scoped shadow-binding emission for block statements.
     - removed default-construction requirements for try-style runtime match value temporaries by emitting `std::optional<T>` storage with `.emplace(...)` and terminal `.value()` extraction.
     - widened local-if statement-lowering triggering to include else-branch early-return/`?` control-flow detection so `let x = if let ... { ... } else { return Err(...); };` remains in statement lowering.
     - fixed single if-let statement-lowering default-init emission when inferred local type is unresolved `auto`: emit `decltype(<then-tail>) local{}` instead of invalid `auto local{}`.
   - focused regressions:
     - `transpiler/src/codegen.rs`:
       - `test_leaf10521_runtime_match_return_block_with_iflet_lowers_without_todo`
       - `test_leaf10521_if_let_else_return_local_uses_statement_lowering_without_todo`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10521 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf1052 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf1051 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-21c-1775875758 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:2209` (try-style error-arm `/* TODO: if-expression */` return-shape mismatch), plus adjacent fallout at `runner.cpp:2218/2223` (void placeholder propagation) and `runner.cpp:2226` (`auto text_shadow1 {}`), is removed.
     - new deterministic head starts at `runner.cpp:2411` (`rusty::String` missing `repeat`), with adjacent downstream families led by `runner.cpp:3115` (local/function-name collision) and `runner.cpp:3271/3282` (`util::req` function/member call-shape mismatch).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-21c-1775875758/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/control-flow/type-context-gated in core try-style runtime and statement-lowering paths, with no crate-specific rewrites/scripts and no generated-text patching.
175. `Leaf 10.5.22` is complete.
   - plan/scope check: shared runtime-surface addition + focused runtime regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared runtime fixes:
     - `include/rusty/string.hpp`: added `rusty::String::repeat(size_t)` with overflow guard (`std::length_error` on size multiplication overflow), pre-sized allocation, and deterministic repeated-copy construction while preserving source immutability.
   - focused regressions:
     - `tests/rusty_string_test.cpp`: `test_string_repeat` (normal repeat shape, zero-count behavior, source non-mutation, overflow guard).
     - `transpiler/tests/runtime_move_semantics.rs`: `test_string_repeat_supports_zero_and_overflow_guard`.
   - verification:
     - `clang++ -std=c++20 -Iinclude tests/rusty_string_test.cpp -o /tmp/rusty_string_test && /tmp/rusty_string_test`
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-22-1775876201 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first hard-error family at `runner.cpp:2411` (`rusty::String` missing `repeat`) is removed.
     - new deterministic head starts at `runner.cpp:3115` (`const auto version = version("1.2.3-rc1");` local/function-name collision use-before-deduction), with adjacent downstream call-shape fallout at `runner.cpp:3271/3282` (`util::req` resolved as function instead of value object).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-22-1775876201/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared in runtime headers, added no crate-specific rewrites/scripts, and performed no generated-text patching.
176. `Leaf 10.5.23` is complete.
   - plan/scope check: shared transpiler-only name-resolution hardening + focused regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler fixes in `transpiler/src/codegen.rs`:
     - expanded local-binding collision detection to include visible module-qualified functions from `module_qualified_functions` so locals no longer collide with imported callable names (`let version = version(...)` now renames local binding).
     - hardened single-segment expression-path lowering to prefer in-scope local/parameter value bindings before function qualification, preserving receiver-call shapes on parameters (`req.matches(...)`) instead of rewriting through module function paths.
   - focused regressions:
     - `test_leaf10523_local_binding_shadowing_module_qualified_function_is_renamed`
     - `test_leaf10523_method_receiver_prefers_parameter_binding_over_qualified_function`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10523 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-23-1775876850 --keep-work-dirs`
   - deterministic semver Stage D frontier movement:
     - previous first-head family at `runner.cpp:3115` (`const auto version = version("...")`) is removed; generated output now emits renamed local binding (`version_shadow1 = util::version(...)`).
     - adjacent receiver-shape family at `runner.cpp:3271/3282` is removed; generated output now preserves parameter receiver calls (`req.matches(parsed)`).
     - new deterministic first head starts at `/home/shuai/git/rusty-cpp/include/rusty/rusty.hpp:136` (`rusty::default_value<identifier::Identifier>()` requiring unavailable default constructor), with adjacent fallout at `/home/shuai/git/rusty-cpp/include/rusty/array.hpp:364` (`rusty::len` on `Prerelease`/`BuildMetadata` lacking `size` surface).
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-23-1775876850/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and AST/scope-gated in core name-resolution paths, with no generated-text patching and no crate-specific rewrites/scripts.
177. `Leaf 10.5.24` is complete.
   - plan/scope check: shared runtime-header hardening + focused runtime regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared runtime fixes:
     - `include/rusty/rusty.hpp`: hardened `rusty::default_value<T>()` fallback selection so non-default-constructible empty-surface types resolve via `T::empty()` before value-init fallback.
     - `include/rusty/array.hpp`: extended `rusty::len(const Container&)` with `as_str()` fallback and requires-gated `std::size` terminal path, preventing unconditional container-size instantiation failures on string-like wrappers.
   - focused regressions:
     - `test_default_value_prefers_empty_for_non_default_constructible_types`
     - `test_len_supports_as_str_wrappers_without_size_surface`
   - verification:
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-24-1775879000 --keep-work-dirs`
   - deterministic semver frontier movement:
     - previous Stage D compile-head family at `/home/shuai/git/rusty-cpp/include/rusty/rusty.hpp:136` (`default_value<identifier::Identifier>()` constructor mismatch) and adjacent `/home/shuai/git/rusty-cpp/include/rusty/array.hpp:364` (`len(Prerelease|BuildMetadata)` `std::size` mismatch) is removed; Stage D now builds successfully.
     - new deterministic frontier moved to Stage E runtime failure: runner exits with `SIGSEGV` (exit 139) immediately after first printed pass, with gdb showing recursive `identifier::Identifier::is_empty()`/destructor chain rooted at `runner.cpp:811-815` and `runner.cpp:861-864` on the `test_align` path.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-24-1775879000/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared in runtime headers with shape-gated fallback logic, with no generated-text patching and no crate-specific rewrites/scripts.
178. `Leaf 10.5.25` is complete.
   - plan/scope check: shared runtime-header `mem::forget` hardening + focused runtime regressions stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared runtime fix:
     - `include/rusty/mem.hpp`: hardened `rusty::mem::forget(T&&)` for const-markable values by matching `remove_cv_t<T>` `rusty_mark_forgotten` surfaces and directly marking the value address when the bound value is const.
   - focused regressions:
     - `test_mem_forget_marks_const_values_with_rusty_drop_guard`
     - `test_mem_forget_const_prevents_is_empty_destructor_recursion_shape`
   - verification:
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-25-1775880200 --keep-work-dirs`
     - `/tmp/rusty-parity-matrix-10-5-25-1775880200/semver/runner` direct replay (`EXIT:1`, no segfault).
   - deterministic semver frontier movement:
     - previous Stage E head (`SIGSEGV`/exit 139 immediately after first pass, recursive `Identifier::is_empty`/destructor chain rooted at `runner.cpp:811-815` and `runner.cpp:861-864`) is removed.
     - new deterministic head is runtime assertion/panic mismatch family starting at `test_align` (`runner.cpp:3114-3169`) with adjacent widespread assertion/unwrap fallout (`test_basic`, `test_new`, `test_parse`, `test_spec_order`, etc.); parity now reports 8 passed / 24 failed.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-25-1775880200/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp,run-direct.log}`.
   - guardrail check against wrong-approach checklist (§11): fix stayed shared in runtime headers with type/shape-gated behavior, with no generated-text patching and no crate-specific rewrites/scripts.
179. `Leaf 10.5.26` is complete.
   - plan/scope check: shared transpiler-side `format_args!` lowering hardening plus runtime fallback formatting/to_string support stayed well below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared transpiler/runtime fixes in `transpiler/src/codegen.rs`:
     - hardened `format_args!` expression lowering to emit concrete `std::format(...)`/`std::string(...)` shapes with `rusty::to_string(...)` wrapping and Rust-debug-spec literal rewrites (`:?`/`:#?`).
     - added expression-aware format-arg conversion fallback for expanded-token `self` member chains (including spaced forms like `self . major` and tuple members `self . 0`) so lowering reuses normal `this->...` field emission.
     - extended runtime fallback helper surfaces: `rusty::fmt::Formatter` now accumulates output in `write_fmt`/`write_str`/`write_char`, and shared `rusty::to_string(...)` now dispatches through `.to_string()`, bool/string-like/as_str, deref-string-view, numeric `std::to_string`, and `fmt` fallback rendering.
   - focused regressions:
     - `test_leaf10526_format_args_non_literal_arg_uses_to_string_wrapper`
     - `test_leaf10526_format_args_debug_spec_is_rewritten_for_std_format`
     - `test_leaf10526_format_args_argument_uses_expression_lowering_for_tuple_field`
     - `test_leaf10526_format_args_argument_with_spaced_self_member_tokens_lowers_to_this_members`
     - `test_leaf10526_format_args_argument_with_spaced_self_tuple_tokens_lowers_to_this_members`
     - `test_leaf10526_runtime_to_string_supports_fmt_display_fallback`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10526 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler -- --nocapture`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-26-1775884100 --keep-work-dirs`
   - deterministic semver frontier movement:
     - previous Stage E head at `test_align` (`runner.cpp:3114-3169`) is removed; `test_align` and `test_display` now pass.
     - previous Stage D compile fallout from raw `self . field` format-arg tokens is removed (`runner.cpp`/target `.cppm` no longer emits `rusty::to_string(self . ...)` in this family).
     - new deterministic Stage E frontier is parser/comparator assertion+unwrap mismatch family starting at `test_basic`, with adjacent `test_cargo3202`/`test_comparator_parse`/`test_parse`/`test_wildcard*` fallout; parity now reports 11 passed / 21 failed.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-26-1775884100/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp,run-direct.log}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and shape-gated in transpiler/runtime lowering paths, with no crate-specific rewrites/scripts and no generated-text patching.
180. `Leaf 10.5.27` is complete.
   - plan/scope check: shared transpiler/runtime hardening plus focused regressions stayed below the <1000 LOC threshold and required no additional decomposition.
   - implemented shared fixes:
     - `transpiler/src/codegen.rs`:
       - extended consuming-binding detection for UpperCamelCase value constructors (tuple-struct/variant constructor paths) so immutable payload locals are emitted non-const when consumed by value.
       - extended consuming-binding detection for struct-literal by-value field payloads so locals forwarded into returned/constructed owned fields are emitted non-const before move emission.
     - `include/rusty/mem.hpp`:
       - made forgotten-address state (`forgotten_addresses` map + mutex) process-lifetime to avoid static-destruction-order use-after-free when global destructors still execute drop-guard calls at exit.
   - focused regressions:
     - `test_leaf10527_tuple_constructor_argument_marks_local_binding_non_const`
     - `test_leaf10527_struct_literal_field_consumes_local_binding_non_const`
     - `test_mem_forgotten_address_storage_survives_global_destructor_calls`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10527 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler --test runtime_move_semantics -- --nocapture`
     - `tests/transpile_tests/run_parity_matrix.sh --crate semver --work-root /tmp/rusty-parity-matrix-10-5-27-1775885088 --keep-work-dirs`
   - deterministic semver frontier movement:
     - previous Stage E parser/comparator assertion+unwrap head family is removed; `test_basic`, `test_cargo3202`, and `test_comparator_parse` now pass.
     - previous early Stage E abort point after `test_display` is removed.
     - new deterministic Stage E frontier is `test_eq_hash FAILED: panic` with follow-on `free(): double free detected in tcache 2` abort.
   - canonical artifacts: `/tmp/rusty-parity-matrix-10-5-27-1775885088/semver/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and shape-gated in AST-aware lowering/runtime surfaces, with no crate-specific rewrites/scripts and no generated-text patching.
181. `Leaf 10.5.28` is complete.
   - plan/scope check: shared transpiler/runtime parity fixes plus focused regressions stayed below the <1000 LOC guardrail and required no additional decomposition.
   - implemented shared fixes:
     - `transpiler/src/codegen.rs`: runtime hash fallback now hashes range-like values (`std::begin/std::end`) element-by-element before `std::hash`/byte fallback.
     - `transpiler/src/codegen.rs`: `Drop`-struct Rule-of-Five emission now generates custom move-assignment reconstruction (`this->~T(); new (this) T(std::move(other));`) instead of defaulted move assignment, preserving forgotten-address transfer semantics.
   - focused regressions:
     - `test_leaf10528_runtime_hash_helper_hashes_ranges_by_elements`
     - `test_leaf10528_tuple_payload_consumes_local_binding_non_const`
     - `test_leaf10528_drop_struct_move_assignment_reconstructs_via_move_ctor`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10528 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `PATH=/tmp/rusty-fake-gpp-bin:$PATH cargo run -p rusty-cpp-transpiler -- parity-test --manifest-path /home/shuai/git/rusty-cpp/tests/transpile_tests/semver/Cargo.toml --work-dir /tmp/rusty-parity-semver-10-5-28-full-1775886945 --keep-work-dir`
   - deterministic semver frontier movement:
     - previous Stage E `test_eq_hash FAILED: panic` + follow-on ownership teardown abort is removed.
     - semver parity now reaches full Stage E success (`32 passed, 0 failed`).
   - canonical artifacts: `/tmp/rusty-parity-semver-10-5-28-full-1775886945/{baseline.txt,build.log,run.log,runner.cpp}`.
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and shape-gated in core codegen/runtime helper paths, with no crate-specific rewrites/scripts and no generated-text patching.
182. `Leaf 10.5.29` is complete.
   - plan/scope check: shared transpiler-only lowering update plus focused regressions stayed below the <1000 LOC guardrail and required no additional decomposition.
   - root-cause finding:
     - in dependent-assoc softening mode, Option value-position lowering still emitted explicit typed constructors (`rusty::Option<typename IterEither::Item>(...)`) even when associated aliases were intentionally skipped; this caused deterministic `either` Stage D failure.
   - implemented shared fix in `transpiler/src/codegen.rs`:
     - generalized dependent-assoc Option ctor suppression to `should_soften_dependent_assoc_mode()` so `None`/`Some(...)` lower to `std::nullopt` / `std::make_optional(...)` in softened associated-type contexts.
     - updated prior module-mode regression to assert softened Option value shapes.
   - focused regressions:
     - `test_leaf10529_module_mode_option_none_avoids_assoc_ctor_type_in_value_position`
     - `test_leaf10529_module_mode_option_some_avoids_assoc_ctor_type_in_value_position`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10529 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf4154333333381_module_mode_option_self_assoc_next_uses_explicit_option_shape -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `PATH=/tmp/rusty-fake-gpp-bin:$PATH tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10-5-29-1775890201 --keep-work-dirs`
   - deterministic full-matrix frontier movement:
     - previous first failing crate (`either` Stage D `IterEither::Item` Option-ctor family) is removed; `either` now passes.
     - new first failing crate is `arrayvec` Stage D (`runner.cpp:803/806/810` comparator/member-shape family), with adjacent `CAPERROR`/`BackshiftOnDrop` fallout.
   - canonical artifacts:
     - previous head capture: `/tmp/rusty-parity-matrix-rerun-top-1775887363/either/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
     - post-fix matrix: `/tmp/rusty-parity-matrix-10-5-29-1775890201/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST/type-shape-gated in core Option constructor lowering; no crate-specific rewrites/scripts or generated-text patching were introduced.
183. `Leaf 10.5.30` is complete.
   - plan/scope check: shared transpiler-only lookup hardening plus focused regressions stayed below the <1000 LOC guardrail and required no additional decomposition.
   - root-cause finding:
     - non-`self` field access rename recovery relied on bare `Type::Path` receiver lookup, so reference-typed receivers (`&Type`) dropped field-rename metadata and emitted method-name member references (`other.element`) instead of renamed fields (`other.element_field`).
   - implemented shared fix in `transpiler/src/codegen.rs`:
     - hardened `lookup_field_type_from_type` and `lookup_field_cpp_name_from_type` to peel reference/paren/group wrappers before struct-field metadata lookup.
   - focused regression:
     - `test_leaf10530_nonself_field_access_uses_renamed_member_for_ref_typed_receiver`
   - verification:
     - `cargo test -p rusty-cpp-transpiler test_leaf10530_nonself_field_access_uses_renamed_member_for_ref_typed_receiver -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler test_leaf41542_field_name_collision_with_method_is_renamed -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `PATH=/tmp/rusty-fake-gpp-bin:$PATH tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10-5-30-1775891702 --keep-work-dirs`
   - deterministic full-matrix frontier movement:
     - previous first hard-error family at `runner.cpp:803/806/810` (`other.element` member-function reference misuse) is removed.
     - new first hard-error family is declaration-order/local-type-order fallout at `runner.cpp:823` (`CAPERROR` undeclared in inline method) and `runner.cpp:1050` (`BackshiftOnDrop` unknown type in local callable signature).
   - canonical artifacts:
     - previous head capture: `/tmp/rusty-parity-matrix-10-5-29-1775890201/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
     - post-fix matrix: `/tmp/rusty-parity-matrix-10-5-30-1775891702/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - guardrail check against wrong-approach checklist (§11): fix stayed shared and AST/type-shape-gated in field-name recovery paths; no crate-specific rewrites/scripts or generated-text patching were introduced.
184. `Leaf 10.5.31` is complete.
   - plan/scope check: shared transpiler-only declaration-order fixes plus focused regressions stayed below the <1000 LOC guardrail and required no additional decomposition.
   - root-cause findings:
     - top-level/module consts were not forward-declared, so inline methods could reference later const definitions before declaration (`CAPERROR`).
     - block-local function hoisting did not hoist block-local type items first, so local function signatures could reference undeclared local types (`BackshiftOnDrop`).
   - implemented shared fixes in `transpiler/src/codegen.rs`:
     - extended `emit_item_forward_decls` to emit deduplicated `extern const <type> <name>;` declarations for supported top-level/module consts.
     - hoisted block-local `struct`/`enum`/`type` items before block-local `fn` item lowering in block emission order.
   - focused regressions:
     - `test_leaf10531_top_level_const_is_forward_declared_before_inline_use`
     - `test_leaf10531_block_local_type_item_is_emitted_before_local_fn_signature_use`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10531 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `PATH=/tmp/rusty-fake-gpp-bin:$PATH tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10-5-31-1775888784 --keep-work-dirs`
   - deterministic full-matrix frontier movement:
     - previous first hard-error family at `runner.cpp:823/1050` (`CAPERROR` undeclared + `BackshiftOnDrop` unknown type) is removed.
     - new first hard-error family is return-type deduction mismatch (`std::nullopt_t` vs `std::optional<T>`) at `runner.cpp:1440` with adjacent repeats at `runner.cpp:1456/737`.
   - canonical artifacts:
     - previous head capture: `/tmp/rusty-parity-matrix-10-5-30-1775891702/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
     - post-fix matrix: `/tmp/rusty-parity-matrix-10-5-31-1775888784/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and order/type-shape-gated in core codegen paths; no crate-specific rewrites/scripts or generated-text patching were introduced.
185. `Leaf 10.5.32` is complete.
   - plan/scope check: shared transpiler-only return-signature/alias-tracking updates plus focused regressions stayed below the <1000 LOC guardrail and required no additional decomposition.
   - root-cause findings:
     - early associated-type alias tracking in struct emission recorded aliases before confirming emission, so constrained-mode skipped aliases still appeared "available" and incorrectly kept explicit dependent-associated return signatures.
     - module-mode trait runtime helper default methods always emitted `static auto` signatures; methods returning `Option<Self::Item>` then mixed `std::nullopt` and `std::make_optional(...)` branch returns, producing C++ return-deduction mismatch (`nullopt_t` vs `optional<T>`) in `arrayvec` (`ArrayVecImpl::pop`).
   - implemented shared fixes in `transpiler/src/codegen.rs`:
     - tightened early associated-alias bookkeeping in `emit_struct` so aliases are recorded only when `ImplItem::Type` emission actually succeeds (not skipped in constrained mode).
     - changed module-mode trait runtime helper signatures to trailing explicit return form (`static auto ... -> <mapped-type>`) with `Self_` mapped through `decltype(self_)`, removing `auto` return-type drift while preserving shared generic lowering.
   - focused regressions:
     - `test_leaf10532_module_mode_assoc_alias_emitted_keeps_explicit_return_type`
     - `test_leaf10532_module_mode_struct_assoc_alias_skipped_still_softens_return_signature`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf10529 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf10532 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf415433_module_mode_trait_default_methods_emit_runtime_helper_and_keep_import -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler leaf415433_module_mode_trait_default_method_self_const_uses_self_alias -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `PATH=/tmp/rusty-fake-gpp-bin:$PATH tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10-5-32b-1775900450 --keep-work-dirs`
   - deterministic full-matrix frontier movement:
     - previous first hard-error family at `runner.cpp:1440/1456/737` (`auto` return deduction mismatch in Option-returning branches) is removed.
     - new first hard-error family is reference-element pointer-surface fallout at `runner.cpp:1228/1231` (`as_ptr`/`as_mut_ptr` pointer-to-reference declarations on `ArrayVec<const int&, 2>`), with adjacent storage-cast failures at `runner.cpp:1245`.
   - canonical artifacts:
     - previous head capture: `/tmp/rusty-parity-matrix-10-5-31-1775888784/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
     - post-fix matrix: `/tmp/rusty-parity-matrix-10-5-32b-1775900450/arrayvec/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and type-shape-gated in core codegen paths; no crate-specific rewrites/scripts or generated-text patching were introduced.
186. `Leaf 10.5.40.5` is complete.
   - plan/scope check: shared callable/format lowering fixes + focused regressions stayed below the <1000 LOC guardrail and required no additional decomposition.
   - implemented shared fixes in `transpiler/src/codegen.rs`:
     - method-item path arguments now lower to variadic forwarding wrappers (`receiver + args`) instead of unary wrappers, removing `inherent(value, input)` arity mismatch fallout.
     - `format_args!` now tracks native conversion chars per placeholder and applies integer-format bridging (`rusty::format_numeric_arg(...)`) for non-integer args on `x/X/o/b/B/d` conversions.
     - runtime fallback helper text now includes `rusty::format_numeric_arg(T&&)` with shape-gated extraction (integral passthrough, integral `_0` payload, integral `bits()` payload).
   - focused regressions:
     - `test_leaf105405_format_args_hex_spec_uses_native_numeric_argument`
     - `test_leaf105405_format_args_hex_spec_uses_numeric_bridge_for_non_integer_arg`
     - `test_leaf105405_method_reference_callable_wrapper_forwards_receiver_and_args`
     - `test_leaf105405_runtime_fallback_has_numeric_format_arg_helper`
   - verification:
     - `cargo test -p rusty-cpp-transpiler leaf105405 -- --nocapture`
     - `cargo test -p rusty-cpp-transpiler`
     - `PATH=/tmp/rusty-fake-gpp-bin:$PATH tests/transpile_tests/run_parity_matrix.sh --work-root /tmp/rusty-parity-matrix-10-5-40-5b-1775904094 --keep-work-dirs`
   - deterministic frontier movement:
     - removed prior `bitflags` Stage D head (`runner.cpp:2850/2966` method-item arity + `runner.cpp:3428+` format consteval family).
     - next deterministic `bitflags` Stage D head is `runner.cpp:1550` (`item._0` payload-shape mismatch) plus adjacent `runner.cpp:4379/4396/4413` `std::span<...>::from_iter` surface mismatch.
   - canonical artifacts:
     - pre-fix: `/tmp/rusty-parity-matrix-10-5-40-4a-1775902990/bitflags/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
     - post-fix: `/tmp/rusty-parity-matrix-10-5-40-5b-1775904094/bitflags/{baseline.txt,build.log,run.log,matrix.log,runner.cpp}`
   - guardrail check against wrong-approach checklist (§11): fixes stayed shared and shape-gated in callable/format lowering; no crate-specific scripts or generated C++ patching were introduced.
187. Current active next leaf is `Leaf 10.5.40.6` (deterministic `bitflags` Stage D iterator/collection call-shape family: `runner.cpp:1550`, `4379/4396/4413`).

### 10.7 Parity Harness and Matrix Command Reference

Primary single-crate parity run:

```bash
cargo run -p rusty-cpp-transpiler -- parity-test \
  --manifest-path tests/transpile_tests/<crate>/Cargo.toml \
  --stop-after run \
  --work-dir /tmp/rusty-parity-<crate>
```

Seven-crate matrix:

```bash
tests/transpile_tests/run_parity_matrix.sh
```

Single-crate matrix reprobe:

```bash
tests/transpile_tests/run_parity_matrix.sh --crate arrayvec
```

Useful options:

- `--work-root <dir>`
- `--keep-work-dirs`
- `--dry-run`

Cpp-module interop compile-stage check:

```bash
tests/transpile_tests/run_cpp_module_interop_compile.sh --work-dir /tmp/rusty-cpp-module-interop
```

Expected support-gating behavior:

- returns `PASS` when compiler+stdlib support `import std;` and both module units compile
- returns deterministic `SKIP` (exit 0) when host toolchain lacks `import std` module support

Failure diagnostics contract:

- first failing crate is printed
- canonical artifact paths are printed:
  - `baseline.txt`
  - `build.log`
  - `run.log`
  - matrix failure log

### 10.8 Documentation Maintenance Rule

When new parity work lands, update this section by topic, not timeline.

Required update pattern:

1. Add the technical change under the relevant §10.4 topic.
2. Update §10.5 with status movement only if matrix/crate frontier changed.
3. Update §10.6 only for active leaf/frontier changes.
4. Do not append new chronological "Leaf X" diary blocks here.

If detailed forensic history is needed, rely on:

- git history
- PR/commit messages
- `TODO.md` leaf chain

## 11. Design Constraints and Rejected Patterns (Consolidated)

This section replaces the prior long enumerated list with grouped constraints that are actively enforced in implementation.

### 11.1 Root-Cause-First Rule

Do not patch downstream compile noise before collapsing the first deterministic blocker family.

Why:

- downstream diagnostics are usually cascades
- fixing cascades first causes churn and regressions

### 11.2 No Global Text-Patching of Generated C++

Rejected pattern:

- string-based post-processing like replacing `return return`, `crate::`, or broad token rewrites

Required approach:

- AST-aware lowering at emission points with explicit context

### 11.3 No Blanket Rewrites Across All Call Sites

Rejected pattern:

- globally stripping references
- globally rewriting all method calls as free functions
- globally forcing constructor template args
- globally injecting expected-type numeric literal casts across unrelated expressions
- globally treating adapter-style methods (for example `.by_ref()`/`.take(...)`) as universal rewrites without receiver-shape/type checks

Required approach:

- apply rewrites only when a recognized shape and type context is present
- keep generic literal emission stable; perform conversions only in targeted coercion sites
- when omitted generic arguments have declared defaults, preserve defaults unless explicit type context requires otherwise (do not blindly capture in-scope generic names)
- for iterator adapters, gate lowering on iterator-like receiver inference so non-iterator methods with the same name are preserved
- for iterator adapter lowering, do not rely on a single direct-receiver item-type inference path; include callable-return and receiver-shape evidence gates so adapter chains (for example call-return iterators and `iter_names()`-style surfaces) do not leak raw `.map()`/`.count()` member calls into C++
- for callable-return iterator adapter lowering, do not require concrete `Item` extraction as the only evidence gate before rewriting adapters; associated iterator surfaces (for example `T::IterNames`) must still route through shared iterator helper lowering
- for bitflags helper merge lowering, do not emit direct self-recursive helper forwarders (`Self::bits(self)`, `Self::from_bits_retain(bits)`, and equivalent owner-qualified forms) that only dispatch back to the same method body; skip those wrappers so concrete helper surfaces remain non-recursive
- for synthetic bitflags helper signatures emitted in-class, do not use incomplete-type member probes (`decltype(std::declval<T>()._0)`) or deduced `auto` return types that are consumed before definition; emit concrete field-mapped bits types in signatures
- for statement `match` lowering that dispatches through `std::visit`, do not feed non-pointer variant scrutinees as address-of values (`&variant`) to the visitor; normalize scrutinee emission to variant value/reference shape before `std::visit`
- for statement `std::visit` arm lowering that contains `?`, do not leave try-returning arm lambdas without a deterministic fallthrough return path; keep try flow arm-local and synthesize a consistent tail return shape so overload resolution and control-flow checks remain well-formed
- for collect lowering, do not emit `Target::from_iter(...)` on non-owning C++ view targets (`std::span`, `std::string_view`, `std::basic_string_view`) that do not provide Rust-style constructor surfaces
- for map adapter lowering, do not rewrite optional-like payload maps (`next()` / `next_back()` receivers) into iterator helper calls; keep `Option::map`/`Result::map` semantics when receiver shape indicates optional next-payload surfaces
- for pointer-typed lowering, do not emit raw `Inner*` when `Inner` can be reference-shaped or dependent; prefer trait-form pointer aliases (`std::add_pointer_t<...>`) to avoid pointer-to-reference forms
- for optional-like lowering, do not preserve Rust `Option` method names on `std::optional`; normalize by inferred container surface (`has_value`/`value` vs `is_some`/`unwrap`)
- for runtime `Option`/`Result` match lowering, do not fall back to `std::visit` for nested binding-only payload patterns (for example `Err(Type { .. })`); keep dispatch on runtime helper surfaces (`is_err`/`unwrap_err`, `is_ok`/`unwrap`)
- for pointer helper calls (`ptr::read`, hole/reference storage APIs), do not cast value expressions directly to pointer aliases; emit address-of forms (`&expr`) before pointer-typed adaptation
- for runtime move-transfer helpers (`ptr::read`, `mem::replace`, `ptr::write`), do not rely on copy-return or copy-assignment fallbacks that require copyable payloads; preserve move-only behavior with move-out/placement-style reconstruction in shared runtime paths
- for `Drop`-bearing struct Rule-of-Five emission, do not default move-assignment operators; default move assignment can leave moved-from ownership live and trigger use-after-free/double-free during temporary teardown. Reconstruct through the move constructor (`this->~T(); new (this) T(std::move(other));`) so forgotten-address transfer semantics remain intact
- for range-bound visitor lowering (`start_bound`/`end_bound` in `drain`-style code), do not emit mixed return categories across `std::visit` alternatives and do not force slice helper results into pointer declarators (`const auto*`) when runtime helpers return span/slice values; unify return/value shapes from local type context first
- for runtime `Result` construction surfaces, do not default-construct inactive `T`/`E` storage arms in ways that require both payload types to be default-constructible; construct only the active arm and preserve move-only/non-default payload support
- for raw-pointer helper/receiver lowering (`as_ptr`/`as_mut_ptr`, `ptr::add`/`ptr::offset`), do not preserve storage-pointer pointee shapes when call context expects payload pointers; propagate expected pointer context and adapt pointee shape explicitly
- for runtime pointer helpers on `MaybeUninit`-backed storage (`as_ptr`/`as_mut_ptr`), do not expose wrapper-element pointers to payload-facing slice/read APIs; normalize helper results to payload pointers (`T*`/`const T*`) via shared runtime adaptation instead of crate-local rewrites
- for repeat/collection construction lowering, do not globally force fixed-array materialization from repeat helpers; gate array-vs-vector lowering on explicit expected-type/fixed-capacity context
- for tuple/assertion constructor scaffolding, do not emit bare `Ok(...)` / `Err(...)` without result-type context; always qualify through expected type or peer-derived constructor context
- for constructor/owned-payload forwarding, do not "fix" invalid moves by stripping `std::move` while keeping payload locals const; track consuming constructor payload bindings (including tuple/variant constructor calls and struct-literal owned-field forwarding) and emit those locals non-const so move construction remains valid where required
- for Result assertion parity, do not add one-off transpiler rewrites that bypass value comparison shape for specific call sites; maintain runtime `rusty::Result` equality surfaces (`operator==`/`operator!=`) so generated assertion scaffolding remains generic
- for `Into` conversion lowering, do not emit Rust trait-style member calls directly on literals/primitives (for example `("a").into()` in C++); lower through valid helper/context conversion surfaces instead
- for receiver-gated generic method args (`push(T)`/`insert(_, T)`/`set(T)`), do not block receiver-driven expected-type recovery just because the declared arg type placeholder is not in current scope; resolve concrete arg type from receiver context before conversion-lowering decisions
- for assertion/equality array-shape fallout, do not add fixture-specific `std::array` equality hacks or crate-local rewrites and do not special-case `assert_eq!` callsites; normalize compared element/container shapes through generic transpiler/runtime surfaces with explicit type/context gating
- for assertion/equality collection-shape fallout, do not hardcode `std::span` mixed equality only for STL-owned buffers (`std::vector`) and do not patch generated assertion callsites per crate; keep shared runtime mixed-view/owned equality surfaces covering Rust-owned containers (`rusty::Vec`) as well
- for omitted-template owner constructor fallout (`Type<auto, ...>::new_()`), do not hardcode crate/type-specific constructor rewrites or globally strip placeholder args; recover owner template args through explicit expected-type/scope inference gates so unaffected constructor sites keep their existing behavior
- for function-call template specialization, do not append inferred template arguments when explicit turbofish/template arguments are already present on the call path
- for tuple/option constructor coercion, do not globally replace `std::make_tuple(...)` / `std::make_optional(...)` with typed constructor forms; emit typed forms only when expected associated-type context requires coercion
- for expression-position block lowering (`[&]() { ... }()`), do not maintain ad-hoc local/statement emission paths that bypass shared shadow-allocation/state-tracking logic; route through shared statement/local lowering so `let x = x` shadow chains keep outer-binding resolution.
- for expected-type associated-call specialization, do not reuse only the mapped function-tail when the mapped path is actually a free helper (for example `std::mem::ManuallyDrop::new` → `rusty::mem::manually_drop_new`); emit `Owner::method(...)` only when mapped owner path matches the expected owner base
- for bound-match visit lowering, do not require explicit owner-qualified pattern paths (`Bound::Included`) as the sole trigger for runtime bound context; imported single-segment variants (`Included`/`Excluded`/`Unbounded`) in `start_bound`/`end_bound` matches must still route through runtime bound variant typing
- for QSelf associated helper-call lowering (for example `<T>::parse_hex(x)`), do not drop owner-qualified shape into bare `parse_hex(x)` free-function calls; preserve resolved owner type and emit explicit runtime helper template calls (`rusty::parse_hex<T>(x)`) when the mapped surface is runtime-scoped
- for borrowed `for`-loop lowering, do not rely only on syntactic `for ... in &expr` detection; include iterable type-shape evidence (reference-typed path bindings) so tuple/ref payload bindings keep reference semantics and unary deref lowering does not leak invalid `*input` forms on value-shaped C++ bindings
- for forward-declaration signatures, do not rely on later in-namespace `use` alias emission for single-segment imported type names; emit explicitly qualified paths when a unique declared crate type is known
- for forward-declaration module ordering, do not reuse delayable-function-namespace deferral that is intended for full definition emission; forward passes must keep dependency-first module order so sibling namespace type surfaces exist before dependent alias/function signatures
- for data-enum forward declarations, do not emit `struct EnumName;` when the generated enum surface is alias-based (`using EnumName = std::variant<...>`); mixed class-key/alias declarations for the same name are conflicting in C++
- for data-enum associated call lowering, do not treat every `Enum::name(...)` shape as a variant constructor; rewrite constructor syntax only when `name` is a declared variant identifier, and preserve non-variant associated methods (for example `Enum::from_inline(...)`) as method calls
- for local placeholder hint recovery via method-call receivers, do not require bare-identifier receiver shapes only; peel reference wrappers (`&` / `&mut`) before local-name resolution so typed-receiver inference still applies
- for method-call lowering on reference-wrapped receivers, do not emit fixed member-access operators from surface syntax (`expr.method(...)`) without post-lowering receiver-shape validation; select `.`/`->` from the lowered receiver type so `(&v)`-style forms remain type-correct
- for current-struct associated-type projections (`Self::Assoc` / `Type::Assoc`) that already resolve through impl-associated aliases, do not append omitted in-scope generic arguments during local-generic recovery; preserve the resolved alias surface instead of forcing owner-template argument injection
- for struct-literal field expected-type lowering, do not consume raw declared field metadata without substituting call-site owner type arguments (including expected-type inferred owner args); apply owner substitution before emitting field initializer expected types to avoid unresolved generic leakage (`T` vs concrete `A`) in nested associated constructor calls
- for tuple/binding assertion reference scaffolding, do not take addresses of coerced temporary expressions (for example `&std::string_view(expr)`); materialize coercions into stable temporaries before address-taking
- for closure payload and constructor-hint recovery, do not resolve closure parameter paths through outer local-shadow bindings; bind closure parameters in nested emission scope and avoid in-progress self-binding leakage during hint inference
- for function-item/path value bindings, do not emit unresolved or overloaded associated-function paths directly as C++ value initializers (for example `const auto s = rusty::String::from;`); lower through context-specialized callable wrappers or disambiguated callable forms
- for assertion tuple string-literal deref shapes, do not preserve borrowed `&*"literal"` RHS lowering as scalar `const char` comparisons; normalize through string-like coercion materialization (for example `std::string_view`) before tuple compare deref
- for consuming `self` return-path lowering (for example `into_iter()`), do not pass lvalue `(*this)` into move-only constructor surfaces; emit move/value-safe forms to avoid deleted-copy constructor fallout
- for struct-literal field lowering in consuming `self` scopes, do not bypass move insertion by using non-move field emission helpers; field payload emission must preserve receiver-aware move semantics
- for nested local-shadow initializer lowering (for example `let rhs = match rhs.next() { ... }`), do not hide outer same-name bindings before initializer emission or reuse outer same-name C++ shadow identifiers in inner scopes; preserve prior binding visibility and allocate distinct shadow identifiers to avoid self-reference/use-before-deduction
- for macro argument lowering (`format_args!`/friends), do not rely only on plain `syn::parse_str` over whitespace-expanded token text; keep shape-gated fallback lowering for method-context forms (for example `self . field`) so emission still routes through normal AST-aware field/receiver logic
- for circular type-ordering fallback, do not silently reorder true by-value SCCs and proceed without explicit unsupported diagnostics; emit deterministic cycle diagnostics so unsupported architecture gaps are visible at generation time
- for by-value SCC diagnostics, do not emit only unordered type sets; include deterministic cycle paths so failure fixtures can assert concrete cycle structure and avoid ambiguous diagnostics
- for opt-in by-value cycle breaking, do not enable rewriting by default and do not choose feedback edges with non-deterministic traversal order; require explicit activation and deterministic edge selection with emitted rewrite diagnostics
- for formatter associated-call lowering, do not preserve Rust associated method syntax (`Formatter::write_str(f, ...)`, `Formatter::write_char(f, ...)`) as static C++ member calls; lower to receiver-method form (`f.write_*`) so instance-only formatter surfaces compile
- for bare module `use` imports in nested scopes (`use crate::{iter};`), do not emit invalid `using ::iter;` and do not replace path-qualified module access needs with blanket `using namespace`; emit namespace-alias lowering (`namespace iter = ::iter;`) with deterministic top-level namespace forward declarations when required
- for namespace glob re-exports (`pub use external::*;`), do not emit `export using namespace ...`; keep namespace directives non-exported and source-order-safe
- for expression-form enum `match` lowering, do not drop struct-variant arms to duplicate generic visit fallbacks (`[&](const auto&) ...`); emit typed variant lambdas with explicit field bindings to keep `std::visit` overload sets unambiguous
- for empty block expressions (`{}`) in value position, do not fall back to `unreachable()`; lower to Rust unit value shape (`std::make_tuple()`)
- for data-enum struct-literal constructors (`Enum::Variant { ... }`), do not preserve scoped variant member syntax in C++; lower to concrete generated variant-struct targets (`Enum_Variant{...}`)

### 11.4 No Rust-Only Namespace Emission as C++ Symbols

Rejected pattern:

- emitting unresolved Rust imports as concrete C++ `using` declarations

Required approach:

- map to known runtime/C++ paths
- or emit explicit Rust-only comments when no valid C++ symbol exists

### 11.5 No Broad Export/Module Hacks

Rejected pattern:

- blanket `export` wrapping of nested namespaces
- forcing re-exports that violate C++ linkage constraints

Required approach:

- top-level export discipline
- guarded module-mode re-export suppression for linkage-sensitive symbols

### 11.6 No Signature De-Dup by Raw Rust Text

Rejected pattern:

- de-dup keyed only by raw Rust generics/where-clause text

Required approach:

- de-dup by emitted C++ signature shape

### 11.7 No Crate-Specific Ad-Hoc Scripts as Product Behavior

Rejected pattern:

- patching generated outputs per fixture
- hard-coded wrapper lists per crate

Required approach:

- generic transpiler/runtime fixes
- deterministic discovery in harness/matrix tooling

### 11.8 No Hidden Matrix Results

Rejected pattern:

- non-deterministic matrix output or missing artifact paths

Required approach:

- deterministic first-failure diagnostics
- stable artifact contract for repro

### 11.9 Known Architecture Gaps (as of 2026-04-09)

The following Rust→C++ translation gaps remain and require fundamental transpiler architecture changes to resolve:

1. **Circular type ordering** — Some Rust crates have circular module dependencies where `mod A` defines a struct used by `mod B`'s function signatures, and `mod B` defines a struct used by `mod A`. C++ requires types to be complete before use in `std::tuple<T>`, creating ordering cycles. Example: semver's `parse::Error` ↔ `Prerelease` ↔ `Version` cycle. Current status: by-value SCC detection + deterministic diagnostics are in place; opt-in cycle-breaking lowering design is documented in §11.9.1 and implementation remains pending.

2. **Rust iterator protocol** — `collect::<Vec<_>>()`, `into_iter()`, `map()`, `fold()` on C++ types. Rust desugars iterators through `IntoIterator` trait. No C++ equivalent exists for the full Rust iterator adapter chain. Fix requires: iterator trait protocol translation or runtime adapter layer.

3. **Trait instance method dispatch** — Default trait methods with `&self`/`&mut self` receivers can't be injected into implementing types because return types reference sibling namespace types causing name collisions. Example: `Flags::iter()` returns `iter::Iter<Self>` which collides with `tests::iter` namespace. Fix requires: qualified return type emission for injected trait methods.

4. **Deleted copy constructors in test runners** — The parity test runner's sequential execution uses `std::move(r)` on each variable use, but some variables are used multiple times. The multi-use detection works for transpiled code but not for test runner-generated assertion scaffolding. Fix requires: test runner awareness of variable lifetimes.

5. **Complex if-let patterns** — Some `if let` chain patterns (nested `if let Some(x) = expr.strip_prefix(...)`) emit `/* TODO: if-expression */` placeholders. Fix requires: comprehensive if-let chain lowering to C++ if-init statements.

6. **`format_args!` with advanced formatting semantics** — core `format_args!` lowering now emits concrete `std::format(...)`/`std::string(...)` with `rusty::to_string(...)` wrapping (including debug-spec rewrite and `self . field` expanded-token fallback), but full Rust formatting parity is still incomplete for richer named/trait-driven formatting behavior in some crates.

7. **Test namespace / function template name collision** — When expanded test code creates sub-modules with the same name as function templates in a sibling module (e.g., `mod parser { fn from_str<B>(...) }` alongside `mod parser { mod from_str { fn valid() } }`), the C++ `namespace from_str` shadows the function template `from_str<B>`. In C++, namespaces hide functions of the same name — no standard mechanism can override this. Attempted fixes: path qualification (fails because `parser::from_str` is ambiguous), namespace renaming (breaks all cross-references), using-declarations (can't disambiguate). Fix requires: comprehensive test-module path rewrite that prefixes test sub-modules with `_test` suffix and updates all references.

### 11.9.1 Design Note: Opt-In By-Value SCC Cycle Breaking

Goal:

- provide an explicit non-default path that can make true by-value SCC crates compilable in C++ when diagnostic-only mode is insufficient.

Activation contract:

- default behavior remains diagnostic-only (`// UNSUPPORTED: ...`) with no semantic rewrites.
- cycle breaking is activated only under an explicit opt-in flag in the transpiler CLI/runtime configuration.

Scope eligibility:

- input units are analyzed with the existing by-value SCC detector.
- only SCCs formed by by-value edges are eligible; reference/raw-pointer/indirection edges are not rewritten.

Proposed lowering strategy:

1. Build a by-value dependency graph with field-level edge metadata `(owner_type, field_name, target_type)`.
2. For each SCC, choose a deterministic feedback edge cut set (stable lexical ordering by owner, then field, then target).
3. Rewrite selected edges to indirection-form storage (default candidate: `rusty::Box<T>`), preserving non-selected edges.
4. Emit explicit diagnostics listing rewritten edges and the cycle path that motivated each rewrite.
5. Keep rewritten surfaces consistent across declarations/constructors/field initializers for affected types.

Safety and compatibility constraints:

- opt-in mode is allowed to change generated C++ layout/ABI for affected types.
- rewritten-edge diagnostics must be emitted to generated output and parity artifacts.
- cycle breaking must be deterministic across runs and independent of hash-map iteration order.

Non-goals for MVP:

- no automatic enablement in default parity runs.
- no crate-specific rewrite scripts.
- no deep semantic reconstruction of Rust ownership behavior beyond explicit indirection edge insertion.

### 11.10 Do Not Expand This Doc as a Chronological Diary

Rejected pattern:

- appending long leaf-by-leaf narrative under new numeric subsections

Required approach:

- integrate by topic under §10.4/§10.5/§10.6
- keep this document architectural and operational
