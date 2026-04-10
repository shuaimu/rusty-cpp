use crate::codegen::CodeGen;
use crate::types::UserTypeMap;
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranspileOptions {
    /// Opt-in diagnostic-only prototype for by-value SCC cycle-breaking planning.
    /// Default is `false`.
    pub by_value_cycle_breaking_prototype: bool,
}

/// Transpile Rust source code to C++ code.
/// If `module_name` is provided, emit C++20 module declarations.
pub fn transpile(rust_source: &str, module_name: Option<&str>) -> Result<String, String> {
    transpile_with_type_map(rust_source, module_name, &UserTypeMap::default())
}

/// Transpile with user-provided type mappings for external crate types.
pub fn transpile_with_type_map(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
) -> Result<String, String> {
    transpile_with_type_map_and_extension_hints_and_options(
        rust_source,
        module_name,
        type_map,
        &HashSet::new(),
        &TranspileOptions::default(),
    )
}

/// Transpile with user-provided type mappings plus cross-source extension-method hints.
pub fn transpile_with_type_map_and_extension_hints(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
) -> Result<String, String> {
    transpile_with_type_map_and_extension_hints_and_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        &TranspileOptions::default(),
    )
}

/// Transpile with user-provided type mappings plus cross-source extension-method
/// hints and explicit transpilation options.
pub fn transpile_with_type_map_and_extension_hints_and_options(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    options: &TranspileOptions,
) -> Result<String, String> {
    transpile_full_with_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        None,
        options,
    )
}

/// Transpile with all options including crate name for path stripping.
pub fn transpile_full(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
) -> Result<String, String> {
    transpile_full_with_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        crate_name,
        &TranspileOptions::default(),
    )
}

/// Transpile with all options including crate name for path stripping and
/// explicit transpilation options.
pub fn transpile_full_with_options(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
    options: &TranspileOptions,
) -> Result<String, String> {
    let file: syn::File = syn::parse_str(rust_source).map_err(|e| format!("Parse error: {}", e))?;

    let mut codegen = if extension_method_hints.is_empty() {
        CodeGen::with_type_map(type_map.clone())
    } else {
        CodeGen::with_type_map_and_extension_hints(type_map.clone(), extension_method_hints.clone())
    };
    if let Some(name) = crate_name {
        codegen.set_crate_name(name);
    }
    codegen.set_by_value_cycle_breaking_prototype(options.by_value_cycle_breaking_prototype);
    codegen.emit_file(&file, module_name);
    Ok(codegen.into_output())
}

/// Collect extension-method names from a Rust source unit.
/// A method is treated as extension-shaped when it appears in a trait impl
/// targeting a non-local type in that same source unit.
pub fn collect_extension_method_hints(rust_source: &str) -> HashSet<String> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return HashSet::new();
    };

    let mut local_types = HashSet::new();
    collect_local_declared_types(&file.items, &[], &mut local_types);

    let mut methods = HashSet::new();
    collect_extension_method_names(&file.items, &[], &local_types, &mut methods);
    methods
}

fn collect_local_declared_types(
    items: &[syn::Item],
    module_path: &[String],
    out: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Struct(s) => record_local_type(module_path, &s.ident.to_string(), out),
            syn::Item::Enum(e) => record_local_type(module_path, &e.ident.to_string(), out),
            syn::Item::Type(t) => record_local_type(module_path, &t.ident.to_string(), out),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_local_declared_types(nested, &nested_path, out);
                }
            }
            _ => {}
        }
    }
}

fn record_local_type(module_path: &[String], type_name: &str, out: &mut HashSet<String>) {
    out.insert(type_name.to_string());
    if !module_path.is_empty() {
        out.insert(format!("{}::{}", module_path.join("::"), type_name));
    }
}

fn collect_extension_method_names(
    items: &[syn::Item],
    module_path: &[String],
    local_types: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                if impl_block.trait_.is_none() {
                    continue;
                }
                let Some(tp) = (match impl_block.self_ty.as_ref() {
                    syn::Type::Path(tp) => Some(tp),
                    _ => None,
                }) else {
                    continue;
                };

                let raw_self_name = tp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let scoped_self_name = qualify_relative_path(&raw_self_name, module_path);
                if local_types.contains(&raw_self_name) || local_types.contains(&scoped_self_name) {
                    continue;
                }

                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        out.insert(method.sig.ident.to_string());
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_extension_method_names(nested, &nested_path, local_types, out);
                }
            }
            _ => {}
        }
    }
}

fn qualify_relative_path(raw: &str, module_path: &[String]) -> String {
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.is_empty() {
        return raw.to_string();
    }
    if parts.len() == 1 {
        if module_path.is_empty() {
            return raw.to_string();
        }
        return format!("{}::{}", module_path.join("::"), raw);
    }

    let mut resolved_prefix = module_path.to_vec();
    let mut idx = 0usize;
    let mut had_relative_prefix = false;
    while idx < parts.len() {
        match parts[idx] {
            "self" => {
                had_relative_prefix = true;
                idx += 1;
            }
            "super" => {
                had_relative_prefix = true;
                if !resolved_prefix.is_empty() {
                    resolved_prefix.pop();
                }
                idx += 1;
            }
            "crate" => {
                had_relative_prefix = true;
                resolved_prefix.clear();
                idx += 1;
            }
            _ => break,
        }
    }

    if !had_relative_prefix {
        return raw.to_string();
    }

    let mut out_parts = resolved_prefix;
    out_parts.extend(parts[idx..].iter().map(|s| s.to_string()));
    if out_parts.is_empty() {
        raw.to_string()
    } else {
        out_parts.join("::")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_basic() {
        let result = transpile("fn main() { let x = 42; }", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("void main()"));
        assert!(output.contains("const auto x = 42;"));
    }

    #[test]
    fn test_transpile_error() {
        let result = transpile("fn {{{ invalid", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_transpile_multiple_items() {
        let result = transpile(
            r#"
            struct Point { x: f64, y: f64 }
            const PI: f64 = 3.14159;
            fn distance(a: &Point, b: &Point) -> f64 {
                0.0
            }
        "#,
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("struct Point"));
        assert!(output.contains("constexpr double PI"));
        assert!(output.contains("double distance"));
    }

    #[test]
    fn test_transpile_complete_program() {
        let result = transpile(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }

            fn main() {
                let result = add(1, 2);
            }
        "#,
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("int32_t add(int32_t a, int32_t b)"));
        assert!(output.contains("return a + b;"));
        assert!(output.contains("void main()"));
        assert!(output.contains("add(1, 2)"));
    }

    #[test]
    fn test_transpile_with_module() {
        let result = transpile("pub fn hello() {}", Some("my_crate"));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("export module my_crate;"));
        assert!(output.contains("export void hello()"));
    }

    #[test]
    fn test_transpile_without_module() {
        let result = transpile("pub fn hello() {}", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.contains("export module"));
        // Without module mode, pub is ignored
    }

    #[test]
    fn test_collect_extension_method_hints_detects_non_local_impl_methods() {
        let src = r#"
            struct Local;
            trait TapOps { fn tap(self) -> Self; }
            impl TapOps for Local { fn tap(self) -> Self { self } }
            trait TapOptionOps<T> { fn tap_none<F>(self, f: F) -> Self; }
            impl<T> TapOptionOps<T> for Option<T> { fn tap_none<F>(self, f: F) -> Self { self } }
        "#;
        let hints = collect_extension_method_hints(src);
        assert!(hints.contains("tap_none"));
        assert!(!hints.contains("tap"));
    }

    #[test]
    fn test_transpile_with_extension_hints_rewrites_method_calls() {
        let mut hints = HashSet::new();
        hints.insert("tap".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            "fn f() { let _ = 10.tap(); }",
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("static_cast<void>(rusty::tap(10));"));
    }

    #[test]
    fn test_transpile_options_toggle_by_value_cycle_breaking_prototype_diagnostics() {
        let src = r#"
            struct A {
                b: B,
            }

            struct B {
                a: A,
            }
        "#;
        let default_out = transpile(src, None).expect("default transpile should succeed");
        assert!(
            !default_out.contains("// PROTOTYPE: by-value cycle-breaking flag enabled"),
            "default mode should not emit prototype cycle-breaking diagnostics\nGot: {default_out}"
        );

        let options = TranspileOptions {
            by_value_cycle_breaking_prototype: true,
        };
        let opt_in_out = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("opt-in transpile should succeed");
        assert!(
            opt_in_out.contains("// PROTOTYPE: by-value cycle-breaking flag enabled"),
            "opt-in mode should emit prototype cycle-breaking diagnostics\nGot: {opt_in_out}"
        );
    }
}
