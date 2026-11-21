//! Maps local command execution to the CLI's output streams and exit status.

mod config;
mod discovery;
mod hooks;
mod install;
mod mcp;
mod output;
mod rtk;
mod run;
mod workspace;

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use config::{HookEntries, HookEntry, HookName};
use serde_json::{Map, Value, json};

const USAGE: &str = "Usage: latch [--help|--version] | latch run <turn_end|after_edit> | latch explain [turn_end|after_edit] | latch config init | latch output <log-or-id> [--grep <text>] | latch output --prune | latch install <openhands|cursor|devin|grok|kiro> [--preview] | latch hook <codex|claude-code|opencode|pi|openhands|kimi|cursor|devin|grok|kiro> | latch hook copilot <event> | latch mcp [--require-project-directory]";

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1).peekable();
    if args.peek().is_none_or(|arg| arg != "output") {
        output::prune_if_due();
    }
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
        Some(arg) if (arg == "--help" || arg == "-h") && args.next().is_none() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some(arg) if arg == "--version" && args.next().is_none() => {
            println!("latch {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some(arg) if arg == "config" => {
            return match args.next().as_deref() {
                Some(option) if option == "init" && args.next().is_none() => initialize_config(),
                _ => usage(),
            };
        }
        Some(arg) if arg == "output" => {
            let Some(id) = args.next().and_then(|id| id.into_string().ok()) else {
                return usage();
            };
            if id == "--prune" {
                return if args.next().is_none() {
                    prune_output()
                } else {
                    usage()
                };
            }
            let query = match args.next() {
                None => None,
                Some(flag) if flag == "--grep" => {
                    let Some(query) = args.next().and_then(|query| query.into_string().ok()) else {
                        return usage();
                    };
                    if args.next().is_some() {
                        return usage();
                    }
                    Some(query)
                }
                _ => return usage(),
            };
            output::prune_if_due();
            return replay_output(&id, query.as_deref());
        }
        Some(arg) if arg == "mcp" => match args.next().as_deref() {
            None => return mcp::dispatch(mcp::ProjectDirectory::WorkingDirectory),
            Some(option) if option == "--require-project-directory" && args.next().is_none() => {
                return mcp::dispatch(mcp::ProjectDirectory::ToolArgument);
            }
            _ => return usage(),
        },
        Some(arg) if arg == "install" => {
            let Some(target) = args
                .next()
                .as_deref()
                .and_then(|name| name.to_str())
                .and_then(install::Target::parse)
            else {
                return usage();
            };
            let option = args.next();
            if args.next().is_some() || option.as_deref().is_some_and(|arg| arg != "--preview") {
                return usage();
            }
            if target.is_preview() && option.is_none() {
                eprintln!("latch: this harness is in preview. Pass --preview to install it.");
                return ExitCode::from(2);
            }
            return match install::install(target) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("latch: could not install integration: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        Some(arg) if arg == "hook" => {
            let harness = args.next();
            let event = args.next();
            if args.next().is_none()
                && let Some(host) = harness
                    .as_deref()
                    .and_then(|name| hooks::Harness::parse(name, event.as_deref()))
            {
                return hooks::dispatch(host);
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

fn initialize_config() -> ExitCode {
    match config::initialize(Path::new(".")) {
        Ok(()) => {
            println!("Created latch.json.");
            ExitCode::SUCCESS
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            eprintln!("latch: latch.json already exists");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("latch: could not create latch.json: {error}");
            ExitCode::FAILURE
        }
    }
}

fn prune_output() -> ExitCode {
    match output::prune_expired() {
        Ok(removed) => {
            println!("Removed {removed} expired output files.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("latch: could not prune output: {error}");
            ExitCode::FAILURE
        }
    }
}

fn replay_output(id: &str, query: Option<&str>) -> ExitCode {
    match output::replay(
        Path::new("."),
        id,
        query,
        io::stdout().lock(),
        io::stderr().lock(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("latch: {error}");
            ExitCode::FAILURE
        }
    }
}

fn explain(selected: config::HookName) -> ExitCode {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let directory = env::current_dir()?;
        let config::LoadedHook {
            config_path,
            entries,
        } = config::load_with_source(&directory, selected)?;
        let mut response = Map::from_iter([
            ("config".into(), json!(config_path)),
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
        ("failure_output".into(), json!(entry.failure_output)),
        ("tail_lines".into(), json!(entry.tail_lines)),
        ("rtk_filter".into(), json!(entry.rtk_filter)),
        ("commands".into(), json!(entry.commands)),
        ("paths".into(), json!(entry.filters.paths)),
        ("extensions".into(), json!(entry.filters.extensions)),
    ])
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}
