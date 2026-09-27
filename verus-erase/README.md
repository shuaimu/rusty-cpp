# verus-erase

Verus's own `verus! { ... }` ghost-code erasure, packaged as an ordinary
(non-proc-macro) library so the transpiler can run it on source it reads.

```rust
let items: proc_macro2::TokenStream = verus_erase::erase_items(tokens_inside_the_braces)?;
```

`erase_items` is exactly what `verus_builtin_macros::verus` does when rustc
compiles a Verus crate without the Verus driver (`verus_keep_ghost` unset):

```rust
syntax::rewrite_items(input, EraseGhost::EraseAll, /* use_spec_traits */ true)
```

It does not reimplement the erasure. The modules in `src/` other than
`lib.rs` are Verus's `builtin_macros` sources, copied with the smallest changes
that let them run outside a procedural-macro invocation. Any `compile_error!`
the rewrite emits (a parse error, or one of Verus's syntax diagnostics) comes
back as `Err`, so callers fail closed.

## The helper binary

The transpiler does not link this crate. `verus_syn` depends on proc-macro2
with the `span-locations` feature, and Cargo's feature unification would turn
that on for the transpiler's own proc-macro2 as well: on SRPC's crate, with
`--verus-exec` off, peak RSS went from 84 to 173 MB and CPU time from 56 to
72 s. Instead the package builds a small helper, `rusty-cpp-verus-erase`,
which `rusty-cpp-transpiler --verus-exec` spawns once per source file that
contains a `verus!` block. `src/protocol.rs` defines the stdin/stdout format;
every response repeats `VERUS_BUILTIN_MACROS_VERSION` and `VERUS_GIT_REV`, and
the transpiler refuses a helper that reports a different revision than the
one it was built against (`rusty-cpp-transpiler --verus-build-info` prints it).

Build the helper in its own Cargo invocation, never in the same one as the
transpiler (`-p rusty-cpp-transpiler -p verus-erase` or `--workspace` would
unify the feature back into the transpiler):

```sh
cargo build --release -p rusty-cpp-transpiler
cargo build --release -p verus-erase        # target/release/rusty-cpp-verus-erase
```

The transpiler looks for it via `--verus-erase-helper PATH`, then
`$RUSTY_CPP_VERUS_ERASE`, then next to its own executable (and, for Cargo test
binaries in `target/<profile>/deps/`, one directory up). Without a helper,
`--verus-exec` fails on the first file that needs it; files without Verus
constructs never spawn it. The transpiler's unit tests build their own copy
into `target/verus-erase-test-helper/`.

## Provenance

| | |
| --- | --- |
| crate | `verus_builtin_macros` `0.0.0-2025-11-10-1957` |
| source | `git+https://github.com/verus-lang/verus?rev=db81a74` (`db81a7496bfffeef3da8b30c306600ea51d2b0fa`), directory `source/builtin_macros/src/` |
| license | MIT, `LICENSE-MIT` (copied verbatim from the same revision) |
| exported as | `VERUS_BUILTIN_MACROS_VERSION`, `VERUS_GIT_REV` |

This is the package Lion's lockfile compiles. Lion depends on `vstd` by git
(`rev = "db81a74"`), and Cargo resolves vstd's path dependencies
(`verus_builtin_macros`, `verus_syn`, `verus_prettyplease`) inside that same git
checkout.

**Why not crates.io.** crates.io carries `verus_builtin_macros`,
`verus_syn` and `verus_prettyplease` under the same version string
`0.0.0-2025-11-10-1957`, but those releases were cut from an earlier commit
and are not byte-identical to `db81a74`. Measured with `diff -r` against the
local registry:

- `builtin_macros/src/syntax.rs`: the `proof_fn` closure rewrite passes the
  closure's declared return type (`#output`) instead of `_` to
  `ProofFnReqEnsDef`, `closure_to_fn_proof` and `proof_fn_as_req_ens`.
- `verus_syn/src/verus.rs`: `WithSpecOnFn` parsing stops at the next spec
  keyword.

Lion at `aa5bebe` exercises neither path (no `proof_fn` closures and no
`with` spec clauses in any of its crates), but "same version string" is not
"same code", so both the vendored sources and the `verus_syn` /
`verus_prettyplease` dependencies come from the git revision.
Building this crate therefore needs that revision in `~/.cargo/git` (it is
there whenever a Lion crate has been built) or network access.

## What was vendored

`rewrite_items` reaches these upstream modules, and nothing else:

| file | local changes |
| --- | --- |
| `syntax.rs` | `proc_macro::` → `proc_macro2::` on the 34 lines outside `#[cfg(verus_keep_ghost)]` code (entry points, `visit_stream_expr`, the `#![trigger]` tuple re-parse, the not-keep-ghost `rejoin_tokens`); `parse_macro_input` dropped from one `use` list |
| `enum_synthesize.rs` | `proc_macro::` → `proc_macro2::` in one not-keep-ghost signature (3 lines) |
| `contrib/mod.rs` | `pub mod exec_spec;` removed (`exec_spec!` is a separate proc macro) |
| `rustdoc.rs`, `syntax_trait.rs`, `unerased_proxies.rs`, `contrib/auto_spec.rs` | none |

`lib.rs` is new. It declares the modules, repeats upstream `lib.rs`'s
`EraseGhost` and `VstdKind` definitions verbatim, defines a proc_macro2
`parse_macro_input!` stand-in (upstream's converts through
`proc_macro::TokenStream`, which panics outside a macro invocation), fixes
`vstd_kind()` to `Imported` (upstream reads `VSTD_KIND` / `CARGO_PKG_NAME`
from the expanding process's environment), and exposes the API.
`protocol.rs` (the helper's wire format) and `bin/rusty-cpp-verus-erase.rs`
are new too; neither is vendored.

Not vendored: every other `#[proc_macro]` entry point, `contrib/exec_spec.rs`,
`struct_decl_inv.rs`, `atomic_ghost.rs`, `calc_macro.rs`, `attr_rewrite.rs`,
`attr_block_trait.rs`, `is_variant.rs`, `fndecl.rs`, `structural.rs`,
`topological_sort.rs`. `rewrite_items` never calls them. Consequently this
crate erases item-level `verus!` only; `#[verus_spec]`, `proof!`,
`struct_with_invariants!` and friends are not handled here.

Every `proc_macro::Diagnostic` warning, and every keep-ghost-only helper
(`cfg_erase`, `env_rustdoc`, `rejoin_tokens`, `cfg_verify_core`), stays
exactly as upstream wrote it behind `#[cfg(verus_keep_ghost)]`. This crate
never sets that cfg (plain rustc doesn't either), so those arms are not
compiled, and `Cargo.toml` repeats upstream's `check-cfg` declaration for it.

## Checking and re-vendoring

Each vendored file starts with a provenance header that ends at the line
`// ---- end of vendoring header ----`; everything after it is upstream text
apart from the listed changes. To see the complete local diff:

```sh
verus-erase/upstream-diff.sh            # locates the checkout through `cargo metadata`
verus-erase/upstream-diff.sh <verus>/source/builtin_macros/src
```

The output must contain only the changes in the table above.

To move to a new Verus revision (it must be the one the consuming Lion pin
resolves, from Lion's `Cargo.lock`):

1. Change `rev` for `verus_syn` and `verus_prettyplease` in `Cargo.toml`, and
   the version, rev and constants in `src/lib.rs`, the headers and this file.
2. Copy the files in the table over `src/` from the new
   `source/builtin_macros/src/`, keeping each file's header.
3. Re-apply the changes in the table. They are mechanical: replace
   `proc_macro::` with `proc_macro2::` wherever the line is not inside
   `#[cfg(verus_keep_ghost)]` code, delete `parse_macro_input` from imports,
   delete `pub mod exec_spec;`.
4. Compare upstream `lib.rs` with the verbatim block in `src/lib.rs`
   (`EraseGhost`, `VstdKind`), and check whether `rewrite_items` now reaches a
   module that is not vendored (`cargo build -p verus-erase` fails if it does).
5. Update `VERUS_BUILTIN_MACROS_VERSION` / `VERUS_GIT_REV` in the
   transpiler's `src/verus_exec.rs` to match (its unit test
   `transpiler_and_helper_crate_agree_on_the_vendored_revision` checks this).
6. Run `upstream-diff.sh`, `cargo test -p verus-erase`, and the transpiler's
   differential check against `cargo expand` of the Lion crates.
