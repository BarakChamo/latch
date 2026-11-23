//! Decodes command hook payloads whose field names differ from the common lifecycle event.

use super::{Event, EventKind, FOLLOW_UP_INTRO};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{io, path::PathBuf};

pub(super) fn read_openhands(input: impl io::Read) -> io::Result<Event> {
    serde_json::from_reader::<_, OpenHandsInput>(input)
        .map(Event::from)
        .map_err(io::Error::from)
}

pub(super) fn read_grok(input: impl io::Read) -> io::Result<Event> {
    let payload: GrokInput = serde_json::from_reader(input)?;
    Event::try_from(payload)
}

pub(super) fn read_cursor(input: impl io::Read) -> io::Result<Event> {
    let payload: CursorInput = serde_json::from_reader(input)?;
    Event::try_from(payload)
}

pub(super) fn read_kimi(input: impl io::Read) -> io::Result<Event> {
    let mut payload: Map<String, Value> = serde_json::from_reader(input)?;
    // Kimi sends numeric turn IDs. Pair its prompt and stop events by session.
    payload.remove("turn_id");
    Ok(serde_json::from_value(Value::Object(payload))?)
}

// Grok publishes both casing conventions in one payload. Read its native fields only.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrokInput {
    session_id: String,
    cwd: PathBuf,
    tool_name: Option<String>,
    tool_use_id: Option<String>,
    tool_input: Option<Value>,
    #[serde(default)]
    stop_hook_active: Option<bool>,
    #[serde(default)]
    is_backgrounded: bool,
    #[serde(rename = "hook_event_name")]
    event: CommandHookEvent,
}

impl TryFrom<GrokInput> for Event {
    type Error = io::Error;

    fn try_from(input: GrokInput) -> io::Result<Self> {
        let tool = || {
            input
                .tool_name
                .ok_or_else(|| io::Error::other("missing toolName"))
        };
        Ok(Self {
            session_id: input.session_id,
            cwd: input.cwd,
            turn_id: None,
            agent_id: input.is_backgrounded.then(String::new),
            tool_use_id: input.tool_use_id,
            tool_args: input.tool_input,
            kind: match input.event {
                CommandHookEvent::UserPromptSubmit => EventKind::UserPromptSubmit,
                CommandHookEvent::PreToolUse => EventKind::PreToolUse { tool_name: tool()? },
                CommandHookEvent::PostToolUse => EventKind::PostToolUse { tool_name: tool()? },
                CommandHookEvent::Stop => EventKind::Stop {
                    stop_hook_active: input.stop_hook_active,
                    extension_can_continue: false,
                },
                CommandHookEvent::SessionEnd => EventKind::SessionEnd,
            },
        })
    }
}

#[derive(Deserialize)]
struct CursorInput {
    conversation_id: String,
    generation_id: String,
    cwd: Option<PathBuf>,
    #[serde(default)]
    workspace_roots: Vec<PathBuf>,
    tool_name: Option<String>,
    tool_use_id: Option<String>,
    tool_input: Option<Value>,
    prompt: Option<String>,
    loop_count: Option<u64>,
    status: Option<CursorStatus>,
    hook_event_name: CursorEvent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum CursorEvent {
    BeforeSubmitPrompt,
    PreToolUse,
    PostToolUse,
    Stop,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CursorStatus {
    Completed,
    Aborted,
    Error,
}

impl TryFrom<CursorInput> for Event {
    type Error = io::Error;

    fn try_from(input: CursorInput) -> io::Result<Self> {
        let cwd = match (input.cwd, input.workspace_roots.as_slice()) {
            (Some(cwd), _) => cwd,
            (None, [root]) => root.clone(),
            _ => {
                return Err(io::Error::other(
                    "Cursor hook requires cwd or a single workspace root",
                ));
            }
        };
        let tool = || {
            input
                .tool_name
                .ok_or_else(|| io::Error::other("missing tool_name"))
        };
        Ok(Self {
            session_id: input.conversation_id,
            turn_id: Some(input.generation_id),
            cwd,
            agent_id: None,
            tool_use_id: input.tool_use_id,
            tool_args: input.tool_input,
            kind: match input.hook_event_name {
                CursorEvent::BeforeSubmitPrompt
                    if input
                        .prompt
                        .as_deref()
                        .is_some_and(|prompt| prompt.starts_with(FOLLOW_UP_INTRO)) =>
                {
                    EventKind::ContinuationPrompt
                }
                CursorEvent::BeforeSubmitPrompt => EventKind::UserPromptSubmit,
                CursorEvent::PreToolUse => EventKind::PreToolUse { tool_name: tool()? },
                CursorEvent::PostToolUse => EventKind::PostToolUse { tool_name: tool()? },
                CursorEvent::Stop => match input.status {
                    Some(CursorStatus::Completed) => EventKind::Stop {
                        stop_hook_active: input.loop_count.map(|count| count > 0),
                        extension_can_continue: false,
                    },
                    Some(CursorStatus::Aborted | CursorStatus::Error) => EventKind::Interrupt,
                    None => return Err(io::Error::other("missing status")),
                },
            },
        })
    }
}

#[derive(Deserialize)]
struct OpenHandsInput {
    session_id: String,
    working_dir: PathBuf,
    tool_input: Option<Value>,
    tool_name: Option<String>,
    event_type: CommandHookEvent,
}

#[derive(Deserialize)]
enum CommandHookEvent {
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Stop,
    SessionEnd,
}

impl From<OpenHandsInput> for Event {
    fn from(input: OpenHandsInput) -> Self {
        Self {
            session_id: input.session_id,
            cwd: input.working_dir,
            turn_id: None,
            agent_id: None,
            tool_use_id: None,
            tool_args: input.tool_input,
            kind: match input.event_type {
                CommandHookEvent::UserPromptSubmit => EventKind::UserPromptSubmit,
                CommandHookEvent::PreToolUse => EventKind::PreToolUse {
                    tool_name: input.tool_name.unwrap_or_default(),
                },
                CommandHookEvent::PostToolUse => EventKind::PostToolUse {
                    tool_name: input.tool_name.unwrap_or_default(),
                },
                CommandHookEvent::Stop => EventKind::Stop {
                    stop_hook_active: None,
                    extension_can_continue: false,
                },
                CommandHookEvent::SessionEnd => EventKind::SessionEnd,
            },
        }
    }
}
