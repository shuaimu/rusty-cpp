use super::*;

fn generate(source: &str) -> String {
    let mut codegen = CodeGen::new();
    codegen.emit_file(&syn::parse_file(source).unwrap(), None);
    codegen.into_output()
}

#[test]
fn qualified_std_constructor_infers_mutex_and_rwlock_payloads() {
    let output = generate(
        r#"
        pub fn exercise() -> i32 {
            let mutex = std::sync::Mutex::new(7i32);
            let rwlock = std::sync::RwLock::new(11i32);
            let first = *mutex.lock().unwrap();
            let second = *rwlock.read().unwrap();
            first + second
        }
    "#,
    );
    assert!(output.contains("rusty::Mutex<int32_t>::new_("), "{output}");
    assert!(output.contains("rusty::RwLock<int32_t>::new_("), "{output}");
    assert!(!output.contains("std::sync::"), "{output}");
}

#[test]
fn qualified_std_constructor_preserves_explicit_type_arguments() {
    let output = generate(
        r#"
        pub fn mutex() -> std::sync::Mutex<i32> {
            std::sync::Mutex::<i32>::new(7)
        }
        pub fn rwlock() -> std::sync::RwLock<i32> {
            std::sync::RwLock::<i32>::new(11)
        }
    "#,
    );
    assert!(output.contains("rusty::Mutex<int32_t>::new_("), "{output}");
    assert!(output.contains("rusty::RwLock<int32_t>::new_("), "{output}");
    assert!(!output.contains("std::sync::"), "{output}");
}

#[test]
fn qualified_std_constructor_keeps_local_mutex_namesake() {
    let output = generate(
        r#"
        pub struct Mutex { pub value: i32 }
        impl Mutex { pub fn new(value: i32) -> Mutex { Mutex { value } } }
        pub fn exercise() -> i32 { let value = Mutex::new(7); value.value }
    "#,
    );
    assert!(output.contains("Mutex::new_("), "{output}");
    assert!(!output.contains("rusty::Mutex"), "{output}");
}

#[test]
fn qualified_std_constructor_keeps_local_std_module_identity() {
    let output = generate(
        r#"
        pub mod std {
            pub mod sync {
                pub struct Mutex<T> { pub value: T }
                impl<T> Mutex<T> {
                    pub fn new(value: T) -> Mutex<T> { Mutex { value } }
                }
            }
        }
        pub fn exercise() -> i32 {
            let value = std::sync::Mutex::<i32>::new(7);
            value.value
        }
    "#,
    );
    assert!(!output.contains("rusty::Mutex"), "{output}");
    assert!(
        output.contains("std_mod::sync_mod::Mutex<int32_t>::new_("),
        "{output}"
    );
}

#[test]
fn qualified_std_constructor_absolute_std_ignores_local_std_module() {
    let output = generate(
        r#"
        pub mod std {
            pub mod sync {
                pub struct Mutex { pub value: i32 }
            }
        }
        pub fn exercise() -> i32 {
            let value = ::std::sync::Mutex::new(7i32);
            let result = *value.lock().unwrap();
            result
        }
    "#,
    );
    assert!(output.contains("rusty::Mutex<int32_t>::new_("), "{output}");
}

#[test]
fn qualified_std_constructor_child_does_not_inherit_parent_std_module() {
    let output = generate(
        r#"
        pub mod std { pub mod sync {} }
        pub mod child {
            pub fn exercise() -> i32 {
                let value = std::sync::Mutex::new(7i32);
                let result = *value.lock().unwrap();
                result
            }
        }
    "#,
    );
    assert!(output.contains("rusty::Mutex<int32_t>::new_("), "{output}");
}

#[test]
fn qualified_std_constructor_exact_child_import_can_name_local_std_module() {
    let source = r#"
        pub mod std {
            pub mod sync {
                pub struct Mutex<T> { pub value: T }
                impl<T> Mutex<T> {
                    pub fn new(value: T) -> Mutex<T> { Mutex { value } }
                }
            }
        }
        pub mod child {
            use crate::std;
            pub fn exercise() -> i32 {
                let value = std::sync::Mutex::<i32>::new(7);
                value.value
            }
        }
    "#;
    let mut codegen = CodeGen::new();
    codegen.emit_file(&syn::parse_file(source).unwrap(), None);
    codegen.module_stack.push("child".into());
    let path: syn::Path = syn::parse_quote!(std::sync::Mutex);
    assert!(
        codegen.standard_path_root_is_local_module(&path),
        "modules={:?}, imports={:?}",
        codegen.declared_module_paths,
        codegen.rust_item_import_bindings
    );
    let owner = codegen.map_type(&syn::Type::Path(syn::TypePath { qself: None, path }));
    assert!(!owner.contains("rusty::Mutex"), "owner={owner}");
    let ty: syn::Type = syn::parse_quote!(std::sync::Mutex<i32>);
    let mapped = codegen.map_type(&ty);
    assert!(!mapped.contains("rusty::Mutex"), "mapped={mapped}");
    let call: syn::ExprCall = syn::parse_quote!(std::sync::Mutex::<i32>::new(7));
    let syn::Expr::Path(callee) = call.func.as_ref() else {
        unreachable!()
    };
    let emitted_path = codegen.emit_expr_path_to_string(&callee.path);
    assert!(
        !emitted_path.contains("rusty::Mutex"),
        "path={emitted_path}"
    );
    let recovered = codegen.emit_call_func_with_owner_template_recovery(&call, None);
    assert!(!recovered.contains("rusty::Mutex"), "recovered={recovered}");
    let inferred_hint: syn::Type = syn::parse_quote!(Mutex<i32>);
    let contextual = codegen.try_emit_associated_call_with_expected_type(&call, &inferred_hint);
    assert!(
        contextual
            .as_ref()
            .is_some_and(|value| value.contains("std_mod::sync_mod::Mutex<int32_t>::new_(")),
        "contextual={contextual:?}"
    );
    let emitted_call = codegen.emit_expr_to_string(&syn::Expr::Call(call));
    assert!(
        !emitted_call.contains("rusty::Mutex"),
        "call={emitted_call}"
    );
    codegen.module_stack.pop();
    let output = codegen.into_output();
    assert!(!output.contains("rusty::Mutex"), "{output}");
    assert!(
        output.contains("std_mod::sync_mod::Mutex<int32_t>::new_("),
        "{output}"
    );
}

#[test]
fn qualified_std_constructor_without_expected_type_keeps_local_owner() {
    let output = generate(
        r#"
        pub mod std {
            pub mod sync {
                pub struct Mutex<T> { pub value: T }
                impl<T> Mutex<T> {
                    pub fn new(value: T) -> Mutex<T> { Mutex { value } }
                }
            }
        }
        pub fn exercise() -> i32 {
            std::sync::Mutex::<i32>::new(7).value
                + std::sync::Mutex::new(11i32).value
        }
    "#,
    );
    assert_eq!(
        output
            .matches("std_mod::sync_mod::Mutex<int32_t>::new_(")
            .count(),
        2,
        "{output}"
    );
    assert!(!output.contains("std::sync::Mutex<"), "{output}");
    assert!(!output.contains("rusty::Mutex"), "{output}");
}
