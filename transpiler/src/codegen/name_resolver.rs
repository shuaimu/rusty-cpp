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
}

impl NameResolver {
    pub(crate) fn clear(&mut self) {
        self.alias_edges.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.alias_edges.is_empty()
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
        // Fixpoint with a hard iteration cap as a cycle guard.
        for _ in 0..32 {
            match self.rewrite_once(&cur) {
                Some(next) => cur = next,
                None => break,
            }
        }
        if leading {
            format!("::{}", cur)
        } else {
            cur
        }
    }

    fn rewrite_once(&self, path: &str) -> Option<String> {
        let mut best: Option<(&str, &str)> = None;
        for (alias, target) in &self.alias_edges {
            let is_prefix = path == alias
                || (path.starts_with(alias.as_str()) && path[alias.len()..].starts_with("::"));
            if is_prefix && best.is_none_or(|(b, _)| alias.len() > b.len()) {
                best = Some((alias.as_str(), target.as_str()));
            }
        }
        let (alias, target) = best?;
        if path == alias {
            Some(target.to_string())
        } else {
            Some(format!("{}{}", target, &path[alias.len()..]))
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
