//! Borrow a locked imported factory through its Box trait-object representation.

use std::path::Path;
use std::process::Command;

const PROVIDER: &str = include_str!("fixtures/imported_guard_coercion/provider.rs");
const CONSUMER: &str = include_str!("fixtures/imported_guard_coercion/consumer.rs");

fn checked(command: &mut Command) {
    let output = command.output().expect("run fixture command");
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn imported_guard_coercion_runs_the_mutable_factory() {
    let scratch = tempfile::tempdir().unwrap();
    let fixture = scratch.path().join("fixture");
    let source = fixture.join("src");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        fixture.join("Cargo.toml"),
        "[package]\nname='guard_owner'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        source.join("lib.rs"),
        "pub mod provider; pub mod consumer;\n",
    )
    .unwrap();
    std::fs::write(source.join("provider.rs"), PROVIDER).unwrap();
    std::fs::write(source.join("consumer.rs"), CONSUMER).unwrap();
    let rust_main = source.join("probe.rs");
    std::fs::write(
        &rust_main,
        "mod provider; mod consumer;\nfn main() {\n\
        assert_eq!(consumer::exercise(provider::make_factory()), 41);\n\
}\n",
    )
    .unwrap();
    let rust_exe = scratch.path().join("rust_run");
    checked(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(&rust_main)
            .arg("-o")
            .arg(&rust_exe),
    );
    checked(&mut Command::new(&rust_exe));
    // Keep the Rust runner outside the crate's provider inventory.
    std::fs::remove_file(rust_main).unwrap();

    let generated = scratch.path().join("generated");
    checked(
        Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(fixture.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&generated)
            .args(["--cxx-namespace", "example", "--offline"]),
    );

    let cxx = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    let compile = || {
        let mut command = Command::new(&cxx);
        command
            .args(["-std=c++23", "-stdlib=libc++", "-pthread"])
            .arg("-I")
            .arg(&include);
        command
    };
    let provider_pcm = scratch.path().join("provider.pcm");
    let consumer_pcm = scratch.path().join("consumer.pcm");
    let provider_map = format!(
        "-fmodule-file=guard_owner.provider={}",
        provider_pcm.display()
    );
    let consumer_map = format!(
        "-fmodule-file=guard_owner.consumer={}",
        consumer_pcm.display()
    );
    checked(
        compile()
            .arg("--precompile")
            .arg(generated.join("guard_owner.provider.cppm"))
            .arg("-o")
            .arg(&provider_pcm),
    );
    checked(
        compile()
            .arg(&provider_map)
            .arg("--precompile")
            .arg(generated.join("guard_owner.consumer.cppm"))
            .arg("-o")
            .arg(&consumer_pcm),
    );
    let provider_object = scratch.path().join("provider.o");
    let consumer_object = scratch.path().join("consumer.o");
    checked(
        compile()
            .arg("-c")
            .arg(&provider_pcm)
            .arg("-o")
            .arg(&provider_object),
    );
    checked(
        compile()
            .arg(&provider_map)
            .arg("-c")
            .arg(&consumer_pcm)
            .arg("-o")
            .arg(&consumer_object),
    );
    let importer = scratch.path().join("main.cc");
    std::fs::write(
        &importer,
        "import guard_owner.provider;\nimport guard_owner.consumer;\nint main() {\n\
        if (example::exercise(example::make_factory()) != 41) return 1;\n\
        return 0;\n}\n",
    )
    .unwrap();
    let cpp_exe = scratch.path().join("cpp_run");
    checked(
        compile()
            .arg(&provider_map)
            .arg(&consumer_map)
            .arg(&importer)
            .arg(&provider_object)
            .arg(&consumer_object)
            .arg("-o")
            .arg(&cpp_exe),
    );
    checked(&mut Command::new(cpp_exe));
}
