//! Runs configured commands sequentially and preserves their individual outcomes.

use crate::config::{HookEntry, OnFailure, OnSuccess};
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Output};

pub(crate) struct Report {
    failed: bool,
    entries: Vec<Entry>,
}

struct Entry {
    number: usize,
    program: String,
    outcome: Result<Output, io::Error>,
}

impl Report {
    pub(crate) fn failed(&self) -> bool {
        self.failed
    }

    /// Replays configured output on its original streams for direct CLI use.
    pub(crate) fn replay(&self, mut stdout: impl Write, mut stderr: impl Write) -> io::Result<()> {
        for entry in &self.entries {
            if let Some(diagnostic) = entry.diagnostic() {
                writeln!(stderr, "{diagnostic}")?;
            }
            if let Ok(output) = &entry.outcome {
                stdout.write_all(&output.stdout)?;
                stderr.write_all(&output.stderr)?;
            }
        }
        stdout.flush()?;
        stderr.flush()
    }

    /// Renders a single report with each failure and each requested success in execution order.
    pub(crate) fn message(&self) -> String {
        let mut message = String::new();
        for entry in &self.entries {
            match entry.diagnostic() {
                Some(diagnostic) => writeln!(message, "{diagnostic}"),
                None => writeln!(
                    message,
                    "latch: command {} ({:?}) succeeded",
                    entry.number, entry.program
                ),
            }
            .expect("writing to a String cannot fail");
            if let Ok(output) = &entry.outcome {
                append_stream(&mut message, "stdout", &output.stdout);
                append_stream(&mut message, "stderr", &output.stderr);
            }
        }
        message
    }
}

impl Entry {
    fn diagnostic(&self) -> Option<String> {
        let number = self.number;
        let program = &self.program;
        match &self.outcome {
            Ok(output) if output.status.success() => None,
            Ok(output) => Some(format!(
                "latch: command {number} ({program:?}) failed: {}",
                output.status
            )),
            Err(error) => Some(format!(
                "latch: could not run command {number} ({program:?}): {error}"
            )),
        }
    }
}

fn append_stream(message: &mut String, label: &str, bytes: &[u8]) {
    const LIMIT: usize = 2_000;
    if bytes.is_empty() {
        return;
    }
    let excerpt = String::from_utf8_lossy(&bytes[..bytes.len().min(LIMIT)]);
    writeln!(message, "{label}:\n{excerpt}").expect("writing to a String cannot fail");
    if bytes.len() > LIMIT {
        message.push_str("[output truncated]\n");
    }
}

/// Runs entries in order with one report. A launch error counts as a failure.
pub(crate) fn run(directory: &Path, entries: impl IntoIterator<Item = HookEntry>) -> Report {
    let mut report = Report {
        failed: false,
        entries: Vec::new(),
    };
    let mut number = 0;
    for entry in entries {
        run_entry(directory, entry, &mut report, &mut number);
    }
    report
}

fn run_entry(directory: &Path, entry: HookEntry, report: &mut Report, number: &mut usize) {
    for command in entry.commands {
        *number += 1;
        let (program, arguments) = command
            .split_first()
            .expect("validated commands have an executable");
        let outcome = Command::new(program)
            .args(arguments)
            .current_dir(directory)
            .output();
        let failed = outcome
            .as_ref()
            .map_or(true, |output| !output.status.success());
        if failed || entry.on_success == OnSuccess::Output {
            report.entries.push(Entry {
                number: *number,
                program: program.clone(),
                outcome,
            });
        }
        report.failed |= failed;
        if failed && entry.on_failure == OnFailure::Stop {
            break;
        }
    }
}
