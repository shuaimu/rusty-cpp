//! Validation for the narrow C++ default-member-initializer marker.
//!
//! Rust has no field-default syntax equivalent to C++'s `member{}`.  A
//! downstream ABI may nevertheless require that exact C++ spelling: it keeps
//! an ordinary aggregate while making plain default construction initialize a
//! scalar field.  The inert source spelling
//!
//! ```ignore
//! #[cfg_attr(any(), cpp_value_init)]
//! field: u64,
//! ```
//!
//! is deliberately compiler-owned and fail-closed.  Only named fields of
//! ordinary structs, and only built-in bool/integer scalar types, may carry it.

use quote::ToTokens;
use syn::visit::Visit;

const MARKER: &str = "cpp_value_init";

fn ident_is_exact(ident: &proc_macro2::Ident, expected: &str) -> bool {
    // Raw identifiers are intentionally not an alternate spelling of a
    // compiler-owned marker or its `any` predicate.
    let spelling = ident.to_string();
    !spelling.starts_with("r#") && ident == expected
}

fn ident_mentions(ident: &proc_macro2::Ident, expected: &str) -> bool {
    let text = ident.to_string();
    text.strip_prefix("r#").unwrap_or(&text) == expected
}

fn path_is_exact_ident(path: &syn::Path, expected: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.first().is_some_and(|segment| {
            ident_is_exact(&segment.ident, expected)
                && matches!(segment.arguments, syn::PathArguments::None)
        })
}

fn token_stream_marker_count(tokens: proc_macro2::TokenStream) -> usize {
    tokens
        .into_iter()
        .map(|token| match token {
            proc_macro2::TokenTree::Ident(ident) => usize::from(ident_mentions(&ident, MARKER)),
            proc_macro2::TokenTree::Group(group) => token_stream_marker_count(group.stream()),
            _ => 0,
        })
        .sum()
}

fn attribute_marker_count(attr: &syn::Attribute) -> usize {
    token_stream_marker_count(attr.meta.to_token_stream())
}

fn attribute_is_exact_marker(attr: &syn::Attribute) -> bool {
    if !path_is_exact_ident(attr.path(), "cfg_attr") {
        return false;
    }
    let Ok(args) = attr.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ) else {
        return false;
    };
    if args.len() != 2 || args.trailing_punct() {
        return false;
    }
    let Some(syn::Meta::List(predicate)) = args.first() else {
        return false;
    };
    let Some(syn::Meta::Path(marker)) = args.iter().nth(1) else {
        return false;
    };
    path_is_exact_ident(&predicate.path, "any")
        && predicate.tokens.is_empty()
        && path_is_exact_ident(marker, MARKER)
}

fn type_is_supported_scalar(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return false;
    }
    path.path.segments.first().is_some_and(|segment| {
        matches!(segment.arguments, syn::PathArguments::None)
            && matches!(
                segment.ident.to_string().as_str(),
                "bool"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
            )
    })
}

/// Whether a field carries the one accepted marker spelling.
///
/// Codegen calls this only after [`validate_file`] has accepted the complete
/// syntax tree, so `any` is sufficient here: validation already proves there
/// is exactly one marker and that the field/type placement is supported.
pub(crate) fn field_has_marker(field: &syn::Field) -> bool {
    field.attrs.iter().any(attribute_is_exact_marker)
}

/// Validate every occurrence of the reserved marker before any C++ is emitted.
pub(crate) fn validate_source(source: &str, file: &syn::File) -> Result<(), String> {
    // Keep marker-free transpilation on its old hot path.  The substring gate
    // cannot admit anything (the AST audit below remains authoritative); it
    // only avoids re-tokenizing and walking large ordinary source files.
    if !source.contains(MARKER) {
        return Ok(());
    }
    validate_file(file)
}

/// Validate a pre-parsed source file when the original text is unavailable.
pub(crate) fn validate_file(file: &syn::File) -> Result<(), String> {
    struct Validator {
        marker_tokens_in_attributes: usize,
        error: Option<String>,
    }

    impl Validator {
        fn reject_attribute(&mut self, attr: &syn::Attribute, placement: &str) {
            let count = attribute_marker_count(attr);
            self.marker_tokens_in_attributes += count;
            if count != 0 && self.error.is_none() {
                self.error = Some(format!(
                    "{MARKER} is supported only as exactly one \
                     #[cfg_attr(any(), {MARKER})] on a named field of an ordinary struct; \
                     found it {placement}"
                ));
            }
        }

        fn validate_named_struct_field(&mut self, field: &syn::Field) {
            let marker_count: usize = field.attrs.iter().map(attribute_marker_count).sum();
            self.marker_tokens_in_attributes += marker_count;
            if marker_count == 0 || self.error.is_some() {
                syn::visit::visit_type(self, &field.ty);
                return;
            }

            let exact_count = field
                .attrs
                .iter()
                .filter(|attr| attribute_is_exact_marker(attr))
                .count();
            if marker_count != 1 || exact_count != 1 {
                self.error = Some(format!(
                    "{MARKER} must use exactly one exact inert attribute \
                     #[cfg_attr(any(), {MARKER})] on a named struct field"
                ));
                return;
            }
            if !type_is_supported_scalar(&field.ty) {
                self.error = Some(format!(
                    "{MARKER} supports only fields whose Rust type is exactly bool or a \
                     built-in signed/unsigned integer primitive"
                ));
                return;
            }

            // Attributes cannot contain nested Rust types, but the field type
            // can contain attributes/macros of its own.  Continue walking it
            // so a second reserved marker cannot hide there.
            syn::visit::visit_type(self, &field.ty);
        }
    }

    impl<'ast> Visit<'ast> for Validator {
        fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
            self.reject_attribute(attr, "outside an eligible named struct field");
        }

        fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
            for attr in &item.attrs {
                self.reject_attribute(attr, "on a struct rather than one of its named fields");
            }
            syn::visit::visit_generics(self, &item.generics);
            match &item.fields {
                syn::Fields::Named(fields) => {
                    for field in &fields.named {
                        self.validate_named_struct_field(field);
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    for field in &fields.unnamed {
                        syn::visit::visit_field(self, field);
                    }
                }
                syn::Fields::Unit => {}
            }
        }
    }

    let marker_tokens = token_stream_marker_count(file.to_token_stream());
    if marker_tokens == 0 {
        return Ok(());
    }

    let mut validator = Validator {
        marker_tokens_in_attributes: 0,
        error: None,
    };
    validator.visit_file(file);
    if let Some(error) = validator.error {
        return Err(error);
    }
    if validator.marker_tokens_in_attributes != marker_tokens {
        return Err(format!(
            "reserved {MARKER} identifier is supported only in exactly one \
             #[cfg_attr(any(), {MARKER})] attribute on an eligible named struct field"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str) -> Result<(), String> {
        validate_file(&syn::parse_file(source).expect("fixture should parse"))
    }

    #[test]
    fn accepts_bool_and_integer_primitive_named_fields() {
        validate(
            r#"
            struct Scalars {
                #[cfg_attr(any(), cpp_value_init)] a: bool,
                #[cfg_attr(any(), cpp_value_init)] b: i8,
                #[cfg_attr(any(), cpp_value_init)] c: i16,
                #[cfg_attr(any(), cpp_value_init)] d: i32,
                #[cfg_attr(any(), cpp_value_init)] e: i64,
                #[cfg_attr(any(), cpp_value_init)] f: i128,
                #[cfg_attr(any(), cpp_value_init)] g: isize,
                #[cfg_attr(any(), cpp_value_init)] h: u8,
                #[cfg_attr(any(), cpp_value_init)] i: u16,
                #[cfg_attr(any(), cpp_value_init)] j: u32,
                #[cfg_attr(any(), cpp_value_init)] k: u64,
                #[cfg_attr(any(), cpp_value_init)] l: u128,
                #[cfg_attr(any(), cpp_value_init)] m: usize,
            }
            "#,
        )
        .expect("supported scalar fields should validate");
    }

    #[test]
    fn rejects_non_exact_active_and_duplicate_spellings() {
        for source in [
            "struct S { #[cpp_value_init] value: u64 }",
            "struct S { #[cfg_attr(all(), cpp_value_init)] value: u64 }",
            "struct S { #[cfg_attr(not(any()), cpp_value_init)] value: u64 }",
            "struct S { #[cfg_attr(any(), cpp_value_init())] value: u64 }",
            "struct S { #[cfg_attr(any(), crate::cpp_value_init)] value: u64 }",
            "struct S { #[cfg_attr(any(), r#cpp_value_init)] value: u64 }",
            "struct S { #[cfg_attr(any(), cpp_value_init, allow(dead_code))] value: u64 }",
            "struct S { #[cfg_attr(any(), cfg_attr(any(), cpp_value_init))] value: u64 }",
            "struct S { #[cfg_attr(any(), cpp_value_init,)] value: u64 }",
            "struct S { #[cfg_attr(any(), cpp_value_init)] #[cfg_attr(any(), cpp_value_init)] value: u64 }",
        ] {
            let error = match validate(source) {
                Ok(()) => panic!("accepted non-exact marker: {source}"),
                Err(error) => error,
            };
            assert!(
                error.contains(MARKER),
                "unexpected error for {source}: {error}"
            );
        }
    }

    #[test]
    fn rejects_every_non_named_struct_field_placement() {
        for source in [
            "#[cfg_attr(any(), cpp_value_init)] struct S { value: u64 }",
            "struct S(#[cfg_attr(any(), cpp_value_init)] u64);",
            "enum E { V { #[cfg_attr(any(), cpp_value_init)] value: u64 } }",
            "enum E { V(#[cfg_attr(any(), cpp_value_init)] u64) }",
            "union U { #[cfg_attr(any(), cpp_value_init)] value: u64 }",
            "fn f(#[cfg_attr(any(), cpp_value_init)] value: u64) {}",
            "const cpp_value_init: u64 = 0;",
            "macro_rules! m { () => { #[cfg_attr(any(), cpp_value_init)] value: u64 } }",
        ] {
            let error = validate(source).expect_err("wrong marker placement must fail closed");
            assert!(
                error.contains(MARKER),
                "unexpected error for {source}: {error}"
            );
        }
    }

    #[test]
    fn rejects_non_scalar_and_non_builtin_field_types() {
        for ty in [
            "char",
            "f32",
            "f64",
            "String",
            "Alias",
            "[u8; 4]",
            "*const u8",
            "&'static u64",
            "Option<u64>",
            "std::primitive::u64",
        ] {
            let source = format!("struct S {{ #[cfg_attr(any(), cpp_value_init)] value: {ty} }}");
            let error = validate(&source).expect_err("non-scalar marker must fail closed");
            assert!(error.contains("bool"), "unexpected error for {ty}: {error}");
        }
    }
}
