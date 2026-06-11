//! Symbol category table — per-scope classification of declared names.
//!
//! Implements Phase A of the design in `docs/rusty-cpp-transpiler.md`
//! Chapter 14. Populated by a single pre-pass over the AST before any
//! code emission; consulted at trait-helper qualification time
//! (`mod.rs:32228`) to decide whether a qualified path like
//! `::a::b::c::FooTraits` will actually resolve at the C++ use site,
//! or whether some segment of the path is shadowed by a non-namespace
//! same-named symbol and the emit should fall back to unqualified.
//!
//! The data structure is simple: a `HashMap<(scope_path, name),
//! CategorySet>`. The categories distinguish whether a name binds to
//! a namespace (which can be a path segment) or to one of the
//! non-namespace categories (function template, type alias, variable)
//! that would shadow it in C++ ambient lookup.

use std::collections::HashMap;

/// What kind of C++ entity a Rust declaration emits as. A single name
/// at a single scope can have multiple categories (e.g. a `mod
/// coalesce` and a `fn coalesce` coexist in Rust — both end up at the
/// same scope in C++, just in different lookup categories).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CategorySet {
    /// `namespace X { … }` — a path segment.
    pub namespace: bool,
    /// `class`/`struct` — also a valid path segment for nested-class
    /// lookup, AND the leaf of a qualified path.
    pub class_type: bool,
    /// `using X = …;` — NOT a path segment in C++, shadows namespaces.
    pub type_alias: bool,
    /// Free function (and templates) — NOT a path segment, shadows
    /// namespaces under ambient lookup at the same scope.
    pub function: bool,
    /// Global variable — NOT a path segment, shadows namespaces.
    pub variable: bool,
    /// `enum class X` — NOT a path segment (we don't emit nested
    /// enumerators as scoped accessible-via-`::`).
    pub enum_class: bool,
}

impl CategorySet {
    /// True if this name can safely appear as a *middle* segment of a
    /// qualified path at this scope. Middle segments must resolve to a
    /// namespace; any non-namespace co-resident category (function,
    /// type-alias, variable) is a shadowing risk because C++ ambient
    /// lookup will pick the non-namespace meaning first.
    pub fn safe_as_middle_segment(self) -> bool {
        self.namespace && !self.function && !self.type_alias && !self.variable
    }

    /// True if this name can be the *last* segment of a qualified
    /// path (a referenced class type or namespace). Class types are
    /// fine as leaves; namespaces are also fine when the path ends
    /// here. Functions/aliases/variables would shadow the intended
    /// leaf.
    pub fn safe_as_leaf_segment(self) -> bool {
        (self.namespace || self.class_type)
            && !self.function
            && !self.type_alias
            && !self.variable
    }

    fn merge(&mut self, other: CategorySet) {
        self.namespace |= other.namespace;
        self.class_type |= other.class_type;
        self.type_alias |= other.type_alias;
        self.function |= other.function;
        self.variable |= other.variable;
        self.enum_class |= other.enum_class;
    }

    fn namespace_only() -> Self {
        Self {
            namespace: true,
            ..Self::default()
        }
    }

    fn class_only() -> Self {
        Self {
            class_type: true,
            ..Self::default()
        }
    }

    fn type_alias_only() -> Self {
        Self {
            type_alias: true,
            ..Self::default()
        }
    }

    fn function_only() -> Self {
        Self {
            function: true,
            ..Self::default()
        }
    }

    fn variable_only() -> Self {
        Self {
            variable: true,
            ..Self::default()
        }
    }

    fn enum_class_only() -> Self {
        Self {
            enum_class: true,
            ..Self::default()
        }
    }
}

/// Per-scope classification of every declared name in the module.
///
/// Keys are `(scope_path, name)`. `scope_path` is the C++ namespace
/// nesting from the crate root, in segments. `name` is the unqualified
/// identifier as it appears in the C++ emit.
#[derive(Debug, Default, Clone)]
pub(crate) struct SymbolCategoryTable {
    by_scope: HashMap<(Vec<String>, String), CategorySet>,
}

impl SymbolCategoryTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop all entries — called from `reset_per_file_state` so a new
    /// `emit_file` call starts with a clean table.
    pub fn clear(&mut self) {
        self.by_scope.clear();
    }

    /// Walk every `syn::Item` recursively and record its category at
    /// the current `scope_path`. Re-entrant; `scope_path` accumulates
    /// as we descend into `ItemMod` content.
    pub fn populate_from_items(&mut self, items: &[syn::Item], scope_path: &[String]) {
        for item in items {
            self.populate_one(item, scope_path);
        }
    }

    fn populate_one(&mut self, item: &syn::Item, scope_path: &[String]) {
        match item {
            syn::Item::Mod(m) => {
                self.record(scope_path, &m.ident.to_string(), CategorySet::namespace_only());
                if let Some((_, sub_items)) = &m.content {
                    let mut nested = scope_path.to_vec();
                    nested.push(m.ident.to_string());
                    self.populate_from_items(sub_items, &nested);
                }
            }
            syn::Item::Struct(s) => {
                self.record(scope_path, &s.ident.to_string(), CategorySet::class_only());
            }
            syn::Item::Enum(e) => {
                // We treat `enum` as `enum_class_only` — data enums
                // emit a `using EnumName = std::variant<…>` alias,
                // not a class. For unit-only enums we emit `enum
                // class EnumName`. Either way, the name shouldn't
                // appear as a *middle* path segment.
                self.record(scope_path, &e.ident.to_string(), CategorySet::enum_class_only());
            }
            syn::Item::Trait(t) => {
                // The trait class itself.
                self.record(scope_path, &t.ident.to_string(), CategorySet::class_only());
                // The generated `<Trait>Traits` helper template.
                let helper_name = format!("{}Traits", t.ident);
                self.record(scope_path, &helper_name, CategorySet::class_only());
            }
            syn::Item::Fn(f) => {
                self.record(
                    scope_path,
                    &f.sig.ident.to_string(),
                    CategorySet::function_only(),
                );
            }
            syn::Item::Type(t) => {
                self.record(scope_path, &t.ident.to_string(), CategorySet::type_alias_only());
            }
            syn::Item::Const(c) => {
                self.record(scope_path, &c.ident.to_string(), CategorySet::variable_only());
            }
            syn::Item::Static(s) => {
                self.record(scope_path, &s.ident.to_string(), CategorySet::variable_only());
            }
            syn::Item::Use(_) => {
                // Use imports complicate lookup: a `use X::Y as Z;`
                // adds `Z` to the current scope as an alias whose
                // category mirrors `Y`'s. Modeling this correctly
                // requires resolving the import target, which depends
                // on the full table being built first. Phase 1 of
                // this fix accepts that `use`-imported names are
                // missing from the table — the qualification logic
                // falls back to unqualified emission for any name it
                // can't find. That preserves correctness at the cost
                // of some missed qualification opportunities.
            }
            _ => {}
        }
    }

    fn record(&mut self, scope_path: &[String], name: &str, cat: CategorySet) {
        let key = (scope_path.to_vec(), name.to_string());
        self.by_scope.entry(key).or_default().merge(cat);
    }

    /// Look up a name as it would resolve via C++ ambient lookup from
    /// `scope_path`. Walks up parent scopes until a hit is found.
    /// Returns `None` if the name isn't recorded at any visible scope.
    ///
    /// This mirrors what the C++ compiler does for *unqualified*
    /// lookup of `Name` from inside `scope_path` — the first scope
    /// (working outward) where the name is recorded wins, and that's
    /// what determines the name's C++ category at the use site.
    pub fn lookup_with_ambient(&self, scope_path: &[String], name: &str) -> Option<CategorySet> {
        let mut path: Vec<String> = scope_path.to_vec();
        loop {
            let key = (path.clone(), name.to_string());
            if let Some(cats) = self.by_scope.get(&key) {
                return Some(*cats);
            }
            if path.is_empty() {
                return None;
            }
            path.pop();
        }
    }

    /// True when every segment of `segments` resolves unambiguously
    /// to a namespace (for middle segments) or namespace/class (for
    /// the leaf), starting from `use_site_scope` and walking via
    /// ambient lookup at the first segment.
    ///
    /// This is the per-use-site validity check that gates trait-helper
    /// qualification.
    pub fn path_resolves_unambiguously(
        &self,
        use_site_scope: &[String],
        segments: &[String],
    ) -> bool {
        if segments.is_empty() {
            return false;
        }
        // Resolve the first segment via ambient lookup from the use
        // site — that simulates the C++ compiler walking up scopes
        // for the leading identifier.
        let head = &segments[0];
        let head_cats = match self.lookup_with_ambient(use_site_scope, head) {
            Some(c) => c,
            None => return false,
        };
        let is_last = segments.len() == 1;
        if is_last {
            return head_cats.safe_as_leaf_segment();
        }
        if !head_cats.safe_as_middle_segment() {
            return false;
        }
        // For each subsequent segment, look it up *inside* the scope
        // built so far (no ambient walk — the path is now absolute
        // from the resolved head).
        //
        // To find the scope path of the resolved head: it's whichever
        // ancestor of `use_site_scope` recorded the head as a
        // namespace. Walk up to find that scope.
        let head_scope = self.find_head_scope(use_site_scope, head);
        let head_scope = match head_scope {
            Some(s) => s,
            None => return false,
        };
        let mut current_scope = head_scope;
        current_scope.push(head.clone());
        for (i, seg) in segments.iter().enumerate().skip(1) {
            let is_leaf = i == segments.len() - 1;
            let key = (current_scope.clone(), seg.clone());
            let cats = match self.by_scope.get(&key) {
                Some(c) => *c,
                None => return false,
            };
            let ok = if is_leaf {
                cats.safe_as_leaf_segment()
            } else {
                cats.safe_as_middle_segment()
            };
            if !ok {
                return false;
            }
            current_scope.push(seg.clone());
        }
        true
    }

    fn find_head_scope(&self, use_site_scope: &[String], head: &str) -> Option<Vec<String>> {
        let mut path: Vec<String> = use_site_scope.to_vec();
        loop {
            let key = (path.clone(), head.to_string());
            if self.by_scope.contains_key(&key) {
                return Some(path);
            }
            if path.is_empty() {
                return None;
            }
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn namespace_only_path_resolves() {
        let mut t = SymbolCategoryTable::new();
        let items: Vec<syn::Item> = vec![
            parse_quote! { pub mod combinations { pub struct PoolIndexTraits; } },
        ];
        t.populate_from_items(&items, &[]);
        // From the crate-root scope, `combinations::PoolIndexTraits`
        // should resolve.
        assert!(t.path_resolves_unambiguously(
            &[],
            &["combinations".to_string(), "PoolIndexTraits".to_string()]
        ));
    }

    #[test]
    fn function_shadowing_breaks_middle_segment() {
        let mut t = SymbolCategoryTable::new();
        // Mirrors itertools' `adaptors` layout: a `coalesce`
        // submodule with a `CountItemTraits` helper, AND a sibling
        // `coalesce` function template in the same parent scope.
        let items: Vec<syn::Item> = vec![parse_quote! {
            pub mod adaptors {
                pub mod coalesce {
                    pub struct CountItemTraits;
                }
                pub fn coalesce<I, F>(iter: I, f: F) -> () {}
            }
        }];
        t.populate_from_items(&items, &[]);
        // From the crate root, the path `adaptors::coalesce::
        // CountItemTraits` should NOT be considered safely
        // qualifiable, because `coalesce` at scope `adaptors` is BOTH
        // namespace AND function.
        let safe = t.path_resolves_unambiguously(
            &[],
            &[
                "adaptors".to_string(),
                "coalesce".to_string(),
                "CountItemTraits".to_string(),
            ],
        );
        assert!(!safe, "function shadowing should make middle segment unsafe");
    }

    #[test]
    fn ambient_lookup_walks_up_scopes() {
        let mut t = SymbolCategoryTable::new();
        let items: Vec<syn::Item> = vec![parse_quote! {
            pub mod outer {
                pub struct Foo;
                pub mod inner {}
            }
        }];
        t.populate_from_items(&items, &[]);
        // From scope `["outer", "inner"]`, ambient lookup of `Foo`
        // should find `outer::Foo`.
        let cats = t.lookup_with_ambient(&["outer".to_string(), "inner".to_string()], "Foo");
        assert!(cats.is_some());
        assert!(cats.unwrap().class_type);
    }

    #[test]
    fn missing_name_returns_none_from_resolver() {
        let t = SymbolCategoryTable::new();
        // Empty table — nothing resolves.
        assert!(!t.path_resolves_unambiguously(&[], &["anything".to_string()]));
    }

    #[test]
    fn trait_emits_both_self_and_helper_class_categories() {
        let mut t = SymbolCategoryTable::new();
        let items: Vec<syn::Item> = vec![parse_quote! {
            pub mod combinations {
                pub trait PoolIndex { type Item; }
            }
        }];
        t.populate_from_items(&items, &[]);
        // Both `PoolIndex` (the trait class) and `PoolIndexTraits`
        // (the assoc-type helper template) should be recorded as
        // class_type at scope `["combinations"]`.
        let p = t.lookup_with_ambient(&["combinations".to_string()], "PoolIndex");
        assert!(p.is_some_and(|c| c.class_type));
        let h = t.lookup_with_ambient(&["combinations".to_string()], "PoolIndexTraits");
        assert!(h.is_some_and(|c| c.class_type));
    }
}
