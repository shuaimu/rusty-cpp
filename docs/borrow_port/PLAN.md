# Transpiler trait-emit fix — plan + scope

Goal: fix the transpiler's `Trait::AssocType` rendering well enough
that `borrow_port`, `core_ascii_port`, and `core_str_port` transpile
to compiling C++. This unblocks `string_port`'s full body.

## Existing machinery (background)

The transpiler already emits `<Trait>Adapter<U>` partial template
specializations for non-generic traits via
`emit_trait_adapter_specializations` (codegen.rs:22815). Per impl
block it emits an explicit specialisation:

```cpp
template <>
class HashAdapter<MyType> final : public Hash {  // for impl Hash for MyType
    MyType value_;
public:
    explicit HashAdapter(MyType v) : value_(std::move(v)) {}
    void hash(...) override { /* delegates to rusty_ext::hash(value_, ...) */ }
};
```

The machinery bails out at codegen.rs:22823 with a TODO when the
trait has any associated type or type parameter:

```rust
if self.interface_traits_with_generics.contains(trait_name) {
    self.writeln(&format!(
        "// TODO(interface_traits): {} is generic — Adapter specializations require partial-spec template headers, not yet emitted",
        trait_name
    ));
    return;
}
```

This is exactly the TODO that lands in `borrow_port.cppm` at line 3868.

## What "fix" means concretely

Three sub-problems, each independently shippable:

### 3a. Emit Adapter for associated-types-only traits (1-2 days)

Lift the bail so traits whose `interface_traits_with_generics` set
membership is due to associated types ONLY (no real `<T>` etc) still
emit adapter specialisations. Each impl supplies `using AssocName =
<ResolvedType>;` typedefs alongside the method overrides.

ToOwned is the canonical case: 1 associated type (`Owned`), 2 methods.
After this step:

```cpp
template <typename T>
struct ToOwnedAdapter;                          // primary, undefined

template <>
struct ToOwnedAdapter<std::string_view> {       // for impl ToOwned for str
    using Owned = std::string;                  // <-- NEW
    static Owned to_owned(const std::string_view& self) { ... }
};
```

### 3b. Rewrite `T::AssocType` → `typename <Trait>Adapter<T>::AssocType` (2-3 days)

When emitting code inside a context with trait-bounded T (where T: Trait),
references to `T::AssocType` should route through the adapter. This
requires the transpiler to:

1. Track trait bounds in scope (already parsed in `where` clauses;
   need to thread through to type-emit).
2. At type-emit time for `T::Owned`, lookup whether T is bound by a
   trait with that assoc type, and rewrite if so.

This is the hardest piece — it touches type rendering and scope
tracking. Without it, 3a alone doesn't help: `Cow_Owned<B>` still
contains `typename B::Owned _0;` which fails when B doesn't have a
nested `Owned` typedef.

### 3c. Verify on borrow_port → ascii_port → str_port (2-4 days)

Apply 3a+3b. Iterate on residual issues (orphan impls,
visit_byte_buf prelude, etc — already-codified patcher patterns).
Aim to compile each port cleanly. Then verify string_port full-body
unblocks downstream.

## Realistic scope

| Phase | Best case | Worst case |
|---|---:|---:|
| 3a alone | 1 day | 2 days |
| 3b (the hard one) | 2 days | 1+ week |
| 3c verification | 2 days | 1 week (per port) |
| **Total** | **5 days** | **3+ weeks** |

Big variance because 3b touches type-rendering paths the
transpiler currently doesn't think about, and the cascading effects
are hard to predict without trying. The work also exposes whatever
follow-on transpiler gaps the dependency ports hit (each has its
own profile of issues).

## Recommendation

Start with 3a alone. It's the cheapest experiment. If 3a does NOT
help (because callers like `Cow_Owned<B>` still need `B::Owned` to
resolve), we'll see the same errors but with adapter specs now
available. Then choose whether to commit to 3b or fall back to the
hand-port option (B from the earlier menu).

This avoids sinking 1+ weeks before we know whether the approach
actually works.
