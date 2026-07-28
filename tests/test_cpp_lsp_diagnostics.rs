use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tempfile::TempDir;

#[test]
fn cpp_lsp_publishes_and_clears_diagnostics() {
    let dir = TempDir::new().expect("create temp dir");
    let lsp_bin = dir.path().join("rusty-cpp-lsp");
    let source_path = dir.path().join("lsp_diagnostics.cpp");
    let uri = format!("file://{}", source_path.display());

    let compiler = std::env::var("CXX").unwrap_or_else(|_| "c++".to_string());
    let compile_output = Command::new(compiler)
        .arg("-std=c++23")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-pedantic")
        .arg("-pthread")
        .arg("tools/rusty-cpp-lsp.cpp")
        .arg("-o")
        .arg(&lsp_bin)
        .output()
        .expect("compile C++ LSP server");
    assert!(
        compile_output.status.success(),
        "C++ LSP server should compile. stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let source = r#"
// @unsafe
int get_raw_int();

// @safe
void f() {
    get_raw_int();
}
"#;

    let mut child = Command::new(&lsp_bin)
        .env("RUSTY_CPP_CHECKER", env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start C++ LSP server");

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "initializationOptions": {
                        "compileCommands": ""
                    }
                }
            }),
        );
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "cpp",
                        "version": 1,
                        "text": source
                    }
                }
            }),
        );
    }

    let stdout = child.stdout.take().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let initialize_response = read_lsp(&mut reader);
    assert_eq!(initialize_response["id"], json!(1));
    assert_eq!(
        initialize_response["result"]["capabilities"]["textDocumentSync"]["change"],
        json!(1)
    );
    assert_eq!(
        initialize_response["result"]["capabilities"]["codeActionProvider"],
        json!(true)
    );

    let diagnostics = read_lsp(&mut reader);
    assert_eq!(
        diagnostics["method"],
        json!("textDocument/publishDiagnostics")
    );
    let published = diagnostics["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert_eq!(published.len(), 1, "expected one diagnostic: {diagnostics}");
    assert!(
        published[0]["message"]
            .as_str()
            .expect("diagnostic message")
            .contains("get_raw_int"),
        "diagnostic should mention unsafe call: {diagnostics}"
    );
    assert_eq!(
        published[0]["range"],
        json!({
            "start": {"line": 6, "character": 0},
            "end": {"line": 6, "character": 18}
        }),
        "diagnostic should underline the unsafe call line: {diagnostics}"
    );

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": r#"
// @unsafe
int get_raw_int();

// @safe
void f() {
    int x = 1;
}
"#
                    }]
                }
            }),
        );
    }

    let cleared_diagnostics = read_lsp(&mut reader);
    assert_eq!(
        cleared_diagnostics["method"],
        json!("textDocument/publishDiagnostics")
    );
    assert!(
        cleared_diagnostics["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .is_empty(),
        "didChange with clean source should clear diagnostics: {cleared_diagnostics}"
    );

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": 3
                    },
                    "contentChanges": [{
                        "text": source
                    }]
                }
            }),
        );
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": 4
                    },
                    "contentChanges": [{
                        "text": r#"
// @safe
void f() {
    int x = 1;
}
"#
                    }]
                }
            }),
        );
    }

    let newest_diagnostics = read_lsp(&mut reader);
    assert!(
        newest_diagnostics["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .is_empty(),
        "rapid edits should publish only the newest diagnostics: {newest_diagnostics}"
    );

    let unannotated_source = r#"
int helper() {
    return 1;
}
"#;

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": 5
                    },
                    "contentChanges": [{
                        "text": unannotated_source
                    }]
                }
            }),
        );
    }

    let unannotated_diagnostics = read_lsp(&mut reader);
    assert_eq!(
        unannotated_diagnostics["method"],
        json!("textDocument/publishDiagnostics")
    );

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/codeAction",
                "params": {
                    "textDocument": {
                        "uri": uri
                    },
                    "range": {
                        "start": {
                            "line": 2,
                            "character": 4
                        },
                        "end": {
                            "line": 2,
                            "character": 12
                        }
                    },
                    "context": {
                        "diagnostics": []
                    }
                }
            }),
        );
    }

    let code_actions = read_lsp(&mut reader);
    assert_eq!(code_actions["id"], json!(3));
    let actions = code_actions["result"]
        .as_array()
        .expect("code action response should be an array");
    assert_eq!(actions.len(), 2, "expected safe and unsafe actions: {code_actions}");
    assert_eq!(actions[0]["title"], json!("Mark function as @safe"));
    assert_eq!(actions[1]["title"], json!("Mark function as @unsafe"));
    assert_eq!(
        actions[0]["edit"]["changes"][uri.as_str()][0]["range"]["start"],
        json!({"line": 1, "character": 0})
    );
    assert_eq!(
        actions[0]["edit"]["changes"][uri.as_str()][0]["newText"],
        json!("// @safe\n")
    );
    assert_eq!(
        actions[1]["edit"]["changes"][uri.as_str()][0]["newText"],
        json!("// @unsafe\n")
    );

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": {
                    "textDocument": {
                        "uri": uri
                    }
                }
            }),
        );
    }

    let close_diagnostics = read_lsp(&mut reader);
    assert!(
        close_diagnostics["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .is_empty(),
        "didClose should clear diagnostics: {close_diagnostics}"
    );

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "shutdown",
                "params": null
            }),
        );
        write_lsp(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            }),
        );
    }
    let _ = child.wait();
}

fn write_lsp<W: Write>(writer: &mut W, message: &Value) {
    let body = serde_json::to_string(message).expect("serialize LSP message");
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body).expect("write LSP message");
    writer.flush().expect("flush LSP message");
}

fn read_lsp<R: BufRead>(reader: &mut R) -> Value {
    let mut content_length = None;

    loop {
        let mut header = String::new();
        reader.read_line(&mut header).expect("read LSP header");
        let header = header.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = Some(value.trim().parse::<usize>().expect("content length"));
            }
        }
    }

    let mut body = vec![0u8; content_length.expect("Content-Length header")];
    reader.read_exact(&mut body).expect("read LSP body");
    serde_json::from_slice(&body).expect("parse LSP JSON")
}
