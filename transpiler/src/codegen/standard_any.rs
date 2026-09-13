use super::*;

impl CodeGen {
    fn standard_any_object(&self, ty: &syn::Type) -> bool {
        let syn::Type::TraitObject(object) = self.peel_paren_group_type(ty) else {
            return false;
        };
        let mut any = false;
        let mut markers = HashSet::new();
        for bound in &object.bounds {
            match bound {
                syn::TypeParamBound::Lifetime(_) => {}
                syn::TypeParamBound::Trait(bound)
                    if bound.lifetimes.is_none()
                        && matches!(bound.modifier, syn::TraitBoundModifier::None)
                        && matches!(
                            bound.path.segments.last().map(|s| &s.arguments),
                            Some(syn::PathArguments::None)
                        ) =>
                {
                    if self.standard_future_path_is(&bound.path, "std::any::Any") {
                        if any {
                            return false;
                        }
                        any = true;
                    } else if self.standard_future_path_is(&bound.path, "std::marker::Send") {
                        if !markers.insert("Send") {
                            return false;
                        }
                    } else if self.standard_future_path_is(&bound.path, "std::marker::Sync") {
                        if !markers.insert("Sync") {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        any
    }

    fn emit_standard_any_value(&self, value: &syn::Expr) -> String {
        if matches!(
            value,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(_),
                ..
            })
        ) {
            return format!("std::string_view({})", self.emit_expr_to_string(value));
        }
        self.emit_expr_maybe_move(value)
    }

    pub(super) fn try_emit_standard_any_call(
        &self,
        call: &syn::ExprCall,
        expected: Option<&syn::Type>,
    ) -> Option<String> {
        if call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(function) = call.func.as_ref() else {
            return None;
        };
        if function.qself.is_some() {
            return None;
        }
        for (path, target) in [
            ("std::panic::catch_unwind", "rusty::panic::catch_unwind_std"),
            (
                "std::panic::resume_unwind",
                "rusty::panic::resume_unwind_std",
            ),
        ] {
            if self.standard_future_path_is(&function.path, path) {
                return Some(format!(
                    "{}({})",
                    target,
                    self.emit_expr_maybe_move(&call.args[0])
                ));
            }
        }
        if self.standard_future_path_is(&function.path, "std::panic::panic_any") {
            return Some(format!(
                "rusty::panic::panic_any({})",
                self.emit_standard_any_value(&call.args[0])
            ));
        }
        if self.standard_future_path_is(&function.path, "std::boxed::Box::new")
            && expected
                .or_else(|| self.current_return_type_hint())
                .and_then(|ty| self.try_map_standard_any_type(ty))
                .is_some()
        {
            return Some(format!(
                "rusty::any_types::BoxAny::new_({})",
                self.emit_standard_any_value(&call.args[0])
            ));
        }
        None
    }

    pub(super) fn try_map_standard_any_type(&self, ty: &syn::Type) -> Option<String> {
        self.map_standard_any_type_depth(ty, 0)
    }

    fn map_standard_any_type_depth(&self, ty: &syn::Type, depth: usize) -> Option<String> {
        if depth >= 16 {
            return None;
        }
        if let syn::Type::Reference(reference) = self.peel_paren_group_type(ty) {
            let inner = self.map_standard_any_type_depth(&reference.elem, depth + 1)?;
            return Some(if reference.mutability.is_some() {
                format!("{}&", inner)
            } else {
                format!("const {}&", inner)
            });
        }
        let mut ty = self.peel_paren_group_type(ty).clone();
        for _ in 0..8 {
            let Some(next) = self.resolve_type_alias_once(&ty) else {
                break;
            };
            if next == ty {
                break;
            }
            ty = next;
            if matches!(ty, syn::Type::Reference(_)) {
                return self.map_standard_any_type_depth(&ty, depth + 1);
            }
        }
        let syn::Type::Path(owner) = &ty else {
            return None;
        };
        if owner.qself.is_some() || !self.standard_future_path_is(&owner.path, "std::boxed::Box") {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &owner.path.segments.last()?.arguments
        else {
            return None;
        };
        let mut types = args.args.iter().filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        });
        let object = types.next()?;
        if types.next().is_some() || !self.standard_any_object(object) {
            return None;
        }
        Some("rusty::any_types::BoxAny".into())
    }
}
