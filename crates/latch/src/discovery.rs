//! Converts native project scripts for Latch events into runner commands.

use crate::config::HookName;
use serde::Deserialize;
use std::{collections::BTreeMap, fmt, fs, io, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageJson {
    package_manager: Option<String>,
    #[serde(default)]
    scripts: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Pyproject {
    project: Option<PythonProject>,
}

#[derive(Deserialize)]
struct PythonProject {
    #[serde(default)]
    scripts: BTreeMap<String, String>,
}

#[derive(Debug)]
pub(crate) enum DiscoveryError {
    Read(&'static str, io::Error),
    PackageJson(serde_json::Error),
    Pyproject(toml::de::Error),
    UnsupportedManager(String),
    ConflictingLockfiles,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(file, error) => write!(formatter, "could not read {file}: {error}"),
            Self::PackageJson(error) => write!(formatter, "could not parse package.json: {error}"),
            Self::Pyproject(error) => write!(formatter, "could not parse pyproject.toml: {error}"),
            Self::UnsupportedManager(manager) => {
                write!(
                    formatter,
                    "unsupported packageManager in package.json: {manager:?}"
                )
            }
            Self::ConflictingLockfiles => write!(
                formatter,
                "multiple JavaScript package manager lockfiles found; set packageManager in package.json"
            ),
        }
    }
}

/// Reads local manifests and returns opted-in commands in JavaScript, then Python order.
pub(crate) fn commands(
    directory: &Path,
    hook: HookName,
) -> Result<Vec<Vec<String>>, DiscoveryError> {
    let (javascript, python) = match hook {
        HookName::AfterEdit => ("hook:afteredit", "hook-afteredit"),
        HookName::TurnEnd => ("hook:turnend", "hook-turnend"),
    };
    let mut commands = Vec::new();
    if let Some(package) = manifest(directory, "package.json")? {
        let package: PackageJson =
            serde_json::from_slice(&package).map_err(DiscoveryError::PackageJson)?;
        if package.scripts.contains_key(javascript) {
            let manager = package_manager(directory, package.package_manager.as_deref())?;
            commands.push([manager, "run", javascript].map(String::from).into());
        }
    }
    if let Some(project) = manifest(directory, "pyproject.toml")? {
        let project: Pyproject = toml::from_slice(&project).map_err(DiscoveryError::Pyproject)?;
        if project
            .project
            .is_some_and(|project| project.scripts.contains_key(python))
        {
            commands.push(["uv", "run", python].map(String::from).into());
        }
    }
    Ok(commands)
}

fn manifest(directory: &Path, file: &'static str) -> Result<Option<Vec<u8>>, DiscoveryError> {
    match fs::read(directory.join(file)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(DiscoveryError::Read(file, error)),
    }
}

fn package_manager(
    directory: &Path,
    declared: Option<&str>,
) -> Result<&'static str, DiscoveryError> {
    let managers = [
        ("npm", &["package-lock.json", "npm-shrinkwrap.json"][..]),
        ("pnpm", &["pnpm-lock.yaml"][..]),
        ("yarn", &["yarn.lock"][..]),
        ("bun", &["bun.lock", "bun.lockb"][..]),
    ];
    if let Some(declared) = declared {
        let name = declared.split_once('@').map_or(declared, |(name, _)| name);
        return managers
            .iter()
            .map(|&(manager, _)| manager)
            .find(|&manager| manager == name)
            .ok_or_else(|| DiscoveryError::UnsupportedManager(declared.into()));
    }
    let mut detected = None;
    for (manager, lockfiles) in managers {
        for &lockfile in lockfiles {
            let exists = directory
                .join(lockfile)
                .try_exists()
                .map_err(|error| DiscoveryError::Read(lockfile, error))?;
            if exists {
                if detected.replace(manager).is_some() {
                    return Err(DiscoveryError::ConflictingLockfiles);
                }
                break;
            }
        }
    }
    Ok(detected.unwrap_or("npm"))
}
