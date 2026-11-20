//! Maps local command execution to the CLI's output streams and exit status.

mod config;
mod discovery;
mod hooks;
mod mcp;
mod run;
mod workspace;

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use config::{HookEntries, HookEntry, HookName};
use serde_json::{Map, Value, json};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    use config::HookName::TurnEnd;
    let selected = match args.next() {
        None => TurnEnd,
        Some(arg) if arg == "run" || arg == "explain" => {
            let name = args.next();
            if args.next().is_some() {
                return usage();
            }
            let selected = match name.as_deref().and_then(config::HookName::parse) {
                Some(selected) => selected,
                None if name.is_none() && arg == "explain" => TurnEnd,
                None => return usage(),
            };
            if arg == "explain" {
                return explain(selected);
            }
            selected
        }
        Some(arg) if arg == "--version" && args.next().is_none() => {
            println!("latch {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some(arg) if arg == "mcp" && args.next().is_none() => return mcp::dispatch(),
        Some(arg) if arg == "hook" => {
            let harness = args.next();
            let event = args.next();
            if args.next().is_none() {
                match (harness.as_deref(), event.as_deref()) {
                    (Some(name), None) if name == "codex" => {
                        return hooks::dispatch(hooks::Harness::Codex);
                    }
                    (Some(name), None) if name == "claude-code" => {
                        return hooks::dispatch(hooks::Harness::ClaudeCode);
                    }
                    (Some(name), Some(event)) if name == "copilot" => {
                        if let Some(event) = hooks::CopilotEvent::parse(event) {
                            return hooks::dispatch(hooks::Harness::Copilot(event));
                        }
                    }
                    _ => {}
                }
            }
            return usage();
        }
        Some(_) => return usage(),
    };

    let directory = Path::new(".");
    let entries = match config::load(directory, selected) {
        Ok(Some(entries)) => entries,
        Ok(None) => {
            eprintln!("latch: hooks.{} is not configured", selected.name());
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("latch: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = run::run(directory, entries.into_vec());
    if let Err(error) = report.replay(io::stdout(), io::stderr()) {
        eprintln!("latch: could not report command output: {error}");
        return ExitCode::FAILURE;
    }
    if report.failed() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn explain(selected: config::HookName) -> ExitCode {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let directory = env::current_dir()?;
        let path = directory.join("latch.json");
        let entries = config::load(&directory, selected)?;
        let mut response = Map::from_iter([
            ("config".into(), json!(path)),
            ("hook".into(), json!(selected.name())),
            ("configured".into(), json!(entries.is_some())),
        ]);
        match entries {
            None => {}
            Some(HookEntries::Single(entry)) => response.extend(explain_entry(selected, entry)),
            Some(HookEntries::Multiple(entries)) => {
                let explanations: Vec<_> = entries
                    .into_iter()
                    .map(|entry| Value::Object(explain_entry(selected, entry)))
                    .collect();
                response.insert("entries".into(), Value::Array(explanations));
            }
        }
        let mut stdout = io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &response)?;
        writeln!(stdout)?;
        Ok(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("latch: {error}");
            ExitCode::FAILURE
        }
    }
}

fn explain_entry(selected: HookName, entry: HookEntry) -> Map<String, Value> {
    let comparison = entry.needs_comparison().then_some(match selected {
        HookName::TurnEnd => "turn_start",
        HookName::AfterEdit => "before_edit",
    });
    Map::from_iter([
        ("when".into(), json!(entry.when)),
        ("comparison".into(), json!(comparison)),
        ("on_success".into(), json!(entry.on_success)),
        ("on_failure".into(), json!(entry.on_failure)),
        ("commands".into(), json!(entry.commands)),
        ("paths".into(), json!(entry.filters.paths)),
        ("extensions".into(), json!(entry.filters.extensions)),
    ])
}

fn usage() -> ExitCode {
    eprintln!(
        "Usage: latch [--version] | latch run <turn_end|after_edit> | latch explain [turn_end|after_edit] | latch hook <codex|claude-code> | latch hook copilot <event> | latch mcp"
    );
    ExitCode::from(2)
}
