//! Exercises the CLI from a project directory with real child processes.

use std::fs;
use std::process::{Command, Output};

fn run_latch(config: Option<&str>) -> Output {
    run_with_args(config, &[])
}

fn run_with_args(config: Option<&str>, args: &[&str]) -> Output {
    let project = tempfile::tempdir().expect("create temporary project");
    if let Some(config) = config {
        fs::write(project.path().join("latch.json"), config).expect("write latch.json");
    }
    Command::new(env!("CARGO_BIN_EXE_latch"))
        .current_dir(project.path())
        .args(args)
        .output()
        .expect("run latch")
}

#[test]
fn schema_reference_is_editor_metadata() {
    let config = r#"{"$schema":"./missing.schema.json","commands":[["check"]]}"#;
    let output = run_with_args(Some(config), &["explain"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let explanation = String::from_utf8(output.stdout).unwrap();
    assert!(explanation.contains("\"check\""));
    assert!(!explanation.contains("missing.schema.json"));
}

#[cfg(unix)]
#[test]
fn compact_failure_output_keeps_direct_cli_output_on_its_original_streams() {
    let output = run_latch(Some(
        r#"{"defaults":{"failure_output":"compact"},"commands":[
            ["sh","-c","yes 'verbose setup line' | head -500; echo 'error: cannot build' >&2; exit 7"]
        ]}"#,
    ));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .matches("verbose setup line")
            .count(),
        500
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("error: cannot build"));
}

#[test]
fn runs_commands_in_order_and_discards_success_output() {
    // Given a second command that needs the first command's work.
    // When both also produce output, then Latch exits successfully and silently.
    let output = run_latch(Some(
        r#"{"commands":[["sh","-c","touch ready; echo hidden"],["sh","-c","test -f ready || exit 9; echo hidden >&2"]]}"#,
    ));

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn reports_multiple_failures_and_still_runs_later_commands() {
    // Given an unsuccessful command, a missing executable, and another unsuccessful command.
    // When Latch runs them, then it emits each failure and exits nonzero.
    let output = run_latch(Some(
        r#"{"commands":[["sh","-c","printf first; printf diagnostic >&2; exit 7"],["./missing-latch-test-command"],["sh","-c","printf second; exit 8"]]}"#,
    ));

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"firstsecond");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("diagnostic"), "{stderr}");
    assert!(stderr.contains("missing-latch-test-command"), "{stderr}");
    assert!(stderr.contains("exit status: 8"), "{stderr}");
}

#[test]
fn rejects_invalid_config_before_launching_any_command() {
    // Given an invalid second command, then the first command must never be attempted.
    let output = run_latch(Some(r#"{"commands":[["./must-not-launch"],[]]}"#));

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("command 2"), "{stderr}");
    assert!(!stderr.contains("must-not-launch"), "{stderr}");
}

#[test]
fn reports_when_no_explicit_or_discovered_commands_exist() {
    let output = run_latch(None);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("hooks.turn_end is not configured"));
}

#[test]
fn discovers_javascript_scripts_using_the_declared_manager_or_lockfile() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(
        root.join("package.json"),
        r#"{
        "packageManager":"bun@1.3.0",
        "scripts":{"hook:afteredit":"echo example", "lint":"echo unrelated"}
    }"#,
    )
    .unwrap();
    fs::write(root.join("pnpm-lock.yaml"), "").unwrap();
    let explain = |hook| {
        let output = Command::new(env!("CARGO_BIN_EXE_latch"))
            .current_dir(root)
            .args(["explain", hook])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    let discovered = explain("after_edit");
    assert_eq!(discovered.get("config"), Some(&serde_json::Value::Null));
    assert_eq!(discovered["configured"], true);
    assert_eq!(
        discovered["commands"],
        serde_json::json!([["bun", "run", "hook:afteredit"]])
    );
    assert_eq!(explain("turn_end")["configured"], false);

    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"hook:turnend":"echo example"}}"#,
    )
    .unwrap();
    assert_eq!(
        explain("turn_end")["commands"],
        serde_json::json!([["pnpm", "run", "hook:turnend"]])
    );

    fs::write(
        root.join("latch.json"),
        r#"{"defaults":{"on_success":"output"}}"#,
    )
    .unwrap();
    let with_defaults = explain("turn_end");
    let config = with_defaults["config"].as_str().unwrap();
    assert_eq!(
        std::path::Path::new(config).canonicalize().unwrap(),
        root.join("latch.json").canonicalize().unwrap()
    );
    assert_eq!(with_defaults["on_success"], "output");
}

#[test]
fn discovered_scripts_share_defaults_and_run_after_explicit_entries() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"hook:afteredit":"echo example"}}"#,
    )
    .unwrap();
    fs::write(
        root.join("pyproject.toml"),
        r#"
[project.scripts]
hook-afteredit = "checks:after_edit"
"#,
    )
    .unwrap();
    fs::write(
        root.join("latch.json"),
        r#"{
        "defaults":{"on_success":"output","on_failure":"stop"},
        "hooks":{"after_edit":{"commands":[["explicit"]],"on_success":"silent"}}
    }"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_latch"))
        .current_dir(root)
        .args(["explain", "after_edit"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        plan["entries"][0]["commands"],
        serde_json::json!([["explicit"]])
    );
    assert_eq!(plan["entries"][0]["on_success"], "silent");
    assert_eq!(
        plan["entries"][1]["commands"],
        serde_json::json!([
            ["npm", "run", "hook:afteredit"],
            ["uv", "run", "hook-afteredit"]
        ])
    );
    assert_eq!(plan["entries"][1]["on_success"], "output");
    assert_eq!(plan["entries"][1]["on_failure"], "stop");
}

#[test]
fn reports_malformed_json() {
    let output = run_latch(Some("{"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("parse latch.json"));
}

#[test]
fn an_empty_command_list_succeeds_silently() {
    let output = run_latch(Some(r#"{"commands":[]}"#));

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn named_run_inherits_success_output_and_overrides_failure_policy() {
    let output = run_with_args(
        Some(
            r#"{
            "defaults":{"on_success":"output","on_failure":"collect"},
            "hooks":{"turn_end":{"on_failure":"stop","commands":[
                ["sh","-c","touch ready; printf success; printf note >&2"],
                ["sh","-c","test -f ready || exit 99; printf first; exit 7"],
                ["sh","-c","printf unexpected; exit 8"]
            ]}}
        }"#,
        ),
        &["run", "turn_end"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"successfirst");
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostics.contains("note") && diagnostics.contains("exit status: 7"));
    assert!(!diagnostics.contains("exit status: 8"));
}

#[cfg(unix)]
#[test]
fn configured_hook_defaults_to_collect_and_silent_success() {
    let output = run_latch(Some(
        r#"{"hooks":{"turn_end":{"commands":[
        ["sh","-c","printf hidden; touch ready"],
        ["sh","-c","test -f ready || exit 99; printf first; exit 7"],
        ["sh","-c","printf second; exit 8"]
    ]}}}"#,
    ));

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"firstsecond");
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostics.contains("exit status: 7") && diagnostics.contains("exit status: 8"));
}

#[test]
fn rejects_misspelled_settings_and_hooks_before_starting_commands() {
    for (config, field) in [
        (
            r#"{"hooks":{"turn_end":{"on_failur":"stop","commands":[["./must-not-run"]]}}}"#,
            "on_failur",
        ),
        (
            r#"{"hooks":{"turn_edn":{"commands":[["./must-not-run"]]}}}"#,
            "turn_edn",
        ),
        (r#"{"commands":[],"hooks":{}}"#, "commands"),
    ] {
        let output = run_latch(Some(config));
        assert_eq!(output.status.code(), Some(1));
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains(field), "{diagnostic}");
        assert!(!diagnostic.contains("must-not-run"));
    }
}

#[cfg(unix)]
#[test]
fn explicit_after_edit_runs_only_its_selected_batch() {
    let output = run_with_args(
        Some(
            r#"{"hooks":{
            "turn_end":{"commands":[["sh","-c","exit 99"]]},
            "after_edit":{"when":"workspace_changed","extensions":["ts"],"on_success":"output","commands":[["sh","-c","printf edit-only"]]}
        }}"#,
        ),
        &["run", "after_edit"],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"edit-only");
}

#[test]
fn explain_shows_effective_config_and_command_order_without_running() {
    let output = run_with_args(
        Some(
            r#"{"defaults":{"on_failure":"stop"},"hooks":{"after_edit":{
            "on_success":"output","paths":["src/**"],"extensions":[".TS"],"commands":[["./must-not-run","two words"],["./second"]]
        }}}"#,
        ),
        &["explain", "after_edit"],
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let explanation: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        explanation["config"]
            .as_str()
            .unwrap()
            .ends_with("latch.json")
    );
    assert_eq!(explanation["hook"], "after_edit");
    assert_eq!(explanation["when"], "workspace_changed");
    assert_eq!(explanation["comparison"], "before_edit");
    assert_eq!(explanation["paths"], serde_json::json!(["src/**"]));
    assert_eq!(explanation["extensions"], serde_json::json!(["ts"]));
    assert_eq!(explanation["on_failure"], "stop");
    assert_eq!(explanation["on_success"], "output");
    assert_eq!(
        explanation["commands"],
        serde_json::json!([["./must-not-run", "two words"], ["./second"]])
    );
}

#[test]
fn explain_resolves_each_array_entry_in_order() {
    let output = run_with_args(
        Some(
            r#"{"defaults":{"on_failure":"stop"},"hooks":{"after_edit":[
            {"when":"always","commands":[["./first"]]},
            {"on_failure":"collect","extensions":[".TS"],"commands":[["./second"]]}
        ]}}"#,
        ),
        &["explain", "after_edit"],
    );
    assert!(output.status.success(), "{output:?}");
    let explanation: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = explanation["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["commands"], serde_json::json!([["./first"]]));
    assert!(entries[0]["comparison"].is_null());
    assert_eq!(entries[0]["on_failure"], "stop");
    assert_eq!(entries[1]["commands"], serde_json::json!([["./second"]]));
    assert_eq!(entries[1]["comparison"], "before_edit");
    assert_eq!(entries[1]["on_failure"], "collect");
    assert_eq!(entries[1]["extensions"], serde_json::json!(["ts"]));
}

#[test]
fn rejects_array_entries_that_are_not_objects() {
    let output = run_latch(Some(
        r#"{"hooks":{"turn_end":[[[["./must-not-run"]],null,null,null,null,null]]}}"#,
    ));
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("could not parse latch.json"), "{error}");
    assert!(!error.contains("must-not-run"), "{error}");
}

#[test]
fn invalid_path_filters_are_rejected_before_running_any_commands() {
    for (field, value) in [
        ("paths", r#"["src/["]"#),
        ("extensions", r#"["*.ts"]"#),
        ("paths", "[]"),
    ] {
        let config = format!(
            r#"{{"hooks":{{"turn_end":{{"commands":[["./must-not-run"]],"{field}":{value}}}}}}}"#
        );
        let output = run_latch(Some(&config));
        assert_eq!(output.status.code(), Some(1));
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostic.contains(&format!("hooks.turn_end.{field}")),
            "{diagnostic}"
        );
        assert!(!diagnostic.contains("must-not-run"));
    }
}
