// verus-erase: Verus's own `verus!` ghost-code erasure as an ordinary library.
//
// Provenance
// ----------
// Every module declared below except this file is vendored, with the minimal
// local changes listed in its own header, from
//
//   crate    verus_builtin_macros 0.0.0-2025-11-10-1957
//   source   git+https://github.com/verus-lang/verus?rev=db81a74
//            #db81a7496bfffeef3da8b30c306600ea51d2b0fa  (builtin_macros/src/)
//   license  MIT (LICENSE-MIT, copied verbatim from the same revision)
//
// That is the exact package Lion's lockfile compiles for its `vstd` git
// dependency. The crates.io publication with the same version string is NOT
// byte-identical to that revision, which is why it is not the source here
// (README.md records the measured difference).
//
// This file is new. It replaces upstream `lib.rs`, whose job is to host the
// `#[proc_macro]` entry points. It carries verbatim copies of the upstream
// `lib.rs` definitions the vendored modules name through `crate::`
// (`EraseGhost`, `VstdKind`), a fixed `vstd_kind()`, a proc_macro2 stand-in
// for `verus_syn::parse_macro_input!`, and the public API.
//
// What `verus!` does under plain rustc (upstream lib.rs, not(verus_keep_ghost)):
//
//   pub fn verus(input) { syntax::rewrite_items(input, cfg_erase(), true) }
//   fn cfg_erase() -> EraseGhost { EraseGhost::EraseAll }
//
// `erase_items` below is that call, with `cfg_erase()` inlined.

// The vendored modules are kept as close to upstream as possible, so their
// lint profile is upstream's, not this workspace's.
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_macros)]
#![allow(clippy::all)]

use proc_macro2::{TokenStream, TokenTree};

/// Stand-in for `verus_syn::parse_macro_input!` over `proc_macro2`.
///
/// Upstream's macro converts through `proc_macro::TokenStream`, which panics
/// outside a procedural-macro invocation. The behaviour is otherwise the same:
/// a parse error returns its `compile_error!` tokens from the enclosing
/// function, and `erase_items` turns those into an `Err`.
macro_rules! parse_macro_input {
    ($tokenstream:ident as $ty:ty) => {
        match verus_syn::parse2::<$ty>($tokenstream) {
            Ok(data) => data,
            Err(err) => {
                return proc_macro2::TokenStream::from(err.to_compile_error());
            }
        }
    };
}

#[macro_use]
mod syntax;
mod contrib;
mod enum_synthesize;
mod rustdoc;
mod syntax_trait;
mod unerased_proxies;

// ---- verbatim from upstream builtin_macros/src/lib.rs ----------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum EraseGhost {
    /// keep all ghost code
    Keep,
    /// erase ghost code, but leave ghost stubs
    Erase,
    /// erase all ghost code
    EraseAll,
}

impl EraseGhost {
    fn keep(&self) -> bool {
        match self {
            EraseGhost::Keep => true,
            EraseGhost::Erase | EraseGhost::EraseAll => false,
        }
    }

    fn erase(&self) -> bool {
        match self {
            EraseGhost::Keep => false,
            EraseGhost::Erase | EraseGhost::EraseAll => true,
        }
    }

    fn erase_all(&self) -> bool {
        match self {
            EraseGhost::Keep | EraseGhost::Erase => false,
            EraseGhost::EraseAll => true,
        }
    }
}

#[derive(Clone, Copy)]
enum VstdKind {
    /// The current crate is vstd.
    IsVstd,
    /// There is no vstd (only verus_builtin). Really only used for testing.
    NoVstd,
    /// Imports the vstd crate like usual.
    Imported,
    /// Embed vstd and verus_builtin as modules, necessary for verifying the `core` library.
    IsCore,
    /// For other crates in stdlib verification that import core
    ImportedViaCore,
}

// ---- end of verbatim upstream lib.rs --------------------------------------

// LOCAL CHANGE. Upstream `vstd_kind()` consults the `VSTD_KIND` and
// `CARGO_PKG_NAME` environment variables of the process expanding the macro,
// then `cfg(verus_verify_core)` / `cfg(verus_no_vstd)` (both false without
// `verus_keep_ghost`). Inside the transpiler that environment belongs to an
// unrelated process, so the answer is fixed to the one a plain-rustc build of
// an ordinary vstd *client* crate (such as Lion) gets: `Imported`.
fn vstd_kind() -> VstdKind {
    VstdKind::Imported
}

/// The `verus_builtin_macros` package version this erasure was vendored from.
pub const VERUS_BUILTIN_MACROS_VERSION: &str = "0.0.0-2025-11-10-1957";

/// The verus-lang/verus git revision this erasure was vendored from, spelled
/// the way Cargo records a git source's resolved commit in `Cargo.lock`.
pub const VERUS_GIT_REV: &str = "db81a7496bfffeef3da8b30c306600ea51d2b0fa";

/// Erase the body of one item-level `verus! { ... }` invocation exactly as
/// plain rustc does, returning the items rustc compiles.
///
/// `tokens` is the macro's input: the token stream *inside* the braces.
/// The result is Verus's `EraseGhost::EraseAll` rewrite with
/// `use_spec_traits = true`, i.e. what `verus_builtin_macros::verus` expands
/// to when `verus_keep_ghost` is not set.
///
/// Any `compile_error!` the rewrite produces (a parse error in the input, or
/// one of Verus's own syntax diagnostics) is reported as `Err`, so a caller can
/// fail closed instead of forwarding an item rustc would reject.
pub fn erase_items(tokens: TokenStream) -> Result<TokenStream, String> {
    let erased = syntax::rewrite_items(tokens, EraseGhost::EraseAll, true);
    let mut errors = Vec::new();
    collect_compile_errors(erased.clone(), &mut errors);
    if errors.is_empty() {
        Ok(erased)
    } else {
        Err(errors.join("; "))
    }
}

/// Collect the message of every `compile_error!(...)` invocation anywhere in
/// `tokens`, including inside delimited groups.
fn collect_compile_errors(tokens: TokenStream, errors: &mut Vec<String>) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let mut index = 0;
    while index < trees.len() {
        match &trees[index] {
            TokenTree::Ident(ident) if ident == "compile_error" => {
                let bang = matches!(trees.get(index + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
                if let (true, Some(TokenTree::Group(group))) = (bang, trees.get(index + 2)) {
                    errors.push(compile_error_message(group.stream()));
                    index += 3;
                    continue;
                }
            }
            TokenTree::Group(group) => collect_compile_errors(group.stream(), errors),
            _ => {}
        }
        index += 1;
    }
}

fn compile_error_message(tokens: TokenStream) -> String {
    let text = tokens.to_string();
    match verus_syn::parse2::<verus_syn::LitStr>(tokens) {
        Ok(literal) => literal.value(),
        Err(_) => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn erase(source: &str) -> Result<String, String> {
        erase_items(TokenStream::from_str(source).expect("test input lexes"))
            .map(|tokens| tokens.to_string())
    }

    #[test]
    fn exec_fn_keeps_body_and_drops_spec_clauses() {
        let erased = erase(
            "pub fn add(a: u64, b: u64) -> (r: u64)
                requires a < 100, b < 100,
                ensures r == a + b,
            {
                proof { assert(a + b < 200); }
                a + b
            }",
        )
        .unwrap();
        let expected = TokenStream::from_str("pub fn add(a: u64, b: u64) -> u64 { {} a + b }")
            .unwrap()
            .to_string();
        assert_eq!(erased, expected);
    }

    #[test]
    fn spec_and_proof_fns_are_removed() {
        let erased = erase(
            "spec fn double(x: int) -> int { x * 2 }
             proof fn lemma(x: int) ensures double(x) == x + x {}
             fn keep() {}",
        )
        .unwrap();
        let expected = TokenStream::from_str("fn keep() {}").unwrap().to_string();
        assert_eq!(erased, expected);
    }

    #[test]
    fn parse_error_is_reported_not_forwarded() {
        let error = erase("fn broken() -> {}").unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn version_constants_name_the_vendored_revision() {
        assert_eq!(VERUS_BUILTIN_MACROS_VERSION, "0.0.0-2025-11-10-1957");
        assert_eq!(VERUS_GIT_REV.len(), 40);
        assert!(VERUS_GIT_REV.starts_with("db81a74"));
    }
}
