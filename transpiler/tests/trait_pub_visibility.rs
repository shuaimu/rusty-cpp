// Tests that `pub trait T` emits the C++ interface class at namespace
// scope (cross-module visible) while `trait T` (non-pub) wraps the
// class in `namespace { }` for module-internal linkage. The visibility
// keyword is the Rust idiom for this decision; mirrors what the
// transpiler already does for structs.

use std::process::Command;

fn transpiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
}

fn write_inline_rust(dir: &std::path::Path, name: &str, dsl: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let source = format!(
        r#"module;
export module {mod_name};
import std;
export namespace x {{
#if RUSTYCPP_RUST
{dsl}
#endif
/*RUSTYCPP:GEN-BEGIN id=t version=1 rust_sha256=0000000000000000000000000000000000000000000000000000000000000000*/
/*RUSTYCPP:GEN-END id=t*/
}}
"#,
        mod_name = name.trim_end_matches(".cpp"),
        dsl = dsl,
    );
    std::fs::write(&path, source).unwrap();
    path
}

#[test]
fn pub_trait_emits_at_namespace_scope_no_anon_wrap() {
    let dir = tempfile::tempdir().unwrap();
    let dsl = r#"
pub trait Job {
    fn ready(&mut self) -> bool;
    fn work(&mut self);
}

struct OneTimeJob { done_field: bool }
impl OneTimeJob { fn new() -> OneTimeJob { OneTimeJob { done_field: false } } }
impl Job for OneTimeJob {
    fn ready(&mut self) -> bool { !self.done_field }
    fn work(&mut self) { self.done_field = true; }
}
"#;
    let path = write_inline_rust(dir.path(), "pubtrait.cpp", dsl);
    let status = transpiler_bin()
        .args(["inline-rust", "--rewrite", "--files"])
        .arg(&path)
        .status()
        .expect("run transpiler");
    assert!(status.success(), "transpiler exited non-zero");

    let out = std::fs::read_to_string(&path).expect("read transpiled file");
    // The interface class itself should NOT be wrapped in `namespace { ... }`.
    // We check by looking at the line immediately before `class Job`.
    let class_pos = out
        .find("class Job ")
        .or_else(|| out.find("class Job {"))
        .expect("emitted class Job not found");
    let prefix = &out[..class_pos];
    // The last non-empty line before the class shouldn't be `namespace {`.
    let last_non_empty = prefix
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    assert!(
        last_non_empty.trim() != "namespace {",
        "pub trait Job should not be wrapped in anon namespace; \
         found prefix line: {:?}",
        last_non_empty
    );
}

#[test]
fn nonpub_trait_keeps_anon_namespace_wrap() {
    let dir = tempfile::tempdir().unwrap();
    let dsl = r#"
trait Job {
    fn ready(&mut self) -> bool;
    fn work(&mut self);
}

struct OneTimeJob { done_field: bool }
impl OneTimeJob { fn new() -> OneTimeJob { OneTimeJob { done_field: false } } }
impl Job for OneTimeJob {
    fn ready(&mut self) -> bool { !self.done_field }
    fn work(&mut self) { self.done_field = true; }
}
"#;
    let path = write_inline_rust(dir.path(), "privtrait.cpp", dsl);
    let status = transpiler_bin()
        .args(["inline-rust", "--rewrite", "--files"])
        .arg(&path)
        .status()
        .expect("run transpiler");
    assert!(status.success(), "transpiler exited non-zero");

    let out = std::fs::read_to_string(&path).expect("read transpiled file");
    let class_pos = out
        .find("class Job ")
        .or_else(|| out.find("class Job {"))
        .expect("emitted class Job not found");
    let prefix = &out[..class_pos];
    let last_non_empty = prefix
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    assert_eq!(
        last_non_empty.trim(),
        "namespace {",
        "non-pub trait should still be wrapped in anon namespace; \
         got prefix line: {:?}",
        last_non_empty
    );
}
