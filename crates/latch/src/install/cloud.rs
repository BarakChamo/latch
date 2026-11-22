//! Adds portable repository hooks without replacing existing harness settings.

use serde_json::{Map, Value, json};
use std::{fs, io, io::Write, path::Path};

use super::{array, object};

pub(crate) enum Target {
    Claude,
    Copilot,
    OpenHands,
}

impl Target {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "claude" | "claude-code" => Some(Self::Claude),
            "copilot" => Some(Self::Copilot),
            "openhands" => Some(Self::OpenHands),
            _ => None,
        }
    }

    fn config(&self) -> (&'static str, &'static str) {
        match self {
            Self::Claude => (
                ".claude/settings.json",
                include_str!("../../../../apps/docs/public/downloads/claude/settings.json"),
            ),
            Self::Copilot => (
                ".github/hooks/latch.json",
                include_str!("../../../../apps/docs/public/downloads/copilot/latch.json"),
            ),
            Self::OpenHands => (
                ".openhands/hooks.json",
                include_str!("../../../../apps/docs/public/downloads/openhands/hooks.json"),
            ),
        }
    }
}

/// Merges the cloud hooks into one settings file at the Git worktree root.
pub(crate) fn install(target: Target) -> io::Result<()> {
    let (relative, template) = target.config();
    let root = crate::workspace::worktree(&std::env::current_dir()?)?;
    let path = root.join(relative);
    let result = merge_file(target, &root, &path, template)
        .map_err(|error| io::Error::new(error.kind(), format!("{relative}: {error}")))?;
    if result {
        println!("Added Latch cloud hooks. Commit {relative} and start a new cloud session.");
    } else {
        println!("Latch cloud hooks are already configured in {relative}.");
    }
    Ok(())
}

fn merge_file(target: Target, root: &Path, path: &Path, template: &str) -> io::Result<bool> {
    for component in path.ancestors().take_while(|component| *component != root) {
        match fs::symlink_metadata(component) {
            Ok(metadata) if metadata.is_symlink() || (component == path && !metadata.is_file()) => {
                return Err(io::Error::other(
                    "expected a regular settings file inside the worktree",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }

    let existing = match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let mut config: Map<String, Value> =
        serde_json::from_slice(existing.as_deref().unwrap_or(b"{}")).map_err(io::Error::other)?;
    let before = config.clone();
    merge_hooks(target, &mut config, template)?;
    if config == before {
        return Ok(false);
    }

    let parent = path
        .parent()
        .expect("cloud settings have a parent directory");
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(temp.as_file_mut(), &config)?;
    temp.write_all(b"\n")?;
    match existing {
        Some(bytes) => {
            temp.as_file()
                .set_permissions(fs::metadata(path)?.permissions())?;
            if fs::read(path)? != bytes {
                return Err(io::Error::other(
                    "settings changed during setup; run the command again",
                ));
            }
            temp.persist(path)?;
        }
        None => {
            temp.persist_noclobber(path)?;
        }
    }
    Ok(true)
}

fn merge_hooks(target: Target, config: &mut Map<String, Value>, template: &str) -> io::Result<()> {
    let mut template: Map<String, Value> =
        serde_json::from_str(template).expect("the bundled cloud settings are valid JSON");

    if matches!(target, Target::Copilot) && config.entry("version").or_insert(json!(1)) != &json!(1)
    {
        return Err(io::Error::other("expected Copilot hook format version 1"));
    }

    let (existing, additions) = match target {
        Target::OpenHands if !config.contains_key("hooks") => (config, &mut template),
        Target::OpenHands => (object(config, "hooks")?, &mut template),
        _ => (object(config, "hooks")?, object(&mut template, "hooks")?),
    };

    for (event, groups) in additions {
        let key = match (event.as_str(), &target) {
            ("user_prompt_submit", Target::OpenHands)
                if existing.contains_key("UserPromptSubmit") =>
            {
                "UserPromptSubmit"
            }
            ("pre_tool_use", Target::OpenHands) if existing.contains_key("PreToolUse") => {
                "PreToolUse"
            }
            ("post_tool_use", Target::OpenHands) if existing.contains_key("PostToolUse") => {
                "PostToolUse"
            }
            ("stop", Target::OpenHands) if existing.contains_key("Stop") => "Stop",
            ("session_end", Target::OpenHands) if existing.contains_key("SessionEnd") => {
                "SessionEnd"
            }
            _ => event,
        };
        let handlers = array(existing, key)?;
        for group in groups.as_array().expect("bundled hook entries are arrays") {
            let commands = || handlers.iter().flat_map(hook_commands);
            if commands().any(|command| hook_commands(group).any(|bundled| command == bundled)) {
                continue;
            }
            if commands().any(is_other_latch_hook) {
                return Err(io::Error::other(format!(
                    "{key} already has a different Latch hook; review it before adding cloud hooks"
                )));
            }
            handlers.push(group.clone());
        }
    }
    Ok(())
}

fn hook_commands(entry: &Value) -> impl Iterator<Item = &str> {
    entry["bash"]
        .as_str()
        .or_else(|| entry["command"].as_str())
        .into_iter()
        .chain(
            entry["hooks"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|hook| hook["command"].as_str()),
        )
}

fn is_other_latch_hook(command: &str) -> bool {
    command.trim_start().starts_with("latch hook ")
        || command.ends_with(super::COMMAND_MARKER)
        || command.contains("/cloud/copilot-hook.sh")
        || (command.contains(" hook ")
            && ["/latch/cloud/", "/bin/latch", "/latch.sh"]
                .iter()
                .any(|marker| command.contains(marker)))
}
