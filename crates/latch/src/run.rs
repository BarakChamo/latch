//! Runs configured commands sequentially and preserves their individual outcomes.

use crate::config::{FailureOutput, HookEntry, OnFailure, OnSuccess, RtkFilter, TailLines};
use crate::output::{self, PREVIEW_BYTES};
use crate::rtk;
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
    failure_output: FailureOutput,
    tail_lines: TailLines,
    rtk_filter: RtkFilter,
}

#[derive(Clone, Copy)]
enum Preview {
    Excerpt,
    Compact,
    Tail(usize),
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
    pub(crate) fn message(&self, project: &Path) -> String {
        let mut message = String::new();
        let mut saved_log = false;
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
            if let Ok(output) = &entry.outcome
                && entry.append_preview(project, &mut message, output)
            {
                match output::save(project, &output.stdout, &output.stderr) {
                    Ok(path) => {
                        saved_log = true;
                        writeln!(message, "Saved log: {}", path.display())
                    }
                    Err(error) => {
                        writeln!(message, "latch: could not save full output: {error}")
                    }
                }
                .expect("writing to a String cannot fail");
            }
        }
        if saved_log {
            message.push_str("For missing details, pass the log path and a literal query to Latch search_output, or search the file directly.\n");
        }
        message
    }
}

impl Entry {
    fn append_preview(&self, project: &Path, message: &mut String, output: &Output) -> bool {
        if output.stdout.is_empty() && output.stderr.is_empty() {
            return false;
        }
        let style = match (output.status.success(), self.failure_output) {
            (false, FailureOutput::Rtk) => {
                let filter = self.rtk_filter.as_str();
                match rtk::filter(project, filter, output) {
                    Ok(preview) => {
                        writeln!(message, "RTK ({filter}):\n{preview}")
                            .expect("writing to a String cannot fail");
                        let missing_diagnostic = String::from_utf8_lossy(&output.stderr)
                            .lines()
                            .map(str::trim)
                            .any(|line| looks_like_diagnostic(line) && !preview.contains(line));
                        if missing_diagnostic {
                            append_stream(message, "stderr", &output.stderr, Preview::Compact);
                        }
                    }
                    Err(error) => {
                        writeln!(
                            message,
                            "RTK preview unavailable ({error}); using compact output."
                        )
                        .expect("writing to a String cannot fail");
                        append_stream(message, "stdout", &output.stdout, Preview::Compact);
                        append_stream(message, "stderr", &output.stderr, Preview::Compact);
                    }
                }
                return true;
            }
            (false, FailureOutput::Compact) => Preview::Compact,
            (false, FailureOutput::Tail) => Preview::Tail(self.tail_lines.get()),
            _ => Preview::Excerpt,
        };
        let stdout_omitted = append_stream(message, "stdout", &output.stdout, style);
        let stderr_omitted = append_stream(message, "stderr", &output.stderr, style);
        !output.status.success() && (stdout_omitted || stderr_omitted)
    }

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

// Returns whether the preview mode omitted content that needs a retrieval ID.
fn append_stream(message: &mut String, label: &str, bytes: &[u8], style: Preview) -> bool {
    const LIMIT: usize = 2_000;
    if bytes.is_empty() {
        return false;
    }
    match style {
        Preview::Tail(lines) => {
            writeln!(message, "{label}:").expect("writing to a String cannot fail");
            return append_tail(message, bytes, lines);
        }
        Preview::Compact if bytes.len() > PREVIEW_BYTES => {
            writeln!(message, "{label}:").expect("writing to a String cannot fail");
            append_compact(message, &String::from_utf8_lossy(bytes), bytes.len());
            return true;
        }
        _ => {}
    }
    let excerpt = String::from_utf8_lossy(&bytes[..bytes.len().min(LIMIT)]);
    writeln!(message, "{label}:\n{excerpt}").expect("writing to a String cannot fail");
    if bytes.len() > LIMIT {
        message.push_str("[output truncated]\n");
    }
    bytes.len() > LIMIT
}

fn append_tail(message: &mut String, bytes: &[u8], count: usize) -> bool {
    const LINE_BYTES: usize = 240;
    let text = String::from_utf8_lossy(bytes);
    let mut reversed = text.lines().rev();
    let lines: Vec<_> = reversed.by_ref().take(count).collect();
    let mut omitted = reversed.next().is_some() || matches!(text, std::borrow::Cow::Owned(_));
    for line in lines.into_iter().rev() {
        let mut start = line.len().saturating_sub(LINE_BYTES);
        while !line.is_char_boundary(start) {
            start += 1;
        }
        if start > 0 {
            omitted = true;
            message.push('…');
        }
        writeln!(message, "{}", &line[start..]).expect("writing to a String cannot fail");
    }
    omitted
}

fn append_compact(message: &mut String, text: &str, original_bytes: usize) {
    // Walk the captured output once and reserve room for end-of-run summaries.
    let diagnostics = text
        .lines()
        .map(str::trim)
        .filter(|line| looks_like_diagnostic(line));
    let mut selected = Vec::with_capacity(8);
    for line in diagnostics {
        if !selected.contains(&line) {
            selected.push(line);
        }
        if selected.len() == 6 {
            break;
        }
    }

    let end: Vec<_> = text
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(if selected.is_empty() { 8 } else { 2 })
        .collect();
    for line in end.into_iter().rev() {
        if !selected.contains(&line) {
            selected.push(line);
        }
    }
    for line in selected {
        let preview = utf8_prefix(line, 140);
        let suffix = if preview.len() < line.len() {
            "…"
        } else {
            ""
        };
        writeln!(message, "{preview}{suffix}").expect("writing to a String cannot fail");
    }
    writeln!(message, "[output compacted from {original_bytes} bytes]")
        .expect("writing to a String cannot fail");
}

fn looks_like_diagnostic(line: &str) -> bool {
    let prefix = utf8_prefix(line, 240).to_ascii_lowercase();
    prefix.starts_with("fail ")
        || prefix.starts_with("fail:")
        || prefix.starts_with("failed ")
        || prefix.starts_with("--- fail:")
        || prefix.starts_with("error")
        || prefix.starts_with("panic")
        || prefix.contains("panicked at ")
        || prefix.contains("... failed")
        || prefix.contains("assertion failed")
        || prefix.contains("error:")
        || prefix.contains(" error ts")
}

fn utf8_prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
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
                failure_output: entry.failure_output,
                tail_lines: entry.tail_lines,
                rtk_filter: entry.rtk_filter.clone(),
            });
        }
        report.failed |= failed;
        if failed && entry.on_failure == OnFailure::Stop {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_preserves_blank_lines_and_shortens_long_unicode_lines_from_the_left() {
        let raw = format!("first\n\n{} suffix", "火".repeat(200));
        let mut preview = String::new();
        assert!(append_tail(&mut preview, raw.as_bytes(), 2));
        assert!(preview.starts_with("\n…火"));
        assert!(preview.ends_with(" suffix\n"));
        assert_eq!(preview.lines().count(), 2);
        assert!(!preview.contains("first"));
        preview.clear();
        assert!(!append_tail(&mut preview, b"short\n", 2));
        assert_eq!(preview, "short\n");
    }

    #[test]
    fn compact_output_deduplicates_diagnostics_and_bounds_long_unicode_lines() {
        let source = format!(
            "{}{}\nfinal line",
            "error: duplicate\n".repeat(100),
            "火".repeat(200)
        );
        let mut message = String::new();
        append_stream(&mut message, "stderr", source.as_bytes(), Preview::Compact);
        assert_eq!(message.matches("error: duplicate").count(), 1);
        assert!(message.contains("火…\nfinal line"));
        assert!(message.len() < 1_200);
    }

    #[test]
    fn compact_output_falls_back_to_the_tail_for_unknown_formats() {
        let source = format!(
            "starting...\n{}step 42: could not bind port\n",
            "working\n".repeat(500)
        );
        let mut message = String::new();
        append_stream(&mut message, "stdout", source.as_bytes(), Preview::Compact);
        assert!(message.contains("step 42: could not bind port"));
        assert!(!message.contains("starting..."));
        assert!(message.contains("[output compacted from"));
    }
}
