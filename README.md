# Latch

Latch runs project checks after native edits or when a coding harness finishes
a turn. Define checks in a package manifest, in `latch.json`, or in both.

## Install

From this checkout, run `cargo install --locked --path crates/latch`. The
[release guide](docs/releasing.md) covers npm and standalone archives. Once
the CLI is on `PATH`, install the plugin for your harness.

Try the [TypeScript or Python example](examples/) to see package-script
discovery with a real project.

The [docs app](apps/docs/) contains the quickstart and configuration guide.

## Configuration

### Use existing package scripts

Latch discovers these names in the hook or CLI working directory.

| Manifest | After a native edit | At turn end |
| --- | --- | --- |
| `package.json` under `scripts` | `hook:afteredit` | `hook:turnend` |
| `pyproject.toml` under `[project.scripts]` | `hook-afteredit` | `hook-turnend` |

Add this to `package.json` to run ESLint after edits.

```json
{
  "scripts": {
    "hook:afteredit": "eslint ."
  }
}
```

Latch runs JavaScript scripts through npm, pnpm, Yarn, or Bun. It uses the
manager named by `packageManager`, then looks for a local lockfile, then
defaults to npm. Set `packageManager` if lockfiles identify different
managers. Latch invokes the selected executable from `PATH`.

For a packaged Python project, add an entry point and Latch runs it with
`uv run hook-afteredit`. The project needs a build system to install the
entry point. See the [uv entry point documentation](https://docs.astral.sh/uv/concepts/projects/config/#command-line-interfaces).

```toml
[project.scripts]
hook-afteredit = "my_project.checks:lint"
```

If both manifests define the same event, JavaScript runs before Python.
The discovered commands run together after any matching explicit entries
and inherit `defaults` from `latch.json`, including `when`, `on_success`,
and `on_failure`. A file containing just `defaults` is valid.

### Add explicit commands

Create `latch.json` in the same working directory.

```json
{
  "defaults": {
    "on_success": "silent",
    "on_failure": "collect"
  },
  "hooks": {
    "turn_end": {
      "when": "workspace_changed",
      "on_failure": "stop",
      "commands": [
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "clippy", "--workspace", "--", "-D", "warnings"]
      ]
    }
  }
}
```

Add an `after_edit` entry under `hooks` to run checks after a native edit and
send the report to the agent during the same turn:

```json
"after_edit": {
  "commands": [["cargo", "fmt", "--all", "--", "--check"]]
}
```

Set `after_edit` to `[]` to disable edit checks, including discovery for that event.
Native edit tools are `apply_patch`
in Codex, `Edit`, `Write`, `MultiEdit`, and `NotebookEdit` in Claude Code,
and `create`, `edit`, `str_replace_editor`, and `apply_patch` in Copilot CLI.
Shell writes and external file changes are covered by the turn-end Git
comparison when they affect Git-visible files.

Each hook accepts a single object or an array of objects. Each object defines
an ordered command list and inherits any omitted settings from project defaults.

| Field | Built-in default | Values |
| --- | --- | --- |
| `when` | `workspace_changed` | `turn_end` compares against the start of the turn. `after_edit` compares immediately before and after the matching edit. Both run when a comparison is unavailable. `always` runs on every matching event. |
| `on_success` | `silent` | `silent` discards successful command output. `output` includes both streams. |
| `on_failure` | `collect` | `collect` runs the full list and reports all failures in order. `stop` skips the remaining commands in that object after the first failure. Later matching objects still run. |

### Filter by changed files

Add `paths`, `extensions`, or both to a hook object. Each object checks the same
set of changed files independently. Matching objects run in array order and
their output is combined into one report.

```json
{
  "hooks": {
    "after_edit": [
      { "commands": [["npm", "run", "lint"]] },
      { "extensions": ["md", "mdx"], "commands": [["npm", "run", "lint:md"]] },
      { "extensions": ["ts", "tsx"], "commands": [["npm", "run", "typecheck"]] }
    ]
  }
}
```

- `paths` matches case-sensitive, Git-worktree-relative globs. Use `/` as the
  separator and `**` for recursive matching.
- `extensions` matches the final filename suffix, case-insensitively and
  with or without a leading dot. For docs-only checks, use `["md", "mdx"]`.
- A changed file must match at least one entry in **each** configured filter.
  The object's commands run if any changed file qualifies. An omitted filter
  imposes no restriction. Empty filter lists are invalid.
- Filters compare against the start of the turn or the start of the native
  edit, depending on the hook. They apply even with `when: "always"`. Renames
  can match either path. If a reliable comparison is unavailable, the
  checks run.

Filters decide whether to start automatic checks. Manual CLI and tool runs
execute all entries for the selected hook on demand. `latch explain` shows
each entry's resolved settings and filters under `entries` when the hook uses
an array or combines explicit and discovered commands.

Each command is an executable followed by its arguments. To use shell
syntax, invoke a shell explicitly, for example `["sh", "-c", "first | second"]`.
A nonzero child exit, termination by signal, or failure to launch counts as
a command failure. Unknown settings or event names fail config validation
before any commands run. An empty array disables one event. An empty
`hooks` object disables both events, including discovery.
Existing files with top-level `commands` still define the `turn_end` list.
Choose either top-level `commands` or `hooks` in a file.

## Running checks

- **CLI.** Run `latch` or `latch run turn_end` to run turn-end checks immediately. Run `latch run after_edit` for edit checks.
  Output goes to its original stdout or stderr stream. The CLI exits `0`
  when all checks pass, `1` on a check or config failure, and `2` for
  invalid CLI usage.
- **Inspect config.** Run `latch explain` for `turn_end` or
  `latch explain after_edit` for edit checks. It prints the config
  path, resolved settings, filters, comparison point, and commands as JSON
  without executing them.
- **Tool.** The plugins expose the local MCP tool `run_turn_end`. It takes
  no arguments, runs the turn-end checks immediately, and gives the model a tool error
  with the report when validation fails. Installations of the plugins require
  `latch` on the harness's `PATH`. The harness launches `latch mcp` over
  stdio in the project directory.
- **Turn end.** Runs once at turn completion, subject to `when`. Codex delivers
  output as an app-server hook warning. The tested version of
  `codex exec --json` omits that warning. Claude Code emits an informational
  event. Copilot CLI emits a hook warning. Latch attaches a failed report once
  to the next user prompt in the same session and working directory, without
  an extra turn or recheck.
- **After edit.** Runs after matching native tool calls when configured. The
  default gate skips an edit that leaves Git-visible content unchanged, even
  if the workspace was already dirty. Concurrent changes to the workspace
  can also count as a change during that comparison.
  Latch attaches the report to the existing tool-result exchange, so the
  agent can see it before replying.

The hook and tool render a single report with the individual command results.
Each reported command can contribute up to 2,000 bytes per output stream.
Direct CLI runs replay the full output selected by the configured policy.

See setup for [Codex](plugins/codex/README.md),
[Claude Code](plugins/claude-code/README.md), or
[GitHub Copilot CLI](plugins/copilot/README.md).

## Development

Install Rust through rustup and use Node.js with npm 11 for the Turborepo tasks.

| Command | Use |
| --- | --- |
| `npm ci` | Install the pinned development tooling |
| `npm run build` | Build the CLI at `target/debug/latch` |
| `npm run check` | Check formatting, run Clippy, and run Cargo tests |
| `npm run fmt` | Apply Rust formatting |
| `cargo run --locked -p latch -- --version` | Check the CLI version with Cargo |

Cargo owns Rust builds and incremental compilation. Turborepo orchestrates the
development tasks with task caching disabled so Cargo can evaluate the local toolchain and target.

See the [project guide](docs/project-guide.md) for workspace conventions.
