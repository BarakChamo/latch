//! Exercises failure previews and raw output retrieval across Latch processes.

#![cfg(unix)]

use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, SystemTime};

fn latch(project: &Path, state: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_latch"));
    command.current_dir(project).env("XDG_STATE_HOME", state);
    command.env("CODEX_HOME", project.join("codex-profile"));
    command
}

fn report(project: &Path, state: &Path, mode: &str, commands: Value) -> String {
    fs::write(
        project.join("latch.json"),
        json!({"hooks":{"turn_end":{
            "when":"always","failure_output":mode,"commands":commands
        }}})
        .to_string(),
    )
    .unwrap();
    hook_report(project, state)
}

fn hook_report(project: &Path, state: &Path) -> String {
    hook_report_with(latch(project, state), project)
}

fn hook_report_with(mut latch: Command, project: &Path) -> String {
    let mut child = latch
        .args(["hook", "codex"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "{}",
        json!({"cwd":project,"session_id":"session","turn_id":"turn",
            "hook_event_name":"Stop","stop_hook_active":false})
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["systemMessage"]
        .as_str()
        .expect("failed check report")
        .to_string()
}

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
fn rtk_formats_captured_output_once_and_preserves_the_command_result() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    let rtk = project.join("rtk");
    fs::write(&rtk, "#!/bin/sh\n[ \"$*\" = 'pipe --filter vitest' ] || exit 9\n/bin/cat > rtk-input\nprintf 'RTK selected a failure\\n'\n").unwrap();
    fs::set_permissions(&rtk, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        project.join("latch.json"),
        json!({
            "defaults":{"failure_output":"rtk","rtk_filter":"log"},
            "hooks":{"turn_end":{"when":"always","rtk_filter":"vitest","commands":[["/bin/sh","-c",
            "echo executed >> executions; echo raw-stdout; echo 'error: raw-stderr' >&2; exit 7"
            ]]}}
        })
        .to_string(),
    )
    .unwrap();
    let mut runner = latch(project, &state);
    runner.env("PATH", project);
    let preview = hook_report_with(runner, project);
    assert!(preview.contains("exit status: 7"), "{preview}");
    assert!(preview.contains("RTK selected a failure"), "{preview}");
    assert!(!preview.contains("raw-stdout"), "{preview}");
    assert!(preview.contains("error: raw-stderr"), "{preview}");
    assert_eq!(
        fs::read(project.join("rtk-input")).unwrap(),
        b"raw-stdout\nerror: raw-stderr\n"
    );
    let id = saved_id(&preview);
    let full = retrieve(project, &state, &[id]);
    assert!(full.status.success(), "{full:?}");
    assert_eq!(full.stdout, b"raw-stdout\n");
    assert_eq!(full.stderr, b"error: raw-stderr\n");
    assert_eq!(fs::read(project.join("executions")).unwrap(), b"executed\n");
    fs::remove_file(project.join("rtk-input")).unwrap();
    let direct = latch(project, &state)
        .env("PATH", project)
        .output()
        .unwrap();
    assert_eq!(direct.status.code(), Some(1));
    assert_eq!(direct.stdout, b"raw-stdout\n");
    assert!(String::from_utf8_lossy(&direct.stderr).contains("error: raw-stderr"));
    assert!(!project.join("rtk-input").exists());
}

#[test]
fn missing_rtk_uses_compact_output_and_still_saves_the_original() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    fs::write(
        project.join("latch.json"),
        json!({
            "defaults":{"failure_output":"rtk"},
            "hooks":{"turn_end":{"when":"always","commands":[["/bin/sh","-c",
                "echo 'error: original check failure' >&2; exit 7"
            ]]}}
        })
        .to_string(),
    )
    .unwrap();
    let mut runner = latch(project, &state);
    runner.env("PATH", project);
    let preview = hook_report_with(runner, project);
    assert!(preview.contains("RTK preview unavailable"), "{preview}");
    assert!(
        preview.contains("error: original check failure"),
        "{preview}"
    );
    let id = saved_id(&preview);
    let full = retrieve(project, &state, &[id]);
    assert!(full.status.success(), "{full:?}");
    assert_eq!(full.stderr, b"error: original check failure\n");
}

#[test]
fn rtk_rejection_empty_output_and_timeout_fall_back_to_the_real_failure() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    let rtk = project.join("rtk");
    fs::write(
        project.join("latch.json"),
        json!({
            "defaults":{"failure_output":"rtk"},
            "hooks":{"turn_end":{"when":"always","commands":[["/bin/sh","-c",
                "echo 'error: real failure' >&2; exit 8"
            ]]}}
        })
        .to_string(),
    )
    .unwrap();
    for (script, reason) in [
        (
            "echo 'vendor noise' >&2; exit 9",
            "rtk filter failed with exit status: 9",
        ),
        ("exit 0", "rtk returned an empty preview"),
        ("exec /bin/sleep 30", "rtk filter timed out"),
    ] {
        fs::write(&rtk, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&rtk, fs::Permissions::from_mode(0o755)).unwrap();
        let started = std::time::Instant::now();
        let mut runner = latch(project, &state);
        runner.env("PATH", project);
        let preview = hook_report_with(runner, project);
        assert!(started.elapsed() < Duration::from_secs(15));
        assert!(preview.contains(reason), "{preview}");
        assert!(preview.contains("exit status: 8"), "{preview}");
        assert!(preview.contains("error: real failure"), "{preview}");
        assert!(preview.contains("Saved log: "), "{preview}");
        assert!(!preview.contains("vendor noise"), "{preview}");
    }
}

fn retrieve(project: &Path, state: &Path, arguments: &[&str]) -> Output {
    latch(project, state)
        .arg("output")
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn tail_inherits_the_line_count_and_keeps_the_raw_streams_retrievable() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    fs::write(project.join("latch.json"), json!({
        "defaults": {"failure_output":"tail","tail_lines":2},
        "hooks": {"turn_end":{"when":"always","commands":[["sh","-c",
            "printf '%s\\n' out-one out-two out-three; printf '%s\\n' err-one err-two err-three >&2; exit 7"
        ]]}}
    }).to_string()).unwrap();
    let preview = hook_report(project, &state);
    assert!(
        preview.contains("stdout:\nout-two\nout-three\n"),
        "{preview}"
    );
    assert!(
        preview.contains("stderr:\nerr-two\nerr-three\n"),
        "{preview}"
    );
    assert!(!preview.contains("out-one"));
    assert!(!preview.contains("err-one"));
    let id = saved_id(&preview);
    let full = retrieve(project, &state, &[id]);
    assert!(full.status.success(), "{full:?}");
    assert_eq!(full.stdout, b"out-one\nout-two\nout-three\n");
    assert_eq!(full.stderr, b"err-one\nerr-two\nerr-three\n");
}

#[test]
fn tail_line_count_can_be_overridden_and_rejects_invalid_values() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    let explain = |tail_lines: Option<usize>| {
        let mut hook = json!({"failure_output":"tail","commands":[["true"]]});
        if let Some(lines) = tail_lines {
            hook["tail_lines"] = json!(lines);
        }
        fs::write(
            project.join("latch.json"),
            json!({"hooks":{"turn_end":hook}}).to_string(),
        )
        .unwrap();
        latch(project, &state).arg("explain").output().unwrap()
    };
    for (requested, expected) in [(None, 20), (Some(1), 1), (Some(100), 100)] {
        let result = explain(requested);
        assert!(result.status.success(), "{result:?}");
        let config: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(config["tail_lines"], expected);
    }
    for invalid in [0, 101] {
        let result = explain(Some(invalid));
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("tail_lines must be between 1 and 100")
        );
    }
}

#[test]
fn normal_invocations_and_explicit_prune_clear_expired_logs_from_other_projects() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path();
    let state = root.join("state");
    let cache = state.join("latch/output-v1");
    let idle_project = cache.join("a".repeat(64));
    fs::create_dir_all(&idle_project).unwrap();
    let expired = idle_project.join("r-0123456789abcdef01234567");
    let fresh = idle_project.join("r-0123456789abcdef01234568");
    let unrelated = idle_project.join("unrelated.txt");
    let abandoned = idle_project.join(".pending-0123456789abcdef01234567");
    let make_expired = |path: &Path| {
        File::create(path)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(8 * 86_400))
            .unwrap();
    };
    make_expired(&expired);
    make_expired(&unrelated);
    make_expired(&abandoned);
    File::create(&fresh).unwrap();
    let version = latch(root, &state).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(!expired.exists());
    assert!(!abandoned.exists());
    assert!(fresh.exists());
    assert!(unrelated.exists());

    make_expired(&expired);
    assert!(
        latch(root, &state)
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(expired.exists(), "the online sweep is throttled");
    let prune = retrieve(root, &state, &["--prune"]);
    assert!(prune.status.success(), "{prune:?}");
    assert_eq!(prune.stdout, b"Removed 1 expired output files.\n");
    assert!(!expired.exists());
    assert!(fresh.exists());
    assert!(unrelated.exists());

    File::options()
        .write(true)
        .open(cache.join(".last-prune"))
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(3_601))
        .unwrap();
    make_expired(&expired);
    assert!(
        latch(root, &state)
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !expired.exists(),
        "a normal invocation resumes sweeping after the interval"
    );
}

#[test]
fn retrieval_replays_original_streams_and_searches_omitted_lines_without_reruns() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    let other = workspace.path().join("other");
    let state = workspace.path().join("state");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&other).unwrap();
    let report = report(
        &project,
        &state,
        "compact",
        json!([[
            "sh",
            "-c",
            "echo ran >> executions; printf 'hidden clue\\n'; yes 'verbose' | head -500; printf 'error: assertion failed\\n' >&2; printf '\\377' >&2; exit 7"
        ]]),
    );
    assert!(report.contains("error: assertion failed"), "{report}");
    assert!(!report.contains("hidden clue"), "{report}");
    let id = saved_id(&report);
    let full = retrieve(&project, &state, &[id]);
    assert!(full.status.success(), "{full:?}");
    assert_eq!(
        full.stdout,
        format!("hidden clue\n{}", "verbose\n".repeat(500)).as_bytes()
    );
    assert_eq!(full.stderr, b"error: assertion failed\n\xff");
    let matches = retrieve(&project, &state, &[id, "--grep", "hidden clue"]);
    assert!(matches.status.success(), "{matches:?}");
    assert_eq!(matches.stdout, b"hidden clue\n");
    assert!(matches.stderr.is_empty());
    let missing = retrieve(&other, &state, &[id]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("not found"));
    assert_eq!(
        fs::read_to_string(project.join("executions")).unwrap(),
        "ran\n"
    );
}

#[test]
fn short_reports_skip_storage_and_truncated_excerpts_can_be_retrieved() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    let short = report(
        project,
        &state,
        "compact",
        json!([["sh", "-c", "echo short; exit 1"]]),
    );
    assert!(!short.contains("Saved log:"));
    assert!(!state.join("latch/output-v1").exists());
    let verbose = json!([[
        "sh",
        "-c",
        "yes 'verbose' | head -500; echo 'error: failed'; exit 7"
    ]]);
    let excerpt = report(project, &state, "excerpt", verbose.clone());
    assert!(excerpt.contains("[output truncated]"));
    let id = saved_id(&excerpt);
    assert!(excerpt.contains("search_output"));
    let restored = retrieve(project, &state, &[id, "--grep", "error: failed"]);
    assert!(restored.status.success(), "{restored:?}");
    assert_eq!(restored.stdout, b"error: failed\n");
    let blocked = project.join("blocked");
    fs::write(&blocked, "blocked state directory").unwrap();
    let fallback = report(project, &blocked, "compact", verbose);
    assert!(fallback.contains("error: failed"));
    assert!(
        fallback.contains("could not save full output"),
        "{fallback}"
    );
    assert!(!fallback.contains("Saved log:"));
}

#[test]
fn saved_log_can_be_read_directly_without_latch_on_path() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path();
    let state = project.join("state");
    let bundle = project.join("agent's plugin");
    fs::create_dir(&bundle).unwrap();
    let binary = bundle.join("latch");
    fs::copy(env!("CARGO_BIN_EXE_latch"), &binary).unwrap();
    fs::write(
        project.join("latch.json"),
        json!({
            "hooks":{"turn_end":{"when":"always","failure_output":"tail","tail_lines":1,
                "commands":[["/bin/sh","-c","echo omitted-marker; echo final-line; exit 1"]]}}
        })
        .to_string(),
    )
    .unwrap();
    let mut runner = Command::new(binary);
    runner
        .current_dir(project)
        .env("XDG_STATE_HOME", &state)
        .env("PATH", project);
    runner.env("CODEX_HOME", project.join("profile"));
    let report = hook_report_with(runner, project);
    let log_path = report
        .lines()
        .find_map(|line| line.strip_prefix("Saved log: "))
        .expect("report includes a log path");
    let log = fs::read_to_string(log_path).unwrap();
    assert!(log.contains("omitted-marker\nfinal-line\n"));
    assert!(!report.contains("omitted-marker"));
    assert!(report.contains("search_output"));
    assert!(!report.contains("on PATH"));
}

#[test]
fn retrieval_rejects_paths_and_empty_search_terms() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path();
    for (arguments, expected) in [
        (vec!["../latch.json"], "invalid output ID"),
        (
            vec!["r-0123456789abcdef01234567", "--grep", ""],
            "search term must not be empty",
        ),
    ] {
        let output = retrieve(root, root, &arguments);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{output:?}"
        );
    }
}
