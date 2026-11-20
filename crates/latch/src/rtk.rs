//! Applies a user-installed RTK stdin filter to captured command output.

use crate::output::RECORD_BYTES;
use std::io::{self, Read, Seek, Write};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PREVIEW_LIMIT: u64 = 2_000;

/// Runs only the filter. The caller retains ownership of the check's status and streams.
pub(crate) fn filter(project: &Path, name: &str, captured: &Output) -> io::Result<String> {
    let stdout = &captured.stdout;
    let stderr = &captured.stderr;
    if stdout.len().saturating_add(stderr.len()) > RECORD_BYTES {
        return Err(io::Error::other("output exceeds the 16 MiB filter limit"));
    }

    // File-backed streams keep the timeout independent of pipe readers or writers.
    let mut input = tempfile::tempfile()?;
    input.write_all(stdout)?;
    if !stdout.is_empty() && !stderr.is_empty() && !stdout.ends_with(b"\n") {
        input.write_all(b"\n")?;
    }
    input.write_all(stderr)?;
    input.rewind()?;
    let mut preview = tempfile::tempfile()?;
    let mut child = Command::new("rtk")
        .args(["pipe", "--filter", name])
        .current_dir(project)
        .stdin(input)
        .stdout(preview.try_clone()?)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(error.kind(), "rtk was not found on PATH")
            } else {
                error
            }
        })?;
    wait_for_filter(&mut child)?;

    preview.rewind()?;
    let mut bytes = Vec::new();
    preview.take(PREVIEW_LIMIT + 1).read_to_end(&mut bytes)?;
    let truncated = bytes.len() as u64 > PREVIEW_LIMIT;
    bytes.truncate(PREVIEW_LIMIT as usize);
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim();
    if text.is_empty() {
        return Err(io::Error::other("rtk returned an empty preview"));
    }
    if truncated {
        Ok(format!("{text}\n[RTK preview truncated]"))
    } else {
        Ok(text.to_string())
    }
}

fn wait_for_filter(child: &mut Child) -> io::Result<()> {
    let deadline = Instant::now() + TIMEOUT;
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(io::Error::other(format!("rtk filter failed with {status}")));
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                break Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "rtk filter timed out",
                ));
            }
            Err(error) => break Err(error),
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    result
}
