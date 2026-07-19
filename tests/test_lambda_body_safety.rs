use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn lambda_bodies_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("lambda_body_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
void takes_lambda(auto f) {
    f();
}

// @safe
void f() {
    auto local = []() {
        get_raw_int();
    };

    takes_lambda([]() {
        get_raw_int();
    });
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .output()
        .expect("run checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "checker should reject unsafe calls inside lambda bodies. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int") && stdout.contains("lambda body"),
        "lambda body unsafe call should be reported. Output: {}",
        stdout
    );
}

#[test]
fn unsafe_blocks_inside_lambda_bodies_are_allowed() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("lambda_body_unsafe_block.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
void f() {
    auto local = []() {
        // @unsafe
        {
            get_raw_int();
        }
    };

    local();
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .output()
        .expect("run checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "checker should allow unsafe calls inside explicit unsafe lambda blocks. stdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}
