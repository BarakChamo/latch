//! Stores omitted command output in a bounded, per-project local cache.

use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, DirBuilder, DirEntry, File};
use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub(crate) const PREVIEW_BYTES: usize = 1_200;
pub(crate) const RECORD_BYTES: usize = 16 * 1024 * 1024;
const PROJECT_BYTES: u64 = 64 * 1024 * 1024;
const PROJECT_RECORDS: usize = 64;
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const SWEEP_STAMP: &str = ".last-prune";
const PENDING_PREFIX: &str = ".pending-";
const LEGACY_HEADER_BYTES: usize = 8;
const MAX_HEADER_BYTES: usize = 64;
const LOG_SUFFIX: &str = ".log";
const LOG_SEPARATOR: &[u8] = b"\n\nstderr:\n";
const ID_LENGTH: usize = 24;
const MATCH_LIMIT: usize = 40;
const MATCH_BYTES: usize = 240;

/// Saves both original streams and returns the path to the readable log.
pub(crate) fn save(project: &Path, stdout: &[u8], stderr: &[u8]) -> io::Result<PathBuf> {
    if stdout.len().saturating_add(stderr.len()) > RECORD_BYTES {
        return Err(io::Error::other("output exceeds the 16 MiB storage limit"));
    }
    save_in(&project_cache(project)?, stdout, stderr)
}

fn cache_root() -> io::Result<PathBuf> {
    let base = env::var_os("XDG_STATE_HOME")
        .filter(|path| Path::new(path).is_absolute())
        .map(PathBuf::from)
        .or_else(|| {
            #[cfg(windows)]
            if let Some(path) =
                env::var_os("LOCALAPPDATA").filter(|path| Path::new(path).is_absolute())
            {
                return Some(PathBuf::from(path));
            }
            env::home_dir().map(|home| home.join(".local").join("state"))
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no user state directory"))?;
    Ok(base.join("latch").join("output-v1"))
}

fn project_cache(project: &Path) -> io::Result<PathBuf> {
    let project = project.canonicalize()?;
    let key = Sha256::digest(project.as_os_str().as_encoded_bytes());
    Ok(cache_root()?.join(format!("{key:x}")))
}

fn save_in(directory: &Path, stdout: &[u8], stderr: &[u8]) -> io::Result<PathBuf> {
    let mut builder = DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.recursive(true).create(directory)?;
    let mut pending = tempfile::Builder::new()
        .prefix(PENDING_PREFIX)
        .rand_bytes(ID_LENGTH)
        .tempfile_in(directory)?;
    writeln!(pending, "stdout ({} bytes):", stdout.len())?;
    pending.write_all(stdout)?;
    pending.write_all(LOG_SEPARATOR)?;
    pending.write_all(stderr)?;
    let name = pending
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(PENDING_PREFIX))
        .expect("tempfile generates an ASCII name with the requested prefix");
    let filename = format!("r-{name}{LOG_SUFFIX}");
    let path = directory.join(&filename);
    pending.persist_noclobber(&path)?;
    prune(directory, &filename);
    Ok(path)
}

fn valid_log_name(name: &str) -> bool {
    valid_id(name.strip_suffix(LOG_SUFFIX).unwrap_or(name))
}

fn valid_id(id: &str) -> bool {
    id.strip_prefix("r-").is_some_and(valid_suffix)
}

fn valid_suffix(suffix: &str) -> bool {
    suffix.len() == ID_LENGTH && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn expired(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified).is_ok_and(|age| age >= MAX_AGE)
}

/// Periodically clears expired files while Latch is in use. Cleanup failures do
/// not affect command results and can be inspected with an explicit prune.
pub(crate) fn prune_if_due() {
    let Ok(root) = cache_root() else {
        return;
    };
    let now = SystemTime::now();
    let recently_swept = root
        .join(SWEEP_STAMP)
        .symlink_metadata()
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| {
            now.duration_since(modified)
                .is_ok_and(|elapsed| elapsed < SWEEP_INTERVAL)
        });
    if !recently_swept {
        let _ = sweep(&root, now);
    }
}

/// Removes expired entries from every project in the current user's cache.
pub(crate) fn prune_expired() -> io::Result<usize> {
    sweep(&cache_root()?, SystemTime::now())
}

fn sweep(root: &Path, now: SystemTime) -> io::Result<usize> {
    match root.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(io::Error::other("output cache is not a directory")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    }
    let mut removed = 0;
    for project in fs::read_dir(root)? {
        let project = project?;
        let name = project.file_name();
        let is_project = name.to_str().is_some_and(|name| {
            name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
        if is_project && project.file_type()?.is_dir() {
            removed += sweep_project(&project.path(), now)?;
        }
    }
    tempfile::NamedTempFile::new_in(root)?.persist(root.join(SWEEP_STAMP))?;
    Ok(removed)
}

fn sweep_project(project: &Path, now: SystemTime) -> io::Result<usize> {
    let mut removed = 0;
    for file in fs::read_dir(project)? {
        let file = file?;
        let name = file.file_name();
        let is_cache_file = name.to_str().is_some_and(|name| {
            valid_log_name(name) || name.strip_prefix(PENDING_PREFIX).is_some_and(valid_suffix)
        });
        if !is_cache_file {
            continue;
        }
        match remove_if_expired(&file, now) {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(removed)
}

fn remove_if_expired(file: &DirEntry, now: SystemTime) -> io::Result<bool> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || !expired(metadata.modified()?, now) {
        return Ok(false);
    }
    fs::remove_file(file.path())?;
    Ok(true)
}

fn cached_record(entry: DirEntry) -> Option<(DirEntry, SystemTime, u64)> {
    if !entry.file_name().to_str().is_some_and(valid_log_name) || !entry.file_type().ok()?.is_file()
    {
        return None;
    }
    let info = entry.metadata().ok()?;
    Some((entry, info.modified().ok()?, info.len()))
}

fn prune(directory: &Path, newest: &str) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let now = SystemTime::now();
    let mut records = Vec::new();
    for entry in entries.flatten() {
        if let Some(record) = cached_record(entry) {
            records.push(record);
        }
    }
    records.sort_unstable_by_key(|(entry, modified, _)| {
        (entry.file_name() != newest, std::cmp::Reverse(*modified))
    });
    let mut used = 0;
    let mut kept = 0;
    for (entry, modified, size) in records {
        if expired(modified, now) || kept >= PROJECT_RECORDS || size > PROJECT_BYTES - used {
            // Cleanup is best effort. Another process may already have removed it.
            let _ = fs::remove_file(entry.path());
        } else {
            used += size;
            kept += 1;
        }
    }
}

#[derive(Debug)]
struct Record {
    bytes: Vec<u8>,
    stdout: Range<usize>,
    stderr_start: usize,
}

impl Record {
    fn streams(&self) -> (&[u8], &[u8]) {
        (
            &self.bytes[self.stdout.clone()],
            &self.bytes[self.stderr_start..],
        )
    }
}

fn read_record(directory: &Path, id: &str) -> io::Result<Record> {
    let log = directory.join(format!("{id}{LOG_SUFFIX}"));
    let (path, metadata, legacy) = match log.symlink_metadata() {
        Ok(metadata) => (log, metadata, false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let previous = directory.join(id);
            let metadata = previous.symlink_metadata()?;
            (previous, metadata, true)
        }
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || expired(metadata.modified()?, SystemTime::now()) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "output not found or expired",
        ));
    }
    let mut bytes = Vec::new();
    let limit = RECORD_BYTES + MAX_HEADER_BYTES + LOG_SEPARATOR.len();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    let (stdout, stderr_start) = if legacy {
        legacy_positions(&bytes)
    } else {
        log_positions(&bytes)
    }
    .filter(|(stdout, stderr)| stdout.len() + bytes.len() - stderr <= RECORD_BYTES)
    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid saved output"))?;
    Ok(Record {
        bytes,
        stdout,
        stderr_start,
    })
}

fn log_positions(bytes: &[u8]) -> Option<(Range<usize>, usize)> {
    let header_end = bytes
        .iter()
        .take(MAX_HEADER_BYTES)
        .position(|&byte| byte == b'\n')?;
    let header = std::str::from_utf8(&bytes[..header_end]).ok()?;
    let length: usize = header
        .strip_prefix("stdout (")?
        .strip_suffix(" bytes):")?
        .parse()
        .ok()?;
    let stdout_start = header_end + 1;
    let stdout_end = stdout_start.checked_add(length)?;
    let stderr_start = stdout_end.checked_add(LOG_SEPARATOR.len())?;
    (bytes.get(stdout_end..stderr_start)? == LOG_SEPARATOR)
        .then_some((stdout_start..stdout_end, stderr_start))
}

fn legacy_positions(bytes: &[u8]) -> Option<(Range<usize>, usize)> {
    let length = u64::from_le_bytes(*bytes.first_chunk::<LEGACY_HEADER_BYTES>()?);
    let split = LEGACY_HEADER_BYTES.checked_add(usize::try_from(length).ok()?)?;
    (split <= bytes.len()).then_some((LEGACY_HEADER_BYTES..split, split))
}

/// Replays raw streams or a bounded literal search within a single saved command.
pub(crate) fn replay(
    project: &Path,
    reference: &str,
    query: Option<&str>,
    mut stdout: impl Write,
    mut stderr: impl Write,
) -> io::Result<()> {
    let reference = Path::new(reference);
    let (filename, id) = reference
        .file_name()
        .and_then(|name| {
            let name = name.to_str()?;
            let id = name.strip_suffix(LOG_SUFFIX).unwrap_or(name);
            let supported =
                valid_id(id) && (reference.is_absolute() || reference == Path::new(name));
            supported.then_some((name, id))
        })
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid output ID or log path")
        })?;
    if let Some(query) = query {
        if query.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "search term must not be empty",
            ));
        }
        if query.len() > MATCH_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "search term exceeds 240 bytes",
            ));
        }
    }
    let directory = project_cache(project)?;
    if reference.is_absolute() && reference != directory.join(filename) {
        return Err(output_not_found());
    }
    let record = read_record(&directory, id).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            output_not_found()
        } else {
            error
        }
    })?;
    let (original_stdout, original_stderr) = record.streams();
    if let Some(query) = query {
        write_matches(
            original_stdout,
            original_stderr,
            query,
            &mut stdout,
            &mut stderr,
        )?;
    } else {
        stdout.write_all(original_stdout)?;
        stderr.write_all(original_stderr)?;
    }
    stdout.flush()?;
    stderr.flush()
}

fn output_not_found() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "output not found in this project or expired",
    )
}

fn write_matches(
    stdout: &[u8],
    stderr: &[u8],
    query: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<()> {
    // Give each stream a budget so verbose stdout cannot hide a stderr match.
    let matches =
        write_stream_matches(stdout, query, out)? + write_stream_matches(stderr, query, err)?;
    if matches == 0 {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no matching lines"));
    }
    Ok(())
}

fn write_stream_matches(bytes: &[u8], query: &str, mut out: impl Write) -> io::Result<usize> {
    let mut count = 0;
    for (line, offset) in String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| line.find(query).map(|offset| (line, offset)))
    {
        if count == MATCH_LIMIT {
            writeln!(out, "[additional matching lines omitted]")?;
            break;
        }
        // Include the literal even when it occurs near the end of a very long line.
        let start = if line.len() > MATCH_BYTES {
            offset.saturating_sub((MATCH_BYTES - query.len()) / 2)
        } else {
            0
        };
        let start = char_boundary(line, start);
        let end =
            char_boundary(line, (start + MATCH_BYTES).min(line.len())).max(offset + query.len());
        let leading = if start > 0 { "…" } else { "" };
        let trailing = if end < line.len() { "…" } else { "" };
        writeln!(out, "{leading}{}{trailing}", &line[start..end])?;
        count += 1;
    }
    Ok(count)
}

fn char_boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(path: &Path) -> &str {
        path.file_stem().unwrap().to_str().unwrap()
    }

    #[test]
    fn expired_output_is_rejected_and_removed_by_the_next_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let expired_file = save_in(path, b"expired", b"").unwrap();
        File::options()
            .write(true)
            .open(&expired_file)
            .unwrap()
            .set_modified(SystemTime::now() - MAX_AGE - Duration::from_secs(2))
            .unwrap();
        let error = read_record(path, id(&expired_file)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        let active = save_in(path, b"fresh", b"").unwrap();
        assert!(!expired_file.exists());
        assert!(active.is_file());
    }

    #[test]
    fn retention_limits_record_count_and_keeps_the_newest_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        for _ in 0..PROJECT_RECORDS {
            save_in(path, b"stored", b"").unwrap();
        }
        let latest = save_in(path, b"latest", b"").unwrap();
        assert_eq!(fs::read_dir(path).unwrap().count(), PROJECT_RECORDS);
        assert!(latest.is_file());
    }

    #[test]
    fn retention_prunes_bytes_and_rejects_oversize_output() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        for _ in 0..4 {
            let log = save_in(path, b"", b"").unwrap();
            File::options()
                .write(true)
                .open(log)
                .unwrap()
                .set_len((RECORD_BYTES + MAX_HEADER_BYTES) as u64)
                .unwrap();
        }
        let latest = save_in(path, b"latest", b"").unwrap();
        let stored: u64 = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().metadata().unwrap().len())
            .sum();
        assert!(stored <= PROJECT_BYTES);
        assert!(latest.is_file());
        let error = save(path, &vec![0; RECORD_BYTES], b"one more byte").unwrap_err();
        assert!(error.to_string().contains("16 MiB storage limit"));
    }

    #[test]
    fn concurrent_saves_publish_distinct_complete_records() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let ids = std::thread::scope(|scope| {
            let saves: Vec<_> = (0..8)
                .map(|index| scope.spawn(move || save_in(path, &[index], b"stderr").unwrap()))
                .collect();
            saves
                .into_iter()
                .map(|save| save.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            ids.iter().collect::<std::collections::HashSet<_>>().len(),
            8
        );
        for (index, log) in ids.iter().enumerate() {
            let record = read_record(path, id(log)).unwrap();
            let (stdout, stderr) = record.streams();
            assert_eq!(stdout, &[index as u8]);
            assert_eq!(stderr, b"stderr");
        }
    }

    #[test]
    fn literal_search_bounds_results_and_centers_the_match_with_utf8_intact() {
        let line = format!("{} target {}\n", "火".repeat(200), "尾".repeat(200));
        let mut out = Vec::new();
        let count =
            write_stream_matches(line.repeat(MATCH_LIMIT + 3).as_bytes(), "target", &mut out)
                .unwrap();
        let result = String::from_utf8(out).unwrap();
        assert_eq!(count, MATCH_LIMIT);
        assert_eq!(
            result
                .lines()
                .filter(|line| line.contains("target"))
                .count(),
            MATCH_LIMIT
        );
        assert!(result.lines().all(|line| line.len() <= MATCH_BYTES + 6));
        assert!(result.ends_with("[additional matching lines omitted]\n"));
    }

    #[test]
    fn corrupt_lengths_return_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let log = save_in(path, b"data", b"").unwrap();
        fs::write(&log, b"stdout (18446744073709551615 bytes):\ninvalid").unwrap();
        let error = read_record(path, id(&log)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn earlier_binary_logs_remain_retrievable() {
        let directory = tempfile::tempdir().unwrap();
        let id = "r-0123456789abcdefghijklmn";
        let mut binary = 3u64.to_le_bytes().to_vec();
        binary.extend_from_slice(b"outerr");
        fs::write(directory.path().join(id), binary).unwrap();
        let record = read_record(directory.path(), id).unwrap();
        assert_eq!(record.streams(), (b"out".as_slice(), b"err".as_slice()));
    }
}
