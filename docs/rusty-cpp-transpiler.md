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

From the active TODO frontier, the currently active leaf work is now in the `arrayvec` Stage D chain.

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
74. Current active next leaf is `Leaf 4.15.4.3.3.3.3.3.27.22.1`.
   - focus: implement generic runtime/transpiler dispatch for `clone_from_slice` when slice-like receivers lower to `std::span`, then re-run matrix.

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
- for runtime `Option`/`Result` match lowering, do not fall back to `std::visit` for nested binding-only payload patterns (for example `Err(Type { .. })`); keep dispatch on runtime helper surfaces (`is_err`/`unwrap_err`, `is_ok`/`unwrap`)
- for pointer helper calls (`ptr::read`, hole/reference storage APIs), do not cast value expressions directly to pointer aliases; emit address-of forms (`&expr`) before pointer-typed adaptation
- for raw-pointer helper/receiver lowering (`as_ptr`/`as_mut_ptr`, `ptr::add`/`ptr::offset`), do not preserve storage-pointer pointee shapes when call context expects payload pointers; propagate expected pointer context and adapt pointee shape explicitly
- for runtime pointer helpers on `MaybeUninit`-backed storage (`as_ptr`/`as_mut_ptr`), do not expose wrapper-element pointers to payload-facing slice/read APIs; normalize helper results to payload pointers (`T*`/`const T*`) via shared runtime adaptation instead of crate-local rewrites
- for repeat/collection construction lowering, do not globally force fixed-array materialization from repeat helpers; gate array-vs-vector lowering on explicit expected-type/fixed-capacity context
- for tuple/assertion constructor scaffolding, do not emit bare `Ok(...)` / `Err(...)` without result-type context; always qualify through expected type or peer-derived constructor context
- for constructor payload forwarding, do not "fix" invalid moves by stripping `std::move` while keeping payload locals const; track consuming constructor payload bindings and emit those locals non-const so move construction remains valid where required
- for Result assertion parity, do not add one-off transpiler rewrites that bypass value comparison shape for specific call sites; maintain runtime `rusty::Result` equality surfaces (`operator==`/`operator!=`) so generated assertion scaffolding remains generic
- for `Into` conversion lowering, do not emit Rust trait-style member calls directly on literals/primitives (for example `("a").into()` in C++); lower through valid helper/context conversion surfaces instead
- for receiver-gated generic method args (`push(T)`/`insert(_, T)`/`set(T)`), do not block receiver-driven expected-type recovery just because the declared arg type placeholder is not in current scope; resolve concrete arg type from receiver context before conversion-lowering decisions
- for assertion/equality array-shape fallout, do not add fixture-specific `std::array` equality hacks or crate-local rewrites and do not special-case `assert_eq!` callsites; normalize compared element/container shapes through generic transpiler/runtime surfaces with explicit type/context gating
- for omitted-template owner constructor fallout (`Type<auto, ...>::new_()`), do not hardcode crate/type-specific constructor rewrites or globally strip placeholder args; recover owner template args through explicit expected-type/scope inference gates so unaffected constructor sites keep their existing behavior
- for local placeholder hint recovery via method-call receivers, do not require bare-identifier receiver shapes only; peel reference wrappers (`&` / `&mut`) before local-name resolution so typed-receiver inference still applies
- for method-call lowering on reference-wrapped receivers, do not emit fixed member-access operators from surface syntax (`expr.method(...)`) without post-lowering receiver-shape validation; select `.`/`->` from the lowered receiver type so `(&v)`-style forms remain type-correct
- for tuple/binding assertion reference scaffolding, do not take addresses of coerced temporary expressions (for example `&std::string_view(expr)`); materialize coercions into stable temporaries before address-taking
- for closure payload and constructor-hint recovery, do not resolve closure parameter paths through outer local-shadow bindings; bind closure parameters in nested emission scope and avoid in-progress self-binding leakage during hint inference
- for function-item/path value bindings, do not emit unresolved or overloaded associated-function paths directly as C++ value initializers (for example `const auto s = rusty::String::from;`); lower through context-specialized callable wrappers or disambiguated callable forms
- for assertion tuple string-literal deref shapes, do not preserve borrowed `&*"literal"` RHS lowering as scalar `const char` comparisons; normalize through string-like coercion materialization (for example `std::string_view`) before tuple compare deref
- for consuming `self` return-path lowering (for example `into_iter()`), do not pass lvalue `(*this)` into move-only constructor surfaces; emit move/value-safe forms to avoid deleted-copy constructor fallout
- for struct-literal field lowering in consuming `self` scopes, do not bypass move insertion by using non-move field emission helpers; field payload emission must preserve receiver-aware move semantics

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
