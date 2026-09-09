use super::*;

const PROVIDER: &str = include_str!("../../tests/fixtures/imported_guard_coercion/provider.rs");
const CONSUMER: &str = include_str!("../../tests/fixtures/imported_guard_coercion/consumer.rs");

fn prepared_consumer() -> CodeGen {
    let units = vec![
        (
            std::path::PathBuf::from("src/lib.rs"),
            "pub mod provider; pub mod consumer;".into(),
        ),
        (std::path::PathBuf::from("src/provider.rs"), PROVIDER.into()),
        (std::path::PathBuf::from("src/consumer.rs"), CONSUMER.into()),
    ];
    let audit =
        crate::cpp_abi::preflight_crate_plan_with_cxx_namespace(&units, Some("example"), None)
            .unwrap();
    let mut cg = CodeGen::new();
    cg.set_crate_name("guard_owner");
    cg.set_flat_import_type_authorizations(audit.flat_import_type_authorizations);
    cg.set_cross_file_type_aliases(
        syn::parse_file(PROVIDER)
            .unwrap()
            .items
            .into_iter()
            .filter_map(|item| match item {
                syn::Item::Type(item) => Some(item),
                _ => None,
            })
            .collect(),
    );
    cg
}

#[test]
fn imported_guard_coercion_requires_exact_scope_and_trait_identity() {
    let mut cg = prepared_consumer();
    cg.current_physical_module = crate::cpp_abi::ModulePath(vec!["consumer".into()]);
    let protected = "rusty::Box<example::FactoryBase>";
    let expected = "rusty::Box<FactoryBase>";
    assert!(cg.guard_coercion_mapped_types_match(protected, expected));
    let alias: syn::Type = syn::parse_quote!(FactoryProxy);
    let boxed_trait: syn::Type = syn::parse_quote!(Box<dyn FactoryBase>);
    assert!(cg.guard_coercion_target_matches(&alias, &boxed_trait));
    assert!(!cg.guard_coercion_mapped_types_match(
        protected,
        "rusty::MutexGuard<rusty::Box<FactoryBase>>"
    ));
    cg.module_stack.push("child".into());
    assert!(!cg.guard_coercion_mapped_types_match(protected, expected));
    assert!(!cg.guard_coercion_target_matches(&alias, &boxed_trait));
    cg.module_stack.clear();
    cg.current_physical_module = crate::cpp_abi::ModulePath(vec!["other".into()]);
    assert!(!cg.guard_coercion_mapped_types_match(protected, expected));
    cg.current_physical_module = crate::cpp_abi::ModulePath(vec!["consumer".into()]);
    assert!(!cg.guard_coercion_mapped_types_match("rusty::Box<unrelated::FactoryBase>", expected));
    cg.root_declared_type_names.insert("FactoryBase".into());
    assert!(!cg.guard_coercion_mapped_types_match(protected, expected));
    assert!(!cg.guard_coercion_target_matches(&alias, &boxed_trait));
    cg.root_declared_type_names.clear();
    let proofs = cg.flat_import_type_authorizations.clone();
    cg.flat_import_type_authorizations
        .retain(|proof| proof.provider_kind != crate::cpp_abi::FlatImportTypeProviderKind::Trait);
    assert!(!cg.guard_coercion_target_matches(&alias, &boxed_trait));
    cg.flat_import_type_authorizations = proofs;
    cg.cross_file_auto_trait_aliases
        .get_mut("FactoryProxy")
        .unwrap()
        .ty = Box::new(syn::parse_quote!(Box<dyn unrelated::FactoryBase>));
    assert!(!cg.guard_coercion_target_matches(&alias, &boxed_trait));
    cg.flat_import_type_authorizations.clear();
    assert!(!cg.guard_coercion_mapped_types_match(protected, expected));
    assert!(!cg.guard_coercion_target_matches(&alias, &boxed_trait));
}
