//! Compares Git-visible worktree contents against a fixed commit across a turn.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Serialize)]
pub(crate) struct Baseline {
    root: PathBuf,
    commit: String,
    files: Snapshot,
}

impl Baseline {
    /// Records the worktree, its reference commit, and the current content fingerprint.
    pub(crate) fn capture(directory: &Path) -> io::Result<Self> {
        let root = worktree(directory)?;
        let commit = git(&root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
        let commit = std::str::from_utf8(without_newline(&commit))
            .map_err(io::Error::other)?
            .to_owned();
        let files = snapshot(&root, &commit)?;
        Ok(Self {
            root,
            commit,
            files,
        })
    }

    /// Lists paths whose fingerprints differ from the original snapshot. Errors mean unknown.
    pub(crate) fn changed_paths(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        let root = worktree(directory)?;
        if root != self.root {
            return Err(io::Error::other(
                "worktree changed during baseline comparison",
            ));
        }
        let current = snapshot(&root, &self.commit)?;
        let mut changed = Vec::new();
        for (path, hash) in current.iter() {
            if self.files.get(path) != Some(hash) {
                changed.push(PathBuf::from(path_bytes(path)?));
            }
        }
        for (path, _) in self.files.iter() {
            if current.get(path).is_none() {
                changed.push(PathBuf::from(path_bytes(path)?));
            }
        }
        Ok(changed)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct Snapshot(Vec<(Vec<u8>, [u8; 32])>);

impl Snapshot {
    fn iter(&self) -> impl Iterator<Item = &(Vec<u8>, [u8; 32])> {
        self.0.iter()
    }

    fn get(&self, path: &[u8]) -> Option<&[u8; 32]> {
        let index = self
            .0
            .binary_search_by(|(name, _)| name.as_slice().cmp(path))
            .ok()?;
        Some(&self.0[index].1)
    }
}

fn git(directory: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(directory)
        .args(["--no-optional-locks", "-c", "core.fsmonitor=false"])
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

/// Resolves the Git worktree that contains the given directory.
pub(crate) fn worktree(directory: &Path) -> io::Result<PathBuf> {
    let output = git(directory, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(path_bytes(without_newline(&output))?))
}

fn without_newline(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\n").unwrap_or(bytes)
}

#[cfg(unix)]
fn path_bytes(bytes: &[u8]) -> io::Result<OsString> {
    use std::os::unix::ffi::OsStringExt;
    Ok(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_bytes(bytes: &[u8]) -> io::Result<OsString> {
    Ok(OsString::from(
        std::str::from_utf8(bytes).map_err(io::Error::other)?,
    ))
}

fn snapshot(root: &Path, commit: &str) -> io::Result<Snapshot> {
    let tracked = git(
        root,
        &[
            "diff",
            "--name-only",
            "-z",
            "--no-relative",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=none",
            commit,
            "--",
        ],
    )?;
    let listing = git(
        root,
        &[
            "ls-files",
            "-v",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let mut untracked = Vec::new();
    for entry in listing
        .split(|&byte| byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        match entry {
            [b'?', b' ', path @ ..] => {
                // Without --directory, Git emits embedded checkouts as opaque directory
                // entries. Their contents belong to a different worktree.
                if !path.ends_with(b"/") {
                    untracked.push(path);
                }
            }
            // Git can omit edits to paths flagged assume-unchanged or skip-worktree.
            [tag, b' ', ..] if tag.is_ascii_lowercase() || *tag == b'S' => {
                return Err(io::Error::other("Git index can hide worktree edits"));
            }
            [_, b' ', ..] => {}
            _ => return Err(io::Error::other("unexpected Git index listing")),
        }
    }
    let paths: BTreeSet<_> = tracked
        .split(|&byte| byte == 0)
        .chain(untracked)
        .filter(|path| !path.is_empty())
        .collect();

    let mut files = Vec::with_capacity(paths.len());
    for name in paths {
        let relative = PathBuf::from(path_bytes(name)?);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(io::Error::other("Git returned a non-relative path"));
        }
        files.push((name.to_vec(), file_fingerprint(root, &relative)?));
    }
    Ok(Snapshot(files))
}

fn file_fingerprint(root: &Path, relative: &Path) -> io::Result<[u8; 32]> {
    let mut digest = Sha256::new();
    // A tracked directory may have been replaced by a symlink or a regular file.
    // Inspect parents from the root so hashing never walks through that replacement.
    let mut path = root.to_path_buf();
    let mut parts = relative.iter().peekable();
    while let Some(part) = parts.next() {
        path.push(part);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Sha256::digest(b"missing").into());
            }
            Err(error) => return Err(error),
        };
        if parts.peek().is_some() {
            if !metadata.is_dir() {
                return Ok(Sha256::digest(b"missing").into());
            }
        } else if metadata.is_symlink() {
            digest.update(b"symlink");
            digest.update(fs::read_link(&path)?.as_os_str().as_encoded_bytes());
        } else if metadata.is_file() {
            digest.update(b"file");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                digest.update([u8::from(metadata.permissions().mode() & 0o100 != 0)]);
            }
            io::copy(&mut File::open(&path)?, &mut digest)?;
        } else {
            return Err(io::Error::other("unsupported Git path type"));
        }
    }
    Ok(digest.finalize().into())
}
