//! Plain crate mode (`--crate`, one TU per file module): a tuple struct
//! declared in a sibling file module, matched by its constructor pattern
//! from the root (`use shapes::P; match p { P(a, b) => .. }`, `shapes::P(..)`)
//! and from another sibling (`use crate::R;`, `crate::shapes::P(..)`), is a
//! struct destructure in every TU — the cross-file struct tables carry the
//! arity — never the unresolved-bare-glob-variant fallback (a TODO marker in
//! expression position, an ill-formed `std::visit` in statement position).

use std::path::Path;
use std::process::Command;

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn shapes_crate(root: &Path) {
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"tspcrate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    write(
        &root.join("src/lib.rs"),
        "pub mod shapes;\npub mod user;\nuse shapes::P;\npub struct R(pub u32);\n\
         pub fn root_match(p: P) -> u32 { match p { P(a, b) => a + b } }\n\
         pub fn root_qualified(p: shapes::P) -> u32 { match p { shapes::P(a, _) => a } }\n\
         pub fn root_stmt(p: P) -> u32 { let mut s = 0; match p { P(a, b) => { s += a * b; } } s }\n",
    );
    write(
        &root.join("src/shapes.rs"),
        "pub struct P(pub u32, pub u32);\npub struct U;\n\
         pub fn local_match(p: P) -> u32 { match p { P(a, _) => a } }\n\
         pub fn local_unit(u: U) -> u32 { match u { U => 7 } }\n",
    );
    write(
        &root.join("src/user.rs"),
        "use crate::R;\nuse crate::shapes::P;\n\
         pub fn sibling_root_struct(r: R) -> u32 { match r { R(v) => v } }\n\
         pub fn sibling_sibling_struct(p: P) -> u32 { match p { P(a, b) => a * b } }\n\
         pub fn sibling_crate_path(p: P) -> u32 { match p { crate::shapes::P(a, b) => a - b } }\n",
    );
}

fn transpile_crate(root: &Path, out: &Path) -> std::collections::BTreeMap<String, String> {
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
    std::fs::read_dir(out)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).unwrap())
        })
        .filter(|(name, _)| name.ends_with(".cppm"))
        .collect()
}

#[test]
fn a_sibling_file_module_tuple_struct_pattern_destructures_in_every_tu() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("subject");
    shapes_crate(&root);
    let emitted = transpile_crate(&root, &directory.path().join("out"));
    assert_eq!(emitted.len(), 3, "{:?}", emitted.keys().collect::<Vec<_>>());
    for (name, text) in &emitted {
        assert!(!text.contains("unresolved bare-glob variant"), "{name}: {text}");
        assert!(!text.contains("std::visit(overloaded {\n"), "{name}: {text}");
        assert!(!text.contains(" == U)"), "{name}: {text}");
    }
    let root_tu = &emitted["tspcrate.cppm"];
    assert!(root_tu.contains("root_match"), "{root_tu}");
    assert!(
        root_tu.contains("auto&& b = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_m)._1);"),
        "{root_tu}"
    );
    assert!(root_tu.contains("bool _m_matched = false;"), "{root_tu}");
    let user_tu = &emitted["tspcrate.user.cppm"];
    assert!(
        user_tu.contains("auto&& v = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_m)._0);"),
        "{user_tu}"
    );
    assert_eq!(user_tu.matches("._1);").count(), 2, "{user_tu}");
}
