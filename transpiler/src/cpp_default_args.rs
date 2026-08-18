use crate::types::UserTypeMap;
use quote::ToTokens;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path as FsPath;
use std::path::PathBuf;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Attribute, FnArg, Item, ItemFn, Meta, Path, Token, Type, Visibility};

const MARKER: &str = "cpp_default_argument";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CppDefaultArgument {
    SourceLocation,
    Stderr,
}

impl CppDefaultArgument {
    fn source_type_description(self) -> &'static str {
        match self {
            Self::SourceLocation => "&::rusty::SourceLocation",
            Self::Stderr => "*mut ::rusty::CFile",
        }
    }

    fn required_mapping(self) -> (&'static str, &'static str) {
        match self {
            Self::SourceLocation => ("rusty::SourceLocation", "std::source_location"),
            Self::Stderr => ("rusty::CFile", "FILE"),
        }
    }

    pub(crate) fn cpp_expression(self) -> &'static str {
        match self {
            Self::SourceLocation => "std::source_location::current()",
            Self::Stderr => "stderr",
        }
    }

    pub(crate) fn required_cpp_parameter_type(self) -> &'static str {
        match self {
            Self::SourceLocation => "const std::source_location&",
            Self::Stderr => "FILE*",
        }
    }
}

fn ident_text(ident: &proc_macro2::Ident) -> String {
    let spelling = ident.to_string();
    spelling.strip_prefix("r#").unwrap_or(&spelling).to_string()
}

fn path_is_one_ident(path: &Path, expected: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.first().is_some_and(|segment| {
            ident_text(&segment.ident) == expected
                && segment.ident.to_string() == expected
                && matches!(segment.arguments, syn::PathArguments::None)
        })
}

fn token_stream_marker_count(tokens: proc_macro2::TokenStream) -> usize {
    tokens
        .into_iter()
        .map(|token| match token {
            proc_macro2::TokenTree::Ident(ident) => usize::from(ident_text(&ident) == MARKER),
            proc_macro2::TokenTree::Group(group) => token_stream_marker_count(group.stream()),
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => 0,
        })
        .sum()
}

pub(crate) fn source_mentions_marker(source: &str) -> bool {
    match source.parse::<proc_macro2::TokenStream>() {
        Ok(tokens) => token_stream_marker_count(tokens) != 0,
        Err(_) => source.contains(MARKER),
    }
}

fn attr_mentions_marker(attr: &Attribute) -> bool {
    token_stream_marker_count(attr.meta.to_token_stream()) != 0
}

fn parse_kind_attr(attr: &Attribute) -> Result<CppDefaultArgument, String> {
    if !path_is_one_ident(attr.path(), "cfg_attr") {
        return Err(format!(
            "{MARKER} must use the exact inert form #[cfg_attr(any(), {MARKER}(source_location|stderr))]"
        ));
    }
    let args = attr
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map_err(|error| format!("could not parse {MARKER} marker: {error}"))?;
    if args.len() != 2 || args.trailing_punct() {
        return Err(format!(
            "{MARKER} cfg_attr must contain exactly any() and one default kind"
        ));
    }
    let mut args = args.iter();
    let Some(Meta::List(predicate)) = args.next() else {
        return Err(format!("{MARKER} must be guarded by exact any()"));
    };
    if !path_is_one_ident(&predicate.path, "any") || !predicate.tokens.is_empty() {
        return Err(format!("{MARKER} must be guarded by exact any()"));
    }
    let Some(Meta::List(marker)) = args.next() else {
        return Err(format!("{MARKER} requires one parenthesized default kind"));
    };
    if !path_is_one_ident(&marker.path, MARKER) {
        return Err(format!(
            "{MARKER} must be unqualified and use its exact reserved spelling"
        ));
    }
    let kind = syn::parse2::<Path>(marker.tokens.clone())
        .map_err(|_| format!("{MARKER} requires one bare identifier kind"))?;
    if path_is_one_ident(&kind, "source_location") {
        Ok(CppDefaultArgument::SourceLocation)
    } else if path_is_one_ident(&kind, "stderr") {
        Ok(CppDefaultArgument::Stderr)
    } else {
        Err(format!(
            "unsupported {MARKER} kind; expected source_location or stderr"
        ))
    }
}

pub(crate) fn parameter_kind(arg: &syn::PatType) -> Result<Option<CppDefaultArgument>, String> {
    let mut found = None;
    for attr in &arg.attrs {
        if !attr_mentions_marker(attr) {
            continue;
        }
        let kind = parse_kind_attr(attr)?;
        if found.replace(kind).is_some() {
            return Err(format!("duplicate {MARKER} marker on one parameter"));
        }
    }
    if found.is_some() && arg.attrs.len() != 1 {
        return Err(format!(
            "{MARKER} cannot be combined with any other parameter attribute"
        ));
    }
    Ok(found)
}

pub(crate) fn function_has_defaults(function: &ItemFn) -> Result<bool, String> {
    for input in &function.sig.inputs {
        if let FnArg::Typed(arg) = input
            && parameter_kind(arg)?.is_some()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn exact_path_type(ty: &Type, expected: &[&str]) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path.path.leading_colon.is_some()
        && path.path.segments.len() == expected.len()
        && path
            .path
            .segments
            .iter()
            .zip(expected)
            .all(|(segment, expected)| {
                segment.ident.to_string() == *expected
                    && matches!(segment.arguments, syn::PathArguments::None)
            })
}

#[derive(Default)]
struct DefaultFacadeTypeVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for DefaultFacadeTypeVisitor {
    fn visit_type(&mut self, ty: &'ast Type) {
        self.found |= exact_path_type(ty, &["rusty", "SourceLocation"])
            || exact_path_type(ty, &["rusty", "CFile"]);
        if !self.found {
            syn::visit::visit_type(self, ty);
        }
    }
}

fn type_mentions_default_facade(ty: &Type) -> bool {
    let mut visitor = DefaultFacadeTypeVisitor::default();
    visitor.visit_type(ty);
    visitor.found
}

fn source_type_matches(kind: CppDefaultArgument, ty: &Type) -> bool {
    match (kind, ty) {
        (CppDefaultArgument::SourceLocation, Type::Reference(reference)) => {
            reference.mutability.is_none()
                && reference.lifetime.is_none()
                && exact_path_type(&reference.elem, &["rusty", "SourceLocation"])
        }
        (CppDefaultArgument::Stderr, Type::Ptr(pointer)) => {
            pointer.mutability.is_some()
                && pointer.const_token.is_none()
                && exact_path_type(&pointer.elem, &["rusty", "CFile"])
        }
        _ => false,
    }
}

fn function_attribute_is_audited_inert(attr: &Attribute) -> bool {
    path_is_one_ident(attr.path(), "doc") || path_is_one_ident(attr.path(), "allow")
}

fn parameter_pattern_is_plain_ident(arg: &syn::PatType) -> bool {
    matches!(
        arg.pat.as_ref(),
        syn::Pat::Ident(ident)
            if ident.by_ref.is_none()
                && ident.mutability.is_none()
                && ident.subpat.is_none()
    )
}

fn validate_mapping(kind: CppDefaultArgument, type_map: &UserTypeMap) -> Result<(), String> {
    let (rust_type, expected_cpp) = kind.required_mapping();
    match type_map.lookup(rust_type) {
        Some(actual) if actual == expected_cpp => Ok(()),
        Some(actual) => Err(format!(
            "{MARKER} requires type map {rust_type} = \"{expected_cpp}\", found \"{actual}\""
        )),
        None => Err(format!(
            "{MARKER} requires type map {rust_type} = \"{expected_cpp}\""
        )),
    }
}

fn validate_function(function: &ItemFn, type_map: Option<&UserTypeMap>) -> Result<usize, String> {
    let mut kinds = Vec::with_capacity(function.sig.inputs.len());
    for input in &function.sig.inputs {
        let kind = match input {
            FnArg::Typed(arg) => parameter_kind(arg)?,
            FnArg::Receiver(_) => None,
        };
        kinds.push(kind);
    }
    let marker_count = kinds.iter().filter(|kind| kind.is_some()).count();
    if marker_count == 0 {
        return Ok(0);
    }
    if !matches!(function.vis, Visibility::Public(_)) {
        return Err(format!(
            "{MARKER} is supported only on public free functions; `{}` is not public",
            function.sig.ident
        ));
    }
    if function
        .attrs
        .iter()
        .any(|attr| !function_attribute_is_audited_inert(attr))
    {
        return Err(format!(
            "{MARKER} function `{}` may carry only inert doc and allow attributes",
            function.sig.ident
        ));
    }
    if function.sig.asyncness.is_some()
        || function.sig.constness.is_some()
        || function.sig.abi.is_some()
        || function.sig.variadic.is_some()
    {
        return Err(format!(
            "{MARKER} is unsupported on async, const, extern, or variadic function `{}`",
            function.sig.ident
        ));
    }
    if let syn::ReturnType::Type(_, return_type) = &function.sig.output {
        if type_mentions_default_facade(return_type) {
            return Err(format!(
                "{MARKER} function `{}` cannot expose its default-only facade type in the return position",
                function.sig.ident
            ));
        }
    }
    let first_default = kinds
        .iter()
        .position(Option::is_some)
        .expect("marker_count is nonzero");
    if kinds[first_default..].iter().any(Option::is_none) {
        return Err(format!(
            "{MARKER} parameters must be a trailing contiguous suffix on `{}`",
            function.sig.ident
        ));
    }
    for (input, kind) in function.sig.inputs.iter().zip(kinds) {
        let Some(kind) = kind else {
            if let FnArg::Typed(arg) = input
                && type_mentions_default_facade(&arg.ty)
            {
                return Err(format!(
                    "{MARKER} function `{}` may use default-only facade types only on marked parameters",
                    function.sig.ident
                ));
            }
            continue;
        };
        let FnArg::Typed(arg) = input else {
            return Err(format!("{MARKER} cannot apply to a receiver"));
        };
        if !parameter_pattern_is_plain_ident(arg) {
            return Err(format!(
                "{MARKER} requires a plain immutable identifier parameter on `{}`",
                function.sig.ident
            ));
        }
        if !source_type_matches(kind, &arg.ty) {
            return Err(format!(
                "{MARKER}({}) requires exact Rust parameter type {}",
                match kind {
                    CppDefaultArgument::SourceLocation => "source_location",
                    CppDefaultArgument::Stderr => "stderr",
                },
                kind.source_type_description()
            ));
        }
        if let Some(type_map) = type_map {
            validate_mapping(kind, type_map)?;
        }
    }
    Ok(marker_count)
}

fn validate_items(items: &[Item], type_map: Option<&UserTypeMap>) -> Result<usize, String> {
    let mut marker_count = 0;
    for item in items {
        match item {
            Item::Fn(function) => marker_count += validate_function(function, type_map)?,
            _ => {}
        }
    }
    Ok(marker_count)
}

fn validate_file_impl(
    file: &syn::File,
    type_map: Option<&UserTypeMap>,
    strict_signature_closure: bool,
) -> Result<bool, String> {
    let mentioned = token_stream_marker_count(file.to_token_stream());
    if mentioned == 0 {
        return Ok(false);
    }
    struct ReservedFacadeAliasVisitor {
        found: bool,
    }
    impl<'ast> Visit<'ast> for ReservedFacadeAliasVisitor {
        fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
            let bound = item
                .rename
                .as_ref()
                .map(|(_, name)| name)
                .unwrap_or(&item.ident);
            if ident_text(bound) == "rusty" {
                self.found = true;
            }
        }
    }
    let mut facade_alias = ReservedFacadeAliasVisitor { found: false };
    facade_alias.visit_file(file);
    if facade_alias.found {
        return Err(format!(
            "{MARKER} reserves absolute ::rusty facade paths; source-defined extern-crate aliases named rusty are unsupported"
        ));
    }
    for attr in &file.attrs {
        if !["doc", "allow", "warn", "deny", "forbid", "expect"]
            .iter()
            .any(|name| path_is_one_ident(attr.path(), name))
        {
            return Err(format!(
                "{MARKER} source files support only doc and lint-level inner attributes; found `{}`",
                attr.path().to_token_stream()
            ));
        }
    }
    let validated = validate_items(&file.items, type_map)?;
    if validated != mentioned {
        return Err(format!(
            "reserved {MARKER} marker is allowed only in the exact inert attribute on a trailing parameter of a public free function"
        ));
    }
    if validated != 0 {
        validate_default_signature_types(&[(Vec::new(), file)], strict_signature_closure)?;
    }
    Ok(true)
}

#[derive(Clone)]
struct TypeAliasDecl {
    module: Vec<String>,
    ty: Type,
    type_parameters: BTreeSet<String>,
    const_parameters: BTreeSet<String>,
}

#[derive(Clone)]
struct ConstDecl {
    module: Vec<String>,
    ty: Type,
    expression: syn::Expr,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ImportOrigin {
    /// `crate::`, `self::`, `super::`, or `extern crate self as ...`.
    Local,
    /// A 2018+ uniform path whose first segment may name either a local
    /// binding or an extern-prelude crate. Resolution is deferred until the
    /// complete source model has been collected.
    Unqualified,
    /// A leading-`::` use or a non-self `extern crate` binding.
    External,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ImportTarget {
    origin: ImportOrigin,
    module: Vec<String>,
    segments: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ResolvedOrigin {
    Local,
    External,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResolvedPath {
    origin: ResolvedOrigin,
    segments: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
enum ImportedSymbolKind {
    TypeAlias,
    Const,
}

impl ImportedSymbolKind {
    fn description(self) -> &'static str {
        match self {
            Self::TypeAlias => "type-alias",
            Self::Const => "const",
        }
    }

    fn is_terminal(self, model: &DefaultSignatureTypes, path: &[String]) -> bool {
        match self {
            Self::TypeAlias => model.aliases.contains_key(path),
            Self::Const => model.constants.contains_key(path),
        }
    }
}

#[derive(Default)]
struct GlobResolution {
    local_paths: Vec<Vec<String>>,
    reached_external_origin: bool,
}

fn path_is_builtin_derive(path: &Path) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.first().is_some_and(|segment| {
            matches!(
                ident_text(&segment.ident).as_str(),
                "Clone"
                    | "Copy"
                    | "Debug"
                    | "Default"
                    | "Eq"
                    | "Hash"
                    | "Ord"
                    | "PartialEq"
                    | "PartialOrd"
            ) && matches!(segment.arguments, syn::PathArguments::None)
        })
}

fn cfg_predicate_is_definitely_false(meta: &Meta) -> bool {
    matches!(meta, Meta::List(list) if path_is_one_ident(&list.path, "any") && list.tokens.is_empty())
}

fn module_has_potential_macro_import(
    model: &DefaultSignatureTypes,
    module: &[String],
    name: &str,
) -> bool {
    model
        .imports
        .get(module)
        .is_some_and(|imports| imports.contains_key(name))
        || (!module.is_empty()
            && model
                .imports
                .get(&[][..])
                .is_some_and(|imports| imports.contains_key(name)))
        || model
            .ambiguous_imports
            .contains(&(module.to_vec(), name.to_string()))
        || (!module.is_empty()
            && model
                .ambiguous_imports
                .contains(&(Vec::new(), name.to_string())))
        || model
            .glob_imports
            .get(module)
            .is_some_and(|imports| !imports.is_empty())
        || (!module.is_empty()
            && model
                .glob_imports
                .get(&[][..])
                .is_some_and(|imports| !imports.is_empty()))
}

fn item_attribute_cannot_generate_bindings(
    meta: &Meta,
    module: &[String],
    model: &DefaultSignatureTypes,
    cpp_inherit_item_form: bool,
) -> Result<bool, String> {
    let path = meta.path();
    let Some(name) = path
        .segments
        .first()
        .filter(|_| path.leading_colon.is_none() && path.segments.len() == 1)
        .map(|segment| ident_text(&segment.ident))
    else {
        return Ok(false);
    };
    if matches!(
        name.as_str(),
        "allow"
            | "warn"
            | "deny"
            | "forbid"
            | "expect"
            | "doc"
            | "cfg"
            | "repr"
            | "non_exhaustive"
            | "deprecated"
            | "must_use"
            | "cold"
            | "inline"
            | "track_caller"
            | "target_feature"
            | "no_mangle"
            | "export_name"
            | "link_section"
            | "used"
            | "test"
            | "bench"
            | "ignore"
            | "should_panic"
            // `#[path]` selects which file backs a module declaration. It
            // introduces no items of its own, and the file it selects is
            // already part of the audited unit set: crate-source discovery
            // resolves the same attribute, and cpp_abi's module graph rejects
            // it in every position discovery does not follow. Like the rest of
            // this family it is a built-in name, so it stays subject to the
            // macro-shadowing test below.
            | "path"
    ) {
        return Ok(!module_has_potential_macro_import(model, module, &name));
    }
    if name == "cpp_inherit" {
        // This is an established source-owned transpiler marker. Crate mode's
        // reserved `rusty` facade validation and exact codegen gate own its
        // no-op Rust expansion; unlike an arbitrary attribute, its C++
        // semantics are interpreted explicitly by this transpiler. Do not
        // permit an arbitrary proc macro to acquire this trusted spelling via
        // `use ... as cpp_inherit`, to attach it to anything except its
        // supported impl-item form, or to run in an ancestor of a relevant
        // signature module.
        let can_affect_relevant_module = model
            .relevant_modules
            .borrow()
            .iter()
            .any(|relevant| relevant.starts_with(module));
        if !cpp_inherit_item_form || !matches!(meta, Meta::Path(_)) || can_affect_relevant_module {
            return Ok(false);
        }
        let Some(target) = model
            .imports
            .get(module)
            .and_then(|imports| imports.get("cpp_inherit"))
        else {
            return Ok(false);
        };
        let target = materialize_import_target(model, target)?;
        return Ok(target.origin == ResolvedOrigin::External
            && target.segments == ["rusty", "cpp_inherit"]);
    }
    if name == "derive" {
        let Meta::List(list) = meta else {
            return Ok(false);
        };
        let derives = list
            .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
            .map_err(|error| {
                format!(
                    "{MARKER} cannot parse derive surface while auditing macro-generated bindings: {error}"
                )
            })?;
        if derives.is_empty() || derives.trailing_punct() {
            return Ok(false);
        }
        for derive in &derives {
            if !path_is_builtin_derive(derive) {
                return Ok(false);
            }
            let derive_name = ident_text(
                &derive
                    .segments
                    .first()
                    .expect("builtin derive path has one segment")
                    .ident,
            );
            if module_has_potential_macro_import(model, module, &derive_name) {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if name == "cfg_attr" {
        let Meta::List(list) = meta else {
            return Ok(false);
        };
        let arguments = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_err(|error| {
                format!(
                    "{MARKER} cannot parse cfg_attr while auditing macro-generated bindings: {error}"
                )
            })?;
        let Some(predicate) = arguments.first() else {
            return Ok(false);
        };
        if cfg_predicate_is_definitely_false(predicate) {
            return Ok(true);
        }
        for nested in arguments.iter().skip(1) {
            if !item_attribute_cannot_generate_bindings(
                nested,
                module,
                model,
                cpp_inherit_item_form,
            )? {
                return Ok(false);
            }
        }
        return Ok(arguments.len() > 1 && !arguments.trailing_punct());
    }
    Ok(false)
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        Item::Verbatim(_) | _ => &[],
    }
}

fn validate_associated_macro_surfaces(
    item: &Item,
    module: &[String],
    model: &DefaultSignatureTypes,
) -> Result<(), String> {
    let validate_attrs = |attrs: &[Attribute], owner: &str| -> Result<(), String> {
        for attr in attrs {
            if !item_attribute_cannot_generate_bindings(&attr.meta, module, model, false)? {
                return Err(format!(
                    "{MARKER} cannot prove that attribute `{}` on {owner} is free of macro-generated bindings in module `{}`",
                    attr.meta.to_token_stream(),
                    module.join("::")
                ));
            }
        }
        Ok(())
    };

    match item {
        Item::Trait(item_trait) => {
            for associated in &item_trait.items {
                match associated {
                    syn::TraitItem::Const(item) => validate_attrs(&item.attrs, "trait const")?,
                    syn::TraitItem::Fn(item) => validate_attrs(&item.attrs, "trait function")?,
                    syn::TraitItem::Type(item) => validate_attrs(&item.attrs, "trait type")?,
                    syn::TraitItem::Macro(item) => {
                        return Err(format!(
                            "{MARKER} cannot prove trait item macro `{}` is free of macro-generated associated bindings in module `{}`",
                            item.mac.path.to_token_stream(),
                            module.join("::")
                        ));
                    }
                    syn::TraitItem::Verbatim(tokens) => {
                        return Err(format!(
                            "{MARKER} cannot prove verbatim trait item `{tokens}` is free of macro-generated bindings in module `{}`",
                            module.join("::")
                        ));
                    }
                    _ => {}
                }
            }
        }
        Item::Impl(item_impl) => {
            for associated in &item_impl.items {
                match associated {
                    syn::ImplItem::Const(item) => validate_attrs(&item.attrs, "impl const")?,
                    syn::ImplItem::Fn(item) => validate_attrs(&item.attrs, "impl function")?,
                    syn::ImplItem::Type(item) => validate_attrs(&item.attrs, "impl type")?,
                    syn::ImplItem::Macro(item) => {
                        return Err(format!(
                            "{MARKER} cannot prove impl item macro `{}` is free of macro-generated associated bindings in module `{}`",
                            item.mac.path.to_token_stream(),
                            module.join("::")
                        ));
                    }
                    syn::ImplItem::Verbatim(tokens) => {
                        return Err(format!(
                            "{MARKER} cannot prove verbatim impl item `{tokens}` is free of macro-generated bindings in module `{}`",
                            module.join("::")
                        ));
                    }
                    _ => {}
                }
            }
        }
        Item::ForeignMod(item_foreign) => {
            for foreign in &item_foreign.items {
                match foreign {
                    syn::ForeignItem::Fn(item) => validate_attrs(&item.attrs, "foreign function")?,
                    syn::ForeignItem::Static(item) => {
                        validate_attrs(&item.attrs, "foreign static")?
                    }
                    syn::ForeignItem::Type(item) => validate_attrs(&item.attrs, "foreign type")?,
                    syn::ForeignItem::Macro(item) => {
                        return Err(format!(
                            "{MARKER} cannot prove foreign item macro `{}` is free of macro-generated bindings in module `{}`",
                            item.mac.path.to_token_stream(),
                            module.join("::")
                        ));
                    }
                    syn::ForeignItem::Verbatim(tokens) => {
                        return Err(format!(
                            "{MARKER} cannot prove verbatim foreign item `{tokens}` is free of macro-generated bindings in module `{}`",
                            module.join("::")
                        ));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_binding_macro_surfaces(
    items: &[Item],
    module: &[String],
    model: &DefaultSignatureTypes,
) -> Result<(), String> {
    for item in items {
        if let Item::Macro(item_macro) = item {
            return Err(format!(
                "{MARKER} cannot prove item macro `{}` is free of macro-generated bindings in module `{}`",
                item_macro.mac.path.to_token_stream(),
                module.join("::")
            ));
        }
        if let Item::Verbatim(tokens) = item {
            return Err(format!(
                "{MARKER} cannot prove verbatim item `{tokens}` is free of macro-generated bindings in module `{}`",
                module.join("::")
            ));
        }
        for attr in item_attributes(item) {
            if !item_attribute_cannot_generate_bindings(
                &attr.meta,
                module,
                model,
                matches!(item, Item::Impl(_)),
            )? {
                return Err(format!(
                    "{MARKER} cannot prove that item attribute `{}` is free of macro-generated bindings in module `{}`",
                    attr.meta.to_token_stream(),
                    module.join("::")
                ));
            }
        }
        validate_associated_macro_surfaces(item, module, model)?;
        if let Item::Mod(item_mod) = item
            && let Some((_, nested)) = &item_mod.content
        {
            let mut nested_module = module.to_vec();
            nested_module.push(ident_text(&item_mod.ident));
            validate_binding_macro_surfaces(nested, &nested_module, model)?;
        }
    }
    Ok(())
}

#[derive(Default)]
struct DefaultSignatureTypes {
    aliases: BTreeMap<Vec<String>, TypeAliasDecl>,
    aliases_by_name: BTreeMap<String, Vec<Vec<String>>>,
    constants: BTreeMap<Vec<String>, Vec<ConstDecl>>,
    constants_by_name: BTreeMap<String, Vec<Vec<String>>>,
    imports: BTreeMap<Vec<String>, BTreeMap<String, ImportTarget>>,
    glob_imports: BTreeMap<Vec<String>, Vec<ImportTarget>>,
    ambiguous_imports: BTreeSet<(Vec<String>, String)>,
    local_type_bindings: BTreeSet<(Vec<String>, String)>,
    local_nonmodule_type_bindings: BTreeSet<(Vec<String>, String)>,
    associated_type_names: BTreeSet<String>,
    associated_const_names: BTreeSet<String>,
    relevant_modules: RefCell<BTreeSet<Vec<String>>>,
}

fn conventional_module_path(path: &FsPath) -> Result<Vec<String>, String> {
    let mut components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let Some(file) = components.pop() else {
        return Err(format!(
            "{MARKER} cannot determine the Rust module for empty source path"
        ));
    };
    let stem = if file == "lib.rs" || file == "main.rs" || file == "mod.rs" {
        None
    } else {
        file.strip_suffix(".rs").map(str::to_string)
    };
    if file != "lib.rs" && file != "main.rs" && file != "mod.rs" && stem.is_none() {
        return Err(format!(
            "{MARKER} source path `{}` is not a conventional Rust source file",
            path.display()
        ));
    }
    if components
        .first()
        .is_some_and(|component| component == "src")
    {
        components.remove(0);
    }
    if let Some(stem) = stem {
        components.push(stem);
    }
    Ok(components)
}

fn collect_use_bindings(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut Vec<(String, Vec<String>)>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(ident_text(&path.ident));
            collect_use_bindings(&path.tree, prefix, out, globs);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let name = ident_text(&name.ident);
            if name == "self" {
                if let Some(binding) = prefix.last().cloned() {
                    out.push((binding, prefix.clone()));
                }
            } else {
                let mut target = prefix.clone();
                target.push(name.clone());
                out.push((name, target));
            }
        }
        syn::UseTree::Rename(rename) => {
            let mut target = prefix.clone();
            target.push(ident_text(&rename.ident));
            out.push((ident_text(&rename.rename), target));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_bindings(item, prefix, out, globs);
            }
        }
        syn::UseTree::Glob(_) => globs.push(prefix.clone()),
    }
}

fn normalize_local_segments(module: &[String], segments: &[String]) -> Option<Vec<String>> {
    let mut index = 0;
    let mut normalized = if segments.first().is_some_and(|segment| segment == "crate") {
        index = 1;
        Vec::new()
    } else if segments.first().is_some_and(|segment| segment == "self") {
        index = 1;
        module.to_vec()
    } else if segments.first().is_some_and(|segment| segment == "super") {
        let mut base = module.to_vec();
        while segments
            .get(index)
            .is_some_and(|segment| segment == "super")
        {
            base.pop()?;
            index += 1;
        }
        base
    } else {
        module.to_vec()
    };
    normalized.extend(segments[index..].iter().cloned());
    Some(normalized)
}

fn import_target(
    module: &[String],
    leading_colon: bool,
    segments: Vec<String>,
) -> Option<ImportTarget> {
    if leading_colon {
        return Some(ImportTarget {
            origin: ImportOrigin::External,
            module: module.to_vec(),
            segments,
        });
    }
    if matches!(
        segments.first().map(String::as_str),
        Some("crate" | "self" | "super")
    ) {
        return normalize_local_segments(module, &segments).map(|segments| ImportTarget {
            origin: ImportOrigin::Local,
            module: module.to_vec(),
            segments,
        });
    }
    Some(ImportTarget {
        origin: ImportOrigin::Unqualified,
        module: module.to_vec(),
        segments,
    })
}

fn record_import_binding(
    model: &mut DefaultSignatureTypes,
    module: &[String],
    binding: String,
    target: ImportTarget,
) {
    if binding == "_" {
        return;
    }
    let ambiguity_key = (module.to_vec(), binding.clone());
    if model.ambiguous_imports.contains(&ambiguity_key) {
        return;
    }
    let module_imports = model.imports.entry(module.to_vec()).or_default();
    match module_imports.get(&binding) {
        Some(previous) if previous != &target => {
            module_imports.remove(&binding);
            model.ambiguous_imports.insert(ambiguity_key);
        }
        Some(_) => {}
        None => {
            module_imports.insert(binding, target);
        }
    }
}

fn collect_signature_type_model(
    items: &[Item],
    module: &[String],
    model: &mut DefaultSignatureTypes,
) -> Result<(), String> {
    for item in items {
        match item {
            Item::Type(alias) => {
                let name = ident_text(&alias.ident);
                model
                    .local_type_bindings
                    .insert((module.to_vec(), name.clone()));
                model
                    .local_nonmodule_type_bindings
                    .insert((module.to_vec(), name.clone()));
                let mut key = module.to_vec();
                key.push(name.clone());
                let type_parameters = alias
                    .generics
                    .type_params()
                    .map(|parameter| ident_text(&parameter.ident))
                    .collect();
                let const_parameters = alias
                    .generics
                    .const_params()
                    .map(|parameter| ident_text(&parameter.ident))
                    .collect();
                let declaration = TypeAliasDecl {
                    module: module.to_vec(),
                    ty: (*alias.ty).clone(),
                    type_parameters,
                    const_parameters,
                };
                if model.aliases.insert(key.clone(), declaration).is_some() {
                    return Err(format!(
                        "{MARKER} found duplicate type alias path `{}` while auditing a default-bearing signature",
                        key.join("::")
                    ));
                }
                model.aliases_by_name.entry(name).or_default().push(key);
            }
            Item::Const(constant) => {
                let name = ident_text(&constant.ident);
                let mut key = module.to_vec();
                key.push(name.clone());
                let declaration = ConstDecl {
                    module: module.to_vec(),
                    ty: (*constant.ty).clone(),
                    expression: (*constant.expr).clone(),
                };
                // Rust commonly spells target-dependent constants as several
                // declarations behind mutually exclusive `cfg` attributes.
                // The source audit intentionally does not evaluate cfg; retain
                // every variant and require every reachable initializer to be
                // independently provable.
                model
                    .constants
                    .entry(key.clone())
                    .or_default()
                    .push(declaration);
                model.constants_by_name.entry(name).or_default().push(key);
            }
            Item::Use(item_use) => {
                let mut bindings = Vec::new();
                let mut globs = Vec::new();
                collect_use_bindings(
                    &item_use.tree,
                    &mut Vec::new(),
                    &mut bindings,
                    &mut globs,
                );
                let leading_colon = item_use.leading_colon.is_some();
                for (binding, segments) in bindings {
                    if let Some(target) = import_target(module, leading_colon, segments) {
                        record_import_binding(model, module, binding, target);
                    }
                }
                for segments in globs {
                    if let Some(target) = import_target(module, leading_colon, segments) {
                        model
                            .glob_imports
                            .entry(module.to_vec())
                            .or_default()
                            .push(target);
                    }
                }
            }
            Item::Trait(item_trait) => {
                model
                    .local_type_bindings
                    .insert((module.to_vec(), ident_text(&item_trait.ident)));
                model
                    .local_nonmodule_type_bindings
                    .insert((module.to_vec(), ident_text(&item_trait.ident)));
                for trait_item in &item_trait.items {
                    match trait_item {
                        syn::TraitItem::Type(associated) => {
                            model
                                .associated_type_names
                                .insert(ident_text(&associated.ident));
                        }
                        syn::TraitItem::Const(associated) => {
                            model
                                .associated_const_names
                                .insert(ident_text(&associated.ident));
                        }
                        _ => {}
                    }
                }
            }
            Item::Mod(item_mod) => {
                model
                    .local_type_bindings
                    .insert((module.to_vec(), ident_text(&item_mod.ident)));
                if let Some((_, nested)) = &item_mod.content {
                    let mut nested_module = module.to_vec();
                    nested_module.push(ident_text(&item_mod.ident));
                    collect_signature_type_model(nested, &nested_module, model)?;
                }
            }
            Item::Struct(item) => {
                let name = ident_text(&item.ident);
                model
                    .local_type_bindings
                    .insert((module.to_vec(), name.clone()));
                model
                    .local_nonmodule_type_bindings
                    .insert((module.to_vec(), name));
            }
            Item::Enum(item) => {
                let name = ident_text(&item.ident);
                model
                    .local_type_bindings
                    .insert((module.to_vec(), name.clone()));
                model
                    .local_nonmodule_type_bindings
                    .insert((module.to_vec(), name));
            }
            Item::Union(item) => {
                let name = ident_text(&item.ident);
                model
                    .local_type_bindings
                    .insert((module.to_vec(), name.clone()));
                model
                    .local_nonmodule_type_bindings
                    .insert((module.to_vec(), name));
            }
            Item::ExternCrate(item) => {
                let crate_name = ident_text(&item.ident);
                let binding = item
                    .rename
                    .as_ref()
                    .map(|(_, rename)| ident_text(rename))
                    .unwrap_or_else(|| crate_name.clone());
                model
                    .local_type_bindings
                    .insert((module.to_vec(), binding.clone()));
                let target = if crate_name == "self" {
                    ImportTarget {
                        origin: ImportOrigin::Local,
                        module: module.to_vec(),
                        segments: Vec::new(),
                    }
                } else {
                    ImportTarget {
                        origin: ImportOrigin::External,
                        module: module.to_vec(),
                        segments: vec![crate_name],
                    }
                };
                record_import_binding(model, module, binding, target);
            }
            _ => {}
        }
    }
    Ok(())
}

fn local_binding_exists(model: &DefaultSignatureTypes, module: &[String], name: &str) -> bool {
    let mut key = module.to_vec();
    key.push(name.to_string());
    model
        .local_type_bindings
        .contains(&(module.to_vec(), name.to_string()))
        || model.aliases.contains_key(&key)
        || model.constants.contains_key(&key)
        || model
            .imports
            .get(module)
            .is_some_and(|imports| imports.contains_key(name))
        || model
            .ambiguous_imports
            .contains(&(module.to_vec(), name.to_string()))
}

fn materialize_import_target(
    model: &DefaultSignatureTypes,
    target: &ImportTarget,
) -> Result<ResolvedPath, String> {
    match target.origin {
        ImportOrigin::Local => Ok(ResolvedPath {
            origin: ResolvedOrigin::Local,
            segments: target.segments.clone(),
        }),
        ImportOrigin::External => Ok(ResolvedPath {
            origin: ResolvedOrigin::External,
            segments: target.segments.clone(),
        }),
        ImportOrigin::Unqualified => {
            let Some(first) = target.segments.first() else {
                return Err(format!(
                    "{MARKER} cannot resolve an empty unqualified import target"
                ));
            };
            // Uniform paths prefer an in-scope local binding. A crate-root
            // binding is the second local candidate; only when neither exists
            // may the first segment come from the extern prelude.
            if local_binding_exists(model, &target.module, first) {
                let mut segments = target.module.clone();
                segments.extend(target.segments.iter().cloned());
                Ok(ResolvedPath {
                    origin: ResolvedOrigin::Local,
                    segments,
                })
            } else if local_binding_exists(model, &[], first) {
                Ok(ResolvedPath {
                    origin: ResolvedOrigin::Local,
                    segments: target.segments.clone(),
                })
            } else {
                Ok(ResolvedPath {
                    origin: ResolvedOrigin::External,
                    segments: target.segments.clone(),
                })
            }
        }
    }
}

fn expand_named_import_chain(
    model: &DefaultSignatureTypes,
    mut path: ResolvedPath,
    kind: &str,
    is_terminal: impl Fn(&[String]) -> bool,
) -> Result<(ResolvedPath, bool), String> {
    let mut followed = false;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(path.clone()) {
            return Err(format!(
                "{MARKER} cannot resolve cyclic {kind} import path `{}`",
                path.segments.join("::")
            ));
        }
        if path.origin == ResolvedOrigin::External {
            return Ok((path, followed));
        }
        if is_terminal(&path.segments) {
            return Ok((path, followed));
        }
        let mut replacement = None;
        // Prefer the binding in the most specific containing module. This
        // handles both `use crate::m::C as D` and module aliases such as
        // `use crate::m as alias; alias::C` without accidentally selecting a
        // same-named root re-export first.
        for index in (0..path.segments.len()).rev() {
            let module = path.segments[..index].to_vec();
            let binding = path.segments[index].clone();
            if model
                .ambiguous_imports
                .contains(&(module.clone(), binding.clone()))
            {
                return Err(format!(
                    "{MARKER} cannot prove ambiguous imported {kind} `{binding}` in module `{}`",
                    module.join("::")
                ));
            }
            let Some(target) = model
                .imports
                .get(&module)
                .and_then(|imports| imports.get(&binding))
            else {
                continue;
            };
            let mut expanded = materialize_import_target(model, target)?;
            expanded
                .segments
                .extend(path.segments[index + 1..].iter().cloned());
            replacement = Some(expanded);
            break;
        }
        let Some(expanded) = replacement else {
            return Ok((path, followed));
        };
        path = expanded;
        followed = true;
    }
}

fn resolve_glob_imports(
    model: &DefaultSignatureTypes,
    module: &[String],
    name: &str,
    kind: ImportedSymbolKind,
    seen: &mut BTreeSet<Vec<String>>,
) -> Result<GlobResolution, String> {
    if !seen.insert(module.to_vec()) {
        return Err(format!(
            "{MARKER} cannot resolve cyclic glob-imported {} `{name}` through module `{}`",
            kind.description(),
            module.join("::")
        ));
    }

    let mut resolution = GlobResolution::default();
    for target in model
        .glob_imports
        .get(module)
        .into_iter()
        .flatten()
    {
        let target = materialize_import_target(model, target)?;
        let (target, _) = expand_named_import_chain(
            model,
            target,
            kind.description(),
            |_| false,
        )?;
        if target.origin == ResolvedOrigin::External {
            resolution.reached_external_origin = true;
            continue;
        }

        let mut candidate = target.segments.clone();
        candidate.push(name.to_string());
        let (candidate, _) = expand_named_import_chain(
            model,
            ResolvedPath {
                origin: ResolvedOrigin::Local,
                segments: candidate,
            },
            kind.description(),
            |path| kind.is_terminal(model, path),
        )?;
        if candidate.origin == ResolvedOrigin::External {
            resolution.reached_external_origin = true;
            continue;
        }
        if kind.is_terminal(model, &candidate.segments) {
            resolution.local_paths.push(candidate.segments);
            continue;
        }

        let nested = resolve_glob_imports(
            model,
            &target.segments,
            name,
            kind,
            seen,
        )?;
        resolution.local_paths.extend(nested.local_paths);
        resolution.reached_external_origin |= nested.reached_external_origin;
    }
    seen.remove(module);
    resolution.local_paths.sort();
    resolution.local_paths.dedup();
    Ok(resolution)
}

fn alias_key_for_path(
    path: &syn::Path,
    module: &[String],
    model: &DefaultSignatureTypes,
) -> Result<Option<Vec<String>>, String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| ident_text(&segment.ident))
        .collect::<Vec<_>>();
    let Some(original_name) = segments.last() else {
        return Ok(None);
    };

    let mut exact_candidates = Vec::<ResolvedPath>::new();
    let mut fallback_names = BTreeSet::new();
    fallback_names.insert(original_name.clone());
    if path.leading_colon.is_some() {
        exact_candidates.push(ResolvedPath {
            origin: ResolvedOrigin::External,
            segments: segments.clone(),
        });
    } else {
        if let Some(relative) = normalize_local_segments(module, &segments) {
            exact_candidates.push(ResolvedPath {
                origin: ResolvedOrigin::Local,
                segments: relative,
            });
        }
        if !matches!(
            segments.first().map(String::as_str),
            Some("crate" | "self" | "super")
        ) {
            exact_candidates.push(ResolvedPath {
                origin: ResolvedOrigin::Local,
                segments: segments.clone(),
            });
        }
    }
    exact_candidates.sort();
    exact_candidates.dedup();
    let mut resolved = Vec::new();
    let mut reached_external_origin = false;
    for candidate in exact_candidates {
        let (expanded, _) =
            expand_named_import_chain(model, candidate, "type-alias", |candidate| {
                model.aliases.contains_key(candidate)
            })?;
        if expanded.origin == ResolvedOrigin::External {
            reached_external_origin = true;
            continue;
        }
        if let Some(name) = expanded.segments.last() {
            fallback_names.insert(name.clone());
        }
        if model.aliases.contains_key(&expanded.segments) {
            resolved.push(expanded.segments);
        }
    }
    resolved.sort();
    resolved.dedup();
    if resolved.len() == 1 {
        return Ok(resolved.into_iter().next());
    }
    if resolved.len() > 1 {
        return Err(format!(
            "{MARKER} cannot prove type alias path `{}` exactly in module `{}`",
            path.to_token_stream(),
            module.join("::")
        ));
    }
    if reached_external_origin {
        // Never let an absolute/extern-prelude type import fall back to a
        // same-tailed local alias. Exact codegen remains responsible for
        // proving genuinely external non-alias types.
        return Ok(None);
    }

    if segments.len() == 1 {
        let mut glob_resolution = resolve_glob_imports(
            model,
            module,
            original_name,
            ImportedSymbolKind::TypeAlias,
            &mut BTreeSet::new(),
        )?;
        if !module.is_empty() {
            let root_resolution = resolve_glob_imports(
                model,
                &[],
                original_name,
                ImportedSymbolKind::TypeAlias,
                &mut BTreeSet::new(),
            )?;
            glob_resolution
                .local_paths
                .extend(root_resolution.local_paths);
            glob_resolution.reached_external_origin |=
                root_resolution.reached_external_origin;
        }
        glob_resolution.local_paths.sort();
        glob_resolution.local_paths.dedup();
        if glob_resolution.local_paths.len() == 1
            && !glob_resolution.reached_external_origin
        {
            return Ok(glob_resolution.local_paths.into_iter().next());
        }
        if glob_resolution.local_paths.len() > 1 {
            return Err(format!(
                "{MARKER} cannot resolve ambiguous glob-imported type alias `{original_name}` in module `{}`; candidates: {}",
                module.join("::"),
                glob_resolution
                    .local_paths
                    .iter()
                    .map(|candidate| candidate.join("::"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if glob_resolution.reached_external_origin {
            // The exact C++ type gate may still prove a genuine external type,
            // but this source audit must never substitute a same-named local
            // alias for an external/glob-imported binding.
            return Ok(None);
        }
    }
    let mut fallback_candidates = fallback_names
        .iter()
        .filter_map(|name| model.aliases_by_name.get(name))
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    fallback_candidates.sort();
    fallback_candidates.dedup();
    if fallback_candidates.is_empty() {
        return Ok(None);
    }
    if fallback_candidates.len() == 1 {
        return Ok(fallback_candidates.into_iter().next());
    }
    Err(format!(
        "{MARKER} cannot resolve ambiguous type alias `{}` in module `{}`; candidates: {}",
        path.to_token_stream(),
        module.join("::"),
        fallback_candidates
            .iter()
            .map(|candidate| candidate.join("::"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn mark_relevant_local_type_path(
    path: &syn::Path,
    module: &[String],
    model: &DefaultSignatureTypes,
) -> Result<(), String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| ident_text(&segment.ident))
        .collect::<Vec<_>>();
    if segments.is_empty() || path.leading_colon.is_some() {
        return Ok(());
    }
    let mut candidates = Vec::<ResolvedPath>::new();
    if let Some(relative) = normalize_local_segments(module, &segments) {
        candidates.push(ResolvedPath {
            origin: ResolvedOrigin::Local,
            segments: relative,
        });
    }
    if !matches!(
        segments.first().map(String::as_str),
        Some("crate" | "self" | "super")
    ) {
        candidates.push(ResolvedPath {
            origin: ResolvedOrigin::Local,
            segments,
        });
    }
    candidates.sort();
    candidates.dedup();
    for candidate in candidates {
        let (expanded, _) =
            expand_named_import_chain(model, candidate, "signature type", |path| {
                let Some((name, parent)) = path.split_last() else {
                    return false;
                };
                model
                    .local_nonmodule_type_bindings
                    .contains(&(parent.to_vec(), name.clone()))
            })?;
        if expanded.origin == ResolvedOrigin::Local {
            let Some((name, parent)) = expanded.segments.split_last() else {
                continue;
            };
            if model
                .local_nonmodule_type_bindings
                .contains(&(parent.to_vec(), name.clone()))
            {
                model.relevant_modules.borrow_mut().insert(parent.to_vec());
            }
        }
    }
    Ok(())
}

fn supported_primitive_associated_const(primitive: &str, member: &str) -> bool {
    crate::types::map_primitive_type(primitive).is_some()
        && matches!(
            member,
            "MAX"
                | "MIN"
                | "MIN_POSITIVE"
                | "EPSILON"
                | "NAN"
                | "INFINITY"
                | "NEG_INFINITY"
                | "BITS"
        )
}

fn primitive_associated_const_path(
    path: &syn::Path,
    module: &[String],
    generic_types: &BTreeSet<String>,
    model: &DefaultSignatureTypes,
) -> Result<bool, String> {
    if path
        .segments
        .iter()
        .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Ok(false);
    }
    let segments = path
        .segments
        .iter()
        .map(|segment| ident_text(&segment.ident))
        .collect::<Vec<_>>();
    if let [root, primitive, member] = segments.as_slice()
        && root == "libc"
        && supported_primitive_associated_const(primitive, member)
    {
        return Err(format!(
            "{MARKER} never treats `libc` package paths as canonical primitive-associated constants: `{}`",
            path.to_token_stream()
        ));
    }
    let (root, primitive, member) = match segments.as_slice() {
        [primitive, member] if path.leading_colon.is_none() => {
            (None, primitive.as_str(), member.as_str())
        }
        [root, primitive, member] if matches!(root.as_str(), "std" | "core") => {
            (Some(root.as_str()), primitive.as_str(), member.as_str())
        }
        [root, primitive_module, primitive, member]
            if matches!(root.as_str(), "std" | "core") && primitive_module == "primitive" =>
        {
            (Some(root.as_str()), primitive.as_str(), member.as_str())
        }
        _ => return Ok(false),
    };
    let first = root.unwrap_or(primitive);
    let binding_may_shadow = |scope: &[String]| {
        local_binding_exists(model, scope, first)
            || model
                .glob_imports
                .get(scope)
                .is_some_and(|globs| !globs.is_empty())
    };
    let shadowed = path.leading_colon.is_none()
        && (generic_types.contains(first)
            || binding_may_shadow(module)
            || (!module.is_empty() && binding_may_shadow(&[])));
    if shadowed {
        return Err(format!(
            "{MARKER} cannot prove shadowed primitive-associated const path `{}` exactly in module `{}`",
            path.to_token_stream(),
            module.join("::")
        ));
    }
    Ok(supported_primitive_associated_const(primitive, member))
}

fn const_key_for_path(
    path: &syn::Path,
    module: &[String],
    model: &DefaultSignatureTypes,
    strict: bool,
) -> Result<Option<Vec<String>>, String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| ident_text(&segment.ident))
        .collect::<Vec<_>>();
    let Some(name) = segments.last() else {
        return Ok(None);
    };
    let mut candidates = Vec::<ResolvedPath>::new();
    if path.leading_colon.is_some() {
        candidates.push(ResolvedPath {
            origin: ResolvedOrigin::External,
            segments: segments.clone(),
        });
    } else {
        let Some(relative) = normalize_local_segments(module, &segments) else {
            return Err(format!(
                "{MARKER} cannot normalize const path `{}` from module `{}`",
                path.to_token_stream(),
                module.join("::")
            ));
        };
        candidates.push(ResolvedPath {
            origin: ResolvedOrigin::Local,
            segments: relative,
        });
        if !matches!(
            segments.first().map(String::as_str),
            Some("crate" | "self" | "super")
        ) {
            candidates.push(ResolvedPath {
                origin: ResolvedOrigin::Local,
                segments: segments.clone(),
            });
        }
    }
    candidates.sort();
    candidates.dedup();

    let mut resolved = Vec::new();
    let mut followed_import = false;
    let mut reached_external_origin = false;
    for candidate in candidates {
        let (expanded, followed) =
            expand_named_import_chain(model, candidate, "const", |candidate| {
                model.constants.contains_key(candidate)
            })?;
        followed_import |= followed;
        if expanded.origin == ResolvedOrigin::External {
            reached_external_origin = true;
        } else if model.constants.contains_key(&expanded.segments) {
            resolved.push(expanded.segments);
        }
    }
    resolved.sort();
    resolved.dedup();
    if resolved.len() == 1 && !reached_external_origin {
        return Ok(resolved.into_iter().next());
    }
    if resolved.len() > 1 {
        return Err(format!(
            "{MARKER} cannot prove named const path `{}` exactly in module `{}`; candidates: {}",
            path.to_token_stream(),
            module.join("::"),
            resolved
                .iter()
                .map(|candidate| candidate.join("::"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if reached_external_origin {
        return if strict {
            Err(format!(
                "{MARKER} cannot prove external const closure `{}` exactly",
                path.to_token_stream()
            ))
        } else {
            Ok(None)
        };
    }

    // A bare name with no explicit import can arrive through a glob import.
    // Follow only the exact imported module/re-export graph. A same-named
    // declaration elsewhere in the crate is not evidence for this binding.
    if segments.len() == 1 && !followed_import {
        let mut glob_resolution = resolve_glob_imports(
            model,
            module,
            name,
            ImportedSymbolKind::Const,
            &mut BTreeSet::new(),
        )?;
        if !module.is_empty() {
            let root_resolution = resolve_glob_imports(
                model,
                &[],
                name,
                ImportedSymbolKind::Const,
                &mut BTreeSet::new(),
            )?;
            glob_resolution
                .local_paths
                .extend(root_resolution.local_paths);
            glob_resolution.reached_external_origin |=
                root_resolution.reached_external_origin;
        }
        glob_resolution.local_paths.sort();
        glob_resolution.local_paths.dedup();
        if glob_resolution.local_paths.len() == 1
            && !glob_resolution.reached_external_origin
        {
            return Ok(glob_resolution.local_paths.into_iter().next());
        }
        if glob_resolution.local_paths.len() > 1 {
            return Err(format!(
                "{MARKER} cannot resolve ambiguous glob-imported named const `{name}` in module `{}`; candidates: {}",
                module.join("::"),
                glob_resolution
                    .local_paths
                    .iter()
                    .map(|candidate| candidate.join("::"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if glob_resolution.reached_external_origin {
            return if strict {
                Err(format!(
                    "{MARKER} cannot prove external glob-imported const closure `{name}` exactly"
                ))
            } else {
                Ok(None)
            };
        }
    }

    if strict {
        Err(format!(
            "{MARKER} cannot prove named const path `{}` exactly in module `{}`",
            path.to_token_stream(),
            module.join("::")
        ))
    } else {
        Ok(None)
    }
}

fn audit_path_arguments(
    arguments: &syn::PathArguments,
    module: &[String],
    generic_types: &BTreeSet<String>,
    generic_consts: &BTreeSet<String>,
    model: &DefaultSignatureTypes,
    alias_stack: &mut Vec<Vec<String>>,
    const_stack: &mut Vec<Vec<String>>,
    strict: bool,
) -> Result<(), String> {
    match arguments {
        syn::PathArguments::None => Ok(()),
        syn::PathArguments::AngleBracketed(arguments) => {
            for argument in &arguments.args {
                match argument {
                    syn::GenericArgument::Type(ty) => audit_signature_type(
                        ty,
                        module,
                        generic_types,
                        generic_consts,
                        model,
                        alias_stack,
                        const_stack,
                        strict,
                    )?,
                    syn::GenericArgument::AssocType(binding) => audit_signature_type(
                        &binding.ty,
                        module,
                        generic_types,
                        generic_consts,
                        model,
                        alias_stack,
                        const_stack,
                        strict,
                    )?,
                    syn::GenericArgument::Const(value) => audit_signature_const_expr(
                        value,
                        module,
                        generic_types,
                        generic_consts,
                        model,
                        alias_stack,
                        const_stack,
                        strict,
                    )?,
                    syn::GenericArgument::AssocConst(binding) => audit_signature_const_expr(
                        &binding.value,
                        module,
                        generic_types,
                        generic_consts,
                        model,
                        alias_stack,
                        const_stack,
                        strict,
                    )?,
                    _ => {}
                }
            }
            Ok(())
        }
        syn::PathArguments::Parenthesized(arguments) => {
            for input in &arguments.inputs {
                audit_signature_type(
                    input,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            if let syn::ReturnType::Type(_, output) = &arguments.output {
                audit_signature_type(
                    output,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            Ok(())
        }
    }
}

fn audit_signature_const_expr(
    expression: &syn::Expr,
    module: &[String],
    generic_types: &BTreeSet<String>,
    generic_consts: &BTreeSet<String>,
    model: &DefaultSignatureTypes,
    alias_stack: &mut Vec<Vec<String>>,
    const_stack: &mut Vec<Vec<String>>,
    strict: bool,
) -> Result<(), String> {
    struct ConstExpressionVisitor<'a> {
        module: &'a [String],
        generic_types: &'a BTreeSet<String>,
        generic_consts: &'a BTreeSet<String>,
        model: &'a DefaultSignatureTypes,
        alias_stack: &'a mut Vec<Vec<String>>,
        const_stack: &'a mut Vec<Vec<String>>,
        strict: bool,
        error: Option<String>,
    }

    impl ConstExpressionVisitor<'_> {
        fn audit_path(&mut self, path: &syn::ExprPath) -> Result<(), String> {
            if path.qself.is_some() {
                return Err(format!(
                    "{MARKER} cannot prove associated-const projection `{}` inside a C++ signature type",
                    path.to_token_stream()
                ));
            }
            let segments = path
                .path
                .segments
                .iter()
                .map(|segment| ident_text(&segment.ident))
                .collect::<Vec<_>>();
            if segments.len() == 1
                && segments
                    .first()
                    .is_some_and(|name| self.generic_consts.contains(name))
            {
                return Ok(());
            }
            if primitive_associated_const_path(
                &path.path,
                self.module,
                self.generic_types,
                self.model,
            )? {
                return Ok(());
            }
            if segments.len() > 1
                && (segments.first().is_some_and(|first| {
                    first == "Self"
                        || self.generic_types.contains(first)
                        || self
                            .model
                            .local_nonmodule_type_bindings
                            .contains(&(self.module.to_vec(), first.clone()))
                }) || segments
                    .last()
                    .is_some_and(|last| self.model.associated_const_names.contains(last)))
            {
                return Err(format!(
                    "{MARKER} cannot prove associated-const projection `{}` inside a C++ signature type",
                    path.to_token_stream()
                ));
            }
            let Some(const_key) =
                const_key_for_path(&path.path, self.module, self.model, self.strict)?
            else {
                return Ok(());
            };
            if self.const_stack.contains(&const_key) {
                let mut cycle = self
                    .const_stack
                    .iter()
                    .map(|key| key.join("::"))
                    .collect::<Vec<_>>();
                cycle.push(const_key.join("::"));
                return Err(format!(
                    "{MARKER} cannot resolve recursive named const closure {}",
                    cycle.join(" -> ")
                ));
            }
            let declarations = self
                .model
                .constants
                .get(&const_key)
                .expect("const key came from the model")
                .clone();
            for declaration in &declarations {
                self.model
                    .relevant_modules
                    .borrow_mut()
                    .insert(declaration.module.clone());
            }
            self.const_stack.push(const_key);
            let empty_types = BTreeSet::new();
            let empty_consts = BTreeSet::new();
            let result = declarations.iter().try_for_each(|declaration| {
                audit_signature_type(
                    &declaration.ty,
                    &declaration.module,
                    &empty_types,
                    &empty_consts,
                    self.model,
                    self.alias_stack,
                    self.const_stack,
                    self.strict,
                )?;
                audit_signature_const_expr(
                    &declaration.expression,
                    &declaration.module,
                    &empty_types,
                    &empty_consts,
                    self.model,
                    self.alias_stack,
                    self.const_stack,
                    self.strict,
                )
            });
            self.const_stack.pop();
            result
        }
    }

    impl<'ast> Visit<'ast> for ConstExpressionVisitor<'_> {
        fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
            if self.error.is_some() {
                return;
            }
            if let Err(error) = self.audit_path(path) {
                self.error = Some(error);
                return;
            }
            if self.error.is_none() {
                syn::visit::visit_expr_path(self, path);
            }
        }

        fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
            if self.error.is_none() {
                self.error = Some(format!(
                    "{MARKER} cannot prove macro expression `{}` inside a C++ signature const closure",
                    expression.to_token_stream()
                ));
            }
        }

        fn visit_type(&mut self, ty: &'ast Type) {
            if self.error.is_some() {
                return;
            }
            if let Err(error) = audit_signature_type(
                ty,
                self.module,
                self.generic_types,
                self.generic_consts,
                self.model,
                self.alias_stack,
                self.const_stack,
                self.strict,
            ) {
                self.error = Some(error);
            }
        }
    }

    let mut visitor = ConstExpressionVisitor {
        module,
        generic_types,
        generic_consts,
        model,
        alias_stack,
        const_stack,
        strict,
        error: None,
    };
    visitor.visit_expr(expression);
    match visitor.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn audit_type_bound(
    bound: &syn::TypeParamBound,
    module: &[String],
    generic_types: &BTreeSet<String>,
    generic_consts: &BTreeSet<String>,
    model: &DefaultSignatureTypes,
    alias_stack: &mut Vec<Vec<String>>,
    const_stack: &mut Vec<Vec<String>>,
    strict: bool,
) -> Result<(), String> {
    if let syn::TypeParamBound::Trait(bound) = bound {
        for segment in &bound.path.segments {
            audit_path_arguments(
                &segment.arguments,
                module,
                generic_types,
                generic_consts,
                model,
                alias_stack,
                const_stack,
                strict,
            )?;
        }
    }
    Ok(())
}

fn audit_signature_type(
    ty: &Type,
    module: &[String],
    generic_types: &BTreeSet<String>,
    generic_consts: &BTreeSet<String>,
    model: &DefaultSignatureTypes,
    alias_stack: &mut Vec<Vec<String>>,
    const_stack: &mut Vec<Vec<String>>,
    strict: bool,
) -> Result<(), String> {
    match ty {
        Type::Path(type_path) => {
            if type_path.qself.is_some() {
                return Err(format!(
                    "{MARKER} cannot prove associated-type projection `{}` as an exact C++ signature type",
                    type_path.to_token_stream()
                ));
            }
            for segment in &type_path.path.segments {
                audit_path_arguments(
                    &segment.arguments,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            let segments = type_path
                .path
                .segments
                .iter()
                .map(|segment| ident_text(&segment.ident))
                .collect::<Vec<_>>();
            if segments.len() > 1
                && (segments
                    .first()
                    .is_some_and(|first| first == "Self" || generic_types.contains(first))
                    || segments
                        .last()
                        .is_some_and(|last| model.associated_type_names.contains(last)))
            {
                return Err(format!(
                    "{MARKER} cannot prove associated-type projection `{}` as an exact C++ signature type",
                    type_path.to_token_stream()
                ));
            }
            mark_relevant_local_type_path(&type_path.path, module, model)?;
            let Some(alias_key) = alias_key_for_path(&type_path.path, module, model)? else {
                return Ok(());
            };
            if alias_stack.contains(&alias_key) {
                let mut cycle = alias_stack
                    .iter()
                    .map(|key| key.join("::"))
                    .collect::<Vec<_>>();
                cycle.push(alias_key.join("::"));
                return Err(format!(
                    "{MARKER} cannot resolve recursive type alias closure {}",
                    cycle.join(" -> ")
                ));
            }
            let alias = model
                .aliases
                .get(&alias_key)
                .expect("alias key came from the model");
            model
                .relevant_modules
                .borrow_mut()
                .insert(alias.module.clone());
            let mut alias_generic_types = generic_types.clone();
            alias_generic_types.extend(alias.type_parameters.iter().cloned());
            let mut alias_generic_consts = generic_consts.clone();
            alias_generic_consts.extend(alias.const_parameters.iter().cloned());
            alias_stack.push(alias_key);
            let result = audit_signature_type(
                &alias.ty,
                &alias.module,
                &alias_generic_types,
                &alias_generic_consts,
                model,
                alias_stack,
                const_stack,
                strict,
            );
            alias_stack.pop();
            result
        }
        Type::Array(array) => {
            audit_signature_type(
                &array.elem,
                module,
                generic_types,
                generic_consts,
                model,
                alias_stack,
                const_stack,
                strict,
            )?;
            audit_signature_const_expr(
                &array.len,
                module,
                generic_types,
                generic_consts,
                model,
                alias_stack,
                const_stack,
                strict,
            )
        }
        Type::BareFn(function) => {
            for input in &function.inputs {
                audit_signature_type(
                    &input.ty,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            if let syn::ReturnType::Type(_, output) = &function.output {
                audit_signature_type(
                    output,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            Ok(())
        }
        Type::Group(group) => audit_signature_type(
            &group.elem,
            module,
            generic_types,
            generic_consts,
            model,
            alias_stack,
            const_stack,
            strict,
        ),
        Type::Paren(paren) => audit_signature_type(
            &paren.elem,
            module,
            generic_types,
            generic_consts,
            model,
            alias_stack,
            const_stack,
            strict,
        ),
        Type::Ptr(pointer) => audit_signature_type(
            &pointer.elem,
            module,
            generic_types,
            generic_consts,
            model,
            alias_stack,
            const_stack,
            strict,
        ),
        Type::Reference(reference) => audit_signature_type(
            &reference.elem,
            module,
            generic_types,
            generic_consts,
            model,
            alias_stack,
            const_stack,
            strict,
        ),
        Type::Slice(slice) => audit_signature_type(
            &slice.elem,
            module,
            generic_types,
            generic_consts,
            model,
            alias_stack,
            const_stack,
            strict,
        ),
        Type::Tuple(tuple) => {
            for element in &tuple.elems {
                audit_signature_type(
                    element,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            Ok(())
        }
        Type::ImplTrait(implementation) => {
            for bound in &implementation.bounds {
                audit_type_bound(
                    bound,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            Err(format!(
                "{MARKER} cannot prove `impl Trait` as an exact C++ signature type"
            ))
        }
        Type::TraitObject(object) => {
            for bound in &object.bounds {
                audit_type_bound(
                    bound,
                    module,
                    generic_types,
                    generic_consts,
                    model,
                    alias_stack,
                    const_stack,
                    strict,
                )?;
            }
            Ok(())
        }
        Type::Infer(_) | Type::Macro(_) | Type::Verbatim(_) => Err(format!(
            "{MARKER} cannot prove signature type `{}` exactly",
            ty.to_token_stream()
        )),
        _ => Ok(()),
    }
}

fn validate_marked_functions_signature_types(
    items: &[Item],
    module: &[String],
    model: &DefaultSignatureTypes,
    strict: bool,
) -> Result<(), String> {
    for item in items {
        let Item::Fn(function) = item else {
            continue;
        };
        if !function_has_defaults(function)? {
            continue;
        }
        model.relevant_modules.borrow_mut().insert(module.to_vec());
        let generic_types = function
            .sig
            .generics
            .type_params()
            .map(|parameter| ident_text(&parameter.ident))
            .collect::<BTreeSet<_>>();
        let generic_consts = function
            .sig
            .generics
            .const_params()
            .map(|parameter| ident_text(&parameter.ident))
            .collect::<BTreeSet<_>>();
        for input in &function.sig.inputs {
            if let FnArg::Typed(argument) = input {
                audit_signature_type(
                    &argument.ty,
                    module,
                    &generic_types,
                    &generic_consts,
                    model,
                    &mut Vec::new(),
                    &mut Vec::new(),
                    strict,
                )
                .map_err(|error| {
                    format!(
                        "{MARKER} function `{}` has an inexact parameter type: {error}",
                        function.sig.ident
                    )
                })?;
            }
        }
        if let syn::ReturnType::Type(_, output) = &function.sig.output {
            audit_signature_type(
                output,
                module,
                &generic_types,
                &generic_consts,
                model,
                &mut Vec::new(),
                &mut Vec::new(),
                strict,
            )
            .map_err(|error| {
                format!(
                    "{MARKER} function `{}` has an inexact return type: {error}",
                    function.sig.ident
                )
            })?;
        }
    }
    Ok(())
}

fn validate_default_signature_types(
    parsed: &[(Vec<String>, &syn::File)],
    strict: bool,
) -> Result<(), String> {
    let mut model = DefaultSignatureTypes::default();
    for (module, file) in parsed {
        collect_signature_type_model(&file.items, module, &mut model)?;
    }
    for aliases in model.aliases_by_name.values_mut() {
        aliases.sort();
        aliases.dedup();
    }
    for constants in model.constants_by_name.values_mut() {
        constants.sort();
        constants.dedup();
    }
    // Populate the module relevance set before applying the macro gate. This
    // dry audit does not authorize any signature: the strict audit below is
    // still authoritative. It only prevents the one explicitly trusted
    // source marker (`rusty::cpp_inherit`) from becoming a macro escape hatch
    // in a module whose declarations feed a default-bearing signature.
    for (module, file) in parsed {
        let _ = validate_marked_functions_signature_types(&file.items, module, &model, false);
    }
    for (module, file) in parsed {
        for attr in &file.attrs {
            if !item_attribute_cannot_generate_bindings(&attr.meta, module, &model, false)? {
                return Err(format!(
                    "{MARKER} cannot prove that file attribute `{}` is free of macro-generated bindings in module `{}`",
                    attr.meta.to_token_stream(),
                    module.join("::")
                ));
            }
        }
        validate_binding_macro_surfaces(&file.items, module, &model)?;
    }
    for (module, file) in parsed {
        validate_marked_functions_signature_types(&file.items, module, &model, strict)?;
    }
    Ok(())
}

fn validate_crate_default_signature_types(inputs: &[(PathBuf, String)]) -> Result<(), String> {
    let mut files = Vec::with_capacity(inputs.len());
    for (path, source) in inputs {
        let module = conventional_module_path(path)?;
        let file = syn::parse_file(source).map_err(|error| {
            format!(
                "{MARKER} crate signature audit could not parse {}: {error}",
                path.display()
            )
        })?;
        files.push((module, file));
    }
    let borrowed = files
        .iter()
        .map(|(module, file)| (module.clone(), file))
        .collect::<Vec<_>>();
    validate_default_signature_types(&borrowed, true)
}

pub(crate) fn validate_file(file: &syn::File, type_map: &UserTypeMap) -> Result<bool, String> {
    validate_file_impl(file, Some(type_map), true)
}

pub(crate) fn validate_file_after_crate_preflight(
    file: &syn::File,
    type_map: &UserTypeMap,
) -> Result<bool, String> {
    // Crate mode has already audited the union of every source file with
    // strict resolution. Per-file exact codegen must not reinterpret a
    // sibling declaration as an unresolved external path.
    validate_file_impl(file, Some(type_map), false)
}

pub(crate) fn validate_required_gmf_includes(
    file: &syn::File,
    includes: &[crate::transpile::GmfIncludeSpec],
) -> Result<(), String> {
    let mut needs_source_location = false;
    let mut needs_stderr = false;
    for item in &file.items {
        let Item::Fn(function) = item else {
            continue;
        };
        for input in &function.sig.inputs {
            let FnArg::Typed(argument) = input else {
                continue;
            };
            match parameter_kind(argument)? {
                Some(CppDefaultArgument::SourceLocation) => needs_source_location = true,
                Some(CppDefaultArgument::Stderr) => needs_stderr = true,
                None => {}
            }
        }
    }

    let has_angle_include = |path: &str| {
        includes.iter().any(|include| {
            include.form == crate::transpile::GmfIncludeForm::Angle && include.path == path
        })
    };
    if needs_source_location && !has_angle_include("source_location") {
        return Err(format!(
            "{MARKER}(source_location) requires structured angle include <source_location> in the owning module preamble"
        ));
    }
    if needs_stderr && !(has_angle_include("stdio.h") || has_angle_include("cstdio")) {
        return Err(format!(
            "{MARKER}(stderr) requires structured angle include <stdio.h> or <cstdio> in the owning module preamble"
        ));
    }
    Ok(())
}

pub(crate) fn preflight_crate_sources_syntax(inputs: &[(PathBuf, String)]) -> Result<bool, String> {
    let mut found = false;
    let mut owners = Vec::new();
    for (path, source) in inputs {
        if !source_mentions_marker(source) {
            continue;
        }
        let file = syn::parse_file(source).map_err(|error| {
            format!(
                "{}: could not parse marker-bearing Rust: {error}",
                path.display()
            )
        })?;
        let owns = validate_file_impl(&file, None, false)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        found |= owns;
        if owns {
            owners.push(path.clone());
        }
    }
    if found {
        crate::cpp_abi::validate_source_contract_module_graph(inputs, MARKER, &owners)?;
        validate_crate_default_signature_types(inputs)?;
    }
    Ok(found)
}

pub(crate) fn preflight_crate_sources(
    inputs: &[(PathBuf, String)],
    type_map: &UserTypeMap,
) -> Result<bool, String> {
    let mut found = false;
    let mut owners = Vec::new();
    for (path, source) in inputs {
        if !source_mentions_marker(source) {
            continue;
        }
        let file = syn::parse_file(source).map_err(|error| {
            format!(
                "{}: could not parse marker-bearing Rust: {error}",
                path.display()
            )
        })?;
        let owns = validate_file_impl(&file, Some(type_map), false)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        found |= owns;
        if owns {
            owners.push(path.clone());
        }
    }
    if found {
        crate::cpp_abi::validate_source_contract_module_graph(inputs, MARKER, &owners)?;
        validate_crate_default_signature_types(inputs)?;
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_type_map() -> UserTypeMap {
        let mut type_map = UserTypeMap::default();
        type_map.mappings.insert(
            "rusty::SourceLocation".to_string(),
            "std::source_location".to_string(),
        );
        type_map
            .mappings
            .insert("rusty::CFile".to_string(), "FILE".to_string());
        type_map
    }

    #[test]
    fn accepts_only_exact_typed_trailing_defaults() {
        let file = syn::parse_file(
            r#"
                pub fn verify<T>(
                    value: &T,
                    #[cfg_attr(any(), cpp_default_argument(source_location))]
                    location: &::rusty::SourceLocation,
                ) where T: Copy {}

                /// Prints a stack trace.
                #[allow(unsafe_code)]
                pub unsafe fn print_stack_trace(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) {}
            "#,
        )
        .unwrap();
        assert!(validate_file(&file, &exact_type_map()).unwrap());
    }

    #[test]
    fn rejects_nontrailing_wrong_types_and_nonfree_placements() {
        let cases = [
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile, x: i32) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *const ::rusty::CFile) {}"#,
            r#"struct S; impl S { pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {} }"#,
            r#"mod nested { pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {} }"#,
            r#"pub fn f(#[cpp_default_argument(stderr)] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument("stderr"))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(other))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), other::cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), r#cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] #[allow(unused)] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] #[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr),)] p: *mut ::rusty::CFile) {}"#,
            r#"#[inline] pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"#[cfg(any())] pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] mut p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] _: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cfg_attr(any(), cpp_default_argument(stderr)))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut rusty::CFile) {}"#,
            r#"pub fn f<'a>(#[cfg_attr(any(), cpp_default_argument(source_location))] p: &'a ::rusty::SourceLocation) {}"#,
            r#"pub fn f(extra: *mut ::rusty::CFile, #[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) -> Option<&'static ::rusty::SourceLocation> { loop {} }"#,
            r#"pub(crate) fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub const fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub async fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"pub extern "C" fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"trait T { fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile); }"#,
            r#"unsafe extern "C" { fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile); }"#,
            r#"pub fn outer() { fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {} }"#,
            r#"macro_rules! m { () => { cpp_default_argument } }"#,
            r#"extern crate self as rusty; pub struct CFile; pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
            r#"#![cfg(any())] pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
        ];
        for source in cases {
            let file = syn::parse_file(source)
                .unwrap_or_else(|error| panic!("invalid negative fixture `{source}`: {error}"));
            assert!(
                validate_file(&file, &exact_type_map()).is_err(),
                "accepted {source}"
            );
        }
    }

    #[test]
    fn requires_exact_type_map_contract() {
        let file = syn::parse_file(
            r#"pub fn f(#[cfg_attr(any(), cpp_default_argument(stderr))] p: *mut ::rusty::CFile) {}"#,
        )
        .unwrap();
        assert!(validate_file(&file, &UserTypeMap::default()).is_err());
        let mut wrong = exact_type_map();
        wrong
            .mappings
            .insert("rusty::CFile".to_string(), "void".to_string());
        assert!(validate_file(&file, &wrong).is_err());
    }

    #[test]
    fn requires_exact_structured_preamble_dependencies() {
        let file = syn::parse_file(
            r#"
                pub fn verify(
                    #[cfg_attr(any(), cpp_default_argument(source_location))]
                    location: &::rusty::SourceLocation,
                ) {}
                pub fn print(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) {}
            "#,
        )
        .unwrap();
        let angle = |path: &str| crate::transpile::GmfIncludeSpec {
            path: path.to_string(),
            form: crate::transpile::GmfIncludeForm::Angle,
        };
        assert!(validate_required_gmf_includes(&file, &[]).is_err());
        assert!(validate_required_gmf_includes(&file, &[angle("source_location")]).is_err());
        validate_required_gmf_includes(&file, &[angle("stdio.h"), angle("source_location")])
            .unwrap();
        validate_required_gmf_includes(&file, &[angle("cstdio"), angle("source_location")])
            .unwrap();
    }

    #[test]
    fn recursively_audits_default_signature_type_aliases() {
        let safe = syn::parse_file(
            r#"
                pub type Word = u32;
                pub type Maybe<T> = Option<T>;
                pub fn safe(
                    value: Maybe<Word>,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
        )
        .unwrap();
        assert!(validate_file(&safe, &exact_type_map()).unwrap());

        let projected_positions = [
            r#"
                pub trait ValueType { type Output; }
                impl ValueType for u32 { type Output = i32; }
                pub fn direct(
                    value: <u32 as ValueType>::Output,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub trait ValueType { type Output; }
                impl ValueType for u32 { type Output = i32; }
                pub type Projected = <u32 as ValueType>::Output;
                pub type Chain = Projected;
                pub fn aliased(
                    value: Chain,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub trait ValueType { type Output; }
                impl ValueType for u32 { type Output = i32; }
                pub type Projected = <u32 as ValueType>::Output;
                pub fn nested(
                    value: Option<(u8, Projected)>,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub trait ValueType { type Output; }
                impl ValueType for u32 { type Output = i32; }
                pub type Projected = <u32 as ValueType>::Output;
                pub fn returned(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) -> Projected { let _ = stream; 0 }
            "#,
            r#"
                pub trait Width { const VALUE: usize; }
                impl Width for u32 { const VALUE: usize = 4; }
                pub type Projected = [u8; <u32 as Width>::VALUE];
                pub fn const_projected(
                    value: Projected,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
        ];
        for source in projected_positions {
            let file = syn::parse_file(source).unwrap();
            assert!(
                validate_file(&file, &exact_type_map()).is_err(),
                "accepted projected signature position: {source}"
            );
        }
    }

    #[test]
    fn recursively_audits_named_const_signature_closures() {
        let safe = syn::parse_file(
            r#"
                pub const BASE: usize = 2;
                pub const OFFSET: usize = 1;
                pub const WIDTH: usize = (BASE + OFFSET) * 2;
                pub const PRIMITIVE: usize = u8::MAX as usize;
                pub type Payload<const N: usize> = [u8; N];
                pub fn safe(
                    arithmetic: Payload<{ WIDTH }>,
                    primitive: [u8; PRIMITIVE],
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (arithmetic, primitive, stream); }
            "#,
        )
        .unwrap();
        assert!(validate_file(&safe, &exact_type_map()).unwrap());

        let safe_cfg_variants = syn::parse_file(
            r#"
                #[cfg(unix)]
                pub const WIDTH: usize = 4;
                #[cfg(windows)]
                pub const WIDTH: usize = 8;
                pub type Payload = [u8; WIDTH];
                pub fn safe(
                    value: Payload,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
        )
        .unwrap();
        assert!(validate_file(&safe_cfg_variants, &exact_type_map()).unwrap());

        let rejected = [
            r#"
                pub trait Width { const VALUE: usize; }
                impl Width for u32 { const VALUE: usize = 4; }
                pub const PROJECTED: usize = <u32 as Width>::VALUE;
                pub fn direct(
                    value: [u8; PROJECTED],
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub trait Width { const VALUE: usize; }
                impl Width for u32 { const VALUE: usize = 4; }
                pub const PROJECTED: usize = <u32 as Width>::VALUE;
                pub type Payload = [u8; PROJECTED];
                pub fn nested(
                    value: Option<(u8, Payload)>,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub trait Width { const VALUE: usize; }
                impl Width for u32 { const VALUE: usize = 4; }
                pub const PROJECTED: usize = <u32 as Width>::VALUE;
                pub type Payload = [u8; PROJECTED];
                pub fn returned(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) -> Payload { let _ = stream; loop {} }
            "#,
            r#"
                macro_rules! width { () => { 4 } }
                pub const EXPANDED: usize = width!();
                pub type Payload = [u8; EXPANDED];
                pub fn macro_hidden(
                    value: Payload,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                #[allow(non_camel_case_types)]
                pub struct u8;
                pub trait Width { const VALUE: usize; }
                impl Width for u32 { const VALUE: usize = 4; }
                impl u8 { pub const MAX: usize = <u32 as Width>::VALUE; }
                pub const SHADOWED: usize = u8::MAX;
                pub type Payload = [u8; SHADOWED];
                pub fn primitive_lookalike(
                    value: Payload,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub mod char { pub const MAX: usize = 4; }
                pub const SHADOWED: usize = char::MAX;
                pub type Payload = [u8; SHADOWED];
                pub fn primitive_module_lookalike(
                    value: Payload,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
            r#"
                pub const LEFT: usize = RIGHT;
                pub const RIGHT: usize = LEFT;
                pub type Payload = [u8; LEFT];
                pub fn cyclic(
                    value: Payload,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
        ];
        for source in rejected {
            let file = syn::parse_file(source).unwrap();
            assert!(
                validate_file(&file, &exact_type_map()).is_err(),
                "accepted unprovable named const closure: {source}"
            );
        }

        let unresolved_external = syn::parse_file(
            r#"
                use dependency::WIDTH;
                pub type Payload = [u8; WIDTH];
                pub fn unresolved(
                    value: Payload,
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = (value, stream); }
            "#,
        )
        .unwrap();
        assert!(validate_file(&unresolved_external, &exact_type_map()).is_err());
    }

    #[test]
    fn rejects_unexpanded_item_and_attribute_macro_binding_surfaces() {
        let rejected = [
            r#"
                #![binding_macros::bind_std]
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                macro_rules! bind_std { () => { use fake_std as std; }; }
                bind_std!();
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                mod nested {
                    macro_rules! make_const { () => { const VALUE: usize = 4; }; }
                    make_const!();
                }
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                trait Trait {
                    associated_items!();
                }
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                struct Owner;
                impl Owner {
                    associated_items!();
                }
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                include!("bindings.rs");
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                #[binding_macros::bind_std]
                struct Anchor;
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                #[clippy::bind_std]
                struct Anchor;
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                use binding_macros::test;
                #[test]
                struct Anchor;
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                #[derive(binding_macros::BindStd)]
                struct Anchor;
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                #[cfg_attr(not(any()), binding_macros::bind_std)]
                struct Anchor;
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                use binding_macros::bind_std as path;
                #[path = "generated.rs"]
                mod remapped;
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
            r#"
                use binding_macros::bind_std as cpp_inherit;
                trait Trait {}
                struct Owner;
                #[cpp_inherit]
                impl Trait for Owner {}
                pub fn rejected(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { let _ = stream; }
            "#,
        ];
        for source in rejected {
            let file = syn::parse_file(source).unwrap();
            assert!(
                validate_file(&file, &exact_type_map()).is_err(),
                "accepted unexpanded binding macro surface: {source}"
            );
        }

        let inert = syn::parse_file(
            r#"
                #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
                struct BuiltinDerives;
                #[cfg_attr(any(), binding_macros::disabled)]
                struct DisabledAttribute;
                #[path = "generated.rs"]
                mod remapped;
                pub fn accepted(
                    #[cfg_attr(any(), cpp_default_argument(stderr))]
                    stream: *mut ::rusty::CFile,
                ) { assert!(!stream.is_null()); }
            "#,
        )
        .unwrap();
        // Unshadowed `#[path]` is inert here: it selects a file, generates no
        // bindings, and the file it selects is already an audited unit.
        assert!(validate_file(&inert, &exact_type_map()).unwrap());
    }

    #[test]
    fn audits_imported_cross_file_alias_closures() {
        let safe_inputs = [
            (PathBuf::from("src/lib.rs"), "pub mod types; pub mod api;"),
            (
                PathBuf::from("src/types.rs"),
                "pub type Word = u32; pub type Chain = Word;",
            ),
            (
                PathBuf::from("src/api.rs"),
                r#"
                    use crate::types::Chain as Imported;
                    pub fn safe(
                        value: Option<Imported>,
                        #[cfg_attr(any(), cpp_default_argument(stderr))]
                        stream: *mut ::rusty::CFile,
                    ) { let _ = (value, stream); }
                "#,
            ),
        ]
        .map(|(path, source)| (path, source.to_string()));
        validate_crate_default_signature_types(&safe_inputs).unwrap();

        let projected_inputs = [
            (PathBuf::from("src/lib.rs"), "pub mod types; pub mod api;"),
            (
                PathBuf::from("src/types.rs"),
                r#"
                    pub trait ValueType { type Output; }
                    impl ValueType for u32 { type Output = i32; }
                    pub type Projected = <u32 as ValueType>::Output;
                    pub type Chain = Projected;
                "#,
            ),
            (
                PathBuf::from("src/api.rs"),
                r#"
                    use crate::types::Chain as Imported;
                    pub fn rejected(
                        value: Option<Imported>,
                        #[cfg_attr(any(), cpp_default_argument(stderr))]
                        stream: *mut ::rusty::CFile,
                    ) { let _ = (value, stream); }
                "#,
            ),
        ]
        .map(|(path, source)| (path, source.to_string()));
        assert!(validate_crate_default_signature_types(&projected_inputs).is_err());

        let safe_const_inputs = [
            (
                PathBuf::from("src/lib.rs"),
                "pub mod constants; pub mod bridge; pub mod aliases; pub mod api;",
            ),
            (
                PathBuf::from("src/constants.rs"),
                r#"
                    pub const BASE: usize = 2;
                    pub const OFFSET: usize = 1;
                    pub const WIDTH: usize = (BASE + OFFSET) * 2;
                    pub const PRIMITIVE: usize = u8::MAX as usize;
                "#,
            ),
            (
                PathBuf::from("src/bridge.rs"),
                r#"
                    pub use crate::constants::WIDTH as MID;
                    pub use crate::constants::PRIMITIVE as PRIMITIVE_MID;
                "#,
            ),
            (
                PathBuf::from("src/aliases.rs"),
                r#"
                    use crate::bridge::MID as IMPORTED;
                    pub type Payload = [u8; IMPORTED];
                "#,
            ),
            (
                PathBuf::from("src/api.rs"),
                r#"
                    use crate::aliases::Payload as Inner;
                    pub use Inner as ImportedPayload;
                    pub fn safe(
                        value: Option<(ImportedPayload, [u8; crate::bridge::PRIMITIVE_MID])>,
                        #[cfg_attr(any(), cpp_default_argument(stderr))]
                        stream: *mut ::rusty::CFile,
                    ) { let _ = (value, stream); }
                "#,
            ),
        ]
        .map(|(path, source)| (path, source.to_string()));
        validate_crate_default_signature_types(&safe_const_inputs).unwrap();

        let safe_glob_const_inputs = [
            (
                PathBuf::from("src/lib.rs"),
                "pub mod constants; pub mod prelude; pub mod api;",
            ),
            (
                PathBuf::from("src/constants.rs"),
                "pub const BASE: usize = 2; pub const WIDTH: usize = BASE * 2;",
            ),
            (
                PathBuf::from("src/prelude.rs"),
                "pub use crate::constants::*;",
            ),
            (
                PathBuf::from("src/api.rs"),
                r#"
                    use crate::prelude::*;
                    pub type Payload = [u8; WIDTH];
                    pub fn safe(
                        value: Payload,
                        #[cfg_attr(any(), cpp_default_argument(stderr))]
                        stream: *mut ::rusty::CFile,
                    ) { let _ = (value, stream); }
                "#,
            ),
        ]
        .map(|(path, source)| (path, source.to_string()));
        validate_crate_default_signature_types(&safe_glob_const_inputs).unwrap();

        let projected_const_inputs = [
            (
                PathBuf::from("src/lib.rs"),
                "pub mod constants; pub mod bridge; pub mod aliases; pub mod api;",
            ),
            (
                PathBuf::from("src/constants.rs"),
                r#"
                    pub trait Width { const VALUE: usize; }
                    impl Width for u32 { const VALUE: usize = 4; }
                    pub const PROJECTED: usize = <u32 as Width>::VALUE;
                "#,
            ),
            (
                PathBuf::from("src/bridge.rs"),
                "pub use crate::constants::PROJECTED as MID;",
            ),
            (
                PathBuf::from("src/aliases.rs"),
                r#"
                    use crate::bridge::MID as IMPORTED;
                    pub type Payload = [u8; IMPORTED];
                    pub use Payload as Reexported;
                "#,
            ),
            (
                PathBuf::from("src/api.rs"),
                r#"
                    use crate::aliases::Reexported as Imported;
                    pub fn rejected(
                        value: Option<Imported>,
                        #[cfg_attr(any(), cpp_default_argument(stderr))]
                        stream: *mut ::rusty::CFile,
                    ) { let _ = (value, stream); }
                "#,
            ),
        ]
        .map(|(path, source)| (path, source.to_string()));
        assert!(validate_crate_default_signature_types(&projected_const_inputs).is_err());
    }
}
