# Nullable owner aliases at C++ boundaries

An explicit type-map entry can retain an existing nullable C++ owner while
Rust uses `Option` to express absence:

```rust
type MaybeArc<T> = Option<std::sync::Arc<T>>;
type MaybeBox<T> = Option<Box<T>>;
```

```toml
MaybeArc = "rusty::Arc"
MaybeBox = "rusty::Box"
```

The alias must resolve to `Option` around a standard `Arc` or `Box`, and its
mapped C++ target must match that inner owner. A target may name an existing
alias for the owner. Ordinary `Option<Arc<T>>` and `Option<Box<T>>` types keep
their normal Option representation.

The profile supports `Some`, `None`, `Default::default`, presence checks,
`as_ref`, `as_mut`, `unwrap`, `take`, `replace`, cloning, and simple `if let`
payload bindings. Rust checks whether the owner can be cloned. Unsupported
Option methods produce an explicit diagnostic. Empty profiled owners use the
runtime's null-pointer constructor; no allocation or payload is fabricated.

Borrowed alias parameters and aliases imported through a validated flat-module
binding retain the same profile. For callback signatures returned through an
imported generic wrapper, give a constructed argument an explicit alias type
before dispatch when the callback's parameter type cannot be inferred.
