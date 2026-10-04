//! Codegen fixtures for `--verus-exec` (plan T6), through the command line.
//!
//! Every file in `tests/fixtures/verus_exec/` is a Verus source transpiled
//! by the real binary with the `rusty-cpp-verus-erase` helper (stage 1,
//! Verus's own erasure), the ghost-residue lowering (stage 2) and codegen.
//! The unit tests in `verus_exec.rs` / `verus_lower.rs` pin each pass on
//! Rust text; these pin what the whole pipeline emits, and that a fail-closed
//! rule really stops the run: non-zero exit, the rule's own diagnostic, and
//! no output file.
//!
//! A fixture states what it checks in `// @` comment lines:
//!
//! - `// @rule <name>`: a lowering rule the fixture exercises. Its C++ must be
//!   produced, carry no spec residue, and compile (`clang++ -fsyntax-only`,
//!   header-only, so fixtures use arrays, not `Vec`), unless the fixture says
//!   `// @compile no: <reason>`.
//! - `// @expect-cpp <text>` / `// @reject-cpp <text>`: the C++ contains /
//!   does not contain `text`.
//! - `// @expect-lowered <text>` / `// @reject-lowered <text>`: the same for
//!   the source codegen saw (`--dump-verus-erasure`, after stage 2).
//! - `// @error <id>` and `// @expect-error <text>`: the run fails with `text`
//!   on stderr and writes nothing.
//!
//! [`RULES`] and [`ERRORS`] are the inventories. Every rule and every
//! fail-closed diagnostic reachable from a source file needs a fixture, and
//! every diagnostic listed must still occur in the transpiler's source, so the
//! list cannot silently drift from the code. Diagnostics that no source file
//! can reach (the helper's output not lexing or not parsing, inner
//! attributes in it, stage 2 re-parsing stage 1's own output) are internal
//! consistency checks and are covered by unit tests or not at all.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// The lowering rules (plan T1-T3): T1's erasure and driver-cfg evaluation,
/// T2 rules 1-6, T3's vstd executable surface.
const RULES: &[&str] = &[
    "verus-erasure",
    "driver-cfg",
    "view",
    "ghost",
    "exec-surface",
    "prune-datatypes",
    "imports",
    "empty-statements",
];

/// Every fail-closed diagnostic a source file can reach: `(fixture id,
/// source file, text the source file must contain)`.
const ERRORS: &[(&str, &str, &str)] = &[
    // stage 1, verus_exec.rs
    ("unparseable-source", "verus_exec.rs", "--verus-exec could not parse the source"),
    ("attributes-on-verus", "verus_exec.rs", "attributes on a `verus!` invocation in"),
    ("verus-rejects-block", "verus_exec.rs", "Verus rejected a `verus!` block in"),
    ("other-verus-macro", "verus_exec.rs", "is a Verus macro this pass does not erase"),
    ("macro-generated-verus", "verus_exec.rs", "macro-generated Verus code is not erased by this pass"),
    ("driver-cfg-in-macro", "verus_exec.rs", "mentions a Verus driver cfg inside its tokens"),
    ("driver-cfg-unevaluated", "verus_exec.rs", "survives in a position this pass does not evaluate"),
    ("driver-cfg-file", "verus_exec.rs", "removes the whole module; unsupported"),
    ("malformed-driver-cfg", "verus_exec.rs", "malformed cfg predicate"),
    ("helper-missing", "verus_exec.rs", "could not be started"),
    ("helper-other-revision", "verus_exec.rs", "vendors a different Verus erasure"),
    // stage 2, verus_lower.rs
    ("reserved-marker", "verus_lower.rs", "is reserved for lowered Verus ghost state"),
    ("unknown-receiver", "verus_lower.rs", "cannot tell whether `.{method}(..)` on"),
    ("no-lowering", "verus_lower.rs", "has no C++ lowering"),
    ("ambiguous-datatype", "verus_lower.rs", "spec-only datatype name(s) also name executable datatypes"),
    ("vstd-path", "verus_lower.rs", "is a vstd path this pass does not lower"),
    ("spec-type", "verus_lower.rs", "the vstd spec type `{root}` is used in executable code"),
    ("ghost-survives", "verus_lower.rs", "survives lowering (only"),
    ("spec-trait", "verus_lower.rs", "the vstd spec trait `{root}` is used in executable code"),
    ("pruned-datatype", "verus_lower.rs", "(pruned because it reaches vstd spec types) is used in executable code"),
    ("ghost-flow-receiver", "verus_lower.rs", "\"the receiver of `.{}()`\""),
    ("ghost-flow-operand", "verus_lower.rs", "\"an operand\""),
    ("ghost-flow-unary", "verus_lower.rs", "Expr::Unary(unary) => self.reject(&unary.expr, \"an operand\")"),
    ("ghost-flow-field_base", "verus_lower.rs", "\"a field base\""),
    ("ghost-flow-indexed", "verus_lower.rs", "\"an indexed value\""),
    ("ghost-flow-index", "verus_lower.rs", "\"an index\""),
    ("ghost-flow-cast", "verus_lower.rs", "\"a cast operand\""),
    ("ghost-flow-if_condition", "verus_lower.rs", "Expr::If(expr_if) => self.reject(&expr_if.cond, \"a condition\")"),
    ("ghost-flow-while_condition", "verus_lower.rs", "Expr::While(expr_while) => self.reject(&expr_while.cond, \"a condition\")"),
    ("ghost-flow-scrutinee", "verus_lower.rs", "\"a match scrutinee\""),
    ("ghost-flow-argument", "verus_lower.rs", "which does not take a ghost value in that position"),
    ("ghost-flow-macro", "verus_lower.rs", "a ghost value is used inside"),
];

/// Not reachable from a fixture file: they need a helper that misbehaves.
const HELPER_ERRORS: &[&str] = &["helper-missing", "helper-other-revision"];

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

#[derive(Default, Debug)]
struct Fixture {
    path: PathBuf,
    rules: Vec<String>,
    errors: Vec<String>,
    expect_error: Vec<String>,
    expect_cpp: Vec<String>,
    reject_cpp: Vec<String>,
    expect_lowered: Vec<String>,
    reject_lowered: Vec<String>,
    compile: bool,
}

fn fixtures() -> Vec<Fixture> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/verus_exec");
    let mut paths = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let mut fixture = Fixture {
                compile: true,
                ..Fixture::default()
            };
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                let Some(directive) = line.strip_prefix("// @") else { continue };
                let (name, value) = directive.split_once(' ').unwrap_or((directive, ""));
                let value = value.to_string();
                match name {
                    "rule" => fixture.rules.push(value),
                    "error" => fixture.errors.push(value),
                    "expect-error" => fixture.expect_error.push(value),
                    "expect-cpp" => fixture.expect_cpp.push(value),
                    "reject-cpp" => fixture.reject_cpp.push(value),
                    "expect-lowered" => fixture.expect_lowered.push(value),
                    "reject-lowered" => fixture.reject_lowered.push(value),
                    "compile" => {
                        assert!(value.starts_with("no: "), "{}: `@compile no: <reason>`", path.display());
                        fixture.compile = false;
                    }
                    other => panic!("{}: unknown directive `@{other}`", path.display()),
                }
            }
            assert!(
                fixture.rules.is_empty() != fixture.errors.is_empty(),
                "{}: a fixture is either a @rule or an @error fixture",
                path.display()
            );
            assert!(
                fixture.errors.is_empty() || !fixture.expect_error.is_empty(),
                "{}: an @error fixture needs @expect-error",
                path.display()
            );
            fixture.path = path;
            fixture
        })
        .collect()
}

struct Run {
    output: Output,
    cpp_path: PathBuf,
    lowered: Option<String>,
    _dir: tempfile::TempDir,
}

fn transpile(source: &Path, helper: &Path) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let cpp_path = dir.path().join("out.cpp");
    let dump = dir.path().join("dump");
    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(source)
        .arg("-o")
        .arg(&cpp_path)
        .arg("--verus-exec")
        .arg("--verus-erase-helper")
        .arg(helper)
        .arg("--dump-verus-erasure")
        .arg(&dump)
        .output()
        .unwrap();
    let lowered = find_file(&dump).map(|path| std::fs::read_to_string(path).unwrap());
    Run {
        output,
        cpp_path,
        lowered,
        _dir: dir,
    }
}

/// The one file the dump directory holds.
fn find_file(dir: &Path) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).ok()? {
            let path = entry.ok()?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                return Some(path);
            }
        }
    }
    None
}

fn idents(source: &str) -> BTreeSet<String> {
    fn walk(tokens: proc_macro2::TokenStream, out: &mut BTreeSet<String>) {
        for tree in tokens {
            match tree {
                proc_macro2::TokenTree::Ident(ident) => {
                    out.insert(ident.to_string());
                }
                proc_macro2::TokenTree::Group(group) => walk(group.stream(), out),
                _ => {}
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(source.parse().expect("the lowered source lexes"), &mut out);
    out
}

fn clang() -> String {
    std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string())
}

#[test]
fn every_rule_and_fail_closed_diagnostic_has_a_fixture() {
    let fixtures = fixtures();
    let rules = fixtures.iter().flat_map(|f| f.rules.iter().cloned()).collect::<BTreeSet<_>>();
    let errors = fixtures.iter().flat_map(|f| f.errors.iter().cloned()).collect::<BTreeSet<_>>();
    for rule in RULES {
        assert!(rules.contains(*rule), "no fixture exercises rule `{rule}`");
    }
    for rule in &rules {
        assert!(RULES.contains(&rule.as_str()), "fixture rule `{rule}` is not in RULES");
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (id, file, text) in ERRORS {
        let source = std::fs::read_to_string(src.join(file)).unwrap();
        assert!(source.contains(text), "{file} no longer contains `{text}` ({id}); update ERRORS");
        assert!(
            errors.contains(*id) || HELPER_ERRORS.contains(id),
            "no fixture exercises the fail-closed diagnostic `{id}`"
        );
    }
    for id in &errors {
        assert!(ERRORS.iter().any(|(known, ..)| known == id), "fixture error `{id}` is not in ERRORS");
    }
}

#[test]
fn rule_fixtures_lower_compile_and_carry_no_spec_residue() {
    let helper = helper();
    let mut compiled = 0;
    for fixture in fixtures().iter().filter(|f| !f.rules.is_empty()) {
        let name = fixture.path.file_name().unwrap().to_string_lossy().into_owned();
        let run = transpile(&fixture.path, &helper);
        assert!(
            run.output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&run.output.stderr)
        );
        let cpp = std::fs::read_to_string(&run.cpp_path).unwrap();
        let lowered = run.lowered.as_deref().unwrap_or_else(|| panic!("{name}: no --dump-verus-erasure output"));
        for text in &fixture.expect_cpp {
            assert!(cpp.contains(text.as_str()), "{name}: C++ lacks `{text}`:\n{cpp}");
        }
        for text in &fixture.reject_cpp {
            assert!(!cpp.contains(text.as_str()), "{name}: C++ contains `{text}`:\n{cpp}");
        }
        for text in &fixture.expect_lowered {
            assert!(lowered.contains(text.as_str()), "{name}: lowered source lacks `{text}`:\n{lowered}");
        }
        for text in &fixture.reject_lowered {
            assert!(!lowered.contains(text.as_str()), "{name}: lowered source contains `{text}`:\n{lowered}");
        }
        // What codegen saw names no spec or ghost vocabulary and no Verus
        // syntax; the C++ has no hand-attention slot and no ghost type.
        let names = idents(lowered);
        for residue in [
            "vstd", "verus", "verifier", "View", "DeepView", "Seq", "Set", "Map", "nat", "int", "Ghost", "Tracked",
            "requires", "ensures", "proof", "spec",
        ] {
            assert!(!names.contains(residue), "{name}: `{residue}` reaches codegen:\n{lowered}");
        }
        for residue in ["vstd::", "TODO", "Ghost<", "Tracked<", "RustyVerusGhost", "Seq<", "verus_keep_ghost"] {
            assert!(!cpp.contains(residue), "{name}: C++ contains `{residue}`:\n{cpp}");
        }
        if !fixture.compile {
            continue;
        }
        let checked = Command::new(clang())
            .args([
                "-std=c++23",
                "-DRUSTY_PORTABLE_INTRINSICS=1",
                "-Werror=return-stack-address",
                "-fsyntax-only",
                "-I",
            ])
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
            .arg(&run.cpp_path)
            .output()
            .unwrap();
        assert!(
            checked.status.success(),
            "{name}: the C++ does not compile:\n{}\n{cpp}",
            String::from_utf8_lossy(&checked.stderr)
        );
        compiled += 1;
    }
    assert!(compiled > 0);
}

#[test]
fn error_fixtures_fail_closed_with_their_diagnostic() {
    let helper = helper();
    for fixture in fixtures().iter().filter(|f| !f.errors.is_empty()) {
        let name = fixture.path.file_name().unwrap().to_string_lossy().into_owned();
        let run = transpile(&fixture.path, &helper);
        let stderr = String::from_utf8_lossy(&run.output.stderr);
        assert!(!run.output.status.success(), "{name}: the transpiler accepted it");
        for text in &fixture.expect_error {
            assert!(stderr.contains(text.as_str()), "{name}: stderr lacks `{text}`:\n{stderr}");
        }
        assert!(!run.cpp_path.exists(), "{name}: a failed run wrote C++");
    }
}

const ANY_VERUS: &str = "use vstd::prelude::*;\nverus! {\npub fn f() -> u64 { 1 }\n}\n";

#[test]
fn a_missing_helper_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("any.rs");
    std::fs::write(&source, ANY_VERUS).unwrap();
    let run = transpile(&source, &dir.path().join("no-such-helper"));
    let stderr = String::from_utf8_lossy(&run.output.stderr);
    assert!(!run.output.status.success());
    assert!(stderr.contains("could not be started"), "{stderr}");
    assert!(!run.cpp_path.exists());
}

#[test]
fn a_helper_vendoring_another_verus_revision_fails_closed() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("any.rs");
    std::fs::write(&source, ANY_VERUS).unwrap();
    // A well-formed response from an erasure vendored at another revision.
    let fake = dir.path().join("rusty-cpp-verus-erase");
    std::fs::write(
        &fake,
        "#!/bin/sh\ncat > /dev/null\nprintf 'rusty-cpp-verus-erase/1\\nverus_builtin_macros 0.0.0-2025-11-10-1957\\nverus_git_rev 0000000000000000000000000000000000000000\\nblocks 1\\nok 0\\n\\n'\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let run = transpile(&source, &fake);
    let stderr = String::from_utf8_lossy(&run.output.stderr);
    assert!(!run.output.status.success());
    assert!(stderr.contains("vendors a different Verus erasure"), "{stderr}");
    assert!(!run.cpp_path.exists());
}
