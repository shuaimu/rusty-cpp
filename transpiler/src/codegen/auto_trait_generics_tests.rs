use super::*;
use std::process::Command;

fn generate(source: &str, siblings: Vec<syn::ItemStruct>, aliases: Vec<syn::ItemType>) -> String {
    let mut cg = CodeGen::new();
    cg.set_cross_file_structs(siblings);
    cg.set_cross_file_type_aliases(aliases);
    cg.emit_file(&syn::parse_file(source).unwrap(), None);
    cg.into_output()
}

fn body<'a>(output: &'a str, name: &str) -> &'a str {
    output.split(&format!("struct {name} {{")).nth(1)
        .unwrap_or_else(|| panic!("missing {name}:\n{output}"))
        .split("};").next().unwrap()
}

#[test]
fn auto_trait_generics_imported_mutex_owner_matches_rust_and_cpp() {
    let definition = "pub struct SharedCell<T> { pub value: std::sync::Mutex<T> }";
    let source = r#"
        pub struct Shared { pub value: SharedCell<i32> }
        pub struct CellShared { pub value: SharedCell<std::cell::Cell<i32>> }
        pub struct NonSend {
            pub first: SharedCell<i32>,
            pub second: SharedCell<*mut i32>,
        }
        pub struct ReadShared { pub value: std::sync::RwLock<std::cell::Cell<i32>> }
    "#;
    let sibling = syn::parse_str(definition).unwrap();
    let consumer = generate(source, vec![sibling], vec![]);
    for owner in ["Shared", "CellShared"] {
        assert!(body(&consumer, owner).contains("is_send = true"), "{consumer}");
        assert!(body(&consumer, owner).contains("is_sync = true"), "{consumer}");
    }
    assert!(!body(&consumer, "NonSend").contains("is_send"), "{consumer}");
    assert!(!body(&consumer, "NonSend").contains("is_sync"), "{consumer}");
    assert!(body(&consumer, "ReadShared").contains("is_send = true"), "{consumer}");
    assert!(!body(&consumer, "ReadShared").contains("is_sync"), "{consumer}");

    let temp = tempfile::tempdir().unwrap();
    let rust_path = temp.path().join("owners.rs");
    let rust_source = format!("{definition}\n{source}\nfn check<T: Send + Sync>() {{}}\nfn proof() {{ check::<Shared>(); check::<CellShared>(); }}");
    std::fs::write(&rust_path, &rust_source).unwrap();
    let output = Command::new("rustc").args(["--edition=2024", "--crate-type=lib"])
        .arg(&rust_path).arg("-o").arg(temp.path().join("owners.rlib")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    for invalid in ["NonSend", "ReadShared"] {
        std::fs::write(&rust_path, format!("{rust_source}\nfn invalid() {{ check::<{invalid}>(); }}")).unwrap();
        let output = Command::new("rustc").args(["--edition=2024", "--crate-type=lib"])
            .arg(&rust_path).arg("-o").arg(temp.path().join("invalid.rlib")).output().unwrap();
        assert!(!output.status.success(), "Rust accepted {invalid} as Send + Sync");
        assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be"));
    }
    let cpp = format!("{}\n{consumer}\nstatic_assert(rusty::is_send<Shared>::value);\nstatic_assert(rusty::is_sync<Shared>::value);\nstatic_assert(rusty::is_sync<CellShared>::value);\nstatic_assert(!rusty::is_send<NonSend>::value);\nstatic_assert(!rusty::is_sync<NonSend>::value);\nstatic_assert(rusty::is_send<ReadShared>::value);\nstatic_assert(!rusty::is_sync<ReadShared>::value);\nint main() {{}}", generate(definition, vec![], vec![]));
    let cpp_path = temp.path().join("owners.cc");
    std::fs::write(&cpp_path, cpp).unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string());
    let output = Command::new(compiler).args(["-std=c++23", "-stdlib=libc++", "-fsyntax-only"])
        .arg("-I").arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&cpp_path).output().expect("clang++ is required for marker parity");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn auto_trait_generics_local_parameters_defaults_and_recursion() {
    let output = generate(r#"
        pub struct SharedCell<T> { pub value: std::sync::Mutex<T> }
        pub struct Generic<T> { pub value: SharedCell<T> }
        pub struct Defaulted<T = i32> { pub value: T }
        pub struct DefaultOwner { pub value: Defaulted }
        pub struct Node<T> { pub value: T, pub next: Option<Box<Node<T>>> }
        pub struct Good { pub value: Node<i32> }
        pub struct Bad { pub value: Node<std::rc::Rc<i32>> }
    "#, vec![], vec![]);
    assert!(body(&output, "Generic").contains("is_send = rusty::is_send<T>::value"), "{output}");
    assert!(body(&output, "Generic").contains("is_sync = rusty::is_send<T>::value"), "{output}");
    assert!(body(&output, "DefaultOwner").contains("is_sync = true"), "{output}");
    assert!(body(&output, "Good").contains("is_sync = true"), "{output}");
    assert!(!body(&output, "Bad").contains("is_send"), "{output}");
}

#[test]
fn auto_trait_generics_follow_callback_alias_bounds_and_keep_sync_distinct() {
    let shared = syn::parse_str("pub struct SharedCell<T>(std::sync::Mutex<T>);").unwrap();
    let callback = syn::parse_str("pub type Callback = rusty::Function<dyn FnMut() + Send>;").unwrap();
    let output = generate(r#"
        pub struct Owner { pub callback: SharedCell<Callback> }
        pub struct Direct { pub callback: Callback }
        pub struct UnsafeCallback { pub callback: rusty::Function<dyn FnMut()> }
    "#, vec![shared], vec![callback]);
    assert!(body(&output, "Owner").contains("is_send = true"), "{output}");
    assert!(body(&output, "Owner").contains("is_sync = true"), "{output}");
    assert!(body(&output, "Direct").contains("is_send = true"), "{output}");
    assert!(!body(&output, "Direct").contains("is_sync"), "{output}");
    assert!(!body(&output, "UnsafeCallback").contains("is_send"), "{output}");
}

#[test]
fn auto_trait_generics_do_not_override_source_owned_adapter_names() {
    let output = generate(r#"
        pub struct Mutex<T> { pub value: std::rc::Rc<T> }
        pub struct Function<T> { pub value: std::rc::Rc<T> }
        pub struct Owner { pub mutex: Mutex<i32>, pub callback: Function<i32> }
    "#, vec![], vec![]);
    assert!(!body(&output, "Owner").contains("is_send"), "{output}");
    assert!(!body(&output, "Owner").contains("is_sync"), "{output}");
    let output = generate(r#"
        pub struct Rc<T> { pub value: T }
        pub struct Owner { pub value: std::rc::Rc<i32> }
        pub mod rusty {
            pub struct Function<T> { pub value: std::rc::Rc<T> }
        }
        pub struct CallableOwner { pub value: rusty::Function<i32> }
    "#, vec![], vec![]);
    assert!(!body(&output, "Owner").contains("is_send"), "{output}");
    assert!(!body(&output, "CallableOwner").contains("is_send"), "{output}");
}

#[test]
fn auto_trait_generics_do_not_guess_ambiguous_sibling_declarations() {
    for name in ["Shared", "Mutex", "Function"] {
        let output = generate(&format!("pub struct Owner {{ pub value: {name}<i32> }}"), vec![
            syn::parse_str(&format!("pub struct {name}<T>(T);")).unwrap(),
            syn::parse_str(&format!("pub struct {name}<T>(std::rc::Rc<T>);")).unwrap(),
        ], vec![]);
        assert!(!body(&output, "Owner").contains("is_send"), "{output}");
    }
}

#[test]
fn auto_trait_concrete_impl_and_standard_handles_match_rust_and_cpp() {
    let source = r#"
        use std::sync::{Weak as ArcWeak, Condvar, Mutex};
        pub struct SendOnly { pub pointer: *mut i32 }
        unsafe impl Send for SendOnly {}
        pub struct Direct { pub value: SendOnly }
        pub struct Protected { pub value: Mutex<SendOnly>, pub condition: Condvar }
        pub struct WeakOwner { pub value: ArcWeak<Protected> }
        pub struct BadWeak { pub value: std::sync::Weak<std::cell::Cell<i32>> }
        pub struct RcWeakOwner { pub value: std::rc::Weak<i32> }
        pub struct SenderOwner { pub value: std::sync::mpsc::Sender<std::cell::Cell<i32>> }
        pub struct ReceiverOwner { pub value: std::sync::mpsc::Receiver<std::cell::Cell<i32>> }
        pub struct JoinOwner { pub value: std::thread::JoinHandle<()> }
    "#;
    let output = generate(source, vec![], vec![]);
    for name in ["Protected", "WeakOwner", "SenderOwner", "JoinOwner"] {
        assert!(body(&output, name).contains("is_send = true"), "{name}: {output}");
        assert!(body(&output, name).contains("is_sync = true"), "{name}: {output}");
    }
    for name in ["Direct", "ReceiverOwner"] {
        assert!(body(&output, name).contains("is_send = true"), "{name}: {output}");
        assert!(!body(&output, name).contains("is_sync"), "{name}: {output}");
    }
    for name in ["BadWeak", "RcWeakOwner"] {
        assert!(!body(&output, name).contains("is_send"), "{name}: {output}");
        assert!(!body(&output, name).contains("is_sync"), "{name}: {output}");
    }
    let temp = tempfile::tempdir().unwrap();
    let rust_path = temp.path().join("handles.rs");
    let rust_source = format!("{source}\nfn both<T: Send + Sync>() {{}}\nfn send<T: Send>() {{}}\nfn proof() {{ both::<Protected>(); both::<WeakOwner>(); both::<SenderOwner>(); both::<JoinOwner>(); send::<Direct>(); send::<ReceiverOwner>(); both::<std::thread::JoinHandle<std::rc::Rc<i32>>>(); }}");
    std::fs::write(&rust_path, &rust_source).unwrap();
    let result = Command::new("rustc").args(["--edition=2024", "--crate-type=lib"])
        .arg(&rust_path).arg("-o").arg(temp.path().join("handles.rlib")).output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    for name in ["Direct", "ReceiverOwner", "BadWeak", "RcWeakOwner"] {
        std::fs::write(&rust_path, format!("{rust_source}\nfn invalid() {{ both::<{name}>(); }}")).unwrap();
        let result = Command::new("rustc").args(["--edition=2024", "--crate-type=lib"])
            .arg(&rust_path).arg("-o").arg(temp.path().join("invalid.rlib")).output().unwrap();
        assert!(!result.status.success(), "Rust accepted {name} as Send + Sync");
    }
    let cpp_path = temp.path().join("handles.cc");
    // Rc weak and std mpsc need separately built standard-library modules.
    // Compile the direct-header fixture here; their marker judgments above
    // are still checked against rustc, including receiver and Rc negatives.
    let mut direct_header_source = syn::parse_file(source).unwrap();
    direct_header_source.items.retain(|item| !matches!(item, syn::Item::Struct(item)
        if matches!(item.ident.to_string().as_str(), "RcWeakOwner" | "SenderOwner" | "ReceiverOwner")));
    let mut cg = CodeGen::new();
    cg.emit_file(&direct_header_source, None);
    let cpp_output = cg.into_output();
    std::fs::write(&cpp_path, format!("{cpp_output}\nstatic_assert(rusty::is_send<Protected>::value && rusty::is_sync<Protected>::value);\nstatic_assert(rusty::is_send<WeakOwner>::value && rusty::is_sync<WeakOwner>::value);\nstatic_assert(rusty::is_send<Direct>::value && !rusty::is_sync<Direct>::value);\nstatic_assert(!rusty::is_send<BadWeak>::value);\nint main() {{}}" )).unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let result = Command::new(compiler).args(["-std=c++23", "-stdlib=libc++", "-fsyntax-only"])
        .arg("-I").arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&cpp_path).output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}

#[test]
fn auto_trait_concrete_impl_proof_is_scoped_and_not_a_generic_opt_in() {
    let output = generate(r#"
        pub mod approved {
            pub struct Buffer { pub pointer: *mut i32 }
            unsafe impl std::marker::Send for Buffer {}
        }
        pub mod other { pub struct Buffer { pub pointer: *mut i32 } }
        pub struct Good { pub value: std::sync::Mutex<approved::Buffer> }
        pub struct Bad { pub value: std::sync::Mutex<other::Buffer> }
        pub struct Generic<T> { pub pointer: *mut T }
        unsafe impl<T: Send> Send for Generic<T> {}
        pub struct NoBlanketClaim { pub value: Generic<std::rc::Rc<i32>> }
        pub mod spoof {
            pub unsafe trait Send {}
            pub struct Buffer { pub pointer: *mut i32 }
            unsafe impl Send for Buffer {}
        }
        pub struct NoSpoofClaim { pub value: spoof::Buffer }
    "#, vec![], vec![]);
    assert!(body(&output, "Good").contains("is_sync = true"), "{output}");
    for name in ["Bad", "NoBlanketClaim", "NoSpoofClaim"] {
        assert!(!body(&output, name).contains("is_send"), "{name}: {output}");
        assert!(!body(&output, name).contains("is_sync"), "{name}: {output}");
    }
    assert!(!output.contains("rusty::is_send<spoof::Buffer>"), "{output}");
}

#[test]
fn auto_trait_standard_handle_identity_rejects_shadowed_and_unbound_aliases() {
    let output = generate(r#"
        use std::sync::Weak as ArcWeak;
        pub mod child { pub struct Unbound { pub value: ArcWeak<i32> } }
        pub mod custom {
            pub struct Weak<T> { pub value: ::std::rc::Rc<T> }
            pub struct Condvar { pub pointer: *mut i32 }
        }
        use custom::Weak as OtherWeak;
        pub struct ShadowWeak { pub value: OtherWeak<i32> }
        pub struct ShadowCondvar { pub value: custom::Condvar }
        pub struct UnknownCondvar { pub value: external::Condvar }
    "#, vec![], vec![]);
    for name in ["Unbound", "ShadowWeak", "ShadowCondvar", "UnknownCondvar"] {
        assert!(!body(&output, name).contains("is_send"), "{name}: {output}");
        assert!(!body(&output, name).contains("is_sync"), "{name}: {output}");
    }
}

#[test]
fn auto_trait_cross_file_fields_keep_their_standard_import_identity() {
    let siblings = crate::transpile::collect_crate_struct_decls(r#"
        use std::marker::PhantomPinned as PinMarker;
        use std::sync::Weak as ArcWeak;
        pub struct Worker { pub marker: PinMarker, pub weak: ArcWeak<i32> }
        pub mod nested {
            use std::rc::Weak as ArcWeak;
            pub struct Local { pub weak: ArcWeak<i32> }
        }
    "#);
    let output = generate(r#"
        pub struct Good { pub worker: Worker }
        pub struct Bad { pub worker: Local }
    "#, siblings, vec![]);
    assert!(body(&output, "Good").contains("is_send = true"), "{output}");
    assert!(body(&output, "Good").contains("is_sync = true"), "{output}");
    assert!(!body(&output, "Bad").contains("is_send"), "{output}");
    assert!(!body(&output, "Bad").contains("is_sync"), "{output}");
}
