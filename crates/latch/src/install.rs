//! Registers local command-hook integrations and their bundled stdio tool.

pub(crate) mod cloud;

use serde_json::{Map, Value, json};
use std::{env, fs, io, path::Path, path::PathBuf};

const COMMAND_MARKER: &str = " # latch-installed";

#[derive(Clone, Copy)]
pub(crate) enum Target {
    OpenHands,
    Cursor,
    Devin,
    Grok,
    Kiro,
}

impl Target {
    pub(crate) fn parse(host: &str) -> Option<Self> {
        match host {
            "openhands" => Some(Self::OpenHands),
            "cursor" => Some(Self::Cursor),
            "devin" => Some(Self::Devin),
            "grok" => Some(Self::Grok),
            "kiro" => Some(Self::Kiro),
            _ => None,
        }
    }

    pub(crate) fn is_preview(self) -> bool {
        matches!(self, Self::Devin | Self::Kiro)
    }

    fn name(self) -> &'static str {
        match self {
            Self::OpenHands => "openhands",
            Self::Cursor => "cursor",
            Self::Devin => "devin",
            Self::Grok => "grok",
            Self::Kiro => "kiro",
        }
    }

    fn hook_path(self) -> &'static str {
        match self {
            Self::OpenHands => ".openhands/hooks.json",
            Self::Cursor => ".cursor/hooks.json",
            Self::Devin => ".devin/hooks.v1.json",
            Self::Grok => ".grok/hooks/latch.json",
            Self::Kiro => ".kiro/hooks/latch.json",
        }
    }

    fn mcp_path(self) -> io::Result<PathBuf> {
        match self {
            Self::OpenHands => env::var_os("OPENHANDS_PERSISTENCE_DIR")
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
                .or_else(|| env::home_dir().map(|home| home.join(".openhands")))
                .map(|path| path.join("mcp.json"))
                .ok_or_else(|| {
                    io::Error::other("could not locate the OpenHands configuration directory")
                }),
            Self::Cursor => Ok(".cursor/mcp.json".into()),
            Self::Devin => Ok(".devin/mcp_config.local.json".into()),
            Self::Grok => Ok(".mcp.json".into()),
            Self::Kiro => Ok(".kiro/settings/mcp.json".into()),
        }
    }
}

/// Wires the selected host to the running Latch executable from the current project.
pub(crate) fn install(target: Target) -> io::Result<()> {
    let binary = env::current_exe()?;
    let executable = binary
        .to_str()
        .filter(|path| !path.chars().any(char::is_control))
        .ok_or_else(|| {
            io::Error::other("use a UTF-8 installation path without control characters")
        })?;
    let project = env::current_dir()?;
    let mut hook_path = project.join(target.hook_path());
    // OpenHands loads one hook file. Keep the user's existing effective scope.
    if matches!(target, Target::OpenHands)
        && !hook_path.try_exists()?
        && let Some(global) = env::home_dir().map(|home| home.join(target.hook_path()))
        && global.try_exists()?
    {
        hook_path = global;
    }
    let command = format!(
        "'{}' hook {}{COMMAND_MARKER}",
        executable.replace('\'', "'\\''"),
        target.name()
    );
    let mut hooks = read_config(&hook_path)?;
    configure_hooks(target, &mut hooks, &command)?;

    let mcp_path = project.join(target.mcp_path()?);
    let mut mcp = read_config(&mcp_path)?;
    let servers = object(&mut mcp, "mcpServers")?;
    let args = json!(["mcp", "--require-project-directory"]);
    if let Some(existing) = servers.get("latch")
        && (existing["args"] != args
            || !existing["command"].as_str().is_some_and(|path| {
                Path::new(path)
                    .file_name()
                    .is_some_and(|name| name == "latch")
            }))
    {
        return Err(io::Error::other(format!(
            "{} already defines a different Latch MCP server",
            mcp_path.display()
        )));
    }
    servers.insert(
        "latch".into(),
        json!({ "command": executable, "args": args }),
    );
    write_config(&mcp_path, &mcp)?;
    write_config(&hook_path, &hooks)?;
    println!(
        "Installed Latch hooks at {}. Start a new session in this folder.",
        hook_path.display()
    );
    Ok(())
}

fn configure_hooks(
    target: Target,
    config: &mut Map<String, Value>,
    command: &str,
) -> io::Result<()> {
    match target {
        Target::OpenHands => {
            let events = if config.contains_key("hooks") {
                object(config, "hooks")?
            } else {
                config
            };
            for (snake, pascal, matcher) in [
                ("user_prompt_submit", "UserPromptSubmit", "*"),
                ("pre_tool_use", "PreToolUse", "file_editor"),
                ("post_tool_use", "PostToolUse", "file_editor"),
                ("stop", "Stop", "*"),
                ("session_end", "SessionEnd", "*"),
            ] {
                let event = if events.contains_key(pascal) {
                    pascal
                } else {
                    snake
                };
                register(
                    events,
                    event,
                    json!({
                        "matcher": matcher, "hooks": [{ "command": command, "timeout": 120 }]
                    }),
                )?;
            }
        }
        Target::Cursor => {
            config.entry("version").or_insert(json!(1));
            let events = object(config, "hooks")?;
            for event in ["beforeSubmitPrompt", "preToolUse", "postToolUse"] {
                register(events, event, json!({ "command": command, "timeout": 120 }))?;
            }
            register(
                events,
                "stop",
                json!({ "command": command, "timeout": 120, "loop_limit": 1 }),
            )?;
        }
        Target::Devin | Target::Grok => {
            let events = if matches!(target, Target::Grok) {
                object(config, "hooks")?
            } else {
                config
            };
            for event in [
                "UserPromptSubmit",
                "PreToolUse",
                "PostToolUse",
                "Stop",
                "SessionEnd",
            ] {
                register(
                    events,
                    event,
                    json!({ "hooks": [{ "type": "command", "command": command, "timeout": 120 }] }),
                )?;
            }
        }
        Target::Kiro => {
            config.entry("version").or_insert(json!("v1"));
            let records = array(config, "hooks")?;
            records.retain(|entry| !managed_command(&entry["action"]["command"]));
            for (event, name) in [
                ("UserPromptSubmit", "Start Latch turn"),
                ("PreToolUse", "Start Latch edit"),
                ("PostToolUse", "Check Latch edit"),
                ("Stop", "Check Latch turn"),
            ] {
                records.push(json!({ "name": name, "trigger": event, "timeout": 120,
                    "action": { "type": "command", "command": command }
                }));
            }
        }
    }
    Ok(())
}

fn register(config: &mut Map<String, Value>, event: &str, entry: Value) -> io::Result<()> {
    let handlers = array(config, event)?;
    handlers.retain(|existing| {
        !managed_command(&existing["command"])
            && !existing["hooks"]
                .as_array()
                .is_some_and(|hooks| hooks.len() == 1 && managed_command(&hooks[0]["command"]))
    });
    handlers.push(entry);
    Ok(())
}

fn managed_command(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|command| command.ends_with(COMMAND_MARKER))
}

fn array<'a>(config: &'a mut Map<String, Value>, key: &str) -> io::Result<&'a mut Vec<Value>> {
    config
        .entry(key)
        .or_insert(json!([]))
        .as_array_mut()
        .ok_or_else(|| io::Error::other(format!("{key} must be a JSON array")))
}

fn object<'a>(
    config: &'a mut Map<String, Value>,
    key: &str,
) -> io::Result<&'a mut Map<String, Value>> {
    config
        .entry(key)
        .or_insert(json!({}))
        .as_object_mut()
        .ok_or_else(|| io::Error::other(format!("{key} must be a JSON object")))
}

fn read_config(path: &Path) -> io::Result<Map<String, Value>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::other(format!("{}: {error}", path.display()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Map::new()),
        Err(error) => Err(error),
    }
}

fn write_config(path: &Path, config: &Map<String, Value>) -> io::Result<()> {
    let destination = match fs::canonicalize(path) {
        Ok(destination) => destination,
        Err(error) if error.kind() == io::ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(error),
    };
    let parent = destination
        .parent()
        .expect("configuration file has a parent");
    fs::create_dir_all(parent)?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(temp.as_file(), config)?;
    temp.persist(destination)?;
    Ok(())
}
