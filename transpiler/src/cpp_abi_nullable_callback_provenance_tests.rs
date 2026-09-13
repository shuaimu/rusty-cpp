use super::*;

fn callback_proof(provider: &str) -> Option<Type> {
    let source = syn::parse_file(provider).unwrap();
    nullable_callback_alias_source(&source.items, "EventTestFn")
        .map(|proof| syn::parse_str(&proof).unwrap())
}

#[test]
fn imported_event_test_callback_has_exact_provider_proof() {
    let provider = "pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;";
    let consumer = r#"
        #[cfg_attr(any(), cpp_import_namespace(probe))]
        use crate::reactor::EventTestFn;
        pub fn accepted(value: EventTestFn) {}
        pub fn qualified(value: crate::reactor::EventTestFn) {}
    "#;
    let units = vec![
        (PathBuf::from("src/lib.rs"), "pub mod reactor; pub mod consumer;".into()),
        (PathBuf::from("src/reactor.rs"), provider.into()),
        (PathBuf::from("src/consumer.rs"), consumer.into()),
    ];
    let audit = preflight_crate_plan_with_cxx_namespace(&units, Some("probe"), None).unwrap();
    let expected: Type = syn::parse_quote!(::core::option::Option<::std::boxed::Box<dyn ::core::ops::Fn(::core::primitive::i32) -> ::core::primitive::bool>>);
    let proofs = audit.flat_import_type_authorizations.iter()
        .filter(|proof| proof.leaf == "EventTestFn").collect::<Vec<_>>();
    assert_eq!(proofs.len(), 2);
    for proof in proofs {
        assert_eq!(proof.provider_physical_module, ModulePath(vec!["reactor".into()]));
        assert_eq!(syn::parse_str::<Type>(proof.nullable_callback_alias_source.as_deref().unwrap()).unwrap(), expected);
    }
}

#[test]
fn callback_proof_normalizes_standard_imports_and_stable_signatures() {
    let expected: Type = syn::parse_quote!(::core::option::Option<::std::boxed::Box<dyn ::core::ops::FnMut(&'static ::core::primitive::i32, ()) -> ::core::primitive::bool + ::core::marker::Send + ::core::marker::Sync + 'static>>);
    for provider in [
        "pub type EventTestFn = Option<Box<dyn FnMut(&'static i32, ()) -> bool + Send + Sync + 'static>>;",
        "use std::boxed::Box as Heap; use std::option::Option as Maybe; use std::ops::FnMut as Call; use std::marker::{Send as S, Sync as Y}; pub type EventTestFn = Maybe<Heap<dyn Call(&'static i32, ()) -> bool + S + Y + 'static>>;",
        "pub type Owned = Box<dyn FnMut(&'static i32, ()) -> bool + Send + Sync + 'static>; pub type EventTestFn = Option<Owned>;",
        "mod std {} pub type EventTestFn = ::std::option::Option<::std::boxed::Box<dyn ::std::ops::FnMut(&'static i32, ()) -> bool + ::std::marker::Send + ::std::marker::Sync + 'static>>;",
    ] {
        assert_eq!(callback_proof(provider), Some(expected.clone()), "{provider}");
    }
    let expected: Type = syn::parse_quote!(::core::option::Option<::std::boxed::Box<dyn ::core::ops::FnOnce(&mut ::core::primitive::u64)>>);
    assert_eq!(callback_proof("pub type EventTestFn = Option<Box<dyn FnOnce(&mut u64)>>;"), Some(expected));
}

#[test]
fn callback_proof_rejects_provider_standard_lookalikes() {
    for provider in [
        "pub struct Box<T: ?Sized> { pub pointer: *const T } pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;",
        "mod std { pub mod boxed { pub struct Box<T: ?Sized> { pub pointer: *const T } } } pub type EventTestFn = Option<std::boxed::Box<dyn Fn(i32) -> bool>>;",
        "mod std { pub mod boxed { pub struct Box<T: ?Sized> { pub pointer: *const T } } } use std::boxed::Box; pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;",
        "pub trait Fn {} pub type EventTestFn = Option<Box<dyn Fn>>;",
        "pub trait Fn<T> {} pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;",
        "mod std { pub mod ops { pub trait Fn<T> {} } } pub type EventTestFn = Option<Box<dyn std::ops::Fn(i32) -> bool>>;",
        "mod local { pub trait Fn<T> {} } use self::local::Fn; pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;",
        "pub trait Send {} pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool + Send>>;",
        "pub struct Option<T> { pub value: T } pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;",
    ] {
        assert!(callback_proof(provider).is_none(), "{provider}");
    }
}

#[test]
fn callback_proof_rejects_unstable_signature_names_and_extra_bounds() {
    for provider in [
        "pub struct Input; pub type EventTestFn = Option<Box<dyn Fn(Input) -> bool>>;",
        "type i32 = bool; pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool>>;",
        "type Count = i32; pub type EventTestFn = Option<Box<dyn Fn(Count) -> bool>>;",
        "pub type EventTestFn = Option<Box<dyn for<'a> Fn(&'a i32) -> bool>>;",
        "pub type EventTestFn = Option<Box<dyn Fn(&'a i32) -> bool>>;",
        "pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool + 'a>>;",
        "pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool + Clone>>;",
        "pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool + Send + Send>>;",
        "pub type EventTestFn = Option<Box<dyn Fn(i32) -> bool + FnMut(i32) -> bool>>;",
        "pub type EventTestFn = Option<std::sync::Arc<dyn Fn(i32) -> bool>>;",
    ] {
        assert!(callback_proof(provider).is_none(), "{provider}");
    }
}
