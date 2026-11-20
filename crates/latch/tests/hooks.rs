//! Verifies harness hook decisions across real Git and Latch processes.

#![cfg(unix)]

use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

struct Project {
    root: tempfile::TempDir,
    harness_home: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let project = Self {
            root: tempfile::tempdir().expect("create project"),
            harness_home: tempfile::tempdir().expect("create harness home"),
        };
        project.write(
            "latch.json",
            r#"{"commands":[["sh","-c","touch checked"]]}"#,
        );
        project
    }

    fn write(&self, path: &str, contents: &str) {
        fs::write(self.root.path().join(path), contents).expect("write project file");
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(self.root.path())
            .args([
                "-c",
                "user.name=Latch Test",
                "-c",
                "user.email=latch@example.test",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("run Git");
        assert!(output.status.success(), "{output:?}");
    }

    fn init(&self) {
        self.git(&["init"]);
        self.write("tracked", "committed");
        self.write(".gitignore", "checked\nignored\n");
        self.git(&["add", "."]);
        self.git(&["commit", "-m", "initial"]);
    }

    fn hook(&self, event: &str, turn: &str, retry: bool) -> Value {
        self.hook_request("codex", event, Some(turn), retry)
    }

    fn claude_code_hook(&self, event: &str, retry: bool) -> Value {
        self.hook_request("claude-code", event, None, retry)
    }

    fn hook_request(&self, adapter: &str, event: &str, turn: Option<&str>, retry: bool) -> Value {
        let payload = json!({
            "hook_event_name": event,
            "session_id": "session",
            "cwd": self.root.path(),
            "stop_hook_active": retry,
        });
        self.send_hook(adapter, turn, payload)
    }

    fn send_hook(&self, adapter: &str, turn: Option<&str>, mut payload: Value) -> Value {
        if let Some(turn) = turn {
            payload["turn_id"] = json!(turn);
        }
        let output = self.invoke_hook(&[adapter], payload);
        assert!(output.status.success(), "{output:?}");
        serde_json::from_slice(&output.stdout).expect("hook stdout is JSON")
    }

    fn copilot(&self, event: &str, mut fields: Value) -> Output {
        fields["sessionId"] = json!("session");
        fields["cwd"] = json!(self.root.path());
        self.invoke_hook(&["copilot", event], fields)
    }

    fn invoke_hook(&self, args: &[&str], payload: Value) -> Output {
        let path = env::var_os("PATH").unwrap_or_default();
        let path = env::join_paths(
            std::iter::once(self.harness_home.path().to_path_buf()).chain(env::split_paths(&path)),
        )
        .expect("test executable search path");
        let mut child = Command::new(env!("CARGO_BIN_EXE_latch"))
            .arg("hook")
            .args(args)
            .env("CODEX_HOME", self.harness_home.path())
            .env("CLAUDE_CONFIG_DIR", self.harness_home.path())
            .env("COPILOT_HOME", self.harness_home.path())
            .env("PI_CODING_AGENT_DIR", self.harness_home.path())
            .env("XDG_STATE_HOME", self.harness_home.path())
            .env("PATH", path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start Latch hook");
        writeln!(child.stdin.take().expect("hook stdin"), "{payload}").expect("send hook input");
        child.wait_with_output().expect("collect hook output")
    }

    fn checked(&self) -> bool {
        self.root.path().join(Path::new("checked")).exists()
    }
}

fn linked_worktree(project: &Project, path: &Path) {
    project.git(&[
        "worktree",
        "add",
        "--detach",
        path.to_str().unwrap(),
        "HEAD",
    ]);
}

fn worktree_hook(project: &Project, directory: &Path, event: &str) -> Value {
    project.send_hook(
        "codex",
        Some("shared-turn"),
        json!({
            "hook_event_name": event,
            "session_id": "shared-session",
            "cwd": directory,
            "tool_name": "apply_patch",
            "tool_use_id": "shared-edit",
            "stop_hook_active": false,
        }),
    )
}

#[test]
fn main_checkout_skips_changes_inside_an_unignored_nested_worktree() {
    let project = Project::new();
    project.write(
        "latch.json",
        r#"{"hooks":{
            "after_edit":{"commands":[["sh","-c","echo edit >> checked"]]},
            "turn_end":{"commands":[["sh","-c","echo turn >> checked"]]}
        }}"#,
    );
    project.init();
    let root = project.root.path();
    let nested = root.join("nested linked checkout");
    linked_worktree(&project, &nested);

    worktree_hook(&project, root, "UserPromptSubmit");
    worktree_hook(&project, root, "PreToolUse");
    fs::write(
        nested.join("tracked"),
        "changed only in the nested worktree",
    )
    .unwrap();
    worktree_hook(&project, root, "PostToolUse");
    worktree_hook(&project, root, "Stop");
    assert!(
        !project.checked(),
        "nested edits triggered the main checkout"
    );

    worktree_hook(&project, root, "UserPromptSubmit");
    worktree_hook(&project, root, "PreToolUse");
    project.write("tracked", "changed in the main checkout");
    worktree_hook(&project, root, "PostToolUse");
    worktree_hook(&project, root, "Stop");
    assert_eq!(
        fs::read_to_string(root.join("checked")).unwrap(),
        "edit\nturn\n"
    );
}

#[test]
fn linked_worktrees_use_their_own_config_and_baselines() {
    let project = Project::new();
    project.init();
    let root = project.root.path();
    let nested = root.join("nested linked checkout");
    let adjacent = tempfile::Builder::new()
        .prefix("latch adjacent checkout ")
        .tempdir_in(root.parent().unwrap())
        .unwrap();
    for (directory, label) in [(nested.as_path(), "nested"), (adjacent.path(), "adjacent")] {
        linked_worktree(&project, directory);
        fs::write(
            directory.join("latch.json"),
            json!({
                "defaults": {"on_success": "output"},
                "hooks": {
                    "after_edit": {"paths": ["tracked"], "commands": [["sh", "-c", "echo edit >> checked"]]},
                    "turn_end": {"paths": ["tracked"], "commands": [["sh", "-c", format!("echo turn >> checked; echo {label}")]]},
                }
            })
            .to_string(),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_latch"))
            .args(["run", "turn_end"])
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, format!("{label}\n").as_bytes());
        fs::remove_file(directory.join("checked")).unwrap();
        fs::write(directory.join("tracked"), "already dirty").unwrap();
    }

    // The same session, turn, and tool call IDs must not overwrite another worktree's state.
    let nested = nested.as_path();
    let adjacent = adjacent.path();
    for (changed, unchanged) in [(nested, adjacent), (adjacent, nested)] {
        for directory in [nested, adjacent] {
            worktree_hook(&project, directory, "UserPromptSubmit");
            worktree_hook(&project, directory, "PreToolUse");
        }
        fs::write(changed.join("tracked"), "edited an already-dirty file").unwrap();
        for directory in [nested, adjacent] {
            worktree_hook(&project, directory, "PostToolUse");
            worktree_hook(&project, directory, "Stop");
        }
        assert_eq!(
            fs::read_to_string(changed.join("checked")).unwrap(),
            "edit\nturn\n"
        );
        assert!(
            !unchanged.join("checked").exists(),
            "no-op triggered {}",
            unchanged.display()
        );
        fs::remove_file(changed.join("checked")).unwrap();
    }
    assert!(
        !project.checked(),
        "a linked worktree ran commands in the main checkout"
    );
}

#[test]
fn extensions_skip_no_ops_and_surface_changed_edit_and_turn_failures() {
    for adapter in ["opencode", "pi"] {
        let project = Project::new();
        project.write(
            "latch.json",
            r#"{"hooks":{
                "after_edit":{"commands":[["sh","-c","echo edit >&2; exit 1"]]},
                "turn_end":{"commands":[["sh","-c","echo turn >> checked; echo turn >&2; exit 1"]]}
            }}"#,
        );
        project.init();
        project.write("tracked", "already dirty");
        let hook = |event: &str, turn: &str| {
            let payload = json!({
                "hook_event_name": event, "session_id": "session", "cwd": project.root.path(),
                "turn_id": turn, "tool_use_id": "edit-call", "tool_name": "edit",
                "stop_hook_active": false,
            });
            let result = project.invoke_hook(&[adapter], payload);
            assert!(result.status.success(), "{result:?}");
            String::from_utf8(result.stdout).unwrap()
        };

        assert_eq!(hook("UserPromptSubmit", "first"), "");
        hook("PreToolUse", "first");
        assert_eq!(hook("PostToolUse", "first"), "");
        assert_eq!(hook("Stop", "first"), "");

        hook("UserPromptSubmit", "second");
        hook("PreToolUse", "second");
        project.write("tracked", "dirty with a new edit");
        assert!(hook("PostToolUse", "second").contains("Latch checks failed."));
        assert!(hook("Stop", "second").contains("Latch checks failed."));
        assert_eq!(
            fs::read_to_string(project.root.path().join("checked")).unwrap(),
            "turn\n"
        );

        let next = hook("UserPromptSubmit", "third");
        if adapter == "opencode" {
            assert!(next.contains("failed checks at the end of the previous turn"));
        } else {
            assert!(next.is_empty()); // Pi persists the turn-end report in its own history.
        }
        assert!(hook("UserPromptSubmit", "fourth").is_empty());
    }
}

#[test]
fn rejects_invalid_hook_events_without_blocking_copilot_edits() {
    let project = Project::new();
    let codex = project.invoke_hook(
        &["codex"],
        json!({
            "cwd": project.root.path(), "session_id": "session",
            "hook_event_name": "UserPromptSubmit"
        }),
    );
    assert_eq!(codex.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&codex.stderr).contains("turn ID for Codex"));
    let copilot = project.copilot("preToolUse", json!({}));
    assert!(copilot.status.success());
    assert!(copilot.stdout.is_empty());
    assert!(String::from_utf8_lossy(&copilot.stderr).contains("missing toolName"));
    assert!(!project.checked());
}

#[test]
fn compact_failure_output_preserves_late_diagnostics_and_other_failures() {
    let project = Project::new();
    project.write(
        "latch.json",
        r#"{"defaults":{"failure_output":"compact"},"hooks":{"turn_end":[
            {"when":"always","commands":[
                ["sh","-c","yes 'passing test' | head -500; printf '%s\\n' 'test parse_input ... FAILED' 'src/lib.rs:85: assertion failed' 'test result: FAILED. 1 failed'; exit 7"],
                ["sh","-c","echo 'another failure' >&2; exit 8"]
            ]},
            {"when":"always","failure_output":"excerpt","commands":[
                ["sh","-c","yes 'unrecognized log' | head -500; echo 'outside excerpt'; exit 9"]
            ]}
        ]}}"#,
    );
    let result = project.hook("Stop", "1", false);
    let report = result["systemMessage"]
        .as_str()
        .expect("failed check report");
    assert!(report.contains("test parse_input ... FAILED"), "{report}");
    assert!(
        report.contains("src/lib.rs:85: assertion failed"),
        "{report}"
    );
    assert!(report.contains("test result: FAILED. 1 failed"), "{report}");
    assert!(report.contains("another failure"), "{report}");
    assert!(!report.contains("passing test"), "{report}");
    assert!(report.contains("unrecognized log"), "{report}");
    assert!(!report.contains("outside excerpt"), "{report}");
}

#[test]
fn skips_preexisting_dirty_files_and_ignored_output() {
    let project = Project::new();
    project.init();
    project.write("tracked", "already dirty");
    project.write("untracked", "already here");
    assert_eq!(project.hook("UserPromptSubmit", "1", false), json!({}));
    project.write("ignored", "new generated output");
    assert_eq!(project.hook("Stop", "1", false), json!({}));
    assert!(!project.checked());
}

#[test]
fn detects_further_edits_to_dirty_tracked_and_untracked_files() {
    for file in ["tracked", "untracked"] {
        let project = Project::new();
        project.init();
        project.write(file, "baseline");
        project.hook("UserPromptSubmit", "1", false);
        project.write(file, "modified");
        assert_eq!(project.hook("Stop", "1", false), json!({}));
        assert!(project.checked(), "change to {file} was missed");
    }
}

#[test]
fn checks_after_an_edit_is_committed_during_the_turn() {
    let project = Project::new();
    project.init();
    project.hook("UserPromptSubmit", "1", false);
    project.write("tracked", "edited and committed");
    project.git(&["commit", "-am", "edit"]);
    project.hook("Stop", "1", false);
    assert!(project.checked());
}

#[test]
fn checks_when_baseline_is_missing_or_does_not_match_the_turn() {
    let project = Project::new();
    project.init();
    project.hook("UserPromptSubmit", "old", false);
    project.hook("Stop", "new", false);
    assert!(project.checked());
}

#[test]
fn checks_in_a_non_git_project() {
    let project = Project::new();
    project.hook("UserPromptSubmit", "1", false);
    project.hook("Stop", "1", false);
    assert!(project.checked());
}

#[test]
fn both_harnesses_report_failure_once_without_continuing() {
    for (adapter, turn) in [("codex", Some("1")), ("claude-code", None)] {
        let project = Project::new();
        project.init();
        project.write(
            "latch.json",
            r#"{"commands":[["sh","-c","echo ran >> ignored; echo stdout; echo stderr >&2; exit 7"]]}"#,
        );
        project.hook_request(adapter, "UserPromptSubmit", turn, false);
        project.write("tracked", "changed");

        let response = project.hook_request(adapter, "Stop", turn, false);
        let message = response["systemMessage"].as_str().expect("failure message");
        assert!(
            message.contains("stdout") && message.contains("stderr"),
            "{message}"
        );
        assert!(message.contains("exit status: 7"));
        assert_eq!(response.as_object().expect("hook response").len(), 1);

        // Another hook can continue the agent. Latch must still only run once.
        assert_eq!(project.hook_request(adapter, "Stop", turn, true), json!({}));
        assert_eq!(
            fs::read_to_string(project.root.path().join("ignored")).unwrap(),
            "ran\n"
        );
    }
}

#[test]
fn hooks_skip_projects_without_config() {
    let project = Project::new();
    fs::remove_file(project.root.path().join("latch.json")).expect("remove fixture config");
    assert_eq!(project.hook("Stop", "1", false), json!({}));
}

#[test]
fn zero_config_hooks_run_native_scripts_only_after_changes() {
    let project = Project::new();
    fs::remove_file(project.root.path().join("latch.json")).unwrap();
    project.write(
        "package.json",
        r#"{"scripts":{"hook:afteredit":"echo edit","hook:turnend":"echo turn"}}"#,
    );
    project.init();
    let npm = project.harness_home.path().join("npm");
    fs::write(
        &npm,
        "#!/bin/sh\ntest \"$1\" = run || exit 9\nprintf '%s\\n' \"$2\" >> ignored\n",
    )
    .unwrap();
    fs::set_permissions(&npm, fs::Permissions::from_mode(0o755)).unwrap();
    let edit = |event, turn| {
        project.send_hook(
            "codex",
            Some(turn),
            json!({
                "hook_event_name":event, "session_id":"session", "cwd":project.root.path(),
                "tool_name":"apply_patch", "tool_use_id":"edit",
            }),
        )
    };
    let log = project.root.path().join("ignored");
    project.hook("UserPromptSubmit", "unchanged", false);
    assert_eq!(edit("PreToolUse", "unchanged"), json!({}));
    assert_eq!(edit("PostToolUse", "unchanged"), json!({}));
    assert_eq!(project.hook("Stop", "unchanged", false), json!({}));
    assert!(!log.exists());

    project.hook("UserPromptSubmit", "changed", false);
    assert_eq!(edit("PreToolUse", "changed"), json!({}));
    project.write("tracked", "new content");
    assert_eq!(edit("PostToolUse", "changed"), json!({}));
    assert_eq!(project.hook("Stop", "changed", false), json!({}));
    assert_eq!(
        fs::read_to_string(log).unwrap(),
        "hook:afteredit\nhook:turnend\n"
    );
}

#[test]
fn a_reverted_edit_leaves_the_workspace_unchanged() {
    let project = Project::new();
    project.init();
    project.hook("UserPromptSubmit", "1", false);
    project.write("tracked", "temporary edit");
    project.write("tracked", "committed");
    assert_eq!(project.hook("Stop", "1", false), json!({}));
    assert!(!project.checked());
}

#[test]
fn checks_after_a_tracked_file_is_deleted() {
    let project = Project::new();
    project.init();
    project.hook("UserPromptSubmit", "1", false);
    fs::remove_file(project.root.path().join("tracked")).expect("delete tracked file");
    project.hook("Stop", "1", false);
    assert!(project.checked());
}

#[test]
fn interrupt_discards_the_baseline_for_that_turn() {
    let project = Project::new();
    project.init();
    project.hook("UserPromptSubmit", "1", false);
    assert_eq!(project.hook("Interrupt", "1", false), json!({}));
    let snapshots = project.harness_home.path().join("latch/baselines-v1");
    assert_eq!(
        fs::read_dir(snapshots)
            .expect("read baseline directory")
            .count(),
        0
    );
}

#[test]
fn claude_code_pairs_prompt_and_stop_without_a_turn_id() {
    let project = Project::new();
    project.init();
    assert_eq!(
        project.claude_code_hook("UserPromptSubmit", false),
        json!({})
    );
    assert_eq!(project.claude_code_hook("Stop", false), json!({}));
    assert!(!project.checked());

    project.claude_code_hook("UserPromptSubmit", false);
    project.write("tracked", "changed in a new turn");
    assert_eq!(project.claude_code_hook("Stop", false), json!({}));
    assert!(project.checked());
}

#[test]
fn claude_code_discards_the_baseline_when_the_session_ends_or_fails() {
    let project = Project::new();
    project.init();

    for event in ["SessionEnd", "StopFailure"] {
        project.claude_code_hook("UserPromptSubmit", false);
        let snapshots = project.harness_home.path().join("latch/baselines-v1");
        assert_eq!(
            fs::read_dir(&snapshots)
                .expect("read baseline directory")
                .count(),
            1
        );
        assert_eq!(project.claude_code_hook(event, false), json!({}));
        assert_eq!(
            fs::read_dir(snapshots)
                .expect("read baseline directory")
                .count(),
            0
        );
    }
}

#[test]
fn does_not_skip_validation_when_git_hides_worktree_edits() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let project = Project::new();
        project.init();
        project.git(&["update-index", flag, "tracked"]);
        project.hook("UserPromptSubmit", "1", false);
        project.write("tracked", "hidden change");
        project.hook("Stop", "1", false);
        assert!(project.checked(), "missed a change hidden by {flag}");
    }
}

#[test]
fn staging_existing_dirty_content_does_not_count_as_a_worktree_edit() {
    let project = Project::new();
    project.init();
    project.write("tracked", "already dirty");
    project.hook("UserPromptSubmit", "1", false);
    project.git(&["add", "tracked"]);
    project.hook("Stop", "1", false);
    assert!(!project.checked());
}

#[test]
fn notices_an_untracked_file_added_with_a_line_break_in_its_name() {
    let project = Project::new();
    project.init();
    project.hook("UserPromptSubmit", "1", false);
    project.write("new\nfile", "added");
    project.hook("Stop", "1", false);
    assert!(project.checked());
}

#[test]
fn checks_in_git_repositories_before_the_first_commit() {
    let project = Project::new();
    project.git(&["init"]);
    project.hook("UserPromptSubmit", "1", false);
    project.hook("Stop", "1", false);
    assert!(project.checked());
}

#[test]
fn hook_can_inherit_always_and_override_the_git_gate() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{
        "defaults":{"when":"always","on_success":"output"},
        "hooks":{"turn_end":{"commands":[["sh","-c","touch checked; echo visible"]]}}
    }"#,
    );
    project.hook("UserPromptSubmit", "1", false);
    let response = project.hook("Stop", "1", false);
    assert!(
        response["systemMessage"]
            .as_str()
            .expect("success message")
            .contains("visible")
    );
    fs::remove_file(project.root.path().join("checked")).expect("clear execution marker");

    project.write(
        "latch.json",
        r#"{
        "defaults":{"when":"always"},
        "hooks":{"turn_end":{"when":"workspace_changed","commands":[["sh","-c","touch checked"]]}}
    }"#,
    );
    project.hook("UserPromptSubmit", "2", false);
    assert_eq!(project.hook("Stop", "2", false), json!({}));
    assert!(!project.checked());
}

#[test]
fn collect_keeps_later_failures_in_the_hook_report_after_a_long_first_failure() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{
        "hooks":{"turn_end":{"when":"always","commands":[
            ["sh","-c","printf '%5000s' x; exit 7"],
            ["sh","-c","echo later-failure; exit 8"]
        ]}}
    }"#,
    );
    let response = project.hook("Stop", "1", false);
    let report = response["systemMessage"].as_str().expect("failure report");
    assert!(report.contains("[output truncated]"));
    assert!(report.contains("later-failure"));
    assert!(report.find("exit status: 7").unwrap() < report.find("exit status: 8").unwrap());
    assert_eq!(response.as_object().expect("hook response").len(), 1);
}

#[test]
fn a_turn_end_failure_is_delivered_once_to_the_next_prompt() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{"hooks":{"turn_end":{"commands":[
        ["sh","-c","echo ran >> ignored; echo next-prompt-failure; exit 7"]
    ]}}}"#,
    );
    project.hook("UserPromptSubmit", "1", false);
    project.write("tracked", "changed");
    let report = project.hook("Stop", "1", false);
    assert!(
        report["systemMessage"]
            .as_str()
            .unwrap()
            .contains("next-prompt-failure")
    );
    let next = project.hook("UserPromptSubmit", "2", false);
    let context = &next["hookSpecificOutput"];
    assert_eq!(context["hookEventName"], "UserPromptSubmit");
    assert!(
        context["additionalContext"]
            .as_str()
            .unwrap()
            .contains("next-prompt-failure")
    );
    assert_eq!(project.hook("Stop", "2", false), json!({}));
    assert_eq!(project.hook("UserPromptSubmit", "3", false), json!({}));
    assert_eq!(
        fs::read_to_string(project.root.path().join("ignored")).unwrap(),
        "ran\n"
    );
}

#[test]
fn opt_in_after_edit_runs_for_native_edits_and_only_delivers_context_in_the_current_turn() {
    for (adapter, turn, edit_tool) in [
        ("codex", Some("1"), "apply_patch"),
        ("claude-code", None, "Edit"),
    ] {
        let project = Project::new();
        project.init();
        project.write(
            "latch.json",
            r#"{"hooks":{"after_edit":{"commands":[
            ["sh","-c","echo ran >> ignored; echo edit-failure; exit 8"]
        ]}}}"#,
        );
        let post_tool = |tool| {
            project.send_hook(adapter, turn, json!({
            "hook_event_name":"PostToolUse", "session_id":"session", "cwd":project.root.path(), "tool_name":tool
        }))
        };
        project.hook_request(adapter, "UserPromptSubmit", turn, false);
        assert_eq!(post_tool("Read"), json!({}));
        project.write("tracked", "changed by the edit");
        let response = post_tool(edit_tool);
        let context = &response["hookSpecificOutput"];
        assert_eq!(context["hookEventName"], "PostToolUse");
        assert!(
            context["additionalContext"]
                .as_str()
                .unwrap()
                .contains("edit-failure")
        );
        assert_eq!(response.as_object().unwrap().len(), 1);
        assert_eq!(
            project.hook_request(adapter, "Stop", turn, false),
            json!({})
        );
        assert_eq!(
            project.hook_request(adapter, "UserPromptSubmit", turn.map(|_| "2"), false),
            json!({})
        );
        assert_eq!(
            fs::read_to_string(project.root.path().join("ignored")).unwrap(),
            "ran\n"
        );
    }
}

#[test]
fn copilot_reports_a_failed_stop_without_continuation_and_delivers_it_once() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{"hooks":{"turn_end":{"commands":[
        ["sh","-c","echo checked >> ignored; echo copilot-turn-failure; exit 7"]
    ]}}}"#,
    );

    assert!(
        project
            .copilot("userPromptSubmitted", json!({}))
            .status
            .success()
    );
    let unchanged = project.copilot("agentStop", json!({"stop_hook_active":false}));
    assert!(unchanged.status.success(), "{unchanged:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&unchanged.stdout).unwrap(),
        json!({})
    );

    assert!(
        project
            .copilot("userPromptSubmitted", json!({}))
            .status
            .success()
    );
    project.write("tracked", "changed");
    let failed = project.copilot("agentStop", json!({"stop_hook_active":false}));
    assert_eq!(failed.status.code(), Some(2));
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("copilot-turn-failure"));

    let original = "Original user prompt";
    assert!(
        project
            .copilot("userPromptSubmitted", json!({}))
            .status
            .success()
    );
    let prompt = project.copilot(
        "userPromptTransformed",
        json!({"transformedPrompt":original}),
    );
    let output: Value = serde_json::from_slice(&prompt.stdout).expect("prompt result");
    let transformed = output["modifiedTransformedPrompt"]
        .as_str()
        .expect("modified prompt");
    assert!(transformed.starts_with(original) && transformed.contains("copilot-turn-failure"));
    let later = project.copilot(
        "userPromptTransformed",
        json!({"transformedPrompt":original}),
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&later.stdout).unwrap(),
        json!({})
    );
    assert_eq!(
        fs::read_to_string(project.root.path().join("ignored")).unwrap(),
        "checked\n"
    );
}

#[test]
fn copilot_attaches_after_edit_feedback_to_the_successful_tool_result() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{"hooks":{"after_edit":{"commands":[
        ["sh","-c","echo copilot-edit-failure >&2; exit 8"]
    ]}}}"#,
    );
    let read = project.copilot("postToolUse", json!({"toolName":"view"}));
    assert_eq!(
        serde_json::from_slice::<Value>(&read.stdout).unwrap(),
        json!({})
    );
    let edit = project.copilot("postToolUse", json!({"toolName":"edit"}));
    assert!(edit.status.success(), "{edit:?}");
    let output: Value = serde_json::from_slice(&edit.stdout).expect("post-tool result");
    assert!(
        output["additionalContext"]
            .as_str()
            .unwrap()
            .contains("copilot-edit-failure")
    );
    assert_eq!(output.as_object().unwrap().len(), 1);
}

#[test]
fn after_edit_compares_each_call_against_its_own_dirty_baseline() {
    for (adapter, turn, tool) in [
        ("codex", Some("turn"), "apply_patch"),
        ("claude-code", None, "Write"),
        ("copilot", None, "edit"),
    ] {
        let project = Project::new();
        project.init();
        project.write("tracked", "already dirty");
        project.write(
            "latch.json",
            r#"{"hooks":{"after_edit":{"commands":[
            ["sh","-c","echo ran >> ignored"]
        ]}}}"#,
        );
        if adapter == "copilot" {
            assert!(
                project
                    .copilot("userPromptSubmitted", json!({}))
                    .status
                    .success()
            );
        } else {
            project.hook_request(adapter, "UserPromptSubmit", turn, false);
        }
        let call =
            |before, id, args| {
                if adapter == "copilot" {
                    let event = if before { "preToolUse" } else { "postToolUse" };
                    let output = project.copilot(event, json!({"toolName":tool,"toolArgs":args}));
                    assert!(output.status.success(), "{output:?}");
                } else {
                    let event = if before { "PreToolUse" } else { "PostToolUse" };
                    project.send_hook(adapter, turn, json!({
                    "hook_event_name":event, "session_id":"session", "cwd":project.root.path(),
                    "tool_name":tool, "tool_use_id":id,
                }));
                }
            };
        call(
            true,
            "first",
            json!({"path": "tracked", "content":"updated"}),
        );
        project.write("tracked", "updated");
        call(
            false,
            "first",
            json!({"path": "tracked", "content":"updated"}),
        );
        assert_eq!(
            fs::read_to_string(project.root.path().join("ignored")).unwrap(),
            "ran\n",
            "the first edit must trigger a check in {adapter}"
        );
        call(
            true,
            "second",
            json!({"path": "tracked", "content":"updated"}),
        );
        project.write("tracked", "updated");
        call(
            false,
            "second",
            json!({"path": "tracked", "content":"updated"}),
        );
        assert_eq!(
            fs::read_to_string(project.root.path().join("ignored")).unwrap(),
            "ran\n",
            "{adapter}"
        );
    }
}

#[test]
fn path_and_extension_filters_match_the_same_changed_file() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{"hooks":{"turn_end":{
        "paths":["src/**"],"extensions":["ts",".TSX"],"commands":[["sh","-c","echo ran >> ignored"]]
    }}}"#,
    );
    fs::create_dir(project.root.path().join("src")).unwrap();
    project.write("src/dirty.ts", "preexisting");

    project.hook("UserPromptSubmit", "1", false);
    project.write("src/README.md", "docs edit");
    project.write("outside.ts", "code outside src");
    project.hook("Stop", "1", false);
    assert!(!project.root.path().join("ignored").exists());

    project.hook("UserPromptSubmit", "2", false);
    project.write("src/dirty.ts", "updated");
    project.hook("Stop", "2", false);
    assert_eq!(
        fs::read_to_string(project.root.path().join("ignored")).unwrap(),
        "ran\n"
    );
}

#[test]
fn after_edit_filters_apply_to_the_edit_even_with_always() {
    let project = Project::new();
    project.init();
    project.write(
        "latch.json",
        r#"{"hooks":{"after_edit":{"when":"always","extensions":["ts"],
        "commands":[["sh","-c","echo ran >> ignored"]]
    }}}"#,
    );
    let edit = |event, id| {
        project.send_hook(
            "codex",
            Some("1"),
            json!({
                "hook_event_name":event, "session_id":"session","cwd":project.root.path(),
                "tool_name":"apply_patch","tool_use_id":id
            }),
        )
    };

    edit("PreToolUse", "docs");
    project.write("README.mdx", "new docs");
    edit("PostToolUse", "docs");
    assert!(!project.root.path().join("ignored").exists());

    edit("PreToolUse", "code");
    project.write("main.TS", "new code");
    edit("PostToolUse", "code");
    assert_eq!(
        fs::read_to_string(project.root.path().join("ignored")).unwrap(),
        "ran\n"
    );
}

#[test]
fn after_edit_runs_all_matching_entries_in_order() {
    for (adapter, turn, tool) in [
        ("codex", Some("turn"), "apply_patch"),
        ("claude-code", None, "Write"),
        ("copilot", None, "edit"),
    ] {
        let project = Project::new();
        project.init();
        project.write(
            "latch.json",
            r#"{"hooks":{"after_edit":[
                {"commands":[["sh","-c","echo lint >> ignored; touch generated.ts"]]},
                {"extensions":["md","mdx"],"commands":[["sh","-c","echo markdown >> ignored"]]},
                {"extensions":["ts"],"commands":[["sh","-c","echo typescript >> ignored"]]}
            ]}}"#,
        );
        let log = project.root.path().join("ignored");
        for (id, paths, expected) in [
            ("none", &[][..], ""),
            ("markdown", &["readme.md"][..], "lint\nmarkdown\n"),
            ("typescript", &["main.ts"][..], "lint\ntypescript\n"),
            (
                "both",
                &["readme.md", "main.ts"][..],
                "lint\nmarkdown\ntypescript\n",
            ),
            ("json", &["settings.json"][..], "lint\n"),
        ] {
            project.write("ignored", "");
            let call = |before| {
                if adapter == "copilot" {
                    let event = if before { "preToolUse" } else { "postToolUse" };
                    let output = project.copilot(event, json!({"toolName":tool,"toolArgs":id}));
                    assert!(output.status.success(), "{output:?}");
                    assert_eq!(
                        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                        json!({})
                    );
                } else {
                    let event = if before { "PreToolUse" } else { "PostToolUse" };
                    assert_eq!(project.send_hook(adapter, turn, json!({
                        "hook_event_name":event, "session_id":"session", "cwd":project.root.path(),
                        "tool_name":tool, "tool_use_id":id,
                    })), json!({}));
                }
            };
            call(true);
            for path in paths {
                project.write(path, id);
            }
            call(false);
            assert_eq!(
                fs::read_to_string(&log).unwrap(),
                expected,
                "{adapter}: {id}"
            );
        }
    }
}

#[test]
fn a_rename_out_of_a_filtered_directory_still_triggers_checks() {
    let project = Project::new();
    project.init();
    fs::create_dir(project.root.path().join("src")).unwrap();
    project.write("src/source.ts", "committed");
    project.git(&["add", "src/source.ts"]);
    project.git(&["commit", "-m", "add source"]);
    project.write(
        "latch.json",
        r#"{"hooks":{"turn_end":{"paths":["src/**"],"commands":[
        ["sh","-c","touch checked"]
    ]}}}"#,
    );

    project.hook("UserPromptSubmit", "1", false);
    project.git(&["mv", "src/source.ts", "moved.ts"]);
    project.hook("Stop", "1", false);
    assert!(project.checked());
}
