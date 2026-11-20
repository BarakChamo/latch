//! Verifies the tool's MCP result envelopes using the installed CLI's stdio entry point.

#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn tool_returns_collected_failures_as_an_error_and_keeps_success_quiet() {
    let project = tempfile::tempdir().expect("create project");
    let config = project.path().join("latch.json");
    fs::write(
        &config,
        r#"{"hooks":{"turn_end":[
            {"on_failure":"stop","commands":[
                ["sh","-c","echo first; exit 7"],
                ["sh","-c","echo must-not-run; exit 9"]
            ]},
            {"extensions":["ts"],"commands":[
                ["sh","-c","echo hidden"],
                ["sh","-c","echo second >&2; exit 8"],
                ["sh","-c","echo third >&2; exit 10"]
            ]}
        ]}}"#,
    )
    .expect("configure collected failures");
    let mut child = Command::new(env!("CARGO_BIN_EXE_latch"))
        .arg("mcp")
        .current_dir(project.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("launch MCP tool");
    let mut input = child.stdin.take().expect("MCP input");
    let mut output = BufReader::new(child.stdout.take().expect("MCP output"));
    let mut response = |id| loop {
        let mut line = String::new();
        assert!(output.read_line(&mut line).expect("read MCP output") > 0);
        let message: Value = serde_json::from_str(&line).expect("MCP output is JSON");
        if message["id"] == json!(id) {
            break message["result"].clone();
        }
    };
    writeln!(input, "{}", json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}
    }})).unwrap();
    assert!(response(1)["capabilities"]["tools"].is_object());
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    )
    .unwrap();
    assert_eq!(response(2)["tools"][0]["name"], "run_turn_end");

    let call = |id| json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"run_turn_end","arguments":{}}});
    writeln!(input, "{}", call(3)).unwrap();
    let failure = response(3);
    assert_eq!(failure["isError"], true);
    let text = failure["content"][0]["text"]
        .as_str()
        .expect("failure text");
    assert!(text.contains("first") && text.contains("second") && text.contains("third"));
    assert!(!text.contains("hidden") && !text.contains("must-not-run"));
    let exits = [7, 8, 10].map(|code| text.find(&format!("exit status: {code}")).unwrap());
    assert!(exits[0] < exits[1] && exits[1] < exits[2]);

    fs::write(config, r#"{"commands":[["sh","-c","echo hidden"]]}"#).expect("configure success");
    writeln!(input, "{}", call(4)).unwrap();
    let success = response(4);
    assert_eq!(success["isError"], false);
    assert_eq!(success["content"], json!([]));
    drop(input);
    assert!(
        child
            .wait()
            .expect("MCP client disconnects cleanly")
            .success()
    );
}
