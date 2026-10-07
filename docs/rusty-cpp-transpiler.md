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

### 2.2.0 Let bindings of reference-returning calls

When `let x = recv.method()` calls a method whose return type is `&T` or `&mut T`, the C++ emit needs `auto&` (or `const auto&`) — not plain `auto`, which would decay the reference to a value copy.

The transpiler recognizes reference-returning methods by:
- Direct return-type inference (when the method's signature is reachable through impl-block lookup).
- A method-name heuristic for the cases where lookup fails (e.g. cross-module calls through generic-T receivers). The heuristic matches names ending in `_mut` or `_ref`, plus a specific list (`get_mut`, `force_mut`, `as_mut`, `deref_mut`, `into_mut`, `borrow_mut`, `reborrow`, `unwrap`-after-ref-chain, etc.).
- Peeling through `unsafe { ... }`, `{ ... }`, parens, and groups so `let x = unsafe { recv.reborrow() }` is recognized the same as the bare call.

When the heuristic fires, the binding is emitted as `auto& x = …` (mutable) or `const auto& x = …` (immutable), preserving the reference through the C++ binding.

### 2.2.0.1 Owner template-arg recovery for absorbed methods (Cluster A)

When an impl-block's generics structurally decompose into a host-class template arg (the "Cluster A" pattern from the BTreeMap port), method bodies absorbed into the host class lose those impl-block generics from scope. Direct references are already substituted with `typename __TemplateArgs<HostParam>::arg_<N>` (§Cluster A completion), but path-level recovery — picking the right template args for a call like `LeafNode::new(alloc)` — needed a separate fix.

The recovery in `recover_omitted_owner_generic_args_from_scope` walks the called owner's declared params (e.g. `K, V` on `LeafNode`). When a declared param is not in lexical scope as a plain ident, the recovery now consults the current method's structural decomposition: if the same name appears as an impl-block generic at a tracked inner-struct position, the recovery returns `typename __TemplateArgs<HostParam>::arg_<pos>` instead of falling back to the loose ordered-scope fallback (which would otherwise grab whatever else is in scope — `A` from a method's allocator template, `Node` from the host class — and emit nonsense like `LeafNode<A, Node>::new_(alloc)`).

The decomposition is keyed to the specific method being emitted (not iterated across all methods on the host), so different methods on the same host with different impl-block specializations don't cross-contaminate.

### 2.2.0.2 Self-recursive nested fns → Y-combinator lambda

A Rust `fn` declared inside a function body can recurse — name resolution treats it like a free function:

```rust
fn outer(n: u32) -> u32 {
    fn rec(x: u32) -> u32 {
        if x == 0 { 0 } else { rec(x - 1) + 1 }
    }
    rec(n)
}
```

The transpiler lowers nested fns to `const auto` lambdas. But a C++ `auto`-deduced lambda can't reference its own name in its body — the lambda's type isn't known until the initializer completes.

When `emit_nested_function` detects self-recursion (any token-stream mention of the fn's own name in its body), it emits a Y-combinator-shaped lambda:

```cpp
const auto rec = [](auto&& __self, uint32_t x) -> uint32_t {
    if (... x == 0) { return 0; }
    else { return __self(__self, x - 1) + 1; }
};
return rec(rec, n);
```

`__self` is prepended as the first parameter; inside the body, recursive calls become `__self(__self, args)`; at external call sites (subsequent statements in the same scope), `rec(args)` becomes `rec(rec, args)` to seed the combinator.

Scope tracking is per-block: when leaving the enclosing block, the recursive name is removed from the in-scope set so unrelated calls outside the block aren't rewritten.

### 2.2.0.3 Tuple-pattern match without scrutinee type info

A Rust `match scrutinee { (Pat0, Pat1) => …, … }` where `scrutinee` is a tuple-typed expression but the transpiler's value-level type inference can't see the tuple shape (most often: scrutinee is a method-call return whose owner-impl is resolved through deep generics) previously fell through to a `std::visit(overloaded { [&](auto&&) { unreachable(); }, … }, _m)` emit — `_m` is a `std::tuple`, `std::visit` is for `std::variant`, and both arms degraded to `unreachable()` because the catch-all in `emit_match_expr_visit` had no clause for `Pat::Tuple`.

The fix infers the scrutinee's tuple arity from the arm patterns themselves: if every arm is a `Pat::Tuple` of the same arity (plus optional `_` wildcards), the match routes to the value-conditions emit, which expands as:

```cpp
[&]() -> ReturnT {
    auto&& _m_tuple = scrutinee;
    auto&& _m0 = std::get<0>(_m_tuple);
    auto&& _m1 = std::get<1>(_m_tuple);
    if (_m0.is_none()) { auto&& y = _m1; return /* arm body */; }
    if (_m0.is_some()) { auto&& x = _m0.unwrap(); auto&& y = _m1; return /* arm body */; }
    rusty::intrinsics::unreachable();
}()
```

For pattern bindings inside the arm (e.g. `Some(x)`), the value-conditions path now also runs `allow_runtime_match_binding_payload_moves`, which strips `std::as_const(_).unwrap()` wraps so the bound payload is movable and the arm body can call non-const member functions on it.

Resolved as of `try_emit_let_match_return_statement_level`: see §2.2.0.4 for the let-pattern + match + early-return-arm case.

### 2.2.0.4 Statement-level `let pat = match { ⇒ return …, ⇒ (…) }`

A Rust statement of the form

```rust
let (a, b) = match make_pair() {
    (None, b) => return b,
    (Some(s), b) => (s + 1, b),
};
```

cannot lower as an IIFE: the arm's `return X` is a *non-local* return that exits the surrounding Rust function, but in C++ it would become lambda-local. With diverging arm types (one arm returns from the outer fn; the other arm yields a tuple), the IIFE's `auto` deduction also fails.

The transpiler recognises this shape at `Stmt::Local` emit time (`try_emit_let_match_return_statement_level`, hooked into `emit_stmt` before the default `emit_local` path) and lifts it to a statement-level sequence:

```cpp
auto&& _let_match_tuple = make_pair();
auto&& _let_match_m0    = std::get<0>(rusty::detail::deref_if_pointer(_let_match_tuple));
auto&& _let_match_m1    = std::get<1>(rusty::detail::deref_if_pointer(_let_match_tuple));
if (_let_match_m0.is_none()) {
    auto&& b = rusty::detail::deref_if_pointer(_let_match_m1);
    return std::move(b);
}
auto _let_match_result = [&]() {
    auto&& s = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_let_match_m0).unwrap());
    auto&& b = rusty::detail::deref_if_pointer(_let_match_m1);
    return std::make_tuple((static_cast<size_t>(s) + static_cast<size_t>(1)), b);
}();
auto [a, b] = _let_match_result;
```

Gates:
- The `let` pattern is `Pat::Tuple`.
- The init is a `match` (after peeling parens/groups).
- Exactly one arm is non-diverging (the success arm); at least one arm has a body that is a non-local `return X` (`expr_is_try_style_return_flow`).
- No arm has a `guard` (we can't easily split the guard test from the arm condition).
- Every arm pattern is `Pat::Tuple` of the same arity, matching the outer let-pat arity.
- `tuple_match_can_lower_as_value_conditions` accepts the arm shape.

Why the success arm sits inside an IIFE: inner pattern bindings (`Some(s), b`) can collide with the outer let-pat bindings (`a, b`). Wrapping the success arm in `[&]() { … }()` opens a fresh C++ scope so the inner `b` shadows safely; the IIFE's return value is then destructured into the outer let-pat via `auto [a, b] = _let_match_result;`. The IIFE's `auto` deduction also succeeds because only the single success arm contributes a value type — no divergent arms to deduce against.

This completes Item #11 of the BTreeMap port's `GENERIC_FIXES_PLAN.md`. The recursive `insert_recursing` in `library/alloc/src/collections/btree/btree_internal.rs` is the canonical instance; the transpiler now emits its outer `let-match-return` shape directly. Together with §2.2.0.5 (tuple `.N` SFINAE dispatch) the function body's inner `while (true)` loop also compiles without patcher intervention — `stub_insert_recursing` has been removed from the BTreeMap port's `post_transpile_patch.py`.

### 2.2.0.5 Tuple `.N` field access with unknown receiver type

In Rust, `t.0` / `t.1` reads the Nth tuple field. The C++ form depends on what `t` is:

- A transpiler-synthesized tuple-struct has named members `_0`, `_1`, … so the right lowering is `t._N`.
- A `std::tuple<…>` (which is what the transpiler maps real Rust tuples to) requires `std::get<N>(t)`.

When the transpiler can infer the receiver's type (either directly via `infer_simple_expr_type` or, for `self.foo()` shapes, via the current struct's method-return table), it picks the right form: `std::get<N>` for tuple-typed receivers, `_N` otherwise. The hard case is when the receiver is `auto&&`-bound through a deref chain that the type-inference pass can't trace — e.g. in the BTreeMap port:

```cpp
auto&& handle = rusty::detail::deref_if_pointer(std::get<0>(...)._0);
auto kv      = handle.into_kv();   // kv: std::tuple<const K&, const V&>
return kv.1;                       // ← can't see kv as std::tuple
```

For this case, the emit path at `Expr::Field` → `Member::Unnamed` falls back to a SFINAE-discriminated IIFE that picks the right form at C++ compile time:

```cpp
([&](auto&& __t) -> decltype(auto) {
    if constexpr (requires { __t._N; })
        return (std::forward<decltype(__t)>(__t)._N);
    else
        return std::get<N>(std::forward<decltype(__t)>(__t));
})(kv)
```

Notes:

- The IIFE evaluates the receiver exactly once (avoiding double-evaluation of side-effecting expressions).
- `std::forward<decltype(__t)>` preserves the receiver's reference category — important for callers that mutate or move the field through the access.
- `decltype(auto)` plus the parens around `__t._N` lets the return propagate as a reference when the underlying field is a reference, matching Rust's semantics for `tuple.N`.
- The verbose form fires only in the third branch (type genuinely unknown). When `infer_simple_expr_type` returns `Some(_)` the simpler `_N` / `std::get<N>` shape stays.

This completes Item #1 of `GENERIC_FIXES_PLAN.md`; the patcher rule `fix_tuple_dot_underscore_access` is removed.

### 2.2.1 Match arms: const-value patterns

A Rust match arm of the form `CONST_NAME => …`, where `CONST_NAME` is a const item in scope, compares the scrutinee to the const's value (NOT a fresh variable binding):

```rust
const LEFT: usize = 5;
match idx {
    LEFT => "hit left",
    _    => "other",
}
```

In syn's AST this looks identical to a binding pattern (`Pat::Ident`), and the transpiler doesn't run a full name-resolution pass. As a heuristic, when the ident is SCREAMING_SNAKE_CASE (all uppercase letters/digits/underscores, at least one letter) the transpiler treats it as a const-value pattern and emits `if (_m == LEFT) { … }` instead of `{ const auto& LEFT = _m; … }`. This matches Rust's naming convention for consts.

A lowercase or mixed-case bare ident is still emitted as a fresh binding alias.

### 2.3 Arrays and Slices

| Rust | C++ |
|------|-----|
| `[T; N]` (array) | `std::array<T, N>` |
| `&[T]` (slice) | `std::span<const T>` (C++20) |
| `&mut [T]` | `std::span<T>` |

**Indexing**: Rust's slice methods `get_unchecked(i)` / `get_unchecked_mut(i)` lower to C++ `[i]`. When the receiver type is statically known to be an array/slice, the transpiler emits `recv[i]` directly. When the receiver is `auto&&`-bound (a deduced type from a method-call chain), the transpiler emits a `requires { recv[idx] }` SFINAE wrapper that picks `recv[idx]` for std::array/std::vector/std::span receivers and `recv.get_unchecked(idx)` for receivers that genuinely expose the named method (e.g. Rusty slice helpers).

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

#### Per-Crate Namespace Wrapping & the Cross-Crate Ownership Map

Rust namespaces by **crate**: `hashbrown::set::Difference` and `indexmap::set::Difference`
are unrelated types that never clash, because every path is rooted at its crate. C++ has no
such rooting — a `namespace set` emitted by one module and a `namespace set` emitted by
another **merge** into a single `::set`. So the moment two dependencies share a module name
(`set`, `map`, `iter`, `error` — extremely common) and one `import`s the other, their
same-named members collide:

```rust
// crate hashbrown            // crate indexmap (depends on hashbrown)
mod set {                     mod set {
    struct Difference<T,S>;       mod iter { struct Difference<T,S>; }
}                                 pub use self::iter::Difference;   // re-export
                              }
```

```cpp
// from module hashbrown
namespace set { template<class T,class S> struct Difference { … }; }   // hashbrown's
// from module indexmap, after `import hashbrown;`
namespace set { export using ::set::iter::Difference; }  // ERROR: ::set already has a
                                                         // (different) `Difference`
```

`hashbrown` compiles in isolation; `indexmap` breaks *only* because it imports hashbrown and
both define `::set`. The same root cause also mis-qualifies plain references — indexmap names
hashbrown's `hash_table::HashTable` as `::hash_table::HashTable` (a namespace that, post-fix,
belongs to `::hashbrown`) and re-declares hashbrown's `TryReserveError` locally.

**Solution — wrap every crate's purview in `namespace <crate> { … }`.** Then
`::hashbrown::set::Difference` and `::indexmap::set::Difference` are distinct, exactly as in
Rust. The wrap itself (`wrap_module_purview_in_crate_namespace`) is a textual insertion after
`export module …;` and its `import` lines; the work is the **re-qualification** it forces,
which must move in lockstep (the three rules below). The set of wrapped crates is gated by
`crate_is_namespace_wrapped`.

1. **Self-crate references** → `::<crate>::…`. A wrapped crate's own items live under
   `namespace <crate>`, so an unqualified or crate-rooted self reference (`indexmap::set::…`,
   or a bare `set::…` resolved against the crate root) must be emitted absolutely as
   `::<crate>::set::…` rather than escaping to the global `::set`.

2. **Cross-crate references** → `::<owner>::…`. When crate A names a type owned by a wrapped
   dependency B, it must emit `::B::<module-path>::<name>`. This is what makes
   `hash_table::HashTable` resolve to `::hashbrown::hash_table::HashTable` and suppresses the
   spurious local re-declaration / re-export forward-decl.

3. **The cross-crate ownership map** is the keystone that powers rule 2. Its data already
   flows through the **UFCS trait manifest** (§3.2.7): each manifest carries the emitting
   crate's name (`module`) and its `declared_types` (each with a within-crate `module_path`
   and `arity`). When a downstream crate merges a dependency manifest whose crate is
   namespace-wrapped, it records, for every declared type, the fully-qualified owner path
   `::<manifest.module>::<declared_type.module_path>::<name>` into its type→module-path map
   (`local_type_module_path`). Path emission then re-qualifies any reference to that name
   through this map. A type the consuming crate declares **itself** always wins (local
   declarations are collected first and are never overwritten), so genuine shadowing is
   preserved.

Because rules 1–3 are entangled, the wrap is applied **selectively and widened
incrementally, matrix-gated** — not flipped on for every crate at once. A measurement
(2026-06-27) wrapping *all* crates regressed 11 of 14 passing crates, because the
self-re-qualification has gaps that each need their own handling:

- **Shared namespaces.** Rule 1 deliberately re-qualifies only *exclusive* namespaces (those
  holding a type THIS crate declares), because a namespace shared with a dependency — serde's
  `de`/`ser` vs serde_core's `de`/`ser` — must keep resolving to the dependency. Naively
  re-qualifying every top-level `::<ns>::` breaks that.
- **Declared crate-root types.** Rule 3 collects crate-root types only from re-export shapes
  (`export using ns::T`, dropped imports, `using ::T;`). A type the crate *declares* at its
  root but never re-exports (either's `Either_Left`) is missed and escapes to global.
- **Non-type-holding own modules.** A module that holds only functions/impls (bitflags's
  `external`) is absent from the exclusive set (which is keyed on declared *types*), so its
  `::external::` self-references are not re-qualified.

So the wrap is enabled per-crate via `crate_is_namespace_wrapped` for crates that actually
collide (serde_bytes; hashbrown, which collides with indexmap on `set`/`map`/`iter`). Reaching
a universal wrap — where collisions *cannot* happen regardless of module name — requires
closing the three gaps above first, then widening one crate at a time. Crates whose module
names never collide with a dependency need no wrap in the meantime.

#### Cross-Module Declarations/Definitions: Rust vs C++

Rust and C++ modules look similar at a distance, but they have different ownership rules for declarations. This difference is a frequent source of parity failures when lowering expanded crates.

**Rust model**:

- Items are defined once at a module path (`crate::a::Type`).
- The compiler resolves the whole crate item graph; declaration order across files is mostly irrelevant.
- Other modules reference by path/import; they do not create competing declarations for the same nominal item.

**C++20 module model**:

- A declaration is attached to a named module unit.
- Redeclaring or defining the same nominal type in different named modules is tightly constrained by linkage rules.
- A forward declaration emitted in helper text is still a declaration surface, and can conflict with the owning module if emitted in the wrong place.

This is why valid Rust can fail after transpilation: we may accidentally duplicate declaration ownership while trying to make generated code compile in multiple modules.

##### Example A: Rust Cross-Module Type Usage (Always Legal in Rust)

```rust
// src/token.rs
pub struct Token {
    pub kind: u8,
}

// src/de.rs
use crate::token::Token;
pub fn peek_kind(t: &Token) -> u8 { t.kind }
```

Rust has one owner for `Token` (`crate::token`), and `de` only imports it.

##### Example B: C++ Module Lowering That Preserves Ownership (Good)

```cpp
// crate.token.cppm
export module crate.token;
export namespace token {
struct Token {
    uint8_t kind;
};
}

// crate.de.cppm
export module crate.de;
import crate.token;
export namespace de {
uint8_t peek_kind(const token::Token& t) { return t.kind; }
}
```

Only `crate.token` declares/defines `token::Token`. `crate.de` imports and uses it.

##### Example C: Anti-Pattern That Breaks in C++ Modules (Bad)

```cpp
// crate.de.cppm
export module crate.de;
namespace token { struct Token; }  // accidental helper predecl in non-owner module
```

```cpp
// crate.token.cppm
export module crate.token;
namespace token { struct Token { uint8_t kind; }; }
```

This pattern can trigger named-module attachment/linkage diagnostics because declaration ownership is now split across module units.

##### Why This Happens in Transpilers

Common triggers:

1. Emitting broad runtime helpers into many modules, where helpers contain nominal predeclarations (`namespace token { struct Token; }`).
2. Generating nominal type checks (`std::holds_alternative<token::Token_X>(...)`) in shared helpers that force foreign type declarations.
3. Flattened fallback assumptions carried into module mode (where declaration ownership matters more).

##### Solution Patterns (With Tradeoffs)

1. **Single-owner declaration rule (recommended default)**
   - Emit each nominal type declaration only in its owning module.
   - In non-owner modules, only `import` and reference.
   - Tradeoff: requires stronger owner tracking in codegen.

2. **Helper splitting by ownership**
   - Keep generic helper block type-agnostic and globally reusable.
   - Emit token-family/type-family helper overloads only in owner modules.
   - Tradeoff: slightly more complex helper selection logic.

3. **Index-based variant discrimination in shared code**
   - Prefer `variant.index() == I` and `std::get<I>(...)` when an index is known.
   - Fall back to constrained nominal checks only when safe.
   - Tradeoff: requires stable mapping from Rust variant path to C++ variant index.

4. **Consistent external linkage when sharing declarations across modules**
   - If a shared declaration must appear in multiple modules, keep declaration and definition under the same external linkage strategy.
   - Do not mix module-attached declarations with differently linked definitions for the same nominal type.
   - Tradeoff: can reduce strict module encapsulation and should be used sparingly.

5. **Dependency-ordered module graph + extraction module for cycles**
   - Build owner modules first.
   - For cycles, extract common nominal type surfaces into a smaller base module and import from both sides.
   - Tradeoff: introduces an extra module but keeps ownership explicit.

##### Transpiler-Level Rules of Thumb

Use this checklist when a crate fails with module attachment or cross-module type diagnostics:

1. Identify the first declaration site of the conflicting nominal type in generated `.cppm` files.
2. Confirm there is exactly one owner module for that type.
3. Remove/helper-gate foreign forward declarations in non-owner modules.
4. Replace nominal shared-helper checks with index/shape checks where possible.
5. Rebuild with deterministic module order and verify no non-serde regressions in the parity matrix.

##### Minimal Before/After for Helper Emission

Before (fragile):

```cpp
// injected everywhere
namespace token { struct Token; }
if (std::holds_alternative<token::Token_SeqEnd>(tok)) { ... }
```

After (robust):

```cpp
// generic helper injected everywhere
if (tok.index() == 7) { ... }  // or helper using known variant index

// token nominal declarations emitted only in token owner module
export namespace token { struct Token { ... }; }
```

Design rule: when Rust says "one item, many paths to it," emit C++ as "one owning module declaration surface, many importers."

#### Within-Module Definition Order & Incomplete Types (Method-Body Deferral)

The previous subsection is about *ownership* of a declaration across modules. This one is about *order* and *completeness* of definitions **within** a single module — a second place where valid Rust fails to compile after a naïve lowering.

**Why this is never a problem in Rust.** Rust compiles a crate in order-independent phases: it first collects every item's signature (types, fields, associated consts/fns, impls) into a global resolution table, and *then* type-checks bodies with full visibility of that table. Source order is irrelevant, and a type is never "incomplete" — once collected, all of its members are known. Mutually-recursive references (`RawIter` ↔ `RawTableInner`) are normal; the only cycle Rust rejects is one of *layout/size* (a struct containing itself by value), caught by a separate check.

```rust
// Order-independent: RawIter may appear BEFORE RawTableInner in source.
struct RawIter<T> { /* … */ }
impl<T> Default for RawIter<T> {
    fn default() -> Self { unsafe { RawTableInner::NEW.iter() } }   // fine
}
struct RawTableInner { /* … */ }
impl RawTableInner { const NEW: Self = /* … */; }
```

**Why C++ has it.** A C++ translation unit is scanned top-to-bottom. At any point a class is either *complete* (its full `{ … }` has been seen) or *incomplete* (only forward-declared). Naming a static member like `RawTableInner::NEW` requires the type to be **complete at that point in the scan**. If `RawIter`'s method body is parsed before `RawTableInner` is defined, the reference is ill-formed:

```cpp
template<typename T> struct RawIter {
    static RawIter<T> default_() { return RawTableInner::NEW.iter<T>(); }  // INLINE body
};                                  // ^ error: incomplete type 'RawTableInner'
struct RawTableInner { /* … */ };   //   named in nested-name-specifier
```

C++'s two-phase template lookup does **not** rescue this: it defers only *dependent* names (those mentioning the template parameter) to instantiation. `RawTableInner::NEW` is **non-dependent**, so it is resolved eagerly at the template's definition point — exactly where `RawTableInner` is still incomplete.

**The fix: method-body deferral (declare inline, define out-of-line).** The standard C++ answer to a definition cycle is to split declaration from definition: declare the member in the class, and define it *after* every type is complete. The transpiler does this for non-template structs — it emits the method **declaration** inline and queues the **definition** into a side buffer flushed after all type definitions. The target shape (shown here for a class template, the case discussed as a gap below) is:

```cpp
template<typename T> struct RawIter {
    static RawIter<T> default_();                 // declaration only (inline)
};
struct RawTableInner { /* … */ };                 // now complete
// … flushed after all types …
template<typename T>
RawIter<T> RawIter<T>::default_() {               // out-of-line definition
    return RawTableInner::NEW.iter<T>();           // RawTableInner is complete here
}
```

This declare-inline / define-out-of-line deferral is implemented in `emit_struct`'s deferral seam + `emit_method`'s out-of-line path (`codegen/emit_items.rs`): bodies are flushed after all type definitions. For a non-template struct the definition is qualified `Owner::method`; for a **class template** (e.g. `RawIter<T>`, hashbrown's `impl<T> Default for RawIter<T>` reading `RawTableInner::NEW`) it is `template<T…> Ret Owner<T>::method()`. Out-of-line definitions placed after the class but in the same C++20 module are valid for instantiation.

The class-template case needs two things the non-template case doesn't — both now handled:

1. **Template machinery.** Re-emit the class's `template<T…>` prefix and qualify as `Owner<T>::method`. The deferral records the class's arg list + prefix; the owner string carries its args (`Owner<T>`) so it doubles as the assoc-alias qualifier base (below).
2. **Completeness-aware triggering — and out-of-line signature-context reconstruction.** Deferral must fire **only for methods that actually fail inline** (those statically accessing a not-yet-complete sibling). A blanket "defer every template method" is both unnecessary and *breaks* methods that compiled fine inline, because an out-of-line signature loses the in-class context — an associated type in the return (`Result<Self, Self::Error>`) needs `Error` re-qualified to `typename Owner<T>::Error` (its `using Error = …;` lives in the body, too late for the signature). The trigger therefore uses emit-time **type-completeness tracking** (`defined_types`: a type is complete once its `emit_struct` has run; defer iff the body names a *still-incomplete* sibling via `Type::item`), and the out-of-line return/param types are assoc-qualified against the `Owner<T>` owner. Triggering on "body references *any* sibling" over-fires — it regressed arrayvec's `TryFrom::try_from` (whose return names an assoc type but whose siblings are all complete); completeness-awareness keeps such methods inline.

**Rules of thumb:**

- Prefer reconstructing a valid *definition order* over reordering type definitions — mutually-recursive types have no valid total order, and out-of-line bodies are the canonical break.
- A member body that names a *non-dependent* sibling type's static member/const is the trigger; making it dependent (the `Self_`/`.template` trick used for self-`sizeof`) does **not** help when the sibling is a fixed foreign type.
- For non-template structs deferral is blanket per deferrable struct; for class templates it is *selective* (completeness-aware) — deferring a template method that compiled fine inline can break it via out-of-line context loss. Any change here is validated against the full parity matrix.

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

### 3.2 Traits → Two Tiers: the C++/Rust Common Subset, and a Namespace Carrier for the Rest

> **Status (2026-10-07): ADOPTED — implementation in progress; §3.2.12 tracks what has landed.**
> Reviewed and adopted 2026-10-07 (design revised 2026-10-04). Until §3.2.12 says otherwise, the
> shipped transpiler still emits the 2026-06 lane described there.
> Third revision. 2026-09-23 said "adapters are the design; direct inheritance is a fast path."
> 2026-09-29 inverted it: **direct inheritance is the design for the subset where Rust and C++
> genuinely agree — the greatest common divisor, a bijection, idiomatic in both languages (tier 1);
> everything outside it is a second, one-way tier.** This revision keeps tier 1 unchanged and
> changes **tier 2's carrier**: the per-impl *body-carrying adapter class* of 09-23/09-29 is retired
> in favour of what the shipped transpiler already does in outline — a **per-trait namespace of free
> functions** — made correct by two additions measured on 2026-10-04 (§3.2.15): every impl function
> takes a per-trait **tag** as its first parameter, and every call goes through a per-method
> **customization-point object** (`Tr_::m(x)`), so lookup is argument-dependent at the point of
> instantiation and never frozen. The virtual interface survives as a **thin helper** for `dyn`:
> pure slots, and one generic forwarder per receiver kind per *trait* (not per impl), each slot one
> line into the namespace. §3.2.15 is the investigation record (incl. the six carrier probes);
> §3.2.16 the migration; §3.2.17 the tier-1 contract.
>
> This design replaces the earlier Microsoft-Proxy facade design; §4.7, §4.8, §5, §6, §8, §9 and
> §10.4.4 still carry Proxy remnants and are on the §3.2.16 sweep list.

Rust lets a trait *add methods to a type* from outside the type's definition — through concrete
impls, blanket impls, impls on foreign types, and default bodies — and resolves `x.foo()` by which
traits are in scope. C++ has none of that. But there is a large region where the two languages say
the same thing in different spelling: a trait whose methods are ordinary receiver methods,
implemented by a type you define, is *exactly* a C++ abstract class implemented by inheritance — one
interface, one `override` per method, `&dyn Tr` a reference to the interface. A C++ programmer would
write that by hand and a Rust programmer would write the trait by hand; neither would recognize the
other's code as generated. That region is **tier 1**: this design defines it precisely (§3.2.1),
emits it as that idiomatic C++ plus one named concept per trait (§3.2.2), and states the mapping as
a contract that is invertible except in two named cells (§3.2.17).

Everything Rust's trait system can express that C++'s class system cannot — impls on types you do
not own, blanket and conditional impls, generic required methods, `Self` in a non-receiver position,
a subtrait redeclaring a supertrait's name, two instantiations of one generic trait on one type — is
**tier 2**. Its carrier is the one C++ mechanism that is non-intrusive by construction: a **free
function taking the receiver as a parameter**, one per impl per method, living in the trait's
namespace or the receiver's, and found by argument-dependent lookup. Three things make that the
*trait system* rather than a pile of overloads: a per-trait **tag** type as every impl function's
first parameter (the ADL anchor and the trait identity, §3.2.2); a per-method **customization-point
object** every call site spells — `Speak_::speak(x)` — whose body performs the unqualified call
(§3.2.3); and a per-trait **marker + concept** (`impls_Tr`, `has_Tr`) that is the trait solver
(§3.2.3). Default bodies are written **once**, as templates in the namespace, and reached from both
tiers (§3.2.13). `dyn` is a thin interface with pure slots plus one generic forwarder per receiver
kind per trait (§3.2.10). Tier 2 is Rust → C++ only.

The governing constraint is unchanged and has a second consequence: the **transpiler stays lexical**
— name resolution and `impl`-shape classification, no type inference, no trait solving — so **tier
membership must itself be lexically decidable**. §3.2.1 gives the test and inventories which of its
inputs the collect pass records today. Which impl applies is still clang's decision: for tier 1 by
virtual dispatch and base-class tests, for tier 2 by overload resolution over the tag-anchored
overload set and by the marker.

**The question that drove this revision — "`x.speak()` on an `i32`: which namespace?"** Nobody looks
it up at the call site. Rust has already resolved the call to one trait (from `use`, a bound, or the
single impl in scope); the emitter *spells* that trait's namespace, `Speak_::speak(x)`. What is
looked up is the *impl* for `i32`, and that is ADL on `(Speak_::impl_::tag, int32_t)` inside the
CPO — which finds `impl Speak for i32` wherever it was declared, in any module, before or after the
caller. The shipped lane's `using namespace Speak_;` + bare `speak(x)` is the shape the probes showed
to be wrong — the directive leaks into child namespaces, a local named `speak` breaks the call, and a
qualified call inside a template freezes its overload set across modules (`11000` where rustc gives
`5550`, §3.2.15) — and it is retired.

| Rust trait usage | tier | C++ lowering |
|------------------|:---:|--------------|
| `trait Tr { fn m(&self); fn n(&mut self); fn k(self); }` | 1 | one `class Tr` with plain names: `virtual R m() const = 0; virtual R n() = 0; virtual R k() && = 0;` (§3.2.17) |
| `impl Tr for T`, `T` a struct this crate declares, at exactly its own parameters | 1 | `struct T : public Tr { R m() const override; … }` — the type *is* the interface |
| default that calls only this trait's / supertraits' slots | 1 | a non-pure `virtual` on the interface forwarding to the one namespace body (§3.2.13) |
| generic default `fn each<F>(&self, f: F) where Self: Sized`, and any default that calls it | 1 | a non-virtual **explicit-object** member on the interface, `template<class F> R each(this auto const& self, F f)`, forwarding to the namespace body — an implementor's override hides it and is reached (§3.2.13) |
| required method with `where Self: Sized`, or `fn new(..) -> Self`, or `const K` | 1 | a member / `static` / `static constexpr` on each implementor, plus a conjunct of the trait's concept (§3.2.17); not `dyn`-usable in Rust either |
| `trait Sub: Super` | 1 | `class Sub : public virtual Super` |
| `trait Tr<A>`, at most one `impl Tr<X> for T` per type | 1 | `template<class A> class Tr`; `struct T : Tr<X>` |
| `x.m()` when `x`'s *declared* type is lexically a tier-1 implementor of `Tr` | 1 | `x.m()` — a member call, no shim |
| `Tr::m(&x)` / `<T as Tr>::m(&x)`, same condition | 1 | `x.m()` — the same virtual member call, never `x.Tr::m()` (§3.2.3) |
| `&dyn Tr` / `&mut dyn Tr` / `Box<dyn Tr>` | 1 | `const Tr&` / `Tr&` / `rusty::Box<Tr>` — an upcast, no forwarder |
| `fn f<T: Tr>(x: &T)` | 1 | `template<class T> requires has_Tr<T> R f(const T& x)` — the concept, not `derived_from`, because the bound must also admit tier-2 implementors (§3.2.17, lossy cell 2) |
| `impl Tr for i32` / `String` / `Vec<T>` / a foreign type / a closure / `&T` | 2 | free functions `R m(Tr_::impl_::tag, const int32_t& self_, …)` in `Tr_::impl_` (§3.2.2); `x.m()` → `Tr_::m(x)` (§3.2.3) |
| `impl Tr for Local` where `(Tr, Local)` fails axis 2 (`impl Tr for W<i32>`, an extra bound, …) | 2 | the same free functions, in `Local`'s declaring namespace (§3.2.5) |
| blanket / conditional impl | 2 | a constrained function template per method, `template<class T> requires has_B<T> R m(tag, const T& self_)`, plus a constrained partial specialization of the marker (§3.2.2) |
| two instantiations of one generic trait on one type (`impl Tr<i32> for T` + `impl Tr<u8> for T`) | 2 (that type) | overloads distinguished by a trailing `rusty::tag<A>`; the interface stays tier 1 (§3.2.2) |
| `impl Tr for &T` beside `impl Tr for T` | 2 | overloads distinguished by a trailing `rusty::self_tag<Self>` (§3.2.2, §3.2.4) |
| generic *required* method; `Self` in a parameter or nested return without `where Self: Sized`; `-> impl Trait`; `async fn` | 2 | the trait is tier 2: no interface slot for it; a free function (template) in `Tr_::impl_` like any other (§3.2.2, §3.2.10) |
| a subtrait that redeclares a supertrait's method name | 2 | the subtrait's family uses per-trait slot names `Sub__m` for the redeclared name; the supertrait is untouched (§3.2.1) |
| one type with a tier-1 impl and a tier-2 impl (or an inherent method) that share a method name | — | legal; the call site's ladder decides by scope with base tests, never by member existence (§3.2.3, §3.2.6) |
| `x.m()` when the receiver's tier is not lexically known, or it may have a same-named inherent method | 2 | the member-first shim with a CPO arm (§3.2.3) |
| `&dyn Tr` over a tier-2 implementor | 2 | `TrAdapterRef<U>(u)` — one *generic* forwarder per trait, each slot `Tr_::m(value_)`; `&mut dyn` through a named `TrAdapterRefMut<U>` local (§3.2.10) |
| `use Tr;` | – | the module `import` only — **no `using namespace`**; the emitter spells `Tr_::` at each call (§3.2.5) |
| operator traits; associated types; std / prelude traits | – | unchanged: §3.2.9, §3.2.8, and dedicated lowering outside this section |

#### 3.2.1 The model: a greatest common divisor, and a second tier for the rest

**What "greatest common divisor" means here.** Take the set of things a Rust trait declaration,
impl, and call site can say; take the set of things a C++ abstract class, derived class, and member
call can say; intersect them; keep the part where the correspondence is one-to-one in both directions
*and* lexically recognizable on both sides. That intersection is tier 1. It is not "what happens to
compile" — a C++ shape that compiles but means something different from the Rust (a base member that
one override serves for two traits, an inherent method that silently becomes an override) is outside
it by construction. The correspondence table, with the C++-side grammar a hand-written interface must
satisfy, is §3.2.17.

**Tier membership is decided on two axes, both lexical, and neither depends on any other crate.**

*Axis 1 — the trait (decides the interface's shape and names).* A trait is tier 1 when:
- every method has a receiver (`&self`, `&mut self`, `self`) or is a no-receiver associated function;
- no *required* method is generic over a type parameter (`impl Trait` parameters and `async fn`
  desugar to one) or returns `impl Trait`;
- no method mentions `Self` outside its receiver — in a parameter, in the return type, or nested in
  either (`o: &Self`, `Option<&Self>`, `-> Option<Self>`; rustc's E0038 rule) — **unless the method
  carries `where Self: Sized`**. A method with `where Self: Sized` is a non-slot in both languages:
  a default one becomes an explicit-object member of the interface (§3.2.13), a required one a member
  of each implementor checked by the trait's concept (§3.2.17). A generic *default* must carry
  `where Self: Sized` (Rust requires it for the trait to stay `dyn`-compatible; E0038 otherwise),
  and so must every default that calls it;
- no method name is declared by both the trait and one of its transitive supertraits;
- every supertrait is itself tier 1.
Associated types and consts, and generic trait parameters, are allowed (§3.2.8, §3.2.17). A tier-1
trait's interface has **plain member names**. A trait that fails a test is tier 2 at the trait level:
no type inherits it directly. Two shapes get special treatment rather than wholesale demotion: a
subtrait that redeclares a supertrait's name mangles *that name only, in its own family* (`Sub__m`;
`class Sub : virtual Super` would otherwise fold two Rust methods into one slot — measured, §3.2.15),
leaving the supertrait's interface untouched; and a trait with `fn new()` / `const K` (without
`Self: Sized`) is tier 1 but not `dyn`-usable in Rust, so its `dyn` row is vacuous — C++ still
accepts `const Tr&` for it, a forward-only difference.

*Axis 2 — the impl (decides direct inheritance vs. adapter).* An impl of a tier-1 trait is tier 1 when:
- the self type, after resolving type aliases within the crate, is a named struct or data-carrying
  enum **declared in this crate**, and the impl's self type is that type applied to **exactly its own
  parameter list** — `impl Tr for T` for non-generic `T`, or `impl<X, Y> Tr for W<X, Y>` with each
  argument one of the impl's own parameters, used once, unbounded beyond the bounds the struct
  itself declares. A C++ base is a property of the class template, not of one instantiation, so any
  other shape — a concrete argument (`impl Tr for W<i32>`), a partial one (`W<In<T>>`), a repeated
  one (`W<T, T>`), an extra bound (`impl<T: B> Tr for W<T>`), a blanket (`impl<T> Tr for T`) — has no
  base clause (measured: the base lands on the template and `W<&str>` becomes an implementor of the
  `i32` body, §3.2.15) and is tier 2: free functions keyed on the full self type
  (`R m(tag, const W<int32_t>& self_)`; `template<class T> R m(tag, const W<In<T>>& self_)`), §3.2.2;
- the self type is not `repr(C)` / `repr(transparent)` (decision (r));
- the self type declares no inherent method with the same name as any method of a trait it
  implements at tier 1 (C++ would silently make the inherent one the override);
- no two *concrete* tier-1 impls on the type declare the same method name (the type cannot inherit
  both with plain names); and the type has at most one instantiation of any generic trait (a
  second instantiation would fold every method that does not mention the differing argument in a
  parameter position into one slot — ill-formed for a return-only argument, silently one body for
  an argument-free method; measured, §3.2.15);
- for every transitive supertrait `Super`, `impl Super for T` is itself a concrete tier-1 impl in this
  crate. Otherwise `struct T : Sub` with `class Sub : virtual Super` would inherit `Super`'s default
  body where the supertrait's real impl is an adapter (measured: `1 0 0` where rustc gives `1 999 999`).
Every other impl of a tier-1 trait — a primitive, a std or dependency type, a reference, slice, tuple,
array, closure, blanket, or any of the shapes above — is tier 2 at the impl level: free functions in the
trait's namespace, reached through the trait's CPOs, with the trait's generic forwarder standing in for
the object under `dyn` (§3.2.10).

**The two axes compose, and the composition is honored at the call site, not by the class.** A
tier-1 member is a C++ member: visible on the type unconditionally, in every translation unit,
whatever traits the Rust call site had in scope — and indistinguishable to `requires { r.m(); }` from
an inherent member. So a type with a tier-1 impl and a tier-2 impl that share a method name is
*legal* (a crate-local type with `impl Tr1 for T` inherited and `impl Tr2 for T` tier 2 because
`Tr2` has a generic required method; or, across crates, an orphan-legal downstream `impl B::Tr2 for
A::T` that `A` never sees) — its calls go through the shim, whose arms test *bases*, not names
(§3.2.3), and whose ambiguity count uses the trait concepts (§3.2.6). No per-type exclusion evaluated
in the type's crate can stand in for that rule, because the second impl may live in a crate the
type's crate never sees. Axis 2's same-name exclusion therefore covers only *concrete* impls the
pre-pass can enumerate; impls a type acquires through a blanket or conditional impl are never
enumerated per type (deciding whether one applies is trait solving, §3.2.11) and are handled by the
call site.

**Why no test depends on another crate.** Axis 1 reads only the trait declaration. Axis 2 reads only
this crate's impl blocks and type declarations. The one test that *looked* program-wide in an earlier
draft — "no impl overrides this generic default" — is gone: a generic default is an explicit-object
member, and a downstream override reaches it by name hiding without the upstream interface changing
(§3.2.13; measured across a precompiled module boundary, §3.2.15). Consequently a downstream crate
never changes an upstream emission; a translation may still be whole-program (it always has the
whole Rust dependency graph, and may use that for optimization), but correctness does not rely on it,
and a trait exposed to hand-written C++ implementors needs no freezing rule.

**What the boundary costs in coverage.** A census of the local parity-matrix crates (§3.2.16 (p))
finds roughly **4 of ~288** crate-trait `(trait, impl)` pairs in tier 1. Library crates are made of
blanket impls, generic required methods (`serialize<S>`), `Self`-taking methods (`PartialEq`-shaped),
and impls on foreign types; tier 1's coverage there is negligible by construction. Tier 1 is the
tier of *application* code and of the C++-interop surface — the code a team writes against its own
types — which is where a bijective, hand-writable mapping pays. The census is the gate metric for
any widening (§3.2.16 (p)).

**Why the boundary is lexical, and what the collect pass records today.** Every test reads the
trait declaration, the impl header, or the crate's own type declarations. Inventory (transpiler
locations in §3.2.16 phase 1): receiver kind — recorded (`trait_method_receiver_kind`); generic /
`impl Trait` / `async` / `-> Self` / `Self` in parameters / `where Self: Sized` — decided per method
at emit time in `emit_trait_interface_pattern`, **not** recorded as a trait-level fact, and the
`Self`-in-parameter and `where Self: Sized` cases are not consulted at all; supertrait lists — read
from syn at emit time, not stored, absent from the manifest; self type declared here —
`local_declared_types`, which also contains type aliases (must resolve); self-type arguments vs the
struct's parameters — not computed (`cpp_inherit` keys on the simple name); blanket / conditional
presence — not recorded generally; inherent same-name overlap — `inherent_impl_method_names` exists,
the overlap test does not; two concrete impls sharing a name on one type — no per-type
implemented-traits map; `repr` — not recorded. The method-name classifier is global
(`classify_method_names_excluding_traits`, transpile.rs), not per type. Phase 1's pre-pass computes
all of these; phase 1's manifest carries per trait its tier, its supertraits, the names of its
non-vtable defaults, and tier-1 traits as owners in `method_owners`, and per dependency type its
inherent method names — the facts the call-site rules of §3.2.3 and §3.2.6 need on a dependency
receiver.

**Slot-name convention used in the rest of this section.** Where this section writes a slot as `Tr__m`
(the §3.2.15 record, and a tier-2 trait's forwarders), read it as *the trait's slot name*: plain `m` for a
tier-1 trait — the common case, and the only case a hand-written C++ implementor ever sees — and `Tr__m`
only for a tier-2 trait's redeclared names. Tier-2 *impl functions* never carry it: their identity is the
tag (§3.2.2), so `m(Sub_::impl_::tag, …)` and `m(Super_::impl_::tag, …)` are distinct without mangling.

#### 3.2.2 Static dispatch: tier 1 is a member call; tier 2 is a free function in the trait's namespace

Classification is syntactic from the `impl` block shape and the §3.2.1 tests:

- `impl T { fn m … }` (no `for`) → **inherent** → a member `m` of `T`. Unchanged.
- `impl Tr for T` with `(Tr, T)` at **tier 1** → `T` inherits `Tr`; `m` is an `override` on `T`.
- `impl Tr for T` with `(Tr, T)` at **tier 2** → `m` is a **free function** `m(Tr_::impl_::tag, const T& self_, …)`
  in the trait's namespace or `T`'s, reached through the CPO `Tr_::m` (below).

**Tier 1.**

```rust
trait Greet { fn hello(&self) -> String; fn rename(&mut self, s: &str); }
#[derive(Clone)] struct Foo { x: i32 }
impl Foo { fn bar(&self) -> i32 { self.x } }
impl Greet for Foo { fn hello(&self) -> String { … } fn rename(&mut self, s: &str) { … } }
fn main() { let mut f = Foo { x: 1 }; f.hello(); f.rename("a"); Greet::hello(&f); let g = f.clone(); }
```
```cpp
class Greet {                                          // the interface: what a C++ author writes
public:
    virtual ~Greet() noexcept(false) {}                // noexcept(false): a Rust Drop may unwind (§3.2.17)
    virtual rusty::String hello() const = 0;
    virtual void rename(std::string_view s) = 0;
protected:                                             // C.67: protected + defaulted, NOT deleted — a deleted
    Greet() = default;                                 //   base copy would delete every implementor's copy/move
    Greet(const Greet&) = default; Greet& operator=(const Greet&) = default;
    Greet(Greet&&) = default;      Greet& operator=(Greet&&) = default;
};
template<class U> struct impls_Greet : std::false_type {};        // marker primary: emitted for EVERY trait (bounds,
template<class U> concept has_Greet = impls_Greet<U>::value       //   later tier-2 impls); a tier-1 impl does NOT
                                    || std::derived_from<U, Greet>;//   specialize it — is_base_of covers it
namespace Greet_ { /* tag, CPOs, default templates, bridges: tier 2, below */ } // + 3 generic forwarders, likewise

struct Foo : public Greet {                            // the impl: inheritance
    int32_t x;
    Foo(int32_t x) : x(x) {}                           // a base kills aggregate-init → the emitter writes the ctor
    Foo(const Foo&) = default; Foo(Foo&&) = default;   // derive(Clone): explicit; the emitter must NOT synthesize
    Foo& operator=(const Foo&) = default; Foo& operator=(Foo&&) = default;   //   a lone move ctor (it deletes copy)
    Foo clone() const { return *this; }                // never a designated initializer on a non-aggregate
    int32_t bar() const { return x; }                  // inherent, unchanged
    rusty::String hello() const override;              // bodies out-of-line, as any member
    void rename(std::string_view s) override;
};
// main:  f.hello();  f.rename("a");  f.hello();  /* Greet::hello(&f) is the SAME call */  Foo g = f;
```

There is no shim and no free function *for this impl*; the trait's module still emits the
marker primary, the concept, the `Greet_` namespace and the generic forwarders, because a bound anywhere (`T:
Greet`) must admit a later tier-2 impl and the trait's crate cannot know whether one exists
(§3.2.17). The call is `f.hello()`; path syntax `Greet::hello(&f)` is the *same* member call — Rust's
`<Foo as Greet>::hello` *is* Foo's override — and the C++ qualified call `f.Greet::hello()` is
**never** emitted: it suppresses virtual dispatch and runs the base body (a default's, or an undefined
reference for a pure slot — measured, §3.2.17). The shipped fast path emits this shape for a single
non-generic trait per type (§3.2.12). Emission details that belong to tier 1 rather than to the type:
a struct that gains a base is no longer an aggregate, so the emitter writes the fieldwise constructor
every `Foo{…}` literal needs and never emits a designated initializer for it (the shipped `clone()`
does); the interface's copy and move are **protected and defaulted, never deleted** (Core Guidelines
C.67 — a deleted base special member implicitly deletes every implementor's, measured §3.2.17); and
the emitter must not synthesize a lone move constructor on an implementor, since a user-declared
move constructor deletes the copy constructor regardless of the base (measured, §3.2.15) — a `Copy`
or `Clone` implementor gets all four defaulted, a non-`Clone` one gets nothing (and is then
copyable in C++ where Rust forbids it: a forward-only difference, §3.2.17).

**Tier 2.** For an impl outside axis 2, or any impl of a tier-2 trait, the body is a **free function**
whose first parameter is the trait's tag and whose second is the receiver, spelled by the Rust
receiver kind (`const U&`, `U&`, `U`). It is the shipped lane's `Tr_::m(const U& self_)` with the tag
added and the call protocol changed; nothing inherits anything. Per trait, emitted once in the
trait's module:

```rust
trait Speak { fn speak(&self) -> i32; fn bump(&mut self, d: i32); fn consume(self) -> i32;
              fn twice(&self) -> i32 { self.speak() * 2 } }
impl Speak for i32 { fn speak(&self) -> i32 { *self } fn bump(&mut self, d: i32) { *self += d; } fn consume(self) -> i32 { self } }
impl Speak for Foo { … }                                    // Foo: crate-local, tier 1 — unchanged, above
impl<T: Score> Speak for T { … }                            // blanket
```
```cpp
// ===== trait Speak — in Speak's module, exported =====
class Speak;                                                                         // the thin dyn helper, defined below
template<class U> struct impls_Speak : std::false_type {};                          // (5) marker: DEFINED false (§3.2.3)
template<class U> concept has_Speak = impls_Speak<U>::value || std::derived_from<U, Speak>;   // (6) what every bound emits
namespace Speak_ {
    namespace impl_ { struct tag {}; }                     // (1) ADL anchor = trait identity: parameter 0 of EVERY impl function
    // (2) one customization-point object per method — the ONLY spelling a call site uses. Trailing args (incl. tags) forwarded.
    inline constexpr struct speak_fn {
        template<class S, class... R> auto operator()(S&& s, R&&... r) const
            -> decltype(speak(impl_::tag{}, std::forward<S>(s), std::forward<R>(r)...))   // trailing decltype: SFINAE-friendly
            { return speak(impl_::tag{}, std::forward<S>(s), std::forward<R>(r)...); }    // UNQUALIFIED: ADL on (tag, S) at instantiation
    } speak{};
    inline constexpr struct bump_fn    { /* same shape */ } bump{};
    inline constexpr struct consume_fn { /* same shape; the receiver arrives as an rvalue */ } consume{};
    inline constexpr struct twice_fn   { /* same shape */ } twice{};
    namespace impl_ {
        // (3) tier-1 bridge: a receiver that INHERITS the interface is reached through its virtual member — one per method
        template<class S> requires std::derived_from<S, Speak> int32_t speak(tag, const S& s) { return s.speak(); }
        template<class S> requires std::derived_from<S, Speak> void    bump(tag, S& s, int32_t d) { s.bump(d); }
        template<class S> requires std::derived_from<S, Speak> int32_t consume(tag, S s) { return std::move(s).consume(); }
        template<class S> requires std::derived_from<S, Speak> int32_t twice(tag, const S& s) { return s.twice(); }
        // (4) the default body, written ONCE (§3.2.13); `self.speak()` → the CPO, never a member call
        template<class S> int32_t default_twice(const S& s) { return Speak_::speak(s) * 2; }
        template<class S> requires (has_Speak<S> && !std::derived_from<S, Speak>)
        int32_t twice(tag, const S& s) { return default_twice(s); }                  // reachable for tier-2 receivers only
    }
}
// (7) the thin dyn helper: pure slots; its own default forwards into the namespace body (§3.2.10, §3.2.13)
class Speak {
public:
    virtual ~Speak() noexcept(false) {}
    virtual int32_t speak() const = 0;
    virtual void    bump(int32_t d) = 0;
    virtual int32_t consume() && = 0;
    virtual int32_t twice() const { return Speak_::impl_::default_twice(*this); }   // tier-1 default: the ONE body, via the bridge (measured: census `describe`)
protected: /* C.67 protected + defaulted special members, as tier 1 above */
};
template<class U> class SpeakAdapterRef final : public Speak {                      // &dyn Speak over a tier-2 U — GENERIC, never specialized
    const U& value_;
public:
    explicit SpeakAdapterRef(const U& u) : value_(u) {}
    int32_t speak() const override   { return Speak_::speak(value_); }               // every slot: one line into the CPO
    void    bump(int32_t) override   { rusty::unreachable_via_const_dyn(); }         // stub: const value_ cannot call bump(U&)
    int32_t consume() && override    { rusty::unreachable_via_const_dyn(); }         // stub: cannot consume through const U&
    int32_t twice() const override   { return Speak_::twice(value_); }               // U's override if any, else the default template
};
template<class U> class SpeakAdapterRefMut final : public Speak { /* U& value_; bump forwards; consume is the stub */ };
template<class U> class SpeakAdapter       final : public Speak { /* U value_; every slot forwards; consume: Speak_::consume(std::move(value_)) */ };

// ===== impl Speak for i32 — in the impl-emitting module (here Speak's own: E0117, §3.2.13) =====
namespace Speak_::impl_ {
    int32_t speak(tag, const int32_t& self_);                  // phase 1: declarations (Rust allows any order; §3.2.3)
    void    bump(tag, int32_t& self_, int32_t d);
    int32_t consume(tag, int32_t self_);
}
template<> struct impls_Speak<int32_t> : std::true_type {};
// ===== impl<T: Score> Speak for T — a blanket: constrained function templates + a constrained partial marker =====
namespace Speak_::impl_ {
    template<class T> requires has_Score<T> int32_t speak(tag, const T& self_);
    template<class T> requires has_Score<T> void    bump(tag, T& self_, int32_t d);
    template<class T> requires has_Score<T> int32_t consume(tag, T self_);
}
template<class T> requires has_Score<T> struct impls_Speak<T> : std::true_type {};
// ===== phase 2: bodies, out of line =====
namespace Speak_::impl_ {
    int32_t speak(tag, const int32_t& self_) { return self_; }
    void    bump(tag, int32_t& self_, int32_t d) { self_ += d; }
    int32_t consume(tag, int32_t self_) { return self_; }
    template<class T> requires has_Score<T> int32_t speak(tag, const T& self_) { return Score_::score(self_); }
    /* … */
}
// call sites (§3.2.3):  x.speak() → Speak_::speak(x);   x.bump(1) → Speak_::bump(x, 1);   x.consume() → Speak_::consume(x) (i32 is Copy)
//   Speak::twice(&x) → Speak_::twice(x);   f.speak() with f: Foo (tier 1) → f.speak();   Speak_::speak(f) is also correct (bridge)
```

**Declaration order inside the trait's module** is fixed by the dependencies in that listing: the interface's
forward declaration → the marker primary → the concept → the `Tr_` namespace (tag, CPOs, bridges, default
templates — the constrained default overload names `has_Tr`) → the interface class → the three forwarders.
(The census probe spelled `has_Shape` inside `Shape_` through the CPOs instead; equivalent for a trait with
non-generic required methods, and the marker form is the one this design mandates — rule 5.)

**Seven rules, each the fix for a measured failure (§3.2.15, 2026-10-04 probes).** They are emitter invariants. Violating 1, 2 or 4 is loud (a redefinition, an unmatched call, a hard
error in a concept); violating 3, 6 or 7, or misordering a *partial* marker under 5, is **silent** — a
frozen overload set, an inherent shadow, or the `T` impl where Rust's probe lands on `&T` — which is the
price of this carrier, and why §3.2.16 phase 2 turns the six probes into oracle-checked matrix targets
before any crate flips.

1. **`Tr_` holds only the CPOs (and ABI-pinned companions, below); impl functions, default templates and
   bridges live in `Tr_::impl_`.** With the CPO as a function *object*, a method name written unqualified
   inside `Tr_` finds the variable `Tr_::speak` by ordinary lookup, and a found variable *suppresses* ADL
   for that call (measured). **Refinement (measured 2026-10-07, `fncpo/fn_cpo.cpp`, identical to the
   oracle):** the CPO may equally be a constrained function *template* in `Tr_` — `template<class S,
   class... R> requires (!std::same_as<std::remove_cvref_t<S>, impl_::tag>) auto m(S&& s, R&&... r) ->
   decltype(m(impl_::tag{}, …)) { return m(impl_::tag{}, …); }` — whose unqualified inner call does the
   same tag-ADL at instantiation; the constraint keeps it out of its own overload set. Ordinary lookup
   finding a *function* does not suppress ADL, so this form has no rule-1 hazard, and a non-template
   overload may sit beside it in `Tr_` — which is what the ABI-pinned companions of §3.2.12 need
   (`Shape_::area(const Sq&)` beside the dispatcher: exact non-template match wins, everything else
   resolves as before). The function-template form is therefore the one phase 2 emits; the two
   namespaces are still not a style choice (an impl function named like the dispatcher *in `Tr_`* would
   join its overload set and be found by qualified lookup where only the dispatcher should be). Two
   facts from landing it (2026-10-07): the dispatcher's unqualified inner call is hijacked by **any
   same-named non-function visible from `Tr_`** — tap's method `tap` against the crate namespace `tap`
   (`unexpected namespace name`) — so `Tr_` also declares a never-viable **ADL enabler** `void
   m(impl_::adl_enabler_);` per method ahead of the dispatchers (ordinary lookup then finds a function and
   ADL applies); and the tag is spelled **relative** (`Tr_::impl_::tag`), because the UFCS passes emit
   inside the crate's namespace wrap, where an absolute `::Tr_` does not exist.
2. **Every impl function takes `impl_::tag` as parameter 0.** The tag gives the call an associated
   namespace even when the receiver has none — *a fundamental type has no associated namespace*, so
   without it `hello(self_)` on `i32` is `call to function 'hello' that is neither visible in the
   template definition nor found by argument-dependent lookup` — it keeps two traits' same-named methods
   on one type apart (`redefinition` without it, H3), and it keeps a downstream trait's `m` on the same
   type from making the call ambiguous (H13: `ambiguous` where rustc prints `3005`; fixed by the tag).
   It is the trait's identity in the overload set.
3. **Every call site spells the CPO, qualified: `Tr_::m(x, …)`.** The qualified name is a *variable*,
   immune to a local named `m` (a bare call with `int32_t m` in scope is `called object type 'int32_t'
   is not a function`, H2) and to inner-scope hiding; the CPO's *body* makes the unqualified call, from
   inside `Tr_`, where ADL runs at the point of instantiation. This is why a generic caller or a default
   body compiled in module A reaches an impl added in module B: a *qualified* call to a function
   template in a template sees only the overloads declared before it (`qlookup` prints `1`; a
   supertrait-only default gives `sonly=11000` where rustc gives `5550`, silently), the CPO's
   unqualified call does not.
4. **The CPO is SFINAE-friendly** — trailing `decltype` on the call, no body-only failure — so a ladder's
   `requires` and a concept conjunct can test it without a hard error.
5. **The predicate is the defined-false marker `impls_Tr`, and the concept is `has_Tr = marker ∨ base`**
   (§3.2.3), as on 09-29. A concept spelled only through the CPOs (`requires { Tr_::speak(c); }`) is not
   a substitute: an all-default trait has no required method to test, a tier-2 trait with a *generic*
   required method has no testable call, and a completeness predicate memoizes silently where the
   defined marker is loud on misordering (§3.2.3). The marker is Fix A's successor (§3.2.13).
6. **Default bodies live in the namespace, once, constrained to tier-2 receivers, and call the CPO.**
   `self.m()` inside a default lowers to `Tr_::m(self_)` — never to a member call and never to the
   member-first shim. The shipped lane's `self_.hello()` resolves to an *inherent* `hello` where Rust's
   default body sees only the trait's (`2002` vs `2`); the CPO sees only trait impls. A concrete impl
   that overrides the default is a **non-template** overload (or a more-constrained template), which
   overload resolution prefers over the default template — the shipped tiebreak, un-retired (§3.2.4,
   §3.2.13).
7. **Generic traits: a trailing, defaulted `rusty::tag<A…>`; the `&T`/`T` collapse: a trailing, defaulted
   `rusty::self_tag<Self>`** (both new one-line `include/rusty` helpers). For `impl Tr<i32> for T` +
   `impl Tr<u8> for T`, every impl function gets `rusty::tag<int32_t> = {}` / `rusty::tag<uint8_t> = {}`
   after its Rust parameters. The emitter passes the tag when `A` is lexically recoverable — a bound, a
   path (`<T as Tr<u8>>::m`), a `let` annotation, the parameter type of the callee the result flows
   into — and **omits it otherwise**, letting overload resolution on the Rust arguments decide:
   `t.m(av)` with `av: u8` from a function return is rustc `304`, and the free-function overload set
   gives `304` with no emitter knowledge. (The 09-29 text "method syntax on a generic trait is emitted
   only when the trait arguments are lexically recoverable; otherwise Rust requires path syntax" was
   wrong — Rust infers `A` from the argument there, and the adapter key as written rejected a program
   rustc accepts.) When neither the lexical context nor an argument determines `A`, Rust itself demands
   an annotation (E0283), so no third case exists. `impl Tr for T` and `impl Tr for &T` are `m(tag,
   const T& self_, rusty::self_tag<T> = {})` and `m(tag, const T& self_, rusty::self_tag<const T&>)`:
   the plain impl is the default, and the emitter passes `self_tag<const T&>` exactly where Rust's
   probe lands on the `&T` impl — a receiver of Rust reference depth ≥ 2 (`(&r).m()`; `rr.m()` with `rr:
   &&T`), `<&T as Tr>::m`, `Box::new(r)` → `Box<&T>`, and a type-parameter receiver bound by `X = &T`
   (`via_bound<const T&>(r)`; deduction would collapse the reference). Measured identical to rustc on
   9/9 and 7/7 lines (§3.2.15). Both tags compose, one trailing parameter each; the dyn forwarders
   carry them as template parameters (`TrAdapterRef<A…, U>`, `TrAdapterRef<const T&>`) and pass them in
   every slot.

**Where an impl's functions are declared** (§3.2.5 has the module rules): in the **self type's
declaring namespace** when the self type is declared by the emitting crate — ADL on the receiver finds
them there, and the impl body's relative names (`Helper::new_()`, `private_::unit_only`) resolve *in
place* with zero relocation, which retires Fix B for these impls (132 of 249 serde_core impls, 69 of 72
serde; measured, §3.2.15); in **`Tr_::impl_`** — the tag's namespace, which ADL always searches — when
the self type is a primitive, a reference, a slice, a tuple, an array, a closure, a std or dependency
type, or a blanket. A crate must **never** add to a namespace another crate owns: two crates each
declaring `a::ext(const a::Bar&)` is `declaration 'ext' attached to named module 't' cannot be attached
to other modules` in any importer of both (H1). The shipped `rusty_ext` lane (local trait × foreign self
type) is this second rule under another name and folds into it (§3.2.14).

**Emission order.** Within a TU the shipped two-pass shape stays: **(1)** every impl's function
*declarations* — including blanket templates — before any default template and before any call;
**(2)** all bodies after. The tag makes textual order irrelevant *across* module boundaries (ADL at the
point of instantiation sees every exported overload the importing TU can reach), but inside one TU a
default template instantiated before a later-declared non-template override would bind the default —
the same point-of-instantiation subtlety as the 09-29 marker rule, resolved the same way: by order,
not by hope. The forward-declaration pass the emitter already performs is that order.

**Non-vtable members** — generic required methods (`fn serialize<S>(&self, s: S)`), `-> Self` methods,
no-receiver associated functions (`fn new() -> Self`), associated consts — are free functions,
function templates and `inline constexpr` variables in `Tr_::impl_` like any other method, reached by
their CPOs (`Serialize_::serialize(x, s)`) or, for a no-receiver item on a type parameter, through
`TrTraits<T>` (§3.2.8: `TrTraits<T>::new_()`, `TrTraits<T>::K`). A generic required method is a function
template in the overload set — the one shape no vtable can carry — and needs no interface slot. This is
the home the 09-23 design gave them as adapter member templates, and it is a phase-2 gate: the shipped
emitter skips all three shapes today, which is why serde's `Serialize` and `Deserialize` get no
interface at all (§3.2.15).

> **Retired (2026-10-04):** the per-impl body-carrying adapter specialization `template<> class
> TrAdapterRef<U> final : Tr { const U& value_; … }` with three flavours, the delegation table between
> flavours, the two-phase adapter rule, and exact receiver keying through `remove_cvref_t` — the 09-23 /
> 09-29 tier-2 mechanism. §3.2.15 records why: it matched rustc on every measured cell, as the namespace
> carrier does, at 28–34 emitted lines per impl against 6–8, five call-site shapes against two, and it
> rejected the arg-inferred generic-trait call. What it had that the namespace carrier must re-earn —
> native cross-module defaults through the vtable, and a loud marker — is kept as the CPO protocol
> (rule 3) and the marker (rule 5). **Retired (2026-09-29, still):** the injected `using namespace
> Tr_;` (rule 3 replaces it), the second emission of each impl body as a struct member, and `rusty_ext`
> as a separate lane.

#### 3.2.3 Self-implemented UFCS (member-first dispatch) — tier 2, and the tier boundary at a call site

**Tier 1 has no shim** when the receiver's *declared* type is lexically known and `(Tr, T)` is tier 1:
`x.m()` is the member call, `Tr::m(&x)` and `<T as Tr>::m(&x)` are the same member call, and a call
through `&dyn Tr` is the member call on the reference. The declared type is recovered from the
parameter list, a `let` annotation, the declared type of the initializer chain, or `Self` in impl
bodies (§3.2.6). The one exception to "path syntax is `x.m()`": an explicit trait argument on a generic
trait, `<T as Tr<u8>>::m(&x, a)`, lowers to `static_cast<const Tr<uint8_t>&>(x).m(a)` — a base-reference
cast with virtual dispatch intact, needed only for a type that inherits more than one instantiation,
which axis 2 already excludes; it is kept for the tier-2 route.

**Tier 2 is a qualified call to the trait's CPO** when the receiver's declared type is lexically known and
`(Tr, DeclaredType)` is tier 2, or when the receiver is a type parameter whose bound (elaborated through
supertraits) names exactly one owner of `m` — a type parameter has no inherent candidates:

```cpp
Tr_::m(x, args)                 // fn m(&self, …)       — x an lvalue; the impl function takes const U&
Tr_::n(x, args)                 // fn n(&mut self, …)   — x a mutable lvalue; the impl function takes U&
Tr_::k(std::move(x), args)      // fn k(self, …)        — a move, or a copy `Tr_::k(U(x), …)` for a Copy self type (decision (s))
Tr_::m(x, args, rusty::tag<A>{})          // generic trait, A lexically recoverable (§3.2.2 rule 7); omitted otherwise
Tr_::m(x, rusty::self_tag<const T&>{})    // the &T impl, where Rust's probe lands on it (§3.2.2 rule 7)
```

Which of the three receiver spellings applies is a property of the method *name within the trait*, read
from the trait declaration or the manifest — no type inference. Path syntax `Tr::m(&x)` and `<U as
Tr>::m(&x)` on a tier-2 pair are the same CPO call. There is no flavour decision, no adapter, no
specialization to name: the CPO's overload set contains every impl's functions and the bridge, and
clang's overload resolution on the receiver's type picks one — or reports `no matching function for call
to object of type 'Tr_::m_fn'`, which is Rust's "the trait bound is not satisfied".

**The shim is emitted otherwise** — the receiver's declared type is not recoverable, or it is recoverable
and has a same-named inherent member that Rust would probe first. It is a generic lambda applied to the
receiver, and its arms are ordered as Rust's probe is — own type before deref, inherent before trait at
each step:

```cpp
([&](auto&& r) -> decltype(auto) {
    using S = std::remove_cvref_t<decltype(r)>;
    // 1 inherent, own type — a member that is NOT a tier-1 slot of ANY known owner of `m`, in scope or not
    if constexpr (requires { r.m(args); } && !(std::is_base_of_v<Tr1, S> || std::is_base_of_v<Tr2, S> /* … */))
        return r.m(args);
    else if constexpr (has_Tr<S>)                       return Tr_::m(r, args);   // 2 the trait: tier 1 via the bridge, tier 2 via the impl function
    /* 3–4: the same two tests on deref(r); continue down the deref chain (rusty::deref_call); final arm static_assert(false) */
})(x)
```

Two shapes, then: the bare CPO call, and this ladder. (The 09-29 ladder had five arms — inherent,
tier-1 base, tier-2 adapter, and their deref repeats — because tier-1 and tier-2 receivers were reached
by different spellings; the bridge in `Tr_::impl_` makes `Tr_::m(r)` correct for both.) **Caveat for
review:** in every 2026-10-04 probe the inherent-vs-trait decision was *hand-resolved* — `w.m()` was
written as the member call where the inherent `m` exists and as `Tr_::m(w)` where it does not; no probe
emitted this ladder. Its arms are the 09-29 arms with the adapter arm replaced, and that replacement is
measured (the bridge: `Shape_::describe(sq)` ≡ `sq.describe()`, census probe), but the ladder as a whole
is unmeasured in the namespace form (§3.2.15).

**Under plain names, `requires { r.m(); }` no longer means "inherent."** It is satisfied equally by an
inherent `m`, a tier-1 override of `Tr::m`, and a tier-1 override of some *other* trait's `m`. Three
rules follow, and every ladder in §3.2.4–§3.2.6 is read with them. *Arm 1 tests inherent, not any
member:* it carries the negation over every tier-1 interface **known to the crate** — its own
declarations and every dependency's manifest — that declares `m`, whether or not that trait is in scope
at this call site. Without it, a cross-crate `impl B::Tr2 for A::T` with only `Tr2` in scope runs
`Tr1`'s body (measured: C++ `1`, rustc `2`), and with both in scope the guard never fires (C++ `1`,
rustc E0034). The negation never conflicts with inherent-shadows-trait, because axis 2 excludes a type
that has both an inherent `m` and a tier-1 base declaring `m`. For a lexically known concrete receiver
the arm is emitted only when its impl blocks declare an inherent `m`; for a type-parameter receiver the
arm does not exist at all (Rust's probe on a type parameter has no inherent candidates). *A tier-1
candidate is a base test,* `std::is_base_of_v<Tr, S>` — true for an implementor and for a `dyn`
receiver, false for an axis-2-excluded type — and under the CPO it is folded into `has_Tr` (base ∨
marker): the bridge overload does the member call. *Counts and predicates over impls use the concept
`has_Tr`, never the marker alone:* a tier-1 impl, and a hand-written implementor under §3.2.17, never
specializes `impls_Tr`.

**Body kind decides the receiver spelling inside trait code.** Inside a **default** body, `self.m()` is
the CPO call `Tr_::m(self_)` — the default sees only trait impls, as Rust's does (`describe = 2` with an
inherent `hello = 2002` present; the shipped lane gives `2002`). Inside an **impl** body, `self.m()` is
the ordinary lowering above — member-first where an inherent `m` exists, because Rust's probe in an impl
body is the same as anywhere else (`W<i32>`'s override of `twice` calling `self.m()` reaches the inherent
`m`: `6006`, while the default `ssum` sees the trait's `m`: `1111`; both = rustc, §3.2.15). The emitter
knows which kind of body it is emitting; this is a new emitter fact.

**The trait solver.** Each trait — tier 1 or 2 — gets, in its module:

```cpp
template <class U> struct impls_Tr : std::false_type {};                            // DEFINED primary (marker)
template <class U> concept has_Tr = impls_Tr<U>::value || std::derived_from<U, Tr>;   // NAMED concept
```

- The *predicate* is the defined-false marker (for tier-2 impls) or the base test (for tier-1 impls),
  because a completeness test written any other way **memoizes silently**: evaluated before the
  specialization it yields false and every later evaluation in the TU stays false, with no diagnostic
  (measured, §3.2.15). A defined primary instantiated before its **explicit** specialization is instead a
  loud `explicit specialization after instantiation` — for a concrete impl the same slip is a compile
  error. A misordered *partial* specialization (a blanket impl) is *not* diagnosed even with a defined
  primary; the emission-order rule of §3.2.2 is the correctness guarantee for both.
- `has_Tr` is a **named concept**: an inline `requires { … }` outside a templated entity is a hard error,
  not a soft false; blanket subsumption (§3.2.4) exists only between concept-ids; and it is what every
  bound emits (§3.2.17). Two blanket partials that neither subsumes make it read false rather than error
  — valid Rust never produces that (E0119).
- A concept through the CPOs (`requires(const U& c) { Tr_::m(c); }`) is admissible *as a conjunct* for
  required methods with a non-generic signature and is what the census probe used; it is not the
  predicate (§3.2.2 rule 5; decision (y)).
- Two facts from landing the marker (2026-10-07): an **unbounded blanket** `impl<T> Tr for T` makes the
  primary itself `std::true_type` — a "partial specialization" on a bare parameter with nothing to
  constrain on is a redefinition of the primary (tap's `TapOps`); and a **view self type** (`impl Tr for
  [T]`) is witnessed under both C++ receiver spellings, `std::span<const T>` and `std::span<T>`, because
  `&[T]` and `&mut [T]` arrive as different types. Only a `pub` trait's marker and concept are `export`ed —
  class templates do not merge across modules the way `namespace Tr_` does, so two dependencies' private
  `Sealed` markers must stay module-linkage (contract 10 / C21c). `impl Tr for &T` is keyed
  `impls_Tr<const T&>` and invisible to a lookup that strips cvref until `self_tag` lands (rule 7).

**Cost.** Byte-identical assembly to the 09-29 adapter route and to a direct free-function call at `-O1`
and above on clang 22 (`cvtsi2sdl (%rdi),%xmm0; mulsd %xmm0,%xmm0; retq` for `Shape_::area(i32)` under
all three; §3.2.15); the CPO object is an empty `constexpr` struct and the tag an empty class, neither
odr-used with linkage at `-O1+`; a dyn forwarder carries a vtable only when bound through the interface.

**Emission order is load-bearing — diagnosed for concrete impls, silent for blanket impls.** Every
`impls_Tr<U>` specialization must precede every evaluation of `impls_Tr<U>::value` in its TU, and every
impl function's *declaration* must precede the first default template (§3.2.2); the forward-declaration
pass guarantees both. There is no "instantiate later" escape: a generic lambda's `operator()` is
instantiated by the end of the *enclosing function*, not the TU (measured, §3.2.15), so order must be
textual. Tier 1 has no specialization to order — a base class is complete when the derived class is
declared, and the interface must simply precede its implementors, which the shipped emitter does not
guarantee for a type declared before its trait (§3.2.16 phase 0).

#### 3.2.4 Method-resolution priority (inherent ▷ trait; concrete ▷ blanket)

*Tier 1:* inherent ▷ trait is C++'s own rule — a member declared on the derived type hides the base's
— and the §3.2.1 exclusion of same-*name* inherent/trait overlap (name, not signature: a different
signature hides the whole base overload set and turns a valid Rust call into "too few arguments" or an
abstract implementor) is what keeps hiding from becoming silent overriding. *Tier 2:* overload
resolution's own tiebreaks, below, read with §3.2.3's rule that predicates use `has_Tr`.


**Tier 2 — priority is overload resolution's own, plus one guard.**

- **inherent ▷ trait** — arm 1 of the shim, lexical, member-first (§3.2.3); never visible to the CPO,
  whose overload set contains trait impls only. (This is exactly why default bodies call the CPO:
  §3.2.2 rule 6.)
- **concrete ▷ blanket**, and **concrete override ▷ default** — the two C++ rules the shipped lane already
  relies on and this revision *un-retires*: a **non-template** function beats a function template on an
  otherwise equal match ([over.match.best]), and among templates the **more constrained** wins by
  subsumption of concept-ids. `describe(tag, const Bar&)` beats `template<class S> describe(tag, const
  S&)`; `template<class T> requires IntoF64Copy<T> each(tag, const Wrapper<T>&, F)` beats the default
  `each` template by being more specialized. Stable Rust never *needs* the second rule for impl
  selection: a concrete impl overlapping a blanket, or two nesting blankets, is E0119, so for valid input
  at most one impl function is viable — the rule's real job is **override ▷ default** (§3.2.13).
- **The marker mirrors it:** a concrete impl is `template<> struct impls_Tr<Foo> : true_type`, a
  blanket a constrained partial specialization; `has_Tr` is one concept either way.

**Receiver keying is by overload resolution, with the tags where C++ would collapse.** The receiver
arrives as `const U&`, `U&` or `U` and the impl functions are overloaded on it. Two places where a C++
parameter type cannot carry a Rust distinction get an explicit key: `impl Tr<i32> for T` beside
`impl Tr<u8> for T` → trailing `rusty::tag<A>`; `impl Tr for T` beside `impl Tr for &T` → trailing
`rusty::self_tag<Self>` (§3.2.2 rule 7). Both are defaulted on the common case, so the plain call stays
`Tr_::m(x)`. This replaces the 09-29 exact keying (`remove_cvref_t<decltype(x)>` selecting a
specialization), and it re-admits what exact keying excluded: **implicit conversions on the receiver**.
Derived-to-base and user-defined conversions have no Rust counterpart — a C++ caller can reach
`m(tag, const Base&)` with a `Derived`; valid Rust never emits that call, so it is a forward-only
acceptance, not a silent-wrong on valid input. One conversion is wanted: it is the free-function lane's
accidental implementation of Rust's **autoderef / unsize** probe step. For an impl whose self type is a
view (`impl Tr for [T]`, `str`, `&[T]`, `&str`), a call on a `Vec<T>`, `String`, `[T; N]` or
string-literal receiver reaches `m(tag, std::span<const T>)` / `m(tag, std::string_view)` through span's
range constructor or `String`'s `string_view` conversion — valid Rust (`Vec<T>: Deref<Target=[T]>`,
`String: Deref<Target=str>`). A concrete `impl Tr for Vec<u8>` beside `impl Tr for [u8]` still wins: an
exact match beats a conversion, which is rustc's probe order. View-typed impl functions take the view by
value. §3.2.16 (g).

> **Retired (2026-10-04):** explicit-specialization ordering as the priority mechanism and the
> `TrAdapterRef<U>` exact-keying rule, with the 09-23 view-adapter constrained partial specialization.
> **Un-retired:** the non-template-▷-template tiebreak on `Tr_::impl_::m` overloads, and the rule that
> trait receivers are emitted **matching the Rust `self` kind exactly, never as forwarding references**
> — a forwarding-reference impl function would beat a `const Foo&` non-template on a "better" conversion
> and break the override tiebreak.

#### 3.2.5 Where the namespace, the impl functions and the forwarders live, and `use` → `import`

*Tier 1:* the interface, the marker primary, the concept, the `Tr_` namespace and the generic forwarders live
in the trait's module, exported (for every trait, since a downstream crate may bound on it or implement
it at tier 2); a directly-inheriting struct lives where it is declared and imports the trait's module;
`use Tr;` is that import and nothing else. A tier-1 impl is a class definition, not a specialization,
and has no reachability obligation beyond an ordinary base class — but a non-`pub` trait's interface
is today wrapped in an anonymous namespace (C21), and a `pub` implementor would then expose a TU-local
base; the wrap must apply only when the trait *and every implementor* are non-`pub` (§3.2.16 phase 0).
The cross-module discussion below concerns tier-2 impl functions and marker specializations only.


**Tier 2.**

- The **`Tr_` namespace** — `impl_::tag`, the CPOs, the default templates, the tier-1 bridges — the
  **marker primary**, the **concept**, the **thin interface** and its **three generic forwarders** live in
  the **trait's module**, exported. Nothing in it is per impl.
- Each impl's **functions** live where §3.2.2 puts them — the self type's declaring namespace for a type
  this crate declares, `Tr_::impl_` otherwise — in the **impl-emitting crate's module**, exported; by the
  orphan rule that is the trait's crate, the self type's crate, or (for a generic trait) the crate owning
  a *trait argument*: `impl From<Local> for Vec<u8>` is legal and lives with `Local`, in `From_::impl_`
  (a foreign self type), reopened from `Local`'s module. Reopening a namespace from another module is
  ordinary C++; what H1 forbids is two modules declaring the *same* function.
- Each impl's **marker specialization** lives in the same module, at **global scope** — an explicit
  specialization must be declared in the primary's namespace ([temp.expl.spec]/3); under §2.5's per-crate
  namespace wrap it is emitted outside the wrap.
- A **nested-module impl on a foreign self type** keeps the shipped Fix B shape (§3.2.14): its functions
  are emitted in a per-module helper namespace where the body's relative names resolve, and bridged into
  `Tr_::impl_` with a using-declaration re-emitted after each impl block — 4 lines per impl, against the
  09-29 adapter bridge's 8 per impl plus 3 per trait. A nested-module impl on a *local* self type needs
  no bridge at all.
- `use Tr;` translates to the module `import`. **There is no `using namespace`.** Every call spells
  `Tr_::m` (§3.2.3).

**Cross-module explicit specialization is conforming.** `impl serde::Serialize for MyType` puts
`template<> struct impls_Serialize<MyType>` in *my* module while the primary is in serde's, and its impl
functions in `MyType`'s namespace.
[temp.expl.spec]/3 restricts only the scope; /4 requires the primary's *declaration* to be
reachable, which `import` provides; /7 requires the specialization before its first use in each
TU; [module.reach]/3 governs reachability. Verified on clang 22.1.8 named modules (§3.2.15). The
one obligation the emitter carries: **the specialization must be reachable from every TU that
names it or tests its marker** — a TU that imports the trait's module but not the impl's reads
`impls_Tr<U>` false. That divergence is ill-formed NDR and *not* diagnosed by modules (what they
diagnose is one class defined with different bases in two units — a different hazard). It is
inert here for two reasons, and the load-bearing one is Rust's own coherence rule: a TU that cannot
reach the impl cannot compile a call on that receiver — it hits §3.2.6's `static_assert`, Rust's
"impl not visible without its crate" error — and coherence guarantees at most one impl of `Tr` for
`U` in a program, so every TU that *does* compile the ladder took the same arm. Note what does
**not** guard it: a ladder inside an inline or template function *is* COMDAT-merged across TUs — a
closure type per call site does not prevent that (reviewer probe) — so the argument must rest on
coherence, never on closure uniqueness. `false_type::value` is never ODR-used with linkage. A
direct CPO call has no such softness: a receiver with no reachable impl function is a loud `no matching
function` in every TU. **Two
invariants follow and must hold in any implementation:** the marker is never ODR-used, and no
emitter path may bypass the `static_assert` on a receiver whose impl is unreachable.

**Two emission rules follow from [temp.expl.spec]/7.** *(1) Interface units only.* A specialization
in a module *implementation* unit is unreachable from importers: a direct use fails loudly, but the
marker reads **silently false** for every importer while the implementation unit itself sees the impl
(reviewer probe D2: `via_iu=7000 has=0`). Impl functions, marker and `TrTraits` specializations are emitted
only into module interface units. *(2) Reachability follows the import graph, not `export`.* An
interface unit is reachable even when only transitively imported through a non-exported `import`
([module.reach]/1); `export` on the specialization is irrelevant ([module.reach]/3), and `pub use` →
`export import` is a *visibility* rule for the type's name, not a reachability rule. The obligation is
met whenever the consumer's module graph includes the impl-emitting crate — which Rust's dependency
graph guarantees for any crate that uses the impl. Both compilers agree on the shape (clang 22.1.8,
gcc 14.2) and mangle the specialization as owned by the primary's module. One caveat for phase 2: the
*shipped* `<Tr>Traits` primary is currently **defined** (`emit_items.rs`, a nested-typedef fallback
for foreign traits), where §3.2.3/§3.2.8 assume declared-never-defined — decide which.

**Scope-sensitivity moves carrier — to the emitter's spelling.** The shipped lane models "the trait is in
scope" as namespace visibility: `using namespace Tr_;` ⟺ `use Tr;` — and in practice injects the
directive for every trait, at global scope, regardless of `use` (§3.2.15). Measured against Rust's rule
that is wrong in three independent ways: a namespace-scope directive flows into child namespaces and
reopened definitions where Rust's `use` is not inherited by child modules (parent `use A` + child `use B`:
`call to 'foo' is ambiguous` at namespace scope, correct `child=2 parent=1` only with the directive at
function-body scope); a directive is defeated by inner-scope hiding where a using-*declaration* is not;
and a local variable named like the method breaks the bare call (H2). Under this design the trait's
identity is **spelled at every call** — `Tr_::m(x)` — from the same `use` / prelude / bound information,
and a multi-owner name becomes §3.2.6's enumerated, guarded ladder. There is nothing to inject and no
scope for it to leak from. Identical information, a carrier exactly as precise as Rust's resolution, and
a requirement rather than a choice (§3.2.16 (i)).

**Std and prelude traits are outside this section.** `Display`, `Debug`, `Clone`, `Default`,
`Hash`, `Iterator`, `Deref`, … keep their dedicated member / operator / range lowering as today;
no `Display_` namespace exists. The namespace lane covers traits this crate declares
and dependency-crate traits reached through the manifest (§3.2.7).

#### 3.2.6 Two traits with the same method name

Traits `A` and `B` both declare `foo`; type `T` implements both. Resolution mirrors Rust:

- **Path syntax** `A::foo(t)` → the member call when `(A, T)` is tier 1 and `T`'s declared type is
  known; otherwise `A_::foo(t)` — the CPO names the trait, so there is nothing to disambiguate. The trait
  is copied from the source path.
- **Method syntax** `t.foo()` when exactly one trait in the candidate set owns `foo` → `A_::foo(t)`
  likewise (or the §3.2.3 shim when an inherent `foo` may exist).
- **Method syntax** `t.foo()` when two or more candidates own `foo` → the transpiler emits a **candidate
  ladder** over them, guarded:

```cpp
([&](auto&& r) -> decltype(auto) {                      // generic lambda — required (P2593)
    using S = std::remove_cvref_t<decltype(r)>;
    // inherent first — excluding EVERY tier-1 owner of `foo` known to the crate, not just the candidates:
    // a call with only B in scope must not take A's override.
    if constexpr (requires { r.foo(); } && !(std::is_base_of_v<A, S> || std::is_base_of_v<B, S>)) return r.foo();
    else {
        constexpr int n = has_A<S> + has_B<S>;           // the CONCEPTS (base ∨ marker), never the markers alone
        static_assert(n <= 1, "ambiguous trait method `foo` for this receiver — use path syntax A::foo(x)");
        if constexpr (has_A<S>)      return A_::foo(r);  // the CPO: tier 1 through the bridge, tier 2 through the impl function
        else if constexpr (has_B<S>) return B_::foo(r);
        else static_assert(false, "no trait in scope provides `foo` for this receiver");
    }
})(t)
```

**What the free functions give for free, and what they do not.** With distinct namespaces there is no
shared overload set to be ambiguous: `A_::foo(t)` and `B_::foo(t)` are different functions, so the
*concrete-vs-concrete* E0034 case cannot even be mis-emitted — the emitter must choose, and the guard is
how it refuses to. But the guard is not redundant. Two E0034 cases are **silent** in any free-function
form that lets C++ pick: a concrete `impl A for u16` beside `impl B for u16 {}` where `B::foo` is a
*default* (rustc E0034; a shared overload set prints `16`), and a concrete `impl A for u16` beside a
blanket `impl<T: A> B for T` (rustc E0034; prints `32`) — C++'s non-template-▷-template and
more-constrained rules make those *unambiguous* where Rust's are *ambiguous*, because Rust's rule is
"two applicable items" and C++'s is "one best". The `has_A + has_B` count asks Rust's question. Measured
(§3.2.15): the guarded ladder gives the assertion on both; the shipped guard-less ladder gives `16` /
`32`.

- only `A` in the candidate set → resolves to `A` (Rust: same);
- both candidates, both implemented for `T` — by impl functions, by direct inheritance, or one of each (a
  concrete `impl A for T` here plus a blanket or foreign-crate `impl B for T` the classifier cannot
  see) → `n == 2` → the assertion fires ≈ Rust's `E0034` (measured: `d.m()` prints `1` with the
  09-23 ladder, which counted markers only and whose first arm had no base exclusion);
- both candidates, one implemented → resolves (Rust: same);
- neither → the final assertion ≈ "no method named `foo`".

**Arm 1 is not only about ambiguity.** With `A` out of scope and `B` in scope, Rust runs `B::foo`
(rustc `1000`); an arm 1 without the base exclusion returns `A`'s override (`1`) because the member
exists regardless of `use`. The exclusion list is therefore scope-*independent* — every tier-1
interface declaring `foo` that the crate or its manifests know — while the candidate set (which arms
follow, and what `n` sums) stays scope-precise. No §3.2.1 rule can substitute for this: the second
impl may live in a crate the self type's crate never sees, and `struct T : A` is already baked into
`T`'s module.

**The candidate set is fixed by the receiver's declared type, in two regimes.** For a receiver whose
declared type is **concrete**, the candidates are the traits in lexical scope — `use`, prelude, glob
imports, traits declared in the enclosing module. For a receiver whose declared type is a **type
parameter** (or a projection of one), the candidates are the parameter's bounds **elaborated through
their supertraits, transitively**, and `use`-scope traits are consulted **only if no elaborated bound
owns the name** — the Rust Reference's order. That second clause is what admits the `use
itertools::Itertools` keystone of §3.2.7; "bounds only" would reject it, and "`use`-scope only" would
silently run the wrong body. Inside a trait's default body, `self` is in the type-parameter regime
with the bound `Self: ThisTrait` plus its supertraits. Both regimes are lexical: the declared type is
recovered from the parameter list, a `let` annotation or the declared type of the initializer chain,
or `Self` in trait and impl bodies; the bound set from the function's generics and where-clauses; the
supertrait closure from the trait declarations and, cross-crate, the manifest. **When the declared type
cannot be recovered lexically**, the emission is the **union** of both regimes under the guard — loud
in the overlap case, a false positive on valid Rust that widening declared-type recovery closes —
never a single regime.

**Two invariants, not options.** The ladder is a **generic lambda**: C++23 (P2593) makes
`static_assert(false)` in a discarded `if constexpr` branch inert only inside a templated entity
(measured, §3.2.15). And the `static_assert(n <= 1)` is a **new guarantee this design adds**: the
shipped emitter's multi-owner form is *already* a guard-less first-wins `if constexpr` ladder over
qualified `A_::foo` / `B_::foo` arms, including traits not in scope (§3.2.15). The namespace form adds a third: a **type-parameter receiver is
spelled through its bound** (`A_::foo(x)` for `X: A`), never through an unqualified name — at `g<int>` the
unqualified form is `ambiguous` once a second owner exists (measured, §3.2.15).

#### 3.2.7 Non-intrusive impls — cross-file, cross-crate, and why not CRTP


> Under the two-tier model this section is the argument for why tier 2 exists and why its carrier is a
> free function. Tier 1 is intrusive by design — the type inherits — and that is fine precisely because
> axis 2 restricts it to types this crate declares; the non-intrusive carrier is needed where the type is
> not ours, which is tier 2's definition. The 2026-09-23/29 revisions re-read the paragraphs below with
> "adapter" substituted for "free function"; the 2026-10-04 revision restores them as written, with three
> updates: the call is the CPO `Tr_::m(x)` and `use Tr;` is the module `import` alone — `using namespace
> itertools::Itertools_;` no longer exists (§3.2.5); every impl function carries the tag (§3.2.2); and the
> manifest additionally carries each trait's tier, supertraits, non-vtable defaults, receiver kinds and
> tier-1 ownership (§3.2.14). The CRTP rejection is *strengthened* by §3.2.15: a **closure's** C++ type is
> synthesized by clang and has no definition anywhere to add a base to, and clang 22 rejects re-emitting
> an upstream module's type with a new base outright (`declaration 'Widget' attached to named module
> 'up' cannot be attached to other modules`). Both are answered by the free function, as the foreign-type
> case always was.

Free functions are the only non-intrusive, cross-boundary mechanism, so this design handles
the cases inheritance cannot. `impl LocalTrait for ForeignType` (orphan-legal because the
trait is local) is just another overload in a namespace; the foreign type's definition is
**untouched**.

**Why not CRTP.** The obvious C++ alternative for static trait dispatch is CRTP —
`struct Person : Greet_<Person>`, with the base template supplying methods. It is rejected
for two structural reasons, not taste: (1) it is **intrusive** — the base must be declared
*at the type's definition*, but Rust `impl`s are routinely written in a different module or
crate than the type, and C++ cannot retroactively add a base to an already-defined (or
foreign) type; (2) it is **static-only** — `Greet_<Dog>` and `Greet_<Cat>` share no base, so
it cannot back `Box<dyn Greet>`; you would *still* need the abstract interface of §3.2.10.
CRTP would be a third mechanism bolted on, not a replacement. Free functions take the
receiver by parameter, so they are non-intrusive by construction and compose with the
interface for `dyn`. (CRTP's genuine wins — natural `p.name()` syntax, default methods, `Self`
returns — are recovered other ways: members for local types, and §3.2.13 for defaults.)

**Within a crate** the cargo-expanded source is one unit (all modules inlined as nested
`mod`s), so a multi-pass scan sees every trait, type, and impl before emitting anything —
cross-*file* is a non-issue.

**Across crates** the boundary is the crate, and the problem is **name resolution, not type
inference** — so it needs *metadata*, not a mini-rustc. Each crate's per-crate analysis
(declared traits, method-name classes, the concrete-impl owner map, default methods,
assoc-const-ness, and the crate's module name) is persisted as a **sidecar manifest** next to
its `.cppm`. Because dependencies are transpiled before their dependents, a dependent loads
its dependencies' manifests (through the existing cross-crate symbol-index channel) and folds
them into its own classifier view. Then `iter.dedup()` against a foreign `itertools::Itertools`
resolves: `dedup` classifies trait-only, owner `Itertools`, namespace `Itertools_`
→ emit `Itertools_::dedup(iter)` — the CPO; `use itertools::Itertools;` is the `import` and nothing else. Manifests must propagate **transitively** in dependency order.
The free functions themselves are emitted with `export`, so they are visible cross-module via
`import`. *(This is the itertools keystone; until the manifest pipeline lands, foreign-trait
methods are simply not classified trait-only and fall through to member calls.)*

**Cross-crate *type* metadata (not just traits).** Classification gets the *call* right, but the
emitted free function's *signature and body still name types*, and a re-exported dependency type
is where this bites. serde's facade crate re-exports serde_core through a `private_::de` module
and emits a UFCS helper for it under `namespace private_::de::__ufcs_…`; the helper returns
serde_core's `de::value::BytesDeserializer<E>`. Two failures follow, both because the facade
crate's per-crate analysis **never saw that type declared** (it lives in the dependency): (1) the
reference is written *relative* — `de::value::BytesDeserializer` — and inside `private_::de` that
binds to the wrong `private_::de::value`; (2) the type is emitted *bare* — `BytesDeserializer`
without `<E>` — because the arity is unknown. The trait manifest above carries *traits*, so it
cannot fix either. The remedy is the symmetric one: the manifest also carries **`declared_types`
— `{name, module_path, arity}` for every type the crate declares** (arity = generic *type*-param
count). A dependent folds these into the *same* maps its own collect pass fills — the
type→module-path map that drives Fix B (§3.2.14) and the type→params map that drives arity
recovery — with **local declarations winning** (the collect pass runs before the merge). Now the
downstream Fix B qualifier absolutizes the reference to `::de::value::BytesDeserializer` (the
crate emits its namespaces at global scope, so `::` is the correct anchor and cannot be shadowed
by an enclosing `private_::de`), and the arity heuristic completes it to `<E>`. The principle
mirrors the trait side: cross-crate is **name resolution via metadata**, extended from "which
names are trait methods" to "where each type lives and how many type params it takes." This makes
`serde_core`, the `serde` facade, and `serde_repr` all compile flag-on. *(Known residual: the
absolute `::` anchor is correct for a crate's **own** namespaces, but a downstream crate that
`import`s a dependency whose module has the **same name** as one of its own — e.g. `serde_bytes`
both declaring and importing a `ser`/`bytes` namespace — can still bind `::ser` to the imported
one; making that fully robust needs collision-aware anchoring per emission scope, tracked
separately.)*

**A foreign self type is the easy direction.** Because a free function takes the receiver by
reference (`Tr_::foo(const S&)`), `S` being defined in another crate is irrelevant to
dispatch — `S` is just an imported type passed as an argument, exactly the thing CRTP could
not handle. The only refinement: the manifest is keyed by the **impl-emitting crate** (by the
orphan rule, the trait's crate or the type's crate), since the `foo(tag, const S&)` overload
lives in *that* crate's module, in `Tr_::impl_`, where the CPO's ADL finds it.

The only irreducible residue is **member-call syntax on a foreign type for a your-crate
impl** — `foreign.your_method()` must be emitted as `YourTrait_::your_method(foreign)`, because C++ cannot
add a member to a foreign definition and has no UFCS fallback.

#### 3.2.8 Associated types

`Self::Item` and projections like `<F as FuncLR>::T` are lowered to a **type-traits map**
specialized per impl, not a solver:

```cpp
template<class Self> struct IteratorTraits;                  // primary, undefined
template<> struct IteratorTraits<Foo> { using Item = int; }; // impl Iterator for Foo { type Item = i32 }
```

Callable/where-clause associated types (e.g. a comparator's return type from
`F: FnMut(&A, &B) -> T`) are emitted as `std::invoke_result_t<F&, …>` — mechanical, not
inferred. A generic param that appears **only** in a where-clause callable-return position
(undeducible in C++) is dropped from the template parameter list and re-introduced as a
`using` alias computed from the callable, so the call deduces normally.


The same shape — a primary template specialized once per impl — is the `impls_Tr` marker of
§3.2.3, with one deliberate difference: `TrTraits`'s primary is declared-only because it is only
ever *used* (a missing binding is a loud error), while the marker's primary is *defined* false
because it is *tested*, and a tested primary must fail loudly on misordering. `TrTraits<U>` also
mirrors an impl's associated consts and no-receiver functions (§3.2.2), so generic bodies can
reach them through the type parameter.

*Tier 1:* an associated type on a tier-1 trait is a template parameter of the interface
(`template<class Item> class Iter { virtual std::optional<Item> next() = 0; }`), bound at the impl
(`struct Counter : Iter<int32_t>`) and at every `dyn` (`dyn Iter<Item = i32>` → `Iter<int32_t>&`) — the
same constraint Rust imposes on `dyn`. `TrTraits<U>` remains the map generic code reads `T::Item`
from; a generic function bounded on such a trait carries the shim, as on any bound (§3.2.17). The
reverse-direction ambiguity this creates (a template parameter could have been a generic trait
parameter) is §3.2.17's first lossy cell.

#### 3.2.9 Operator traits

Rust's operator traits map **directly** to C++ operator overloading — no free-function or
adapter machinery:

```rust
impl Add for Point { type Output = Point; fn add(self, o: Point) -> Point { … } }
```
```cpp
Point operator+(Point lhs, const Point& rhs) { return Point{ lhs.x + rhs.x, lhs.y + rhs.y }; }
```

`Index` → `operator[]`, `Deref`/`DerefMut` → `operator*`/`operator->`, `PartialEq` →
`operator==`, `Iterator`/`IntoIterator` → `begin()`/`end()` + a range interface, `Drop` →
destructor, `Clone`/`Copy` → copy/move semantics. These are *syntax* in both languages, so
they are never routed through the UFCS path.


Two boundaries this section keeps: the *operator* is the C++ operator, but the same trait's
**named** calls — `a.add(b)`, `Add::add(a, b)`, `a.eq(&b)`, `Ord::max(a, b)`, `PartialEq::ne` —
go through the §3.2.3 shim like any trait method, and an operator trait's non-operator defaults
(`Ord::max/min/clamp`) live where §3.2.13 puts defaults. `Iterator` is a std trait
and stays on its dedicated lowering (§3.2.5): `impl Iterator for T` keeps emitting a **member**
`next()` on `T`, because the runtime's range machinery (`rusty::into_iter_range`, `for_in`,
`has_option_like_next` in `include/rusty/slice.hpp`) keys on that member — a `for` loop does not use
`begin()`/`end()` (the book's own Iterator lowering rule 4). Crate traits blanket over `Iterator`
(`Itertools`) are ordinary tier-2 impls, and their 70+ provided methods are §3.2.13's concern.

#### 3.2.10 Dynamic dispatch: `dyn Trait` in tier 1 is the interface itself; in tier 2 it is a thin forwarder

**Tier 1.** There is nothing to build. The type inherits the interface, so `&dyn Tr` is `const Tr&`
bound to the object, `&mut dyn Tr` is `Tr&`, `Box<dyn Tr>` is a `rusty::Box<Tr>` holding the object
(constructed through `Box`'s converting constructor from a `rusty::Box<Dog>`, never by passing the
abstract base by value — the shipped `Box<Animal>::new_(Dog{…})` spelling does not compile, §3.2.12),
and supertrait upcasting (`&dyn Sub` → `&dyn Super`) is a base-class conversion — implicit, and
unambiguous because supertraits are **virtual** bases (`Ord: Eq + PartialOrd`, `Eq: PartialEq`,
`PartialOrd: PartialEq` is a diamond; with non-virtual bases `derived_from<D, Super>` is false and the
upcast ambiguous — the shipped fast path emits non-virtual bases, §3.2.12). This is the one place tier
1 is strictly *better* than the adapter design: the 09-23 investigation found today's adapters abstract
for any supertrait and `dyn` upcasting impossible through them.

```cpp
class Named { public: virtual ~Named() noexcept(false) {} virtual rusty::String name() const = 0; /* C.67 members */ };
class Animal : public virtual Named {
public:
    virtual int32_t speak() const = 0;
    virtual int32_t twice() const { return speak() * 2; }       // default: calls only slots
    /* C.67: protected, defaulted special members (§3.2.2) */
};
struct Dog : public Animal {
    int32_t n;
    Dog(int32_t n) : n(n) {}
    rusty::String name() const override;                       // the supertrait's slot: Dog's OWN tier-1 impl of Named
    int32_t speak() const override;
};
// &dyn Animal:  const Animal& a = dog;   a.twice();  a.name();          — base-class references
// Box<dyn>:     rusty::Vec<rusty::Box<Animal>> zoo; zoo.push(rusty::Box<Animal>(rusty::Box<Dog>::new_(Dog{3})));
// upcast:       const Named& nm = a;                                     — implicit (virtual base)
```

A tier-1 implementor never inherits a supertrait default it did not get through a tier-1 impl:
axis 2 requires `impl Named for Dog` to be a concrete tier-1 impl before `impl Animal for Dog` can be
(§3.2.1), so `Named`'s default body is reachable on `Dog` only when Rust's `impl Named for Dog` left it
unoverridden. The borrow checker's view is the ordinary one: `const Animal& a = dog;` is a reference
borrow of `dog`, and the analyzer fires on it exactly as on any reference (measured, §3.2.15) — no
`StructBorrow`, no adapter temporary, nothing new for the analyzer to learn.

**Tier 2.** Where the object cannot inherit — a foreign or primitive self type, a blanket, a tier-2 trait,
an axis-2-excluded local type — a **forwarder** inherits the interface in the object's stead and is what
`dyn` binds to: `const Tr& r = TrAdapterRef<U>(u);`. The forwarder is **one class template per receiver
kind per trait**, written once in the trait's module and never specialized; every slot is one line into
the namespace; it carries no body of any impl:

```cpp
// === trait Animal: Named { fn speak(&self) -> i32; fn rename(&mut self, s: &str); fn eat(self) -> i32;
//                            fn twice(&self) -> i32 { self.speak() * 2 } }                     // default
class Named  { public: virtual ~Named() noexcept(false) {} virtual rusty::String name() const = 0; /* C.67 */ };
class Animal : public virtual Named {                        // supertraits are VIRTUAL bases (diamonds)
public:
    virtual int32_t speak() const = 0;
    virtual void    rename(std::string_view s) = 0;
    virtual int32_t eat() && = 0;
    virtual int32_t twice() const { return Animal_::impl_::default_twice(*this); }   // the ONE default body, §3.2.13
protected: /* C.67 protected + defaulted */
};
template <class U> class AnimalAdapterRef final : public Animal {        // &dyn Animal over a tier-2 U
    const U& value_;
public:
    explicit AnimalAdapterRef(const U& u) : value_(u) {}                 // explicit → StructBorrow fires
    rusty::String name() const override { return Named_::name(value_); } // SUPERTRAIT slot → the supertrait's CPO (override-or-default)
    int32_t speak() const override      { return Animal_::speak(value_); }
    void    rename(std::string_view) override { rusty::unreachable_via_const_dyn(); }   // STUB: const value_ cannot call rename(U&)
    int32_t eat() && override           { rusty::unreachable_via_const_dyn(); }         // STUB: cannot consume through const U&
    int32_t twice() const override      { return Animal_::twice(value_); }              // → U's override if any, else the default template
};
template <class U> class AnimalAdapterRefMut final : public Animal { U& value_; /* rename forwards; eat is the stub */ };
template <class U> class AnimalAdapter       final : public Animal { U value_;  /* every slot forwards; eat: Animal_::eat(std::move(value_)) */ };
// usage:  const Animal& a = AnimalAdapterRef<int32_t>(n);  a.twice();  a.name();
//         AnimalAdapterRefMut<Cat> __m{cat}; feed(__m);                                 // &mut dyn: a NAMED local (a prvalue cannot bind Animal&)
```

- **Every slot forwards through the CPO, never to a namespace function directly.** `Animal_::twice(value_)`
  is the CPO; `Animal_::impl_::twice(Animal_::impl_::tag{}, value_)` — a qualified call to the default
  template — freezes the overload set at the forwarder's definition and runs the default where `U` has an
  override: `dyn bar.describe=10` where rustc gives `777` (measured). A bare `twice(tag{}, value_)` at
  class-member scope is also wrong: class-member lookup finds the member `twice` and suppresses ADL
  (measured: *unqualified at member scope finds the member itself*). The qualified CPO is the only
  correct spelling from inside a class.
- **Supertrait slots forward to the supertrait's CPO; they are never inherited as bodies.**
  `Named_::name(value_)` reaches `impl Named for U`'s function or `Named`'s default template by the same
  overload resolution as a static call. Inheriting `Named`'s virtual default instead would run the
  supertrait's *default* where `impl Named for U` overrides it (`1100` where rustc gives `99900`,
  measured on the 09-23 adapter; the thin forwarder gives `99900`, §3.2.15). The forwarder enumerates the
  transitive supertrait closure — one line per supertrait slot — where the shipped adapters enumerate
  nothing and are abstract for any supertrait.
- **The three stub cells are per trait, not per impl.** `Ref` cannot call a `&mut self` or `self` slot,
  `RefMut` cannot call a `self` slot: `const U&` cannot yield `U&`, and neither reference can be consumed
  without a copy Rust forbids (E0507; a silent copy is observable through `Rc` counts and `Drop`). A
  `Ref` forwarder is only ever bound through `const Tr&`, and C++ will not call a non-const or `&&` member
  through it, so for correct emission the stubs are unreachable; an emitter path that binds a `Ref`
  forwarder through non-const `Tr&` is a bug the stub turns loud at first call
  (`rusty::unreachable_via_const_dyn`, a new `include/rusty` helper that traps). Enforcement is runtime;
  the phase-2 suite must include the negative case. Decision (d): with the stubs costing three lines per
  *trait*, the 09-23 `Tr` / `TrMut` interface split is recommended retired for tier-2 traits too — one
  interface per trait, in both tiers.
- **Reference depth and generic-trait arguments ride on the forwarder's template parameters.** `impl Tr
  for &T` → `TrAdapterRef<const T&>` holding `const T&` and passing `rusty::self_tag<const T&>` in every
  slot; `impl Tr<u8> for T` → `TrAdapterRef<uint8_t, T> : Tr<uint8_t>` passing `rusty::tag<uint8_t>`
  (§3.2.2 rule 7; measured `dyn x=7 dyn r=1007`, `dyn: name via i32 … | name via u8 …`).
- **`Box<dyn Tr>` holds `TrAdapter<U>`** (by value), constructed through `Box`'s converting constructor
  from `rusty::Box<TrAdapter<U>>`; `&mut dyn` coercion at a call site binds through a **named local**
  because a non-const reference cannot bind a prvalue; the borrow checker sees a named struct borrow.

**One impl, one body — now also for `dyn`.** In the shipped output one default method's body appears in
**nine** places (§3.2.15); the 09-29 adapter design brought it to one body per impl in the adapter class,
reached by both routes. Here it is one body per impl, *in the namespace*, and the forwarder has none:

```
                 Tr_::impl_::m(tag, const U&)   ←──── the ONE body (or the ONE default template)
                  ▲                          ▲
   static route ──┘                          └── dynamic route
   Tr_::m(u)  [CPO → ADL → the body]            const Tr& r = TrAdapterRef<U>(u);  r.m()  [vtable → slot → Tr_::m(value_)]
   (byte-identical to a direct call, -O1+)      (one runtime hop, then the same CPO)
```

**Heterogeneous collections — where the vtable earns its keep.** Unchanged in substance:

```cpp
rusty::Vec<rusty::Box<Animal>> zoo;
zoo.push(rusty::Box<Animal>(rusty::Box<AnimalAdapter<int32_t>>::new_(3)));     // tier 2: the forwarder owns the i32
zoo.push(rusty::Box<Animal>(rusty::Box<Dog>::new_(Dog{3})));                   // tier 1: the object itself
for (auto& a : zoo) a->speak();                              // dispatches per element at runtime
```

**Object safety.** Only object-safe methods get slots. Excluded from the vtable — required or default, a free
function (template) in `Tr_::impl_` like any other method (§3.2.2), with no interface slot: generic methods (including `impl Trait` parameters, which desugar to a type parameter, and `async fn`, which desugars to an `impl Future` return), methods that mention `Self` in a parameter or nested in the return type (`o: &Self`, `Option<&Self>`) — §3.8 maps it to
`rusty::Task<…>` for free functions, but the method emitter never consults `sig.asyncness`, so
inherent and trait `async fn` are emitted as plain synchronous members today, lane-independent),
`-> Self` and `-> impl Trait`
returns, `where Self: Sized` methods, no-receiver functions. The shipped interface emitter's
skip-list covers only three of these; RPITIT and APIT reach it and break the build (§3.2.15).
Receivers `self: Box<Self>` / `Rc<Self>` / `Arc<Self>` / `Pin<P>` *are* dispatchable in Rust and
have no slot shape here yet — §3.2.16 (m). A generic method called on a `dyn` receiver is a compile
error under this design — no slot, and no impl function for the interface type — which is Rust's E0038.
Associated types in `dyn` must be bound (`dyn Iterator<Item=X>` → the family is parameterized by
`X`). `impl Tr for dyn Other` works for the `Ref`/`RefMut` forwarders only — the by-value forwarder
cannot hold an abstract, non-movable interface — so "up to three" is honest.

**Borrow-checker integration.** The `dyn` path is implemented and tested: `TrAdapterRef<U>`
carries `const U& value_`, registered in the analyzer's `types_with_ref_members`; its `explicit`
constructor emits a `StructBorrow` IR node (`src/ir/mod.rs`) recording an immutable borrow
(`src/analysis/mod.rs`); move-while-borrowed, assign-to-borrowed, source-outlived-by-view and
scope release all fire — pinned by `tests/test_struct_ref_member_borrows.rs`. Two things are
*not* implemented: `TrAdapterRefMut<U>` is emitted but every `StructBorrow` is recorded as an
immutable borrow — a `StructBorrowMut` recording a *mutable* borrow is still the open extension it
was; and the **static route constructs nothing under this revision** — `Tr_::m(x)` is a plain call on `x`, a
use the analyzer already sees — so item (k) shrinks to the `&dyn` coercion in *argument position*
(`f(TrAdapterRef<U>(x))`), which today records no borrow: the only `StructBorrow` emission site
(`src/ir/mod.rs`) fires for a named-local construction `lhs = Ctor(args)`. Item (k) is an IR
addition (expression-level `StructBorrow`, released at the end of the full-expression), not a
verification (§3.2.16 (k)).

*(The 09-23 closing paragraph on "why interface + adapter rather than inheritance" is superseded by this section's tier-1 opening and §3.2.15.)*

#### 3.2.11 What the transpiler does (lexical) vs. does not (semantic)

The whole design hinges on keeping the transpiler on the cheap side of the name-resolution /
type-inference cliff:

**Does (lexical, bounded — a name resolver + transliterator):**
- parse; translate `use` / module deps → `import`;
- classify `impl T {}` vs `impl Tr for T {}` → member vs (tier 1) override vs (tier 2) free function in the
  trait's namespace;
- classify method *names* globally → inherent / trait / both (drives §3.2.3), with each trait
  method's **receiver kind** (drives the receiver spelling at the call, §3.2.3) and each default's **call graph over
  other defaults** (drives §3.2.13's layer choice);
- emit `x.m()` as a native member, as the CPO call `Tr_::m(x)`, or as the shim (§3.2.3), and `Tr::m(x)` as
  `Tr_::m(x)`;
- **enumerate and order the candidate traits** for a multi-owner name (§3.2.6) from the receiver's
  declared-type regime — elaborated bounds first for a type parameter, lexical scope for a concrete
  type, their union under the guard when the declared type is not lexically recoverable;
- emit the type-traits map, the marker + concept, the `Tr_` namespace (tag, CPOs, default templates,
  bridges), the thin interface and its three generic forwarders.

**Does not (semantic — what would make it a mini-rustc, and clang does instead):**
- infer the type of any expression;
- select which impl applies / solve trait obligations / check coherence & overlap;
- borrow-check generic trait code; monomorphize.

**The candidate ladder against this budget.** §3.2.6 is the one place the transpiler names traits
at a call site. The candidate set comes from the same `use` / prelude / bound information the
shipped lane consumes to decide which `using namespace Tr_;` to inject, and "which traits own `m`"
is the global classification already on the *does* list. The transpiler **orders** that set;
clang decides the receiver's type, whether each candidate's marker is specialized for it, and
therefore which arm is live. The `static_assert(n <= 1)` is what keeps ordering from becoming
adjudication: if the order ever mattered to the answer, the assertion has already fired. The
dividing test is unchanged: nothing here needs "the type of `a`" or "which impl wins."

**Tier membership against this budget.** Every §3.2.1 test is a property of one declaration or a
per-crate relation between declarations; none requires the type of an expression. Two duties are added
to the *does* list: decide each `(trait, impl)`'s tier from those declarations, and enumerate, per
method name, every tier-1 interface (own or from a manifest) that declares it — the exclusion list the
shim's inherent arm carries (§3.2.3). One is added to the *does not* list: decide whether a blanket or
conditional impl applies to a given type — §3.2.1 leaves that to the call site's concepts, which clang
evaluates. The shim is the lexical escape hatch for a receiver whose tier is not recoverable; it is
never wrong *provided its tier-1 arms test the base, not the name* (§3.2.3).

#### 3.2.12 Implementation status and migration

- **What ships behind `#[cfg_attr(any(), cpp_trait_member_dispatch)]` + `#[cfg_attr(any(), cpp_inherit)]`**
  is a partial tier 1: one interface class with plain names, `struct Dog : public Animal`, `override`
  for required methods, a synthesized fieldwise constructor, an inlined default when the body is a
  single expression calling same-trait methods, no `Animal_` namespace, no adapters (measured, §3.2.15).
  Everything else in §3.2.17 is **not** emitted, and each is a phase-0 item (§3.2.16): one base slot per
  type, un-parameterized and never virtual (no two tier-1 traits on one type, no generic-trait base, no
  `public virtual` supertrait — `derived_from<D, Super>` false, upcast ambiguous); generic defaults →
  `TODO … not yet supported` and the default emitted *pure*, so every implementor is abstract; a default
  that calls a supertrait method emitted pure (body dropped); a `Self`-typed parameter reaching the
  interface as `const Tr&` (implementor hides, becomes abstract); `-> Self`, RPITIT, APIT reaching the
  interface; no `&&` slots (a `self` receiver emitted `const`); assoc-const traits skipping the interface
  entirely (a `cpp_inherit` implementor of one is five compile errors); tuple and unit structs without a
  constructor; `derive(Clone)` emitting a designated initializer on the now-non-aggregate; a synthesized
  lone move constructor deleting the copy constructor (`derive(Copy)` types non-copyable, `let q = p;`
  lowered to `std::move`); the interface's special members *deleted* (§3.2.2); `Box::new(local)` into
  `Box<dyn Tr>` naming the suppressed adapter; the call-site `&dyn` coercion likewise; path syntax
  applied per *trait*, so `Tr::m(&i)` on a tier-2 impl of the trait becomes a member call on an `int`;
  bounds emitted unconstrained with the `deref_call` shim; `cpp_inherit` on a foreign self type a *silent*
  no-op; non-`pub` traits anonymous-namespace-wrapped; a type declared before its trait inheriting an
  incomplete class; an `fn tenfold` override mis-emitted as `operator-`.

- **Tier 2 is the shipped lane revised in place, and the revision is not implemented.** The shipped
  default lane is UFCS free functions (`Tr_`, `rusty_ext`) with forwarding adapters and injected `using
  namespace`; it already has the forward-declaration pass, the Fix B bridge, the Fix A markers and the
  non-template-▷-template tiebreak this design keeps. What changes, item by item — each a §3.2.15
  measurement, each a phase-2 step (§3.2.16):

  | shipped (2026-06) | revised (2026-10-04) | why |
  |---|---|---|
  | `using namespace Tr_;` injected per trait, global, unconditional | retired; every call spells `Tr_::m(x)` | directive leaks into child namespaces, inner-scope hiding, local-name capture (§3.2.5) |
  | `Tr_::m(const U& self_, …)` — the method name *is* the function | `Tr_::m` is a CPO; the function is `Tr_::impl_::m(tag, const U&, …)` | a qualified call in a template freezes its overload set across modules (`11000` vs `5550`); a bare name in `Tr_` would find the CPO and suppress ADL (§3.2.2 rules 1–3) |
  | default body: `self_.hello()`, member-first | default body: `Tr_::hello(self_)` (the CPO) | inherent shadow in a default (`2002` vs `2`, §3.2.13) |
  | per-impl forwarding adapters (3 per impl), each slot a qualified `Tr_::m(value_)` | 3 *generic* forwarders per trait, each slot the CPO | `dyn bar.describe = 10` vs `777` through a qualified slot (§3.2.10) |
  | `__ufcs_impls(const U&)` marker + `requires { Tr_::__ufcs_impls(s) }` on defaults (Fix A) | `impls_Tr<U>` defined-false marker + `requires has_Tr<S>` on defaults | loud on misordering where the SFINAE marker was soft (§3.2.3) |
  | `emit_multi_owner_ufcs_call`: guard-less first-wins `if constexpr` over `A_::foo` / `B_::foo`, traits not in scope included | scope-derived candidates, `static_assert(has_A + has_B <= 1)`, base-excluded inherent arm | silent first-wins where rustc is E0034 (`16`, `32`, §3.2.6) |
  | `rusty_ext` lane for local trait × foreign self type | the placement rule: such impls live in `Tr_::impl_` (§3.2.2, §3.2.5) | one rule instead of two lanes; same bytes, different name |
  | `impl Tr for &T` / `for T` collapse: warning + parked body | trailing `rusty::self_tag<Self>` | `impl Tr for i32` + `for &i32` silently routes to one body (§3.2.4) |
  | generic trait, second impl on one type dropped (`HARD C++ LIMIT`) | trailing `rusty::tag<A>`, defaulted; omitted when an argument determines `A` | `<uint8_t, X>` missing; arg-inferred `t.m(av)` = rustc `304` (§3.2.2 rule 7) |
  | type-parameter receiver: `deref_call` shim, bound unconstrained | `A_::foo(x)` from the bound; `requires has_A<X>` | the unqualified form is `ambiguous` at `g<int>` once a second owner exists (§3.2.6) |
  | Fix B bridge for every nested-module impl | bridge only for foreign-self impls in nested modules; local-self impls emitted in place | receiver-namespace placement: 132/249 serde_core impls need no bridge (§3.2.5) |
  | non-vtable members (generic required methods, `-> Self`, `new`, consts) skipped | free functions / templates / `inline constexpr` in `Tr_::impl_` + `TrTraits` | serde's `Serialize` / `Deserialize` get no interface today (§3.2.2) |
  | **ABI-pinned companions**: `Tr_::m(U& self_)` for a `cpp_inherit` impl and every `pub` trait's `<Tr>_` functions are symbols an *incumbent* C++ object owns (srpc's ratified ABI: `rrr::Job_::{Ready,Work,Done}(OneTimeJob&)`; 50 `Serialize_`/`Deserialize_`/`rusty_ext` symbols in rrr.serializable; pinned by `test_cpp_inherit_impl_keeps_both_virtual_members_and_ufcs_companions`) | the pinned non-template companions stay in `Tr_` as *overloads beside the dispatcher*, forwarding to the member / `impl_` body — which requires the CPO to be a constrained **function template** `template<class S, class... R> requires (!same_as<remove_cvref_t<S>, impl_::tag>) auto m(S&&, R&&...)` rather than a function object (an object cannot share its name with a function; a function template can, and ordinary lookup finding a *function* does not suppress ADL — this also removes rule 1's hazard). **Measured 2026-10-07**: the function-template CPO with a pinned `Shape_::area(const Sq&)` companion is identical to the rustc oracle on every census cell (§3.2.15) | a design constraint the 10-04 revision did not record; found 2026-10-07 at `mod.rs:21161` |

- **Landed (phase 2, in the order §3.2.16 gives).** *2026-10-07 — step (6) + rule 6:* multi-owner method
  calls take their candidate set from scope — the receiver's elaborated bounds for a type-parameter receiver,
  else the traits `use`d or declared in the call site's Rust module (`scope_import_bindings`,
  `ufcs_declared_trait_modules`; a glob import marks the scope *unknown* and keeps the shipped all-owners
  ladder) — one candidate is the single-owner call, two or more the ladder with `static_assert(__ufcs_n <=
  1)` (§3.2.6); a `self.m()` inside a default body resolves to `<Tr>_::m(self_)` first, member only as
  fallback (§3.2.13 rule 6); a method-less concrete impl (`impl A for u8 {}`) emits its Fix-A implementor
  marker. Oracle: `tests/transpile_tests/trait_probes_scoping` flips to PASS; unit tests `test_ufcs_*`
    (scope, guard, bound regime, default body, marker). The guard counts implementors by the exact-type
  marker, never by call viability: `foo(const int32_t&)` is viable for an `int64_t` receiver through an
  integral conversion, and a viability count fired E0034 on valid Rust (measured on the scoping probe).
- *2026-10-07 — step (2):* `Tr_::m` is the **dispatcher** — a function template (not an object) whose
  unqualified inner call does tag-ADL at the point of instantiation; emitted in **two overloads**, a plain
  call and a template-id call `m<E0, E...>(tag, …)` for explicit template arguments (a template-id binds
  function templates only, so a single explicit-pack dispatcher could not reach a non-template impl —
  measured). Impl functions, default templates and the Fix B using-declaration bridges live in
  `Tr_::impl_` with `::Tr_::impl_::tag` as parameter 0 (rules 1–2); call sites still spell `Tr_::m(x)`
  (rule 3). A `cpp_inherit` impl keeps its pre-revision `Tr_::m(Self&, …)` as a non-template forwarder
  beside the dispatcher — the ABI-pinned companion of the table above. The `rusty_ext` lane is untouched
  (step 5). `impl Tr for &T` is still keyed on `const T&` and invisible to cvref-stripping lookups
  until `self_tag` lands (step 7).
- *Marker hygiene found by the comparison gates (2026-10-07):* the explicit-specialization dedupe keys on
  the same canonical spelling the free-function emitters use (`isize`/`i64`, `NonZero<usize>`/`<u64>` are
  one C++ type — `redefinition of impls_Serialize<long>` otherwise); a nested-module impl whose self type
  contains a **bare local type name** after global qualification (serde's `Error`, hashbrown's
  `std::span<const Tag>`) skips its marker with a visible comment — an explicit specialization must sit at
  the primary's scope where that name does not resolve; a blanket's partial specialization is skipped when
  a parameter appears only in a non-deduced context (`f<N>()`, `T::X`; alloc's
  `impls_IsZero<std::array<T, sanitize_array_capacity<N>()>>` is `-Wunusable-partial-specialization`).
- **Baseline repairs (2026-10-07), found by the first gate on this tree, all in the trait lane:** the
  empty interface shell for a trait whose methods are all generic / by-value ignored the trait's
  template parameters while its forward declaration carried them (`redefinition … as a different kind
  of symbol`: tap, bitflags, smallvec); an associated-const trait was forward-declared as an interface
  class and then aliased to its `RuntimeHelper` (arrayvec); an unresolvable trait path's placeholder
  key opened `namespace @unresolved-trait {` (arrayvec; now a visible comment and no bridge). Still
  open on `main` and outside this lane: `::de::value::rusty_ext::into_deserializer` spelled where the
  function lives in `__private::de::rusty_ext` (serde_core, serde, serde_bytes); the `alloc`/`rusty`/
  `path` stdlib-port build scripts (cargo `rustc` extra-arguments rule; port codegen errors).
- **Coverage.** The reviewer's census over the local matrix crates (`either`, `bitflags`, `serde_core`,
  `smallvec`, …): roughly 4 of ~288 crate-trait `(trait, impl)` pairs satisfy §3.2.1 — and the four are
  tuple structs the shipped lane cannot construct. Tier 1 does not change how those crates transpile; it
  changes how application code and C++-interop-facing types do. §3.2.16 (p).
- **Migration** (§3.2.16): phase 0 fixes the tier-1 lane's defects; phase 1 makes tier 1 the *default*
  for every `(trait, impl)` that passes §3.2.1, with the free-function lane carrying the rest; phase 2
  revises the free-function lane in place per the table above, behind a per-crate switch; phase 3 deletes
  what the revision orphaned (`rusty_ext`, the Fix A markers, the `using` injection, the per-impl adapters).

#### 3.2.13 Default methods

A default method lives in the trait declaration, is generic over `Self`, and calls the type's other
methods through `self`. Which C++ form it takes follows a lexical call graph over the trait's default
bodies, and the answer is the same in both tiers because it is a property of the *trait*:

```rust
trait Greet {
    fn hello(&self) -> i32;                                                          // required
    fn describe(&self) -> String { format!("v={}", self.hello()) }                   // default: calls only a slot
    fn map_hello<F: Fn(i32) -> i32>(&self, f: F) -> i32 where Self: Sized { f(self.hello()) }   // generic default
    fn via(&self) -> i32 where Self: Sized { self.map_hello(|x| x + 1) }             // default calling it
}
```

```cpp
// ===== in Greet's module: the bodies, ONCE, in the namespace =====
namespace Greet_::impl_ {
    template<class S> rusty::String default_describe(const S& s) { return rusty::format("v={}", Greet_::hello(s)); }   // self.hello() → the CPO
    template<class S, class F> int32_t default_map_hello(const S& s, F f) { return f(Greet_::hello(s)); }
    template<class S> int32_t default_via(const S& s) { return Greet_::map_hello(s, [](int32_t x) { return x + 1; }); }  // self.map_hello() → the CPO
    // tier-2 receivers reach them through the CPO: the constrained default overloads (§3.2.2 rule 6)
    template<class S> requires (has_Greet<S> && !std::derived_from<S, Greet>) rusty::String describe(tag, const S& s) { return default_describe(s); }
    template<class S, class F> requires (has_Greet<S> && !std::derived_from<S, Greet>) int32_t map_hello(tag, const S& s, F f) { return default_map_hello(s, std::move(f)); }
    template<class S> requires (has_Greet<S> && !std::derived_from<S, Greet>) int32_t via(tag, const S& s) { return default_via(s); }
}
// ===== the interface: tier-1 receivers reach the same bodies through members =====
class Greet {
public:
    virtual int32_t hello() const = 0;                                        // required: pure slot
    virtual rusty::String describe() const                                    // slot-only default: non-pure virtual …
        { return Greet_::impl_::default_describe(*this); }                    //   … forwarding to the ONE body; hello() → bridge → vtable → the implementor
    template <class F> int32_t map_hello(this auto const& self, F f)          // generic default: EXPLICIT-OBJECT member …
        { return Greet_::impl_::default_map_hello(self, std::move(f)); }      //   … `self` deduces the implementor's type
    int32_t via(this auto const& self)                                        // default calling it: explicit-object too
        { return Greet_::impl_::default_via(self); }                          //   → Greet_::map_hello(self) → bridge → self.map_hello(): the implementor's own, if any
};
struct Dog : Greet { int32_t hello() const override { return 5; }
    template <class F> int32_t map_hello(F f) const { return f(5) * 10; } };   // override = hiding; via() reaches it (60)
// tier 2:  impl Greet for Bar { fn hello … fn describe(&self) -> String { "777".into() } }
namespace Greet_::impl_ { int32_t hello(tag, const Bar&); rusty::String describe(tag, const Bar&); }   // NON-template ▷ the default template
```


- **A default that calls only vtable slots** (`describe`) is a **non-pure `virtual`** on the interface for tier 1 and a constrained default template in the namespace for tier 2 — **one body**, `default_describe`, which the virtual forwards to. Its `Greet_::hello(s)` resolves through the bridge and the vtable to a tier-1 implementor's override, and through overload resolution to a tier-2 impl function; a tier-1 impl overrides it with `override`, a tier-2 impl with a non-template overload (`describe(tag, const Bar&)` = `777`). Measured (§3.2.15): static and `dyn` both match rustc in one TU and across two named modules, and it *fixes* a shipped bug — the shipped default resolves `self_.hello()` to an *inherent* `hello` where Rust's default body sees only the trait's (`2002` vs `2`); the CPO sees only trait impls.
- **A generic default, and every default that transitively calls one,** is a **non-virtual
  explicit-object member** of the interface (C++23; `this auto const&` for `&self`, `this auto&` for
  `&mut self`, `this auto&&` for `self`). The object parameter deduces the implementor's static type,
  so `self.map_hello(…)` inside `via` binds the implementor's own member template when one exists —
  Rust's override, by name hiding — and the interface's default otherwise. This holds for an override
  in *any* crate with the upstream interface unchanged: measured `dog: gmap=60 via=60 | cat: gmap=6
  via=6` = rustc in one TU, across two named modules with the interface precompiled before the override
  existed (`60`), for a downstream subtrait's default calling the upstream generic default (`60 70 60`),
  and for all three receiver kinds (§3.2.15). The shape it replaces — the default as a `virtual` whose
  body calls the generic member through `this` — binds `Tr::map_hello` from the base and is silently
  wrong for any override (`6` where rustc gives `60`). The Rust side must carry `where Self: Sized` on
  the generic default and its callers (E0038 otherwise), which is exactly the non-slot spelling; no
  `dyn` route is lost, because Rust cannot call them on `dyn`. Forward-only caveat: C++ code that calls
  such a default through a base-typed reference (`const Tr& t = dog; t.via()`) gets the base body
  (`6`), a call Rust cannot express. **This retires the CRTP `TrDefaults<Adapter>` mixin in both tiers.** For tier 2 the question does not
  arise: a tier-2 override of a generic default is a more-constrained function template in the namespace
  (`each(tag, const Wrapper<T>&, F)` beats the default `each`), and `Greet_::via(w)` reaches it by ADL at
  instantiation, across modules (measured `77`, §3.2.15). The manifest carries, per trait, the names of its non-vtable defaults, so a
  downstream subtrait whose default calls one classifies the call correctly.
- **`-> Self` defaults with `where Self: Sized`** are explicit-object members with a deduced return
  (`auto dup(this auto const& self) { return self; }`), which is how `Self` acquires a C++ spelling.
- **`Self::Assoc` in a default body** is a plain name on an interface parameterized by its associated
  types (§3.2.8); in a namespace body it is `TrTraits<S>::Assoc`.

**The override mechanism in tier 2 is overload resolution, and it works across modules only through the
CPO.** A concrete impl's `describe(tag, const Bar&)` is a non-template; the default is a template;
[over.match.best] prefers the non-template. A blanket's constrained template beats the default by being
more constrained. That is the shipped tiebreak, retired on 09-29 as "a simulation of override built from
overload resolution" and **un-retired here** — because the probes showed it is not a simulation that
drifts: it matched rustc in every default cell (single TU `a: … describe=2 each=11 via=2 | b: describe=777
via=6 | i32: describe=42 via=22`; two modules `down.via()=60 down.each(+1)=60 plain.via()=6 up.via()=6
down.describe()=10 show(down2)=777 | dyn down2.describe=777`), *provided* the default body and the dyn slot
call the CPO. With a qualified call to the default template instead, the override is invisible from
another module (`sonly=11000` vs `5550`; `show(down2)=10` vs `777`) — the 09-29 adapter's vtable had no
such failure mode, which is the one structural advantage the namespace carrier gives up and re-earns
through §3.2.2 rule 3.

**E0117 bounds the exposure.** A primitive impl of a *non-generic* trait can only live in the trait's own
crate (`only traits defined in the current crate can be implemented for primitive types`), where the
emitter controls declaration order; so the cross-module "default compiled before the primitive impl
existed" case exists only for a generic trait with a local argument — `impl Tr2<Local> for u8` in a
downstream crate — and that is the case the CPO is measured on (`describe2=42`, §3.2.15).

> **Retired:** the 09-23 CRTP `TrDefaults<Adapter>` mixin and the 09-29 body-carrying adapter's
> `override` / hiding-member override form (§3.2.15). **Kept in purpose, changed in mechanism:** Fix A of
> §3.2.14 — the `Tr_::__ufcs_impls(const U&)` marker per concrete impl and the `requires {
> Tr_::__ufcs_impls(s) }` clause on default templates, which solved the multi-owner-default problem
> (serde's `size_hint` ∈ `MapAccess` ∩ `SeqAccess`) — is the `impls_Tr` type-trait of §3.2.3 and the
> `requires has_Tr<S>` clause on every default template, loud on misordering where the free-function
> marker was SFINAE-soft. A multi-owner *call* is the §3.2.6 ladder.

**Would be fixed by this design** (both are open bugs in the shipped lane; neither is fixed until
§3.2.16 phase 2 lands):
1. §3.2.14's *"interface-default-body instantiation"* — an object-safe default calling a required
   method made the interface's `virtual m()` instantiate the free-function default template on
   the abstract interface itself (`Z_::rz<Z>` → "no member `v` in `Z`"). Here the base default forwards to the
   namespace body, whose `Greet_::hello(*this)` reaches the pure virtual through the bridge; `describe`
   above is that case.
2. A default calling a required method on a type with a same-named **inherent** method: the
   shipped static route `Greet_::describe(const Self_&)` calls `self_.hello()` with `Self_ = Foo`
   and picks the *inherent* `hello` — `2002` where rustc gives `2` (§3.2.15); the `dyn` route was
   already right. The CPO call in the default body sees only trait impls on both routes (measured
   `describe=2`, §3.2.15).

**What needs wiring (phase 2).** The emitter already places slot-only defaults on the interface (the
shipped `virtual twice()` shows it) and already emits one `template<class Self_>` default per method in
`Tr_`. New: the `impl_` split and the tag (§3.2.2 rules 1–2); the CPOs (rule 3); default bodies lowered
with the CPO for `self.m()` (rule 6) and the body-kind flag that keeps impl bodies on the ordinary
lowering (§3.2.3); the interface's virtual and explicit-object members as *forwarders* into the namespace
bodies; `requires has_Tr<S>` on the default templates in place of the `__ufcs_impls` clause; the
non-vtable members of §3.2.2; removal of the member-first shim from default bodies. The
body-transpilation caveats — expressing a default *generically*, bounds as constraints, per-impl
materialization as the fallback for a default that cannot be — are unchanged and independent of where
the default lives. Assoc-const / runtime-helper traits: the `<Tr>RuntimeHelper` static path for their
*defaults* depends on the **struct members** that §3.2.2 retires (reviewer probe `rh_dep.cpp`), so it
cannot simply be kept — in phase 2 their defaults are namespace templates like any other default and
their consts `inline constexpr` in `Tr_::impl_` / `TrTraits`; until then those traits keep their struct
members. Phase-2 gate (§3.2.16).

#### 3.2.14 Implementation realities (the long tail)

**Disposition under this design.** Each invariant below was learned making real crates compile under
the shipped lane — the lane this revision keeps and revises in place — so almost all of them are
**inherited as written**; the 09-29 table that marked most of them "deleted with `Tr_`" is superseded.
The "(implemented)" labels in the kept text refer to the shipped lane. One column states whether the
invariant touches tier 1 (almost none does: the long tail was learned making free functions resolve, and
tier 1 has none).

| Invariant / mechanism | tier 1 | disposition |
|---|:---:|---|
| Owner-map pruning — never qualify to a symbol you didn't emit | – | **Inherited.** The ledger (`ufcs_emitted_trait_methods`) records emitted impl functions per `(trait, self type, method)`, filled by a **pre-pass over impl blocks**, not by emission order; a call may spell `Tr_::m` only if the pre-pass recorded an owner for `m`. |
| The cross-crate manifest classifies; it does not qualify | ✓ | **Inherited, extended.** Still classifies bare names and names owning traits (`method_owners`, now listing tier-1 traits too); the emitted call is the bare `Tr_::m` CPO, never `dep::Tr_::m` — every crate emits `Tr_` at global scope inside its own module. From **phase 1** it also carries each trait's tier and supertraits, the names of its non-vtable defaults, per-method receiver kinds, blanket presence per `(trait, method)`, and per dependency type its inherent method names; a stale manifest is invalidated, never trusted. |
| Free functions are global; impl-local types are not (Fix B) | – | **Inherited for foreign-self impls in nested modules; retired for local-self impls** (§3.2.5): a local self type's functions are emitted in its declaring namespace, where the body's relative names resolve in place (132/249 serde_core impls, measured). The helper-namespace + re-emitted using-declaration bridge survives for the other 117/249, bridging into `Tr_::impl_`. |
| Multi-owner defaults need a constraint, not a guess (Fix A) | – | **Mechanism changed, purpose kept:** `impls_Tr` marker + `requires has_Tr<S>` on default templates (§3.2.3, §3.2.13). |
| Distinct Rust types, one C++ type → dedupe by canonical signature | – | **Inherited** for `isize`/`i64` and the alias families, the tag now part of the signature. **Fixed** for `impl Tr for &T` vs `impl Tr for T`: `rusty::self_tag<Self>` makes them distinct overloads (§3.2.4, decision (f)). |
| The impl-collapse preserved member `rusty_<Tr>_<m>` + tagged probe | – | **Superseded by the row above** once `self_tag` lands; until then inherited. |
| Open: associated-type / generic resolution in free-function *bodies* | – | **Inherited (open).** A default template's body names `Self::Assoc` and needs the `TrTraits<S>::Assoc` substitution (§3.2.13). |
| Trait static methods in free-fn bodies | – | **Inherited.** |
| `PhantomData` as a value, concrete-type-as-phantom-param | – | **Inherited.** |
| Don't intercept a runtime-helper method with UFCS | – | **Inherited.** `method_prefers_runtime_helper_namespace` applies to the CPO interception as it did to the free-function interception. |
| Open: interface-default-body instantiation | – | **Would be fixed** (§3.2.13). |
| `rusty_ext` — local trait × foreign self type lane (`emit_cross_crate_rusty_ext_bridge`, `rusty_ext_methods_by_module`, the `rusty_ext_fallback` probe, its block-relocation splice) | – | **Folded into the placement rule** (§3.2.2, §3.2.5): a foreign-self impl of a local trait lives in `Tr_::impl_`, which is where `rusty_ext` put it under another name. The separate namespace, the fallback probe and the retargets are deleted in phase 3; the relocation splice *is* the Fix B bridge and stays. |
| `rusty::deref_call` / `__mdisp_*` — the deref-chain method dispatcher (`include/rusty/dispatch.hpp`) | ✓ | **Kept.** It is what walks the shim's deref arms (§3.2.3), for a receiver of either tier. |
| Internal-linkage wrapping of synthesized trait machinery (C21) | ✓ | **Kept** for the namespace machinery and the forwarders. A tier-1 interface is an exported class, not synthesized machinery; wrapped only when the trait *and all its implementors* are non-`pub` (§3.2.5). |
| Dependency-provided dedup; `Self_` turbofish threading; text-splice relocations | – | **Kept.** Each exists to make a free function's signature or call resolve, and the carrier is still a free function; the tags make most explicit `Self_` arguments unnecessary at call sites but not in the declarations. |
| `impl Tr for &T` vs `impl Tr for T` collapse | – | tier 2 (references are never tier-1 self types); `self_tag` (above). |
| The TRAIT-ARG collapse (task #206: two instantiations of one generic trait on one type) | ✓ | the shipped lane warns `HARD C++ LIMIT` and parks the losing body; under this design the pair is tier 2 for that type by §3.2.1 and the impls are overloads keyed by `rusty::tag<A>` (§3.2.2 rule 7). |
| The injected `using namespace Tr_;` (three emission sites, `mod.rs`) | – | **Deleted in phase 2** — the first step, since every other step spells `Tr_::m` (§3.2.5). |
| Per-impl forwarding adapters | – | **Replaced** by three generic forwarders per trait (§3.2.10). |

The model above is clean; making real crates (serde, itertools) compile flag-on surfaced a
set of invariants that are easy to violate and worth stating outright. Each is a consequence
of one fact: **a qualified member access to a name C++ cannot find is a hard error, not a
SFINAE soft failure.** Qualification buys precision but forfeits the "try the next candidate"
safety net, so every qualified name the transpiler emits must be guaranteed to exist.

- **Owner-map pruning — never qualify to a symbol you didn't emit.** Call-site qualification
  reads an owner map (method name → owning trait). That map must be pruned to the methods
  *actually emitted* (`ufcs_emitted_trait_methods`), not merely declared. The regression that
  taught this: `into_either` was added to the owner map as a default, but the emitter skipped
  it for the concrete impl, so `x.into_either()` qualified to a non-existent
  `IntoEither_::into_either` → hard error. A method may be classified-and-routed only if its
  `Tr_::m` will be there.

- **The cross-crate manifest classifies; it does not qualify.** `ufcs-traits.json` (§3.2.7)
  tells a downstream crate *which* bare names are trait methods and *which* trait owns each —
  enough to classify a call and pick the `Tr_` to name. But the emitted call is **bare**
  `Tr_::m`, never `dep::Tr_::m`. Every crate emits its `Tr_` namespaces at global scope
  *inside its own C++ module*; a downstream `import` brings those names into scope unqualified.
  A module-prefixed `dep::Tr_::m` names a namespace that does not exist. The manifest's module
  field is for disambiguating *classification*, not for building a path.

- **Free functions are global; impl-local types are not (Fix B).** A trait method that was a
  member *inside its type's module* flag-off becomes a free function in `Tr_` at **global**
  scope flag-on. Its self/param/return types must therefore be qualified to their declaring
  module: `VariantAccess_::unit_variant(de::value::private_::MapAsEnum self_)`, not bare
  `MapAsEnum`. A `local_type_module_path` map drives this, with an **ambiguity guard**: a bare
  name declared in ≥2 modules cannot be qualified from a global name→module map alone (which
  module did *this* impl mean?), so it is left bare. That guard is correct but incomplete — the
  residual ambiguous names (serde's `Error` in `de`/`ser`/`private_::doc`) need
  *impl-module-context* resolution (qualify relative to the impl's own module first), which the
  current global map cannot do.

  Fix B reaches *signatures* only. A free-function **body** still names impl-module entities
  module-relative (`I32Deserializer::new_(…)`, `.map(private_::unit_only)`), which do not
  resolve at global `Tr_` scope. A tempting `using namespace ::de::value;` inside `Tr_` is the
  *wrong primitive*: flat using-directives place names at one scope level, so an injected nested
  name (`de::value::private_`) collides with a global same-named one (`private_::doc`) →
  `reference to 'private_' is ambiguous`, where real lexical nesting would have shadowed.

  **Resolution (implemented).** A nested-module impl's free functions — both the forward
  declaration and the definition — are emitted into a per-module helper namespace
  `namespace <impl-module> { namespace __ufcs_<Tr> { … } }`, where the body's relative paths
  resolve by lexical shadowing (`private_` finds `de::value::private_`, shadowing the global
  `private_::doc` — exactly as the member form did flag-off). The names are then bridged into
  `Tr_` with a `using`-**declaration** (`namespace Tr_ { using ::de::value::__ufcs_<Tr>::m; }`),
  which is what every call site's qualified `Tr_::m` resolves against. Crate-root impls (empty
  module path) keep the original flat `Tr_` shape. Three subtleties make this work: (1) the body
  is emitted with `module_stack` *unchanged* — only literal namespace text is wrapped around it —
  so the bytes are identical to the flat emission and merely *relocated*; (2) the bridge is driven
  by the same actually-emitted ledger (`ufcs_emitted_trait_methods`) so it never names a
  non-existent symbol; (3) the `using`-declaration is **re-emitted per impl block**, not deduped —
  a using-declaration captures only the overloads declared *before* it, so re-emission after each
  block is what makes every incrementally-added overload visible in `Tr_`. The `__ufcs_impls`
  markers and `using namespace Tr_;` stay directly in `Tr_`. The remaining serde_core gap is now a
  *different*, orthogonal layer: distinct Rust types collapsing to one C++ type (`isize`/`i64`,
  `usize`/`u64`, `NonZero*`/`Atomic*` variants, `CStr`/`CString`) produce identical free-function
  signatures → `redefinition`; impl-module-context qualification for the residual ambiguous names;
  and a few STL-shape gaps.

- **Multi-owner defaults need a constraint, not a guess (Fix A).** When a *default* method is
  owned by two traits (serde's `size_hint` ∈ MapAccess ∩ SeqAccess), first-owner qualification
  is wrong: both owners emit unconstrained `template<class Self_> … size_hint(const Self_&)`,
  each matches every receiver, and the first always wins regardless of which trait the receiver
  implements. The fix reuses the §3.2.4 priority idea at the *default* layer: each concrete
  `impl Tr for U` emits a marker `Tr_::__ufcs_impls(const U&)`, the default template carries
  `requires { Tr_::__ufcs_impls(s) }`, and a base `void __ufcs_impls();` is declared per
  constrained trait so the `requires` is *SFINAE-false* (not a hard "no member") for a trait
  that declares the default but has no concrete impl in this TU (`SerializeStructVariant`). A
  multi-owner call then tries each `<Owner>_::m` qualified — no unqualified `size_hint` to
  collide with the `de::size_hint` *module*.

- **Distinct Rust types, one C++ type → dedupe by canonical signature (implemented).** serde
  provides separate impls for `isize`/`i64`, `usize`/`u64`, `NonZero*`/`Atomic*` width variants,
  and `CStr`/`CString`/`OsStr`/`OsString`/`Path` (all emitted as `std::string`). These lower to
  identical C++ free-function signatures in one `<module>::__ufcs_<Tr>` (or flat `<Tr>_`)
  namespace → C++ `redefinition`. The non-UFCS path already deduped via
  `extension_free_function_dedupe_key` (whose `canonicalize_extension_overload_type_for_dedupe`
  folds `ptrdiff_t→int64_t`, `size_t→uint64_t`, the rusty alias families, …); the UFCS decl/def
  emitters now apply the same dedupe through a per-phase, per-`(module,trait)` seen-set
  (`ufcs_def_dedupe_seen`, cleared at the start of *both* the decl and def phase so the two
  dedupe identically and the bridge matches the definitions). One definition serves both Rust
  types — they *are* the same C++ type, so the call site for either binds to it. (Validated:
  serde_core's `redefinition` bucket → 0.)

- **Open: associated-type / generic resolution in free-function *bodies*.** Fix B and §3.2.8
  qualify a free function's *signature* (`rusty::Result<typename ::de::VisitorTraits<V>::Value,
  E>`), but a generic/default body still names the trait's associated types unqualified
  (`Result<Value, E>::Err(…)`, `Error::custom(…)`), which do not resolve at free-function scope —
  and worse, a bare `Error` is captured by a `using namespace Error_;` for the *trait* `Error`
  (→ "unexpected namespace name 'Error_'"). This is the body-side counterpart of the §3.2.13
  default-body caveat and the dominant remaining serde_core layer; it needs the body emitter to
  carry the same `Self::Assoc`→`TrTraits<Self>::Assoc` substitution the signature path uses.

- **Trait static methods (associated functions, no `self`) in free-fn bodies (implemented).** A
  trait-name static call `Error::invalid_value(…)` (Rust infers `Self = E` from the method's
  `E: Error` bound + the return type) must lower to `E::invalid_value(…)`. The member emitter does
  this because it pushes the unstripped `method.sig.generics` (so the trait→param bound map has
  `{Error: E}`); the free-function emitter pushed bound-*stripped* `free_generics` (the emitted
  `template<class E>` must carry no constraint against the abstract interface), leaving the map
  empty → a bare `Error::` leaked. Fix: re-populate the body scope's bound map from the original
  generics after the push — resolver state only, emitted template unchanged.

- **`PhantomData` as a value, and concrete-type-as-phantom-param (implemented).** Two small but
  serde-blocking emission bugs: (1) a bare `PhantomData` value in the no-expected-type path (a UFCS
  member-fallback shim's `auto&&` arg) emitted the bare class-template name `rusty::PhantomData`
  (a parse error) instead of a constructed `rusty::PhantomData<…>{}`; (2) a unit-struct used as a
  concrete associated binding (`impl Visitor for IgnoredAny { type Value = IgnoredAny }`) was
  collected as a *spurious* `typename IgnoredAny` free-fn template param, which a body local of the
  same name then shadowed — fixed by excluding `local_declared_types` from the type-param candidates.

- **Don't intercept a runtime-helper method with UFCS (implemented, bitflags).** A handful of
  trait-method names (`size_hint`, `left`, `right`, `write_hex`) are special-cased flag-off to
  lower to a hand-written `rusty::<name>` runtime helper rather than a member call. Those helpers
  take their writer/sink parameter by **forwarding reference** (`Writer&& writer`), so a move-only
  lvalue argument binds without a copy. The UFCS per-type free function, faithful to Rust's owned
  `fn write_hex<W: Write>(&self, mut writer: W)`, takes that parameter **by value** — and the
  call-site shim forwards the argument as an *lvalue* (no move analysis at this layer). bitflags'
  `remaining.write_hex(writer)` (where `remaining: B::Bits` is a primitive and `writer: rusty::String`
  is non-copyable) therefore fails the dispatch `requires` on the by-value copy, falls through to the
  member-fallback `remaining.write_hex(writer)` on a *primitive* receiver, and hard-errors
  (`member reference base type 'const unsigned char' is not a structure or union`); `to_writer` is
  then ill-formed and every caller reports `no matching function for call to 'to_writer'`. Fix: the
  UFCS method-call interception skips exactly the names that prefer the runtime helper
  (`method_prefers_runtime_helper_namespace`, shared with the flag-off
  `should_prefer_runtime_namespace` set), so flag-on output for these names is byte-identical to
  flag-off. The owned-by-value-param vs. lvalue-forward mismatch is general, but these four names
  are the only ones with a forwarding-ref runtime helper that exposes it; a broader fix would need
  move-insertion at the UFCS argument layer. (Validated: bitflags 0 errors, 39/39 tests pass
  flag-on as of 2026-06-16.)

- **Open: interface-default-body instantiation.** An *object-safe* trait whose default body
  **calls a required method** makes the interface's `virtual m()` body instantiate the free-fn
  default on the *abstract interface class itself* (`Z_::rz<Z>` → "no member `v` in `Z`"). A
  default whose body does not call a required method (serde's `size_hint` returns `{}`) dodges
  this; the general case is unresolved and tracked as a dedicated follow-up. (Not hit by
  `serde_core`, which is GREEN flag-on as of 2026-06-16.)

#### 3.2.15 Why not "always a virtual class" — the 2026-09 investigation

This section is the evidence behind the design above: a proposal, the strongest form it was
escalated to, what was measured, and where every line of inquiry ended. All measurements: clang
22.1.8, `-std=c++23`, this repo's `include/`; the snippets are short enough to reproduce from the
text. Probe directories are under the session scratchpad (`inherit-probe/`, `escalation/`,
`review/`).

**The proposal.** Drop the free-function lane and *always* lower a trait to a virtual class that
the implementing struct inherits — "we always have all the Rust source, so we can always change
the generated C++ to add inheritance." Escalated, when the first objections were raised, to:
(A) map every primitive to a C++ class so it *can* carry a base, and (B) abandon prebuilt modules
and re-emit the whole program so any type can be re-emitted with new bases.

**Verdict.** *No* to the literal question — inheritance cannot be the only path. *Yes* to the
question underneath it — one lane is achievable, and it is the body-carrying adapter of this
section, with direct inheritance as its fast path. Every independent probe converged on that
mechanism; that convergence, more than any single measurement, is why the design changed. *Amended 2026-10-04:* the lane is a **namespace of free functions with tag-anchored ADL and per-method CPOs**, with a thin virtual helper for `dyn`; the body-carrying adapter was the 09-23 / 09-29 answer. Six probes written in both designs' fixed forms matched rustc on every measured cell; the namespace carrier did it at 2–4× less emitted code, closer to the shipped emitter, and admitted the arg-inferred generic-trait call the adapter key rejected. Its price — a lookup protocol whose invariants are silent if violated — is written down as §3.2.2's seven rules (measurements below).

**The two hard reasons `virtual`-for-everything fails.**
1. *C++ forbids `virtual` on member function templates* — a language rule in every standard
   mode, because a vtable is a fixed, finite array and a template is an infinite family. Rust
   encodes the identical fact as **object safety**: all 56 generic methods on `core::Iterator`
   carry `where Self: Sized` precisely to keep them out of the `dyn` vtable, and rustc rejects
   `dyn` on a trait with an unbounded generic method (E0038). Generic trait methods are pervasive
   — 86 of 132 `Itertools` methods, serde's `serialize<S>` / `deserialize<D>` — so a static lane
   survives by language rule. (The steelman *was* tried: monomorphize whole-program and emit one
   slot per instantiation. It works for `Self`-free returns — ~30 of `Iterator`'s 56 — and cannot
   for `Self`-parameterized returns; and no valid Rust program can dispatch through those slots.)
2. *A `virtual` base on a primitive is ruinous.* Itanium gives each non-primary polymorphic base
   its own vptr: `sizeof(I32)` is 16 at one base, 168 at twenty (`-fdump-record-layouts`: 20
   vptrs, payload at offset 160), **784 bytes on a 97-base model of std's surface on `i32`** — a
   1M-element `Vec<i32>` goes from 3.8 MB to 748 MB; no longer trivially copyable or standard
   layout; `repr(C)` `Point{x,y}` becomes 32 bytes; `memset` to zero nulls the vptr silently.
   Whole-program visibility (B) makes this *worse* by maximizing the base set.

**What the escalations buy — and where they stop.** (A) works for its target: `struct i32_cls :
MyTrait_` with a conversion operator compiles and runs, including `switch`, arithmetic and
class-typed NTTPs; marker traits are free (EBO: `sizeof` stays 4). With `virtual` *dropped* —
non-virtual CRTP empty bases — the entire cost case evaporates (`sizeof(I32)` 4 under 97 bases,
trivially copyable, identical vectorization, `repr(C)` / `atomic` / `bit_cast` / `memcpy` intact).
But a non-virtual base has no vtable, so `dyn` needs the interface + adapters back — two lanes
again, the free function respelled as a base member, the same call-site shim. The simplification
does not survive.

**The premise "we always have all the Rust source" — three ways it fails.** *Closures:* a C++
closure type is synthesized by clang at the call site — no header, no `.cppm`, nothing to re-emit;
every `.map(|x| …)` produces one. *Hand-written runtime types:* `rusty::Option`,
`rusty::slice_iter::Iter<T>` (`include/rusty/slice.hpp`) — adding a base is `redefinition of
'Option'`; (B) regenerates Rust, these are C++. *Cross-module re-emission is rejected outright:*
clang 22, `declaration 'Widget' attached to named module 'up' cannot be attached to other modules`.

**Trait scoping has no base-class analogue — and the failure is silent.** One type, `impl A for
T` and `impl B for T`, called from two modules that `use` different traits: rustc prints `1, 2`;
always-inherit with the obvious `using T_via_A::foo;` repair compiles at exit 0 and prints **`1,
1`** with zero diagnostics under `-Wall -Wextra -Werror`. All three spellings were compiled — emit
both bodies (hard error), collapse to one (silently wrong), proxy bases (false positive) — none is
Rust-faithful, because Rust's disambiguation lives at the *call site* and a base list on the
*class*. Per-trait renaming fixes it only if the transpiler decides which trait — the trait solving
§3.2.11 rules out. This is why §3.2.6 keeps the decision at the call site behind a `static_assert`.

**Measurements that shaped §3.2.2–§3.2.14.**

| Claim in the design | Measurement |
|---|---|
| Cross-module explicit specialization works and is conforming (§3.2.5) | primary in module `iface`, `template<> class SpeakAdapterRef<int32_t>` in module `impl_a`, consumer imports both: `int=70 / dyn=70` on clang 22.1.8 and gcc 14.2; [temp.expl.spec]/3,/4,/7, [module.reach]/3. A specialization in an *implementation* unit: direct use loud, marker silently false for importers (`via_iu=7000 has=0`) |
| Adapter route costs nothing at `-O1+` (§3.2.3) | `-O2`: adapter and free-function routes byte-identical (`leal (%rdi,%rdi,2),%eax; incl %eax; retq`), by-value delegating flavour included; `-O0` materializes the wrapper |
| A completeness predicate on a declared-only primary memoizes silently (§3.2.3) | `has<int>` evaluated before `template<> struct R<int>` → `early=0`; evaluated again after → still `late=0`; no diagnostic. A two-trait ladder emitted before the second impl resolved to the wrong trait with the guard never firing |
| A defined-false marker is loud on misordering — for explicit specializations only (§3.2.3) | `impls_R<int>` instantiated before its explicit specialization → `error: explicit specialization of 'impls_R<int>' after instantiation`; a *partial* specialization of the same defined primary after instantiation → silent stale false (reviewer probe `p5_partial_after_inst.cpp`) |
| No "defer to end of TU" escape for the ladder (§3.2.3) | a generic lambda's `operator()` — with `decltype(auto)` or a declared return type — is instantiated by the end of the enclosing function: a ladder placed before the specialization reads `early=-1` (reviewer probes `p6`, `p6b`) |
| Nested-module bodies already precede adapters in the shipped output (§3.2.16 (j)) | `inner::go` emitted at line 115, adapter specializations hoisted to 123–182, top-level bodies at 187+ (reviewer probe `order.rs`) |
| Marker specialized in another module; the non-importing TU disagrees silently (§3.2.5) | primary in module `a`, `template<> struct impls<int> : true_type` in `b`; a TU importing `a`+`b` reads true and runs; a TU importing only `a` reads false — both exit 0 |
| The shipped emitter keys adapters on trait args (§3.2.2) | `template <class T, class U> class ConvAdapter;` / `class ConvAdapter<int32_t, X> final : public Conv<int32_t>`; only the *first* impl's adapters are emitted (`<uint8_t, X>` missing) |
| Interface split + delegation + mixin reproduces rustc (§3.2.2, §3.2.10, §3.2.13) | trait with `&self`, `&mut self`, `self`, slot-only default, generic default, a default calling it, and an overridden supertrait default — rustc 1.97.1 oracle: `get=5 twice=10 base=99 gmap=60 via_g=60 \| dyn twice=12 base=99 \| consume=107`, and the C++ probe matches on every field; `const Tr& d; d.bump()` → `error: no member named 'bump'`. (`via_g` calls a `Self: Sized` method, so valid Rust makes it `Self: Sized` too — `dyn via_g` is E0038 — which is why it lives on the mixin and never on a slot; the probe's first cut put it on the vtable and reported a `dyn via_g` Rust cannot express) |
| A generic-calling default on the base binds the trait's version (§3.2.13) | same probe with `via_g` as a slot on the base: `via_g=6` (rustc `60`); reviewer's independent probe: `2/11` vs rustc `999/15` |
| Supertrait default inherited instead of delegated (§3.2.10) | reviewer probe: `1100` where rustc gives `99900` |
| Supertrait + subtrait sharing a method name collapse to one slot (§3.2.2) | `Super::foo` / `Sub::foo`: emitting both bodies → redefinition; one body → `dyn` upcast returns the sub body (`2` where rustc gives `1`) |
| Native override serves both routes (§3.2.13) | `Dog` overrides `twice`, `Cat` inherits: `static: 100 10 / dyn: 100 10` = rustc; 0 indirect calls in the static path at `-O2` |
| The §3.2.6 ladder must be a generic lambda | non-generic `[&]()` over a concrete receiver → `error: static assertion failed` on the discarded arm; `[&](auto&& r)` → clean |
| The shipped multi-owner form is silently first-wins (§3.2.6) | `emit_multi_owner_ufcs_call` emits a guard-less qualified `if constexpr` ladder over `A_::foo` / `B_::foo`, including traits not in scope; prints the first owner's body where rustc errors E0034. The unqualified form: `error: call to 'm' is ambiguous` |
| `self.m()` inside an adapter body must go through the shim (§3.2.2) | unqualified `hello()` in the adapter body binds the adapter's own override: `1` where rustc gives `1001` (reviewer probe) |
| The shipped shim inverts Rust's probe order (§3.2.3) | inherent `Dog::m` + `impl Tr for Box<Dog>`: rustc `3`; shipped shim's first arm is `deref(...).m()` → `1` |
| The shipped static route resolves an inherent shadow inside a default (§3.2.13) | `Greet_::describe` calls `self_.hello()` → inherent: `2002` where rustc gives `2`; the `dyn` route gives `2` |
| The prior design emits one body many times (§3.2.10) | one default method (`twice`) has **nine** textual definitions in the shipped output |
| The two existing attributes already emit the fast path (§3.2.12) | `namespace Animal_` ×0, adapters ×0, `struct Dog : public Animal`, `twice() override`; path syntax → `(&d)->speak()`; call-site `&dyn` coercion names the deleted adapter; generic-bound receiver leaves one error |
| `&T` / `T` receivers already collide (§3.2.14) | transpiler warns `two impls collapse to a single C++ signature`; `impl Tr for i32` + `impl Tr for &i32` → `r.m()` returns the `i32` body |
| Shipped `use`-scoping is coarse (§3.2.5) | `using namespace A_;` **and** `using namespace B_;` emitted when only `use a::A;` is present |
| Whole-program re-emission cost (B) | `.ninja_log`: full module-cache build 11.1 → 29.1 min CPU (2.6×); BMI storage 0.82 → 7.08 GB (8.7×); one-binary edit loop 0.65 s → 125.8 s (194×) |
| Header-mode ODR silence vs module-mode diagnosis | one class with different bases in two TUs: plain link and `-flto -Wodr` both exit 0 and read wrong bytes; named modules diagnose it (`found 1 base class … but in 'mcore' found 0`). Predicate divergence across importers is *not* diagnosed either way |

**Live defects found along the way** (none introduced by this design; unfiled at time of
writing). RPITIT `fn m(&self) -> impl Trait` in a trait breaks the build in the shipped default
lane (19 errors, `only virtual member functions can be marked 'override'`) and APIT `fn m(&self, s:
impl Src)` emits `virtual … (const auto& s) = 0` — both object-safety exclusions missing from the
interface skip-list; `-> Self` also reaches the interface (`virtual Consume dup() const = 0`, abstract
by value). GAT `type Item<'a>` → `redefinition of 'Lend' as different kind of symbol`. `#[cpp_inherit]`
is a silent no-op on foreign / primitive self types. Supertrait adapters are abstract. The `&dyn`
call-site coercion and generic-bound receiver bugs under `cpp_inherit` (§3.2.12). A generic trait
with two impls for one type gets adapters for the first only. `impl Tr for i32` + `impl Tr for
&i32` silently routes to one body. The multi-owner shim is guard-less first-wins and the plain shim
inverts probe order (table). The UFCS shim emits a textually duplicated, dead first branch. User
marker traits get a full polymorphic interface (`virtual ~Marker()`, deleted copy). Unrelated to
traits: `let mut sum = 0; for x in [4_000_000_000u64; 2] { sum += x }` prints `8000000000` under
rustc and `-589934592` from the shipped output.

**How the two-tier model relates to that investigation.** The investigation's verdict was "no to
*always* inherit, yes to one lane," and it identified the adapter as that lane. This revision keeps
every measurement and moves one thing: the *default*. The investigation's own steelman ("a coherent
restricted dialect where always-inherit genuinely is simpler: crate-local named structs, object-safe
methods, no blanket impls, no foreign receivers — the C++-interop-facing surface") is tier 1, and
§3.2.1 is that dialect written down as a decidable test, with its coverage measured honestly
(§3.2.12). Three of the investigation's findings become tier-1 *exclusions*: the
inherent-method-silently-overrides case, the subtrait-redeclares-name case, and `Self` in a
non-receiver position. Two are superseded: per-trait slot names were introduced for one type
implementing two same-named traits — under the two-tier model that type's impls are adapters, each
inheriting one interface, so the collision never reaches C++ and tier 1 keeps plain names, with the
call site's base-tested ladder carrying Rust's scope rule; and the CRTP defaults mixin is replaced by
explicit-object members. One is *re*-admitted by plain names and fixed at the call site: the
"trait scoping has no base-class analogue" failure, for a type with a tier-1 and a tier-2 impl sharing
a name (§3.2.6).

**Measurements added by the 2026-09-29 review** (clang 22.1.8, rustc 1.95–1.97; probes under
`review3/` and `review/tier1probe/`). Explicit-object defaults: `dog: gmap=60 via=60 | cat: gmap=6 via=6`
= rustc; the same across a precompiled module boundary (`60`); downstream subtrait `60 70 60`; the
`virtual`-caller shape `6 6 60 6` vs rustc `6 6 60 60`. Plain-name shadowing: crate B `impl Tr2 for a::Dog`,
only `Tr2` in scope, 09-23 ladder `1` vs rustc `2`; both in scope `1` vs E0034; guarded arm 1 + `has_`
count: `2` / assertion. Bound: `requires std::derived_from<T, Tr>` rejects `impl Tr for i32` (`f(&42)`:
`derived_from<int, Tr>` evaluated to false) where rustc prints `7 1042`; `has_Tr` gives `7 1042`. Generic
trait, two instantiations: `A` only in the return → `functions that differ only in their return type
cannot be overloaded`; `A`-free method → one body serves both bases (`i32-impl` where rustc gives
`u8-impl`); explicit trait arg `<T as Tr<u8>>::m(&t,5)` → `t.m(5)` picks `int32_t` (`6`) where the base
cast gives rustc's `105`. Concrete self type on a generic struct: `template<class T> struct W : Tr`
accepts `W<&str>` (rustc E0277) and compiles the `i32` body against it. Subtrait over a tier-2
supertrait impl: `1 0 0` where rustc gives `1 999 999`. `Self` in a parameter: `hides virtual member
function`, implementor abstract; rustc E0038. Deleted base copy: derived `= default` copy implicitly
deleted; protected+defaulted: derived copies work, `Tr& a = x; a = y` rejected. Synthesized lone move
ctor: `call to implicitly-deleted copy constructor`. `x.Tr::d()` = base body (`100` vs override `2`);
pure slot → undefined reference. `virtual int k() && = 0` legal, callable directly and through an owning
`dyn`. Non-virtual supertrait bases: `derived_from<D, Super>` false. Census: ~4 / ~288 pairs tier 1.

**Measurements added by the 2026-10-04 carrier probes** (clang 22.1.8, rustc 1.95–1.97; six probes, each
writing the namespace carrier and the body-carrying adapter in their *fixed* forms against a rustc oracle;
`nsvsadapter/{probe,defaults,scoping,thinhelper,probe_crosscrate,costs,hazards}/`, verdicts in
`nsvsadapter/results.txt`). Both designs matched rustc on every cell; the rows record what each must
emit or know to do so.

| Probe | Measured |
|---|---|
| **collapse-tags** — `trait Tr<A>` with `Tr<i32>` + `Tr<u8>` on one type; `impl Tr for T` + `for &T` | trailing `rusty::tag<A>` / `rusty::self_tag<Self>` ≡ the adapter's `<A, U>` / `<const T&>` key: `diff` vs rustc identical on 7/7 and 9/9 lines. **`t.m(av)` with `av: u8` from a function return: rustc `304`; the free-function overload set gives `304` with no emitter knowledge; the 09-29 §3.2.2 rule rejected the program.** Bare `t.name()` / `let c = t.conv()` with two impls in scope: rustc E0283 — no third case. Machinery: 0 explicit / marker specializations, 0 out-of-line bodies, 1 forwarder template per trait vs 4 / 5 / 6 / 3 for the adapter |
| **defaults** — inherent shadow, non-template override, primitive impl, cross-module override, `dyn` | fixed namespace form = rustc on all: `a: foo.hello()=1001 <Foo as Tr>::hello=1 describe=2 each=11 via=2 \| b: describe=777 via=6 \| i32: describe=42 via=22 \| d: 2/777/42`; two modules `c: down.via()=60 down.each(+1)=60 plain.via()=6 up.via()=6 down.describe()=10 show(down2)=777 \| plain.each(+1)=6 dyn down2.describe=777`; `impl Tr2<Local> for u8` downstream `describe2=42`. Three traps on the way: primitive impl declared *after* the default templates without a tag → `call to function 'hello' that is neither visible in the template definition nor found by argument-dependent lookup`; dyn slot as a qualified call to the default template → `dyn bar.describe=10` (rustc 777), unqualified at member scope → finds the member itself; `using namespace` defeated by inner-scope hiding (compile failure) → per-name using-declarations, retired altogether under the CPO. Downstream override placed in `Tr_` → silently `show(down2)=10`. E0117: `impl a::Tr for u8` downstream rejected by rustc. Adapter form: all cells, no variant switching |
| **scoping / E0034** — six scope cases, three E0034 cases, parent/child modules, type-param receiver | namespace form = rustc on `i=1 ii=2 iv=11 ivb=1000 v=1/2/2 vi=1/11/1000`; concrete-vs-concrete `call to 'foo' is ambiguous` for free; **concrete-vs-default (`16`) and concrete-vs-blanket (`32`) silent where rustc is E0034** — the `has_A + has_B` guard restores both (adapter ladder: assertion on both). Parent `use A` + child `use B`: ambiguous with namespace-scope directives, `child=2 parent=1` with function-scope ones — the shipped emitter emits six global directives for two traits (`mod.rs:20900/20974/21309`). Type-parameter receiver must be bound-qualified (`A_::foo(x)`): unqualified is ambiguous at `g<int>`. Cross-crate default inherited through a nested-module override: `7000` both forms. 18 vs 60 surface lines; 1–2 vs 5–7 per impl; one call vs a 6-line ladder per site |
| **dyn thin forwarders** — `&dyn` over `i32`, `W<i32>`, a blanket; defaults; supertrait override; `&mut dyn`; `Box<dyn>` | rustc `i32 m=6 twice=12 ssum=99906 s=99900 \| W m=11 twice=6006 ssum=1111 s=1100 \| Sc m=140 twice=280 ssum=147 s=7 \| mut y=10 w2=5 \| box 5/10/99905 15/6006/1115 101/202/108`; `ns_thin.cpp` identical, 0 warnings; 6 vs 40 lines per impl; stubs 3 per trait vs 9; -O2 static route byte-identical to both adapter routes. **Body kind:** `W`'s override `twice` sees the inherent `m` (`6006`), the default `ssum` sees the trait's (`1111`). **Module boundary:** a qualified call in a template sees only earlier overloads (`qlookup` prints `1`); `Tr_::twice<BT>` loud `no viable conversion`; supertrait-only default **silently `sonly=11000`** (rustc 5550). Fixed two ways: ADL tags with unqualified calls (`ns_thin_adl.cpp`, `W` prototypes *after* the defaults, identical to oracle) or per-impl materialization; the adapter form passes natively through the vtable |
| **cross-crate placement** — 4 modules, local-self in a nested module, foreign-self, blanket on `Box<Local>`, hazards | base program identical to rustc in all three forms (`foo.m=13 foo.describe=501 foo.hello=2005 … bar.ext=41 only_ext2=2040`). Receiver-namespace placement: the body with `Helper` / `private_::unit_only` compiles **in place** (zero relocation, `private_` shadowing correct, `13`); Fix B gone for local-self (132/249 serde_core, 69/72 serde), kept for foreign-self in nested modules (117/249; `use of undeclared identifier 'Helper'` otherwise), 4 lines/impl vs 8 + 3/trait. **H1** two crates adding `a::ext(const Bar&)`: `declaration 'ext' attached to named module 't' cannot be attached to other modules`. **H2** local `int32_t m` in the caller: `called object type 'int32_t' is not a function`. **H3** two traits, one method name, one type: `redefinition`. **H13** downstream trait's `m` on the same type: `ambiguous` where rustc `3005`. H2/H3/H13 fixed by the tag form (`ns_tag/`, identical to rustc); H1 is the placement rule |
| **machinery census** — one trait, four impls incl. a blanket and a tier-1 struct, every use site | identical to rustc in all three forms (`a_ns`, `a_pure`, `b_adapter`), 0 diagnostics at `-O2 -Wall -Wextra`; static-call asm identical. Totals 158 vs 197 lines; trait machinery 95 vs 117; **per tier-2 impl 6–8 vs 28–34**; call-site region 47 vs 64; 2 vs 5 call-site shapes. Per *trait* the adapter is smaller (17 vs 63 lines). The namespace form works **only** as: a CPO per method (a plain qualified `Shape_::area(s)` in a generic caller cannot see a later impl — compile error), SFINAE-friendly CPOs (else `has_Shape` hard-errors), impl functions in `impl_` not `Tr_` (a bare `area` inside `Shape_` names the CPO variable and suppresses ADL), tag parameter 0. Concept memoization silent (`early=0 late=0`) vs defined marker loud |
| **fn-template CPO + ABI companion** (2026-10-07, `fncpo/fn_cpo.cpp`) — the census program with every `Shape_::m` a constrained function template instead of an object, and a non-template `Shape_::area(const Sq&)` beside it | identical to the rustc oracle, 0 diagnostics at `-O2 -Wall -Wextra`; `Shape_::area(sq)` binds the companion (non-template ▷ template), generic `f(sq)` likewise, every tier-2 and `dyn` cell unchanged. The variant phase 2 emits (§3.2.2 rule 1) |

**Unmeasured in both forms** (by construction only): a tier-1 and a tier-2 impl of same-named traits on
one type reached through the bridge; `Box<i32>` autoderef through `deref_call` into a CPO; by-value `self` through a CPO with the `Copy`-copy rule, and the by-value bridge `consume(tag, S s)` into an `&&` slot (the §3.2.2 example writes it; no probe compiled it); generic *required* methods through a CPO; associated
types in a namespace default body; marker reachability under the namespace form across a module graph
(taken from the 09-29 row); the §3.2.3 ladder as emitted (every probe hand-resolved inherent vs trait).

#### 3.2.16 Migration plan and open review items

**Execution order (2026-10-07).** Phases are executed **2 → 0 → 1 → 3**, not in the numbered order.
Decision (e) ("phase 1 before phase 2") was carried from 09-23, when phase 2 *replaced* the lane with no
measuring corpus; phase 2 now *revises the lane every matrix crate uses*, each of its steps has a probe
cell that flips from FAIL to PASS (`tests/transpile_tests/trait_probes`, §3.2.12), and the §3.2.12 table is
already its step list — whereas phases 0–1 change the emitted shape of every local-struct trait impl in
the unit corpus for a census of 4 pairs in the matrix. Within phase 2 the steps run in the order that
keeps each push gate-green, not the table's order; the first is the §3.2.6 guard (self-contained, loud
on failure), and the `using namespace Tr_;` deletion waits until every call site is measured to spell
`Tr_::m` (default bodies and the shipped shim's member branches may still lean on the directive).

**Phase 0 — prerequisites in the tier-1 lane** (every item is a measured defect, §3.2.12): multiple and
virtual bases; `&&` slots for `self` receivers and move/copy insertion at their call sites; generic
defaults and their transitive callers as explicit-object members; supertrait-calling defaults kept as
virtual bodies (and the `operator-` mis-emission); the interface skip-list extended to `Self` in
parameters, RPITIT, APIT, `-> Self`, `async fn`, GAT, and `where Self: Sized` — deciding *tier*, not
only slot emission; assoc-const traits given an interface; constructors for tuple and unit structs;
`clone()` and every literal through the fieldwise constructor; no synthesized lone move constructor, all
four special members defaulted for `Copy`/`Clone` implementors; interface special members protected and
defaulted; `Box<dyn>` construction through `Box`'s converting constructor; the call-site `&dyn` coercion;
path syntax decided per `(trait, declared receiver)`, not per trait; `cpp_inherit` on a foreign or
concrete-on-generic self type a *diagnosed* no-op; interface hoisted before any implementor; the
anonymous-namespace wrap only for non-`pub` traits with non-`pub` implementors; `rusty::unreachable_via_const_dyn`
added to `include/rusty`.

**Phase 1 — tier 1 becomes the default.** A program-wide impl pre-pass (the shape of
`set_cross_file_traits`, over every impl block in the dependency graph) computes what §3.2.1 needs and
the collect pass lacks: blanket and conditional presence per `(trait, method name)`; per-type
implemented-trait sets and same-name collisions over concrete impls; inherent-vs-tier-1 name overlap;
self-type arguments against `declared_type_params`; alias resolution; supertrait lists (local, and from
the manifest); generic-default and `Self: Sized` flags per method; `repr`. Every `(trait, impl)` passing
§3.2.1 is emitted tier 1 with no attribute; everything else keeps the shipped free-function lane.
`cpp_trait_member_dispatch` / `cpp_inherit` become *force* attributes that may override only the
coverage-motivated tests (decision (w)), never the semantic ones; an opt-out attribute per impl. **The
manifest gains its tier-1 fields in this phase** (§3.2.14), because `impl DepTrait for LocalType` needs
the dependency's tier the moment a second crate exists. Touch list: `predicates.rs` (marker recognition),
`collect_passes.rs` (the two `cpp_inherit` gates and the pre-pass), `transpile.rs`
(`classify_method_names_excluding_traits`, the manifest), `mod.rs` (the tier decision ahead of the
classification at the owner-collection site; five call-site gates), `emit_items.rs` (the interface),
`emit_expr.rs` (call sites, path syntax, the coercion), `type_mapping.rs`, `main.rs` (crate-mode wiring)
— eight files. Revertable by deleting the predicate. Gate: the parity matrix plus a **tier census** in
each crate's log (how many pairs went tier 1, and which §3.2.1 test excluded each of the rest).

**Phase 2 — tier 2: revise the shipped free-function lane in place**, behind a per-crate switch, in the
order of the §3.2.12 table — each step independently revertable and gated by the parity matrix: (1)
delete the three `using namespace Tr_;` emission sites and spell `Tr_::m` at every classified call from
the resolved owner; (2) the `impl_` split, the tag parameter, the CPO per method, `impls_Tr` +
`has_Tr`, `requires has_Tr<S>` on default templates (retiring `__ufcs_impls`); (3) default bodies
lowered with the CPO for `self.m()`, the body-kind flag; (4) the three generic forwarders per trait
replacing per-impl adapters, slots through the CPO, supertrait slots enumerated; (5) the placement rule
— local-self in place, foreign-self in `Tr_::impl_`, `rusty_ext` re-pointed; (6) the §3.2.6 guard
replacing `emit_multi_owner_ufcs_call`'s first-wins ladder, and bound-qualified type-parameter
receivers; (7) `rusty::tag<A>` and `rusty::self_tag<Self>`; (8) the non-vtable members. Gates before
any crate flips: the non-vtable members exist (else serde has no carrier), assoc-const traits have their
namespace form, the expression-level `StructBorrow` for the `&dyn` argument coercion is added and pinned
((k)), the negative test for the unreachable stubs, and the §3.2.3 ladder measured *as emitted*.
Migration hazards from the 09-23 revision stand: the deprecated no-op `--interface-traits` flag (revive
as the switch or delete) and the eight vendored-port `post_transpile_patch.py` scripts whose `Tr_::` /
`rusty_ext::` anchors patch nothing silently — step (2) changes the spelling of every `Tr_::m`
definition to `impl_::m(tag, …)`, so all eight go stale in step (2), and phase 1 already re-emits
`alloc`, `std_port` and possibly `hashbrown` impls on local structs.

**Phase 3 — delete what the revision orphaned**, in independently revertable steps: the `rusty_ext`
namespace, its fallback probe and retargets, including the hand-written `namespace ser::impls::rusty_ext`
block in `include/rusty/rusty.hpp`; the `__ufcs_impls` markers and their `requires` clauses; the per-impl
forwarding-adapter emitters; the manifest's retired fields, `version` 2, and a consumer hard-error on
mismatch (the loader today reads no version and skips unparseable files).

**Decisions for review.** Letters (a)–(o) keep their 2026-09-23 numbering so kept text can cite them;
(a), (b), (d), (f), (g), (i), (j), (k) are re-stated for the 2026-10-04 carrier.

- **(a) Fix B shape** — tier 2 only: retired for local-self impls (emitted in the self type's declaring
  namespace), kept as the shipped helper-namespace + using-declaration bridge for foreign-self impls in
  nested modules (§3.2.5).
- **(b) Cross-module reachability** — impl functions reached by tag-ADL at the point of instantiation
  (measured across named modules); marker specializations conforming as 09-23, interface units only; tier 2
  only.
- **(c) The `static_assert(n ≤ 1)` guard** — a hard requirement; it now counts with `has_` and follows a
  base-excluded inherent arm (§3.2.6).
- **(d) The unreachable stubs on the `Ref` / `RefMut` forwarders** (§3.2.10) vs. splitting the interface.
  Recommendation: the stubs — three lines per *trait* now, not per impl — with the negative test, and **one
  interface per trait in both tiers** (the 09-23 `Tr` / `TrMut` split retired for tier-2 traits too).
  Enforcement is runtime.
- **(e) Phase 1 before phase 2** — unchanged.
- **(f) `&T` / `T` receiver collapse** — tier 2; `rusty::self_tag<Self>` fixes it (measured 9/9). Not blocking.
- **(g) View-typed self impls** — tier 2; the view conversion in overload resolution (`span` / `string_view`),
  exact match winning, and its dedicated test (§3.2.4).
- **(h) Naming** — `cpp_trait_member_dispatch` / `cpp_inherit` become *force* attributes (see (w)); decide
  their names and the opt-out's.
- **(i) Scope-precise candidates** — tier-2 requirement, now carried by the emitter's spelling `Tr_::m` and
  the §3.2.6 ladder; no `using` of any kind; the inherent arm's exclusion list is scope-*independent* (§3.2.6).
- **(j) Emission ordering** — tier 2: the forward-declaration pass (all impl declarations before any default
  template) within a TU; tag-ADL across modules. Tier 1: the interface must precede its implementors.
- **(k) The expression-level `StructBorrow`** — tier 2, for the `&dyn` coercion in argument position only
  (the static route constructs nothing); tier 1's `dyn` is an ordinary reference.
- **(l) Bounds** — `T: Tr` → `requires has_Tr<T>` for **every** trait. `std::derived_from<T, Tr>` is
  never emitted: it rejects every tier-2 implementor of a tier-1 trait (measured), and the trait's
  crate cannot know whether one exists. It survives as the reverse-direction reading (§3.2.17).
- **(m) `self: Box<Self>` / `Rc<Self>` / `Pin<P>` receivers** — open, tier 2.
- **(n) Interop naming** — *resolved by the model*: tier-1 interfaces have plain names; hand-written C++
  overrides `m`; a hand-written `t.m()` always gets the tier-1 body, whatever Rust trait a Rust author
  had in scope — the price of a plain-named member (§3.2.17).
- **(o) Book sweep outside §3.2** — unchanged list, plus §12 and the C++ module index (§3.2.17's reverse
  half is new work there), and, of the 76 code comments citing §3.2.N, the 25 citing §3.2.2/3/5/10/13 — the sections whose
  mechanism this revision changed — which must be re-read against it (the other 51 cite §3.2.7, §3.2.9 and
  §3.2.4, whose mechanisms did not move).
- **(p) The tier census as the gate metric.** Today ~1.4% of the matrix corpus. Every widening below is
  judged by what it adds to that number versus what it costs the contract.
- **(q) Associated types in tier 1** as interface template parameters — recommendation: yes, with the
  reverse-direction annotation (§3.2.17, lossy cell 1).
- **(r) `repr(C)` / `repr(transparent)` types** — recommendation: exclude; a vptr breaks the layout the
  attribute exists for. Likewise types whose `size_of` the crate asserts.
- **(s) By-value `self` receivers** — `virtual R k() && = 0` is legal and maps under
  `docs/method_qualifiers.md` (measured); the call site inserts `std::move` — or a copy for `Copy` self
  types, since `std::move(d).k()` would let the body mutate the original Rust copies. Recommendation:
  tier 1 with that rule; Rust forbids calling it on `dyn`, C++ allows `std::move(*box).k()` — forward-only.
- **(t) Associated consts in tier 1** as `static constexpr` on the implementor plus a concept conjunct —
  recommendation: yes; note the trait is then not `dyn`-usable in Rust.
- **(u) Conditional impls as `std::conditional_t` bases** — a widening candidate: `template<class T>
  struct W : Tr, std::conditional_t<has_B<T>, Other, NoBase<Other>>` is legal and behaves as Rust does
  (measured), but the implementor cannot write `override` (the member is plain when the base is absent),
  each conditional impl needs a distinct empty tag base, and the reverse direction must recognize the
  pattern. Recommendation: stay tier 2 for now; revisit under (p).
- **(v) Frozen interop traits** — *retired*: with explicit-object defaults and call-site base tests, no
  downstream impl changes an upstream interface, so nothing needs freezing.
- **(w) Force attributes** — define what they may override: only the coverage-motivated tests (`repr`,
  the at-most-one-instantiation rule when every method is parameter-distinguished, the supertrait-impl
  rule when the author asserts the supertrait impl is concrete elsewhere), never the semantic ones (an
  inherent/trait name overlap forced to tier 1 is the silent-override case).

- **(x) Tier-2 carrier (adopted 2026-10-04).** A namespace of free functions + tag-anchored ADL +
  per-method CPOs, with a thin virtual helper for `dyn`; the body-carrying adapter retired to the §3.2.15
  record. Adopted on the six-probe comparison; the open items are the seven rules of §3.2.2 as *emitter
  invariants*, each silent if violated, and the unmeasured list in §3.2.15.
- **(y) The predicate.** `has_Tr = impls_Tr<U>::value ∨ derived_from<U, Tr>` with the defined-false
  marker (loud; one line per impl; required for all-default traits and for generic required methods) vs. a
  concept through the CPOs (zero per-impl lines; silent memoization). Recommendation: the marker as the
  predicate; a CPO-`requires` conjunct admissible for non-generic required methods.
- **(z) The tag/CPO protocol as the C++-interop surface for tier 2.** A C++ author implements a Rust
  trait for a type they cannot or will not modify by writing one free function per method — `R
  m(Tr_::impl_::tag, const Mine& self_, …)` in `Tr_::impl_` or in `Mine`'s namespace — plus `template<>
  struct impls_Tr<Mine> : std::true_type {}`; every Rust call site and every `dyn` forwarder reaches it
  with no further declaration. This is the non-intrusive half of the interop story §3.2.17 gives the
  intrusive half of, and it falls out of the carrier. Recommendation: document it as supported once
  phase 2 lands; the reverse direction (Rust consuming a C++ free-function impl) needs an index entry
  kind like §3.2.17's `interface`.

#### 3.2.17 The tier-1 contract: a bijection between Rust traits and C++ interfaces, in all but two cells

This section is the definition tier 1 is held to. Each row is a two-way correspondence: the transpiler
emits the right column from the left, and a C++ author who writes the right column has written
something a Rust crate can consume as the left. **The reverse half is new work**, not an existing path:
§12's inline-Rust profile excludes trait definitions, impls and `dyn`; the C++ module index (§3.13) has
callable and opaque-type kinds but no *interface* kind. Consuming a hand-written interface from Rust
needs an index entry `kind = "interface"` with per-method `{receiver kind, signature, pure | default}`,
a Rust-side trait stub generated from it, lowering of `impl cpp::Iface for Local` to inheritance and
`override` with qualifiers from the index, and `&dyn cpp::Iface` → `const ns::Iface&`; an interface with
associated types is not reverse-consumable without parsing C++. Receiver qualifiers are those of
`docs/method_qualifiers.md`.

| Rust | C++ | notes |
|---|---|---|
| `trait Tr { … }` | `class Tr { public: virtual ~Tr() noexcept(false) {} …; protected: Tr() = default; Tr(const Tr&) = default; Tr& operator=(const Tr&) = default; /* move likewise */ }` | destructor `noexcept(false)`: a Rust `Drop` may unwind (the shipped form is load-bearing; the reverse reads `= default` too); special members **protected and defaulted** (C.67), never deleted (measured) |
| `fn m(&self) -> R;` / `fn m(&mut self) -> R;` | `virtual R m() const = 0;` / `virtual R m() = 0;` | |
| `fn m(self) -> R;` | `virtual R m() && = 0;` | call sites `std::move(x).m()`, or a copy for a `Copy` self type; decision (s) |
| `fn m(&self) -> R { body }` (calls only slots) | `virtual R m() const { body }` | default ↔ non-pure |
| `fn m<F: ..>(&self, f: F) -> R where Self: Sized { body }` | `template<class F> R m(this auto const& self, F f) { body }` | explicit-object member template; overridable by a hiding member on the implementor in any crate; the `where Self: Sized` is the Rust spelling of "not a slot" |
| `fn m(&self) -> R where Self: Sized { body }` (calls a generic default, transitively) | `R m(this auto const& self) { body }` | explicit-object; reverse: an explicit-object member with no other template parameter is a non-generic default |
| `fn m(&self) -> Self where Self: Sized { body }` | `auto m(this auto const& self) { body }` | deduced return is how `Self` gets a C++ spelling |
| required `fn m(&self, o: &Self) -> R where Self: Sized;` / `fn new(a: A) -> Self;` / `const K: T;` | `R m(const T& o) const;` / `static T new_(A a);` / `static constexpr T K;` on each implementor, plus a conjunct of `has_Tr` (`requires(const T& a, A x) { a.m(a); T::new_(x); T::K; }`) | not a slot in either language; the trait is then not `dyn`-usable in Rust (its `dyn` row is vacuous — C++ still accepts `const Tr&`, forward-only); decision (t) |
| `trait Sub: Super` | `class Sub : public virtual Super` | virtual base ↔ supertrait; diamonds legal on both sides; the reverse also accepts a non-virtual `public Super` for a single-supertrait chain |
| `trait Tr<A, B>` | `template<class A, class B> class Tr` | |
| `trait Tr { type Item; }` | `template<class Item> class Tr` | **lossy cell 1**: the reverse maps a template parameter to a generic parameter unless the C++ carries `// @assoc Item`; decision (q) |
| `impl Tr for T` (`T` declared here, non-generic) | `struct T : public Tr { R m() const override; … }` | plus the fieldwise constructor and, for `Copy`/`Clone`, all four defaulted special members (§3.2.2); a non-`Clone` implementor is still copyable in C++ — forward-only |
| `impl<X> Tr for W<X>` (`struct W<X>`, exactly its own parameters, unbounded beyond the struct's) | `template<class X> struct W : public Tr` | the reverse of `template<class X> struct W : Tr` is always this row; `impl Tr for W<i32>` and any partial / nested / repeated / extra-bounded list are tier 2 |
| `impl Tr<X> for T` (one instantiation per type) | `struct T : Tr<X>` | two instantiations on one type are tier 2 for that type |
| `x.m()` | `x.m()` | holds when exactly one `m` reaches `T` from all sources; a hand-written `t.m()` always gets the tier-1 body whatever Rust trait a Rust author had in scope — the price of a plain-named member, and why a tier-1 × tier-2 same-name pair routes through the ladder |
| `Tr::m(&x)` / `<T as Tr>::m(&x)` | `x.m()` | the same virtual call; `x.Tr::m()` is never emitted (measured: base body / undefined reference). The sole exception is an explicit trait argument on a generic trait, `static_cast<const Tr<A>&>(x).m(..)` (§3.2.3) |
| `&dyn Tr` / `&mut dyn Tr` / `Box<dyn Tr>` | `const Tr&` / `Tr&` / `rusty::Box<Tr>` | |
| `&dyn Sub` → `&dyn Super` | implicit base conversion | |
| `fn f<T: Tr>(x: &T)` | forward: `template<class T> requires has_Tr<T> R f(const T& x)`; reverse: `requires std::derived_from<T, Tr>` *or* `has_Tr<T>` read as `T: Tr` | **lossy cell 2**: forward must emit the concept because the bound must also admit tier-2 implementors the trait's crate cannot see (`impl Tr for i32`; `impl<T: Tr + ?Sized> Tr for Box<T>` — measured); the body's `x.m()` is the CPO call `Tr_::m(x)` or the §3.2.3 shim; decision (l) |
| `impl Tr for i32`, `for &T`, `for [T]`, `for Foreign`, for a closure; blanket; conditional; concrete-on-generic | — | tier 2; not in the contract |
| generic *required* method; `Self` in a parameter or nested return without `where Self: Sized`; `-> impl Trait`; `async fn` | — | the trait is tier 2; not in the contract. C++ has no `Self`: the interface can only spell `const Tr&`, and the implementor's `const D&` hides rather than overrides (measured) |
| a subtrait redeclaring a supertrait's method; an inherent method named like a tier-1 trait method; two concrete same-named tier-1 impls on one type; two instantiations of a generic trait on one type | — | tier 2 (the last three per type); not in the contract |

**C++-side grammar.** A hand-written class is in the contract when: it has no data members and no
non-public members other than the protected default constructor and the protected defaulted copy/move;
every member function is one of — `virtual`, pure or with a body, with `const`, nothing, or `&&` as its
only qualifier; a non-virtual member template; a non-virtual explicit-object member whose object
parameter is `auto const&`, `auto&` or `auto&&`; — `final`, `noexcept`, `[[nodiscard]]` and other
attributes are ignored; no two member functions share a name (Rust has no overloading — an overload set
has no trait image and is rejected); no default arguments; no `static` members on the interface (they
belong on implementors); a `virtual` member with a body calls no non-virtual member template or
explicit-object member (a hand-written interface that does so has the hidden-override defect of
§3.2.13 and is outside the contract); base classes are other interfaces in the contract, inherited
`public virtual` (or `public` for a single chain), and **an interface declares no name a base
interface declares**; the NVI idiom (public non-virtual calling private virtuals) is outside the
contract. An implementor is in the contract when it publicly inherits interfaces in the contract,
overrides each pure virtual with a matching qualifier, may hide an explicit-object or template member
(that is the override), declares no other member with the name of an inherited virtual, and **inherits
no two interfaces that declare the same name**.

**Measured (clang 22.1.8; `review/tier1probe/`, `review3/`).** A base that deletes copy makes a derived
`Dog(const Dog&) = default` implicitly deleted; protected-and-defaulted base members give the derived
type working copy and move and reject `Tr& a = x; a = y;` with `'operator=' is a protected member`. A
synthesized lone move constructor deletes the copy constructor regardless of the base. `x.Tr::d()` on an
overridden default returns the base body (`100`; the override gives `2`); `x.Tr::m()` on a pure slot is
an undefined reference. `virtual int k() && = 0;` is legal, overridable, callable directly and through an
owning `dyn`. `requires std::derived_from<T, Tr>` accepts a direct implementor and rejects a type with a
merely same-named member — but also rejects every tier-2 implementor (`impl Tr for i32`, `f(&42)`:
`derived_from<int, Tr>` evaluated to false; rustc `7 1042`); `has_Tr` gives `7 1042`. Explicit-object
defaults reach a downstream hiding override across a precompiled module boundary (`60` = rustc).
`struct T : Tr<int32_t>, Tr<uint8_t>` with `A` only in the return type: `virtual function 'conv' has a
different return type than the function it overrides`; with an `A`-free method: one body for both bases.
`template<class T> struct W : Tr` for `impl Tr for W<i32>`: `W<&str>` accepted (rustc E0277) and the `i32`
body compiled against it. A `Self`-typed parameter: `hides virtual member function`, implementor abstract
(rustc E0038).

**What the contract buys.** In the forward direction, a crate that stays inside tier 1 transpiles to
classes, inheritance, virtual calls, and — per trait — one named concept and its marker primary, which
is what a C++ author writes to constrain a template on an interface; the CPO call `Tr_::m(x)` appears only inside
generic functions bounded on a trait. In the reverse direction, once the index work above exists, a C++
library that exposes its abstractions as interfaces in this grammar can be consumed from Rust as traits
and implemented from Rust by inheritance, without adapters and without the borrow checker learning
anything new — the interop case `#[cpp_inherit]` was created for, now stated as the default rather than
the exception, and with its price stated: a plain-named member is the tier-1 body to every C++ caller.

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
rusty::ptr::write(rusty::ptr::add(ptr, i), value);
rusty::ptr::copy_nonoverlapping(src, dst, n);
```

This keeps generated C++ valid when Rust raw-pointer method shapes do not map directly to well-typed C++ member calls. The intrinsic runtime that backs these calls is described in **The `rusty::ptr` Intrinsic Runtime** below.

#### Design Decision

The transpiler emits `// @unsafe` (not just `// unsafe`) so that the rusty-cpp analyzer can enforce safety boundaries on the transpiled output. This means:

1. Transpiled safe code is checked by the analyzer (borrow rules, pointer safety)
2. Transpiled unsafe blocks are skipped by the analyzer (programmer responsibility)
3. The safety boundary is preserved across the transpilation — Rust's `unsafe` maps exactly to rusty-cpp's `@unsafe`

This is consistent with the forward correctness guarantee: if Rust's borrow checker approved the safe code, the transpiled C++ should also pass the rusty-cpp analyzer's checks.

#### Two Halves of the Problem

Translating unsafe Rust splits into two largely independent obligations:

1. **Boundary preservation** — keep Rust's `unsafe` regions marked so the rusty-cpp analyzer can keep enforcing safety on the *safe* code around them. This is the `unsafe { }` → `// @unsafe` mapping above, and it is purely a *comment*: the generated C++ carries no runtime or compile-time enforcement from the marker (see *What the `@unsafe` Comment Does — and Doesn't — Mean*).
2. **Operation lowering** — turn each unsafe *operation* (raw-pointer arithmetic, `read`/`write`, `transmute`, pointer casts, byte-string decay, FFI types) into *well-typed, compilable* C++. C++ raw pointers have no methods and Rust's pointer intrinsics have no direct C++ spelling, so most of the work lives here.

The rest of this section is about the second half. It is the part that decides whether a raw-pointer-heavy crate compiles at all.

#### The `rusty::ptr` Intrinsic Runtime

Rust calls pointer operations as *methods* (`ptr.add(i)`, `ptr.write(v)`, `ptr.offset_from(other)`) or as `core::ptr::*` free functions. Neither has a direct C++ form — a C++ `T*` has no member functions. The transpiler lowers both shapes onto a free-function runtime in `include/rusty/ptr.hpp` under namespace `rusty::ptr`:

| Family | Members (`rusty::ptr::…`) |
|--------|---------------------------|
| Arithmetic | `add`, `sub`, `offset`, `wrapping_offset` (→`offset`), `offset_from` |
| Read/write/copy | `read`, `read_unaligned`, `write`, `write_unaligned`, `write_bytes`, `copy`, `copy_nonoverlapping`, `swap`, `swap_nonoverlapping`, `replace` |
| Construction / null | `null`, `null_mut`, `null<T>()`, `null_mut<T>()`, `without_provenance[_mut]` |
| Cast / view | `cast_mut`, `cast_const`, `as_ref`/`as_mut` (→ `Option<T&>`), `addr_of[_mut]` |
| Lifetime | `drop_in_place` (scalar + range overloads) |
| Wrapper types | `NonNull<T>`, `Unique<T>` (= `NonNull<T>`), `Alignment` |

Two lowering paths feed this runtime:

- **Method calls** on a raw-pointer receiver are rewritten in `emit_expr.rs`, *gated* by the predicate `is_expr_raw_pointer_like` (`predicates.rs`). That gate is the linchpin: it must recognise the receiver as a pointer (a `*mut`/`*const` local, a pointer-typed struct field, `as_ptr()`/`as_mut_ptr()`, `UnsafeCell::get`, `AtomicPtr::load`, or a pointer-arithmetic chain). When the gate *fails*, the method falls through to a C++ member call `(*p).add(i)` on a non-struct and the file does not compile — so most raw-pointer regressions trace back to an inference gap in this predicate rather than to the lowering itself.
- **Free-function paths** `std::ptr::*` / `core::ptr::*` are rewritten to `rusty::ptr::*` in `transpiler/src/types.rs`; `slice::from_raw_parts[_mut]` maps to `rusty::from_raw_parts[_mut]`.

```rust
let len = end.offset_from(start);        // *const u8 method
unsafe { dst.add(i).write(byte); }       // chained pointer method
let s = core::slice::from_raw_parts(p, n);
```

```cpp
const auto len = rusty::ptr::offset_from(end, start);
rusty::ptr::write(rusty::ptr::add(dst, i), byte);
const auto s = rusty::from_raw_parts(p, n);
```

#### Casts, `transmute`, and the Cast Matrix

`as` casts that touch pointers do **not** all become `static_cast`. C++ forbids `static_cast` between unrelated pointer types and between pointers and integers, so the transpiler selects a cast kind per source/target shape (the matrix lives in `mod.rs`’s cast emitter):

| Rust cast | C++ emitted |
|-----------|-------------|
| `p as *const U` / `*mut U` (ptr→ptr) | `reinterpret_cast<U*>(p)` |
| `p as *mut U` from a `const` source | `const_cast<U*>(reinterpret_cast<const U*>(p))` |
| `p as *mut *mut U` (pointer **depth** change) | `(U**)(p)` (C-style — the const_cast/reinterpret dance cannot bridge a level) |
| `n as *const T` (int→ptr) | `reinterpret_cast<const T*>(static_cast<std::uintptr_t>(n))` |
| `p as usize` (ptr→int) | `static_cast<size_t>(reinterpret_cast<std::uintptr_t>(p))` |
| `&x as *const T` (ref→ptr) | `static_cast<const T*>(&x)`, or `reinterpret_cast<…>(rusty::addr_of_temp(…))` for temporaries |
| `b"…" as *const u8` (byte-string→ptr) | function-local-static decay (below) |

`mem::transmute::<Src, Dst>(x)` lowers to `rusty::mem::transmute<Src, Dst>(x)` (`emit_expr.rs`), backed by a size-checked `memcpy` into aligned storage plus `std::launder` in `include/rusty/mem.hpp` — Rust’s “reinterpret the bits” with a static `sizeof` assertion.

**Byte strings.** A `b"…"` literal is a value `std::array<uint8_t, N>{{…}}`. But in Rust `b"…"` has type `&'static [u8; N]`, so casting it to a pointer must decay to *static storage*, not route the array value through `static_cast<uintptr_t>` (ill-formed — an array is not an integer). The transpiler emits a function-local static:

```rust
let tag = b"tag:yaml.org,2002:str\0" as *const u8 as *mut yaml_char_t;
```
```cpp
auto tag = const_cast<yaml_char_t*>(reinterpret_cast<const yaml_char_t*>(
    ([]() -> const uint8_t* { static const uint8_t _bs[] = { 0x74, /*…*/ }; return _bs; }())));
```

#### FFI and `c_void`

`core::ffi::c_void` / `std::os::raw::c_void` lower to `rusty::ffi::c_void`, which `include/rusty/ffi.hpp` defines as `using c_void = void;` — so `*mut c_void` → `void*` and `*const c_void` → `const void*`. (`c_void` is only ever used behind a raw pointer, so aliasing it to `void` is exactly right.) The neighbouring C-string/OS-string family — `CStr`, `CString`, `OsStr`, `OsString` — maps to `rusty::ffi::*` in `transpiler/src/types.rs`. Note this is the *C ABI* surface; the separate `use cpp::…` C++-module interop and its safety contract are covered in §3.13.

#### What the `@unsafe` Comment Does — and Doesn't — Mean

The emitted `// @unsafe` is consumed only by the rusty-cpp **analyzer**, as a directive to *skip* safety checking inside the region. It is erased from the program semantics: the generated C++ for an `unsafe { … }` block is just `{ … }` with a comment, and an `unsafe fn` is an ordinary function with a comment above it. There is no C++-level re-verification of the unsafe operations.

This is deliberate and follows the forward-correctness guarantee: the input already type-checked and borrow-checked as Rust, so the transpiler’s job for unsafe regions is *faithful lowering*, not re-litigating safety. The analyzer still enforces the *safe* code that surrounds them.

#### Case Study: Porting c2rust Output (`unsafe-libyaml`)

The hardest stress test for this machinery is not hand-written Rust but **machine-generated** Rust: a [c2rust](https://github.com/immunant/c2rust) transliteration of a C library, which is *wall-to-wall* raw pointers, unions, pointer arithmetic, and byte-string casts, with no idiomatic Rust to lean on. We use **`unsafe-libyaml` 0.2.11** (dtolnay’s c2rust port of libyaml) as the benchmark. Crucially it is a *pure-Rust* port (no `extern "C"`, no FFI) — the goal is to *translate* it, exercising the unsafe-lowering paths end to end, not to bind to C.

The methodology that works here is **systemic-gap reduction**, not bug-by-bug fixing:

1. Transpilation itself essentially never fails (no panic) — the AST lowers fine.
2. The thousands of resulting C++ *compile* errors collapse into a *handful* of systemic gaps, each worth hundreds of errors.
3. Fix one gap at the root, re-transpile the whole crate, and *measure* the error delta. This is reliable in a way that guessing from transpiled output is not (synthetic mini-repros consistently diverge from the real shapes — always measure against the real crate build).

Representative systemic fixes, in error-impact order:

| Gap | Root-cause fix | C++ errors removed |
|-----|----------------|--------------------|
| `rusty::ffi::c_void` undefined | add `using c_void = void;` to `ffi.hpp` | ~797 |
| `ptr.field` chains untyped | infer struct-field-access types (the “keystone”) | ~1002 |
| renamed primitive re-export (`u8 as yaml_char_t`) | collapse the alias target to `uint8_t` | ~389 |
| Deref-coercion field access via user `impl Deref` | track Deref targets, emit `(*base).field` | ~222 |
| `b"…" as *ptr` → `static_cast<uintptr_t>(array)` | function-local-static decay | ~195 |
| `null_mut::<T>()` not a template | add `template<typename T> T* null_mut()` overload | ~138 |
| un-annotated `let x = …` left `x` untyped | record the inferred initializer type | ~115 |
| `c_offset_from` auto-deref’d the receiver | dedicated lowering to `rusty::ptr::offset_from` | ~73 |

Across these (and a few more), the benchmark moved from **3471 → ~327** C++ compile errors (≈ −91%) with the transpiler unit suite green throughout. What remains is a *diffuse* long tail rather than another systemic gap: bare C-like enum-variant qualification (`use Enum::*` variants emitted under their flat-alias name), per-function inconsistency in extern-declaration type qualification, and c2rust anonymous-union types. The detailed remaining-error breakdown is tracked in the project memory (`unsafe-rust-translation-roadmap`) and the parity log (§10.6).

Two lessons generalise beyond libyaml: (a) the biggest leverage on raw-pointer code is **type inference**, not the lowerings themselves — when `is_expr_raw_pointer_like` / field inference resolves a receiver, the existing lowering just fires; and (b) for generated-code ports, **always measure against the real crate build**, because hand-made repros drift from the real expression shapes.

### 3.8 Async/Await → Pollable State Machine on C++20 Coroutines

Start from Rust semantics, then map to C++:

#### Rust Async Model First

Rust async is trait-based and poll-driven:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

Key properties:

- `async fn` does not run immediately. It returns a lazy future value.
- The compiler lowers `async fn` into a stackless state machine that captures locals across suspension points.
- Each call to `poll(...)` advances that state machine until:
  - it reaches another suspension point (`Poll::Pending`), or
  - it finishes (`Poll::Ready(output)`).
- For `Future`, `Poll::Ready` is terminal (complete). `Poll::Pending` is not complete.
- Progress after `Pending` depends on the `Waker` in `Context`; the runtime/executor re-polls when wake is signaled.

In short: Rust async is "lazy state machine + explicit polling contract", not "spawned thread" and not eager execution.

With that contract as the source of truth, C++20 coroutines are used as the codegen mechanism for the same state-machine shape.

#### Core Types

```cpp
#include <coroutine>
#include <functional>
#include <utility>

// Poll<T> — Rust's Poll enum
template<typename T>
struct Poll {
    bool ready;
    T value;

    static Poll ready_with(T v) { return Poll{true, std::move(v)}; }
    static Poll pending() { return Poll{false, T{}}; }
    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
};

template<>
struct Poll<void> {
    bool ready;
    static Poll ready_with() { return Poll{true}; }
    static Poll pending() { return Poll{false}; }
    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
};

// Waker — notification callback for IO readiness
struct Waker {
    std::function<void()> wake_fn;
    void wake() const { if (wake_fn) wake_fn(); }
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
        T result{};
        Context* current_ctx = nullptr;
        std::coroutine_handle<> continuation{};

        Task get_return_object() {
            return Task{std::coroutine_handle<promise_type>::from_promise(*this)};
        }

        // KEY: suspend_always makes it LAZY — nothing runs until poll()
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }

        void return_value(T value) { result = std::move(value); }
        void unhandled_exception() { std::terminate(); }
    };

    // poll() — drives the state machine one step (like Rust's Future::poll)
    Poll<T> poll(Context& cx) {
        if (!handle_ || handle_.done()) {
            return Poll<T>::ready_with(std::move(handle_.promise().result));
        }
        handle_.promise().current_ctx = &cx;
        handle_.resume();  // runs until next co_await or co_return
        if (handle_.done()) {
            return Poll<T>::ready_with(std::move(handle_.promise().result));
        }
        return Poll<T>::pending();
    }

    // Awaiter support: makes Task<T> co_await-able
    bool await_ready() const { return handle_.done(); }
    void await_suspend(std::coroutine_handle<> caller) {
        handle_.promise().continuation = caller;
    }
    T await_resume() { return std::move(handle_.promise().result); }

    ~Task() { if (handle_) handle_.destroy(); }
    Task(Task&& o) noexcept : handle_(std::exchange(o.handle_, nullptr)) {}
    Task& operator=(Task&& o) noexcept {
        if (this != &o) {
            if (handle_) handle_.destroy();
            handle_ = std::exchange(o.handle_, nullptr);
        }
        return *this;
    }
    Task(const Task&) = delete;
    Task& operator=(const Task&) = delete;

private:
    explicit Task(std::coroutine_handle<promise_type> h) : handle_(h) {}
    std::coroutine_handle<promise_type> handle_;
};
```

#### Why `Task<T>` and not `Future<T>`?

Rust has two layers:

- `Future` is a trait (`poll(...)` contract).
- `async fn` returns a concrete compiler-generated state-machine type (`impl Future`).

In this runtime, `Task<T>` is that concrete state-machine owner (coroutine handle + `poll` + awaiter support), so the transpiler maps `async fn` to `rusty::Task<...>` directly.

Using `Task` also avoids confusion with `std::future`, which has different semantics from Rust's poll-based `Future`.

#### Executor — The Event Loop (like tokio)

```cpp
class Executor {
public:
    void spawn(Task<void> task) {
        tasks_.push_back(std::move(task));
        ready_queue_.push(tasks_.size() - 1);
    }

    void run() {
        while (!ready_queue_.empty()) {
            auto idx = ready_queue_.front();
            ready_queue_.pop();

            Waker waker{[this, idx]() { ready_queue_.push(idx); }};
            Context cx{&waker};

            auto result = tasks_[idx].poll(cx);
            // Pending → waker will re-enqueue when IO fires
            // Ready → task is done
        }
    }

private:
    std::vector<Task<void>> tasks_;
    std::queue<size_t> ready_queue_;
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
rusty::Task<rusty::Result<rusty::String, Error>> fetch(std::string_view url) {
    auto response = co_await client.get(url).send();
    if (response.is_err()) {
        co_return rusty::Result<rusty::String, Error>::Err(response.unwrap_err());
    }
    auto body = co_await response.unwrap().text();
    if (body.is_err()) {
        co_return rusty::Result<rusty::String, Error>::Err(body.unwrap_err());
    }
    co_return rusty::Result<rusty::String, Error>::Ok(body.unwrap());
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

The module-map key is the named-module identity used by the emitted `import`;
`namespace` is the C++ namespace that qualifies that module's indexed exports.
They need not be equal. Namespace values are restricted to nonempty C++
identifier paths separated by `::` (no leading `::`, dotted paths, templates,
punctuation, C++ keywords, double underscores, or a leading underscore on the
global segment).

CLI configuration:

- pass one or more `--cpp-module-index <path>` flags in single-file, crate, or parity flows.
- index files are merged deterministically; conflicting duplicate module/symbol definitions are rejected.
- when `use cpp::...` imports are present and no non-empty index is configured, transpilation fails immediately.

If a `cpp::` import or referenced symbol cannot be resolved, transpilation fails with an explicit import/symbol error.

Calls require an explicit Rust `unsafe` context after the module, symbol, and
callable signature match the index.

#### MVP Support Limits (Enforced)

Current enforced MVP surface is intentionally narrow:

- supported:
  - free/static function calls through imported bindings (`binding::symbol(...)`),
  - indexed member-function calls via receiver-first syntax (`binding::Type::method(receiver, args...)`),
  - module constants in value position (`binding::CONSTANT`).
- unsupported (fail-fast with explicit `TODO(leaf22.7)` diagnostics):
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

Date baseline for this status snapshot: **April 18, 2026**.

### 10.1 Crosswalk: Where the Former Appended Content Now Lives

| Source intent (old post-§9 appendices) | Integrated location in this doc | What is now captured |
|---|---|---|
| Whole-crate transpilation planning and orchestration | §10.2, §10.3 | Current pipeline (`parity-test`), crate/module orchestration, stage model |
| Real-crate gap fixing (`either`, expanded tests) | §10.4.1-§10.4.7, §10.5.1 | Import/path lowering, constructor/type-context recovery, pattern lowering, runtime shims |
| Phase 18/20 leaf progress | §10.4, §10.5, §10.6 | Consolidated by technical theme instead of timeline |
| Matrix harness and CI behavior | §10.3.3, §10.5.2, §10.7 | Deterministic first-failure diagnostics, matrix execution contract |
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

- Expected-type propagation now covers typed `let`, assignment, return, match-arm, and closure-body contexts.
- Generic constructor call-site inference includes block-level forward scans for owner-placeholder locals and usage-driven recovery of missing template args.
- Owner-placeholder recovery is wired for `OnceCell`/`OnceBox`/`Lazy`/`Box` constructor families (`new/new_`, `from`, `with_value`) with expected-type substitution; unresolved sites degrade to explicit `auto` placeholders instead of invalid partial specialization.
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
- local owner-placeholder scans now use forward usage evidence (`set`, `get_or_init`, `get_or_try_init`, assignments, and associated-call argument positions) to backfill omitted generic args for earlier initializers.
- `OnceCell`/`OnceBox` fallback inner-type inference scans the block for consistent initializer evidence and extracts `Result<T, E>` payloads for `get_or_try_init` closures.
- expected owner substitution now resolves `_` and single-letter type params in associated-call argument hints, while `infer_hint_type_from_expr` skips unresolved type-parameter bindings to avoid leaking `T/F/E` into emitted C++ template args.

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
- closure emission now pushes outer expected return hints into inner closure contexts when closure output is default/unresolved, which stabilizes `get_or_try_init(|| Err(...))` and related `Result`-returning lambdas.
- `Ok/Err` qualification in closures now normalizes to `rusty::Ok`/`rusty::Err` (including unqualified and turbofish forms) with fail-early guardrails for unresolved constructor-context cases.

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
- receiver lowering for `get_or_init`/`wait`/`force` families now binds references explicitly so pointer/reference categories survive downstream method lowering.
- this iteration also added shared runtime/transpiler surfaces for `Box::from_raw` template inference, `thread::scope`, `thread::sleep(Duration)`, `Barrier::new_`, `Barrier::wait() const`, `Result<const T&, E>` pointer-safety adaptation, and cross-type comparison helpers used by parity crates (`rusty::String` vs `std::string`, `NonZero<T>` equality).

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

#### 10.4.8 April 12-18, 2026 Commit-Integrated Additions

Integrated outcomes from `d67bf42..bb69a16`:

- generic constructor/type-inference infrastructure and rollout to `OnceCell`/`OnceBox`/`Lazy`/`Box` owner placeholders (`b6c1137`, `1d3100d`, `0736977`, `c1e27c2`, `caa49d3`, `508fb20`, `c6ed939`, `e0c71fd`).
- closure `Result` typing and `Ok/Err` qualification hardening for `get_or_try_init`/related lambda bodies (`6bb5e14`, `6d0c74b`, `45fe626`, `f989d5e`, `897e589`).
- runtime/import/module-surface additions needed by parity expansion (`17c59ea`, `577cb20`, `a900f00`, `03b2cb6`, `c198aee`, `e5e3908`, `4e92c3e`, `5689a73`, `806001f`).
- post-once_cell regression stabilization across transpiler/runtime parity surfaces (`060c66f`, `10d31da`, `ec41837`, `cc8e417`, `bb69a16`).

#### 10.4.9 Allocator Translation Model: `rusty::alloc::Allocator` + `Global`

Goal: transpile Rust APIs that bound generics on `core::alloc::Allocator`
(notably the rust-lang/rust `library/alloc/src/collections/btree/*` corpus,
whose public types are `BTreeMap<K, V, A: Allocator + Clone = Global>` and
companions) into C++ that names the right second type parameter and the
right concept — not into one-arg shims that silently drop allocator state.

**Design choice (Option 2, faithful):** mirror Rust's allocator surface at the
type level inside `rusty::alloc::*`, default the second template parameter
to `Global` on the allocator-aware types so existing one-arg call sites
remain source-compatible, and route Rust's `<A: Allocator>` and `<A: Clone>`
bounds to real C++ concepts in the emitted `requires` clause.

Rejected alternatives (kept here so the choice is auditable):

- **Option 1, erase entirely:** drop the second type parameter and emit
  one-arg `Box<T>` / `Vec<T>` everywhere — loses overload distinction for
  alloc-parametric code and breaks any `<A: ...>` shape that flows through.
- **Option 3, alias to `std::allocator`:** rebinds Rust's allocator trait
  onto STL's allocator concept — mismatched signatures (`allocate(Layout)`
  vs `allocate(n)`, byte-pointer vs typed-pointer return) cause emitted
  call sites to silently miscompile.
- **Option 4, custom marker tag only:** declare `rusty::alloc::Allocator`
  as a tag struct without a concept body — no actual constraint enforcement,
  so non-allocator types satisfy `<A: Allocator>` bounds in transpiled C++.

Integrated outcomes (commit chain `fe254c1..33dbbe6`):

- `rusty::alloc::Allocator` is now a faithful C++20 concept requiring
  `ca.allocate(layout) -> rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>`
  and `ca.deallocate(p, layout)` on `const A&`. `rusty::alloc::Global`
  satisfies the concept via `::rusty::alloc::alloc` / `dealloc` and is
  statically asserted at header-load time.
- `rusty::alloc::Layout` gains `new_<T>()` (the post-keyword-escape spelling
  of Rust's `Layout::new::<T>()`), `for_value<T>()` (readable alias), and
  `array<T>(n)` factories so allocation sites compile by name without
  per-instance constructor sites.
- `rusty::alloc::AllocError` is a zero-sized error type, mirroring
  `core::alloc::AllocError`.
- `rusty::Box` and `rusty::Vec` are now `Box<T, A = rusty::alloc::Global>`
  and `Vec<T, A = rusty::alloc::Global>`, with `A` stored as a real field
  (`[[no_unique_address]] A alloc_;`) that **collapses to zero bytes for
  empty `A`** thanks to C++20 attribute-driven empty-member optimization.
  Verified: `sizeof(rusty::Box<int>) == sizeof(int*)` and
  `sizeof(rusty::Vec<int>) == sizeof(int*) + 2 * sizeof(size_t)` (commit
  `dae6b7a` / `e44507f`).
- `Box::new_in(value, alloc)` is now faithful: it asks `alloc.allocate(
  Layout::for_value<T>())` for raw bytes, placement-news `T` into them,
  and stores the allocator alongside the pointer. `Box::new_(value)`
  routes through `new_in` with a default-constructed `A`. A new
  `Box::emplace<Args...>(args...)` factory avoids the intermediate
  value-move; `make_box(args...)` calls it.
- `Box::~Box()` and `Vec::deallocate_storage(...)` perform the **split-
  destruct** pattern (`ptr->~T(); alloc_.deallocate(NonNull{...}, Layout::
  for_value<T>())`), matching Rust's `impl Drop for Box<T, A>` and
  `RawVec<T, A>::dealloc`. Move ctor/asgn transfer both the pointer and
  the allocator state; converting moves (`Box<U, UA>` → `Box<T, A>`)
  require matching allocator types so the destination faithfully owns
  the source's bytes. `Box::clone` requires `std::copyable<A>` and clones
  the allocator alongside the value, matching Rust's
  `impl<T: Clone, A: Allocator + Clone> Clone for Box<T, A>`.
- Converting move ctor / assignment and `operator==` / `!=` on `Box` /
  `Vec` now span across allocator types (`<U, UA>` source → `<T, A>`
  destination), so cross-A construction in transpiled Rust code compiles.
- Type-map adds `core::alloc::{AllocError, Allocator}`,
  `alloc::alloc::Global` (and `std::` siblings) so import statements like
  `use core::alloc::{Allocator, Layout};` lower to using-aliases of the
  `rusty::alloc::*` surface without per-import special casing.
- `well_known_concept_for_trait_path` runs before the facade-name lookup
  in `collect_emitted_template_parts`, so a generic bound that resolves to
  a known standard concept emits a faithful `requires (concept<T>)`
  clause even in module mode (where facade-style `…Facade::is_satisfied_by`
  constraints are deliberately stripped). The current mapping is:
  `Allocator` (any prefix) → `rusty::alloc::Allocator<T>`,
  `Clone` (any prefix) → `std::copyable<T>`.

Emitted shape (the target the rust-lang/rust btree corpus requires):

```cpp
template<typename T, typename A>
    requires (rusty::alloc::Allocator<A> && std::copyable<A>)
rusty::Box<T, A> boxed_in(T value, A alloc);
```

End-to-end verification: a Rust `fn f<T, A: Allocator + Clone>(value: T, alloc: A) -> Box<T, A>`
transpiles to the above shape and compiles against the support headers
without further hand-patching. Stage 3 added two focused codegen tests
(`test_allocator_bound_lowers_to_rusty_alloc_concept`,
`test_allocator_plus_clone_bound_emits_concept_pair`) and updated the
existing `test_trait_bound` / `test_where_clause` to assert
`std::copyable<T>` (the new faithful target) instead of the prior
facade-stub text. Net unit-test delta: 1155 → 1157 passing.

Directly supports:

- §1.10 Collections (allocator-parametric containers)
- §3.11 Generics / Templates (real concept emission for trait bounds)
- §4.4 Orphan rule (consistent target naming for cross-crate impls of
  the Allocator trait, which would otherwise route through facades)

### 10.5 Parity Program Status

#### 10.5.1 Phase 18 (`either`) Consolidated Outcome

Outcome: The former long Phase 18 sequence is complete at the feature level and is now represented by the thematic integration in §10.4.

Key result:

- parity work moved from syntax/bootstrap blockers to semantic-shape correctness (constructors, match lowering, import/path/runtime surfaces), with repeated reprobe cycles until the first deterministic blocker family advanced.

#### 10.5.2 Phase 20+ (Nine-Crate Matrix) Consolidated Outcome

Target matrix:

- `either`, `tap`, `cfg-if`, `take_mut`, `arrayvec`, `semver`, `bitflags`, `smallvec`, `once_cell`

Current observed matrix frontier:

- latest full g++ matrix run (`/tmp/rusty-parity-matrix-fix-20260418-gpp-all`) passes all nine crates (`pass=9`, `fail=0`).
- focused `once_cell` g++ repro (`/tmp/rusty-parity-matrix-fix-20260418-once-gpp/once_cell`) passes (`75 passed`, `0 failed`).
- latest full clang matrix repro (`/tmp/rusty-parity-matrix-fix-20260418-full-clang`) is `pass=8`, `fail=1` with remaining failure at `once_cell` (`export using ...` outside module purview).
- `itertools` remains intentionally deferred from the active matrix set while the nine-crate parity baseline is stabilized.

Crate-focused progress integrated from former appendices:

- `either`: parity-control crate and expanded-test correctness hardening
- `tap`: literal/method-shape and extension-trait lowering fixes
- `cfg-if`: baseline resiliency and alias/import typing fixes
- `take_mut`: type/lifetime order, ptr/mem path lowering, and template/context fixes
- `semver`: import/re-export lowering and expanded build-shape fixes
- `bitflags`: Stage D+E parity chain is closed in active matrix runs
- `arrayvec`: post-once_cell regressions from recent codegen/runtime edits were fixed and revalidated in full g++ matrix runs
- `smallvec`: deterministic Stage D/E leaf chain is collapsed in the active matrix set
- `once_cell`: constructor/closure/result-typing families now pass in the active g++ matrix set

### 10.6 Active Frontier and Next Work

The active g++ frontier from the April 11, 2026 ten-crate snapshot is closed for the current nine-crate matrix set (`itertools` deferred).

Current status snapshot:

1. Focused `once_cell` g++ parity repro passes: `/tmp/rusty-parity-matrix-fix-20260418-once-gpp/once_cell/{baseline.txt,build.log,run.log,matrix.log}` (`75 passed`, `0 failed`).
2. Full nine-crate g++ matrix passes: `/tmp/rusty-parity-matrix-fix-20260418-gpp-all/{either,tap,cfg-if,take_mut,arrayvec,semver,bitflags,smallvec,once_cell}/...` (`pass=9`, `fail=0`).
3. Full nine-crate clang repro currently remains `pass=8`, `fail=1` with `once_cell` failing module-purview export emission: `/tmp/rusty-parity-matrix-fix-20260418-full-clang/once_cell/{baseline.txt,build.log,run.log,matrix.log}`.
4. Next active work is clang module-purview export cleanup for `once_cell`, then re-enabling `itertools` in the active matrix.
5. Historical April 11, 2026 frontier chain is retained below for traceability.
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
30. Focused `smallvec` repros after `Leaf 5.1.18` (`/tmp/rusty-parity-matrix-5-1-18-20260411a/smallvec/...` then `/tmp/rusty-parity-matrix-5-1-18-20260411b/smallvec/...`) collapse the prior post-5.1.17 equality/literal first-head family from deterministic first slots (`runner.cpp:2418` array-equality mismatch and adjacent `runner.cpp:2442` as-slice/span equality mismatch); follow-up revalidation (`/tmp/rusty-parity-matrix-5-1-18-20260411c/smallvec/...`) preserves the same post-collapse first-head family.
31. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1612` (`deduced type 'void' for structured bindings is incomplete`), followed by adjacent `runner.cpp:1615` invalid use of void-expression fallout (`triple()`/structured-binding lowering family).
32. Focused `smallvec` repro after `Leaf 5.1.19` (`/tmp/rusty-parity-matrix-5-1-19-20260411b/smallvec/...`) collapses the prior post-5.1.18 structured-binding/void-return first-head family (`runner.cpp:1612/1615` `void`-deduced `triple()` binding path) by hardening shared divergence detection and typed match-arm fallback lowering.
33. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2379` (`taking address of rvalue`), followed by adjacent `runner.cpp:2387` equality-surface mismatch (`ConstNonNull<rusty::String> == const char*`) and downstream independent compile families.
34. Guardrail check against §11 remains satisfied for `Leaf 5.1.19`: fixes were shared and AST/type-shape-aware in core transpiler lowering; no crate-specific ad-hoc scripts and no generated-output text patching were introduced.
35. Focused `smallvec` repros after `Leaf 5.1.20` (`/tmp/rusty-parity-matrix-5-1-20-20260411a/smallvec/...` through `/tmp/rusty-parity-matrix-5-1-20-20260411d/smallvec/...`) collapse the prior post-5.1.19 index-reference/assertion first-head family in `test_spill` (`runner.cpp:2379/2387`) by preserving softened reference-return categories (`decltype(auto)`), hardening tuple index-reference pointer bridging (`rusty::as_ref_ptr(...)`), and tightening runtime const-pointer helper fallback.
36. New first deterministic Stage D head in `smallvec` now starts at `/usr/include/c++/14/array:103` (`forming pointer to reference type` for `std::array<const unsigned int&, 2>`), with immediate adjacent reference-element array instantiation fallout at `array:104/107/108/111/112` and `array:61` through `MaybeUninit<std::array<const unsigned int&, 2>>`.
37. Guardrail check against §11 remains satisfied for `Leaf 5.1.20`: fixes were shared and AST/runtime-shape-gated in transpiler/runtime helper surfaces; no crate-specific ad-hoc scripts and no generated-output text patching were introduced.
38. Focused `smallvec` repro after `Leaf 5.1.21` (`/tmp/rusty-parity-matrix-5-1-21-20260411a/smallvec/...`) collapses the prior post-5.1.20 reference-element array/`MaybeUninit` first-head family by lowering Rust reference-element fixed arrays to `std::array<std::reference_wrapper<...>, N>` (plus matching fixed-array materialization paths), removing the `std::array<T&, N>` instantiation surface.
39. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2615` (`Drain<std::array<unsigned char, 2>>` has no member `rev`), with immediate adjacent same-family fallout at `runner.cpp:2636` and downstream `IntoIter<...>::rev` gaps at `runner.cpp:2727/2749`.
40. Guardrail check against §11 remains satisfied for `Leaf 5.1.21`: fixes stayed shared and AST/type-shape-gated in core array lowering/materialization paths; no crate-specific ad-hoc scripts and no generated-output text patching were introduced.
41. Focused `smallvec` repro after `Leaf 5.1.22` (`/tmp/rusty-parity-matrix-5-1-22-20260411a/smallvec/...`) collapses the prior post-5.1.21 iterator-reverse adapter first-head family by broadening iterator-adapter shape gating for `rev`/`enumerate` to include probable iterator-shaped receivers and by classifying `drain` chains as probable iterator receivers.
42. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2656` (`cannot convert Vec<int> to Vec<unsigned char>` in `SmallVec::<[u8; N]>::from_vec(...)` boxed-array literal paths), with immediate adjacent same-family fallout at `runner.cpp:3227` and `runner.cpp:4364/4384/4404/4424`.
43. Guardrail check against §11 remains satisfied for `Leaf 5.1.22`: fixes stayed shared and receiver-shape-gated in core iterator adapter lowering; no crate-specific ad-hoc scripts and no generated-output text patching were introduced.
44. Focused `smallvec` repro after `Leaf 5.1.23` (`/tmp/rusty-parity-matrix-5-1-23-20260411c/smallvec/...`) collapses the prior post-5.1.22 boxed-array numeric-typing first-head family by hardening expected-owner associated-call argument typing (owner-substitution-aware expected arg recovery in `try_emit_associated_call_with_expected_type`) and broadening `into_vec` boxed-array specialization path-shape matching.
45. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2691` (`operator==` mismatch between `rusty::Vec<const unsigned char*>` and `std::array<int, 1>` in `into_iter` assertion shape), with adjacent same-family fallout at `runner.cpp:2713` (`std::array<int, 3>`).
46. Guardrail check against §11 remains satisfied for `Leaf 5.1.23`: fixes stayed shared and AST/type-context-gated in core associated-call and `into_vec` lowering paths; no crate-specific ad-hoc scripts and no generated-output text patching were introduced.
47. Focused `smallvec` repro after `Leaf 5.1.24` (`/tmp/rusty-parity-matrix-5-1-24-20260411d/smallvec/...`) collapses the prior post-5.1.23 `into_iter` assertion element-shape first-head family (`runner.cpp:2691/2713`) by making collect-lowering bridge of `.into_iter()` receivers conditional on unresolved/type-parameter receiver shape.
48. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1966` (`std::tuple<...>` projected via `._0` in `triple()`/`triple_mut()` paths), with adjacent option-return-shape fallout at `runner.cpp:2285/2300` (`std::nullopt_t` vs `std::optional<...>` deduction in iterator `next`/`next_back`).
49. Guardrail check against §11 remains satisfied for `Leaf 5.1.24`: fix stayed shared and receiver/type-shape-gated in core collect lowering, with no crate-specific ad-hoc scripts and no generated-output text patching.
50. Focused `smallvec` repro after `Leaf 5.1.25` (`/tmp/rusty-parity-matrix-5-1-25-20260412a/smallvec/...`) collapses the prior post-5.1.24 tuple-field/option-return first-head family by:
   - lowering tuple unnamed-field projection to `std::get<N>(...)` for tuple-like receivers (including local `self.method().N` tuple-return paths), and
   - scoping typed `Option` ctor lowering to auto-return methods that mix `Some(...)` and `None` under dependent-assoc softening (removing mixed-branch auto-deduction drift).
51. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2956` (`template<class T> class rusty::Box used without template arguments`), with adjacent same-family fallout at `runner.cpp:3014` and nearby Box-shape/type-surface errors (`runner.cpp:1387`).
52. Guardrail check against §11 remains satisfied for `Leaf 5.1.25`: fixes stayed shared and AST/type-shape/context-gated in core lowering paths; no crate-specific ad-hoc scripts and no generated-output text patching were introduced.
53. Focused `smallvec` repro after `Leaf 5.1.26` (`/tmp/rusty-parity-matrix-5-1-26-20260412a/smallvec/...`) collapses the prior post-5.1.25 `rusty::Box` unspecialized first-head family by:
   - allowing method-arg receiver inference for assoc-projection-shaped expected types even when the projection owner type param is out-of-scope at the concrete call site (`A::Item` call contexts),
   - recovering `SmallVec` element expected types from array-like owner args for element-taking methods (`push`/`insert` family), and
   - hardening `std::add_pointer_t<T>` const-cast lowering to use `std::add_pointer_t<std::add_const_t<T>>` without extra pointer indirection.
54. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1730` (`SmallVec<...>` missing `swap` member in `swap_remove` path), with adjacent early fallout at `runner.cpp:1387` (`rusty::ptr::NonNull<...>::new_` missing member) and downstream `triple_mut`/iterator assertion shape errors; guardrail check against §11 remains satisfied (`Leaf 5.1.26` fixes stayed shared and AST/type-context-gated, with no crate-specific scripts and no generated-output patching).
55. Focused `smallvec` repro after `Leaf 5.1.28` (`/tmp/rusty-parity-matrix-5-1-28b/smallvec/...`) collapses the prior post-5.1.27 `triple_mut` tuple-shape first-head family (`runner.cpp:1549/1592/1593`) by:
   - preserving typed tuple constructor element-lowering for reference-bearing expected tuple shapes,
   - hardening typed local shadow initializer resolution (`let x: T = x`) to avoid self-referential RHS mapping,
   - and recovering self-method return-type inference for tuple local bindings so reference-like deref collapse is applied in downstream unary-deref lowering.
56. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1747` (`rusty::copy` call-shape mismatch in `remove`/`swap_remove` paths), with adjacent downstream fallout at `runner.cpp:2998` (invalid unary `*` on `std::array<rusty::Box<unsigned char>, 8>`) and iterator-map deref-shape failures (`runner.cpp:3016/3060/...`).
57. Guardrail check against §11 remains satisfied for `Leaf 5.1.28`: fixes stayed shared and AST/type-shape-gated in core codegen paths, with no crate-specific scripts and no generated-output text patching.
58. Focused `smallvec` repro after `Leaf 5.1.29` (`/tmp/rusty-parity-matrix-5-1-29/smallvec/...`) collapses the prior post-5.1.28 `runner.cpp:1747` `ptr::copy` call-shape first-head family by:
   - preserving overwritten same-scope shadow binding types for in-progress initializer lookup (`let x = x...`), so shadowed `as_ptr` receiver inference still sees the previous binding type, and
   - making `as_ptr`/`as_mut_ptr` pointer inference wrapper-aware for both pointee and mutability (`NonNull`/`ConstNonNull`/`Unique`/`Ptr`/`MutPtr`), so `NonNull::as_ptr()` lowers to mutable raw-pointer shape.
59. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3016` (`invalid type argument of unary '*'` in iterator-map boxed-byte deref shape), with adjacent same-family fallout at `runner.cpp:3060/3104/3148` and downstream callback-signature mismatch chain in `slice.hpp`.
60. Guardrail check against §11 remains satisfied for `Leaf 5.1.29`: fixes stayed shared and AST/type-context-gated in core local-binding/pointer inference paths, with no crate-specific scripts and no generated-output text patching.
61. Focused `smallvec` repro after `Leaf 5.1.30` (`/tmp/rusty-parity-matrix-5-1-30/smallvec/...`) collapses the prior post-5.1.29 iterator-map unary-deref first-head family by:
   - introducing scoped iterator-map callback parameter tracking for closure emission, and
   - collapsing exactly one unary-deref layer only for untyped iterator-map callback parameter paths (`*v -> v`, `**v -> *v`) while keeping non-map closure deref lowering unchanged.
62. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3171` (`passing const SmallVec<...> as this argument discards qualifiers`) in `catch_unwind([=](){ ... })` closure bodies, with adjacent same-family callable-shape fallout at `runner.cpp:3170/3180/3190/3200/3210` (`catch_unwind` expects `AssertUnwindSafe<F>` wrapper surface).
63. Guardrail check against §11 remains satisfied for `Leaf 5.1.30`: fix stayed shared and narrowly context-gated to iterator-map callback-parameter scopes; no crate-specific scripts and no generated-output text patching were introduced.
64. Focused `smallvec` repro after `Leaf 5.1.31` (`/tmp/rusty-parity-matrix-5-1-31/smallvec/...`) collapses the prior post-5.1.30 `catch_unwind(move || ...)` first-head family by:
   - adding shared runtime `catch_unwind(F&&)` support for plain callables (while preserving explicit `AssertUnwindSafe<F>` overload routing), and
   - emitting move closures as `[=](...) mutable` so captured-by-value bindings remain mutable in lambda bodies.
65. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3254` (slice element-type mismatch in `insert_from_slice`: `std::span<const int,2>` cannot convert to `std::span<const unsigned char,...>`), with adjacent same-family/API-surface fallout at `runner.cpp:3298` (`extend_from_slice` shape) and downstream independent compile families.
66. Guardrail check against §11 remains satisfied for `Leaf 5.1.31`: fixes stayed shared and semantics-gated in runtime/closure lowering paths; no crate-specific scripts and no generated-output text patching were introduced.
67. Focused `smallvec` repro after `Leaf 5.1.32` (`/tmp/rusty-parity-matrix-5-1-32/smallvec/...`) collapses the prior post-5.1.31 `insert_from_slice`/`extend_from_slice` first-head family by:
   - deriving receiver-item slice expected types from receiver owner generics for `insert_from_slice`/`extend_from_slice` method families (including array-owner `A::Item` extraction), and
   - switching extension-call rewrite collision gating to an inherent-method-only index so local inherent methods block `rusty::method(...)` rewriting without suppressing valid trait-extension rewrites.
68. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3401/3403` (`auto _m0_tmp/_m1_tmp` deduced from `void` in hash assertion tuple temporaries), with adjacent same-family fallout at `runner.cpp:3425/3427`; guardrail check against §11 remains satisfied for `Leaf 5.1.32` (fixes stayed shared and receiver/type-shape-gated with no crate-specific scripts and no generated-output text patching).
69. Focused `smallvec` repro after `Leaf 5.1.33` (`/tmp/rusty-parity-matrix-5-1-33b/smallvec/...`) collapses the prior post-5.1.32 unit-temp and mutable-surface first-head family by:
   - materializing tuple-match unit-valued rvalue expressions as explicit unit values (`std::make_tuple()`) while preserving side effects, and
   - adding shape-gated `self` mutable-reference coercion to `deref_mut()` for expected `&mut Target` returns (non-`Self`) when `deref_mut` exists.
70. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3729/3747` (`std::span<const int, ...>` has no member `from`), with adjacent downstream owner/item-shape conversion fallout (`SmallVec::from(Vec<int>)`, `.from_slice`, and related mismatches); guardrail check against §11 remains satisfied for `Leaf 5.1.33` (fixes stayed shared and AST/type-shape-gated with no crate-specific scripts and no generated-output text patching).
71. Focused `smallvec` repro after `Leaf 5.1.34` (`/tmp/rusty-parity-matrix-5-1-34/smallvec/...`) collapses the prior post-5.1.33 UFCS overreach family by:
   - preventing UFCS trait-call rewriting for local concrete type owners, and
   - constraining self-UFCS fallback rewriting so constructor/static `from*` associated calls are not rewritten as receiver methods.
72. New first deterministic Stage D head in `smallvec` remains at `runner.cpp:3729/3747` but moves to the next compile family: valid associated-call shape with mismatched payload typing (`SmallVec<std::array<uint32_t,2>>::from(std::span<const int, ...>)`), followed by downstream omitted-owner/item-typing conversion fallout (`rusty::Vec` unspecialized owner, `Vec<int>` vs `Vec<unsigned char>`, and related `from_slice` type surfaces); guardrail check against §11 remains satisfied for `Leaf 5.1.34` (fixes stayed shared and shape-gated in UFCS detection/rewrite logic, with no crate-specific scripts and no generated-output text patching).
73. Focused `smallvec` repro after `Leaf 5.1.35` (`/tmp/rusty-parity-matrix-5-1-35e-20260412/smallvec/...`) collapses the prior post-5.1.34 associated-`from` overload expected-type loss family by:
   - preserving raw owner-scoped associated-method argument-type variants (alongside merged hints) so overloaded `Owner::from(...)` sites can still recover expected argument type context,
   - selecting owner-scoped overload hints via call-argument shape (slice-like / vec-like / array-like) when merged hints are ambiguous, and
   - lowering concrete-owner `::Item` qself projections through `rusty::detail::associated_item_t<Owner>` to avoid invalid concrete `Owner::Item` C++ surfaces.
74. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3764` (`template<class T> class rusty::Vec used without template arguments` in `const auto vec = rusty::Vec::new_();`), followed by downstream conversion families (`Vec<int>` / `std::array<int,...>` / `std::vector<int>` into `SmallVec<std::array<uint8_t,...>>::from(...)`) and adjacent runtime-surface gaps (`as_slice`, `from_iter`, `from_raw_parts`); guardrail check against §11 remains satisfied for `Leaf 5.1.35` (fixes stayed shared and shape-gated in overload expected-type recovery and associated-item mapping, with no crate-specific scripts and no generated-output text patching).
75. Focused `smallvec` repro after `Leaf 5.1.36` (`/tmp/rusty-parity-matrix-5-1-36b-20260412/smallvec/...`) collapses the prior post-5.1.35 local-placeholder-associated-`from(...)` family by:
   - collecting initialized-local placeholder hints from associated-call expected-type fallback (not only direct function metadata),
   - applying owner substitutions from statement-local expected context (`let x: Owner<...> = Owner::from(arg)`) before deriving placeholder hints,
   - extending placeholder owner-target coverage to `into_vec(...)` and array/repeat locals, and
   - allowing `into_vec` boxed payload specialization on `Box::new`/`Box::new_` call shapes under recovered element context.
76. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3765` (`use of deleted function rusty::Vec<uint8_t>::Vec(const Vec&)`), reflecting const-local consumption shape in associated `from(...)` calls after owner/item typing is recovered; downstream gaps now continue at runtime surface families (`as_slice`, `from_iter`, `from_raw_parts`) and unrelated iterator/adapter/test surfaces. Guardrail check against §11 remains satisfied for `Leaf 5.1.36` (fixes stayed shared and AST/type-context-gated, with no crate-specific scripts and no generated-output text patching).
77. Focused `smallvec` repro after `Leaf 5.1.37` (`/tmp/rusty-parity-matrix-5-1-37a-20260412/smallvec/...`) collapses the prior post-5.1.36 const-local by-value call-argument family by:
   - layering signature-aware by-value argument consumption analysis on top of existing consuming-receiver/tail-value local detection,
   - using collected function/method pass-style metadata plus owner-scoped associated expected-type fallback to classify ambiguous associated calls (`Owner::from(...)`, `Owner::from_vec(...)`), and
   - forcing immutable consumed locals to lower as non-const storage so `std::move(local)` remains a true move for move-only runtime types.
78. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3848` (`std::vector<unsigned char>` has no member `as_slice`), followed by downstream runtime/associated-surface gaps (`A::Item` span-return surface, `from_iter`/`from_raw_parts`, iterator adapter coverage, and related test-surface fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.37` (fixes stayed shared and metadata/AST-shape-gated, with no crate-specific scripts and no generated-output text patching).
79. Focused `smallvec` repro after `Leaf 5.1.38` (`/tmp/rusty-parity-matrix-5-1-38a-20260412/smallvec/...`) collapses the prior post-5.1.37 `.as_slice()` non-member surface family by:
   - lowering non-pointer `.as_slice()` / `.as_mut_slice()` method calls through shared runtime helpers (`rusty::as_slice(...)` / `rusty::as_mut_slice(...)`) instead of direct member-call emission, and
   - adding shared runtime helper surfaces in `include/rusty/array.hpp` that dispatch through `slice_full(...)` with const/mutable view intent and temporary-receiver support.
80. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3906` (`std::span<const A::Item>` invalid associated-type surface in lambda return position), followed by downstream runtime-surface gaps (`Option::value()`, iterator adapter methods, `from_iter`/`from_raw_parts`, and related test-surface fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.38` (fixes stayed shared and method-shape/runtime-helper-gated, with no crate-specific scripts and no generated-output text patching).
81. Focused `smallvec` repro after `Leaf 5.1.39` (`/tmp/rusty-parity-matrix-5-1-39a-20260412/smallvec/...`) collapses the prior post-5.1.38 invalid qself-associated span-return family by:
   - hardening `try_emit_reference_expr_with_expected_span_storage(...)` to detect out-of-scope type-parameter leakage in expected span element surfaces, and
   - lowering those cases via deduced span-storage lambda return shapes instead of explicit `std::span<const A::Item>` emission.
82. New first deterministic Stage D head in `smallvec` remains at `runner.cpp:3906/3964` but moves to concrete element-typing mismatch (`SmallVec<std::array<uint32_t,2>>::from(std::span<const int, ...>)` no matching overload), followed by downstream runtime-surface gaps (`Option::value()`, iterator adapters, `from_iter`/`from_raw_parts`, and related test-surface fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.39` (fix stayed shared and type-surface-gated, with no crate-specific scripts and no generated-output text patching).
83. Focused `smallvec` repro after `Leaf 5.1.40` (`/tmp/rusty-parity-matrix-5-1-40a-20260412/smallvec/...`) collapses the prior post-5.1.39 concrete span element mismatch family by:
   - applying owner-segment generic substitutions to associated-call fallback expected types (`Owner::<...>::method(...)`) before argument lowering, and
   - preserving specialized element context for indexed-slice associated `from(...)` calls so fallback no longer degrades to `std::span<const int>`.
84. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4087` (`clone_iter.next().value()` lowering to private `Option` member/call surface), followed by downstream runtime/adapter gaps (`IntoIter::skip`, `Vec::from_raw_parts`, `Vec::from_iter`, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.40` (fix stayed shared and owner-type-substitution-gated in core associated-call fallback typing, with no crate-specific scripts and no generated-output text patching).
85. Focused `smallvec` repro after `Leaf 5.1.41` (`/tmp/rusty-parity-matrix-5-1-41a-20260412/smallvec/...`) collapses the prior post-5.1.40 optional-surface unwrap family by:
   - tightening `unwrap()` method-call rewrite gating from syntactic optional-like receiver checks to inferred `std::optional` receiver type checks, and
   - preserving Rust runtime `Option` receiver lowering as `.unwrap()` (instead of `.value()`) when receiver inference does not resolve to `std::optional`.
86. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4124` (`SmallVec::IntoIter` missing `.skip(...)` adapter surface), followed by downstream runtime/type-surface gaps (`Vec::from_raw_parts`, `Vec::from_iter`, literal array element typing, `Rc::new_`, iterator adapter methods, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.41` (fix stayed shared and inference-gated in core method-call lowering, with no crate-specific scripts and no generated-output text patching).
87. Focused `smallvec` repro after `Leaf 5.1.42` (`/tmp/rusty-parity-matrix-5-1-42a-20260412/smallvec/...`) collapses the prior post-5.1.41 iterator `.skip(...)` adapter family by:
   - rewriting iterator-like `.skip(n)` method calls to shared runtime helper form `rusty::skip(receiver, n)` in core method-call lowering, and
   - adding shared runtime `rusty::skip(...)` support (`skip_next_iter`) in `include/rusty/slice.hpp` aligned with existing option-like iterator adapter helpers (`take/map/rev/enumerate`).
88. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1869/1874` (`rusty::Vec<...>::from_raw_parts` / `rusty::Vec<...>::from_iter` missing member surfaces in `SmallVec::into_vec()` paths), followed by downstream runtime/type-surface gaps (`Result::Ok(std::array{0,1})` element typing, `Rc::new_`, `scan`, `filter`, `get`, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.42` (fix stayed shared and iterator-shape/runtime-helper-gated with no crate-specific scripts and no generated-output text patching).
89. Focused `smallvec` repro after `Leaf 5.1.43` (`/tmp/rusty-parity-matrix-5-1-43b-20260412/smallvec/...`) collapses the prior post-5.1.42 `Vec::from_raw_parts` / `Vec::from_iter` missing-member family by:
   - extending owner-template recovery for `Vec::from_raw_parts*` to explicit-owner and omitted-owner call shapes with pointer-driven arg recovery (including `decltype` fallback when direct type hints are weak), and
   - adding shared runtime `Vec` static surfaces in `include/rusty/vec.hpp` for `from_raw_parts`, `from_raw_parts_in`, and `from_iter` (option-like `next()` and range inputs).
90. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4233/4252` (`rusty::Vec<unsigned char>` vs `rusty::Vec<int>` equality mismatch), followed by `runner.cpp:4267` array element conversion mismatch (`std::array<int,...>` to `std::array<uint8_t,...>`) and downstream runtime/adapter gaps (`Rc::new_`, `scan`, `filter`, `get`, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.43` (fixes stayed shared and pointer/type-shape-gated in transpiler/runtime surfaces, with no crate-specific scripts and no generated-output text patching).
91. Focused `smallvec` repro after `Leaf 5.1.44` (`/tmp/rusty-parity-matrix-5-1-44a-20260412/smallvec/...`) collapses the prior post-5.1.43 cross-type `Vec` equality family by:
   - extending runtime `rusty::Vec` equality/inequality surfaces to support `Vec<L>` vs `Vec<R>` comparisons when element values are comparable (including symmetric fallback and empty-marker element handling), and
   - adding runtime regression coverage for cross-numeric-element `Vec` equality in `transpiler/tests/runtime_move_semantics.rs`.
92. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4267` (`Result::Ok(std::array{0,1})` array element conversion mismatch from `std::array<int,...>` to `std::array<uint8_t,...>`), followed by downstream runtime/adapter/type-surface gaps (`Rc::new_`, `scan`, `filter`, `get`, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.44` (fix stayed shared and container-shape-gated in runtime equality surfaces, with no crate-specific scripts and no generated-output text patching).
93. Focused `smallvec` repro after `Leaf 5.1.45` (`/tmp/rusty-parity-matrix-5-1-45b-20260412/smallvec/...`) collapses the prior post-5.1.44 `Result::Ok(std::array{...})` numeric-array conversion family by:
   - propagating peer-result payload expected type into tuple-match peer-context `Ok(...)`/`Err(...)` argument emission in transpiler core lowering, and
   - adding shared runtime `Result::Ok(const std::array<U,N>&)` payload conversion support for array targets with convertible element types (`include/rusty/result.hpp`), plus focused regression coverage in transpiler/runtime tests.
94. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4600/4642` (`rusty::Rc` unspecialized owner surface and missing `Rc<int>::new_()`), followed by downstream runtime/adapter/type-surface gaps (`scan`, `filter`, `get`, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.45` (fixes stayed shared and context/shape-gated in constructor lowering/runtime payload conversion surfaces, with no crate-specific scripts and no generated-output text patching).
95. Focused `smallvec` repro after `Leaf 5.1.46` (`/tmp/rusty-parity-matrix-5-1-46a-20260412/smallvec/...`) collapses the prior post-5.1.45 `Rc::new` owner/runtime surface family by:
   - extending omitted-owner constructor recovery to include `Rc::new/new_` payload-driven owner inference in core call lowering, and
   - adding shared runtime `Rc<T>::new_(...)` alias plus static UFCS-compatible `Rc<T>::clone(const Rc<T>&)` surface in `include/rusty/rc.hpp`.
96. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4864` (`rusty::str_runtime::Chars` missing `.scan(...)` adapter surface), followed by downstream runtime/adapter/type-surface gaps (`range::filter`, `SmallVec::get`, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.46` (fixes stayed shared and owner/type-surface-gated in transpiler/runtime changes, with no crate-specific scripts and no generated-output text patching).
97. Focused `smallvec` repro after `Leaf 5.1.47` (`/tmp/rusty-parity-matrix-5-1-47a-20260412/smallvec/...`) collapses the prior post-5.1.46 iterator `.scan(...)` adapter family by:
   - rewriting iterator-like `.scan(state, f)` method calls to shared runtime helper form `rusty::scan(receiver, state, f)` in transpiler method-call lowering under iterator/probable-iterator receiver gating, and
   - adding shared runtime iterator-scan surfaces in `include/rusty/slice.hpp` (`scan_next_iter`, `make_scan_next_iter`, and public `rusty::scan(...)`) with option-like next/closure-result enforcement.
98. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4970` (`rusty::range<int>` missing `.filter(...)` adapter surface), followed by downstream runtime/type-surface gaps (`SmallVec::get`, inline-capacity static-call shape, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.47` (fixes stayed shared and receiver/iterator-shape-gated in transpiler/runtime changes, with no crate-specific scripts and no generated-output text patching).
99. Focused `smallvec` repro after `Leaf 5.1.48` (`/tmp/rusty-parity-matrix-5-1-48a-20260412/smallvec/...`) collapses the prior post-5.1.47 iterator `.filter(...)` adapter family by:
   - rewriting iterator-like `.filter(pred)` method calls to shared runtime helper form `rusty::filter(receiver, pred)` in transpiler method-call lowering under iterator/probable-iterator receiver gating while preserving Option/Result and non-iterator `filter` surfaces, and
   - adding shared runtime iterator-filter surfaces in `include/rusty/slice.hpp` (`filter_next_iter`, `make_filter_next_iter`, and public `rusty::filter(...)`) with generated-callsite-compatible `size_hint()._0` shape.
100. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:5144` (`SmallVec<std::array<int, 2>>` missing `.get(...)` method surface), followed by downstream runtime/type-surface gaps (inline-capacity static-call shape and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.48` (fixes stayed shared and receiver/iterator-shape-gated in transpiler/runtime changes, with no crate-specific scripts and no generated-output text patching).
101. Focused `smallvec` repro after `Leaf 5.1.49` (`/tmp/rusty-parity-matrix-5-1-49c-20260412/smallvec/...`) collapses the prior post-5.1.48 container `.get(index)` missing-member family by:
   - rewriting slice-like/Vec-like `.get(index)` calls to shared runtime helper form `rusty::get(receiver, index)` under receiver-shape gating (while preserving non-slice-like user-defined `get(...)` methods), and
   - adding shared runtime `rusty::get(container, index)` in `include/rusty/array.hpp` using `slice_full(...)` + `as_ptr`/`len` surfaces for Option-reference return shape compatibility across transpiled container wrappers.
102. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1466` (`A::size()` emitted as static-call shape inside `SmallVec::new_/inline_capacity` paths: `cannot call member function ... without object`), followed by downstream declaration/surface fallout (`array.hpp` mixed-container equality assumptions on `.size()`/`.begin()` for `SmallVec` wrappers, drain/NonNull method-surface gaps, and related errors). Guardrail check against §11 remains satisfied for `Leaf 5.1.49` (fixes stayed shared and receiver/container-shape-gated in transpiler/runtime changes, with no crate-specific scripts, no blanket callsite rewrites, and no generated-output text patching).
103. Focused `smallvec` repro after `Leaf 5.1.50` (`/tmp/rusty-parity-matrix-5-1-50b-20260412/smallvec/...`) collapses the prior post-5.1.49 type-parameter static `A::size()` call-shape family by:
   - rewriting type-parameter static call shape `A::size()` to shared runtime helper form `rusty::detail::type_level_size<A>()` under narrow shape gating (type parameter owner in scope, no qself, no args, two-segment path), and
   - adding shared runtime `rusty::detail::type_level_size<T>()` in `include/rusty/array.hpp` with tuple-size/static-size detection so type-level capacity checks avoid invalid direct static-member-call emission.
104. New first deterministic Stage D head in `smallvec` now starts at `include/rusty/array.hpp:185` (`operator==(const L&, const std::array<...>&)` assumes `lhs.as_slice()` exposes `.size()`/`.begin()` directly for `SmallVec` wrappers), followed by downstream drain-constructor/adapter surface gaps (`Drain<...>` iterator constness mismatch, missing `for_each`, `NonNull::as_mut`, and related errors). Guardrail check against §11 remains satisfied for `Leaf 5.1.50` (fixes stayed shared and shape-gated in transpiler/runtime type-level-size handling, with no crate-specific scripts, no blanket callsite rewrites, and no generated-output text patching).
105. Focused `smallvec` repro after `Leaf 5.1.51` (`/tmp/rusty-parity-matrix-5-1-51a-20260412/smallvec/...`) collapses the prior post-5.1.50 `.as_slice()` identity-return equality family by:
   - normalizing `has_member_as_slice` equality surfaces through shared `rusty::as_slice(...)` instead of directly consuming `.as_slice()` results as `.size()`/`.begin()`-bearing views, and
   - hardening `slice_full` mutable/const helpers to detect identity-return `.as_slice()` / `.as_mut_slice()` shapes and materialize span views via shared `as_ptr`/`as_mut_ptr` + `len` fallback.
106. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1571` (`Drain<...>` constructor call shape mismatch: `Iter<const unsigned char>` passed where mutable iterator payload is expected), followed by downstream `Drain`/pointer-surface gaps (`Drain::for_each`, `NonNull::as_mut`, iterator adapter fallout, and related errors). Guardrail check against §11 remains satisfied for `Leaf 5.1.51` (fixes stayed shared and shape-gated in runtime slice/equality helpers, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
107. Focused `smallvec` repro after `Leaf 5.1.52` (`/tmp/rusty-parity-matrix-5-1-52a-20260412/smallvec/...`) collapses the prior post-5.1.51 `slice::Iter` constness mismatch family by:
   - lowering `core/std/alloc::slice::Iter<'a, T>` type surfaces to `rusty::slice_iter::Iter<const T>` while preserving `slice::IterMut<'a, T>` as `rusty::slice_iter::Iter<T>`, and
   - applying the rewrite through normalized path-shape gating in transpiler type lowering (`map_type`) rather than generated-output patching.
108. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2211` (`Drain<...>` missing `.for_each` surface, followed by `NonNull::as_mut` at `runner.cpp:2215`), with downstream iterator-adapter/call-shape fallout (`rusty::iter` over `IntoIter`, CTAD mismatch in nested `DropOnPanic`, and related errors). Guardrail check against §11 remains satisfied for `Leaf 5.1.52` (fixes stayed shared and type-shape-gated in transpiler lowering, with no crate-specific scripts, no blanket callsite rewrites, and no generated-output text patching).
109. Focused `smallvec` repro after `Leaf 5.1.53` (`/tmp/rusty-parity-matrix-5-1-53b-20260412/smallvec/...`) collapses the prior post-5.1.52 missing `for_each`/`NonNull::as_mut` family by:
   - lowering iterator-like `.for_each(...)` receiver calls to shared runtime `rusty::for_each(...)` with inherent-method preservation and self-receiver iterator fallback gated by `next` surface detection,
   - preserving callable function-item argument wrapping for rewritten `for_each` calls via shared call-argument pass-style emission (avoids invalid direct move of overloaded function surfaces),
   - adding shared runtime `rusty::for_each(...)` helper in `include/rusty/slice.hpp`,
   - adding shared runtime `NonNull<T>::as_mut()` surface in `include/rusty/ptr.hpp`, and
   - hardening local-binding inference/emission so `let x = ptr.as_mut()` preserves mutable-reference declaration shape instead of decaying to `const auto` copy.
110. New first deterministic Stage D head in `smallvec` now starts at `include/rusty/slice.hpp:583` (`rusty::iter requires iter(), data()/size(), or dereferenceable receiver`) with adjacent `runner.cpp:2273` invalid-void-expression fallout and downstream iterator/option-surface gaps. Guardrail check against §11 remains satisfied for `Leaf 5.1.53` (fixes stayed shared and AST/type-shape-gated across transpiler/runtime surfaces, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
111. Focused `smallvec` repro after `Leaf 5.1.54` (`/tmp/rusty-parity-matrix-5-1-54a-20260412/smallvec/...`) collapses the prior post-5.1.53 `rusty::iter` option-like `next()` rejection family by:
   - allowing `rusty::iter(...)` to treat option-like `next()` receivers as valid iterable surfaces while preserving receiver forwarding/reference category, and
   - making option-like `next()` detection in `include/rusty/slice.hpp` self-contained (slice-local option probe/take helpers) so iterator-shape recognition no longer depends on helper definitions from other headers/include order.
112. New first deterministic Stage D head in `smallvec` now starts at `include/rusty/option.hpp:211` (`Option<T>::unwrap_or_else` default callable returns `void` in `swap_remove` unreachable-path lowering at `runner.cpp:1731`), followed by downstream CTAD/adapter families (`DropOnPanic` constructor deduction at `runner.cpp:1838`, `iterable.into_iter()` shape at `runner.cpp:1815`, and related runtime-surface fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.54` (fix stayed shared and shape-gated in runtime iterator adaptation, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
113. Focused `smallvec` repro after `Leaf 5.1.55` (`/tmp/rusty-parity-matrix-5-1-55a-20260412/smallvec/...`) collapses the prior post-5.1.54 `Option::unwrap_or_else` divergent-fallback family by:
   - hardening runtime `Option<T>::unwrap_or_else(F&&)` to accept void-shaped fallback callables (invoke then terminate via `std::abort`) while preserving typed fallback returns, and
   - adding focused runtime regression coverage that compiles and runs a `Some` path with a void fallback callable surface.
114. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1838` (`DropOnPanic` class template argument deduction failure in `insert_many`), followed by downstream adapter/surface gaps (`iterable.into_iter()` at `runner.cpp:1815`, `range<int>::size_hint` surface, and related fallout). Guardrail check against §11 remains satisfied for `Leaf 5.1.55` (fix stayed shared and type-shape-gated in runtime `Option` behavior, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
115. Focused `smallvec` repro after `Leaf 5.1.56` (`/tmp/rusty-parity-matrix-5-1-56a-20260412/smallvec/...`) collapses the prior post-5.1.55 local generic drop-guard instantiation family by:
   - recovering omitted local generic owner arguments for method-scope struct literals (`DropOnPanic { ... }` -> `DropOnPanic<A>{...}` when recoverable), and
   - keeping hoisted local generic type metadata plus hoisted local impl/drop override metadata active during method-body emission so local Drop-bearing guard literals lower with valid non-aggregate constructor call shapes instead of CTAD/non-aggregate designated-initializer fallout.
116. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2152` (`SmallVec::into_iter` emits `return typename SmallVec::IntoIter<A>(...)` parse-shape/type-surface mismatch), with downstream iterator-surface fallout (`iterable.into_iter()` and related families). Guardrail check against §11 remains satisfied for `Leaf 5.1.56` (fixes stayed shared and AST/type-context-gated in core codegen paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
117. Focused `smallvec` repro after `Leaf 5.1.57` (`/tmp/rusty-parity-matrix-5-1-56a-20260412/smallvec/...`) collapses the prior post-5.1.56 associated-alias + generic-`into_iter` head family by:
   - preventing expected-associated-target struct-literal lowering from appending spurious owner template arguments (`typename SmallVec::IntoIter` no longer rewritten to `typename SmallVec::IntoIter<A>` in value-construction shape), and
   - bridging direct generic type-parameter `.into_iter()` calls through `rusty::iter(receiver)` while preserving concrete receiver `.into_iter()` member-call surfaces.
118. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1847` (`insert_many` copies move-only payload before `ptr::write`, causing `use of deleted function ...::PanicOnDoubleDrop(const ...)`), with downstream iterator/runtime surface fallout (`runner.cpp:2081` missing `range<int>::size_hint`, comparison/runtime helper families, and related errors). Guardrail check against §11 remains satisfied for `Leaf 5.1.57` (fixes stayed shared and AST/type-context-gated in core codegen lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
119. Focused `smallvec` repro after `Leaf 5.1.58` (`/tmp/rusty-parity-matrix-5-1-58d-20260412/smallvec/...`) collapses the prior post-5.1.57 move-only `insert_many` copy family by:
   - adding mapped runtime call-argument consumption fallback for pointer write/copy surfaces during block pre-scan (so value-position locals are not emitted `const` when consumed),
   - applying consumed-local move semantics in `match ... { Some(x) => x, None => break }` local initializer lowering (`auto element = std::move(x);`), and
   - making `rusty::for_in` option-like next-iterator dereference mutable in non-const contexts so move-only payload loop tails are no longer forced through const-reference copy paths.
120. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2081` (`rusty::range<int>` has no member `size_hint()` in `SmallVec::extend`), with adjacent downstream comparison/runtime-surface fallout (`runner.cpp:94/95` span ordering surface and later helper/type-surface families). Guardrail check against §11 remains satisfied for `Leaf 5.1.58` (fixes stayed shared and AST/runtime-shape-gated, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
121. Focused `smallvec` repro after `Leaf 5.1.59` (`/tmp/rusty-parity-matrix-5-1-59c-20260412/smallvec/...`) collapses the prior post-5.1.58 range `size_hint` runtime-surface family by:
   - adding shared Rust-style `size_hint()` surfaces to `rusty::range<T>` and `rusty::range_inclusive<T>` that report exact finite remaining bounds (`(remaining, Some(remaining))`), and
   - adding shared Rust-style `size_hint()` surface to `rusty::range_from<T>` that reports open-ended bounds (`(usize::MAX, None)`).
122. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:94/95` (ordering compare scaffolding emits `operator<` on `std::span<...>` payloads, which has no viable overload), with downstream comparison/runtime-surface fallout. Guardrail check against §11 remains satisfied for `Leaf 5.1.59` (fixes stayed shared and runtime-surface-gated, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
123. Focused `smallvec` repro after `Leaf 5.1.60` (`/tmp/rusty-parity-matrix-5-1-60b-20260412/smallvec/...`) collapses the prior post-5.1.59 `std::span` ordering compare family by:
   - routing generated runtime fallback `cmp`/`partial_cmp` ordering through shared `rusty::cmp::detail::less_than(...)` instead of unconditional direct `a < b`/`b < a`,
   - implementing `less_than(...)` with dual support for direct `<`-comparable values and lexicographically comparable begin/end ranges via `std::lexicographical_compare(...)`.
124. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2004` (`NonNull` pointee-owner mismatch in `from_slice`/`from_heap`: `unsigned int*` cannot convert to `std::array<unsigned int, 2>*`), with adjacent downstream runtime-surface fallout (`rusty::len` static-assert and iterator/option surfaces). Guardrail check against §11 remains satisfied for `Leaf 5.1.60` (fix stayed shared and comparator-shape-gated in runtime-helper emission, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
125. Focused `smallvec` repro after `Leaf 5.1.61` (`/tmp/rusty-parity-matrix-5-1-61a-20260412/smallvec/...`) collapses the prior post-5.1.60 `NonNull` owner/pointee mismatch family by:
   - hardening omitted-owner `NonNull::new*` template recovery to prefer pointer-decltype-derived pointee recovery when direct inferred pointee type is placeholder-like (`auto`, TODO markers, or single-segment type-param-like placeholders),
   - preserving direct inferred pointee types when concrete and stable.
126. New first deterministic Stage D head in `smallvec` now starts at `/home/shuai/git/rusty-cpp/include/rusty/array.hpp:597` (`rusty::len` static assertion on unsupported range shape), with adjacent downstream runtime-surface fallout (`/home/shuai/git/rusty-cpp/include/rusty/vec.hpp` option helper lookup and iterator/option shape errors). Guardrail check against §11 remains satisfied for `Leaf 5.1.61` (fix stayed shared and type-shape-gated in owner-template recovery, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
127. Focused `smallvec` repro after `Leaf 5.1.62` (`/tmp/rusty-parity-matrix-5-1-62a-20260412/smallvec/...`) collapses the prior post-5.1.61 iterator-`len`/option-helper lookup family by:
   - extending shared runtime `rusty::len(...)` fallback coverage to iterator-like receivers with `size_hint()` (including tuple-like lower/upper hint extraction with Option-like upper-bound handling) and `into_iter()` fallback,
   - making `Vec::from_iter` use self-contained option-like `next()` probing/value extraction (`is_some`/`has_value`/bool + `unwrap`/dereference-take), removing reliance on cross-header helper lookup order.
128. New first deterministic Stage D head in `smallvec` now starts at `/home/shuai/git/rusty-cpp/include/rusty/vec.hpp:118` (`Vec::from_iter` option-like payload shape mismatch: `const unsigned char*` to `unsigned char`), with adjacent downstream runtime-surface fallout (`runner.cpp:2031` unresolved `repeat(...)` and later iterator/string adapter shape families). Guardrail check against §11 remains satisfied for `Leaf 5.1.62` (fixes stayed shared and shape-gated in runtime helper surfaces, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
129. Focused `smallvec` repro after `Leaf 5.1.63` (`/tmp/rusty-parity-matrix-5-1-63a-20260412/smallvec/...`) collapses the prior post-5.1.62 `Vec::from_iter` pointer/value mismatch family by:
   - adding shared iterator-item normalization in `Vec::from_iter` so pointer/reference-wrapper yielded items are materialized into value-`Vec<T>` payloads (while preserving pointer-`Vec<T*>` collection unchanged),
   - keeping option-like `next()` collection generic and runtime-local (no crate-specific rewrites).
130. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2031` (`repeat(...)` unresolved in `SmallVec::resize` extension path), with adjacent downstream iterator/string adapter/runtime-surface fallout (`runner.cpp:4890` char32_t `is_whitespace` surface and later `slice::scan/iter` fallout families). Guardrail check against §11 remains satisfied for `Leaf 5.1.63` (fixes stayed shared and shape-gated in runtime collection surfaces, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
131. Focused `smallvec` repro after `Leaf 5.1.64` (`/tmp/rusty-parity-matrix-5-1-64a-20260412/smallvec/...`) collapses the prior post-5.1.63 unresolved `repeat(...).take(...)` family by:
   - mapping Rust `repeat` path variants (`repeat`, `iter::repeat`, `core::iter::repeat`, `std::iter::repeat`) to shared runtime `rusty::repeat(...)`,
   - extending iterator-like receiver inference to recognize `repeat(...)` call expressions so adapter chains lower through shared helper surfaces,
   - adding shared runtime `rusty::repeat(...)` option-like iterator adapter (`next` + `size_hint`) compatible with existing `take`/`for_in` flows.
132. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:4890` (`char32_t` receiver emitted with `.is_whitespace()` in `scan(chars(...))` lambda), with adjacent downstream callable/iterator fallout at `include/rusty/slice.hpp:552` (`std::invoke` mismatch for `scan`) and `include/rusty/slice.hpp:684` (`rusty::iter` static assertion on the derived scan iterator). Guardrail check against §11 remains satisfied for `Leaf 5.1.64` (fixes stayed shared and iterator-shape-gated in transpiler/runtime surfaces, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
133. Focused `smallvec` repro after `Leaf 5.1.65` (`/tmp/rusty-parity-matrix-5-1-65a-20260412/smallvec/...`) collapses the prior post-5.1.64 char predicate/member-call family by:
   - adding shape-gated lowering for `is_whitespace()` method calls to shared runtime `rusty::char_runtime::is_whitespace(...)` when receiver is char-like,
   - extending scan-closure emission for `chars`-sourced iterators to track item-parameter names so untyped closure params (`|_, ch|`) can still use char-predicate lowering without blanket method rewrites,
   - extending runtime fallback helper surface with `char_runtime::is_whitespace(char32_t)` and marker detection support.
134. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2117` (`scan_next_iter<...>` has no `size_hint()` member in `SmallVec::extend`), with adjacent downstream optional-interface and data-layout families. Guardrail check against §11 remains satisfied for `Leaf 5.1.65` (fixes stayed shared and type/context-gated in transpiler lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
135. Focused `smallvec` repro after `Leaf 5.1.66` (`/tmp/rusty-parity-matrix-5-1-66a-20260412/smallvec/...`) collapses the prior post-5.1.65 `scan_next_iter` `size_hint` family by:
   - adding shared `scan_next_iter::size_hint()` in runtime `slice` adapter surfaces with conservative scan bounds (`lower=0`) and upper-bound forwarding from underlying iterator hints when available,
   - tightening done-state scan hints to `(0, Some(0))` to match adapter termination behavior.
136. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1879` (`std::optional<int>` emitted with Rust `Option` member surface `.is_none()/.is_some()/.unwrap()`), with adjacent downstream method-surface/data-layout families. Guardrail check against §11 remains satisfied for `Leaf 5.1.66` (fixes stayed shared and iterator-surface-gated in runtime headers, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
137. Focused `smallvec` repro after `Leaf 5.1.67` (`/tmp/rusty-parity-matrix-5-1-67c-20260412/smallvec/...`) collapses the prior post-5.1.66 iterator optional-surface family by:
   - normalizing shared runtime iterator-adapter `next()` return surfaces (`map`, `enumerate`, `rev`, `take`, `skip`, `filter`, `scan`) to emit `rusty::Option<...>` while still accepting option-like upstream iterators, and
   - normalizing shared runtime range `next()` surfaces (`range`, `range_inclusive`, `range_from`) to emit `rusty::Option<...>` instead of leaking `std::optional` into transpiled `.next()` callsites that expect Rust `Option` methods.
138. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2162` (`SmallVec::clone_from` emits unresolved `split_at`/`clone_from_slice` method surfaces), with adjacent downstream data-layout/runtime helper families. Guardrail check against §11 remains satisfied for `Leaf 5.1.67` (fixes stayed shared and iterator/range-surface-gated in runtime headers/tests, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
139. Focused `smallvec` repro after `Leaf 5.1.68` (`/tmp/rusty-parity-matrix-5-1-68b-20260412/smallvec/...` + bounded compile probe) collapses the prior post-5.1.67 clone-from slice-method family by:
   - lowering `split_at` and `clone_from_slice` method calls on known slice-deref container receivers (`Vec`/`ArrayVec`/`SmallVec`) through shared slice helper surfaces (`rusty::split_at(rusty::as_slice(...), ...)`, `rusty::clone_from_slice(rusty::as_mut_slice(...), ...)`), while preserving non-slice/custom receiver member calls unchanged,
   - adding shared runtime generic `rusty::split_at(...)` helper for span/container slice-like surfaces (prefix/suffix with bounds validation).
140. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1426` (`SmallVecData<A>::Inline(...)` unresolved variant-constructor surface), with adjacent CTAD fallback at `runner.cpp:1438` (`SmallVecData_Heap{...}` deduction failure) and downstream data-layout/runtime helper families. Guardrail check against §11 remains satisfied for `Leaf 5.1.68` (fixes stayed shared and container/slice-shape-gated in transpiler/runtime paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
141. Focused `smallvec` repro after `Leaf 5.1.69` (`/tmp/rusty-parity-matrix-5-1-69a-20260412/smallvec/...`) collapses the prior post-5.1.68 `SmallVecData` variant-constructor family by:
   - preventing generic associated-call expected-type lowering from intercepting data-enum variant constructors (`Enum::Variant(...)`) so real variant constructor lowering remains active,
   - normalizing data-enum variant struct target emission (call and struct-literal forms) to carry explicit or recovered enum template arguments (`Enum_Variant<T...>`) instead of unresolved member surfaces (`Enum<T>::Variant(...)`) or CTAD-prone variant structs (`Enum_Variant{...}`).
142. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1323` (`CollectionAllocErr` err-arm payload lowered as direct member access `_mv2.layout` on the sum wrapper), with adjacent downstream runtime helper families (`ptr::write` array construction shape, `MaybeUninit::new_` surfaces, and adapter/ptr helper mismatches). Guardrail check against §11 remains satisfied for `Leaf 5.1.69` (fixes stayed shared and data-enum/variant-shape-gated in transpiler lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
143. Focused `smallvec` repro after `Leaf 5.1.70` (`/tmp/rusty-parity-matrix-5-1-70a-20260412/smallvec/...`) collapses the prior post-5.1.69 `CollectionAllocErr` payload field-access family by:
   - introducing shared runtime-match payload lowering that jointly computes payload bindings and payload conditions for `Result`/`Option` tuple-struct arms,
   - guarding data-enum struct payload patterns with `std::holds_alternative<Enum_Variant>(payload)` and binding fields through `std::get<Enum_Variant>(payload).field` instead of direct sum-wrapper field access (`payload.field`),
   - preserving runtime statement/expression match lowering structure without crate-specific rewriting.
144. New first deterministic Stage D head in `smallvec` remains at `runner.cpp:1323` but moves to runtime-match expression return-type unification fallout (`infallible` lambda deduces inconsistent return types `T` vs `void` across diverging `Err(...)` arm branches), with adjacent downstream pointer/runtime helper families. Guardrail check against §11 remains satisfied for `Leaf 5.1.70` (fixes stayed shared and AST/control-flow-shape-gated in core runtime-match lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
145. Focused `smallvec` repro after `Leaf 5.1.71` (`/tmp/rusty-parity-matrix-5-1-71b-20260412/smallvec/...`) collapses the prior post-5.1.70 runtime-match expression return-type unification family by:
   - hardening runtime expression-match return-prefix selection to distinguish diverging body shape (typed diverging IIFEs keep `return`; untyped diverging calls omit `return`),
   - extending diverging call-path detection to include Rust alloc error surfaces (`alloc::alloc::handle_alloc_error`, `core::alloc::handle_alloc_error`, `std::alloc::handle_alloc_error`),
   - preserving shared expression-match lowering (no crate-specific rewriting).
146. New first deterministic Stage D head in `smallvec` now starts at `include/rusty/ptr.hpp:174` (`rusty::ptr::write` emits `std::construct_at(std::array<...>*, scalar)` in `SmallVec::insert_many` paths), with adjacent downstream pointer/helper/runtime families (`runner.cpp:1663/1669/1673` `NonNull`/call-shape fallout and later helper surfaces). Guardrail check against §11 remains satisfied for `Leaf 5.1.71` (fixes stayed shared and expression-shape-gated in core runtime-match/diverging detection, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
147. Focused `smallvec` repro after `Leaf 5.1.72` (`/tmp/rusty-parity-matrix-5-1-72a-20260412/smallvec/...`) collapses the prior post-5.1.71 `ptr::write` array-target construction family by:
   - hardening `SmallVec<Owner>` item-type recovery in shared inference (`extract_iter_item_type_from_type`) to return `rusty::detail::associated_item_t<Owner>` instead of owner type,
   - hardening `as_ptr`/`as_mut_ptr` local return-type fallback in `SmallVec` contexts to use associated item projection (`associated_item_t<A>`) instead of raw owner parameter (`A`),
   - preserving shared transpiler lowering (no crate-specific rewriting).
148. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2865` (`v.into_iter().next().is_some()` lowering emits `.has_value()` against `rusty::Option<...>`, causing private-member/call-shape failure), with adjacent downstream option/runtime helper families. Guardrail check against §11 remains satisfied for `Leaf 5.1.72` (fixes stayed shared and type-shape-gated in core inference paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
149. Focused `smallvec` repro after `Leaf 5.1.73` (`/tmp/rusty-parity-matrix-5-1-73a-20260412/smallvec/...`) collapses the prior post-5.1.72 iterator-`next()` option-surface mismatch family by:
   - hardening local method-call result inference for iterator-like `next`/`next_back` to recover Rust `Option<Item>` surface instead of `std::optional<Item>`,
   - preserving `has_value`/`value` rewriting only for true `std::optional` receivers and keeping `is_some`/`unwrap` on iterator-next `rusty::Option` paths,
   - preserving shared transpiler lowering (no crate-specific rewriting).
150. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:3054` (assertion array literal typed as `std::array<rusty::detail::associated_item_t<std::array<rusty::Box<uint8_t>, 8>>, 3>{0, 3, 2}` causing `int -> rusty::Box<uint8_t>` conversion failures), with adjacent downstream owner/item-shape fallout (`DropOnPanic<A>` pointer owner mismatch at `runner.cpp:1874` and related helper surfaces). Guardrail check against §11 remains satisfied for `Leaf 5.1.73` (fixes stayed shared and receiver/type-shape-gated in core local inference/method lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
151. Focused `smallvec` repro after `Leaf 5.1.74` (`/tmp/rusty-parity-matrix-5-1-74a-20260412/smallvec/...`) collapses the prior post-5.1.73 associated-item assertion-array mismatch family by:
   - hardening iterator-map item-type inference for deref-chain closure bodies on untyped map params (mirroring the one-layer deref-collapse behavior used in map closure emission),
   - teaching deref-result inference to resolve `rusty::detail::associated_item_t<Owner>` proxies through concrete owner item shape before applying deref,
   - preserving shared transpiler lowering (no crate-specific rewriting).
152. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1874` (`DropOnPanic<A>` constructor pointer-owner mismatch: emitted payload pointer (`A::Item*`) passed where constructor expects owner pointer (`std::add_pointer_t<A>`), causing no viable overload), with adjacent downstream pointer/helper-surface fallout (`NonNull::cast`/call-shape and `MaybeUninit::new_` families). Guardrail check against §11 remains satisfied for `Leaf 5.1.74` (fixes stayed shared and AST/type-shape-gated in iterator-map inference paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
153. Focused `smallvec` repro after `Leaf 5.1.75` (`/tmp/rusty-parity-matrix-5-1-75a-20260412/smallvec/...`) collapses the prior post-5.1.74 `DropOnPanic<A>` owner/payload pointer mismatch family by:
   - hardening omitted local generic arg recovery for struct literals to infer generic args from field-type/initializer-type pairs before owner-scope fallback,
   - allowing `DropOnPanic { start, ... }` to specialize as payload item type (`DropOnPanic<associated_item_t<A>>`) when `start` is `*mut A::Item`, instead of forcing owner-type fallback (`DropOnPanic<A>`),
   - preserving shared transpiler lowering (no crate-specific rewriting).
154. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1663/1669/1673` (`reserve` growth path emits `layout.size()` callable-shape mismatch and `NonNull::cast` member-surface fallout), with adjacent downstream helper/runtime families (`MaybeUninit::new_`, `Result::map_err` payload unification, iterator `size_hint`). Guardrail check against §11 remains satisfied for `Leaf 5.1.75` (fixes stayed shared and AST/type-shape-gated in struct-literal generic recovery paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
155. Focused `smallvec` repro after `Leaf 5.1.76` (`/tmp/rusty-parity-matrix-5-1-76a-20260412/smallvec/...`) collapses the prior post-5.1.75 `reserve` growth `layout`/`NonNull` call-surface family by:
   - lowering zero-arg `layout.size()` / `layout.align()` method-call surfaces to field access (`layout.size`, `layout.align`) when the receiver is alloc `Layout`,
   - extending shared runtime `rusty::NonNull<T>` cast surface to support both contextual `.cast()` and explicit `.cast<U>()` forms used by transpiled reserve/growth paths,
   - preserving shared transpiler/runtime lowering (no crate-specific rewriting).
156. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1530` (`rusty::MaybeUninit<A>::new_(...)` missing member surface in `SmallVec::from_buf`), with adjacent downstream helper/runtime families (`mem::swap` pointer-shape mismatch, iterator `size_hint` surface, `Result::map_err` payload unification). Guardrail check against §11 remains satisfied for `Leaf 5.1.76` (fixes stayed shared and AST/type-shape-gated in core lowering/runtime surfaces, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
157. Focused `smallvec` repro after `Leaf 5.1.77` (`/tmp/rusty-parity-matrix-5-1-77a-20260412/smallvec/...`) collapses the prior post-5.1.76 `MaybeUninit::new_` constructor-surface family by:
   - adding shared runtime `rusty::MaybeUninit<T>::new_(T)` and keeping `new_with(T)` as a compatibility alias to the new canonical surface,
   - preserving shared runtime/transpiler behavior (no crate-specific rewriting).
158. New first deterministic Stage D head in `smallvec` now starts at `include/rusty/mem.hpp:194` (`rusty::mem::swap` invoked as `swap(int* const&, int* const&)`), with adjacent downstream helper/runtime families (`take_next_iter::size_hint` surface at `runner.cpp:2117` and `Result::map_err` payload unification at `runner.cpp:1332`). Guardrail check against §11 remains satisfied for `Leaf 5.1.77` (fix stayed shared and runtime-surface-gated in core `MaybeUninit` API, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
159. Focused `smallvec` repro after `Leaf 5.1.78` (`/tmp/rusty-parity-matrix-5-1-78a-20260412/smallvec/...`) collapses the prior post-5.1.77 `mem::swap` pointer-const first-head family by:
   - broadening pointer-shape detection to treat alias-pointer surfaces (`std::add_pointer_t<...>`) as raw-pointer-like in shared expression lowering,
   - propagating pointer-like alias result types through pointer arithmetic local inference (`add`/`offset`/`sub` and wrapping variants),
   - preserving deref shape for expected-context reborrows (`&*expr` → `*expr`) so pointer-backed borrow calls no longer collapse to bare pointer operands.
160. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:2117` (`take_next_iter<repeat_next_iter<int>>` missing `.size_hint()` surface in `extend` paths), with adjacent downstream payload-family fallout at `runner.cpp:1332` (`Result::map_err` error-payload unification). Guardrail check against §11 remains satisfied for `Leaf 5.1.78` (fixes stayed shared and AST/type-shape-gated in core reborrow/pointer inference paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
161. Focused `smallvec` repro after `Leaf 5.1.79` (`/tmp/rusty-parity-matrix-5-1-79a-20260412/smallvec/...`) collapses the prior post-5.1.78 `take_next_iter::size_hint` first-head family by:
   - adding shared runtime `take_next_iter::size_hint()` with bounded `take` semantics over inner iterator hints (`lower/upper` clipped by `remaining`),
   - normalizing inner size-hint extraction across tuple-like and struct-like (`_0/_1`) bound carriers with option-like upper conversion,
   - collapsing `remaining_` to zero when `next()` observes early source exhaustion so post-exhaustion hint surfaces are deterministic.
162. New first deterministic Stage D head in `smallvec` now starts at `runner.cpp:1332` (`layout_array(...).map_err(...)` payload-family mismatch: `Result<Layout, CollectionAllocErr_CapacityOverflow>` cannot convert to `Result<Layout, CollectionAllocErr>`), with repeated instantiation fallout across `layout_array<T>` call sites. Guardrail check against §11 remains satisfied for `Leaf 5.1.79` (fix stayed shared and runtime-surface-gated in core iterator adapters, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
163. Focused `smallvec` repro after `Leaf 5.1.80` (`/tmp/rusty-parity-matrix-5-1-80a-20260412/smallvec/...`) collapses the prior post-5.1.79 `map_err` payload-family first-head family by:
   - adding expected-error-family-gated `map_err` callable wrapping in codegen so context-known `Result<Ok, ErrExpected>` sites emit typed callable returns (`-> ErrExpected`) instead of deducing narrow variant payload result families,
   - preserving callable-path normalization for associated method callable paths (`Type::simplify`) while applying expected-family typing.
164. `smallvec` deterministic frontier moves past Stage D: C++ compile now passes, and the new first deterministic failure is Stage E runtime execution (`SIGSEGV`, exit 139) before test output; gdb backtrace roots at forgotten-address bookkeeping (`rusty::mem::mark_forgotten_address`) reached from `Drain<...>` teardown (`rusty::for_each` / `next_iter_range` path). Guardrail check against §11 remains satisfied for `Leaf 5.1.80` (fix stayed shared and expected-type-gated in core `map_err` lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
165. Focused `smallvec` repro after `Leaf 5.1.81` (`/tmp/rusty-parity-matrix-5-1-81a-20260412/smallvec/...`) collapses the prior post-5.1.80 Stage E `SIGSEGV` by:
   - preserving lvalue `next()` iterator receivers by reference in `make_next_iter_range(...)` instead of value-decay move/copy,
   - updating `next_iter_range` storage/iterator cursor shape to support reference-backed iterators safely (`std::remove_reference_t<NextIter>*`), preventing `rusty::for_each(*this, ...)` drop paths from re-entering destructor recursion through temporary iterator-adapter ownership.
166. `smallvec` deterministic frontier remains in Stage E but moves to runtime semantic/allocator fallout: first failure is `tests_drain` (`assertion failed`) followed by allocator abort (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.81` (fix stayed shared and runtime-surface-gated in core iterator ownership/lifetime behavior, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
167. Focused `smallvec` repro after `Leaf 5.1.82` (`/tmp/rusty-parity-matrix-5-1-82c-20260412/smallvec/...`) collapses the prior post-5.1.81 `tests_drain` assertion family by:
   - hardening tuple destructuring for mutable/reassigned reference elements to pointer-alias form (`len_ref` slot + pointer binding), preserving Rust rebinding semantics (`len = heap_len`) instead of mutating pointees,
   - routing rebind-reference path/assignment lowering through pointer semantics (auto-deref for value contexts; pointer rebinding for assignments),
   - adding rebind-assignment RHS address-of fallback for unresolved local lvalues plus owner-method return-type inference fallback on non-`self` receivers (`self.data.heap_mut()`), so nested tuple field-method destructuring does not degrade into value/pointer mismatches.
168. `smallvec` deterministic frontier remains in Stage E and moves past `tests_drain`: `tests_drain` now passes, and the new first deterministic failure is `tests_drain_forget` allocator abort (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.82` (fixes stayed shared and AST/type-shape-gated in core tuple/reference lowering + inference paths, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
169. Focused `smallvec` repro after `Leaf 5.1.83` (`/tmp/rusty-parity-matrix-5-1-83b-20260412/...`) collapses the prior post-5.1.82 `tests_drain_forget` allocator-abort family by:
   - adding shared `rusty::mem::detail::leak_construct<T>(...)` support for intentional forget-leak construction with nothrow allocation and constructor-exception cleanup,
   - hardening `rusty::mem::forget(T&&)` fallback for non-`rusty_mark_forgotten` owning types to move ownership into leaked storage (copy fallback when move is unavailable), so forgotten moved values do not keep owning the same allocation and trigger double free later.
170. `smallvec` deterministic frontier remains in Stage E and moves past `tests_drain_forget`: `tests_drain_forget` now passes, and the new first deterministic failure family is `tests_insert_many_panic_panic_*` panic fallout (starting at `tests_insert_many_panic_panic_early_at_end`), with trailing allocator abort (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.83` (fix stayed shared and runtime-semantics-gated in core `mem::forget` behavior, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
171. Focused `smallvec` repro after `Leaf 5.1.84` (`/tmp/rusty-parity-matrix-5-1-84c-20260412/...`) collapses the prior post-5.1.83 `tests_insert_many_panic_panic_*` Stage E family by:
   - hardening move-closure lowering to emit explicit move-init captures for referenced outer locals (`[=, x = std::move(x)]`) so Rust move ownership is preserved in mutated capture paths (for example `catch_unwind(move || { vec.push(...) })`) rather than copy-capture fallback,
   - hardening shared Rust-layout runtime behavior for `std::array<T, N>` in `mem::size_of`/`mem::align_of` (including `N=0`) to match Rust array layout semantics used by panic-path size/assertion checks,
   - preserving shared transpiler/runtime lowering (no crate-specific rewriting).
172. `smallvec` deterministic frontier remains in Stage E and moves past `tests_insert_many_panic_panic_*`: that family now passes, and the new first deterministic failure is `tests::into_iter` runtime failure (`Range end out of bounds`) with trailing allocator abort (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.84` (fixes stayed shared and AST/runtime-shape-gated in closure lowering and runtime layout helpers, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
173. Focused `smallvec` repro after `Leaf 5.1.85` (`/tmp/rusty-parity-matrix-5-1-85a-20260412/...`) collapses the prior post-5.1.84 `tests::into_iter` Stage E ownership failure family by:
   - hardening runtime `collect_range` option-like `next()` collection to consume through forwarding reference (`auto&& iter = range_like`) instead of by-value iterator-owner materialization,
   - removing extra move-created iterator-owner teardown in `collect_range(v.into_iter())` paths that can duplicate moved-from owner destruction for generated `IntoIter`-shaped types,
   - preserving shared runtime helper behavior (no crate-specific rewriting).
174. `smallvec` deterministic frontier remains in Stage E and moves past `tests::into_iter`: `tests::into_iter` now passes, and the new first deterministic failure is `tests::into_iter_drop` runtime assertion failure (`assertion failed`) with trailing allocator abort (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.85` (fix stayed shared and runtime-shape-gated in core iterator collection helpers, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
175. Focused `smallvec` repro after `Leaf 5.1.86` (`/tmp/rusty-parity-matrix-5-1-86b-20260412/...`) collapses the prior post-5.1.85 `tests::into_iter_drop` Stage E failure family by:
   - adding shared forgotten-address range cleanup (`rusty::mem::clear_forgotten_address_range`) so released storage addresses are removed from global forgotten bookkeeping,
   - applying the cleanup in shared Vec teardown paths (`grow`, `reserve`, move-assignment cleanup, destructor), preventing stale address marks from poisoning allocator-reused storage,
   - normalizing `Vec` owned storage allocation/deallocation through shared `rusty::alloc::{alloc,dealloc}` helpers so `from_raw_parts`-owned buffers use the runtime allocator contract consistently.
176. `smallvec` deterministic frontier remains in Stage E and moves past `tests::into_iter_drop`: `tests::into_iter_drop` now passes, and the new first deterministic failure is `tests::into_iter_rev` runtime failure (`Range end out of bounds`) with trailing allocator abort (`free(): double free detected in tcache 2`). ASan single-test isolation (`--rusty-single-test rusty_test_tests_into_iter_rev`) roots the new head at reverse-iteration ownership/lifetime handoff (`rusty::detail::make_rev_next_iter` / `rev_next_iter::next`), where a consumed `IntoIter` source is read after free. Guardrail check against §11 remains satisfied for `Leaf 5.1.86` (fixes stayed shared and runtime-shape-gated in memory/ownership helpers, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
177. Focused `smallvec` repro after `Leaf 5.1.87` (`/tmp/rusty-parity-matrix-5-1-87a-20260412/...`) collapses the prior post-5.1.86 `tests::into_iter_rev` Stage E ownership failure family by:
   - hardening shared `Drop` move-constructor emission for nested first-field `Drop` ownership chains: when `consume_forgotten_address(&other)` is true and the first field type also has `Drop`, generated move ctors now emit one extra `other.rusty_mark_forgotten();` to preserve source forgotten-mark depth,
   - preserving runtime/AST-gated behavior with focused codegen regressions that assert two marks for nested-first-field `Drop` structs and one mark otherwise.
178. `smallvec` deterministic frontier remains in Stage E and moves past `tests::into_iter_rev`: `tests::into_iter_rev` now passes, and the new first deterministic failure is `tests_max_swap_remove FAILED: expected panic` with trailing allocator abort (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.87` (fix stayed shared and AST-shape-gated in generic `Drop` move-constructor generation, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
179. Focused `smallvec` repro after `Leaf 5.1.88` (`/tmp/rusty-parity-matrix-5-1-88b-20260412/...`) collapses the prior post-5.1.87 `tests_max_swap_remove` expected-panic mismatch family by:
   - hardening shared `swap(i, j)` index-swap lowering in `transpiler/src/codegen.rs` to evaluate both indices once (`_swap_i`, `_swap_j`), guard with explicit length bounds, and panic (`rusty::panicking::panic("index out of bounds")`) before any index access,
   - preserving generic lowering behavior (no crate-specific rewrites) and strengthening `leaf5127` swap-lowering regressions to assert bounds-guarded panic shape and single-evaluation index temporaries.
180. `smallvec` deterministic frontier remains in Stage E and moves past `tests_max_swap_remove`: `tests_max_swap_remove` now passes (`expected panic` honored), and the new first deterministic failure is allocator abort after `tests_test_as_mut` with single-test deterministic repro on `rusty_test_tests_test_as_ref` (`free(): double free detected in tcache 2`). Guardrail check against §11 remains satisfied for `Leaf 5.1.88` (fix stayed shared and AST-shape-gated in method-call lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
181. Focused `smallvec` repro after `Leaf 5.1.89` (`/tmp/rusty-parity-matrix-5-1-89b-20260412/...`) collapses the prior post-5.1.88 `tests_test_as_ref` allocator-abort family by:
   - hardening shared const `self` path coercion in `transpiler/src/codegen.rs`: when expected type is an immutable non-`Self` reference and `deref` is available, `self` now lowers to `operator*()` (`this->operator*()` / `<self_override>.operator*()`), symmetric to existing mutable `deref_mut` coercion,
   - preventing incorrect lowering of `as_ref(&self) -> &[T] { self }` to `return (*this);` (object self-reference) which previously induced shallow `SmallVec` value copies in assertion scaffolding and double-free at teardown.
182. `smallvec` deterministic frontier remains in Stage E and moves past `tests_test_as_ref`: `tests_test_as_ref` now passes, and the new first deterministic failure is `tests_test_double_spill FAILED: assertion failed` (single-test deterministic repro: `/tmp/rusty-parity-matrix-5-1-89b-20260412/runner --rusty-single-test rusty_test_tests_test_double_spill`). Guardrail check against §11 remains satisfied for `Leaf 5.1.89` (fix stayed shared and AST/type-shape-gated in path coercion lowering, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).
183. Focused `smallvec` repro after `Leaf 5.1.90` (`/tmp/rusty-parity-matrix-5-1-90e-20260412/...`) collapses the prior post-5.1.89 `tests_test_double_spill` assertion family by:
   - hardening shared runtime slice helpers for rvalue `std::array` temporaries (`include/rusty/array.hpp`): added `owned_array_slice<T, N>` plus `slice_full(std::array<T, N>&&)` so transpiled `rusty::slice_full(std::array{...})` no longer yields dangling `std::span` views,
   - forwarding `as_slice`/`as_mut_slice` through rvalue-aware paths for array temporaries, preserving value-comparison surfaces in assertion scaffolding without crate-specific rewrites.
184. `smallvec` deterministic frontier remains in Stage E and moves past `tests_test_double_spill`: `tests_test_double_spill` now passes, and the new first deterministic failure is `tests_test_from FAILED: assertion failed` (`/tmp/rusty-parity-matrix-5-1-90e-20260412/runner --rusty-single-test rusty_test_tests_test_from`). Guardrail check against §11 remains satisfied for `Leaf 5.1.90` (fix stayed shared and runtime-surface-gated in slice lifetime semantics, with no crate-specific scripts, no blanket generated-output rewrites, and no generated-output text patching).

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
- for iterator-like `next()`/`next_back()` local result inference, do not infer `std::optional` for Rust iterator receivers by default; keep Rust `Option` surface so downstream method rewrites do not emit `has_value`/`value` on `rusty::Option`
- for iterator-map item-type inference on untyped closure params, do not fall back directly to source-item shape when closure body is a deref chain (`*param`, `**param`, ...); apply the same map-context deref-collapse rule used in closure emission and resolve associated-item proxies before deref
- for shared runtime iterator/range adapters consumed by transpiled `.next()` callsites, do not leak `std::optional` return surfaces when generated code expects Rust `Option` members; normalize adapter `next()` surfaces to `rusty::Option` at the runtime boundary
- for slice-deref method surfaces on container receivers (`Vec`/`ArrayVec`/`SmallVec`), do not preserve unresolved member calls (`.split_at(...)`, `.clone_from_slice(...)`) that only exist through Rust slice deref resolution; lower through shared slice helpers while preserving non-slice/custom receiver member calls
- for data-enum variant constructors in generic associated contexts, do not route `Enum::Variant(...)` through expected-type associated static-call lowering and do not emit CTAD-only variant struct targets (`Enum_Variant{...}`) without enum template args; keep variant-constructor lowering on the variant path and emit explicit/recovered template args on variant struct targets
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
- for omitted local generic struct-literal recovery, do not prefer outer-scope owner fallback when field initializers provide stronger payload-type evidence (`*mut U` field fed by `*mut A::Item` local); infer generic args from field-type/initializer-type pairs before scope fallback
- for tuple/binding assertion reference scaffolding, do not take addresses of coerced temporary expressions (for example `&std::string_view(expr)`); materialize coercions into stable temporaries before address-taking
- for closure payload and constructor-hint recovery, do not resolve closure parameter paths through outer local-shadow bindings; bind closure parameters in nested emission scope and avoid in-progress self-binding leakage during hint inference
- for unary-deref lowering inside closures, do not globally collapse deref on untyped closure params across all closure kinds; restrict deref-collapse to proven callback contexts (for example iterator-map callback parameter scope) and collapse at most one layer per deref expression
- for move-closure lowering, do not emit bare `[=]` lambdas without mutability in contexts that mutate captured-by-value state (for example `catch_unwind(move || { vec.push(...) })`); preserve Rust move-closure mutability by emitting mutable value-capture lambdas
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
- for trait-associated-type helper qualification (`typename FooTraits<I>::AssocName`), do not pre-collect trait assoc-types into `trait_associated_type_names` ahead of the forward-decl pre-pass without per-use-site resolution; the qualified path can pass through namespace segments that are *also* function-template or type-alias names at the use site, and ambient lookup will shadow the namespace (surfaced by `'coalesce' is not a class, namespace, or enumeration` in itertools when a naive pre-collect was attempted). See **Chapter 14** for the full design.

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

## 12. Strict V1 Spec: Inline Rust Blocks in C++ (Low-Risk Profile)

This section defines a strict v1 profile for embedding Rust in C++ files and generating fallback C++ in-place.
The goal is incremental migration with deterministic, reviewable generated output while avoiding high-risk language surfaces.

### 12.1 Goals and Non-Goals

Goals:

1. Allow replacing small C++ regions with Rust semantics incrementally.
2. Keep generated fallback C++ in the same file for immediate rollback and debugging.
3. Make generation deterministic so CI can enforce "no manual edits in generated regions."
4. Support only shapes that compile cleanly when lowered code stays local to the containing file/translation unit unless users explicitly place shared surfaces.

Non-goals for v1:

1. No automatic `.h`/`.cpp` declaration/definition splitting.
2. No trait objects, async/await, macros, or procedural macro expansion inside inline blocks.
3. No cross-block type inference or global optimization.
4. No ABI-compatibility guarantees with pre-existing handwritten C++ layout unless explicitly constrained.

### 12.2 File-Form Constraints (Strict)

V1 accepts only:

1. Header-only files (`.h`/`.hpp`) where rewritten functions/templates are inline-safe.
2. C++20 module interface units (`.cppm`) where definitions can remain in the interface unit.
3. Source files (`.cpp`/`.cc`/`.cxx`) when the transpiled surfaces are intended to remain local to that translation unit, or when users manually maintain any required external declarations.

V1 rejects:

1. Automatic `.h`/`.cpp` declaration synchronization or generation across translation-unit boundaries.
2. Inline Rust blocks that require the transpiler to auto-expose declarations to other translation units.
3. Inline Rust blocks that require emitting non-inline ODR-sensitive globals across multiple translation units.

Rationale: this avoids the highest-risk mapping problem (automatic ownership projection across C++ declaration/definition boundaries) while still allowing deliberate `.cpp`-local migration.

### 12.3 Block Activation Contract

Use numeric preprocessor gating, not symbol-presence gating:

- `#if RUSTYCPP_RUST` is valid.
- `#ifdef RUSTYCPP_RUST` is invalid for this workflow because `#define RUSTYCPP_RUST 0` still evaluates true under `#ifdef`.

Required compile modes:

1. Normal builds set `RUSTYCPP_RUST=0` and compile generated C++ fallback.
2. Authoring/verification tooling may set `RUSTYCPP_RUST=1` for extraction/parsing checks, but production compile remains `0` in v1.

### 12.4 Inline Block Grammar (Normative)

Each migration unit MUST use this shape:

```cpp
#if RUSTYCPP_RUST
// Rust subset code (raw Rust grammar, no @rust wrapper required)
#endif
/*RUSTYCPP:GEN-BEGIN id=<stable_id> version=1 rust_sha256=<hex>*/
// generated C++ fallback (read-only)
/*RUSTYCPP:GEN-END id=<stable_id>*/
```

Grammar (EBNF-like):

```text
RustyBlock      := IfRust RustRegion EndIf GenRegion
IfRust          := "#if" "RUSTYCPP_RUST"
RustRegion      := RustToken*
GenRegion       := GenBegin CppToken* GenEnd
GenBegin        := "/*RUSTYCPP:GEN-BEGIN id=" StableId
                   " version=1 rust_sha256=" HexDigest "*/"
GenEnd          := "/*RUSTYCPP:GEN-END id=" StableId "*/"
EndIf           := "#endif"
StableId        := [A-Za-z0-9_.:-]+
HexDigest       := [0-9a-f]+
```

Validity rules:

1. `rust_sha256` MUST be computed from the normalized Rust payload in the `#if RUSTYCPP_RUST` region.
2. Generator MUST overwrite only the `GEN` region and MUST NOT modify text outside the inline block + generated region pair.
3. Manual edits inside `GEN` region are allowed locally but are non-authoritative and overwritten on next generation.
4. Legacy `#else` + `RUST-BEGIN/END` forms are rejected in v1.1+; use only the canonical post-`#endif` generated-region layout above.

### 12.5 Allowed Rust Subset (V1)

Only this subset is accepted in the first branch (`#if RUSTYCPP_RUST ... #endif`):

1. Item forms:
   - `fn` (free functions)
   - `struct` with named fields
   - `impl` inherent methods (`impl Type { ... }`), no trait impls
   - `type` aliases that resolve to supported types
2. Type forms:
   - primitives (`i*`, `u*`, `f*`, `bool`, `char`)
   - `()` and tuples up to arity 4
   - references `&T`, `&mut T` (no explicit lifetimes in syntax)
   - `Option<T>`, `Result<T, E>`, `Vec<T>`, `String` where `T/E` are supported
3. Statements/expressions:
   - `let` bindings (with optional type ascription)
   - assignment, arithmetic, comparison, boolean ops
   - `if`/`else`, `while`, `loop`, `break`, `return`
   - method calls and associated function calls on supported runtime surfaces

Explicitly unsupported in v1:

1. `async`/`await`, generators, coroutines
2. `match` with guard-heavy or deep pattern shapes (simple literal/enum matches may be added in v1.1)
3. trait definitions/impls, trait objects, `dyn`
4. macros beyond inert attributes; no `macro_rules!` or proc-macro expansion
5. unsafe blocks and raw-pointer-heavy FFI surfaces in inline mode

### 12.6 Generation Markers and Determinism

Generator requirements:

1. Deterministic output for identical Rust payload and tool version.
2. Stable formatting profile for generated C++ regions.
3. Emitted metadata in `GEN-BEGIN`:
   - `id`
   - `version=1`
   - `rust_sha256`
4. Optional emitted metadata comment in `GEN` body:
   - transpiler version
   - generation timestamp (informational only; do not use for cache keys)
5. Inline mode does not emit `#include` directives into `GEN` regions.
   - Users own include/import management for inline units.
   - This keeps inline rewrites local and avoids generator-owned header policy.

CI policy recommendation:

1. Run generator in check mode.
2. Fail if any `GEN` region diff appears after regeneration.
3. Fail if marker pairs are malformed or duplicate `id` values exist in one file.

### 12.7 CMake Integration (V1 Contract)

The build flow is "extract/regen first, then compile normally with `RUSTYCPP_RUST=0`."

Minimal CMake pattern:

```cmake
# 1) Tool path
set(RUSTYCPP_INLINE_TOOL "${CMAKE_SOURCE_DIR}/tools/rustycpp-inline")

# 2) Inputs that may contain inline Rust blocks
set(RUSTYCPP_INLINE_SOURCES
    ${CMAKE_SOURCE_DIR}/include/foo.hpp
    ${CMAKE_SOURCE_DIR}/src/foo.cpp
    ${CMAKE_SOURCE_DIR}/src/bar.cppm
)

# 3) Regeneration target (in-place update of GEN regions)
add_custom_target(rustycpp_inline_regen
    COMMAND ${RUSTYCPP_INLINE_TOOL} --rewrite --files ${RUSTYCPP_INLINE_SOURCES}
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Regenerating inline Rust fallback C++ regions"
)

# 4) Main targets depend on regeneration
add_dependencies(my_target rustycpp_inline_regen)

# 5) Compile fallback path in normal builds
target_compile_definitions(my_target PRIVATE RUSTYCPP_RUST=0)
```

Operational notes:

1. For CI, add a check-only invocation (`--check`) before compile.
2. For module builds, include regenerated `.cppm` files in the same module pipeline as other interfaces.
3. Do not compile with `RUSTYCPP_RUST=1` in production targets for v1.

### 12.8 Translation-Unit Boundary Decision for V1

V1 mapping rule:

1. If a region requires the transpiler to automatically surface declarations into other translation units, it is out of scope and MUST be rejected with a diagnostic.
2. Users may place inline Rust blocks in `.cpp` files, but then they own boundary design: what is local vs what is manually declared/exported elsewhere.
3. For automatic discoverability by other units in v1, place shared APIs in headers or module interfaces.

Recommended diagnostic text:

```text
inline-rust-v1: automatic cross-translation-unit declaration surfacing is unsupported;
move this block to a header/module interface, or provide matching declarations manually.
```

This keeps v1 intentionally strict and prevents silent ODR/linkage regressions during incremental migration.

#### 12.8.1 Source-Owned C++ ABI Adapters

Inline module carriers may preserve a narrow, source-owned legacy STL boundary with inert Rust attributes:

- `#[cfg_attr(any(), cpp_abi(...))]` on a free function or static inherent method
- `#[cfg_attr(any(), cpp_abi_alias(std_vector))]` on a public `Vec<f64>` alias

The supported adapters are deliberately closed: `std_string_bytes` maps by-value `Vec<u8>` parameters and returns to `std::string`, while `const_ref(Alias)` maps `&[f64]` to `const std::vector<double>&`. A marked vector alias and its `const_ref` consumer MUST be in the same block. Unsupported attributes, shapes, uses, or placements fail closed.

An adapted callable and every block that calls it MUST be in the same physical carrier and the same full immediate `export namespace` scope. A direct call may target a provider in the current block or an earlier block only. Backward or cross-file references, function values, qualified/imported/macro-mediated uses, ambiguous names, and lexical shadows are rejected. This ordering rule does not add cross-block type inference.

The carrier MUST contain exactly one unconditional top-level `export module` declaration. Exact unconditional top-level `import std;` and `import rusty;` declarations MUST follow it and precede every participating block. If `<rusty/rusty.hpp>` is also included, it MUST be an exact global-module-fragment include: `module;` is the first meaningful host construct, followed by the include and then the export-module declaration. Conditional, macro-produced, or otherwise ambiguous host prerequisites are rejected.

Generated semantic and conversion helpers are `inline`, carrier/module-uniquely named implementation details; conversion support is emitted once in its owning block. Public ABI facades remain ordinary non-inline definitions so their legacy strong symbols are preserved. Host code remains responsible for its own declarations and imports.

For a multi-file command, adapter preflight and rendering complete for every input before any file is atomically replaced; `--check` runs the same validation without writing. `--emit-rust --block-id` includes the required earlier-provider dependency closure in deterministic dependency-first order. Marker-free carriers retain the legacy byte path.

#### 12.8.2 Private Imports in a Shared C++ Namespace

An adapter crate may import unadapted public leaves from one crate child whose
C++ module exports into the same flat namespace:

```rust
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::{randgen_rand_max, randgen_rand_raw};
```

This is a source-owned exception to the ordinary sibling-module namespace
mapping. The accepted form is exact and closed:

1. The attribute MUST be the sole, inert
   `#[cfg_attr(any(), cpp_import_namespace(NS))]` attribute on a private
   `use` item. `NS` is a canonical, non-raw C++ namespace path and MUST exactly
   equal the active `--cxx-namespace` (crate mode) or the block's full immediate
   `export namespace` path (inline mode).
2. The use tree MUST be `crate::<one-child>::Name` or a nonempty group of
   simple, unique `Name` leaves. Public uses, glob/`self` imports, renames,
   deeper paths, raw identifiers, C++ keywords, active or malformed markers,
   companion attributes, and macro-assembled markers are rejected.
   An imported leaf is reserved in its Rust module: another item, ordinary
   `use`, glob, generic, or lexical binding may not shadow it, and opaque
   attribute or macro tokens may not mention it. Every shadow/collision check
   compares the shared escaped C++ spelling as well as Rust syntax, so a raw
   Rust keyword cannot alias a permitted suffixed leaf (for example
   `r#static` versus `static_`).
3. Crate mode requires `<one-child>` to be exactly one physical generated
   crate-root module (an inline `mod` is not sufficient) and retains that exact
   root-child named-module dependency before the namespace opens; a nearer
   same-named nested module never substitutes for `crate::<one-child>`. Inline
   mode requires the exact private, unconditional top-level sibling import
   before the marked block (for example `import rrr.rand;`). From the sole
   `export module` terminator through the participating `export namespace`
   opener, only complete unconditional private literal named-module imports
   are accepted. Missing, late, conditional, exported (`export import`),
   differently named, macro-assembled, or otherwise prefixed imports fail
   closed. A second exported import of the provider anywhere in the carrier is
   forbidden even when the required private import is also present;
   preprocessor `export`/`import` assembly and token-pasting are rejected.
4. The generated C++ deliberately emits no `using` declaration or namespace
   alias. The imported declaration is already visible to unqualified calls in
   the validated shared namespace; adding a `using` inside an exported
   namespace would incorrectly re-export Rust's private import through the
   consumer module.
5. The crate-mode provider leaf MUST be a direct, exact-public,
   unconditional, ordinary non-generic free function at the physical child's
   source root. Missing/private leaves, constants, types, re-exports, adapted
   functions or owners, and other callable shapes are rejected before output.
   The ordinary whole-crate sibling audit remains authoritative, and this
   contract does not authorize cross-crate adapter calls. (Inline mode instead
   validates the exact host module import because the provider's Rust source is
   not part of the carrier.)
6. The leaf may be referenced only through the marked module's unqualified
   Rust binding. Qualified provider/binding paths, unmarked imports or provider
   aliases, and references from sibling or descendant modules are rejected in
   crate preflight. Imports or aliases of a marked consumer module—or any of
   its module ancestors—are also rejected, closing descendant qualified-access
   aliases. While a flat contract exists, `crate`/`self`/`super` root aliases
   and `extern crate self` aliases are rejected so those checks cannot be
   bypassed. Unrelated ordinary aliases remain permitted.
7. Every physical source root emits into the same explicit C++ namespace.
   Therefore a source-root item or use binding outside the exact provider may
   not have an imported leaf's escaped C++ name, and source-root glob imports fail closed;
   otherwise an ambient declaration could collide with or hide the provider.
   Within the provider, only the validated direct leaf function may have that
   C++ spelling; a second raw/item/use/foreign spelling collision is rejected.
   Lexical bindings and declarations inside a genuine nested module namespace
   remain permitted outside the marked module. Block-local functions,
   constants, statics, and non-generic types also remain permitted. At a
   physical source root, a colliding block-local generic type, impl-forced
   type, or static referenced by such a type fails closed because code
   generation may namespace-hoist it or otherwise cannot safely lower it as a
   block-local C++ declaration. The same local names inside a genuine nested
   Rust module remain in that module's nested C++ namespace and are permitted.
8. Inline references MUST additionally remain in the physical block that
   contains the marked `use`. This makes `--emit-rust --block-id` closed by
   construction: selecting an unrelated block neither adds an unused import nor
   silently emits Rust with a missing binding. Cross-block and nested-module
   mentions are rejected during carrier preflight. Because generated calls are
   deliberately unqualified, every imported leaf identifier is also reserved
   in host C++ outside comments, literals, and the replaceable Rust/GEN regions;
   host declarations, aliases, macros, and preprocessor tokens cannot capture
   it.

A flat-import-only block does not request string/vector adapter support,
semantic helpers, `import rusty;`, or earlier-provider dependency closure.
Marker-only files still activate crate/inline preflight; sources without any
reserved marker retain the exact legacy fast path. Direct single-file named
module transpilation is unsupported because it has neither the crate physical
module census nor the prepared inline host-import proof.

### 12.9 Assignment Expressions, Unit Semantics, and Statement Peephole

Rust assignment and compound-assignment are expressions, but their value is always unit `()`.

Examples in Rust:

```rust
let mut x = 1;
let u = (x = 5);    // u: ()
let v = (x += 2);   // v: ()
```

In C++, assignment operators return the assigned lvalue, so the transpiler must preserve Rust unit semantics explicitly in value position.

Value-position lowering keeps a unit wrapper:

```cpp
[&]() { static_cast<void>(x += 2); return std::make_tuple(); }()
```

This wrapper guarantees:

1. side effects happen once,
2. the expression type remains Rust unit (`std::tuple<>` in runtime model),
3. nested expression typing remains consistent.

For statement-only position, the return value is unused. The transpiler applies a peephole optimization and emits a plain statement:

```cpp
x += 2;
```

This reduces noise without changing semantics, while value-position forms continue to use the unit wrapper.

## 13. Type Inference Engine

### 13.1 Why this chapter exists

Repeatedly across the parity matrix we hit a single shape of failure: Rust code transpiles into C++ that asks the C++ compiler to do work it cannot do. The C++ deduction machinery looks at one expression at a time; Rust's idiomatic patterns expect the type checker to thread information across many. When the transpiler emits the literal shape of the Rust expression without resolving the threading, the result fails CTAD or template-argument deduction at the C++ side.

This chapter is the architectural answer: the transpiler must contain a real inference engine, and it must run before emit. Emit then becomes type-direct, not type-hopeful.

### 13.2 Root cause: Rust has inference, C++ has deduction

These are not the same operation.

**Rust's type checker is a constraint solver.** It walks an expression (often an entire function), assigns a fresh type variable to anything whose type isn't immediately known, collects equality constraints from every use, and runs unification to a fixpoint. Information flows in both directions: from arguments outward, *and* from later uses back into earlier expressions. The classic case:

```rust
let xs = Vec::new();   // xs: Vec<?T>
xs.push(42_i32);       // constraint: ?T = i32
                       // xs is solved as Vec<i32>
```

The `Vec::new()` call site by itself cannot determine `T`. The `push` call later contributes the constraint that pins it. Rust's inference engine sees both, unifies them, and the program type-checks.

**C++'s template argument deduction is a per-call-site pattern match.** At each instantiation, the compiler looks at the actual argument types and tries to match them against the template's parameter signature. If a parameter doesn't appear in the signature, or appears only partially, deduction fails *immediately* at that site — there is no facility to leave an unknown and have a later expression supply it. There is no notion of "wait and see"; there are no type variables that span call sites.

The four families of bug this produces:

1. **Cross-arm unification** — ternary / `match` where each arm fixes a different subset of the parameters. Rust unifies across arms; C++ computes a common type per arm and fails when arms only see partial information. The canonical case is `either`: `cond ? Either{Left{a}} : Either{Right{b}}` — see §13.3.

2. **Backward flow into earlier sites** — `let xs = collect()` whose element type comes from a later `.push()` or a downstream consumer. Rust gathers the constraint backward; C++ sees `collect<?>()` with no way to fill `?`. This bites *only* when the element type isn't already on the surface of a known type at the call site: an ordinary `Vec<i32>` field carries `i32` in its own spelling, so emit reads it straight off the container and no backward flow is needed. The case that genuinely needs the engine is an **opaque container** — serde's `ByteBuf`/`ByteArray<N>` deserialize — whose element type (`u8`) survives only in the consuming `push`/assignment. This is the live `#36` instance worked through in §13.13.

3. **Multi-return closure unification** — `|x| match x { A => None, B => Some(y) }` returning `Option<T>`. Rust unifies the two arms; C++ deduces the lambda's return from the *first* return (often `None_t`) and rejects the second. We hit a specific instance of this with `RUSTY_TRY_OPT`-inside-lambda; the commit annotating an explicit return type is the local-fix flavor of what a real engine would handle generically.

4. **Target-driven deduction failure** — `Content_Bytes{rusty::to_owned(value)}` where the *field* type tells you what `to_owned` should produce, but C++ deduces `to_owned`'s return *first* and then asks if the resulting type fits. Rust uses the field type as a constraint that flows backward into `to_owned`'s return.

All four are symptoms of one architectural gap. The transpiler emits Rust *syntax* on the assumption that the C++ compiler will reconstruct the inference. It won't.

### 13.3 The canonical case in detail: `either`'s ternary

The Rust:

```rust
let reader = if use_empty {
    Either::Left(cursor_a)
} else {
    Either::Right(cursor_b)
};
```

`Either<L, R>` is declared with two type parameters. `Left(L)` only mentions `L`; `Right(R)` only mentions `R`. Rust's solver assigns `?L` and `?R` as fresh variables, sees:

- arm A's expression has type `Either<typeof(cursor_a), ?R>` — pins `?L`
- arm B's expression has type `Either<?L, typeof(cursor_b)>` — pins `?R`
- both arms must produce the same type (it's an `if/else`)

Unification: `?L = typeof(cursor_a)`, `?R = typeof(cursor_b)`. Solved.

The emit lowers each `Either::Left(a)` to `Either_Left<L, R>{a}` and the outer `if/else` to a ternary. With the original two-param emit, every `Either_Left{a}` fails CTAD individually (Layer 1: `R` doesn't appear in the constructor). Trimming the params (Approach A in §1 of `TODO-misc.md`) fixes Layer 1 but exposes Layer 2: the ternary's two arms each fix one param, and C++ has no machinery to unify them across the `?:`. Layer 3 is that `using` aliases don't accept user-provided deduction guides anyway.

A proper inference engine in the transpiler solves Layer 2 directly: it runs Rust's unification at codegen time, computes `<L, R>` from the union of the arms, and emits `either::Left<L, R>(cursor_a)` and `either::Right<L, R>(cursor_b)` with explicit arguments. C++ now has no deduction left to do — the types are written out.

### 13.4 Design goals

The engine has to be:

- **Sound** — never produce a substitution that contradicts a constraint. If the Rust source type-checks, the engine reaches the same solution Rust would; if it can't, it falls back cleanly to local deduction.
- **Partial** — it does not need to be a full Rust type checker. It runs on already-type-checked Rust (we have `syn::Type` annotations from the source). Its job is to *propagate* known types across the expression graph, not to rediscover them from scratch.
- **Localized** — confined to a single function body per run. Cross-function inference is what Rust handles via signatures; the transpiler already reads those signatures. We don't need whole-program inference.
- **Decidable and fast** — the rule of thumb is one unification pass per function plus one occurs-check per constraint. No exotic features (HKTs, dependent types).
- **Composable with current emit** — the engine runs *before* emit and decorates the AST with resolved types. Emit reads the decoration and produces concrete C++. We do not retrofit emit to be inference-aware on a case-by-case basis; we resolve once and emit type-directly.

### 13.5 Architecture

Three layers.

**Term language.** A `TyTerm` is either:
- a concrete type (`syn::Type`, but normalized — we strip `Box<…>`, `&`, parens, etc. into a canonical form),
- a type variable `?n`,
- or an applied constructor `Ctor(args…)` where `args` are themselves `TyTerm`s — this lets us represent partially-known types like `Vec<?T>` or `Either<i32, ?R>`.

This is the standard ML/Rust inference shape; no novelty. Crucially, `Ctor` here covers Rust path types (including user-defined enums), tuple types, slice types, and reference types — anything the Rust source can write.

**Constraint store.** A flat `Vec<Constraint>` where `Constraint::Eq(TyTerm, TyTerm)` is "these two terms must unify." We never need disjunctive constraints (subtyping in Rust is structural via lifetimes, which we handle separately; the C++ side doesn't model lifetimes anyway, so we elide them at this layer).

**Substitution.** A `HashMap<VarId, TyTerm>` mapping type variables to their resolved terms. Applied transitively until fixpoint.

The solver is textbook Robinson unification with occurs-check:

```
unify(σ, T, U):
    T'  ← σ(T)
    U'  ← σ(U)
    case (T', U'):
        (Var v, Var v')        if v == v' → σ
        (Var v, _)             if v ∉ U'  → σ ∪ {v ↦ U'}
        (_,     Var v')        if v' ∉ T' → σ ∪ {v' ↦ T'}
        (Ctor c args, Ctor c' args') if c == c' and |args|==|args'|
                               → fold unify over zip(args, args')
        (Concrete A, Concrete B) if A == B → σ
        otherwise              → fail (caller falls back)
```

Failure does not abort the transpile. The engine simply records "could not resolve" for that node, and emit reverts to the current best-effort local CTAD path. Over time, every fallback is a candidate bug to investigate.

### 13.6 Constraint collection

A single AST walk before emit. The walker visits each expression and adds constraints:

- **`let x = e;`** — if `x` has an annotation, constrain `typeof(e) = annotation`. Otherwise allocate `?x_ty`, constrain `typeof(e) = ?x_ty`, and store `?x_ty` as the binding's pending type.
- **Function return** — constrain the body's tail expression to the declared return type.
- **`if … { a } else { b }` and `match`** — allocate `?merge`; constrain `typeof(a) = ?merge` and `typeof(b) = ?merge`. This is the case that solves Either.
- **Closure return** — allocate `?ret`; constrain every `return e;` inside the closure and the tail expression to `?ret`. The closure's type becomes `Fn(…) -> ?ret`.
- **Field/element/index access** — propagate the container's type to constrain the access result.
- **`Vec::new()`-style calls with element from later use** — the call site allocates `?T` for each unbound type parameter of the function. Later `.push(x)` constrains `?T = typeof(x)`. This is the backward-flow case.
- **Brace-init `Struct{field: e}`** — constrain `typeof(e) = field's declared type`. This is the target-driven case that fixes `Content_Bytes{to_owned(value)}`.

Each constraint records the AST node it came from so the engine can attribute a failure if it occurs.

The walker does *not* recompute types we already have: when `syn` already gives us a fully concrete type at a node (e.g. `42_i32`), the constraint is `Concrete(i32) = Concrete(i32)` — trivially satisfied. The engine only does real work where types are partial.

### 13.7 Emit becomes type-directed

After the solver finishes, every interesting AST node carries a resolved `TyTerm`. Emit then:

- **For `Either::Left(a)` inside a ternary** — the parent expression's resolved type is `Either<L, R>` (concrete). Emit `either::Left<L, R>(a)` with explicit args. No CTAD needed.
- **For `Vec::new()` whose `?T` was resolved to `i32`** — emit `rusty::Vec<int32_t>::new_()` instead of leaving `<T>` for C++ to figure out.
- **For closure returning `Option<T>`** — emit `[&]() -> rusty::Option<T_cpp> { … }` directly.
- **For `to_owned(value)` whose target field is `rusty::Vec<u8>`** — emit `rusty::Vec<u8>::from_iter(value)` directly.

When the engine fails to resolve (genuinely under-constrained Rust, which Rust itself would reject, or a constraint pattern we haven't taught the engine), emit reverts to today's heuristics. This keeps the transpiler shipping while the engine grows.

### 13.8 What the engine is *not*

It is not a replacement for Rust's borrow checker or trait resolution. Those are upstream — by the time we run, the input has already type-checked under `rustc`. We are only re-running the *type variable propagation* layer to get the values Rust would have computed.

It is not a region inference engine. Lifetimes don't survive into C++ in this codebase; the engine treats `&'a T` and `&'b T` as both `&T` for unification purposes.

It is not a Hindley-Milner *let-generalization* engine. We don't need to generalize `let f = |x| x` into a polymorphic schema — Rust closures are monomorphic per use site, and the transpiler already handles monomorphization via per-instantiation emit.

### 13.9 Scope and phasing

Phase 1 — scaffolding. Add the `TyTerm` / `Constraint` / `Substitution` data model in `transpiler/src/codegen/inference.rs`. Construct an `InferenceContext` at the entry of every `emit_fn_body`. No new behavior; just plumbing and tests for the data model.

Phase 2 — constraint collection. Walk the body once before emit and populate the constraint store. Record but don't yet act on the results. Add a debug mode that prints the collected constraints for golden-test inspection.

Phase 3 — unification solver. Implement Robinson with occurs-check. Make it iterate to fixpoint on the collected constraints. Validate against hand-written examples (including the Either ternary, the Vec::new()+push pair, the `to_owned`-into-field pattern).

Phase 4 — wire to emit. Three high-value sites first:
  - the Either ternary path (§1 of TODO-misc.md),
  - the `to_vec` / `to_owned` brace-init path (§4 of TODO-misc.md, line 8607),
  - the multi-arm closure return type (we currently patch this site-by-site with explicit return annotations; replace with engine-driven).

Phase 5 — validation. Run the parity matrix. Expected: `either` flips to PASS, `serde` / `serde_bytes` advance past their respective deduction errors. Regression check the 11 currently-passing crates. Any case still falling back to local CTAD becomes documented and prioritized.

### 13.10 Acceptance criteria

The engine is "real" when:

- the Either ternary in §13.3 emits with explicit `<L, R>` and compiles,
- the `Content_Bytes{to_owned(value)}` brace-init resolves backwards from the field type and compiles,
- removing the manual `-> rusty::Option<…>` annotation from the checked-arithmetic lambda emit (the local fix landed in commit 5b6698f) still produces compiling code, because the engine derives the same annotation generically,
- no regression on the 11 currently-passing matrix crates,
- the engine's resolved types are visible in a `--print-inference` dump so reviewers can audit what it decided.

The engine has *failed cleanly* (i.e. fallen back, not crashed) when a constraint set has no solution — this should be observable via the debug dump and counted in the matrix telemetry. Frequent fallbacks are the next round's bug list.

**Status as of Phase 5 validation (commits 8eb9a6d → e2986e8):**

| Acceptance criterion | Status |
| --- | --- |
| Phases 1–4 in place (data model, collector, solver, CodeGen bridge) | ✅ landed |
| Engine resolves the §13.3 Either ternary into `App("Either", [?L, ?R])` with distinct slots | ✅ verified by `infer_branch_merge_either_ternary_resolves_through_constructors` |
| Engine never crashes on under-constrained input — returns `None` and lets emit fall back | ✅ verified by `try_infer_ternary_arm_type_*` |
| `CodeGen::try_infer_ternary_arm_type` consumable from any emit site | ✅ landed |
| Either / serde / serde_bytes flip to PASS on the matrix | ⏳ blocked on variant-template-trim (TODO §1 / approach A) — engine is ready, emit-side rewrite isn't |
| `--print-inference` debug dump | ⏳ deferred to a follow-up; today's solver records errors but doesn't surface them |
| No regression on the 11 PASS crates | ✅ Phase 5 matrix run confirmed 11/15 stable |

The engine is "real-enough" for the criteria that don't require an emit-site rewrite. The remaining matrix flip needs the variant template trim plus a conditional emit consumer; both are emit-pipeline work, not engine work, so they're queued under the existing TODO-misc.md §1 / §4 entries rather than as new engine phases.

**Update (2026-06 — serde_bytes serialize landed; deserialize is the remaining inference case).** The serialize-side deduction blockers for serde_bytes were resolved *without* the engine — explicit dispatcher return types, cross-module dispatcher unification, and a defined `<Trait>Traits` primary (commits e59945d → f0c1d56). That is expected: serialize *dispatches on a value whose type is already known* (forward flow), so name resolution suffices. serde_bytes' *deserialize* targets (`test_derive` / `test_serde`) remain skipped on exactly the backward-flow case this chapter exists for — recovering `next_element`'s element type from later use (§13.13, `#36`). The engine (Phases 1–4) is the intended home for it but is not yet wired to the `next_element` turbofish slot; today emit falls back to a byte-name heuristic that types the local `Vec` but never reaches the call.

### 13.11 Relationship to existing local fixes

Several commits already land local fixes for symptoms of this gap:

- 5b6698f — explicit `Option<T>` return on checked-arith lambdas
- 2cc52b6 — `variant_holds<E_Tag>` via `variant_ctx.enum_name`
- de9d340 — slice `to_owned` / `to_vec` routed through `Vec::from_iter`

These are *not* wasted work. Each documents a specific constraint shape the engine will eventually handle generically. When the engine lands, each local fix becomes a regression test: "the engine on this AST should produce the same emit as the local fix did." Local fixes shouldn't be reverted aggressively — they're cheap insurance against the engine missing a case.

### 13.12 Open questions deferred to implementation

- Where to canonicalize `syn::Type` (during collection, lazily on lookup, or both).
- Whether to model `Self` as a distinct constructor or as a substitution-on-entry.
- How to integrate with the existing `infer_simple_expr_type` helper — replace it, or use it as the engine's "give me a starting point" oracle.
- Whether constraint failures should warn during development builds even when the fallback succeeds (probably yes; gated by a flag).

These are implementation choices, not design choices. The choices above lock the *architecture*: a single inference pass, runs before emit, talks to emit by decorating the AST with resolved types, falls back cleanly on failure.

### 13.13 The live deserialize instance: serde's hidden element type (#36)

The most current worked case for the engine is serde sequence deserialize. It shows precisely where the backward-flow gap bites — and, just as importantly, why *most* deserialize escapes it.

A visitor pulls elements one at a time and builds a container:

```rust
let mut bytes = Vec::new();
while let Some(b) = seq.next_element()? {   // next_element::<?>()
    bytes.push(b);                          // b flows into bytes  ⇒  ? = element(bytes)
}
Ok(ByteBuf::from(bytes))
```

`next_element` is generic over its *output* `T`, and `T` appears in no argument — so C++ deduction cannot recover it (this is §13.2 case 2). The call has to be spelled `next_element<uint8_t>(seq)`.

**Why most deserialize already works — the container spells the element.** For an ordinary field of type `Vec<i32>`, the visitor's output type *is* `Vec<i32>`, and `Vec<i32>` names its element on its surface. The existing expected-type plumbing carries `Result<Vec<i32>, E>` to the call, extracts `i32` from `Vec<i32>`, and emits `next_element<i32>`. No constraint-solving is needed: the element is readable straight off the container type. This is why ordinary seq/map deserialize passes across the matrix without the engine.

**Why bytes break — the wrapper hides the element.** serde_bytes' outputs are *opaque* wrappers — `ByteBuf`, `ByteArray<N>`, `Bytes`. The element is `u8`, but `ByteBuf` does not look like `Vec<u8>` at the type level, so "extract element from the container type" returns nothing (`extract_option_inner_type_for_hint(ByteBuf) = None`). The only remaining witness for `u8` is the *consumption* — `bytes.push(b)` / `*byte = …` — which is exactly the backward constraint of §13.6 ("element from later use") that C++ cannot gather.

**Two tiers — and the shortcut to retire.** There are two ways to supply the `u8`:

1. *Name/shape heuristic* (interim, fragile). Special-case the byte wrappers (`ByteBuf → u8`, `ByteArray<N> → u8`), or guess from a `byte`-named local. The transpiler does the latter today (`inference.rs:9233`: `local_lower.contains("byte") → uint8_t`) — but only to *type the local `Vec`*; it never threads that `u8` to the `next_element` call. This is precisely the name-driven shortcut the engine is meant to replace: correct for serde_bytes, generalizing to nothing.
2. *Engine-driven backward flow* (proper). Collect `element(typeof(next_element())) = typeof(b)` from `bytes.push(b)`, unify with `typeof(b) = element(typeof(bytes))`, resolve `u8`, and decorate the call so emit spells `next_element<uint8_t>`. This generalizes to any opaque-wrapper sequence deserialize, not just bytes.

**Where it plugs in.** The plumbing already exists: the call routes through a turbofish slot (`::de::rusty_ext::next_element<T>(…)`, `mod.rs:24826`) and an injection point in the UFCS dispatch (`emit_expr.rs ~5035`). What is missing is the *value* of `T` — the engine must resolve it from the `push`/assignment target and decorate the node. The two surface shapes differ in difficulty: `while let Some(b) = next_element()? { … push(b) }` exposes the target one statement away, while `*byte = next_element()?.ok_or(…)?` buries it behind `ok_or_else` / `?`, so the constraint must survive those hops (a `?`-and-`ok_or`-transparent propagation rule in the collector).

**The serialize/deserialize asymmetry — why one needed the engine and the other didn't.** Serialize *dispatches on a value whose type is already known*: information flows **forward**, which the name-resolution machinery (Chapter 14) handles. Deserialize must *produce a value of a type recovered from how the result is later used*: information flows **backward**, which only this engine supplies. `#36` splits cleanly along the two chapters: the element-type recovery above is Chapter 13's. Its sibling — the `next_value_seed` ambiguity, where the same `(A&, V)` signature is emitted with two return *spellings* (`V::Value` vs `VisitorTraits<V>::Value`, because the associated name `Value` is co-owned by `Visitor` and `DeserializeSeed`) — is a *spelling*/qualification problem and belongs to Chapter 14, not here.

### 13.14 Phase 4 implementation plan (type-directed emit)

§13.9 phased the work and §13.10 fixed the acceptance bar; this section is the concrete plan for **Phase 4 — making the solver authoritative at emit time**, written after an audit of the engine and the emit seam (June 2026).

**Why this section exists.** As of mid-2026 the engine is *staged, not live*. Phases 1–3 are real (Robinson `unify` + occurs-check, `Substitution`, `InferenceContext::solve` to fixpoint, `resolve`), and Phase 4a builds one `InferenceContext` per function in `emit_function` (emit_items.rs:144–177) and stores it on `self.inference` (mod.rs:630). But **no consumer reads `self.inference`**: the only live queries (`infer_branch_merge`, `infer_local_owner_element_from_block`) re-solve transiently, and `try_infer_ternary_arm_type` (inference.rs:28) is called only from tests. Meanwhile every real type decision is still made by a growing pile of forward-scan heuristics (the `augment_*_local_type_hints_*` passes in mod.rs, plus the per-shape patches: bytebuf newtype-consumer, `ok_or` receiver threading, `*x` deref-seed). Each new shape adds a pass. The serde_bytes deserialize tail (§13.13) is the symptom: three nearly-identical opaque-wrapper cases (ByteBuf / ByteArray / Cow) that each want their own heuristic. Phase 4 ends the treadmill by making emit ask the solver first.

**Invariants (carry forward from §13.8/§13.10).** (1) *Engine-first where wired, heuristic-fallback, never-crash* — a `None` or unresolved variable falls back to today's path unchanged. (2) *Flagged rollout* — `RUSTY_CPP_INFER_ENGINE` (mirrors the UFCS flag precedent, Ch. 14 / §UFCS); graduated to **default-on, opt-out** (`=0/off/false/no`) after a clean flag-on matrix. (3) *Matrix-gated, freeze-and-grow* — every step ends with the parity matrix no worse than today. **The retire-not-add stance was abandoned (June 2026) and replaced by freeze + grow + reconcile** (see §13.14.1): the `augment_*` heuristics are *frozen* (no new ones — new shapes go to the engine only), the engine *grows* one C-rule at a time as a fill-only fallback, and a heuristic is *superseded* only when disagreement-telemetry proves the engine subsumes it without regression — never blind-deleted. The earlier "augment_* count strictly lower per step" target proved wrong: the heuristics are broad (one pass covers several shapes) while each C-rule covers a slice, so deletion regresses (empirically: deleting `augment_local_generic_placeholder_hints_from_function_calls` broke 7 unit tests).

**A. Close the engine I/O boundary (infra prereqs).**
- *A1 — public `TyTerm → syn::Type`.* `tyterm_to_syn_type` (type_solver.rs:1115) is private and `resolve()` returns `TyTerm`; emit consumes `syn::Type`. Promote a `pub(crate) tyterm_to_syn(&TyTerm) -> Option<syn::Type>`.
- *A2 — stable AST-node → `TyVarId` identity (the linchpin).* `proc-macro2` spans on expanded input are not usable map keys and local *names* shadow. Use the **positional pre-order index** trick: the collector already walks the body in order — have it stamp each constrained `let` (later, each constrained expr) with a deterministic pre-order counter and record `index → TyVarId`. Emit walks the *same* body in the *same* order and looks up by its own counter. No spans, no shadowing ambiguity. First cut indexes `Local` statements only (covers `emit_local`, the dominant seam).
- *A3 — consolidate onto the per-function context.* Make queries consult the already-built `self.inference` instead of re-solving transiently; this removes the duplicate solve and guarantees one consistent solution per function.

**B. The oracle seam (one funnel, flagged).** `emit_local` (emit_stmt.rs:1542) is *the* seam: it resolves a local's type via `infer_local_binding_type_from_initializer` (1599) → `infer_simple_expr_type` (1607) → `infer_local_type_from_placeholder_hint` (1610). Add `engine_type_of_local(let_index) -> Option<syn::Type>` (= `self.inference.resolve(var) → tyterm_to_syn`) as the **first** source, everything else fallback, behind `RUSTY_CPP_INFER_ENGINE`. (Extend the same funnel into `infer_simple_expr_type`, inference.rs:5431, once A2 indexes exprs.)

**C. Widen the collector — one rule per shape, each as a fill-only fallback.** Add the rule + a unit test → matrix. The "Supersedes" column names the heuristic the rule is *aiming* to cover, but the heuristic is **not deleted on landing** — it stays frozen until disagreement-telemetry (§D) proves the engine resolves every site the heuristic did, identically, across the matrix. Until then both run, engine consulted only where the heuristic returns nothing (the fill-only seam).

| # | Constraint rule (in `ConstraintCollector`) | Supersedes (via telemetry, not on landing) | Unblocks |
|---|---|---|---|
| C1 | Struct/variant construction: `Struct{f: e}` ⇒ `typeof(e)=field_ty`; `Wrapper::from/new(x)` ⇒ `x=field_ty`. (`StructFieldInit` origin already exists at type_solver.rs:118, never pushed.) | `augment_vec_local_element_hints_from_newtype_consumer` (already deleted — first slice) | bytebuf **+** bytearray array-field **+** Cow, from one rule |
| C2 | Call signatures: resolve callee → unify args↔params, result↔return. Generalize the `ItemResolver` callback into a `SignatureResolver`/`MethodResolver`. | the call/method-RETURN→owner-element slice of `augment_local_generic_placeholder_hints_from_function_calls` | the long tail |
| C3 | Assignment + deref-assign: `x=e`, `*x=e` ⇒ `deref(typeof(x))=typeof(e)`. | the `*x` deref-seed patch | bytearray `*byte = …` |
| C4 | `?` / `ok_or` / `ok_or_else` / Option–Result flow. | the `ok_or` receiver-threading patch | bytearray/bytebuf `?` chains |
| C5 | for-loop binding from iterator item, incl. `.iter_mut()`/`.enumerate()` tuple patterns. | for-binding gap; `collect_repeat_element_type_hints` | `for (idx, byte) in …` |
| C6 | Generic-path/turbofish backward (`next_element::<T>()`, `T` in no arg) — connect the call to the solved var. | byte-name fallback (inference.rs:9233) + serde turbofish plumbing | serde_bytes #36 cleanly (§13.13) |

After C1–C5 the engine *can* resolve bytebuf/bytearray/Cow from the *same* constraint set (field-init + assignment + for-iter + `?`) rather than three bespoke passes — the point of the chapter. Whether the corresponding heuristics are then retired is a telemetry decision (§D), not automatic on landing.

### 13.14.1 Soundness model and the freeze-grow-reconcile policy

The earlier plan said "retire each heuristic as its C-rule lands, augment_* count strictly down." Two findings overturned it:

**The heuristics are broad; the C-rules are slices.** A single `augment_*` pass covers several unrelated shapes (e.g. `augment_local_generic_placeholder_hints_from_function_calls` resolves map-`insert(K,V)`, assignment RHS, arg-direction, *and* method-return-inferred locals). A C-rule covers one shape. Deleting the pass when one slice lands regresses the others — empirically, deleting that pass broke 7 unit tests (OnceBox new+set, map-new from next_value/next_entry, while-let next_element push payload, Result-Ok-moves-local). So **count-down-per-step is the wrong target**; it forces premature deletion.

**Neither path is provably sound; the question is where the unsoundness lives.** The engine's *kernel* — Robinson unification + occurs-check — is sound: given correct constraints it produces the most general unifier or fails. The unsoundness is in the **periphery**:
- *Resolvers* answer by fuzzy name-based lookup (`call_path_candidates`, receiver-blind `lookup_unique_method_return_type_by_name`). A wrong resolution feeds a wrong fact to a sound kernel → wrong type, confidently.
- *Collection* is lossy: the C1 rule `Wrapper::from(x) ⇒ typeof(x) = field_ty` is **unsound by construction** — it is only correct for *newtype* `from` (identity wrapping); for a *converting* `from` (`Foo::from(bar)` where `Foo`'s field is some `T` built from `bar`) it is simply false. It is merely *lucky* on serde_bytes, whose `from` is a true newtype. Tightening C1 to demand `typeof(x) = param_type_of_from` would be sound but would *lose* the serde_bytes case (the param is generic `T: Into<…>`), so the rule is deliberately kept loose and fill-only.

The heuristics are equally best-effort — they were written shape-by-shape against observed code, never proven. So we cannot rank "engine sound, heuristic unsound" or vice-versa in the abstract.

**Therefore: freeze + grow + reconcile.**
1. *Freeze* the heuristics. No new `augment_*` passes or per-shape patches. Every newly-encountered "type known only from later use" shape is added to the **engine** (a C-rule), never to the heuristic pile. This stops the treadmill without risking the working set.
2. *Grow* the engine as a **fill-only fallback** — consulted only where the frozen heuristics produce nothing (the `if hints.contains_key(&name) { continue; }` seam in `augment_owner_local_type_hints_from_solver`). A new C-rule can therefore only *add* coverage; it cannot change an answer the heuristics already gave, so it cannot regress the matrix.
3. *Reconcile* via **disagreement-telemetry** (§D, reframed). Run engine and heuristic side-by-side on every site and log: (a) sites where both fire and *agree* (candidates to retire the heuristic), (b) sites where both fire and *disagree* (a real bug in one — investigate, this is how we find unsound heuristics *and* unsound resolvers), (c) sites where only one fires (coverage gaps). A heuristic is retired only after telemetry shows category (a) covers all its sites with zero (b). This makes retirement evidence-driven, never a blind `git rm`.

**Conservative-resolver principle.** When a resolver cannot answer confidently, it must return `None`, not a guess — the engine then leaves the variable unresolved and the (frozen, fill-only) heuristic or the existing fallback handles it. Trading coverage for soundness at the resolver boundary is always the right call here, because the kernel amplifies whatever the resolver asserts. Fails-safe-on-conflict is already automatic: a `unify` error collapses the inference to `None` → fallback, never a crash, never a wrong forced type.

> **Landed (reconcile telemetry + the narrow-seam finding, June 2026).** `RUSTY_CPP_PRINT_INFERENCE` is implemented (opt-in, default-off, logging-only; a `print_inference` field + `set_print_inference`, a pure `classify_inference_reconcile` with 6 unit tests, instrumented at the owner-element seam). It compares the heuristic and engine answers per owner-`Vec`-local and emits `[infer-reconcile] {AGREE|DISAGREE|ENGINE-ONLY|HEURISTIC-ONLY}` to stderr. The verdict is computed on the *normalized element* — the hint map carries two equivalent representations (engine stores `Vec<T>`, heuristics store bare `T`, consumer reads both via `extract_vec_element_type_for_hint(...).unwrap_or(self)`), so a raw-string compare would falsely DISAGREE on every owner-Vec-local. A synthetic probe (`v.push(1u8)`, `w.push(make()→u8)`) shows both AGREE — the first empirical check of C1/C2 against the heuristics.
>
> **The strategically important finding:** running the telemetry over the real matrix crates (serde_bytes, arrayvec) produced **zero** comparisons. serde_bytes' expanded targets contain *no* `Vec::new()` sites at all (its deserialize uses visitor methods + a fixed `std::array`, never the `Vec::new()`-element-from-usage shape), and large expanded functions are gated out of the augment pipeline anyway (`stmts.len() <= 128`, emit_expr.rs:532). So the engine's *current* consumption seam — the owner-`Vec::new()` local — is a **backwater on real code**; it fires mostly in small synthetic unit-test functions. This refines what "grow the engine" must mean: the lever is **new emit consumer seams** (C3 deref-assign, C5 for-iter, C6 turbofish), *not* more collector rules feeding the one narrow owner-Vec seam. The owner-Vec seam was the correct *first* slice — fully understood, fill-only, safe — but adding C-rules that only feed it will not move the matrix. The next growth step should therefore open a seam that fires on real code: the serde_bytes #36 chain (C5 types `byte`, C3 makes `*byte = e` set `typeof(e)`, C4 threads it through `?`/`ok_or`, C6 decorates `next_element<u8>`) is exactly such a seam, and telemetry coverage widens as each new seam lands.

**D. Disagreement-telemetry + acceptance.** Implement the deferred §13.10 dump as `RUSTY_CPP_PRINT_INFERENCE`, but in the **reconcile** form (§13.14.1): at every owner-element/local site, compute *both* the heuristic answer and the engine answer and log one of {AGREE, DISAGREE, ENGINE-ONLY, HEURISTIC-ONLY, NEITHER}, with the site, the two types, and the collected constraints/solve-iterations/substitution for the engine side. DISAGREE lines are bugs (in a resolver or a heuristic) and are the highest-signal output; ENGINE-ONLY/HEURISTIC-ONLY lines name coverage gaps; AGREE counts per heuristic are the retirement ledger. Acceptance: §13.10 met; matrix ≥ today (14 pass / 1 known-fail); **zero DISAGREE across the matrix** (each one fixed or explained before a step lands); a heuristic retired only once its sites are all AGREE; and the serde_bytes test targets (`test_derive`/`test_serde`) genuinely compile (closes #35/#36). Note: `augment_*` count is *no longer* an acceptance metric — it may stay flat for many steps and only drops when telemetry licenses a retirement.

**E. Risks.** A2 identity is the crux — prove the positional index consistent between collect and emit (with a unit test that resolves a known local) before touching any emit path. Big-bang risk is contained by the flag + the **fill-only** seam (a new C-rule can only add coverage where the frozen heuristics are silent, so it cannot regress the matrix) + telemetry-gated retirement. The `stmts.len() <= 128` augmentation guard (emit_expr.rs:532) has an analogue here: per-function solve is O(constraints) and should be cheaper than the N forward-scans it replaces, but keep a ceiling and measure. The work does not vanish — but new shapes relocate from "new pass" to "new rule", and rules **compose in one solver** instead of multiplying as blind passes. The heuristic count stays flat (frozen) and falls only when telemetry licenses a retirement — it is no longer the thing we optimize per step.

**F. First slice (proves the seam end-to-end).** `A1 + A2(let-index only) + B(flagged) + C1` → re-express the bytebuf newtype-consumer pass as the C1 field-init constraint → delete `augment_vec_local_element_hints_from_newtype_consumer` → matrix. One slice exercising the public renderer, the identity index, the emit seam, and one retired heuristic, on a case already fully understood — and it should pick up the bytearray array-field case for free.

> **Landed (first slice, leaner than written above).** The bytebuf shape turned out to need *only* C1 plus the **existing** `augment_owner_local_type_hints_from_solver` → `infer_local_owner_element_from_block` seam — that path already runs the collector+solver over the block and feeds the placeholder-hint pipeline (the same `bytes ⇒ Vec<u8>` hint the deleted pass produced). So the slice added the **C1 collector rule** (`record_newtype_field_constraint`: `Wrapper::from/new(local)` ⇒ `typeof(local) = field_ty`) and a **`FieldResolver`** callback (CodeGen's `single_field_type_of_struct`, parallel to the existing `ItemResolver`), then **deleted** `augment_vec_local_element_hints_from_newtype_consumer` and its two walker helpers. The `emit_local` engine-first seam (A1/A2/B) was **not** needed for this shape and is deferred to the expr-level slice (the §13.14 A2-full/B2 task), where `infer_simple_expr_type` consults the engine directly. Result: bytebuf still emits `next_element<uint8_t>`; parity matrix 14 pass / 1 known-fail (no regression); 1636 unit tests pass; one heuristic pass retired into the engine — the pattern the rest of Phase 4 repeats.

> **Investigated, NOT landed (the ByteArray `next_element` turbofish — C5+C4, on branch `wip/serde-bytes-bytearray-36`).** The serde_bytes ByteArray `visit_seq` (`for (idx, byte) in bytes.iter_mut().enumerate() { *byte = seq.next_element()?.ok_or_else(…)? }`) emitted a bare, unresolvable `SeqAccess_::next_element` ("no member named next_element"). Two findings overturned the §13.14 framing for this case: (1) **it is forward-flow, not the backward engine.** The element type flows *forward* `typeof(*byte)=u8` → `next_element`, recoverable by the **existing** expected-type-threading machinery once two gaps closed — the type-solver engine was never involved. **C5** types the `enumerate` tuple for-binding (`emit_for_loop` only typed `Pat::Ident`; tuple bindings stayed untyped `auto&&`, so `*byte`'s type was unknown). **C4** extends the existing `ok_or` receiver-threading handler to `ok_or_else` (same `Option<T>→Result<T,E>` shape) so the assignment-target `u8` survives the `.ok_or_else(…)?` hop and reaches the call, where the **already-present** Path-1 cross-crate (`SeqAccess_::`) turbofish injection (emit_expr.rs:5056) fires. Verified on the real crate: 8/8 `SeqAccess_::next_element` get `<uint8_t>`; 1651 unit tests pass; **C5 alone is matrix-green (14/1)**. (2) **Why it cannot land on main yet — the precompile-skip-gate.** parity-test *skips* a test target whose precompile fails (compile-validation only → crate PASS); that is how serde_bytes "passes" today with `test_derive`/`test_serde` not genuinely compiling (#35). C4 fixes *enough* of `test_derive` that it **escapes the skip-gate**, and the full build then fails on the **remaining** de-hollow errors C5/C4 do not touch: serde_core `IgnoredAny::visit_seq` while-let `next_element` (no turbofish), map `next_entry` (`DeserializerMapVisitor`/`EnumMapVisitor`), and `__DeserializeWith` deduced-return-before-defined. So C5+C4 regress serde_bytes PASS→FAIL. **Lesson:** progress that lifts a target past the skip-gate but not all the way to compiling is a *net matrix regression* — the de-hollow targets must be fixed *as a set*, not incrementally. The remaining three error classes are the gating work for #35; C5+C4 are preserved on the branch to land together with them.

### 13.15 Return-position-only type and const parameters (the fifth family)

§13.2 named four families of deduction failure. There is a fifth, and it is
structurally different from the others: **a type parameter that appears only in
the function's return type, never in any value parameter.**

```rust
pub trait IntersperseElement<Item> {
    fn generate(&mut self) -> Item;     // Item: in the return type only
}
```

lowers to

```cpp
template<typename Item, typename F>
Item generate(F& self_);                // Item deducible from nothing
```

C++ deduction reads only the argument list, so `Item` is unrecoverable at every
call site and the compiler reports `couldn't infer template argument 'Item'`.
This is not a variant of §13.2's cases 1–4: there is no arm to unify, no
container to read the element off, no closure return to merge, and no target
field to flow backward from. The information Rust used is simply not present in
the C++ signature.

The shape is common in iterator-adjacent APIs. In itertools it accounts for the
bulk of the residual error tail: `generate`, `extract_item`, `next_array` (`N`),
`next_tuple` (`T`), `flatten_ok`, `map_ok`, `intersperse`.

#### 13.15.1 Why the existing turbofish path does not cover it

For this family, explicit template arguments are supplied at exactly one
condition (`emit_expr.rs:6761`, the single-owner extension-call path):

```rust
if let Some(turbofish) = &mc.turbofish && !callee.contains('<') { … }
```

That is: **only when the Rust source itself wrote a turbofish.** On this path
the emitter threads what the programmer spelled; it computes nothing. So
`.next_array::<2>()` gets `<2>` and `.next_array()` gets nothing.

The restriction to *this family* is deliberate — it is not the only injector in
the emitter, just the only one that can fire here. Three serde-specific paths
also append template arguments to an extension callee, all keyed to the accessor
names `next_element` / `next_key` / `next_value` / `next_entry` and all gated on
`mc.turbofish.is_none()`, the exact complement of the gate above:

* `emit_expr.rs:6729-6750`, eleven lines *above* this gate in the same function,
  infers the element type from the expected `Result<Option<T>, E>` via
  `infer_serde_access_method_template_type_from_expected` (`inference.rs:11865`).
  The `!callee.contains('<')` clause above exists to avoid double-appending
  after that block has run.
* `emit_expr.rs:12634-12735` applies the same inference as
  `default_de_template_args` on the `rusty_ext` resolution path.
* `emit_expr.rs:21980-21998` splices *hardcoded* defaults (`<::de::IgnoredAny>`,
  `<std::tuple<>>`) with no inference and no turbofish; `21955` synthesizes
  `next_key<__Field…>` from a seed type.

So the emitter demonstrably *can* compute an argument the source never wrote.
What it cannot do is compute one for the §13.15 family, because none of those
computations generalize past the serde accessor name list. **That existing
expected-type injector is the working precedent this family's fix should
generalize** — the mechanism is present and proven; only its keying is wrong.

The distribution matters when scoping work here. Counting *code* call sites in
itertools (excluding `///` doc examples, which inflate a naive grep): `next_array`
4 bare : 1 turbofished, `next_tuple` 3 : 1, and `generate` (2), `intersperse`
(10), `map_ok` (3), `flatten_ok` (9), `extract_item` (2) are bare at *every*
site. Any fix keyed on the presence of a turbofish therefore addresses a small
minority of the occurrences.

A related consequence: the auto-deref dispatcher emits
`rusty_ext::generate(__self)` with no template arguments inside its
`if constexpr (requires { … })` probe. That is not a defect in the dispatcher —
it faithfully passes along the callee string it was given, and for a bare call
site there is nothing to pass. Fixing the dispatcher in isolation would change
nothing.

#### 13.15.2 The split: three sub-families with different costs

The family does not have one fix. Rust recovers the missing parameter by three
different mechanisms, and each demands different machinery from us.

**(a) Impl-determined — the type argument is fixed by the impl.**

```rust
impl<Item: Clone> IntersperseElement<Item> for IntersperseElementSimple<Item> {
    fn generate(&mut self) -> Item { self.0.clone() }
}
```

Given `Self = IntersperseElementSimple<u8>`, `Item = u8` follows structurally
from the impl header. No flow analysis is involved: this is a *lookup*, and the
answer is available statically at the point of emit. `generate` and
`extract_item` are here, and — per the `Combination`-on-tuple errors — so is the
`PoolIndex` family.

**(b) Flow-determined — the argument comes from how the result is used.**

Note first that `N` below is a *const generic*, not a type parameter —
`fn next_array<const N: usize>(&mut self) -> Option<[Self::Item; N]>`; rustc's
diagnostic is "cannot infer the **value** of the const parameter `N`". Its
sibling `next_tuple<T>` *is* a type parameter, so this family spans both kinds.
The distinction has teeth for §13.15.3: a per-trait *type* map cannot carry a
const, and needs a `static constexpr std::size_t` member rather than a nested
typedef. The C++ analogue of a return-position-only const is a non-type template
parameter — undeducible for the same reason.

```rust
assert_eq!(iter.next_array(), Some([]));                    // N = 0
assert_eq!(iter.next_array().map(|[&x, &y]| [x, y]), …);    // N = 2
```

Neither call site carries a clue. In the first, `N = 0` comes from the **arity
of the empty array literal** in the sibling operand: `Some([])` has type
`Option<[_; 0]>`, and `Option`'s `PartialEq` is homogeneous, so the comparison
forces the lengths equal. The macro is incidental to Rust — a bare
`iter.next_array() == Some([])` infers identically; `assert_eq!` matters only to
*us*, because our collector must see through the expansion to reach the operand.
In the second, `N = 2` is fixed by the **closure parameter's array pattern**, one
combinator (`Option::map`) removed from the call — rustc infers an array length
from a fixed-arity pattern with no rest binding. The `Some([1, 2])` on the right
contributes nothing to `N`; it pins the closure's *output*. Replacing
`|[&x, &y]|` with a plain binding makes the line fail with `E0284` even with that
operand intact. This is §13.2's case 2 (backward flow), but from constraint
sources §13.6 does not collect: sibling operands of a comparison, and pattern
arity inside a closure parameter.

**(c) Turbofish — the programmer already wrote it.** The mechanism exists; see
§13.15.1 and the `Self_`-ordering interaction in §13.15.4.

Collapsing (a) and (b) into a single "undeducible type argument" project is a
scoping error worth avoiding: (a) is a bounded lookup that fits the existing
architecture, while (b) requires new constraint sources and genuinely harder
inversion. They should be planned and estimated separately.

#### 13.15.3 Sub-family (a) versus the `<Trait>Traits` map — and the §13.8 boundary

Chapter 14 already defines the mechanism that would answer (a): the per-trait
type map, `PoolIndexTraits<Self>::Item`, specialized per impl. But it models
**associated types**, and (a) is about **trait type parameters**. The difference
is not cosmetic:

```rust
trait PoolIndex           { type Item; }        // associated  → keyed on Self alone
trait IntersperseElement<Item> { … }            // parameter   → Self is not a key
```

An associated type of a *non-generic* trait is a function of `Self`: one impl,
one `Item`, so `Traits<Self>::Item` is well-defined. Neither real case is that
simple.

`PoolIndex` is generic **and** has an associated type — `trait PoolIndex<T>:
BorrowMut<[usize]> { type Item; }` — and its array impl defines
`Item = [T; K]`, naming a `T` that appears nowhere in `Self`. So
`PoolIndexTraits<Vec<usize>>::Item` is not merely ambiguous, it is ill-defined,
and Chapter 14's single-parameter sketch cannot be emitted for the real trait.
The trait-*parameter* case is the same defect from the other side: one type may
implement a trait at several arguments, as itertools does with a blanket
`impl<Item, F: FnMut() -> Item> IntersperseElement<Item> for F` alongside the
concrete `impl<Item: Clone> IntersperseElement<Item> for
IntersperseElementSimple<Item>`.

Both are one recorded limitation — the `…Traits` primary is single-parameter and
cannot carry the trait's own type arguments — and the fix is uniform: key the map
on `(Self, trait args)`, i.e. emit `PoolIndexTraits<Self, T>`. That is precisely
why (a) cannot be answered without consulting the impl set.

This sits close to a stated non-goal without crossing it. §13.8 says the engine
"is not a replacement for Rust's borrow checker or trait resolution. Those are
upstream — by the time we run, the input has already type-checked under `rustc`."
Read to the end of the sentence, that disclaims *redoing* trait resolution, and
its justification is the very premise family (a) relies on: rustc has already
chosen the impl. Nor is the operation foreign to §13.5's solver — recovering
`Item` from `impl … IntersperseElement<Item> for IntersperseElementSimple<Item>`
given `Self = IntersperseElementSimple<u8>` is exactly the
`(Ctor c args, Ctor c' args')` rule, and §13.6 already reads declared types out
of signatures and struct fields. What (a) adds is a new *source* for that
lookup — the impl set — and per §13.15.5 Option 1 even that lives in Chapter 14's
type map, which §13.8's non-goals do not scope.

What the boundary does need is an explicit statement, since the impl set has not
been named as a lookup source before:

> The engine does not perform trait *selection* — it never chooses among
> candidate impls by checking bounds, coherence, or specialization. It does
> perform trait-argument *recovery*: given a `Self` type that already
> type-checked under rustc, look up the impl and read off the trait's type
> arguments.

Recovery is decidable and cheap; selection is neither. Where recovery is
ambiguous — several impls of the same trait for one `Self` — the engine must
report failure and fall back, exactly as §13.5 prescribes for unification
failure. It must not guess.

#### 13.15.4 Interaction with `Self_` parameter ordering

Where explicit arguments *are* supplied, C++ fills them positionally from the
left. So if a call site is to spell only the parameters C++ cannot deduce, those
must form a prefix — they must precede `Self_`, which is deducible (it types the
`self_` function parameter). This is a consequence of wanting to spell only a
prefix, not a language rule: spelling every argument explicitly works at any
ordering.

Spelling `Self_` explicitly is possible but fragile, because the correct spelling
depends on the callee's receiver kind. A `&mut self` method emits its receiver as
a *forwarding reference* `Self_&&`; substituting a non-reference explicit
argument turns that into an rvalue reference, which cannot bind an lvalue
receiver. (A by-value `self` method, emitting `Self_ self_`, has no such problem —
a by-value parameter copy-initializes from an lvalue happily. The failure is
specific to the forwarding-reference receiver, not to by-value spelling.) A
spelling that carries the receiver's category — `decltype(__self)` — works, and
the emitter uses exactly that where it must spell `Self_`. The simpler discipline,
and the one adopted, is to supply only the undeducible prefix and let `Self_`
deduce, preserving the category without naming it.

One C++ constraint falls out of this ordering and is easy to miss: a default
template argument may only name parameters already declared, so item-projection
defaults of the form `= next_item_t<Self_>::…` are ill-formed on a parameter
placed *before* `Self_`. The resolution is to keep such parameters *behind*
`Self_` with their defaults intact: only the **bare** undeducible parameters —
return-position-only and undefaulted — form the explicitly spelled prefix, after
which `Self_` deduces and the defaults behind it fill themselves in. This is what
`extension_bare_undeducible_prefix_len` (`mod.rs:19191`) implements, and it is
already on main. Dropping the defaults instead was tried and abandoned.

#### 13.15.5 Options

**Blanket impls make (a) harder than a lookup.** itertools declares *two* impls
of `IntersperseElement`, and both are emitted as overloads:

```cpp
Item generate(IntersperseElementSimple<Item>& self_);   // concrete impl: Item IS deducible
template<typename Item, typename F> Item generate(F& self_);  // blanket impl: F matches anything
```

The concrete overload is fine — `Item` deduces from `IntersperseElementSimple<u8>`.
The blanket one, from `impl<Item, F: FnMut() -> Item> IntersperseElement<Item> for F`,
has a universally-matching parameter and an undeducible `Item`. Spelling `Item`
at the call site therefore does *not* settle it: both overloads remain viable,
so the fix must also constrain the blanket overload — its Rust bound
`F: FnMut() -> Item` has to survive as a C++ `requires` clause, which both
restores deducibility (via the callable's return type) and removes it from the
overload set for non-callable receivers. Any plan for (a) that stops at
"recover the type argument and spell it" is incomplete.

Note also where the emission lives: these are *per-impl* free functions, not the
`Self`-templated default-method path, so a fix hooked only into the latter never
fires for them.

**Option 1 — trait-argument recovery via an extended type map (addresses (a)).**
Extend the per-trait map so the trait's own type arguments are recoverable, and
have emit consult it to spell the undeducible prefix. Bounded, fits the existing
architecture, and reuses Chapter 14's machinery. Requires resolving the
single-key ambiguity above — either by keying on more than `Self`, or by
detecting multiplicity and failing cleanly.

**Option 2 — engine-driven backward flow (addresses (b)).** Add the missing
constraint sources to §13.6's collector: equality/comparison operands (including
through macro expansion) and closure-parameter pattern arity. This is the same
mechanism §13.13 specifies for `next_element`, applied to new syntax. Note the
harder half: recovering `N` from `|[&x, &y]|` requires inverting through the
`.map()` combinator, not merely reading a sibling's type.

**Option 3 — require a turbofish in the source.** Rejected. It would mean
editing the crates under translation, which the parity program exists to avoid.

Options 1 and 2 are independent and can land in either order. Neither subsumes
the other.

#### 13.15.6 Consequence for parity scope

Because (a) and (b) are independent, no single change clears a crate whose tail
contains both. itertools is exactly such a crate: landing Option 1 removes the
impl-determined errors but leaves the flow-determined ones, so the crate cannot
reach a green gate on that work alone. Scheduling should treat "add the crate as
a tracked known-FAIL row" and "make the crate green" as separate milestones, and
should not assume the second follows shortly from the first.

## 14. Name Resolution: Trait Helper Qualification and the Two-Pass Inconsistency Problem

### 14.1 Why this chapter exists

Chapter 13 was about *type inference* — bridging Rust's HM-style propagation and C++'s local deduction. This chapter is about *name resolution* — bridging Rust's path resolution (crate-wide, scope-aware, defers unresolved names to later passes) and C++'s lexical scope rules (definition-order, ambient-lookup-first, shadowed by any same-named symbol).

The two problems sound similar but live in different machinery. Type inference asks "what is the concrete type of expression E?" Name resolution asks "where in C++'s namespace tree does this symbol live, and will the spelling I emit at this use site actually resolve to that location?"

Today the transpiler does name resolution by ad-hoc hashmap lookups (`trait_declared_path_by_short_name`, `trait_associated_type_names`, `imported_use_paths`, …) plus string template formatting. That works for the simple cases. It breaks on two fronts:

1. **Two-pass inconsistency**: the same symbol is emitted with different spellings in different passes because the hashmap is populated incrementally during the final pass.
2. **Use-site shadowing**: a qualified path like `::a::b::c::Type` can fail when one of the middle segments (`b` or `c`) is *also* a function-template or type-alias name at the use site, and ambient lookup picks the non-namespace meaning first.

Both produce real C++ compile errors in itertools. This chapter documents the problem, the failed attempts, and the design for a proper fix.

### 14.2 The canonical case in detail: itertools' `CombinationsWithReplacement`

Rust source (simplified from `itertools` `src/combinations_with_replacement.rs` after `cargo expand`):

```rust
mod combinations {
    pub trait PoolIndex {
        type Item;
    }
}

mod combinations_with_replacement {
    use super::combinations::PoolIndex;

    pub struct CombinationsWithReplacement<I: PoolIndex>
    where I::Item: Copy {
        iter: I,
    }
}
```

The constraint `I::Item: Copy` is what becomes a C++ `requires` clause.

C++ templates don't have associated types. To simulate `Self::Item`, the transpiler emits a separate template called `<Trait>Traits` and specializes it per-impl:

```cpp
namespace combinations {
    template<typename I> struct PoolIndexTraits;       // primary

    template<typename I> class PoolIndex { /* …interface… */ };

    // For each `impl PoolIndex for Vec<usize> { type Item = usize; }`:
    template<> struct PoolIndexTraits<Vec<usize>> { using Item = usize; };
}
```

So `Self::Item` in Rust must lower to `typename PoolIndexTraits<Self>::Item` in C++. The transpiler does this rewrite by consulting `trait_associated_type_names: HashMap<TraitName, Vec<AssocName>>`, populated during the trait's emit pass.

Now the actual emission. The transpiler runs **two passes** that both touch this constraint:

```cpp
// Pre-pass forward decl (itertools.cppm:4231):
namespace combinations_with_replacement {
    template<typename I>
        requires (std::copyable<typename I::Item>)   // ← unqualified
    struct CombinationsWithReplacement;
}

// Final-pass forward decl (itertools.cppm:9633) + struct body (9657):
namespace combinations_with_replacement {
    template<typename I>
        requires (std::copyable<typename ::combinations::PoolIndexTraits<I>::Item>)   // ← qualified
    struct CombinationsWithReplacement {
        I iter;
    };
}
```

C++ rejects: `requires clause differs in template redeclaration` (clang) or the equivalent in gcc. Even though the two requires-clauses are *logically equivalent* for any `I` that has both `I::Item` and `PoolIndexTraits<I>::Item` defined the same way, **C++ does syntactic equivalence on requires clauses**, not semantic. The text has to match.

### 14.3 Self-contained reproduction

This compiles fine if you remove either declaration of `Foo`, but fails when both are present:

```cpp
// repro.cpp — clang++ -std=c++20 repro.cpp
#include <concepts>

template<typename I> struct PoolIndexTraits;

// Forward decl — one spelling of the constraint
template<typename I>
    requires (std::copyable<typename I::Item>)
struct Foo;

// Body — different spelling of the same logical constraint
template<typename I>
    requires (std::copyable<typename PoolIndexTraits<I>::Item>)
struct Foo {
    I x;
};

int main() {}
```

Error:

```
repro.cpp:13:18: error: requires clause differs in template redeclaration
    requires (std::copyable<typename PoolIndexTraits<I>::Item>)
                 ^
```

The minimal isolated form makes the issue obvious: it isn't itertools-specific, it isn't even Rust-specific — it's a generic C++ rule about template redeclarations.

### 14.4 Why the spelling differs

Both passes call `map_type` to translate the Rust type. `map_type` consults `lookup_unique_trait_for_assoc_name(assoc_name) → Option<TraitName>`, which queries `trait_associated_type_names`. The two passes hit this lookup at different times:

- **Pre-pass**: forward-declaration walk of the whole module. The trait `PoolIndex` hasn't been emitted yet, so `trait_associated_type_names` doesn't contain `"PoolIndex"`. Lookup returns `None`. `map_type` falls through to the literal `typename I::Item`.

- **Final pass**: traits are emitted first in the final pass, populating the registry. When the constraint gets mapped later in the same pass, lookup returns `Some("PoolIndex")`. `map_type` rewrites to `typename ::combinations::PoolIndexTraits<I>::Item` (the leading `::` comes from `trait_declared_path_by_short_name`, the qualification fix landed at commit `2f9c570`).

Same code path. Different outputs. Because the registry is populated incrementally during the final pass.

### 14.4a Deeper finding: the transpiler flattens some Rust modules

While implementing Phase A in `transpiler/src/codegen/symbol_category.rs`, a second-order issue surfaced that the original design didn't account for: **the transpiler doesn't always preserve Rust's module nesting in the C++ output**.

itertools' Rust source:

```rust
mod adaptors {
    mod coalesce {
        pub trait CountItem { type CItem; }
        pub fn coalesce<I, F>(iter: I, f: F) -> /* ... */ { /* ... */ }
    }
}
```

What ends up in the C++ output:

```cpp
namespace adaptors {
    // coalesce module is FLATTENED — trait and function both land
    // at adaptors:: directly, NOT under a nested namespace coalesce.
    class CountItem;
    template<typename I, typename F> auto coalesce(I iter, F f);
}
```

So `adaptors::coalesce` as a namespace doesn't exist in the C++ output, but `trait_declared_path_by_short_name["CountItem"]` records the Rust-source path `adaptors::coalesce::CountItem`. The qualification step then emits `::adaptors::coalesce::CountItemTraits<C>::CItem` — referencing a namespace that was never declared.

A `SymbolCategoryTable` populated from `syn::Item` walks records the Rust-source structure (`mod coalesce` → namespace in our table) and agrees with `trait_declared_path_by_short_name`. **Both registries are wrong in the same way.** The C++ emit pipeline silently flattens the intermediate `mod coalesce` and neither registry sees it.

**Why flattening happens:** the transpiler's module-emission code has logic that lifts a module's contents into its parent under certain conditions (e.g. when the inner module has no visible items beyond the trait, or when the items collide with sibling-namespace names, or other heuristics that haven't been traced in detail).

**Implication for Ch. 14:** the design as written can't fix the requires-clause-differs issue alone. Even if both passes consult the same `SymbolCategoryTable` and produce identical strings, the strings are correct relative to the Rust source but wrong relative to the actual C++ emit. The fix requires modeling the flattening rules — a third source of truth about "which namespaces actually exist in the C++ output".

### 14.5 The failed pre-collect attempt

The obvious fix: walk all items recursively *before* the pre-pass runs and populate `trait_associated_type_names` for every trait. Then both passes see the same registry and produce the same qualified form.

This was tried (this session, reverted in-place — no commit). The `requires clause differs` error cleared on `CombinationsWithReplacement`. But itertools then failed elsewhere:

```cpp
// itertools.cppm:6472 (after pre-collect):
rusty::Option<rusty::Option<typename ::adaptors::coalesce::CountItemTraits<C>::CItem>> last;
//                                  ^^^^^^^^
// error: 'coalesce' is not a class, namespace, or enumeration
```

The shape of the regression:

```rust
mod adaptors {
    pub mod coalesce {
        pub trait CountItem { type CItem; }   // trait in `coalesce` submodule
    }

    pub fn coalesce<I, F>(iter: I, f: F) -> /* ... */ { /* ... */ }
    //         ^^^^^^^^
    // function ALSO named `coalesce`, in `adaptors` scope
}
```

After pre-collect populated `trait_associated_type_names["CountItem"]`, the qualifier mechanism rewrote `C::CItem` to `::adaptors::coalesce::CountItemTraits<C>::CItem`. The path *exists* in the namespace tree. But at the use site, C++ name lookup walks left-to-right through the segments:

1. `::adaptors` — finds `namespace adaptors`. OK.
2. `coalesce` — looks for `coalesce` inside `adaptors`. Finds the **function template**, not the **submodule**. Reports error.

In Rust, the function and the submodule occupy different name categories (value namespace vs type namespace). Rust's path resolver picks the type-category meaning when followed by `::Foo`. C++ has no such category — `coalesce::CountItemTraits` is one path, and ambient lookup of `coalesce` resolves uniquely to whatever the compiler finds first. Function name wins.

The final pass somehow avoids this — it must be selectively gating which traits get registered, but I haven't traced the exact rule. Likely it's that the final pass emits the body of `class PoolIndex` (and `class CountItem`) inside the trait's home namespace, and only at that point does `trait_associated_type_names` get populated. Anything that references the assoc type **outside that namespace** but **before the registry gets populated** gets the unqualified fallback (`typename I::Item`). It just happens that the only "outside, before" emit path is the pre-pass forward decl, which is what produces the `requires clause differs` mismatch.

### 14.6 What the proper fix needs to know

Two pieces of information that the transpiler currently doesn't track per emit site:

1. **The lexical scope of the use site**: where in C++'s namespace tree is the emit happening? Today the transpiler tracks `module_stack` (an ordered list of namespace names), but only uses it loosely for paths-to-`use`-aliased-imports. It doesn't ask "is `X` in this scope a namespace or a function template?"

2. **The candidate qualified path for a symbol**: for `PoolIndex`, the candidate is `::combinations::PoolIndex`. For `CountItem`, the candidate is `::adaptors::coalesce::CountItem`. The transpiler tracks these via `trait_declared_path_by_short_name`. Good.

What's missing: **a way to ask, at the use site, "does each segment of this qualified path resolve uniquely to a namespace (or class type for nested-class disambiguation) in the current scope?"** If yes, qualify. If no, fall back to the unqualified form.

The same question applies to **all** cross-namespace qualified emission, not just trait helpers. The matrix has gotten away with bespoke handling so far because most cross-namespace references happen to use namespace names that aren't shadowed. Itertools is the first crate that hits the collision in earnest.

### 14.7 Design: per-scope symbol category table

Introduce a new emit-time data structure:

```rust
/// Records what category each top-level (or nested) name resolves to
/// within a given C++ namespace scope. Populated during a pre-pass that
/// walks all `syn::Item`s in dependency order and classifies each
/// declared name by its C++ emission category.
pub struct SymbolCategoryTable {
    /// Keyed by (scope path, name). E.g. ("adaptors", "coalesce") →
    /// {NamespaceSubmodule, FunctionTemplate}.
    pub by_scope: HashMap<(Vec<String>, String), CategorySet>,
}

bitflags::bitflags! {
    pub struct CategorySet: u8 {
        const NAMESPACE       = 0b0000_0001;   // namespace X { … }
        const CLASS_TYPE      = 0b0000_0010;   // class/struct
        const TYPE_ALIAS      = 0b0000_0100;   // using X = …
        const FUNCTION        = 0b0000_1000;   // free function (and templates)
        const VARIABLE        = 0b0001_0000;   // global variable
        const ENUM            = 0b0010_0000;   // enum class
    }
}
```

A name is **safely qualifiable as a namespace segment** if its category set at the lookup scope includes `NAMESPACE` and no `FUNCTION | TYPE_ALIAS | VARIABLE` — because those non-namespace categories live in the same lookup space and would shadow the namespace at ambient resolution time.

### 14.8 The two-phase emit pipeline

#### Phase A: Symbol classification (new)

A new pre-pass walks every `syn::Item` recursively and populates `SymbolCategoryTable`. For each item:

- `syn::Item::Mod(m)` → record `(parent_scope, m.ident)` with `NAMESPACE` flag. Recurse into `m.content`.
- `syn::Item::Struct(s)` → `(parent_scope, s.ident)` with `CLASS_TYPE`.
- `syn::Item::Enum(e)` → `(parent_scope, e.ident)` with `ENUM`.
- `syn::Item::Trait(t)` → record:
  - `(parent_scope, t.ident)` with `CLASS_TYPE` (the trait class itself).
  - `(parent_scope, format!("{}Traits", t.ident))` with `CLASS_TYPE` (the helper template).
  - Populate `trait_associated_type_names[t.ident.to_string()] = t.items.iter().filter_map(assoc_type_ident).collect()`.
- `syn::Item::Fn(f)` → `(parent_scope, f.sig.ident)` with `FUNCTION`.
- `syn::Item::Type(t)` → `(parent_scope, t.ident)` with `TYPE_ALIAS`.
- `syn::Item::Use(u)` → recurse into use tree; record imports as aliases pointing at their resolved target.

This pass runs **before any code emission**. It's pure data collection — no string output, no state mutation outside `SymbolCategoryTable` and `trait_associated_type_names`.

#### Phase B: Scope-aware path qualification (replaces ad-hoc lookup)

Replace the existing `lookup_unique_trait_for_assoc_name` + `trait_declared_path_by_short_name` + string format at `mod.rs:32228` with:

```rust
/// Given the current emit scope and a trait helper reference like
/// (trait_name="PoolIndex", base_param="I", assoc_name="Item"),
/// return either:
///   - the safely-qualified form `::scope::PoolIndexTraits<I>::Item`,
///     when every segment of the path resolves uniquely to a namespace
///     at the current emit scope; or
///   - the fallback unqualified form `typename I::Item`, when
///     qualification would risk shadowing.
fn emit_assoc_type_projection(
    &self,
    trait_name: &str,
    base_param: &str,
    assoc_name: &str,
) -> String {
    let Some(qualified_trait) = self.trait_declared_path_by_short_name.get(trait_name) else {
        return format!("typename {}::{}", base_param, assoc_name);
    };
    let segments: Vec<&str> = qualified_trait.split("::").collect();
    let helper_segments = {
        let (parent, last) = segments.split_last().unwrap();
        let mut s: Vec<String> = parent.iter().map(|s| s.to_string()).collect();
        s.push(format!("{}Traits", last));
        s
    };
    // Walk the path: each segment must resolve to NAMESPACE (or CLASS_TYPE
    // for the helper's own segment) without conflicting non-namespace
    // categories at the lookup scope.
    if self.path_resolves_unambiguously(&helper_segments) {
        format!(
            "typename ::{}<{}>::{}",
            helper_segments.join("::"),
            base_param,
            assoc_name
        )
    } else {
        // Fall back to unqualified — `requires` clauses with `typename
        // I::Item` are valid C++ when I genuinely exposes Item.
        format!("typename {}::{}", base_param, assoc_name)
    }
}

fn path_resolves_unambiguously(&self, segments: &[String]) -> bool {
    let scope = &self.module_stack;
    let mut scope_path: Vec<String> = scope.clone();
    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        let required = if is_last {
            CategorySet::CLASS_TYPE | CategorySet::NAMESPACE
        } else {
            CategorySet::NAMESPACE
        };
        let cats = self.symbol_category.lookup_with_ambient(&scope_path, seg);
        if !cats.intersects(required) {
            return false;
        }
        if !is_last && cats.intersects(
            CategorySet::FUNCTION | CategorySet::TYPE_ALIAS | CategorySet::VARIABLE,
        ) {
            // shadowing risk — function/alias/variable with same name as
            // a namespace at this scope. C++ ambient lookup wins for
            // those, breaking the qualified path.
            return false;
        }
        scope_path.push(seg.clone());
    }
    true
}
```

`lookup_with_ambient` walks the scope stack the way C++ does — checks `scope_path::seg`, then `parent_scope::seg`, then `parent_of_parent::seg`, etc., until it finds a hit or runs out of scopes. This is the same algorithm C++ uses for unqualified lookup, so it correctly reports the *first* match — which is what the compiler will see.

#### Phase C: Both passes call into the same `emit_assoc_type_projection`

The pre-pass and the final pass both go through the same code path. The function returns the same string for the same input. The two requires clauses match.

For itertools' `combinations_with_replacement`:
- Lookup of `combinations` in scope `["combinations_with_replacement"]`: not found locally. Walk up to the crate root. Found as `NAMESPACE`. No `FUNCTION` overlap. OK.
- Lookup of `PoolIndexTraits` in scope `["combinations"]`: found as `CLASS_TYPE`. Last segment, OK.
- → emit `typename ::combinations::PoolIndexTraits<I>::Item` in both passes.

For itertools' `adaptors::coalesce::CountItemTraits`:
- Lookup of `adaptors` in current scope: found as `NAMESPACE`. OK.
- Lookup of `coalesce` in scope `["adaptors"]`: found as `NAMESPACE | FUNCTION` (both the submodule and the function). **`FUNCTION` flag → shadowing risk → fallback.**
- → emit `typename C::CItem` in both passes (unqualified, but consistent across passes).

Both itertools sites compile.

### 14.9 What about the existing `lookup_unique_trait_for_assoc_name`?

The function still exists, still does the same lookup. The new logic *adds* a per-use-site validity check on top of the result. The change is additive — it doesn't remove machinery, it gates the qualification step with a name-resolution simulator.

`trait_associated_type_names` is still populated lazily during final-pass trait emit. The new `SymbolCategoryTable` is populated eagerly during Phase A. The two registries serve different purposes:

- `trait_associated_type_names` — "given an assoc name, which trait owns it?" Used to decide whether to attempt qualification.
- `SymbolCategoryTable` — "given a path and a use site, does the path resolve unambiguously?" Used to decide whether to *commit* to the qualification.

### 14.10 Open questions deferred to implementation

- How aggressive should `SymbolCategoryTable` be about recording `use` aliases? A `use X::Y as Z;` brings `Z` into the current scope as an alias for `X::Y`. The `Y` end of the alias still lives in `X`, but `Z` becomes a new name in the current scope. Need to model this without double-counting.
- How to handle anonymous-namespace traits (file-internal linkage). They shouldn't appear in cross-namespace qualification.
- Whether to support qualification *through* a `using namespace X` directive (probably yes for completeness, but lower priority — the matrix doesn't use them).
- The scan currently walks `syn::Item`s top-down; some items reference others before they're declared (recursive types, mutual recursion across modules). The scan needs to be order-insensitive — collect all declarations first, then build the category sets in one shot.

### 14.11 Acceptance criteria

- The C++ minimal repro from §14.3 compiles without modification when generated by the transpiler.
- itertools' `requires clause differs in template redeclaration` clears.
- itertools' `'coalesce' is not a class, namespace, or enumeration` does not regress.
- 14 currently-passing crates remain passing (arrayvec, bitflags, cfg-if, either, once_cell, pollster, semver, serde, serde_bytes, serde_core, serde_repr, smallvec, take_mut, tap).
- The unit-test suite remains at 1585 passing, 9 pre-existing failures unchanged.

### 14.12 Relationship to Chapter 13

Type inference (Ch. 13) and name resolution (Ch. 14) are complementary subsystems, not competing ones. Type inference figures out what `T` is. Name resolution figures out how to *spell* `T` at the use site. The two passes can run independently and are likely both correct simultaneously for any given AST node.

The shared philosophy: stop relying on incremental-during-emit state, move to pre-collected tables consulted by emit. Chapter 13's engine collects type constraints; Chapter 14's table collects symbol categories. In both cases the emit code becomes a lookup against pre-computed data rather than a string template against a partially-populated state machine.
