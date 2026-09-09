//! Imported method results retain their provider type inside Option branches.

use std::path::Path;
use std::process::Command;

const PROVIDER: &str = r#"
pub struct Queue { pub value: i32 }
pub struct Admission { pub value: i32 }
impl Queue {
    pub fn admit(&self) -> Admission { Admission { value: self.value } }
}
impl Admission {
    pub fn notify(self) -> i32 { self.value }
}
"#;

const CONSUMER: &str = r#"
#[cfg_attr(any(), cpp_import_namespace(example))]
use crate::queue::Queue;

pub struct Consumer { pub pending: Queue }

pub fn optional(enabled: bool) -> i32 {
    let conn = Consumer { pending: Queue { value: 37 } };
    let admission = {
        let selected = enabled;
        if selected {
            Some(conn.pending.admit())
        } else {
            None
        }
    };
    if let Some(value) = admission { value.notify() } else { -1 }
}
"#;

fn checked(command: &mut Command) {
    let output = command.output().expect("run fixture command");
    assert!(output.status.success(), "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

#[test]
fn imported_option_payload_compiles_and_runs_both_branches() {
    let scratch = tempfile::tempdir().unwrap();
    let fixture = scratch.path().join("fixture");
    let source = fixture.join("src");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(fixture.join("Cargo.toml"),
        "[package]\nname='option_owner'\nversion='0.1.0'\nedition='2024'\n").unwrap();
    std::fs::write(source.join("lib.rs"), "pub mod queue; pub mod consumer;\n").unwrap();
    std::fs::write(source.join("queue.rs"), PROVIDER).unwrap();
    std::fs::write(source.join("consumer.rs"), CONSUMER).unwrap();
    let rust_main = source.join("probe.rs");
    std::fs::write(&rust_main, "mod queue; mod consumer;\nfn main() {\n\
        assert_eq!(consumer::optional(true), 37);\n\
        assert_eq!(consumer::optional(false), -1);\n}\n").unwrap();
    let rust_exe = scratch.path().join("rust_run");
    checked(Command::new("rustc").arg("--edition=2024").arg(&rust_main).arg("-o").arg(&rust_exe));
    checked(&mut Command::new(&rust_exe));
    // Keep the Rust runner outside the crate's provider inventory.
    std::fs::remove_file(rust_main).unwrap();

    let generated = scratch.path().join("generated");
    checked(Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate").arg(fixture.join("Cargo.toml"))
        .arg("--output-dir").arg(&generated)
        .args(["--cxx-namespace", "example", "--offline"]));

    let cxx = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    let compile = || {
        let mut command = Command::new(&cxx);
        command.args(["-std=c++23", "-stdlib=libc++", "-pthread"])
            .arg("-I").arg(&include);
        command
    };
    let provider_pcm = scratch.path().join("queue.pcm");
    let consumer_pcm = scratch.path().join("consumer.pcm");
    let provider_map = format!("-fmodule-file=option_owner.queue={}", provider_pcm.display());
    let consumer_map = format!("-fmodule-file=option_owner.consumer={}", consumer_pcm.display());
    checked(compile().arg("--precompile").arg(generated.join("option_owner.queue.cppm"))
        .arg("-o").arg(&provider_pcm));
    checked(compile().arg(&provider_map).arg("--precompile")
        .arg(generated.join("option_owner.consumer.cppm")).arg("-o").arg(&consumer_pcm));
    let provider_object = scratch.path().join("queue.o");
    let consumer_object = scratch.path().join("consumer.o");
    checked(compile().arg("-c").arg(&provider_pcm).arg("-o").arg(&provider_object));
    checked(compile().arg(&provider_map).arg("-c").arg(&consumer_pcm).arg("-o").arg(&consumer_object));
    let importer = scratch.path().join("main.cc");
    std::fs::write(&importer, "import option_owner.consumer;\nint main() {\n\
        if (example::optional(true) != 37) return 1;\n\
        if (example::optional(false) != -1) return 2;\n\
        return 0;\n}\n").unwrap();
    let cpp_exe = scratch.path().join("cpp_run");
    checked(compile().arg(&provider_map).arg(&consumer_map).arg(&importer)
        .arg(&provider_object).arg(&consumer_object).arg("-o").arg(&cpp_exe));
    checked(&mut Command::new(cpp_exe));
}
