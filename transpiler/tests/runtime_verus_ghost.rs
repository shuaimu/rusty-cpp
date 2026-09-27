//! `--verus-exec` end to end: a Verus source with ghost state, `View` impls,
//! spec-only datatypes and vstd's `set` goes through the
//! `rusty-cpp-verus-erase` helper (stage 1, Verus's own erasure) and the
//! ghost-residue lowering (stage 2); the emitted C++ is compiled and RUN.
//!
//! The codegen unit fixtures (`codegen::verus_ghost_tests`) pin what is
//! emitted; this closes the half that asks whether `rusty::Ghost` and the
//! lowered `set` are real C++ that behaves like the Rust: tuple arity
//! kept, ghost fields taking no storage, `Clone`/`Copy` through the tag, and
//! `Debug` of a `Tracked` field printing nothing, as vstd's impl does.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const SOURCE: &str = r#"
use vstd::prelude::*;

verus! {

pub type InstantView = nat;
pub type Log = Seq<u64>;

#[derive(Clone, Copy)]
pub struct TimerEntry {
    pub deadline: u64,
    pub log_index: Ghost<int>,
}

#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub id: u64,
    pub perm: Tracked<u8>,
}

// An array rather than a Vec: this test compiles the header-only
// single-file output, where `rusty::Vec` (the vec_port module) is not
// available. vstd's `ArrayAdditionalExecFns::set` lowers exactly like
// `VecAdditionalExecFns::set` (index assignment).
pub struct Wheel {
    pub slots: [u64; 4],
    pub log: Ghost<Log>,
}

impl View for Wheel {
    type V = Seq<u64>;

    open spec fn view(&self) -> Seq<u64> {
        self.slots@
    }
}

impl Wheel {
    pub fn new() -> (r: Self)
        ensures
            r@.len() == 4,
    {
        let mut slots: [u64; 4] = [0; 4];
        let mut i: usize = 0;
        while i < 4
            invariant
                i <= 4,
            decreases 4 - i,
        {
            slots.set(i, 0);
            i = i + 1;
        }
        Wheel { slots, log: Ghost::assume_new_fallback(|| unreachable!()) }
    }

    pub fn put(&mut self, i: usize, v: u64)
        requires
            i < old(self)@.len(),
        ensures
            self@ == old(self)@.update(i as int, v),
    {
        self.slots.set(i, v);
    }

    pub fn entry(&self, i: usize) -> (TimerEntry, Ghost<InstantView>)
        requires
            i < self@.len(),
    {
        let log_index: Ghost<int> = Ghost::assume_new_fallback(|| unreachable!());
        let deadline: Ghost<InstantView> = Ghost::assume_new();
        proof {
            assert(i < self@.len());
        }
        (TimerEntry { deadline: self.slots[i], log_index }, deadline)
    }
}

pub fn check_wheel() -> i32 {
    let mut w = Wheel::new();
    w.put(2, 42);
    w.put(3, 7);
    let (e, _deadline) = w.entry(2);
    let copied = e;
    let cloned = e.clone();
    if copied.deadline == 42 && cloned.deadline == 42 && w.slots[3] == 7 && w.slots[0] == 0 {
        0
    } else {
        1
    }
}

pub fn token_debug() -> String {
    let token = Token { id: 7, perm: Tracked::assume_new() };
    let again = token;
    format!("{:?}", again)
}

} // verus!
"#;

/// The helper, built once per test process into its own target directory
/// (never the one this `cargo test` holds a lock on), unless
/// `$RUSTY_CPP_VERUS_ERASE` names one.
fn helper() -> PathBuf {
    static HELPER: OnceLock<PathBuf> = OnceLock::new();
    HELPER
        .get_or_init(|| {
            if let Some(path) = std::env::var_os("RUSTY_CPP_VERUS_ERASE").filter(|v| !v.is_empty()) {
                return PathBuf::from(path);
            }
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
            let target_dir = workspace.join("target").join("verus-erase-test-helper");
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let status = Command::new(cargo)
                .current_dir(&workspace)
                .args([
                    "build",
                    "--release",
                    "--locked",
                    "-p",
                    "verus-erase",
                    "--bin",
                    "rusty-cpp-verus-erase",
                    "--target-dir",
                ])
                .arg(&target_dir)
                .status()
                .expect("cargo runs");
            assert!(status.success(), "building the verus-erase helper failed");
            target_dir
                .join("release")
                .join(format!("rusty-cpp-verus-erase{}", std::env::consts::EXE_SUFFIX))
        })
        .clone()
}

#[test]
fn verus_ghost_state_and_vstd_set_compile_and_run_in_cpp() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("ghosts.rs");
    let generated = directory.path().join("ghosts.cpp");
    let dump = directory.path().join("dump");
    std::fs::write(&source, SOURCE).unwrap();
    let translated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&generated)
        .arg("--verus-exec")
        .arg("--verus-erase-helper")
        .arg(helper())
        .arg("--dump-verus-erasure")
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        translated.status.success(),
        "{}",
        String::from_utf8_lossy(&translated.stderr)
    );

    // What codegen saw: no spec vocabulary, ghost state as the marker.
    let lowered = std::fs::read_to_string(dump.join("ghosts.rs")).unwrap();
    let idents = syn_idents(&lowered);
    for residue in ["vstd", "View", "Seq", "nat", "int", "Log", "InstantView", "Ghost", "Tracked"] {
        assert!(
            !idents.iter().any(|ident| ident == residue),
            "`{residue}` survives stage 2:\n{lowered}"
        );
    }
    assert!(!lowered.contains(".set("), "{lowered}");

    let mut cpp = std::fs::read_to_string(&generated).unwrap();
    assert!(cpp.contains("[[no_unique_address]] rusty::Ghost log_index;"), "{cpp}");
    assert!(cpp.contains("this->slots.at(i) = std::move(v);"), "{cpp}");
    assert!(cpp.contains("std::tuple<TimerEntry, rusty::Ghost>"), "{cpp}");
    cpp.push_str(
        "\nint main() {\n    auto debug = token_debug();\n    std::printf(\"%d|%s|%zu\\n\", check_wheel(), std::string(debug.as_str()).c_str(), sizeof(TimerEntry));\n    return 0;\n}\n",
    );
    std::fs::write(&generated, cpp).unwrap();

    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string());
    let binary = directory.path().join("ghosts");
    let compiled = Command::new(compiler)
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-pthread", "-I"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&generated)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&compiled.stderr),
        std::fs::read_to_string(&generated).unwrap()
    );
    let ran = Command::new(&binary).output().unwrap();
    assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
    // 0: the wheel checks passed; Rust's `{:?}` of `Token` is
    // `Token { id: 7, perm:  }` (vstd's `Tracked` Debug writes nothing);
    // the ghost field adds no storage to `TimerEntry`.
    assert_eq!(
        String::from_utf8(ran.stdout).unwrap(),
        format!("0|Token {{ id: 7, perm:  }}|{}\n", std::mem::size_of::<u64>())
    );
}

/// Identifiers of a Rust source.
fn syn_idents(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(tokens: proc_macro2::TokenStream, out: &mut Vec<String>) {
        for tree in tokens {
            match tree {
                proc_macro2::TokenTree::Ident(ident) => out.push(ident.to_string()),
                proc_macro2::TokenTree::Group(group) => walk(group.stream(), out),
                _ => {}
            }
        }
    }
    walk(source.parse().unwrap(), &mut out);
    out
}
