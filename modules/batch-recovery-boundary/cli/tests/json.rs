use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_memoria"))
}

#[test]
fn json_success_and_request_errors_are_separate_streams() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let init = command()
        .args(["--json", "init", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(init.status.success());
    let reply: Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(reply["schema_version"], 1);
    assert!(init.stderr.is_empty());

    let batch = temp.path().join("bad.json");
    std::fs::write(&batch, b"{bad").unwrap();
    let bad = command()
        .args([
            "--json",
            "--repo",
            root.to_str().unwrap(),
            "apply",
            batch.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty());
    let error: Value = serde_json::from_slice(&bad.stderr).unwrap();
    assert_eq!(error["schema_version"], 1);
    assert_eq!(error["error"]["code"], "invalid_request");

    let unknown = command().args(["--json", "unknown"]).output().unwrap();
    assert!(!unknown.status.success());
    assert!(unknown.stdout.is_empty());
    let error: Value = serde_json::from_slice(&unknown.stderr).unwrap();
    assert_eq!(error["error"]["code"], "invalid_request");
}

#[test]
fn search_json_reports_lines_and_limit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    assert!(command()
        .args(["init", root.to_str().unwrap()])
        .output()
        .unwrap()
        .status
        .success());
    std::fs::write(root.join("knowledge/example.md"), "needle\nneedle\n").unwrap();
    let output = command()
        .args([
            "--json",
            "--repo",
            root.to_str().unwrap(),
            "search",
            "needle",
            "--limit",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply["schema_version"], 1);
    assert_eq!(reply["data"]["matches"][0]["path"], "knowledge/example.md");
    assert_eq!(reply["data"]["matches"][0]["line"], 1);
    assert_eq!(reply["data"]["matches"][0]["text"], "needle");
    assert_eq!(reply["data"]["limited"], true);
}

#[test]
fn invalid_utf8_stdin_is_a_typed_request_error() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    assert!(command()
        .args(["init", root.to_str().unwrap()])
        .output()
        .unwrap()
        .status
        .success());
    let mut child = command()
        .args([
            "--json",
            "--repo",
            root.to_str().unwrap(),
            "write",
            "knowledge/x.md",
            "--expect",
            "absent",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&[0xff]).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    let error: Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], "invalid_request");
    assert!(!root.join("knowledge/x.md").exists());
}
