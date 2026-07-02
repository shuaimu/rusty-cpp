use std::collections::HashMap;

/// Unified name/path alias resolution — the systematic core that is replacing
/// the scattered per-flavor alias maps + per-site `normalize_*`/`rewrite_*`
/// rewriters (see the alias-handling design notes).
///
/// An ALIAS EDGE maps a fully-qualified alias path to its canonical target:
///   - `extern crate alloc as stdalloc;`  ->  `stdalloc`            -> `alloc`
///   - `use sse2 as imp;` in `control::group`
///                                        ->  `control::group::imp` -> `control::group::sse2`
///
/// `resolve_prefix` rewrites the leading alias segment(s) of a path and follows
/// edges TRANSITIVELY to a fixpoint, so chained aliases (and re-exports routed
/// through an alias) collapse to their canonical path in a single call — which
/// the previous single-hop `normalize_*` helpers could not do (that gap caused
/// both the `::stdalloc::alloc::alloc::alloc` mangling and the
/// `control::group::imp::X` leak).
///
/// First slice of the name-resolution engine: it currently absorbs the
/// extern-crate and module-rename flavors. Later slices fold in `use … as …`
/// type/value renames, `pub use` re-exports, and the scoped-vs-bare PRECEDENCE
/// rule (the other recurring alias bug class).
#[derive(Default, Clone)]
pub(crate) struct NameResolver {
    alias_edges: HashMap<String, String>,
    /// Rust-visible bindings introduced by `use cpp::...` interop imports:
    /// binding name (alias or tail segment) -> imported C++ module path (no
    /// `cpp::`). Kept in a SEPARATE map from `alias_edges` on purpose: these
    /// are C++-interop binding names (e.g. `std`), not Rust path-prefix
    /// aliases, so they must never feed `resolve_prefix` (a binding named
    /// `std` must not rewrite an unrelated Rust `std::…` path). Consumers look
    /// them up single-hop via `cpp_binding` / iterate via `cpp_bindings`.
    cpp_module_bindings: HashMap<String, String>,
    /// External-crate-root -> transpiled C++ module namespace (e.g.
    /// `serde_core -> serde_core` identity, `itoa -> ""` strip). CRATE-LIFETIME
    /// config set once via `set_external_crate_aliases` (from transpile
    /// options), NOT per-file collected — so it is deliberately NOT touched by
    /// `clear()`. Kept separate from `alias_edges` because its values carry
    /// strip/identity/presence semantics (empty target = strip; identity edge
    /// must remain a queryable presence) that the `add_alias`/`resolve_prefix`
    /// engine does not model. Consumers look up single-hop via
    /// `external_crate_target` and do their own empty-string / presence
    /// branching, exactly as before.
    external_crate_aliases: HashMap<String, String>,
}

impl NameResolver {
    /// Clears only the PER-FILE collected state (`alias_edges`,
    /// `cpp_module_bindings`). `external_crate_aliases` is crate-lifetime config
    /// and is intentionally preserved across files.
    pub(crate) fn clear(&mut self) {
        self.alias_edges.clear();
        self.cpp_module_bindings.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.alias_edges.is_empty()
    }

    /// Record a `use cpp::<path> [as <alias>]` interop binding
    /// (`binding -> module_path`). Last writer wins, matching the prior
    /// `HashMap::insert` semantics this replaces.
    pub(crate) fn record_cpp_binding(&mut self, binding: String, module_path: String) {
        self.cpp_module_bindings.insert(binding, module_path);
    }

    /// Single-hop lookup of a `use cpp::…` binding's C++ module path.
    pub(crate) fn cpp_binding(&self, binding: &str) -> Option<&str> {
        self.cpp_module_bindings.get(binding).map(String::as_str)
    }

    pub(crate) fn cpp_bindings_is_empty(&self) -> bool {
        self.cpp_module_bindings.is_empty()
    }

    /// Iterate `(binding, module_path)` for the `use cpp::…` interop bindings.
    pub(crate) fn cpp_bindings(&self) -> impl Iterator<Item = (&String, &String)> {
        self.cpp_module_bindings.iter()
    }

    /// Replace the crate-lifetime external-crate-root -> C++ module map (from
    /// transpile options). Mirrors the prior whole-map assignment.
    pub(crate) fn set_external_crate_aliases(&mut self, aliases: HashMap<String, String>) {
        self.external_crate_aliases = aliases;
    }

    /// Single-hop lookup of an external-crate root's mapped C++ module path.
    /// Returns `&String` (not `&str`) so callers' existing `.trim()/.is_empty()/
    /// .clone()` and empty-string-strip branching are unchanged from when this
    /// was a bare `HashMap::get`.
    pub(crate) fn external_crate_target(&self, root: &str) -> Option<&String> {
        self.external_crate_aliases.get(root)
    }

    /// The external-crate roots this crate imports (keys of the alias map).
    pub(crate) fn external_crate_roots(&self) -> impl Iterator<Item = &String> {
        self.external_crate_aliases.keys()
    }

    /// Record `alias -> target`. First writer wins (matches the `or_insert`
    /// semantics of the maps this replaces); self-edges are ignored.
    pub(crate) fn add_alias(&mut self, alias: String, target: String) {
        if alias.is_empty() || alias == target {
            return;
        }
        self.alias_edges.entry(alias).or_insert(target);
    }

    /// Rewrite the leading alias-prefix of `path` to its target, transitively.
    /// Matching is on SEGMENT boundaries (so `stdalloc` matches `stdalloc::x`
    /// but not `stdallocx`), and the LONGEST matching alias prefix wins.
    /// Preserves a leading `::`. No-op when no edge prefixes `path`.
    pub(crate) fn resolve_prefix(&self, path: &str) -> String {
        if self.alias_edges.is_empty() {
            return path.to_string();
        }
        let leading = path.starts_with("::");
        let mut cur = path.trim_start_matches("::").to_string();
        // Fixpoint with a hard iteration cap as a cycle guard. Each alias
        // edge applies AT MOST ONCE per resolution: a SELF-REFERENTIAL
        // rename (`use crate::libyaml::error as libyaml;` inside a module —
        // target `libyaml::error` begins with the alias `libyaml`) would
        // otherwise re-match its own output every iteration
        // (`libyaml::Mark` → `libyaml::error::error::…::Mark`, serde_yaml's
        // fix_mark).
        let mut used: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for _ in 0..32 {
            match self.rewrite_once(&cur, &used) {
                Some((next, alias)) => {
                    used.insert(alias);
                    cur = next;
                }
                None => break,
            }
        }
        if leading {
            format!("::{}", cur)
        } else {
            cur
        }
    }

    fn rewrite_once<'a>(
        &'a self,
        path: &str,
        used: &std::collections::HashSet<&str>,
    ) -> Option<(String, &'a str)> {
        let mut best: Option<(&str, &str)> = None;
        for (alias, target) in &self.alias_edges {
            if used.contains(alias.as_str()) {
                continue;
            }
            let is_prefix = path == alias
                || (path.starts_with(alias.as_str()) && path[alias.len()..].starts_with("::"));
            if is_prefix && best.is_none_or(|(b, _)| alias.len() > b.len()) {
                best = Some((alias.as_str(), target.as_str()));
            }
        }
        let (alias, target) = best?;
        if path == alias {
            Some((target.to_string(), alias))
        } else {
            Some((format!("{}{}", target, &path[alias.len()..]), alias))
        }
    }
}

#[cfg(test)]
mod resolver_tests {
    use super::NameResolver;

    fn resolver(edges: &[(&str, &str)]) -> NameResolver {
        let mut r = NameResolver::default();
        for (a, t) in edges {
            r.add_alias(a.to_string(), t.to_string());
        }
        r
    }

    #[test]
    fn extern_crate_leading_segment() {
        let r = resolver(&[("stdalloc", "alloc")]);
        assert_eq!(r.resolve_prefix("stdalloc::alloc::Layout"), "alloc::alloc::Layout");
        // segment-boundary: no false substring match
        assert_eq!(r.resolve_prefix("stdallocx::Y"), "stdallocx::Y");
        // exact alias
        assert_eq!(r.resolve_prefix("stdalloc"), "alloc");
        // leading `::` preserved
        assert_eq!(r.resolve_prefix("::stdalloc::X"), "::alloc::X");
    }

    #[test]
    fn module_alias_interior_qualified() {
        let r = resolver(&[("control::group::imp", "control::group::sse2")]);
        assert_eq!(
            r.resolve_prefix("control::group::imp::BITMASK_ITER_MASK"),
            "control::group::sse2::BITMASK_ITER_MASK"
        );
        // a shorter same-prefix path that isn't the alias is untouched
        assert_eq!(r.resolve_prefix("control::group::X"), "control::group::X");
    }

    #[test]
    fn transitive_chained_aliases_reach_fixpoint() {
        // The single-hop normalizers could not do this: a -> b and b::c -> d
        // chained, plus a self-cyclic edge must not loop forever.
        let r = resolver(&[("a", "b"), ("b::c", "d::e")]);
        assert_eq!(r.resolve_prefix("a::c::Item"), "d::e::Item");
        let cyclic = resolver(&[("x", "y"), ("y", "x")]);
        // bounded by the iteration cap; just must terminate
        let _ = cyclic.resolve_prefix("x::Z");
    }

    #[test]
    fn longest_prefix_wins() {
        // When two NESTED aliases both prefix-match, the longest applies. (The
        // shorter `a::b` and longer `a::b::imp` share a domain only because
        // `a::b` here aliases the parent that contains `imp`; a real edge set
        // never makes a shorter alias a prefix of a longer alias's CANONICAL
        // result, so no over-application occurs.)
        let r = resolver(&[("a::b::imp", "a::b::sse2"), ("q", "r")]);
        assert_eq!(r.resolve_prefix("a::b::imp::T"), "a::b::sse2::T");
        assert_eq!(r.resolve_prefix("q::x"), "r::x");
    }
}
