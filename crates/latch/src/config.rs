//! Resolves project defaults and hook overrides before commands can run.

use crate::discovery::{self, DiscoveryError};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(rename = "$schema")]
    _schema: Option<String>,
    commands: Option<Vec<Vec<String>>>,
    hooks: Option<Hooks>,
    #[serde(default)]
    defaults: Settings,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hooks {
    turn_end: Option<HookEntries<EntryConfig>>,
    after_edit: Option<HookEntries<EntryConfig>>,
}

pub(crate) enum HookEntries<T> {
    Single(T),
    Multiple(Vec<T>),
}

impl HookEntries<HookEntry> {
    pub(crate) fn needs_comparison(&self) -> bool {
        let entries = match self {
            Self::Single(entry) => std::slice::from_ref(entry),
            Self::Multiple(entries) => entries.as_slice(),
        };
        entries.iter().any(HookEntry::needs_comparison)
    }

    pub(crate) fn into_vec(self) -> Vec<HookEntry> {
        match self {
            Self::Single(entry) => vec![entry],
            Self::Multiple(entries) => entries,
        }
    }
}

impl<'de> Deserialize<'de> for HookEntries<EntryConfig> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = HookEntries<EntryConfig>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a hook object or an array of hook objects")
            }

            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                EntryObject.visit_map(map).map(HookEntries::Single)
            }

            fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = seq.next_element_seed(EntryObject)? {
                    entries.push(entry);
                }
                Ok(HookEntries::Multiple(entries))
            }
        }

        deserializer.deserialize_any(EntriesVisitor)
    }
}

// Serde can also deserialize a struct from a positional sequence. Hook entries require objects.
struct EntryObject;

impl<'de> DeserializeSeed<'de> for EntryObject {
    type Value = EntryConfig;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<EntryConfig, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for EntryObject {
    type Value = EntryConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a hook object")
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        EntryConfig::deserialize(MapAccessDeserializer::new(map))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HookName {
    TurnEnd,
    AfterEdit,
}

impl HookName {
    pub(crate) fn parse(name: &std::ffi::OsStr) -> Option<Self> {
        match name.to_str()? {
            "turn_end" => Some(Self::TurnEnd),
            "after_edit" => Some(Self::AfterEdit),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::TurnEnd => "turn_end",
            Self::AfterEdit => "after_edit",
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryConfig {
    commands: Vec<Vec<String>>,
    paths: Option<Vec<String>>,
    extensions: Option<Vec<String>>,
    when: Option<When>,
    on_success: Option<OnSuccess>,
    on_failure: Option<OnFailure>,
    failure_output: Option<FailureOutput>,
    tail_lines: Option<TailLines>,
    rtk_filter: Option<RtkFilter>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    when: Option<When>,
    on_success: Option<OnSuccess>,
    on_failure: Option<OnFailure>,
    failure_output: Option<FailureOutput>,
    tail_lines: Option<TailLines>,
    rtk_filter: Option<RtkFilter>,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum When {
    WorkspaceChanged,
    Always,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OnSuccess {
    #[default]
    Silent,
    Output,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OnFailure {
    #[default]
    Collect,
    Stop,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FailureOutput {
    #[default]
    Excerpt,
    Compact,
    Tail,
    Rtk,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub(crate) struct RtkFilter(String);

impl Default for RtkFilter {
    fn default() -> Self {
        Self("log".into())
    }
}

impl TryFrom<String> for RtkFilter {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !value.is_empty()
            && value.len() <= 64
            && value.as_bytes()[0].is_ascii_alphanumeric()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            Ok(Self(value))
        } else {
            Err("rtk_filter must be a filter name such as vitest or cargo-test")
        }
    }
}

impl RtkFilter {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(try_from = "usize")]
pub(crate) struct TailLines(usize);

impl Default for TailLines {
    fn default() -> Self {
        Self(20)
    }
}

impl TryFrom<usize> for TailLines {
    type Error = &'static str;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if (1..=100).contains(&value) {
            Ok(Self(value))
        } else {
            Err("tail_lines must be between 1 and 100")
        }
    }
}

impl TailLines {
    pub(crate) fn get(self) -> usize {
        self.0
    }
}

pub(crate) struct HookEntry {
    pub(crate) when: When,
    pub(crate) on_success: OnSuccess,
    pub(crate) on_failure: OnFailure,
    pub(crate) failure_output: FailureOutput,
    pub(crate) tail_lines: TailLines,
    pub(crate) rtk_filter: RtkFilter,
    pub(crate) filters: PathFilters,
    pub(crate) commands: Vec<Vec<String>>,
}

impl HookEntry {
    pub(crate) fn needs_comparison(&self) -> bool {
        self.when == When::WorkspaceChanged || self.filters.is_set()
    }
}

pub(crate) struct PathFilters {
    pub(crate) paths: Option<Vec<String>>,
    pub(crate) extensions: Option<Vec<String>>,
    globs: Option<GlobSet>,
}

impl PathFilters {
    fn is_set(&self) -> bool {
        self.paths.is_some() || self.extensions.is_some()
    }

    pub(crate) fn matches(&self, path: &Path) -> bool {
        self.globs.as_ref().is_none_or(|globs| globs.is_match(path))
            && self.extensions.as_ref().is_none_or(|extensions| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        extensions
                            .iter()
                            .any(|allowed| allowed.eq_ignore_ascii_case(extension))
                    })
            })
    }
}

#[derive(Debug)]
pub(crate) enum ConfigError {
    Read(io::Error),
    Parse(serde_json::Error),
    Discovery(DiscoveryError),
    ConflictingLists,
    InvalidCommand(HookName, usize),
    EmptyFilter(HookName, &'static str),
    InvalidPath(HookName, String),
    CompilePaths(HookName, globset::Error),
    InvalidExtension(HookName, String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "could not read latch.json: {error}"),
            Self::Parse(error) => write!(formatter, "could not parse latch.json: {error}"),
            Self::Discovery(error) => error.fmt(formatter),
            Self::ConflictingLists => {
                write!(formatter, "latch.json must choose either commands or hooks")
            }
            Self::EmptyFilter(name, field) => write!(
                formatter,
                "hooks.{}.{field} must contain at least one value",
                name.name()
            ),
            Self::InvalidPath(name, path) => write!(
                formatter,
                "hooks.{}.paths contains an invalid worktree-relative pattern: {path:?}",
                name.name()
            ),
            Self::CompilePaths(name, error) => {
                write!(formatter, "invalid hooks.{}.paths: {error}", name.name())
            }
            Self::InvalidExtension(name, extension) => write!(
                formatter,
                "invalid extension {extension:?} in hooks.{}.extensions",
                name.name()
            ),
            Self::InvalidCommand(name, number) => write!(
                formatter,
                "command {number} for {} in latch.json needs a nonempty executable and arguments without NUL bytes",
                name.name()
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Resolves the selected hook by appending native scripts to validated explicit entries.
pub(crate) fn load(
    directory: &Path,
    selected: HookName,
) -> Result<Option<HookEntries<HookEntry>>, ConfigError> {
    load_with_source(directory, selected).map(|loaded| loaded.entries)
}

pub(crate) struct LoadedHook {
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) entries: Option<HookEntries<HookEntry>>,
}

/// Resolves the hook and reports the config file that was successfully read.
pub(crate) fn load_with_source(
    directory: &Path,
    selected: HookName,
) -> Result<LoadedHook, ConfigError> {
    let path = directory.join("latch.json");
    let (config_path, config) = match fs::read(&path) {
        Ok(bytes) => (
            Some(path),
            serde_json::from_slice(&bytes).map_err(ConfigError::Parse)?,
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (None, Config::default()),
        Err(error) => return Err(ConfigError::Read(error)),
    };
    Ok(LoadedHook {
        config_path,
        entries: resolve_hook(directory, selected, config)?,
    })
}

fn resolve_hook(
    directory: &Path,
    selected: HookName,
    config: Config,
) -> Result<Option<HookEntries<HookEntry>>, ConfigError> {
    let hooks = match (config.commands, config.hooks) {
        (Some(_), Some(_)) => return Err(ConfigError::ConflictingLists),
        (None, None) => Hooks::default(),
        (None, Some(hooks)) if hooks.after_edit.is_none() && hooks.turn_end.is_none() => {
            return Ok(None);
        }
        (None, Some(hooks)) => hooks,
        (Some(commands), None) => Hooks {
            after_edit: None,
            turn_end: Some(HookEntries::Single(EntryConfig {
                commands,
                ..EntryConfig::default()
            })),
        },
    };
    let turn_end = hooks
        .turn_end
        .map(|hook| resolve_entries(HookName::TurnEnd, hook, &config.defaults))
        .transpose()?;
    let after_edit = hooks
        .after_edit
        .map(|hook| resolve_entries(HookName::AfterEdit, hook, &config.defaults))
        .transpose()?;
    let explicit = match selected {
        HookName::TurnEnd => turn_end,
        HookName::AfterEdit => after_edit,
    };
    if matches!(&explicit, Some(HookEntries::Multiple(entries)) if entries.is_empty()) {
        return Ok(explicit);
    }
    let commands = discovery::commands(directory, selected).map_err(ConfigError::Discovery)?;
    if commands.is_empty() {
        return Ok(explicit);
    }
    let discovered = resolve(
        selected,
        EntryConfig {
            commands,
            ..EntryConfig::default()
        },
        &config.defaults,
    )?;
    Ok(Some(match explicit {
        None => HookEntries::Single(discovered),
        Some(entries) => {
            let mut entries = entries.into_vec();
            entries.push(discovered);
            HookEntries::Multiple(entries)
        }
    }))
}

fn resolve_entries(
    name: HookName,
    entries: HookEntries<EntryConfig>,
    defaults: &Settings,
) -> Result<HookEntries<HookEntry>, ConfigError> {
    match entries {
        HookEntries::Single(entry) => resolve(name, entry, defaults).map(HookEntries::Single),
        HookEntries::Multiple(entries) => entries
            .into_iter()
            .map(|entry| resolve(name, entry, defaults))
            .collect::<Result<Vec<_>, _>>()
            .map(HookEntries::Multiple),
    }
}

fn resolve(
    name: HookName,
    hook: EntryConfig,
    defaults: &Settings,
) -> Result<HookEntry, ConfigError> {
    validate_commands(name, &hook.commands)?;
    let filters = compile_filters(name, hook.paths, hook.extensions)?;
    Ok(HookEntry {
        when: hook
            .when
            .or(defaults.when)
            .unwrap_or(When::WorkspaceChanged),
        on_success: hook.on_success.or(defaults.on_success).unwrap_or_default(),
        on_failure: hook.on_failure.or(defaults.on_failure).unwrap_or_default(),
        failure_output: hook
            .failure_output
            .or(defaults.failure_output)
            .unwrap_or_default(),
        tail_lines: hook.tail_lines.or(defaults.tail_lines).unwrap_or_default(),
        rtk_filter: hook
            .rtk_filter
            .or_else(|| defaults.rtk_filter.clone())
            .unwrap_or_default(),
        filters,
        commands: hook.commands,
    })
}

fn compile_filters(
    name: HookName,
    paths: Option<Vec<String>>,
    mut extensions: Option<Vec<String>>,
) -> Result<PathFilters, ConfigError> {
    let globs = paths
        .as_deref()
        .map(|patterns| compile_paths(name, patterns))
        .transpose()?;
    if let Some(extensions) = &mut extensions {
        if extensions.is_empty() {
            return Err(ConfigError::EmptyFilter(name, "extensions"));
        }
        for extension in extensions {
            let suffix = extension.strip_prefix('.').unwrap_or(extension);
            if suffix.is_empty()
                || !suffix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Err(ConfigError::InvalidExtension(name, extension.clone()));
            }
            *extension = suffix.to_ascii_lowercase();
        }
    }
    Ok(PathFilters {
        paths,
        extensions,
        globs,
    })
}

fn compile_paths(name: HookName, patterns: &[String]) -> Result<GlobSet, ConfigError> {
    if patterns.is_empty() {
        return Err(ConfigError::EmptyFilter(name, "paths"));
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if pattern.is_empty()
            || Path::new(pattern).is_absolute()
            || pattern.split('/').any(|part| part == "..")
        {
            return Err(ConfigError::InvalidPath(name, pattern.clone()));
        }
        builder.add(
            GlobBuilder::new(pattern)
                .literal_separator(true)
                .backslash_escape(true)
                .build()
                .map_err(|error| ConfigError::CompilePaths(name, error))?,
        );
    }
    builder
        .build()
        .map_err(|error| ConfigError::CompilePaths(name, error))
}

fn validate_commands(name: HookName, commands: &[Vec<String>]) -> Result<(), ConfigError> {
    for (index, command) in commands.iter().enumerate() {
        if command.first().is_none_or(|program| program.is_empty())
            || command.iter().any(|argument| argument.contains('\0'))
        {
            return Err(ConfigError::InvalidCommand(name, index + 1));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_settings_override_defaults_independently() {
        let defaults = Settings {
            when: Some(When::Always),
            on_failure: Some(OnFailure::Stop),
            ..Settings::default()
        };
        let hook = EntryConfig {
            on_success: Some(OnSuccess::Output),
            commands: vec![vec!["verify".into(), "--quiet".into()]],
            ..EntryConfig::default()
        };
        let resolved = resolve(HookName::TurnEnd, hook, &defaults).unwrap();
        assert!(resolved.when == When::Always);
        assert!(resolved.on_failure == OnFailure::Stop);
        assert!(resolved.on_success == OnSuccess::Output);
        assert_eq!(resolved.commands, [["verify", "--quiet"]]);
        assert!(!resolved.needs_comparison());

        let fallback = resolve(
            HookName::TurnEnd,
            EntryConfig::default(),
            &Settings::default(),
        )
        .unwrap();
        assert!(fallback.when == When::WorkspaceChanged);
        assert!(fallback.on_failure == OnFailure::Collect);
        assert!(fallback.on_success == OnSuccess::Silent);
        assert!(fallback.needs_comparison());
    }

    #[test]
    fn filters_combine_recursive_paths_and_normalized_final_extensions() {
        let filters = compile_filters(
            HookName::AfterEdit,
            Some(vec!["src/**".into()]),
            Some(vec![".TS".into(), "tsx".into()]),
        )
        .unwrap();
        for (path, matches) in [
            ("src/main.Ts", true),
            ("src/deep/view.tsx", true),
            ("Src/main.ts", false),
            ("docs/example.ts", false),
            ("src/file.ts.bak", false),
            ("src/types", false),
        ] {
            assert_eq!(filters.matches(Path::new(path)), matches, "{path}");
        }
    }

    #[test]
    fn rejects_parent_globs_and_compound_extensions() {
        assert!(matches!(
            compile_filters(HookName::AfterEdit, Some(vec!["src/../**".into()]), None),
            Err(ConfigError::InvalidPath(HookName::AfterEdit, path)) if path == "src/../**"
        ));
        assert!(matches!(
            compile_filters(HookName::TurnEnd, None, Some(vec![".tar.gz".into()])),
            Err(ConfigError::InvalidExtension(HookName::TurnEnd, suffix)) if suffix == ".tar.gz"
        ));
    }
}
