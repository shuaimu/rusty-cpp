//! `fn f(P(a, b): P)` — a destructuring pattern in a free function's
//! parameter.
//!
//! Every parameter emitter names a parameter from a `Pat::Ident`; any other
//! pattern became a bare `_` with the pattern's bindings never declared, so
//! the body named undeclared identifiers (`a`, `b`). Two `_` parameters in
//! one signature were also a C++ parameter redefinition (clang 22, C++23).
//!
//! Rewrite the parameter to the positional placeholder `_argN` the member
//! emitters already use (`N` = the input's position in the signature) and
//! prepend `let <pattern> = _argN;` to the body: the let lowering already
//! destructures every pattern shape (tuple struct, tuple, struct, reference,
//! nested, `mut`, `_`, `..`), which is exactly what the parameter means. A
//! `_` parameter is renamed with no `let`. Free functions only (nested ones
//! included): impl and trait methods keep their own destructure route in the
//! member emitters.

use syn::visit_mut::VisitMut;

/// Rewrite every destructuring or `_` parameter of every free function in
/// `file`. Returns the count of rewritten parameters.
pub(crate) fn rewrite_fn_param_patterns(file: &mut syn::File) -> usize {
    let mut pass = Rewrite { rewritten: 0 };
    pass.visit_file_mut(file);
    pass.rewritten
}

struct Rewrite {
    rewritten: usize,
}

impl VisitMut for Rewrite {
    fn visit_item_fn_mut(&mut self, item: &mut syn::ItemFn) {
        self.rewritten += rewrite_signature(&mut item.sig, &mut item.block);
        syn::visit_mut::visit_item_fn_mut(self, item);
    }
}

/// `fn f(P(a, b): P, _: u8) { body }` →
/// `fn f(_arg0: P, _arg1: u8) { let P(a, b) = _arg0; body }`.
fn rewrite_signature(sig: &mut syn::Signature, block: &mut syn::Block) -> usize {
    let mut lets: Vec<syn::Stmt> = Vec::new();
    let mut rewritten = 0;
    for (idx, input) in sig.inputs.iter_mut().enumerate() {
        let syn::FnArg::Typed(pat_type) = input else {
            continue;
        };
        let destructures = match pat_type.pat.as_ref() {
            // `x`, `mut x`, `ref x`, `x @ ..` name the parameter already.
            syn::Pat::Ident(_) => continue,
            syn::Pat::Wild(_) => false,
            _ => true,
        };
        let placeholder = syn::Ident::new(&format!("_arg{idx}"), proc_macro2::Span::call_site());
        let placeholder_pat = syn::Pat::Ident(syn::PatIdent {
            attrs: Vec::new(),
            by_ref: None,
            mutability: None,
            ident: placeholder.clone(),
            subpat: None,
        });
        let original = std::mem::replace(pat_type.pat.as_mut(), placeholder_pat);
        if destructures {
            lets.push(syn::Stmt::Local(syn::Local {
                attrs: Vec::new(),
                let_token: Default::default(),
                pat: original,
                init: Some(syn::LocalInit {
                    eq_token: Default::default(),
                    expr: Box::new(syn::Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: syn::Path::from(placeholder),
                    })),
                    diverge: None,
                }),
                semi_token: Default::default(),
            }));
        }
        rewritten += 1;
    }
    block.stmts.splice(0..0, lets);
    rewritten
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    fn rewrite(src: &str) -> (usize, String) {
        let mut file: syn::File = syn::parse_str(src).unwrap();
        let n = rewrite_fn_param_patterns(&mut file);
        (n, file.to_token_stream().to_string())
    }

    #[test]
    fn a_destructuring_parameter_becomes_a_placeholder_plus_a_let() {
        let (n, out) = rewrite(
            "fn f(P(a, b): P, (c, d): (u8, u8), &S { x, .. }: &S, mut m: u8) -> u8 { a + b + c + d + x + m }",
        );
        assert_eq!(n, 3);
        assert!(
            out.contains("fn f (_arg0 : P , _arg1 : (u8 , u8) , _arg2 : & S , mut m : u8) -> u8 { let P (a , b) = _arg0 ; let (c , d) = _arg1 ; let & S { x , .. } = _arg2 ; a + b"),
            "{out}"
        );
    }

    #[test]
    fn a_wild_parameter_is_renamed_without_a_let() {
        let (n, out) = rewrite("fn f(_: u8, _: u8) -> u8 { 1 }");
        assert_eq!(n, 2);
        assert!(out.contains("fn f (_arg0 : u8 , _arg1 : u8) -> u8 { 1 }"), "{out}");
    }

    #[test]
    fn nested_free_functions_are_rewritten_but_methods_and_closures_are_not() {
        let (n, out) = rewrite(
            "struct P(u8, u8); impl P { fn m(P(a, b): P) -> u8 { fn inner(P(c, d): P) -> u8 { c + d } a + b } } fn g() { let f = |P(a, b): P| a + b; }",
        );
        assert_eq!(n, 1);
        assert!(out.contains("fn m (P (a , b) : P) -> u8 { fn inner (_arg0 : P) -> u8 { let P (c , d) = _arg0 ; c + d } a + b }"), "{out}");
        assert!(out.contains("| P (a , b) : P | a + b"), "{out}");
    }
}
