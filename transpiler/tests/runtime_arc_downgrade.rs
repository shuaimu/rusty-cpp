use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX")
        && !cxx.trim().is_empty()
        && Command::new(&cxx)
            .arg("--version")
            .output()
            .ok()
            .is_some_and(|out| {
                out.status.success()
                    && String::from_utf8_lossy(&out.stdout)
                        .to_ascii_lowercase()
                        .contains("clang")
            })
    {
        return Some(cxx);
    }
    for candidate in ["clang++", "clang++-22", "clang++-21", "clang++-20"] {
        let status = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn project_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include")
}

fn transpiler_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
    if !path.exists() {
        path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/rusty-cpp-transpiler");
    }
    path
}

#[test]
fn transpiled_arc_downgrade_compiles_and_weak_tracks_strong_lifetime() {
    let Some(clang) = find_clang() else {
        eprintln!("skipping Arc downgrade runtime regression: clang++ not found");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust_path = temp.path().join("arc_downgrade.rs");
    let cpp_path = temp.path().join("arc_downgrade.cpp");
    let binary_path = temp.path().join("arc_downgrade.bin");
    let rust_source = r#"
        use std::sync::{Arc, Weak as ArcWeak};

        type SharedI32 = Arc<i32>;

        fn downgrade_reference(value: &Arc<i32>) -> ArcWeak<i32> {
            Arc::downgrade(value)
        }

        pub fn check_arc_downgrade() -> i32 {
            let strong = Arc::new(41);
            let weak: ArcWeak<i32> = downgrade_reference(&strong);
            let alias_weak: ArcWeak<i32> = SharedI32::downgrade(&strong);

            if weak.strong_count() != 1 { return 1; }
            if weak.weak_count() != 2 { return 2; }
            if alias_weak.weak_count() != 2 { return 3; }

            let upgraded = weak.upgrade();
            if upgraded.is_none() { return 4; }
            if **upgraded.as_ref().unwrap() != 41 { return 5; }

            drop(upgraded);
            drop(strong);
            if weak.upgrade().is_some() { return 6; }
            if alias_weak.upgrade().is_some() { return 7; }
            0
        }
    "#;
    std::fs::write(&rust_path, rust_source).expect("write Rust source");

    let transpile = Command::new(transpiler_bin())
        .arg(&rust_path)
        .arg("-o")
        .arg(&cpp_path)
        .output()
        .expect("run transpiler");
    assert!(
        transpile.status.success(),
        "transpile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&transpile.stdout),
        String::from_utf8_lossy(&transpile.stderr)
    );

    let mut cpp = std::fs::read_to_string(&cpp_path).expect("read generated C++");
    assert_eq!(
        cpp.matches("rusty::sync::downgrade(").count(),
        2,
        "both associated calls must use the free runtime seam:\n{cpp}"
    );
    assert!(!cpp.contains(">::downgrade("), "{cpp}");
    cpp.push_str("\nint main() { return check_arc_downgrade(); }\n");
    std::fs::write(&cpp_path, cpp).expect("append C++ main");

    let compile = Command::new(&clang)
        .arg("-std=c++23")
        .arg("-Wno-deprecated-declarations")
        .arg("-I")
        .arg(project_include_dir())
        .arg(&cpp_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .expect("invoke clang++");
    assert!(
        compile.status.success(),
        "clang++ failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&binary_path)
        .output()
        .expect("run Arc downgrade regression");
    assert!(
        run.status.success(),
        "Arc/Weak lifetime regression exited {:?}\nstdout:\n{}\nstderr:\n{}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
