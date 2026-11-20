//! Runs shared workspace validation for host lifecycle hooks.

use crate::{
    config::{self, HookName},
    run,
    workspace::Baseline,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) enum Harness {
    Codex,
    ClaudeCode,
    Copilot(CopilotEvent),
}

impl Harness {
    fn read_event(&self, input: impl io::Read) -> io::Result<Event> {
        let event = match self {
            Self::Copilot(event) => event.read(input),
            _ => serde_json::from_reader::<_, Event>(input).map_err(io::Error::from),
        }
        .map_err(|error| io::Error::new(error.kind(), format!("invalid hook input: {error}")))?;
        if !event.cwd.is_absolute()
            || event.session_id.is_empty()
            || self.turn_key(&event).is_none()
        {
            return Err(io::Error::other(
                "hook requires a session ID, an absolute cwd, and a turn ID for Codex",
            ));
        }
        Ok(event)
    }

    fn home(&self) -> Option<PathBuf> {
        let (variable, default) = match self {
            Self::Codex => ("CODEX_HOME", ".codex"),
            Self::ClaudeCode => ("CLAUDE_CONFIG_DIR", ".claude"),
            Self::Copilot(_) => ("COPILOT_HOME", ".copilot"),
        };
        env::var_os(variable)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::home_dir().map(|home| home.join(default)))
    }

    fn is_edit_tool(&self, name: &str) -> bool {
        match self {
            Self::Codex => name == "apply_patch",
            Self::ClaudeCode => matches!(name, "Edit" | "Write" | "MultiEdit" | "NotebookEdit"),
            Self::Copilot(_) => matches!(
                name,
                "create" | "edit" | "str_replace_editor" | "apply_patch"
            ),
        }
    }

    fn turn_key<'a>(&self, event: &'a Event) -> Option<&'a str> {
        match self {
            Self::Codex => event.turn_id.as_deref().filter(|id| !id.is_empty()),
            // Claude Code and Copilot pair foreground events by session ID.
            Self::ClaudeCode | Self::Copilot(_) => Some(""),
        }
    }
}

pub(crate) enum CopilotEvent {
    PromptSubmitted,
    PromptTransformed,
    PreToolUse,
    PostToolUse,
    Stop,
    SessionEnd,
}

impl CopilotEvent {
    pub(crate) fn parse(name: &std::ffi::OsStr) -> Option<Self> {
        match name.to_str()? {
            "userPromptSubmitted" => Some(Self::PromptSubmitted),
            "userPromptTransformed" => Some(Self::PromptTransformed),
            "preToolUse" => Some(Self::PreToolUse),
            "postToolUse" => Some(Self::PostToolUse),
            "agentStop" => Some(Self::Stop),
            "sessionEnd" => Some(Self::SessionEnd),
            _ => None,
        }
    }

    fn read(&self, input: impl io::Read) -> io::Result<Event> {
        let payload: CopilotInput = serde_json::from_reader(input)?;
        let kind = match self {
            Self::PromptSubmitted => EventKind::UserPromptSubmit,
            Self::PromptTransformed => EventKind::PromptTransformed {
                prompt: payload
                    .transformed_prompt
                    .ok_or_else(|| io::Error::other("missing transformedPrompt"))?,
            },
            Self::PreToolUse | Self::PostToolUse => {
                let tool_name = payload
                    .tool_name
                    .ok_or_else(|| io::Error::other("missing toolName"))?;
                match self {
                    Self::PreToolUse => EventKind::PreToolUse { tool_name },
                    _ => EventKind::PostToolUse { tool_name },
                }
            }
            Self::Stop => EventKind::Stop {
                stop_hook_active: payload.stop_hook_active,
            },
            Self::SessionEnd => EventKind::SessionEnd,
        };
        Ok(Event {
            session_id: payload.session_id,
            cwd: payload.cwd,
            turn_id: None,
            agent_id: None,
            tool_use_id: payload.tool_use_id,
            tool_args: payload.tool_args,
            kind,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopilotInput {
    session_id: String,
    cwd: PathBuf,
    transformed_prompt: Option<String>,
    tool_name: Option<String>,
    tool_args: Option<Value>,
    #[serde(alias = "toolCallId")]
    tool_use_id: Option<String>,
    #[serde(default, alias = "stop_hook_active")]
    stop_hook_active: bool,
}

#[derive(Deserialize)]
struct Event {
    session_id: String,
    turn_id: Option<String>,
    cwd: PathBuf,
    agent_id: Option<String>,
    tool_use_id: Option<String>,
    tool_args: Option<Value>,
    #[serde(flatten)]
    kind: EventKind,
}

#[derive(Deserialize)]
#[serde(tag = "hook_event_name")]
enum EventKind {
    UserPromptSubmit,
    PromptTransformed { prompt: String },
    Stop { stop_hook_active: bool },
    PreToolUse { tool_name: String },
    PostToolUse { tool_name: String },
    Interrupt,
    SessionEnd,
    StopFailure,
}

pub(crate) fn dispatch(harness: Harness) -> ExitCode {
    let event = match harness.read_event(io::stdin().lock()) {
        Ok(event) => event,
        Err(error) => {
            eprintln!("latch: {error}");
            return invalid_input(&harness);
        }
    };
    let response = handle(&harness, event);
    if matches!(harness, Harness::Copilot(CopilotEvent::Stop))
        && let Some(message) = response.get("systemMessage").and_then(Value::as_str)
    {
        // Copilot renders stderr from a stop-hook exit 2 as a warning without continuing.
        eprintln!("{message}");
        return ExitCode::from(2);
    }
    match writeln!(io::stdout().lock(), "{response}") {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn invalid_input(harness: &Harness) -> ExitCode {
    if matches!(harness, Harness::Copilot(CopilotEvent::PreToolUse)) {
        // A Copilot pre-tool exit other than 0 would deny the edit.
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn handle(harness: &Harness, event: Event) -> Value {
    if event.agent_id.is_some() {
        return json!({});
    }
    let state = state_path(harness, &event);
    let edits = state.as_deref().map(|path| path.with_extension("edits"));
    match event.kind {
        EventKind::UserPromptSubmit => {
            clear_edit_baselines(edits.as_deref());
            if let Some(path) = state {
                // Invalidate any previous snapshot before trying to replace it.
                let _ = fs::remove_file(&path);
                if let Err(error) = capture(&event.cwd, &path) {
                    eprintln!("latch: could not save workspace baseline: {error}");
                }
            }
            // Copilot's submission hook captures the baseline. Its transformation hook
            // runs separately and is the hook that can deliver model-facing text.
            if matches!(harness, Harness::Copilot(_)) {
                return json!({});
            }
            match take_pending(harness, &event) {
                Some(report) => context_response("UserPromptSubmit", report),
                None => json!({}),
            }
        }
        EventKind::PromptTransformed { ref prompt } => match take_pending(harness, &event) {
            Some(report) => json!({ "modifiedTransformedPrompt": format!("{prompt}\n\n{report}") }),
            None => json!({}),
        },
        EventKind::PreToolUse { ref tool_name } if harness.is_edit_tool(tool_name) => {
            if let Some(path) = edits
                .as_deref()
                .and_then(|dir| edit_path(dir, &event, tool_name))
                && let Err(error) = capture_edit(&event.cwd, &path)
            {
                eprintln!("latch: could not save edit baseline: {error}");
            }
            json!({})
        }
        EventKind::PostToolUse { ref tool_name } if harness.is_edit_tool(tool_name) => {
            let baseline = edits
                .as_deref()
                .and_then(|dir| edit_path(dir, &event, tool_name))
                .and_then(|path| take::<Option<Baseline>>(&path))
                .flatten();
            match check_workspace(&event.cwd, HookName::AfterEdit, baseline) {
                Some(feedback) if matches!(harness, Harness::Copilot(_)) => {
                    json!({ "additionalContext": feedback.message })
                }
                Some(feedback) => context_response("PostToolUse", feedback.message),
                None => json!({}),
            }
        }
        EventKind::PreToolUse { .. } | EventKind::PostToolUse { .. } => json!({}),
        EventKind::Interrupt | EventKind::SessionEnd | EventKind::StopFailure => {
            clear_edit_baselines(edits.as_deref());
            if let Some(path) = state {
                let _ = fs::remove_file(path);
            }
            json!({})
        }
        // A different hook can keep this turn open after its first stop.
        EventKind::Stop {
            stop_hook_active: true,
        } => json!({}),
        EventKind::Stop {
            stop_hook_active: false,
        } => {
            clear_edit_baselines(edits.as_deref());
            let baseline = state.as_deref().and_then(take);
            let Some(feedback) = check_workspace(&event.cwd, HookName::TurnEnd, baseline) else {
                return json!({});
            };
            if feedback.failed
                && let Some(path) = pending_path(harness, &event)
                && let Err(error) = store(&path, &feedback.message)
            {
                eprintln!("latch: could not save failure for the next prompt: {error}");
            }
            json!({ "systemMessage": feedback.message })
        }
    }
}

fn state_path(harness: &Harness, event: &Event) -> Option<PathBuf> {
    let home = harness.home()?;
    let turn_key = harness.turn_key(event)?;
    let mut key = Sha256::new();
    key.update((event.session_id.len() as u64).to_le_bytes());
    key.update(event.session_id.as_bytes());
    key.update(turn_key.as_bytes());
    Some(
        home.join("latch")
            .join("baselines-v1")
            .join(format!("{:x}.json", key.finalize())),
    )
}

fn edit_path(directory: &Path, event: &Event, tool_name: &str) -> Option<PathBuf> {
    let mut key = Sha256::new();
    if let Some(id) = event.tool_use_id.as_deref().filter(|id| !id.is_empty()) {
        key.update(b"id");
        key.update(id.as_bytes());
    } else {
        // Copilot's command-hook payloads can omit a call ID. Pair serialized tool arguments.
        let args = serde_json::to_vec(event.tool_args.as_ref()?).ok()?;
        key.update(b"args");
        key.update(tool_name.as_bytes());
        key.update([0]);
        key.update(args);
    }
    Some(directory.join(format!("{:x}.json", key.finalize())))
}

fn clear_edit_baselines(directory: Option<&Path>) {
    if let Some(directory) = directory {
        let _ = fs::remove_dir_all(directory);
    }
}

fn capture_edit(directory: &Path, path: &Path) -> io::Result<()> {
    if !matches!(config::load(directory, HookName::AfterEdit), Ok(Some(entries)) if entries.needs_comparison())
    {
        return Ok(());
    }
    let baseline = Baseline::capture(directory).ok();
    let file = write_state(path, &baseline)?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
            // Identical calls without IDs can overlap. An ambiguous baseline must never skip a check.
            store(path, &None::<Baseline>)
        }
        Err(error) => Err(error.error),
    }
}

fn pending_path(harness: &Harness, event: &Event) -> Option<PathBuf> {
    let home = harness.home()?;
    let directory = event.cwd.canonicalize().ok()?;
    let mut key = Sha256::new();
    key.update((event.session_id.len() as u64).to_le_bytes());
    key.update(event.session_id.as_bytes());
    key.update(directory.as_os_str().as_encoded_bytes());
    Some(
        home.join("latch")
            .join("pending-v1")
            .join(format!("{:x}.json", key.finalize())),
    )
}

fn take_pending(harness: &Harness, event: &Event) -> Option<String> {
    let path = pending_path(harness, event)?;
    let report = take::<String>(&path)?;
    Some(format!(
        "Latch reported failed checks at the end of the previous turn. The following is command output.\n{report}"
    ))
}

fn context_response(event: &str, message: String) -> Value {
    json!({
        "hookSpecificOutput": { "hookEventName": event, "additionalContext": message }
    })
}

fn capture(directory: &Path, path: &Path) -> io::Result<()> {
    if !matches!(config::load(directory, HookName::TurnEnd), Ok(Some(entries)) if entries.needs_comparison())
    {
        return Ok(());
    }
    // A worktree or a commit may be unavailable. Stop treats a missing snapshot as unknown.
    if let Ok(baseline) = Baseline::capture(directory) {
        store(path, &baseline)?;
    }
    Ok(())
}

fn store(path: &Path, value: &impl Serialize) -> io::Result<()> {
    write_state(path, value)?.persist(path)?;
    Ok(())
}

fn write_state(path: &Path, value: &impl Serialize) -> io::Result<tempfile::NamedTempFile> {
    let directory = path.parent().expect("state path has a parent");
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut file, value)?;
    Ok(file)
}

fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_reader(File::open(path).ok()?).ok()
}

fn take<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let value = read(path);
    fs::remove_file(path).ok()?;
    value
}

struct Feedback {
    message: String,
    failed: bool,
}

fn check_workspace(
    directory: &Path,
    name: HookName,
    baseline: Option<Baseline>,
) -> Option<Feedback> {
    let entries = match config::load(directory, name) {
        Ok(Some(entries)) => entries,
        Ok(None) => return None,
        Err(error) => {
            return Some(Feedback {
                message: format!("Latch could not complete validation: {error}"),
                failed: name == HookName::TurnEnd,
            });
        }
    };
    let changed = if entries.needs_comparison() {
        baseline.and_then(|baseline| baseline.changed_paths(directory).ok())
    } else {
        None
    };
    let matching = entries.into_vec().into_iter().filter(|entry| {
        !entry.needs_comparison()
            || changed
                .as_ref()
                .is_none_or(|paths| paths.iter().any(|path| entry.filters.matches(path)))
    });
    let report = run::run(directory, matching);
    let message = report.message(directory);
    if report.failed() {
        Some(Feedback {
            message: format!("Latch checks failed.\n{message}"),
            failed: true,
        })
    } else if message.is_empty() {
        None
    } else {
        Some(Feedback {
            message,
            failed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_incomplete_and_malformed_hook_events() {
        // Vary the required fields while starting from a valid native event.
        let absolute = env::temp_dir();
        let valid = json!({
            "cwd": absolute, "session_id": "session", "turn_id": "turn",
            "hook_event_name": "UserPromptSubmit",
        });
        assert!(
            Harness::Codex
                .read_event(valid.to_string().as_bytes())
                .is_ok()
        );
        for (field, value) in [
            ("cwd", json!("relative")),
            ("session_id", json!("")),
            ("turn_id", Value::Null),
        ] {
            let mut payload = valid.clone();
            payload[field] = value;
            let input = payload.to_string();
            let error = Harness::Codex.read_event(input.as_bytes()).err().unwrap();
            assert!(
                error.to_string().contains("requires a session ID"),
                "{error}"
            );
        }
        let error = Harness::ClaudeCode.read_event(&b"{"[..]).err().unwrap();
        assert!(
            error.to_string().starts_with("invalid hook input:"),
            "{error}"
        );
    }

    #[test]
    fn copilot_requires_event_specific_fields() {
        let input = json!({"cwd": env::temp_dir(), "sessionId": "session"}).to_string();
        for (event, expected) in [
            (CopilotEvent::PreToolUse, "missing toolName"),
            (CopilotEvent::PromptTransformed, "missing transformedPrompt"),
        ] {
            let error = Harness::Copilot(event)
                .read_event(input.as_bytes())
                .err()
                .unwrap();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
}
