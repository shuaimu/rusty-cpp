//! `<u8>::MAX` / `<u8>::from_str_radix(..)`: a bare qself on a primitive.
//!
//! bitflags' `impl Bits for $u { const ALL: $u = <$u>::MAX; fn parse_hex(..)
//! { <$u>::from_str_radix(input, 16) } }` spells the primitives' inherent
//! items through a qself. Every expression route lowers the plain
//! `u8::MAX` / `u8::from_str_radix(..)` spelling (numeric_limits, the
//! `rusty::from_str_radix<T>` helper); the qself form reached codegen with
//! its owner dropped (`::MAX`, `uint8_t::from_str_radix`). Rewrite the qself
//! into the plain path before codegen, for the primitives' own inherent items
//! only: a trait method the crate implements for a primitive
//! (`<u8>::parse_hex(s)`) keeps its qself, which routes it to the extension
//! helper.

use syn::visit_mut::VisitMut;

const PRIMITIVES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64", "char", "bool",
];

/// Inherent constants of the primitives that the plain-path lowering knows.
const CONSTS: &[&str] = &[
    "MAX", "MIN", "BITS", "EPSILON", "NAN", "INFINITY", "NEG_INFINITY", "MIN_POSITIVE", "MAX_EXP",
    "MIN_EXP", "DIGITS", "RADIX", "MANTISSA_DIGITS",
];

/// Inherent constructors of the primitives that the plain-path lowering knows.
const CTORS: &[&str] = &[
    "from_str_radix", "from_be_bytes", "from_le_bytes", "from_ne_bytes", "from_bits", "from_u32",
    "from_digit",
];

/// Rewrite every qualifying qself path in `file`. Returns the count.
pub(crate) fn rewrite_primitive_qself_paths(file: &mut syn::File) -> usize {
    let mut pass = Rewrite { rewritten: 0 };
    pass.visit_file_mut(file);
    pass.rewritten
}

struct Rewrite {
    rewritten: usize,
}

/// `<u8>::NAME` → `u8::NAME` when `NAME` is in `allowed`.
fn plain_path(path: &syn::ExprPath, allowed: &[&str]) -> Option<syn::ExprPath> {
    let qself = path.qself.as_ref()?;
    if qself.position != 0 || path.path.segments.len() != 1 {
        return None;
    }
    let syn::Type::Path(owner) = &*qself.ty else {
        return None;
    };
    if owner.qself.is_some() || owner.path.segments.len() != 1 {
        return None;
    }
    let owner_seg = &owner.path.segments[0];
    if !matches!(owner_seg.arguments, syn::PathArguments::None)
        || !PRIMITIVES.contains(&owner_seg.ident.to_string().as_str())
    {
        return None;
    }
    let name = path.path.segments[0].ident.to_string();
    if !allowed.contains(&name.as_str()) {
        return None;
    }
    let mut plain = owner.path.clone();
    plain.segments.push(path.path.segments[0].clone());
    Some(syn::ExprPath { attrs: path.attrs.clone(), qself: None, path: plain })
}

impl VisitMut for Rewrite {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        match expr {
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(func) = &*call.func
                    && let Some(plain) = plain_path(func, CTORS)
                {
                    call.func = Box::new(syn::Expr::Path(plain));
                    self.rewritten += 1;
                }
            }
            syn::Expr::Path(path) => {
                if let Some(plain) = plain_path(path, CONSTS) {
                    *path = plain;
                    self.rewritten += 1;
                }
            }
            _ => {}
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite(source: &str) -> (usize, String) {
        let mut file: syn::File = syn::parse_str(source).unwrap();
        let n = rewrite_primitive_qself_paths(&mut file);
        (n, quote::quote!(#file).to_string())
    }

    #[test]
    fn primitive_consts_and_ctors_lose_their_qself() {
        let (n, out) = rewrite(
            r#"
            const ALL: i8 = <u8>::MAX as i8;
            fn parse(s: &str) -> u8 { <u8>::from_str_radix(s, 16).unwrap() }
            "#,
        );
        assert_eq!(n, 2);
        assert!(out.contains("u8 :: MAX as i8"), "{out}");
        assert!(out.contains("u8 :: from_str_radix (s , 16)"), "{out}");
    }

    #[test]
    fn trait_methods_and_non_primitives_keep_their_qself() {
        let (n, out) = rewrite(
            r#"
            fn a(s: &str) -> u8 { <u8>::parse_hex(s) }
            fn b() -> usize { <Vec<u8>>::MAX }
            fn c() -> u8 { <u8 as Tr>::MAX }
            "#,
        );
        assert_eq!(n, 0);
        assert!(out.contains("< u8 > :: parse_hex"), "{out}");
    }
}
