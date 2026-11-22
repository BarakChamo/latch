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
fn single_command_forms_resolve_to_the_same_arguments() {
    let expected = serde_json::json!([["bun", "run", "lint", "two words", "", "$HOME", "&&"]]);
    for commands in [
        serde_json::json!("bun run lint \"two words\" '' '$HOME' &&"),
        expected.clone(),
    ] {
        for config in [
            serde_json::json!({"commands": commands}),
            serde_json::json!({"hooks": {"turn_end": {"commands": commands}}}),
        ] {
            let output = run_with_args(Some(&config.to_string()), &["explain", "turn_end"]);
            assert!(output.status.success(), "{output:?}");
            let plan: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("parse explanation");
            assert_eq!(plan["commands"], expected);
        }
    }
}

#[test]
fn rejects_flat_command_arrays() {
    for commands in [
        serde_json::json!(["make", "lint"]),
        serde_json::json!(["bun run test", "bun run lint"]),
    ] {
        let config = serde_json::json!({"hooks": {"turn_end": {"commands": commands}}});
        let output = run_latch(Some(&config.to_string()));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stderr.contains("a command string or an array of argument arrays"),
            "{stderr}"
        );
    }
}

#[test]
fn malformed_command_text_is_rejected_before_running_any_checks() {
    for (command, diagnostic) in [
        ("make 'lint", "unterminated quote or escape"),
        ("make lint\\", "unterminated quote or escape"),
        (" \t\n", "nonempty executable"),
        ("make lint\0", "without NUL bytes"),
    ] {
        let config = serde_json::json!({
            "hooks": {
                "turn_end": [
                    {"commands": [["./must-not-launch"]]},
                    {"commands": command}
                ]
            }
        });
        let output = run_latch(Some(&config.to_string()));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr.contains(diagnostic), "{stderr}");
        assert!(!stderr.contains("must-not-launch"), "{stderr}");
    }
}

#[cfg(unix)]
#[test]
fn command_string_runs_directly_without_expanding_shell_syntax() {
    let config = serde_json::json!({
        "defaults": {"on_success": "output"},
        "hooks": {"turn_end": {"commands": "printf '%s\\n' 'two words' $HOME |" }}
    });
    let output = run_latch(Some(&config.to_string()));
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("two words\n$HOME\n|\n"));
}

#[test]
fn help_succeeds_without_loading_project_configuration() {
    for flag in ["--help", "-h"] {
        let output = run_with_args(Some("invalid json"), &[flag]);
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: latch"));
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[test]
fn config_init_preserves_script_discovery_and_refuses_to_overwrite() {
    let project = tempfile::tempdir().expect("create temporary project");
    let config_path = project.path().join("latch.json");
    fs::write(
        project.path().join("package.json"),
        r#"{"scripts":{"hook:afteredit":"echo lint"}}"#,
    )
    .expect("write package.json");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_latch"))
            .current_dir(project.path())
            .args(args)
            .output()
            .expect("run latch")
    };

    let created = run(&["config", "init"]);
    assert!(created.status.success(), "{created:?}");
    let starter: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).expect("read starter configuration"))
            .expect("parse starter configuration");
    assert_eq!(starter["defaults"]["on_failure"], "collect");
    let explanation = run(&["explain", "after_edit"]);
    assert!(explanation.status.success(), "{explanation:?}");
    let resolved: serde_json::Value =
        serde_json::from_slice(&explanation.stdout).expect("parse explanation");
    assert_eq!(
        resolved["commands"],
        serde_json::json!([["npm", "run", "hook:afteredit"]])
    );

    let existing = br#"{"defaults":{"on_success":"output"}}"#;
    fs::write(&config_path, existing).expect("customize starter configuration");
    let repeated = run(&["config", "init"]);
    assert_eq!(repeated.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("latch.json already exists"));
    assert_eq!(
        fs::read(config_path).expect("read existing configuration"),
        existing
    );
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

#[cfg(unix)]
#[test]
fn openhands_setup_preserves_existing_hooks_and_is_idempotent() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let hooks = project.path().join(".openhands/hooks.json");
    fs::create_dir(hooks.parent().unwrap()).unwrap();
    fs::write(
        &hooks,
        r#"{"stop":[{"hooks":[{"command":"existing-check"}]}]}"#,
    )
    .unwrap();
    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_latch"))
            .current_dir(project.path())
            .env("OPENHANDS_PERSISTENCE_DIR", home.path())
            .args(["install", "openhands"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    let config: serde_json::Value = serde_json::from_slice(&fs::read(hooks).unwrap()).unwrap();
    assert_eq!(config["stop"][0]["hooks"][0]["command"], "existing-check");
    assert_eq!(config["stop"].as_array().unwrap().len(), 2);
    assert_eq!(config["pre_tool_use"][0]["matcher"], "file_editor");
    let mcp: serde_json::Value =
        serde_json::from_slice(&fs::read(home.path().join("mcp.json")).unwrap()).unwrap();
    assert_eq!(
        mcp["mcpServers"]["latch"]["args"],
        serde_json::json!(["mcp", "--require-project-directory"])
    );
}

#[cfg(unix)]
#[test]
fn cursor_devin_and_kiro_install_only_when_preview_is_selected() {
    let project = tempfile::tempdir().unwrap();
    for (host, path) in [
        ("cursor", ".cursor/hooks.json"),
        ("devin", ".devin/hooks.v1.json"),
        ("kiro", ".kiro/hooks/latch.json"),
    ] {
        let run = |options: &[&str]| {
            Command::new(env!("CARGO_BIN_EXE_latch"))
                .current_dir(project.path())
                .args(["install", host])
                .args(options)
                .output()
                .unwrap()
        };
        let hooks = project.path().join(path);
        assert_eq!(run(&[]).status.code(), Some(2));
        assert!(!hooks.exists());
        assert!(run(&["--preview"]).status.success());
        assert!(
            fs::read_to_string(hooks)
                .unwrap()
                .contains(&format!("hook {host}"))
        );
    }
}

#[test]
fn cloud_setup_merges_existing_hooks_and_leaves_reruns_unchanged() {
    use serde_json::{Value, json};

    for (host, path, event, nested) in [
        ("claude", ".claude/settings.json", "Stop", true),
        ("copilot", ".github/hooks/latch.json", "agentStop", true),
        ("openhands", ".openhands/hooks.json", "Stop", false),
    ] {
        // Given a project with its own settings and a hook for the same event.
        let project = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(project.path())
                .status()
                .unwrap()
                .success()
        );
        let settings = project.path().join(path);
        fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let existing = json!({"matcher":"custom","hooks":[{"type":"command","command":"keep me"}]});
        let mut initial = json!({"permissions":{"allow":["Read"]}});
        if nested {
            initial["hooks"] = json!({(event):[existing.clone()]});
        } else {
            initial[event] = json!([existing.clone()]);
        }
        fs::write(&settings, initial.to_string()).unwrap();

        let subdirectory = project.path().join("src");
        fs::create_dir(&subdirectory).unwrap();
        let install = || {
            Command::new(env!("CARGO_BIN_EXE_latch"))
                .args(["install", "cloud", host])
                .current_dir(&subdirectory)
                .output()
                .unwrap()
        };

        // When setup runs from inside the worktree, it adds Latch beside the existing hook.
        let output = install();
        assert!(output.status.success(), "{host}: {output:?}");
        let mut merged: Value = serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
        assert_eq!(merged["permissions"], initial["permissions"]);
        let hooks = if nested {
            &mut merged["hooks"]
        } else {
            &mut merged
        };
        assert_eq!(hooks[event][0], existing);
        assert_eq!(hooks[event].as_array().unwrap().len(), 2);

        // Then a rerun also preserves any customizations made to the Latch hook.
        if host == "copilot" {
            hooks[event][1]["timeoutSec"] = json!(75);
        } else {
            hooks[event][1]["hooks"][0]["timeout"] = json!(75);
        }
        let custom = merged.to_string();
        fs::write(&settings, &custom).unwrap();
        let output = install();
        assert!(output.status.success(), "{host}: {output:?}");
        assert_eq!(fs::read_to_string(settings).unwrap(), custom);
    }
}

#[test]
fn cloud_setup_leaves_invalid_settings_untouched() {
    let project = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(project.path())
            .status()
            .unwrap()
            .success()
    );
    fs::create_dir(project.path().join(".claude")).unwrap();
    let settings = project.path().join(".claude/settings.json");

    for (original, expected) in [
        ("{invalid", "settings.json"),
        (r#"{"hooks":{"Stop":false}}"#, "Stop must be a JSON array"),
        (
            r#"{"hooks":{"Stop":[{"hooks":[{"command":"sh .claude/latch.sh hook claude-code"}]}]}}"#,
            "already has a different Latch hook",
        ),
    ] {
        fs::write(&settings, original).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_latch"))
            .args(["install", "cloud", "claude"])
            .current_dir(project.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{output:?}"
        );
        assert_eq!(fs::read_to_string(&settings).unwrap(), original);
    }
}
