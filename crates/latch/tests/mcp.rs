//! Verifies the tool's MCP result envelopes using the installed CLI's stdio entry point.

#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn saved_id(report: &str) -> &str {
    let log = report
        .lines()
        .find_map(|line| line.strip_prefix("Saved log: "))
        .expect("preview includes a readable log");
    std::path::Path::new(log)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
}

#[test]
fn supports_discovery_first_mcp_clients() {
    let project = tempfile::tempdir().expect("create project");
    let plugin = tempfile::tempdir().expect("create plugin directory");
    let state = tempfile::tempdir().expect("create state directory");
    fs::write(
        project.path().join("latch.json"),
        r#"{"defaults":{"failure_output":"compact"},"commands":[
            ["sh","-c","echo hidden MCP context; yes 'verbose' | head -500; echo 'error: explicit project'; exit 7"]
        ]}"#,
    )
    .expect("configure project check");
    let mut child = Command::new(env!("CARGO_BIN_EXE_latch"))
        .args(["mcp", "--require-project-directory"])
        .current_dir(plugin.path())
        .env("XDG_STATE_HOME", state.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("launch MCP tool");
    let mut input = child.stdin.take().expect("MCP input");
    let mut output = BufReader::new(child.stdout.take().expect("MCP output"));
    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"},
        "io.modelcontextprotocol/clientCapabilities": {},
    });
    for (id, method, mut params) in [
        (1, "server/discover", json!({})),
        (2, "tools/list", json!({})),
        (
            3,
            "tools/call",
            json!({"name": "run_turn_end", "arguments": {"project_directory": "relative"}}),
        ),
        (
            4,
            "tools/call",
            json!({"name": "run_turn_end", "arguments": {"project_directory": project.path()}}),
        ),
    ] {
        params["_meta"] = meta.clone();
        writeln!(
            input,
            "{}",
            json!({
                "jsonrpc": "2.0", "id": id, "method": method, "params": params
            })
        )
        .expect("send MCP request");
        let mut line = String::new();
        assert!(output.read_line(&mut line).expect("read MCP output") > 0);
        let response: Value = serde_json::from_str(&line).expect("MCP output is JSON");
        assert_eq!(response["id"], id);
        match id {
            1 => assert!(
                response["result"]["capabilities"]["tools"].is_object(),
                "{response}"
            ),
            2 => {
                assert_eq!(
                    response["result"]["tools"][0]["inputSchema"]["required"],
                    json!(["project_directory"])
                );
                assert_eq!(response["result"]["tools"][1]["name"], "search_output");
                assert_eq!(
                    response["result"]["tools"][1]["annotations"]["readOnlyHint"],
                    true
                );
                assert_eq!(
                    response["result"]["tools"][1]["inputSchema"]["required"],
                    json!(["id", "query", "project_directory"])
                );
            }
            3 => assert_eq!(
                response["error"]["message"],
                "project_directory must be an absolute path"
            ),
            _ => {
                let result = &response["result"];
                assert_eq!(result["isError"], true, "{response}");
                let message = result["content"][0]["text"].as_str().unwrap();
                assert!(message.contains("error: explicit project"), "{message}");
                assert!(!message.contains("hidden MCP context"), "{message}");
                let id = saved_id(message);
                let log = message
                    .lines()
                    .find_map(|line| line.strip_prefix("Saved log: "))
                    .unwrap();
                let retrieved = Command::new(env!("CARGO_BIN_EXE_latch"))
                    .args(["output", id, "--grep", "hidden MCP context"])
                    .current_dir(project.path())
                    .env("XDG_STATE_HOME", state.path())
                    .output()
                    .unwrap();
                assert!(retrieved.status.success(), "{retrieved:?}");
                assert_eq!(retrieved.stdout, b"hidden MCP context\n");
                for (request, directory, query, expected_error) in [
                    (5, project.path(), "hidden MCP context", None),
                    (
                        6,
                        project.path(),
                        "missing detail",
                        Some("no matching lines"),
                    ),
                    (
                        7,
                        plugin.path(),
                        "hidden MCP context",
                        Some("not found in this project"),
                    ),
                ] {
                    let query = json!({
                        "jsonrpc":"2.0","id":request,"method":"tools/call",
                        "params":{"name":"search_output","_meta":meta,"arguments":{
                            "project_directory":directory,"id":log,"query":query
                        }}
                    });
                    writeln!(input, "{query}").unwrap();
                    let mut line = String::new();
                    assert!(output.read_line(&mut line).unwrap() > 0);
                    let search: Value = serde_json::from_str(&line).unwrap();
                    assert_eq!(search["id"], request);
                    let result = &search["result"];
                    let message = result["content"][0]["text"].as_str().unwrap();
                    if let Some(expected) = expected_error {
                        assert_eq!(result["isError"], true);
                        assert!(message.contains(expected), "{search}");
                    } else {
                        assert_ne!(result["isError"], true);
                        assert_eq!(message, "stdout:\nhidden MCP context\n");
                    }
                }
            }
        }
    }
    drop(input);
    assert!(
        child
            .wait()
            .expect("MCP client disconnects cleanly")
            .success()
    );
}

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

    let status = json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"latch_status","arguments":{}}});
    writeln!(input, "{status}").unwrap();
    let result = response(9);
    let snapshot: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(snapshot["checks"][1]["config"]["configured"], true);
    assert!(snapshot["last_prompt_unix_seconds"].is_null());

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
