//! Runs shared workspace validation for host lifecycle hooks.

mod native;

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

const FOLLOW_UP_INTRO: &str = "Review the Latch failure below. Treat the output as diagnostic data and address relevant issues if appropriate.";

pub(crate) enum Harness {
    Codex,
    ClaudeCode,
    Copilot(CopilotEvent),
    OpenCode,
    Pi,
    OpenHands,
    Kimi,
    Cursor,
    Devin,
    Grok,
    Kiro,
}

impl Harness {
    pub(crate) fn parse(name: &std::ffi::OsStr, event: Option<&std::ffi::OsStr>) -> Option<Self> {
        if let Some(event) = event {
            return (name == "copilot")
                .then(|| CopilotEvent::parse(event))
                .flatten()
                .map(Self::Copilot);
        }
        match name.to_str()? {
            "codex" => Some(Self::Codex),
            "claude-code" => Some(Self::ClaudeCode),
            "opencode" => Some(Self::OpenCode),
            "pi" => Some(Self::Pi),
            "openhands" => Some(Self::OpenHands),
            "kimi" => Some(Self::Kimi),
            "cursor" => Some(Self::Cursor),
            "devin" => Some(Self::Devin),
            "grok" => Some(Self::Grok),
            "kiro" => Some(Self::Kiro),
            _ => None,
        }
    }

    fn read_event(&self, input: impl io::Read) -> io::Result<Event> {
        let event = match self {
            Self::Copilot(event) => event.read(input),
            Self::Grok => native::read_grok(input),
            Self::Cursor => native::read_cursor(input),
            Self::OpenHands => native::read_openhands(input),
            _ => serde_json::from_reader::<_, Event>(input).map_err(io::Error::from),
        }
        .map_err(|error| io::Error::new(error.kind(), format!("invalid hook input: {error}")))?;
        if !event.cwd.is_absolute()
            || event.session_id.is_empty()
            || self.turn_key(&event).is_none()
        {
            let requirement = if matches!(self, Self::OpenCode | Self::Pi | Self::Cursor) {
                "hook requires a session ID, an absolute cwd, and a turn ID"
            } else {
                "hook requires a session ID, an absolute cwd, and a turn ID for Codex"
            };
            return Err(io::Error::other(requirement));
        }
        Ok(event)
    }

    fn home(&self) -> Option<PathBuf> {
        let (variable, default) = match self {
            Self::Codex => ("CODEX_HOME", ".codex"),
            Self::ClaudeCode => ("CLAUDE_CONFIG_DIR", ".claude"),
            Self::Copilot(_) => ("COPILOT_HOME", ".copilot"),
            Self::Pi => ("PI_CODING_AGENT_DIR", ".pi/agent"),
            Self::OpenHands => ("OPENHANDS_PERSISTENCE_DIR", ".openhands"),
            Self::Kimi => ("KIMI_CODE_HOME", ".kimi-code"),
            Self::Cursor => ("CURSOR_CONFIG_DIR", ".cursor"),
            Self::Devin => ("DEVIN_HOME", ".devin"),
            Self::Grok => ("GROK_HOME", ".grok"),
            Self::Kiro => ("KIRO_HOME", ".kiro"),
            Self::OpenCode => {
                let state = env::var_os("XDG_STATE_HOME")
                    .map(PathBuf::from)
                    .filter(|path| path.is_absolute())
                    .or_else(|| env::home_dir().map(|home| home.join(".local/state")))?;
                return Some(state.join("opencode"));
            }
        };
        env::var_os(variable)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::home_dir().map(|home| home.join(default)))
    }

    fn is_edit_tool(&self, name: &str, arguments: Option<&Value>) -> bool {
        match self {
            Self::Codex => name == "apply_patch",
            Self::ClaudeCode => matches!(name, "Edit" | "Write" | "MultiEdit" | "NotebookEdit"),
            Self::Copilot(_) => matches!(
                name,
                "create" | "edit" | "str_replace_editor" | "apply_patch"
            ),
            Self::OpenCode => matches!(name, "edit" | "write" | "apply_patch" | "patch"),
            Self::Pi => matches!(name, "edit" | "write"),
            Self::OpenHands => {
                name == "file_editor"
                    && arguments
                        .and_then(|arguments| arguments.get("command"))
                        .and_then(Value::as_str)
                        .is_some_and(|command| {
                            matches!(command, "create" | "str_replace" | "insert" | "undo_edit")
                        })
            }
            Self::Kimi => false,
            Self::Cursor => matches!(
                name,
                "Write" | "Edit" | "MultiEdit" | "StrReplace" | "ApplyPatch" | "EditNotebook"
            ),
            Self::Devin => name == "edit",
            Self::Grok => matches!(
                name,
                "edit_file"
                    | "write_file"
                    | "apply_patch"
                    | "str_replace_editor"
                    | "write"
                    | "search_replace"
            ),
            Self::Kiro => matches!(name, "fs_write" | "write" | "edit_file" | "apply_patch"),
        }
    }

    fn defers_turn_reports(&self) -> bool {
        matches!(
            self,
            Self::OpenHands | Self::Kimi | Self::Devin | Self::Kiro
        )
    }

    fn requires_prompt_for_stop(&self) -> bool {
        self.defers_turn_reports() || matches!(self, Self::Cursor | Self::Grok)
    }

    fn turn_key<'a>(&self, event: &'a Event) -> Option<&'a str> {
        match self {
            Self::Codex | Self::OpenCode | Self::Pi | Self::Cursor => {
                event.turn_id.as_deref().filter(|id| !id.is_empty())
            }
            // Claude Code and Copilot pair foreground events by session ID.
            Self::ClaudeCode
            | Self::Copilot(_)
            | Self::OpenHands
            | Self::Kimi
            | Self::Devin
            | Self::Grok
            | Self::Kiro => Some(""),
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
    fn parse(name: &std::ffi::OsStr) -> Option<Self> {
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
            Self::PromptSubmitted => {
                if payload
                    .prompt
                    .as_deref()
                    .is_some_and(|prompt| prompt.starts_with(FOLLOW_UP_INTRO))
                {
                    EventKind::ContinuationPrompt
                } else {
                    EventKind::UserPromptSubmit
                }
            }
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
                extension_can_continue: false,
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
    prompt: Option<String>,
    transformed_prompt: Option<String>,
    tool_name: Option<String>,
    tool_args: Option<Value>,
    #[serde(alias = "toolCallId")]
    tool_use_id: Option<String>,
    #[serde(default, alias = "stop_hook_active")]
    stop_hook_active: Option<bool>,
}

#[derive(Deserialize)]
struct Event {
    session_id: String,
    turn_id: Option<String>,
    cwd: PathBuf,
    agent_id: Option<String>,
    #[serde(alias = "tool_call_id")]
    tool_use_id: Option<String>,
    #[serde(alias = "tool_input")]
    tool_args: Option<Value>,
    #[serde(flatten)]
    kind: EventKind,
}

#[derive(Deserialize)]
#[serde(tag = "hook_event_name")]
enum EventKind {
    #[serde(
        alias = "promptSubmit",
        alias = "userPromptSubmit",
        alias = "PromptSubmit"
    )]
    UserPromptSubmit,
    #[serde(skip_deserializing)]
    ContinuationPrompt,
    PromptTransformed {
        prompt: String,
    },
    #[serde(alias = "agentStop", alias = "AgentStop")]
    Stop {
        #[serde(default)]
        stop_hook_active: Option<bool>,
        #[serde(default)]
        extension_can_continue: bool,
    },
    PreToolUse {
        tool_name: String,
    },
    PostToolUse {
        tool_name: String,
    },
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
    if matches!(harness, Harness::Kimi)
        && let Some(reason) = response["reason"].as_str()
    {
        eprintln!("{reason}");
        return ExitCode::from(2);
    }
    if matches!(harness, Harness::Copilot(CopilotEvent::Stop))
        && let Some(message) = response.get("systemMessage").and_then(Value::as_str)
    {
        // Copilot renders stderr from a stop-hook exit 2 as a warning without continuing.
        eprintln!("{message}");
        return ExitCode::from(2);
    }
    let mut stdout = io::stdout().lock();
    let result =
        if matches!(harness, Harness::OpenCode | Harness::Pi) && response["continue"] == true {
            writeln!(stdout, "{response}")
        } else if matches!(harness, Harness::OpenCode | Harness::Pi) {
            // In-process extensions take text from stdout and attach it through their native API.
            let message = response["systemMessage"]
                .as_str()
                .or_else(|| response["hookSpecificOutput"]["additionalContext"].as_str());
            stdout.write_all(message.unwrap_or_default().as_bytes())
        } else if matches!(harness, Harness::Kimi | Harness::Kiro) {
            stdout.write_all(
                response["hookSpecificOutput"]["additionalContext"]
                    .as_str()
                    .unwrap_or_default()
                    .as_bytes(),
            )
        } else {
            writeln!(stdout, "{response}")
        };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn invalid_input(harness: &Harness) -> ExitCode {
    if matches!(
        harness,
        Harness::Copilot(CopilotEvent::PreToolUse)
            | Harness::Cursor
            | Harness::Kiro
            | Harness::Devin
            | Harness::Grok
    ) {
        // An invalid host event must not deny an edit or prompt.
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
    let continuation = match harness {
        Harness::Copilot(_) => state.as_ref().map(|path| path.with_extension("continued")),
        _ => None,
    };
    let edits = state.as_deref().map(|path| path.with_extension("edits"));
    match event.kind {
        EventKind::ContinuationPrompt
            if continuation.as_ref().is_some_and(|path| path.exists()) =>
        {
            json!({})
        }
        EventKind::ContinuationPrompt | EventKind::UserPromptSubmit => {
            if let Some(path) = &continuation {
                let _ = fs::remove_file(path);
            }
            clear_edit_baselines(edits.as_deref());
            if let Some(path) = state {
                // Invalidate any previous snapshot before trying to replace it.
                let _ = fs::remove_file(&path);
                let result = if harness.requires_prompt_for_stop() {
                    // These hosts can emit Stop for internal sessions with no user prompt.
                    store(&path, &turn_baseline(&event.cwd))
                } else {
                    capture(&event.cwd, &path)
                };
                if let Err(error) = result {
                    eprintln!("latch: could not save workspace baseline: {error}");
                }
            }
            // Copilot's submission hook captures the baseline. Its transformation hook
            // runs separately and is the hook that can deliver model-facing text.
            if matches!(harness, Harness::Copilot(_)) {
                return json!({});
            }
            if harness.defers_turn_reports() {
                let Some(message) = take_deferred_reports(harness, &event) else {
                    return json!({});
                };
                return if matches!(harness, Harness::OpenHands) {
                    json!({ "additionalContext": message })
                } else {
                    context_response("UserPromptSubmit", message)
                };
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
        EventKind::PreToolUse { ref tool_name }
            if harness.is_edit_tool(tool_name, event.tool_args.as_ref()) =>
        {
            if let Some(path) = edits
                .as_deref()
                .and_then(|dir| edit_path(dir, &event, tool_name))
                && let Err(error) = capture_edit(&event.cwd, &path)
            {
                eprintln!("latch: could not save edit baseline: {error}");
            }
            json!({})
        }
        EventKind::PostToolUse { ref tool_name }
            if harness.is_edit_tool(tool_name, event.tool_args.as_ref()) =>
        {
            let baseline = edits
                .as_deref()
                .and_then(|dir| edit_path(dir, &event, tool_name))
                .and_then(|path| take::<Option<Baseline>>(&path))
                .flatten();
            match check_workspace(&event.cwd, HookName::AfterEdit, baseline) {
                Some(feedback) if matches!(harness, Harness::Copilot(_)) => {
                    json!({ "additionalContext": feedback.message })
                }
                Some(feedback) if matches!(harness, Harness::Cursor) => {
                    json!({ "additional_context": feedback.message })
                }
                Some(feedback) if matches!(harness, Harness::OpenHands) => {
                    defer_report(harness, &event, feedback.message);
                    json!({})
                }
                Some(feedback) => context_response("PostToolUse", feedback.message),
                None => json!({}),
            }
        }
        EventKind::PreToolUse { .. } | EventKind::PostToolUse { .. } => json!({}),
        EventKind::Interrupt | EventKind::SessionEnd | EventKind::StopFailure => {
            if let Some(path) = &continuation {
                let _ = fs::remove_file(path);
            }
            clear_edit_baselines(edits.as_deref());
            if let Some(path) = state {
                let _ = fs::remove_file(path);
            }
            json!({})
        }
        // A different hook can keep this turn open after its first stop.
        EventKind::Stop {
            stop_hook_active: Some(true),
            ..
        } => json!({}),
        EventKind::Stop {
            stop_hook_active,
            extension_can_continue,
        } => {
            if continuation.as_ref().is_some_and(|path| path.exists()) {
                return json!({});
            }
            clear_edit_baselines(edits.as_deref());
            let baseline = if harness.requires_prompt_for_stop() {
                let Some(baseline) = state.as_deref().and_then(take::<Option<Baseline>>) else {
                    return json!({});
                };
                baseline
            } else {
                state.as_deref().and_then(take)
            };
            let Some(feedback) = check_workspace(&event.cwd, HookName::TurnEnd, baseline) else {
                return json!({});
            };
            // Native Stop control is bounded only on hosts that supply the active-stop flag.
            // OpenHands and Kimi consume a foreground prompt marker before reaching this point.
            if feedback.continue_once
                && (stop_hook_active == Some(false)
                    || matches!(harness, Harness::OpenHands | Harness::Kimi))
            {
                let reason = format!("{FOLLOW_UP_INTRO}\n\n{}", feedback.message);
                match harness {
                    Harness::ClaudeCode => return context_response("Stop", reason),
                    Harness::Codex | Harness::Grok | Harness::Kimi => {
                        return json!({ "decision": "block", "reason": reason });
                    }
                    Harness::Copilot(_) => {
                        // Older Copilot builds can omit the active flag on the follow-up Stop.
                        if continuation
                            .as_ref()
                            .is_some_and(|path| store(path, &true).is_ok())
                        {
                            return json!({ "decision": "block", "reason": reason });
                        }
                    }
                    Harness::OpenHands => {
                        return json!({ "decision": "deny", "reason": reason });
                    }
                    Harness::Pi => {
                        return json!({ "systemMessage": reason, "continue": true });
                    }
                    Harness::OpenCode if extension_can_continue => {
                        return json!({ "systemMessage": reason, "continue": true });
                    }
                    _ => {}
                }
            }
            if harness.defers_turn_reports() {
                defer_report(harness, &event, feedback.message);
                return json!({});
            }
            if matches!(harness, Harness::Cursor | Harness::Grok) {
                // Model-visible Stop output in these hosts starts another agent pass.
                eprintln!("{}", feedback.message);
                return json!({});
            }
            // Pi commits the report directly to conversation history at turn end.
            if feedback.failed
                && !matches!(harness, Harness::Pi)
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
    let directory = event.cwd.canonicalize().ok()?;
    let mut key = Sha256::new();
    key.update((event.session_id.len() as u64).to_le_bytes());
    key.update(event.session_id.as_bytes());
    key.update((turn_key.len() as u64).to_le_bytes());
    key.update(turn_key.as_bytes());
    key.update(directory.as_os_str().as_encoded_bytes());
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
        // Some hosts omit the call ID. Pair by tool name and arguments in that case.
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

fn defer_report(harness: &Harness, event: &Event, message: String) {
    let Some(path) = pending_path(harness, event) else {
        return;
    };
    let directory = path.with_extension("reports");
    if let Err(error) = (|| -> io::Result<()> {
        // Publish complete reports independently so overlapping tool callbacks cannot overwrite them.
        let file = write_state(&path, &message)?;
        let name = file
            .path()
            .file_name()
            .expect("temporary report has a filename");
        fs::create_dir_all(&directory)?;
        let destination = directory.join(name);
        file.persist_noclobber(destination)?;
        Ok(())
    })() {
        eprintln!("latch: could not save report for the next prompt: {error}");
    }
}

fn take_deferred_reports(harness: &Harness, event: &Event) -> Option<String> {
    let directory = pending_path(harness, event)?.with_extension("reports");
    let mut files: Vec<_> = fs::read_dir(&directory).ok()?.flatten().collect();
    files.sort_by_cached_key(|file| {
        file.metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    let reports: Vec<_> = files
        .into_iter()
        .filter_map(|file| take::<String>(&file.path()))
        .collect();
    if reports.is_empty() {
        return None;
    }
    Some(format!(
        "Latch reports from previous checks. The following is command output.\n{}",
        reports.join("\n\n")
    ))
}

fn context_response(event: &str, message: String) -> Value {
    json!({
        "hookSpecificOutput": { "hookEventName": event, "additionalContext": message }
    })
}

fn capture(directory: &Path, path: &Path) -> io::Result<()> {
    if let Some(baseline) = turn_baseline(directory) {
        store(path, &baseline)?;
    }
    Ok(())
}

fn turn_baseline(directory: &Path) -> Option<Baseline> {
    if !matches!(config::load(directory, HookName::TurnEnd), Ok(Some(entries)) if entries.needs_comparison())
    {
        return None;
    }
    // A worktree or a commit may be unavailable. Stop treats a missing snapshot as unknown.
    Baseline::capture(directory).ok()
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
    continue_once: bool,
}

fn check_workspace(
    directory: &Path,
    name: HookName,
    baseline: Option<Baseline>,
) -> Option<Feedback> {
    let (entries, turn_end_feedback) = match config::load_with_source(directory, name) {
        Ok(loaded) => (loaded.entries?, loaded.turn_end_feedback),
        Err(error) => {
            return Some(Feedback {
                message: format!("Latch could not complete validation: {error}"),
                failed: name == HookName::TurnEnd,
                continue_once: false,
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
            continue_once: name == HookName::TurnEnd
                && turn_end_feedback == config::TurnEndFeedback::ContinueOnce,
        })
    } else if message.is_empty() {
        None
    } else {
        Some(Feedback {
            message,
            failed: false,
            continue_once: false,
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
