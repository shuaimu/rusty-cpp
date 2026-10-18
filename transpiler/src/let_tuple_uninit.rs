//! `let (a, b);` — a tuple pattern with no initializer declares deferred-init
//! locals (alloc's btree `split.rs`: `let (length_a, length_b); if .. {
//! length_a = ..; length_b = ..; } else { .. }`).
//!
//! The let emitter lowers a deferred-init `let x;` by typing `x` from its
//! later assignments; a tuple pattern has no such route and emitted nothing,
//! so every later assignment named an undeclared identifier. Split the pattern
//! into one `let` per binding before codegen — the annotated form
//! `let (a, b): (A, B);` splits with its element types — which is exactly what
//! the pattern means (nothing is destructured; there is no value yet).

use syn::visit_mut::VisitMut;

/// Split every uninitialized tuple-pattern `let` in `file`. Returns the count.
pub(crate) fn split_uninit_tuple_lets(file: &mut syn::File) -> usize {
    let mut pass = Split { rewritten: 0 };
    pass.visit_file_mut(file);
    pass.rewritten
}

struct Split {
    rewritten: usize,
}

impl VisitMut for Split {
    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        let mut out: Vec<syn::Stmt> = Vec::with_capacity(block.stmts.len());
        for stmt in block.stmts.drain(..) {
            match split_stmt(&stmt) {
                Some(parts) => {
                    self.rewritten += 1;
                    out.extend(parts);
                }
                None => out.push(stmt),
            }
        }
        block.stmts = out;
        syn::visit_mut::visit_block_mut(self, block);
    }
}

/// `let (a, mut b);` → `let a; let mut b;` (`let (a, b): (A, B);` →
/// `let a: A; let b: B;`). Only plain identifier elements qualify.
fn split_stmt(stmt: &syn::Stmt) -> Option<Vec<syn::Stmt>> {
    let syn::Stmt::Local(local) = stmt else {
        return None;
    };
    if local.init.is_some() {
        return None;
    }
    let (pat, ty): (&syn::Pat, Option<&syn::Type>) = match &local.pat {
        syn::Pat::Type(pt) => (pt.pat.as_ref(), Some(pt.ty.as_ref())),
        other => (other, None),
    };
    let syn::Pat::Tuple(tuple) = pat else {
        return None;
    };
    let elem_tys: Option<Vec<&syn::Type>> = match ty {
        Some(syn::Type::Tuple(tt)) if tt.elems.len() == tuple.elems.len() => {
            Some(tt.elems.iter().collect())
        }
        Some(_) => return None,
        None => None,
    };
    let mut out = Vec::with_capacity(tuple.elems.len());
    for (i, elem) in tuple.elems.iter().enumerate() {
        let syn::Pat::Ident(ident) = elem else {
            return None;
        };
        let pat = match elem_tys.as_ref() {
            Some(tys) => syn::Pat::Type(syn::PatType {
                attrs: Vec::new(),
                pat: Box::new(syn::Pat::Ident(ident.clone())),
                colon_token: Default::default(),
                ty: Box::new(tys[i].clone()),
            }),
            None => syn::Pat::Ident(ident.clone()),
        };
        out.push(syn::Stmt::Local(syn::Local {
            attrs: local.attrs.clone(),
            let_token: local.let_token,
            pat,
            init: None,
            semi_token: local.semi_token,
        }));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    fn rewrite(src: &str) -> (usize, String) {
        let mut file: syn::File = syn::parse_str(src).unwrap();
        let n = split_uninit_tuple_lets(&mut file);
        (n, file.to_token_stream().to_string())
    }

    #[test]
    fn splits_an_uninitialized_tuple_let_per_binding() {
        let (n, out) = rewrite("fn f(c: bool) -> (usize, usize) { let (a, mut b); if c { a = 1; b = 2; } else { b = 3; a = 4; } (a, b) }");
        assert_eq!(n, 1);
        assert!(out.contains("let a ; let mut b ;"), "{out}");
        assert!(!out.contains("let (a , mut b)"), "{out}");
    }

    #[test]
    fn an_annotated_tuple_let_splits_with_its_element_types() {
        let (n, out) = rewrite("fn f() { let (a, b): (u8, String); a = 1; b = String::new(); }");
        assert_eq!(n, 1);
        assert!(out.contains("let a : u8 ; let b : String ;"), "{out}");
    }

    #[test]
    fn initialized_and_nested_patterns_are_left_alone() {
        let (n, out) = rewrite("fn f() { let (a, b) = (1, 2); let ((c, d), e); }");
        assert_eq!(n, 0);
        assert!(out.contains("let (a , b) = (1 , 2) ;"), "{out}");
        assert!(out.contains("let ((c , d) , e) ;"), "{out}");
    }
}
