use std::fmt::Write as _;
use std::process::Command;

fn transpiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
}

fn transpile_with_consumer_map(
    rust_source: &str,
    current_cpp_module: &str,
    modules: &[(&str, &str, &str)],
) -> String {
    transpile_with_consumer_map_and_scope(rust_source, current_cpp_module, None, modules)
}

fn transpile_with_consumer_map_and_scope(
    rust_source: &str,
    current_cpp_module: &str,
    consumer_rust_module: Option<&str>,
    modules: &[(&str, &str, &str)],
) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.rs");
    let map_path = dir.path().join("consumer-modules.toml");
    let output_path = dir.path().join("output.cppm");

    std::fs::write(&input, rust_source).expect("write Rust fixture");

    let mut map = String::from("version = 1\n");
    for (rust_module, cpp_module, cpp_namespace) in modules {
        writeln!(
            map,
            r#"
[[module]]
rust_module = "{rust_module}"
cpp_module = "{cpp_module}"
cpp_namespace = "{cpp_namespace}""#
        )
        .expect("write module-map fixture");
    }
    std::fs::write(&map_path, map).expect("write module map");

    let mut command = transpiler_bin();
    command
        .arg(&input)
        .arg("--module-name")
        .arg(current_cpp_module)
        .arg("--consumer-module-map")
        .arg(&map_path);
    if let Some(rust_module) = consumer_rust_module {
        command
            .arg("--consumer-rust-module")
            .arg(rust_module);
    }
    let output = command
        .arg("-o")
        .arg(&output_path)
        .output()
        .expect("run transpiler");
    assert!(
        output.status.success(),
        "transpiler failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::read_to_string(output_path).expect("read generated module")
}

const CLIENT_AND_BASE: &[(&str, &str, &str)] = &[
    ("crate::rpc::client", "rrr.client", "client_ns"),
    ("crate::base::sync", "rrr.basetypes", "base_ns"),
];

#[test]
fn grouped_epoll_implementation_uses_interface_projection_without_self_import() {
    let cpp = transpile_with_consumer_map_and_scope(
        r#"
use crate::runtime::epoll::PollMode;

pub fn platform_mask(mode: PollMode) -> i32 {
    mode.0 | PollMode::WRITE.0
}
"#,
        "rrr.epoll_wrapper",
        Some("crate::runtime::epoll_linux"),
        &[("crate::runtime::epoll", "rrr.epoll_wrapper", "rrr")],
    );

    assert!(cpp.contains("export module rrr.epoll_wrapper;"), "{cpp}");
    assert!(cpp.contains("namespace rrr {"), "{cpp}");
    assert!(cpp.contains("::rrr::PollMode"), "{cpp}");
    assert!(!cpp.contains("import rrr.epoll_wrapper;"), "{cpp}");
    assert!(!cpp.contains("runtime::epoll::PollMode"), "{cpp}");
}

#[test]
fn unrelated_imported_and_local_types_are_not_consumer_projected() {
    let cpp = transpile_with_consumer_map_and_scope(
        r#"
pub mod local {
    pub struct PollMode;
}

use crate::runtime::timer::Timer;
use self::local::PollMode as LocalMode;

pub fn retain(timer: Timer, mode: LocalMode) {
    let _ = timer;
    let _ = mode;
}
"#,
        "rrr.epoll_wrapper",
        Some("crate::runtime::epoll_linux"),
        &[("crate::runtime::epoll", "rrr.epoll_wrapper", "rrr")],
    );

    assert!(cpp.contains("runtime::timer::Timer"), "{cpp}");
    assert!(cpp.contains("local::PollMode"), "{cpp}");
    assert!(!cpp.contains("::rrr::Timer"), "{cpp}");
    assert!(!cpp.contains("::rrr::LocalMode"), "{cpp}");
    assert!(!cpp.contains("::rrr::PollMode"), "{cpp}");
}

#[test]
fn fully_qualified_cross_module_path_records_import_without_use_item() {
    let cpp = transpile_with_consumer_map(
        r#"
pub fn round_trip(
    value: crate::base::sync::Counter,
) -> crate::base::sync::Counter {
    let _fresh = crate::base::sync::make_counter();
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(cpp.contains("import rrr.basetypes;"), "{cpp}");
    assert!(cpp.contains("base_ns::Counter"), "{cpp}");
    assert!(cpp.contains("base_ns::make_counter()"), "{cpp}");
    assert!(!cpp.contains("::base::sync"), "{cpp}");
}

#[test]
fn nested_inline_self_and_super_use_the_full_rust_scope() {
    let modules = &[
        ("crate::rpc::client", "rrr.client", "client_ns"),
        (
            "crate::rpc::client::leaf",
            "rrr.wrong_leaf",
            "wrong_leaf_ns",
        ),
        ("crate::rpc::shared", "rrr.wrong_shared", "wrong_shared_ns"),
    ];
    let cpp = transpile_with_consumer_map(
        r#"
pub mod shared {
    pub struct Thing;
}

pub mod inner {
    pub mod leaf {
        pub struct Thing;
    }

    pub fn self_path(value: self::leaf::Thing) -> self::leaf::Thing {
        value
    }

    pub fn super_path(value: super::shared::Thing) -> super::shared::Thing {
        value
    }
}
"#,
        "rrr.client",
        modules,
    );

    assert!(cpp.contains("inner::leaf::Thing"), "{cpp}");
    assert!(cpp.contains("shared::Thing"), "{cpp}");
    assert!(!cpp.contains("wrong_leaf_ns::Thing"), "{cpp}");
    assert!(!cpp.contains("wrong_shared_ns::Thing"), "{cpp}");
}

#[test]
fn local_module_shadow_stops_consumer_lookup_from_walking_outward() {
    let cpp = transpile_with_consumer_map(
        r#"
pub mod base {
    pub mod sync {
        pub struct Counter;
    }
}

pub fn local(value: base::sync::Counter) -> base::sync::Counter {
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(
        cpp.contains("base::sync::Counter") || cpp.contains("base::sync_mod::Counter"),
        "{cpp}"
    );
    assert!(!cpp.contains("base_ns::Counter"), "{cpp}");
    assert!(!cpp.contains("import rrr.basetypes;"), "{cpp}");
}

#[test]
fn external_child_module_declaration_uses_its_mapped_cpp_module() {
    let modules = &[
        ("crate::rpc::client", "rrr.client", "client_ns"),
        ("crate::rpc::client::child", "rrr.special_child", "child_ns"),
    ];
    let cpp = transpile_with_consumer_map("pub mod child;", "rrr.client", modules);

    assert!(cpp.contains("export import rrr.special_child;"), "{cpp}");
    assert!(!cpp.contains("export import rrr.client.child;"), "{cpp}");
}

#[test]
fn mapped_glob_import_bridges_the_target_namespace() {
    let cpp = transpile_with_consumer_map(
        r#"
use crate::base::sync::*;

pub fn build() -> Counter {
    make_counter()
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(cpp.contains("import rrr.basetypes;"), "{cpp}");
    assert!(
        cpp.contains("using namespace ::base_ns;") || cpp.contains("using namespace base_ns;"),
        "{cpp}"
    );
    assert!(!cpp.contains("namespace sync = ::base_ns;"), "{cpp}");
}

#[test]
fn mapped_module_and_item_aliases_use_the_mapped_surface() {
    let cpp = transpile_with_consumer_map(
        r#"
use crate::base::sync as clocks;
use crate::base::sync::Counter as Count;

pub fn round_trip(value: Count) -> Count {
    let _fresh = clocks::make_counter();
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert_eq!(cpp.matches("import rrr.basetypes;").count(), 1, "{cpp}");
    assert!(
        cpp.contains("namespace clocks = ::base_ns;")
            || cpp.contains("namespace clocks = base_ns;"),
        "{cpp}"
    );
    assert!(
        cpp.contains("using Count = ::base_ns::Counter;")
            || cpp.contains("using Count = base_ns::Counter;"),
        "{cpp}"
    );
    assert!(cpp.contains("base_ns::make_counter()"), "{cpp}");
    assert!(!cpp.contains("base::sync::Counter"), "{cpp}");
}

#[test]
fn unqualified_mapped_root_is_resolved_before_external_rejection() {
    let cpp = transpile_with_consumer_map(
        r#"
use base::sync::Counter;

pub fn identity(value: Counter) -> Counter {
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(cpp.contains("import rrr.basetypes;"), "{cpp}");
    assert!(
        cpp.contains("using ::base_ns::Counter;") || cpp.contains("using base_ns::Counter;"),
        "{cpp}"
    );
    assert!(!cpp.contains("Rust-only unresolved import"), "{cpp}");
}

#[test]
fn leading_colon_external_path_is_not_consumer_projected() {
    let cpp = transpile_with_consumer_map(
        r#"
pub fn external_identity(
    value: ::base::sync::Counter,
) -> ::base::sync::Counter {
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(cpp.contains("::base::sync::Counter"), "{cpp}");
    assert!(!cpp.contains("base_ns::Counter"), "{cpp}");
    assert!(!cpp.contains("import rrr.basetypes;"), "{cpp}");
}

#[test]
fn mapped_nonterminal_generic_arguments_are_retained() {
    let cpp = transpile_with_consumer_map(
        r#"
pub fn invoke<T>() {
    crate::base::sync::Factory::<T>::make();
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(cpp.contains("import rrr.basetypes;"), "{cpp}");
    assert!(cpp.contains("base_ns::Factory<T>::make()"), "{cpp}");
    assert!(!cpp.contains("base_ns::Factory::make()"), "{cpp}");
}

#[test]
fn mapped_enum_pattern_uses_the_consumer_namespace() {
    let cpp = transpile_with_consumer_map(
        r#"
pub enum State {
    Ready,
    Busy,
}

pub fn is_ready(value: State) -> bool {
    match value {
        crate::rpc::client::State::Ready => true,
        _ => false,
    }
}
"#,
        "rrr.client",
        &[("crate::rpc::client", "rrr.client", "client_ns")],
    );

    assert!(cpp.contains("client_ns::State::Ready"), "{cpp}");
    assert!(!cpp.contains("crate::rpc::client::State::Ready"), "{cpp}");
}

#[test]
fn crate_root_map_projects_flattened_root_item_use() {
    let cpp = transpile_with_consumer_map(
        r#"
pub struct RootType;
use crate::RootType as Alias;

pub fn identity(value: Alias) -> Alias {
    value
}
"#,
        "rrr.root",
        &[("crate", "rrr.root", "root_ns")],
    );

    assert!(cpp.contains("using Alias = ::root_ns::RootType;"), "{cpp}");
    assert!(!cpp.contains("using Alias = ::RootType;"), "{cpp}");
    assert!(!cpp.contains("import rrr.root;"), "{cpp}");
}

#[test]
fn imported_shadow_does_not_fall_through_to_mapped_root() {
    let cpp = transpile_with_consumer_map(
        r#"
use dependency::other as base;

pub fn external(value: base::sync::Counter) -> base::sync::Counter {
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(!cpp.contains("base_ns::Counter"), "{cpp}");
    assert!(!cpp.contains("import rrr.basetypes;"), "{cpp}");
}

#[test]
fn leading_colon_use_is_not_consumer_projected() {
    let cpp = transpile_with_consumer_map(
        r#"
use ::base::sync::Counter;

pub fn external(value: Counter) -> Counter {
    value
}
"#,
        "rrr.client",
        CLIENT_AND_BASE,
    );

    assert!(!cpp.contains("base_ns::Counter"), "{cpp}");
    assert!(!cpp.contains("import rrr.basetypes;"), "{cpp}");
}
