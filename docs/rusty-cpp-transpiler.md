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

```cpp
size_t fill(Cursor<span<const uint8_t>>& cursor, span<uint8_t> buf) {
    return cursor.read(buf).unwrap();
}
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

#### 3.11.1 Import Rewriting

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

#### 3.11.2 Runtime Path Lowering

Representative examples:

| Rust path | Lowered path |
|---|---|
| `core::intrinsics::unreachable` | `rusty::intrinsics::unreachable` |
| `core::panicking::panic_fmt` | `rusty::panicking::panic_fmt` |
| `std::str::Utf8Error` | `rusty::str_runtime::Utf8Error` |
| `std::char::from_u32` | `rusty::char_runtime::from_u32` |
| `std::io::Result<T>` | `rusty::io::Result<T>` |

The lowering table should be centralized; do not distribute these rewrites across unrelated emit paths.

#### 3.11.3 Associated/Omitted-Template Recovery

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

Directly supports:

- §1.4 Control flow
- §3.5 loop/break-value adaptation
- §3.7 unsafe/control-flow edge behavior

#### 10.4.6 Iterator, Slice, Range, and IO Surfaces

Integrated outcomes:

- `.iter()` / `.iter_mut()` lowering is mapped to shared runtime iterator helpers.
- `slice::Iter` / `slice::IterMut` type paths map to runtime iterator wrappers.
- range/slice/index shapes (`range*`, collect, buffer arg lowering, `map/fold` frontier) are handled incrementally with parity checks.
- io and string/char path families (`from_utf8*`, `encode_utf8`, boundary checks, formatter/debug chains) are lowered to runtime-safe targets.
- `MaybeUninit` reference-typed storage access is hardened to avoid pointer-to-reference emission shapes (pointer aliases via `std::add_pointer_t` and laundered storage access).
- mixed optional-like surfaces in iterator/test-shape lowering now normalize `std::optional` receiver methods (`is_some`/`is_none`/`unwrap` → `has_value`/`!has_value`/`value`) while preserving runtime `Option` surfaces.
- pointer-helper cast lowering now preserves reference payload address semantics even when pointer target types are emitted as alias forms (`std::add_pointer_t<...>`): reference-like cast sources are emitted as `&expr` before pointer-typed adaptation in generic cast/read paths.

Directly supports:

- §4.6 Iterators
- §3.3 pattern-driven iterator/match code
- §3.4 `?` in iterator-heavy contexts

#### 10.4.7 Module Emission, Ordering, and De-duplication

Integrated outcomes:

- inline module impl collection/merge is scoped and deterministic.
- duplicate methods are resolved by emitted C++ signature shape.
- forward declarations are emitted with guards to avoid alias-dependent type-order failures.
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

- latest full matrix run (post-11.1) advanced through 5 crates and stopped on first failure (`arrayvec` Stage D)
- confirmed passes in that run: `either`, `tap`, `cfg-if`, `take_mut`
- `semver` and `bitflags` were not reached in that specific run because matrix execution stops at first fail

Crate-focused progress integrated from former appendices:

- `either`: parity-control crate and expanded-test correctness hardening
- `tap`: literal/method-shape and extension-trait lowering fixes
- `cfg-if`: baseline resiliency and alias/import typing fixes
- `take_mut`: type/lifetime order, ptr/mem path lowering, and template/context fixes
- `semver`: import/re-export lowering and expanded build-shape fixes
- `bitflags`: re-export/type-order fixes and doc/baseline normalization
- `arrayvec`: remaining deterministic frontier, advanced through many Stage D blocker families

### 10.6 Active Frontier and Next Work

From the active TODO frontier, the currently active leaf work is now in the `take_mut` Stage D chain.

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
7. Current active next leaf is `Leaf 4.15.4.3.3.3.3.3.12.1`.
   - collapse the new `arrayvec` Stage D fixed-array repeat/constructor mismatch generically, then rerun matrix.

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
- for pointer-typed lowering, do not emit raw `Inner*` when `Inner` can be reference-shaped or dependent; prefer trait-form pointer aliases (`std::add_pointer_t<...>`) to avoid pointer-to-reference forms
- for optional-like lowering, do not preserve Rust `Option` method names on `std::optional`; normalize by inferred container surface (`has_value`/`value` vs `is_some`/`unwrap`)
- for pointer helper calls (`ptr::read`, hole/reference storage APIs), do not cast value expressions directly to pointer aliases; emit address-of forms (`&expr`) before pointer-typed adaptation
- for repeat/collection construction lowering, do not globally force fixed-array materialization from repeat helpers; gate array-vs-vector lowering on explicit expected-type/fixed-capacity context

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

### 11.9 Do Not Expand This Doc as a Chronological Diary

Rejected pattern:

- appending long leaf-by-leaf narrative under new numeric subsections

Required approach:

- integrate by topic under §10.4/§10.5/§10.6
- keep this document architectural and operational
