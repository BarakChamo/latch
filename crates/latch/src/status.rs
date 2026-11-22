//! Reports resolved commands and recent hook activity without running checks.

use crate::{config::HookName, explanation, output};
use serde::Serialize;
use serde_json::{Value, json};
use std::{fs, io, path::Path, time::SystemTime};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Passed,
    Failed,
    Unchanged,
    Filtered,
    NotConfigured,
    NoCommands,
    ConfigurationError,
}

pub(crate) fn observe(project: &Path) {
    write(project, "prompt", &timestamp());
}

pub(crate) fn record(project: &Path, hook: HookName, outcome: Outcome) {
    write(
        project,
        hook.name(),
        &json!({
            "outcome": outcome,
            "unix_seconds": timestamp(),
        }),
    );
}

pub(crate) fn snapshot(project: &Path) -> Value {
    let cache = directory(project);
    let read = |name: &str| read(cache.as_deref().ok()?, name);
    let checks = [HookName::AfterEdit, HookName::TurnEnd].map(|name| {
        let config = match explanation(project, name) {
            Ok(config) => Value::Object(config),
            Err(error) => json!({ "hook": name.name(), "error": error.to_string() }),
        };
        json!({ "config": config, "last_check": read(name.name()) })
    });
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "project_directory": project.canonicalize().unwrap_or_else(|_| project.to_path_buf()),
        "last_prompt_unix_seconds": read("prompt"),
        "checks": checks,
    })
}

fn timestamp() -> u64 {
    SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap_or_default()
        .as_secs()
}

fn write(project: &Path, name: &str, value: &impl Serialize) {
    // Status is diagnostic. Failure to save it must not change hook behavior.
    let _ = (|| -> io::Result<()> {
        let directory = directory(project)?;
        fs::create_dir_all(&directory)?;
        let mut file = tempfile::NamedTempFile::new_in(&directory)?;
        serde_json::to_writer(&mut file, value)?;
        file.persist(directory.join(format!("status-{name}.json")))?;
        Ok(())
    })();
}

fn directory(project: &Path) -> io::Result<std::path::PathBuf> {
    Ok(output::state_root()?
        .join("status-v1")
        .join(output::project_key(project)?))
}

fn read(directory: &Path, name: &str) -> Option<Value> {
    let file = fs::File::open(directory.join(format!("status-{name}.json"))).ok()?;
    serde_json::from_reader(file).ok()
}
