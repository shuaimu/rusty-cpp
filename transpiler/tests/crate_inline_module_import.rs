//! Crate mode: a sibling file module's `use` of an INLINE module
//! (`use super::node::{marker, …}` where `node.rs` holds `pub mod marker`)
//! names a namespace. A using-declaration cannot refer to one; the emitter
//! aliases it. The btree port's `map.rs` imports `node::marker` this way.

use std::path::Path;
use std::process::Command;

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// A btree-shaped crate: `btree::node` declares the inline `marker` module
/// and a type alias with an inherent impl; `btree::map` imports both.
fn nested_crate(root: &Path) {
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"inline_mod\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    write(&root.join("src/lib.rs"), "pub mod btree;\n");
    write(&root.join("src/btree/mod.rs"), "pub mod node;\npub mod map;\n");
    write(
        &root.join("src/btree/node.rs"),
        r#"
pub mod marker { pub struct Owned; pub struct LeafOrInternal; }
pub struct NodeRef<B, K, V, T> { pub h: usize, _p: core::marker::PhantomData<(B, K, V, T)> }
pub(super) type Root<K, V> = NodeRef<marker::Owned, K, V, marker::LeafOrInternal>;
impl<K, V> Root<K, V> {
    pub fn fix_top(&mut self) { self.h += 1; }
    pub fn split_off(&mut self, at: usize) -> Self { self.fix_top(); NodeRef { h: at, _p: core::marker::PhantomData } }
    pub fn new_pillar(height: usize) -> Root<K, V> { NodeRef { h: height, _p: core::marker::PhantomData } }
}
"#,
    );
    write(
        &root.join("src/btree/map.rs"),
        r#"
use super::node::{self, marker, NodeRef, Root};
pub struct Map<K, V> { root: Option<Root<K, V>> }
impl<K, V> Map<K, V> {
    pub fn go(&mut self, at: usize) -> Root<K, V> { let left_root = self.root.as_mut().unwrap(); left_root.split_off(at) }
    pub fn mk(h: usize) -> Root<K, V> { Root::new_pillar(h) }
}
"#,
    );
}

fn transpile_crate(root: &Path, out: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .args(["--auto-namespace", "--output-dir"])
        .arg(out)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--crate transpilation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(out.join("inline_mod.btree.map.cppm")).expect("map module emitted")
}

#[test]
fn inherent_impl_on_a_sibling_alias_of_a_crate_struct_is_a_member_impl() {
    // `impl<K, V> Root<K, V>` in node.rs is an impl on NodeRef (the alias's
    // target): the methods are members, so map.rs's `left_root.split_off(at)`
    // and `Root::new_pillar(h)` resolve without alias helpers whose generic
    // parameters nothing could deduce.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("crate");
    nested_crate(&root);
    let out = directory.path().join("out");
    let map = transpile_crate(&root, &out);
    let node = std::fs::read_to_string(out.join("inline_mod.btree.node.cppm")).expect("node module emitted");
    assert!(node.contains(" split_off(size_t at)"), "{node}");
    assert!(!node.contains("__rusty_alias_Root_"), "{node}");
    assert!(map.contains("left_root.split_off("), "{map}");
    assert!(!map.contains("__rusty_alias_Root_"), "{map}");
}

#[test]
fn sibling_use_of_an_inline_module_is_a_namespace_alias() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("crate");
    nested_crate(&root);
    let map = transpile_crate(&root, &directory.path().join("out"));
    assert!(
        map.contains("namespace marker = ::inline_mod::btree::node::marker;"),
        "{map}"
    );
    assert!(
        !map.contains("using ::inline_mod::btree::node::marker;"),
        "a using-declaration cannot refer to a namespace:\n{map}"
    );
}
