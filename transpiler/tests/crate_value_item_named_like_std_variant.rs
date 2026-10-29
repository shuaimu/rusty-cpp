//! Plain crate mode (`--crate`, one TU per file module): the root's
//! `use either::Left;` of a sibling FILE module's tuple struct `Left` is that
//! struct, although another module of the crate declares its own
//! `enum Either { Left(L), Right(R) }`. The bare-variant routes spelled it
//! `Alignment::Left(2)._0` (the relative-owner fallback consulted the sibling
//! enum's variant names) while `--crate-graph` was already clean. The oracle
//! is the same crate with the struct renamed `LeftX`, every emitted file
//! byte-equal after the rename.

use std::path::Path;
use std::process::Command;

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn left_crate(root: &Path, struct_name: &str) {
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"leftcrate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    write(
        &root.join("src/lib.rs"),
        &"pub mod either;\nuse either::NAME;\npub fn root_left() -> u32 { NAME(2).0 + either::left_total() }\n"
            .replace("NAME", struct_name),
    );
    write(
        &root.join("src/either.rs"),
        &"pub enum Either<L, R> { Left(L), Right(R) }\npub fn pick<L, R>(e: Either<L, R>) -> bool { matches!(e, Either::Left(_)) }\npub struct NAME(pub u32);\npub fn left_total() -> u32 { NAME(1).0 }\n"
            .replace("NAME", struct_name),
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
fn a_sibling_file_module_tuple_struct_named_left_is_that_struct_in_crate_mode() {
    let directory = tempfile::tempdir().unwrap();
    let subject_root = directory.path().join("subject");
    left_crate(&subject_root, "Left");
    let subject = transpile_crate(&subject_root, &directory.path().join("subject_out"));
    let oracle_root = directory.path().join("oracle");
    left_crate(&oracle_root, "LeftX");
    let oracle = transpile_crate(&oracle_root, &directory.path().join("oracle_out"));

    let root = subject.get("leftcrate.cppm").expect("root module emitted");
    assert!(root.contains("root_left"), "{root}");
    for (name, text) in &subject {
        assert!(!text.contains("Alignment::Left("), "{name}: {text}");
        assert!(!text.contains("Alignment::Right("), "{name}: {text}");
    }
    assert_eq!(
        subject.keys().collect::<Vec<_>>(),
        oracle.keys().collect::<Vec<_>>(),
        "same emitted files"
    );
    for (name, text) in &subject {
        assert_eq!(text, &oracle[name].replace("LeftX", "Left"), "{name}");
    }
}
