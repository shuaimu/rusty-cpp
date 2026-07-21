use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn annotate_safety_marks_unannotated_functions() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("annotate_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

int good() {
    return 1;
}

int bad() {
    return get_raw_int();
}

int wrapper() {
    return bad();
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg("--annotate-safety")
        .arg(&file_path)
        .output()
        .expect("run checker");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "annotator should succeed. stdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    let annotated = fs::read_to_string(&file_path).expect("read annotated source");
    assert!(
        annotated.contains("// @safe\nint good()"),
        "good function should be annotated safe:\n{}",
        annotated
    );
    assert!(
        annotated.contains("// @unsafe\nint bad()"),
        "bad function should be annotated unsafe:\n{}",
        annotated
    );
    assert!(
        annotated.contains("// @unsafe\nint wrapper()"),
        "wrapper should become unsafe after bad is discovered unsafe:\n{}",
        annotated
    );
    assert_eq!(
        annotated.matches("// @unsafe\nint get_raw_int();").count(),
        1,
        "existing unsafe annotation should not be duplicated:\n{}",
        annotated
    );
}
